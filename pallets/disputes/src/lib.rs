//FELIX REVIEW: Eventually it will be nice to have a short introduction here explaining what this pallet does and the
// avaliable methods etc.

// 1: Raise dispute using DisputeRaiser from pallet_proposals
// - It takes the raiser_id,project_id as dispute_key, list of jury(randomly selected upto 7 to 9 count), reason, fund_account
// - Exisiting implementation looks good, need to update the votes while inserting the new dispute

// 2: Vote on dispute.
// Get the vote as single yes or no and divide based on the number of the voters
// Need to come up with a way to change the votes that might require the storing the votes of each voter

// 3: finalise it in the on_initialize hook.
// Signal that this is ready for continuation. pallet-refund/pallet-proposals.
// Refund, Everythings ok.

// 4: an extrinsic is called claim_back(parameter: who, where.)

//pub mod impls;
#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;
pub mod impls;
pub mod traits;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use crate::traits::DisputeHooks;
    use codec::{FullCodec, FullEncode};
    use frame_support::{
        dispatch::fmt::Debug, pallet_prelude::*, weights::Weight, BoundedBTreeMap,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{AtLeast32BitUnsigned, Saturating};

    pub(crate) type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// The weights generated by the benchmarks.
        type WeightInfo: WeightInfoT;
        /// The key that links the dispute to the project.
        type DisputeKey: AtLeast32BitUnsigned
            + FullEncode
            + FullCodec
            + MaxEncodedLen
            + TypeInfo
            + Debug
            + Copy;
        /// Used to specify the milestones the dispute is being raised on.
        type SpecificId: AtLeast32BitUnsigned
            + FullEncode
            + FullCodec
            + MaxEncodedLen
            + TypeInfo
            + Debug
            + Copy;
        /// This is the max length for specifying the reason while raising the dispute
        type MaxReasonLength: Get<u32>;
        /// This is number of juries that can be assigned to a given dispute
        type MaxJurySize: Get<u32>;
        /// This is number of specifics that can be assigned to a given dispute
        type MaxSpecifics: Get<u32>;
        /// The amount of time a dispute takes to finalise.
        type VotingTimeLimit: Get<<Self as frame_system::Config>::BlockNumber>;
        /// The origin used to force cancel and pass disputes.
        type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// External hooks to handle the completion of a dispute.
        type DisputeHooks: DisputeHooks<Self::DisputeKey>;
    }

    /// Used to store the disputes that is being raised, given the dispute key it returns the Dispute
    /// Key: DisputeKey
    /// Value: Dispute<T>
    #[pallet::storage]
    #[pallet::getter(fn disputes)]
    pub type Disputes<T: Config> =
        StorageMap<_, Blake2_128Concat, T::DisputeKey, Dispute<T>, OptionQuery>;

    /// Stores the dispute keys that will finalise on a given block.
    /// Key: BlockNumber
    /// Value: Vec<DisputeKey>
    #[pallet::storage]
    pub type DisputesFinaliseOn<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<T::DisputeKey, ConstU32<1000>>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A dispute has been raised.
        DisputeRaised {
            dispute_key: T::DisputeKey,
        },
        /// A disute has been voted on
        DisputeVotedOn {
            who: AccountIdOf<T>,
        },
        /// A dispute has been completed.
        // TODO: Not in use
        DisputeCompleted {
            dispute_key: T::DisputeKey,
        },
        /// A dispute has been cancelled.
        // TODO: Not in use
        DisputeCancelled {
            dispute_key: T::DisputeKey,
        },
        /// A dispute has been extended.
        DisputeExtended {
            dispute_key: T::DisputeKey,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Dispute key does not exist.
        DisputeDoesNotExist,
        /// Dispute key already exists.
        DisputeAlreadyExists,
        // This account is not part of the specified jury.
        NotAJuryAccount,
        /// There have been too many disputes on this block. Try next block.
        TooManyDisputesThisBlock,
        /// The dispute has already been extended. You can only extend a dispute once.
        DisputeAlreadyExtended,
        /// There have been more than required votes for a given dispute
        TooManyDisputeVotes,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        // TODO: WEIGHT + BENCHMARKS
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            let expiring_disputes = DisputesFinaliseOn::<T>::take(n);
            expiring_disputes.iter().for_each(|dispute_id| {
                if let Some(dispute) = Disputes::<T>::get(dispute_id) {
                    let result = dispute.calculate_winner();
                    // TODO: Gonna have to do a trick to benchmark this correctly.
                    // Maybe return a weight from the method is a good idea and simple.
                    let _ = <T::DisputeHooks as DisputeHooks<T::DisputeKey>>::on_dispute_complete(
                        *dispute_id,
                        result,
                    );

                    Self::deposit_event(Event::<T>::DisputeCompleted { dispute_key: *dispute_id });
                }
            });
            Weight::default()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        /// Vote on a dispute that already exists.
        /// If all votes are unanimous and everyone has voted, the dispute is autofinalised.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::vote_on_dispute())]
        pub fn vote_on_dispute(
            origin: OriginFor<T>,
            dispute_key: T::DisputeKey,
            is_yay: bool,
        ) -> DispatchResult {
            let mut total_jury = 0;
            let who = ensure_signed(origin)?;
            let votes = Disputes::<T>::try_mutate(dispute_key, |dispute| {
                if let Some(d) = dispute {
                    total_jury = d.jury.len();
                    ensure!(
                        d.jury.iter().any(|e| e == &who.clone()),
                        Error::<T>::NotAJuryAccount
                    );

                    d.votes
                        .try_insert(who.clone(), is_yay)
                        .map_err(|_| Error::<T>::TooManyDisputeVotes)?;

                    //TODO: This is kinda messy, ideally we dont want to clone such a big data set.
                    Ok::<BoundedBTreeMap<AccountIdOf<T>, bool, T::MaxJurySize>, DispatchError>(
                        d.votes.clone(),
                    )
                } else {
                    Err(Error::<T>::DisputeDoesNotExist.into())
                }
            })?;

            if votes.len() == total_jury {
                if votes.iter().all(|v|*v.1) {
                    Dispute::<T>::try_finalise_with_result(dispute_key, DisputeResult::Success)?;
                }
    
                if votes.iter().all(|v|!*v.1) {
                    Dispute::<T>::try_finalise_with_result(dispute_key, DisputeResult::Failure)?;
                }
            }
            
            Self::deposit_event(Event::<T>::DisputeVotedOn { who });
            Ok(().into())
        }

        /// Force a dispute to fail. 
        /// Must be called by T::ForceOrigin
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::force_fail_dispute())]
        pub fn force_fail_dispute(
            origin: OriginFor<T>,
            dispute_key: T::DisputeKey,
        ) -> DispatchResult {
            todo!();
            Ok(().into())
        }

        /// Force a dispute to pass.
        /// Must be called by T::ForceOrigin.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::force_succeed_dispute())]
        pub fn force_succeed_dispute(
            origin: OriginFor<T>,
            dispute_key: T::DisputeKey,
        ) -> DispatchResult {
            todo!();
            Ok(().into())
        }

        /// Extend a given dispute by T::VotingTimeLimit.
        /// This can only be called once and must be called by a member of the jury.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::extend_dispute())]
        pub fn extend_dispute(origin: OriginFor<T>, dispute_key: T::DisputeKey) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut dispute =
                Disputes::<T>::get(dispute_key).ok_or(Error::<T>::DisputeDoesNotExist)?;
            ensure!(!dispute.is_extended, Error::<T>::DisputeAlreadyExtended);
            ensure!(
                dispute.jury.iter().any(|e| e == &who),
                Error::<T>::NotAJuryAccount
            );
            // Ensure that the dispute does not end on the old date.
            DisputesFinaliseOn::<T>::mutate(dispute.expiration, |finalising| {
                // TODO: see if this works lol
                let _ = finalising
                    .iter()
                    .filter(|finalising_id| **finalising_id == dispute_key)
                    .collect::<Vec<_>>();
            });

            // Insert the new date.
            let new_expiry = dispute.expiration.saturating_add(T::VotingTimeLimit::get());
            DisputesFinaliseOn::<T>::try_mutate(new_expiry, |finalising| {
                finalising
                    .try_push(dispute_key)
                    .map_err(|_| Error::<T>::TooManyDisputesThisBlock)?;
                Ok::<(), DispatchError>(())
            })?;

            // Mutate the expiration date on the dispute itself.
            dispute.expiration = new_expiry;
            dispute.is_extended = true;
            Disputes::<T>::insert(dispute_key, dispute);

            Self::deposit_event(Event::<T>::DisputeExtended { dispute_key });

            Ok(())
        }
    }

    #[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Dispute<T: Config> {
        /// Who this was raised by.
        pub raised_by: AccountIdOf<T>,
        /// The votes of each jury.
        pub votes: BoundedBTreeMap<AccountIdOf<T>, bool, T::MaxJurySize>,
        /// The party responsible for the vote.
        pub jury: BoundedVec<AccountIdOf<T>, <T as Config>::MaxJurySize>,
        /// The specific entities the dispute is raised upon.
        pub specifiers: BoundedVec<T::SpecificId, T::MaxSpecifics>,
        /// Marks wether a dispute has already had its time limit extended.
        pub is_extended: bool,
        /// The expiration block of the dispute.
        pub expiration: BlockNumberFor<T>,
    }

    impl<T: Config> Dispute<T> {
        // Create a new dispute and setup state so that pallet will operate as intended.
        pub(crate) fn new(
            dispute_key: T::DisputeKey,
            raised_by: AccountIdOf<T>,
            jury: BoundedVec<AccountIdOf<T>, T::MaxJurySize>,
            specifiers: BoundedVec<T::SpecificId, T::MaxSpecifics>,
        ) -> Result<(), DispatchError> {
            ensure!(
                !Disputes::<T>::contains_key(dispute_key),
                Error::<T>::DisputeAlreadyExists
            );

            let expiration_block =
                frame_system::Pallet::<T>::block_number().saturating_add(T::VotingTimeLimit::get());
            let dispute = Self {
                raised_by,
                jury,
                votes: Default::default(),
                specifiers,
                is_extended: false,
                expiration: expiration_block,
            };

            Disputes::<T>::insert(dispute_key, dispute);
            DisputesFinaliseOn::<T>::try_mutate(expiration_block, |b_vec| {
                b_vec
                    .try_push(dispute_key)
                    .map_err(|_| Error::<T>::TooManyDisputesThisBlock)?;

                Ok::<(), DispatchError>(())
            })?;

            //::deposit_event(Event::<T>::DisputeRaised { dispute_key }.into());
            Ok(())
        }

        /// Calculate the winner in a dispute.
        pub(crate) fn calculate_winner(&self) -> DisputeResult {
            if self.votes.values().filter(|&&x| x).count()
                >= self.votes.values().filter(|&&x| !x).count()
            {
                DisputeResult::Success
            } else {
                DisputeResult::Failure
            }
        }

        /// Falliably finalise a dispute.
        /// This method will clean up storage associated with a dispute and the dispute itself.
        pub(crate) fn try_finalise_with_result(
            dispute_key: T::DisputeKey,
            result: DisputeResult,
        ) -> Result<(), DispatchError> {
            let dispute =
                Disputes::<T>::take(dispute_key).ok_or(Error::<T>::DisputeDoesNotExist)?;
            DisputesFinaliseOn::<T>::mutate(dispute.expiration, |finalising| {
                let _ = finalising
                    .iter()
                    .filter(|finalising_id| **finalising_id == dispute_key)
                    .collect::<Vec<_>>();
            });
            let _ = T::DisputeHooks::on_dispute_complete(dispute_key, result);
            ///remove the dispute once the hooks gets successfully completed
            Disputes::<T>::remove(dispute_key);
            Ok(())
        }
    }
    pub enum DisputeResult {
        Success,
        Failure,
    }

    pub trait WeightInfoT {
        fn vote_on_dispute() -> Weight;
        fn force_cancel_dispute() -> Weight;
        fn extend_dispute() -> Weight;
        fn raise_dispute() -> Weight;
        fn on_dispute_complete() -> Weight;
        fn on_dispute_cancel() -> Weight;
        fn force_succeed_dispute() -> Weight;
        fn force_fail_dispute() -> Weight;
    }
}
