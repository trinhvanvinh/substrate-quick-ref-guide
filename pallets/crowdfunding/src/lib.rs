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

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use frame_support::{
	ensure,
	sp_runtime::traits::{AccountIdConversion, Hash, Saturating, Zero},
	storage::child,
	traits::{Currency, ExistenceRequirement, Get, ReservableCurrency, WithdrawReasons},
	PalletId
};

use frame_system::{ensure_signed};

pub type FundIndex = u32;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

pub struct FundInfo<AccountId, Balance, BlockNumber>{
	beneficiary: AccountId,
	deposit: Balance,
	raised: Balance,
	end: BlockNumber,
	goal: Balance
}

type FundInfoOf<T> = FundInfo<AccountIdOf<T>, BalanceOf<T>, <T as frame_system::Config>::BlockNumber>;

const PALLET_ID: ModuleId = ModuleId(*b"ex/cfund");

#[frame_support::pallet]
pub mod pallet {
	pub use super::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: ReservableCurrency<Self::AccountId>;
		type SubmissionDeposit: Get<BalanceOf<Self>>;
		type MinContribution: Get<BalanceOf<Self>>;
		type RetirementPeriod: Get<Self::BlockNumber>;
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
	#[pallet::getter(fn funds)]
	pub(super) type Funds = StorageMap<_, Blake2_128Concat, FundIndex, FundInfoOf<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn fund_count)]
	pub(super) type FundCount = StorageValue<_, FundIndex, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),

		Created(FundIndex, <T as frame_system::Config>::BlockNumber),
		Contributed(<T as frame_system::Config>::AccountId, FundIndex, BalanceOf<T>, <T as frame_system::Config>::BlockNumber),
		Withdraw(<T as frame_system::Config>::AccountId, FundIndex, BalanceOf<T>, <T as frame_system::Config>::BlockNumber),
		Retiring(FundIndex, <T as frame_system::Config>::BlockNumber),
		Dissolved(FundIndex, <T as frame_system::Config>::BlockNumber, <T as frame_system::Config>::AccountId),
		Dispensed(FundIndex, <T as frame_system::Config>::BlockNumber, <T as frame_system::Config>::AccountId)
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,

		EndTooEarly,
		ContributionTooSmall,
		InvalidIndex,
		ContributionPeriodOver,
		FundStillActive,
		NoContribution,
		FundNotRetired,
		UnsuccessfullFund
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


		#[pallet::weight(10_000)]
		fn create(origin: OriginFor<T>, beneficiary: AccountIdOf<T>, goal: BalanceOf<T>, end: T::BlockNumber)-> DispatchResult{
			let creator = ensure_signed(origin)?;
			let now = <frame_system::Module<T>>::block_number();
			ensure!(end> now, Error::<T>::EndTooEarly);
			let deposit = T::SubmissionDeposit::get();
			let imb = T::Currency::withdraw(creator, deposit, WithdrawReasons::TRANSFER, ExistenceRequirement::AllowDeath);
			let index = <FundCount<T>>::get();
			<FundCount<T>>::put(index + 1);
			T::Currency::resolve_creating(Self::fund_account_id(index), imb);
			<Funds<T>>::insert(index, FundInfo{
				beneficiary,
				deposit,
				raised: Zero::zero(),
				end,
				goal
			});
			Self::deposit_event(Event::Created(index, now));

			Ok(())
		}

		#[pallet::weight(10_000)]
		fn contribute(origin: OriginFor<T>, index: FundIndex, value: BalanceOf<T>)-> DispatchResult{
			let who = ensure_signed(origin)?;
			ensure!(value >= T::MinContribution::get() , Error::<T>::ContributionTooSmall);
			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidIndex);

			let now = <frame_system::Module<T>>::block_number();
			ensure!(fund.end > now, Error::<T>::ContributionPeriodOver);

			// add contribute
			T::Currency::transfer(who, Self::fund_account_id(index), value, ExistenceRequirement::AllowDeath);

			fund.raised += value;
			Funds::<T>::insert(index, fund);

			let balance = Self::contribution_get(index, who);
			let balance = balance.saturating_add(value);
			Self::contribution_put(index, who, balance);

			Self::deposit_event(Event::Contributed(who, index, balance, now));

			Ok(())
		}

		#[pallet::weight(10_000)]
		fn withdraw(origin: OriginFor<T>, index: FundIndex)-> DispatchResult{
			let who = ensure_signed(origin)?;

			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidIndex);
			let now = <frame_system::Module<T>>::block_number();
			ensure!(fund.end < now, Error::<T>::FundStillActive);

			let balance = Self::contribution_get(index, who);
			ensure!(balance > Zero::zero(), Error::<T>::NoContribution);

			let _ = T::Currency::resolve_into_existing(who, T::Currency::withdraw(
					Self::fund_account_id(index),
					balance,
					WithdrawReasons::TRANSFER,
					ExistenceRequirement::AllowDeath
			));

			Self::contribution_kill(index, who);
			fund.raised = fund.raised.saturating_sub(balance);
			<Funds<T>>::insert(index, fund);

			Self::deposit_event(Event::Withdraw(who, index, balance, now));

			Ok(())
		}

		#[pallet::weight(10_000)]
		fn dissolve(origin: OriginFor<T>, index: FundIndex)-> DispatchResult {
			let reporter = ensure_signed(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidIndex);

			let now = <frame_system::Module<T>>::block_number();
			ensure!(now > fund.end + T::RetirementPeriod::get(), Error::<T>::FundNotRetired);

			let account = Self::fund_account_id(index);

			let _ = T::Currency::resolve_creating(reporter, T::Currency::withdraw(
				account,
				fund.deposit + fund.raised ,
				WithdrawReasons::TRANSFER,
				ExistenceRequirement::AllowDeath
			));

			<Funds<T>>::remove(index);

			Self::crowdfund_kill(index);

			Self::deposit_event(Event::Dissolved(index, now, reporter));

			Ok(())
		}

		#[pallet::weight(10_000)]
		fn dispense(origin: OriginFor<T>, index: FundIndex)-> DispatchResult{
			let caller = ensure_signed(origin);

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidIndex);

			let now = <frame_system::Module<T>>::block_number();

			ensure!(now >= fund.end, Error::<T>::FundStillActive);

			ensure!(fund.raised >= fund.goal, Error::<T>::UnsuccessfullFund);

			let account = Self::fund_account_id(index);

			let _ = T::Currency::resolve_creating(fund.beneficiary, T::Currency::withdraw(
				account,
				fund.raised,
				WithdrawReasons::TRANSFER,
				ExistenceRequirement::AllowDeath
			));

			<Funds<T>>::remove(index);

			Self::crowdfund_kill(index);

			Self::deposit_event(Event::Dispensed(index, now, caller));

			Ok(())
		}

	}
}

impl<T: Config> Pallet<T>{
	pub fn fund_account_id(index: FundIndex) -> T::AccountId{
	
		PALLET_ID.into_sub_account(index)
	}

	pub fn id_from_index(index: FundIndex)-> child::ChildInfo{
		let mut buf = Vec::new();
		buf.extend_from_slice(b"crowdfnd");
		buf.extend_from_slice(&index.to_le_bytes()[..]);

		child::ChildInfo::new_default(T::Hashing::hash(&buf[..]).as_ref())
	}

	pub fn contribution_put(index: FundIndex, who: T::AccountId, balance: BalanceOf<T>){
		let id = Self::id_from_index(index);
		who.using_encoded(|b| child::put(id,b,balance));
	}

	pub fn contribution_get(index: FundIndex, who: T::AccountId)-> BalanceOf<T>{
		let id = Self::id_from_index(index);
		who.using_encoded(|b| child::get_or_default::<BalanceOf<T>>(id,b))
	}

	pub fn contribution_kill(index: FundIndex, who: T::AccountId){
		let id = Self::id_from_index(index);
		who.using_encoded(|b| child::kill(id, b));
	}

	pub fn crowdfund_kill(index: FundIndex){
		let id = Self::id_from_index(index);
		child::kill_storage(id, None);
	}

}
