#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::{Decode, Encode};
use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::MaxEncodeLen,
	traits::{BalanceStatus, Currency, Get, ReservableCurrency},
	weights:: Weight,
	RuntimeDebugNoBond
}

use scale_info::TypeInfo;
use sp_io::hashing::blake2_256;
use sp_runtime::RuntimeDebug;
use sp_std::{
	marker:: PhantomData,
	ops::{
		Deref, DerefMut
	},
	prelude::*
};

pub struct PendingSwap<T: Config> {
	pub source: T::AccountId,
	pub action: T::SwapAction,
	pub en_block: T::BlockNumber
}

pub trait SwapAction<AccountId, T:Config>{
	fn reserve(self, source: AccountId)-> DispatchResult;
	fn claim(self, source: AccountId, target: AccountId)-> bool;
	fn weight(self)-> Weight;
	fn cancel(self, source: AccountId);
}

pub type HashedProof = [u8;32];

#[scale_info(skip_type_params(C))]
pub struct BalanceSwapAction<AccountId, C: ReservableCurrency<AccountId>>{
	value: <C as Currency<AccountId>>::Balance,
	_marker: PhantomData<C>
}

impl<AccountId, BalanceSwapAction<AccountId, C> where C: ReservableCurrency<AccountId> {
	pub fn new(value: <C as Currency<AccountId>>:: Balance)-> self{
		Self {value, _marker: PhantomData}
	}
}

impl<AccountId, C> Deref for BalanceSwapAction<AccountId, C> where C: ReservableCurrency<AccountId>{
	type Target = <C as Currency<AccountId>>::Balance;
	fn deref(self)-> Self::Target{
		self.value
	}
}

impl<AccountId, C> DerefMut for BalanceSwapAction<AccountId, C> where C: ReservableCurrency<AccountId>{
	fn deref_mut(self)-> Self::Target{
		self.value
	}
}

impl<T: Config, AccountId, C> SwapAction<AccountId, T> for BalanceSwapAction<AccountId, C> where C: ReservableCurrency<AccountId>{
	fn reserve(self, source: AccountId)-> DispatchResult{
		C::reserve(source, self.value)
	}
	fn claim(self, source: AccountId, target: AccountId)-> bool{
		C::repatriate_reserved(source, target, self.value, BalanceStatus::Free).is_ok()
	}
	fn weight(self)-> Weight{
		T::DbWeight::get().reads_writes(1, 1)
	}
	fn cancel(self, source, AccountId){
		C::unreserve(source, self.value)
	}
}


#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;


	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type SwapAction: SwapAction<Self::AccountId, Self> + Parameter + MaxEncodeLen;

		#[pallet::constant]
		type ProofLimit: Get<u32>
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	#[pallet::storage]
	pub type PendingSwaps<T: Config> = StorageDoubleMap<_, Twox64Concat, T::AccountId, Blake2_128Concat, HashedProof, PendingSwap<T>>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),

		NewSwap{account: T::AccountId, proof: HashedProof, swap: PendingSwap<T>},
		SwapClaimed{account: T::AccountId, proof: HashedProof, success: bool},
		SwapCancelled{account: T::AccountId, proof: HashedProof}
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,

		AlreadyExist,
		InvalidProof,
		ProofTooLarge,
		SourceMismatch,
		AlreadyClaimed,
		NoExist,
		ClaimActionMismatch,
		DurationNotPassed
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let who = ensure_signed(origin)?;

			// Update storage.
			<Something<T>>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored(something, who));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => return Err(Error::<T>::NoneValue.into()),
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(())
				},
			}
		}


		pub fn create_swap(origin: OriginFor<T>, target: T::AccountId, hashed_proof: HashedProof, 
			action: T::SwapAction, duration: T::BlockNumber )-> DispatchResult{

			let source = ensure_signed(origin)?;
			ensure!(!PendingSwap::<T>::contains_key(target, hashed_proof), Error::<T>::AlreadyExist);	

			action.reserve(source);
			let swap = PendingSwap{
				source,
				action,
				end_block: frame_system::Pallet::<T>::block_number()+ duration
			};

			PendingSwap::<T>::insert(target.clone(), hashed_proof, swap.clone());
			Self::deposit_event(Event::NewSwap{account: target, proof: hashed_proof, swap});

			Ok(())

		}

		pub fn claim_swap(origin: OriginFor<T>, proof: Vec<u8>, action: T::SwapAction)-> DispatchResult{

			ensure!(proof.len() <= T::ProofLimit::get() as usize, Error::<T>:: ProofTooLarge);

			let target = ensure_signed(origin)?;
			let hashed_proof = blake2_256(proof);

			let swap = PendingSwaps::<T>::get(target, hashed_proof).ok_or(Error::<T>::InvalidProof);
			ensure!(swap.action == action, Error::<T>::ClaimActionMismatch);
			
			let succeeded = swap.action.claim(swap.source, target);

			PendingSwaps::<T>::remove(target.clone(), hashed_proof);

			Self::deposit_event(Event::SwapClaimed{account: target, proof: hashed_proof, success: succeeded});

			Ok(())
		}

		pub fn cancel_swap(origin: OriginFor<T>, target: T::AccountId, hashed_proof: HashedProof)-> DispatchResult{

			let source = ensure_signed(origin)?;

			let swap = PendingSwaps::T::get(target, hashed_proof).ok_or(Error::<T>::NotExist);

			ensure!(swap.source == source, Error::<T>::SourceMismatch);
			ensure!(frame_system::Pallet::<T>::block_number() >= swap.end_block, Error::<T>::DurationNotPassed);

			swap.action.cancel(swap.source);
			PendingSwaps::<T>::remove(target, hashed_proof);

			Self::deposit_event(Event::SwapCancelled{account: target, proof: hashed_proof});

			Ok(())
		}
	}
}
