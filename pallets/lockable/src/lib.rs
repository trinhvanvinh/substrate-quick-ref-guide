#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	traits::{Currency, LockIdentifier, LockableCurrency, WithdrawReasons},
};
// type BalanceOf<T> =
// 	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


#[frame_support::pallet]
pub mod pallet {
	pub use super::*;
	use sp_runtime::traits::{AtLeast32BitUnsigned, Saturating};

	const EXAMPLE_ID: LockIdentifier = *b"example ";

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		//type Currency: Currency<Self::AccountId>;


		//type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		type Balance: Member + Parameter + AtLeast32BitUnsigned + Default + Copy;
		
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;


	#[pallet::storage]
	#[pallet::getter(fn get_balance)]
	pub(super) type BalanceToAccount<T:Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::Balance, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),

		Locked(T::AccountId, BalanceOf<T>),
		Unlocked(T::AccountId),
		LockExtended(T::AccountId, BalanceOf<T>),

		MinteNewSupply(T::AccountId),
		Transfered(T::AccountId, T::AccountId, T::Balance)
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}


	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

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

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn lock_capital( origin: OriginFor<T>, #[pallet::compact] amount: BalanceOf<T> )-> DispatchResultWithPostInfo{
			
			let user = ensure_signed(origin)?;
			T::Currency::set_lock(EXAMPLE_ID, &user, amount, WithdrawReasons::all());

			Self::deposit_event(Event::Locked(user, amount));
			
			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn extend_lock(origin: OriginFor<T>, #[pallet::compact] amount: BalanceOf<T>)-> DispatchResultWithPostInfo{
			let user = ensure_signed(origin)?;

			T::Currency::extend_lock(EXAMPLE_ID, &user, amount, WithdrawReasons::all());

			Self::deposit_event(Event::LockExtended(user, amount));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn unlock_all(origin: OriginFor<T>)-> DispatchResultWithPostInfo{

			let user = ensure_signed(origin)?;

			T::Currency::remove_lock(EXAMPLE_ID, &user);

			Self::deposit_event(Event::Unlocked(user));

			Ok(().into())
		}

		#[pallet::weight(10_000)]
		pub(super) fn mint(origin: OriginFor<T>, #[pallet::compact] amount: T::Balance) -> DispatchResult{
			let sender = ensure_signed(origin)?;
			<BalanceToAccount>::insert(sender, amount);
			Self::deposit_event(Event::MinteNewSupply(sender));

			Ok(())
		}

		#[pallet::weight(1_000)]
		pub(super) fn transfer(origin: OriginFor<T>, to: T::AccountId, amount: T::Balance)-> DispatchResult{
			let sender = ensure_signed(origin)?;
			let sender_balance = Self::get_balance(sender);
			let receiver_balance = Self::get_balance(to);

			// calculate new balance
			let update_sender = sender_balance.saturating_sub(amount);
			let update_to = receiver_balance.saturating_add(amount);

			// update accounts balance storage
			<BalanceToAccount>::insert(sender, update_sender);
			<BalanceToAccount>::insert(to, update_to);

			Self::deposit_event(Event::Transfered(sender, to, amount));

			Ok(())
		}
	}
}
