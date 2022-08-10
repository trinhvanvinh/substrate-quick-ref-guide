#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

use sp_runtime::{
	RuntimeDebug, traits::{
		AtLeast32BitUnsigned, Zero, Saturating, CheckedAdd, CheckedSub
	}
};

pub struct MetaData<AccountId, Balance>{
	issuance: Balance,
	minter: AccountId,
	burner: AccountId
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		// the type used to store balance
		type Balance: Member + Parameter + AtLeast32BitUnsigned + Default + Copy;
		// the minimum balance necessary for an account to exist
		type MinBalance: Get<Self::Balance>;
	}

	#[pallet::storage]
	#[pallet::getter(fn meta_data)]
	pub(super) type MetaDataStore<T:Config> = StorageValue<_, MetaData<T::AccountId, T::Balance>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn account)]
	pub(super) type Accounts<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::Balance, ValueQuery> ;


	// === genesis ===
	#[pallet::genesis_config]
	pub struct GenesisConfig<T:Config>{
		pub admin: T::AccountId
	}

	#[cfg(feature="std")]
	impl<T:Config> Default for GenesisConfig<T>{
		fn default() -> Self{
			Self{
				admin: Default::default()
			}
		}
	}
	#[pallet::genesis_build]
	impl<T:Config> GenesisBuild<T> for GenesisConfig<T>{
		fn build(self){
			MetaDataStore::<T>::put(MetaData{
				issuance: Zero::zero(),
				minter: self.admin.clone(),
				burner: self.admin.clone()
			});
		}
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

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),

		Created(T::AccountId),
		Killed(T::AccountId),
		Minted(T::AccountId, T::Balance),
		Burned(T::AccountId, T::Balance),
		Transfered(T::AccountId, T::AccountId, T::Balance)
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,

		BelowMinBalance,
		NoPermission,
		Overflow,
		Underflow,
		CannotBurnEmpty,
		InsufficientBalance
	}


	#[pallet::hooks]
	impl<T:Config> Hooks<BlockNumberFor<T>> for Pallet<T>{
		fn on_initialize(_n: T::BlockNumber)-> Weight{
			let mut meta = MetaDataStore::<T>::get();
			let value: T::Balance = 50u8.into();
			meta.issuance = meta.issuance.saturating_add(value);
			Accounts::<T>::mutate(meta.minter, |bal|{
				bal = bal.saturating_add(value);
			} );
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
		}

		#[pallet::weight(1_000)]
		pub(super) fn mint(origin: OriginFor<T>, beneficiary: T::AccountId, #[pallet::compact] amount: T::Balance )-> DispatchResult{
			let sender = ensure_signed(origin)?;
			ensure!(amount >= T::MinBalance()::get(), Error::<T>::BelowMinBalance);
			let mut meta = Self::meta_data();
			ensure!(sender== meta.minter, Error::<T>::NoPermission);

			meta.issuance = meta.issuance.checked_add(amount).ok_or(Error::<T>::Overflow);
			MetaDataStore::<T>::put(meta);

			if Self::increase_balance(beneficiary, amount){
				Self::deposit_event(Event::<T>::Created(beneficiary));
			}
			Self::deposit_event(Event::<T>::Minted(beneficiary, amount));

			Ok(())
		}

		#[pallet::weight(1_000)]
		pub(super) fn burn(origin: OriginFor<T>, burned: T::AccountId, #[pallet::compact] amount: T::Balance, allow_killing: bool)-> DispatchResult{
			let sender = ensure_signed(origin)?;
			let mut meta = Self::meta_data();
			ensure!(sender == meta.burner, Err::<T>::NoPermission );

			let balance = Accounts::<T>::get(burned);
			ensure!(balance> Zero::zero(), Error::<T>::CannotBurnEmpty);
			let new_balance = balance.saturating_sub(amount);
			let burn_amount = if new_balance < T::MinBalance::get(){
				ensure!(allow_killing, Error::<T>::BelowMinBalance);
				let burn_amount = balance;
				ensure!(meta.issuance.checked_sub(burn_amount).is_some(), Error::<T>::Underflow);
				Accounts::<T>:remove(burned);
				Self::deposit_event(Event::<T>::Killed(burned.clone()));
				burn_amount
			}else{
				let burn_amount = amount;
				ensure!(meta.issuance.checked_sub(burn_amount).is_some(), Error::<T>::Underflow);
				Accounts::<T>::insert(burned, new_balance);
				burn_amount

			}

			meta.issuance = meta.issuance.saturating_sub(burn_amount);
			MetaDataStore::<T>::put(meta);

			Self::deposit_event(Event::<T>::Burned(burned, burn_amount));

			Ok(())
		}

		#[pallet::weight(1_000)]
		#[transactional]
		pub(super) fn transfer(origin: OriginFor<T>, to: T::AccountId, #[pallet::compact] amount: T::Balance)-> DispatchResult{
			let sender = ensure_signed(origin)?;

			Accounts::<T>::try_mutate(to, |bal|-> DispatchResult{
				let new_bal = bal.checked_sub(amount).ok_or(Error::<T>::InsufficientBalance);
				ensure!(new_bal >= T::MinBalance::get(), Error::<T>::BelowMinBalance );
				bal = new_bal;
				Ok(())
			});

			Accounts::<T>::try_mutate(to, |bal|-> DispatchResult{
				let new_bal = bal.saturating_add(amount);
				ensure!(new_bal>= T::MinBalance::get(), Error::<T>::BelowMinBalance);
				bal = new_bal;
				Ok(())
			});

			Self::deposit_event(Event::<T>::Transfered(sender, to, amount));

			Ok(())
		}

	}
}

impl<T::Config> Pallet<T>{
	fn increase_balance(acc: T::AccountId, amount: T::Balance)-> bool{
		Accounts::<T>::mutate(acc, |bal|{
			let created = bal == Zero::zero();
			bal = bal.saturating_add(amount);
			created
		})
	}
}
