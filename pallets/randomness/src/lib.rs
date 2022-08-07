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
use frame_support::traits::Randomness;
use frame_support::inherent::Vec;
use frame_support::sp_runtime::traits::Hash;
use frame_support::traits::Currency;
use frame_support::traits::ReservableCurrency;
//use frame_support::PalletId;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(
	Encode, Decode, Default, Eq, PartialEq, RuntimeDebug, scale_info::TypeInfo, MaxEncodedLen,
)]
pub struct LotteryConfig<BlockNumber, Balance>{
	// price for entry
	price: Balance,
	// startblock of lottery
	start: BlockNumber,
	//length of lottery (start + length = end)
	length: BlockNumber,
	//deplay for choosing the winner of the lottery. (start + length + delay = paypout)
	// randomness in payout block will be used to determine the winner
	delay: BlockNumber,
	// whether this lottery will repeat after it completes
	repeat: bool
}

#[derive(Clone, Copy, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub struct PalletId(pub [u8; 8]);


pub trait ValidateCall<T:Config>{
	fn validate_call()-> bool;
}

impl<T:Config> ValidateCall<T> for (){
	fn validate_call()-> bool{
		false
	}
}

impl<T: Config> ValidateCall<T> for Pallet<T> {
	fn validate_call()-> bool{
		false
	}
}

type CallIndex = (u8, u8);


#[frame_support::pallet]
pub mod pallet {
	pub use super::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type MyRandomness: Randomness<Self::Hash, Self::BlockNumber>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		// the max number calls available in a single lottery
		#[pallet::constant]
		type MaxCalls: Get<u32>;

		#[pallet::constant]
		type MaxGenerateRandom: Get<u32>;

		

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
	#[pallet::getter(fn myStorageItem)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub type MyStorageItem<T> = StorageValue<_, <T as frame_system::Config>::Hash >;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub type Nonce<T> = StorageValue<_, u32>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
		UniqueCreated(<T as frame_system::Config>::Hash),

		LotteryStarted,
		CallsUpdated,
		Winner {winner: T::AccountId, lottery_balance: BalanceOf<T>},
		TicketBought {who: T::AccountId, call_index: CallIndex }
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,

		NotConfigured,
		InProgress,
		AlreadyEnded,
		InvalidCall,
		AlreadyParticipating,
		TooManyCalls,
		EncodingFailed
	}

	#[pallet::storage]
	pub(crate) type LotteryIndex<T> = StorageValue<_, u32, ValueQuery>;

	// the configuration for the current lottery
	#[pallet::storage]
	pub(crate) type Lottery<T: Config> = StorageValue<_, LotteryConfig<T::BlockNumber, BalanceOf<T> >>;

	// user who have purchased a ticket
	#[pallet::storage]
	pub(crate) type Participants<T:Config> = StorageMap<_, Twox64Concat, T::AccountId, (u32, BoundedVec<CallIndex, T::MaxCalls>), ValueQuery>;

	// Total number of tickets sold
	#[pallet::storage]
	pub(crate) type TicketsCount<T> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	pub(crate) type Tickets<T: Config> = StorageMap<_, Twox64Concat, u32, T::AccountId>;

	#[pallet::storage]
	pub(crate) type CallIndices<T:Config> = StorageValue<_, BoundedVec<CallIndex, T::MaxCalls>, ValueQuery>;



	#[pallet::hooks]
	impl<T:Config> Hooks<BlockNumberFor<T>> for Pallet<T>{
		fn on_initialize(n: T::BlockNumber) -> Weight{
			Lottery::<T>::mutate(|lottery|-> Weight{
				if let Some(config) = lottery{
					let payout_block = config.start.saturating_add(config.length).saturating_add(config.delay);

					if payout_block <= n{
						//let
					}
				}
				T::DbWeight::get().reads(1)
			})
		}
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
			//Ok(())
		}

		#[pallet::weight(100)]
		pub fn create_uniquie(origin: OriginFor<T>)-> DispatchResult {
			let sender = ensure_signed(origin)?;

			let nonce = Self::get_and_increment_nonce();
			let (randomValue, _) = T::MyRandomness::random(&nonce);

			<MyStorageItem<T>>::put(randomValue);
			Self::deposit_event(Event::UniqueCreated(randomValue));

			Ok(())
		}



	}

}

impl<T:Config> Pallet<T> {
	fn get_and_increment_nonce() -> Vec<u8>{
		let nonce = Nonce::<T>::get();
		Nonce::<T>::put(nonce.unwrap().wrapping_add(1));
		nonce.encode()
	}

	pub fn account_id()-> T::AccountId{
		T::PalletId::get().into_account_truncating()
	}
}






