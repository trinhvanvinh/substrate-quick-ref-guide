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

use frame_support::{
	traits::{
		BalanceStatus, ChangeMembers, Currency, EnsureOrigin, ExistenceRequirement::AllowDeath,
		Imbalance, OnUnbalanced, Randomness, ReservableCurrency
	},
	PalletId
};

use rand_chacha::{
	rand_core::{RngCore, SeedableRng},
	ChaChaRng
};

use scale_info::TypeInfo;

use sp_runtime::{
	traits::{
		AccountIdConversion, CheckedSub, Hash, InterSquareRoot, Saturating, StaticLookup,
		TrailingZeroInput, Zero
	},
	Percent, RuntimeDebug
};

type BalanceOf<T, I> = <<T as Config<I>>:: Currency as Currency< <T as frame_system::Config>::AccountId >>::Balance;
type NegativeImbalanceOf<T, I> = <<T as Config<I>>:: Currency as Currency<< T as frame_system::Config>::AccountId, >>::NegativeImbalance;
type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

pub enum Vote{
	Skeptic,
	Reject,
	Approve
}

pub enum Judgement{
	Rebid,
	Reject,
	Approve
}

pub enum Payout<Balance, BlockNumber>{
	value: Balance,
	begin: BlockNumber,
	duration: BlockNumber,
	paid: Balance,
}

pub enum VouchingStatus{
	Vouching,
	Banned
}

pub type StrikeCount = u32;

pub struct BidKind<AccountId, Balance>{
	Deposit(Balance),
	Vouch(AccountId, Balance)
}

pub struct Bid<AccountId, Balance>{
	who: AccountId,
	kind: BidKind<AccountId, Balance>,
	value: Balance
}

impl<AccountId: PartialEq, Balance> BidKind<AccountId, Balance>{
	fn check_voucher(self, v: AccountId)-> DispatchResult{
		if let BidKind::Vouch(a, _) = self{
			if a == v{
				Ok(())
			}else{
				Err("incorrect identity".into())
			}
		}else{
			Err("not vouched".into())
		}
	}
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

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		type Currency: ReservableCurrency<Self::AccountId>;
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		#[pallet::constant]
		type CandidateDeposit: Get<BalanceOf<Self, I>>;

		#[pallet::constant]
		type WrongSideDeduction: Get<BalanceOf<Self, I>>;

		#[pallet::constant]
		type MaxStrikes: Get<u32>;

		#[pallet::constant]
		type PeriodSpend: Get<BalanceOf<Self, I>>;

		type MembershipChanged: ChangeMembers<Self::AccountId>;

		#[pallet::constant]
		type RotationPeriod: Get<Self::BlockNumber>;

		#[pallet::constant]
		type MaxLockDuration: Get<Self::BlockNumber>;

		type FounderSetOrigin: EnsureOrigin<Self::Origiin>;

		type SuspensionJudgementOrigin: EnsureOrigin<Self::Origin>;

		#[pallet::constant]
		type ChallengePeriod: Get<Self::BlockNumber>;

		#[pallet::constant]
		type MaxCandidateIntake: Get<u32>;

	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I=()>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;


	#[pallet::storage]
	#[pallet::getter(fn founder)]
	pub type Founder<T:Config> = StorageValue<_, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn rules)]
	pub type Rules<T: Config> = StorageValue<_, T::Hash>;

	#[pallet::storage]
	#[pallet::getter(fn candidates)]
	pub type Candidates<T:Config> = StorageValue<_, Vec<Bid<T::AccountId, BalanceOf<T, I>>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn suspend_candidate)]
	pub type SuspendedCandidates<T:Config> = StorageMap<_, Twox64Concat, T::AccountId, (BalanceOf<T, I>, BidKind<T::AccountId, BalanceOf<T, I>> ) >;

	#[pallet::storage]
	#[pallet::getter(fn pot)]
	pub type Pot<T:Config> = StorageValue<_, BalanceOf<T, I>, ValueQuery> ;

	#[pallet::storage]
	#[pallet::getter(fn head)]
	pub type Head<T:Config> = StorageValue<_, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn members)]
	pub type Members<T:Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn suspend_member)]
	pub type SuspendMember<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bids)]
	pub type Bids<T:Config> = StorageValue<_, Vec<Bid<T::AccountId, BalanceOf<T, I>>>, ValueQuery >;

	#[pallet::storage]
	#[pallet::getter(fn vouching)]
	pub type Vouching<T:Config> = StorageMap<_, Twox64Concat, T::AccountId, VouchingStatus>;

	#[pallet::storage]
	#[pallet::getter(fn payout)]
	pub type Payouts<T:Config> = StorageMap<_, Twox64Concat, T::AccountId, Vec<(T::BlockNumber, BalanceOf<T, I>)> , ValueQuery> ;

	#[pallet::storage]
	#[pallet::getter(fn trike)]
	pub type Strike<T:Config> = StorageMap< _, Twox64Concat, T::AccountId, StrikeCount, ValueQuery >;

	#[pallet::storage]
	#[pallet::getter(fn votes)]
	pub type Votes<T:Config> = StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, T::AccountId, Vote>;

	#[pallet::storage]
	#[pallet::getter(fn defender)]
	pub type Defender<T: Config> = StorageValue<_, T::AccountId> ;

	#[pallet::storage]
	#[pallet::getter(fn defender_vote)]
	pub type DefenderVotes<T:Config> = StorageMap<_, Twox64Concat, T::AccountId, Vote>;

	#[pallet::storage]
	#[pallet::getter(fn max_member)]
	pub type MaxMembers<T:Config> = StorageValue<_, u32, ValueQuery> ;


	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),

		Founded{founder: T::AccountId},
		Bid{candidate_id: T::AccountId, offer: BalanceOf<T, I>},
		Vouch{candidate_id: T::AccountId, offer: BalanceOf<T, I>, vouching: T::AccountId},
		AutoUnbid{candidate: T::AccountId},
		Unbid{candidate: T::AccountId},
		Unvouch{candidate: T::AccountId},
		Inducted{primary: T::AccountId, candidate: Vec<T::AccountId>},
		SuspendeMemberJudgement{who: T::AccountId, judged: bool},
		CandidateSuspended{member: T::AccountId},
		MemberSuspended{member: T::AccountId},
		Challenged{member: T::AccountId},
		Vote{candidate: T::AccountId, voter: T::AccountId, vote: bool},
		DefenderVote{voter: T::AccountId, vote: bool},
		NewMaxMember{max: u32},
		Unfounded{founder: T::AccountId},
		Deposit{value: BalanceOf<T, I>}
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,

		BadPosition,
		NotMember,
		AlreadyMember,
		Suspended,
		NotSuspended,
		NoPayout,
		AlreadyFounded
		InsufficientPot,
		AlreadyVouching,
		NotVouching,
		Head,
		Founder,
		AlreadyBid,
		AlreadyCandidate,
		NotCandidate,
		MaxMembers,
		NotFounder,
		NotHead
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
	}
}
