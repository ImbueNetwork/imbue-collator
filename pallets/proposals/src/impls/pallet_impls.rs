use crate::*;
use scale_info::prelude::format;
use sp_runtime::traits::{Saturating, Zero};
use pallet_disputes::{
    traits::DisputeHooks,
    DisputeResult,
};

impl<T: Config> Pallet<T> {
    /// The account ID of the fund pot.
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn project_account_id(key: ProjectKey) -> AccountIdOf<T> {
        T::PalletId::get().into_sub_account_truncating(format!("//{key}"))
    }

    // Take a project and submit an associated milestone.
    pub(crate) fn new_milestone_submission(
        who: T::AccountId,
        project_key: ProjectKey,
        milestone_key: MilestoneKey,
    ) -> DispatchResultWithPostInfo {
        let project = Projects::<T>::get(project_key).ok_or(Error::<T>::ProjectDoesNotExist)?;

        ensure!(project.initiator == who, Error::<T>::UserIsNotInitiator);
        let milestone = project
            .milestones
            .get(&milestone_key)
            .ok_or(Error::<T>::MilestoneDoesNotExist)?;
        ensure!(!milestone.is_approved, Error::<T>::MilestoneAlreadyApproved);

        let expiry_block =
            <T as Config>::MilestoneVotingWindow::get() + frame_system::Pallet::<T>::block_number();
        Rounds::<T>::insert(
            (project_key, milestone_key),
            RoundType::VotingRound,
            expiry_block,
        );
        RoundsExpiring::<T>::try_mutate(expiry_block, |keys| {
            keys.try_push((project_key, RoundType::VotingRound, milestone_key))
                .map_err(|_| Error::<T>::Overflow)?;
            Ok::<(), DispatchError>(())
        })?;

        IndividualVoteStore::<T>::try_mutate(project_key, |maybe_votes| {
            if let Some(individual_votes) = maybe_votes {
                individual_votes.clear_milestone_votes(milestone_key);
            } else {
                return Err(Error::<T>::IndividualVoteNotFound.into());
            };
            Ok::<(), DispatchError>(())
        })?;

        MilestoneVotes::<T>::try_mutate(project_key, |vote_btree| {
            vote_btree
                .try_insert(milestone_key, Vote::default())
                .map_err(|_| Error::<T>::TooManyMilestoneVotes)?;

            Ok::<(), DispatchError>(())
        })?;

        Self::deposit_event(Event::MilestoneSubmitted(who, project_key, milestone_key));
        Self::deposit_event(Event::VotingRoundCreated(project_key));
        Ok(().into())
    }

    pub(crate) fn new_milestone_vote(
        who: T::AccountId,
        project_key: ProjectKey,
        milestone_key: MilestoneKey,
        approve_milestone: bool,
    ) -> DispatchResultWithPostInfo {
        let project = Projects::<T>::get(project_key).ok_or(Error::<T>::ProjectDoesNotExist)?;
        ensure!(
            Rounds::<T>::contains_key((project_key, milestone_key), RoundType::VotingRound),
            Error::<T>::VotingRoundNotStarted
        );
        ensure!(
            !ProjectsInDispute::<T>::get(project_key).contains(&milestone_key),
            Error::<T>::MilestonesAlreadyInDispute
        );
        
        let contribution_amount = project
            .contributions
            .get(&who)
            .ok_or(Error::<T>::OnlyContributorsCanVote)?
            .value;
        let now = frame_system::Pallet::<T>::block_number();
        let user_has_voted_key = (project_key, RoundType::VotingRound, milestone_key);

        IndividualVoteStore::<T>::try_mutate(project_key, |maybe_individual_votes| {
            if let Some(individual_votes) = maybe_individual_votes {
                individual_votes.insert_individual_vote(milestone_key, &who, approve_milestone)?;
            }
            Ok::<(), DispatchError>(())
        })?;

        let vote: Vote<BalanceOf<T>> =
            MilestoneVotes::<T>::try_mutate(project_key, |vote_btree| {
                if let Some(vote) = vote_btree.get_mut(&milestone_key) {
                    if approve_milestone {
                        vote.yay = vote.yay.saturating_add(contribution_amount);
                    } else {
                        vote.nay = vote.nay.saturating_add(contribution_amount);
                    }
                    Ok::<Vote<BalanceOf<T>>, DispatchError>(vote.clone())
                } else {
                    Err(Error::<T>::VotingRoundNotStarted.into())
                }
            })?;

        let funding_threshold: BalanceOf<T> =
            T::PercentRequiredForVoteToPass::get().mul_floor(project.raised_funds);

        Self::try_auto_finalise_milestone_voting(
            project_key,
            &vote,
            funding_threshold,
            user_has_voted_key,
            who.clone(),
        )?;

        Self::deposit_event(Event::VoteSubmitted(
            who,
            project_key,
            milestone_key,
            approve_milestone,
            now,
        ));
        Ok(().into())
    }

    pub(crate) fn new_withdrawal(
        who: T::AccountId,
        project_key: ProjectKey,
    ) -> DispatchResultWithPostInfo {
        let project = Projects::<T>::get(project_key).ok_or(Error::<T>::ProjectDoesNotExist)?;

        ensure!(!project.cancelled, Error::<T>::ProjectWithdrawn);
        ensure!(who == project.initiator, Error::<T>::UserIsNotInitiator);

        // Collect and calculate the amount that can be withdrawn.
        let mut unlocked_funds: BalanceOf<T> = Zero::zero();
        for (_, ms) in project.milestones.iter() {
            if ms.is_approved {
                let per_milestone = ms.percentage_to_unlock.mul_floor(project.raised_funds);
                unlocked_funds = unlocked_funds.saturating_add(per_milestone);
            }
        }

        let withdrawable: BalanceOf<T> = unlocked_funds.saturating_sub(project.withdrawn_funds);
        ensure!(
            withdrawable != Zero::zero(),
            Error::<T>::NoAvailableFundsToWithdraw
        );

        let fee = <T as Config>::ImbueFee::get().mul_floor(withdrawable);
        let withdrawn = withdrawable.saturating_sub(fee);
        let project_account = Self::project_account_id(project_key);

        // Take the fee and send to ImbueFeeAccount   
        T::MultiCurrency::transfer(
            project.currency_id,
            &project_account,
            &<T as Config>::ImbueFeeAccount::get(),
            fee,
        )?;

        // Transfer to initiator
        T::MultiCurrency::transfer(
            project.currency_id,
            &project_account,
            &project.initiator,
            withdrawn,
        )?;

        // Remove the project if the funds left are 0.
        Projects::<T>::mutate_exists(project_key, |project| -> DispatchResult {
            if let Some(p) = project {
                p.withdrawn_funds = p.withdrawn_funds.saturating_add(withdrawable);
                if p.withdrawn_funds == p.raised_funds {
                    <T as Config>::DepositHandler::return_deposit(p.deposit_id)?;
                    CompletedProjects::<T>::try_mutate(
                        &p.initiator,
                        |completed_projects| -> DispatchResult {
                            completed_projects
                                .try_push(project_key)
                                .map_err(|_| Error::<T>::TooManyProjects)?;
                            Ok(())
                        },
                    )?;
                    *project = None;
                }
            }
            Ok(())
        })?;

        Self::deposit_event(Event::ProjectFundsWithdrawn(
            who,
            project_key,
            withdrawn,
            project.currency_id,
        ));

        Ok(().into())
    }

    /// Try and fund a project based on its FundingPath.
    /// Will error is the 
    /// If the funds have actually been transferred this will return and Ok(true)
    /// If the funds have not been transferred (i.e awaiting funding) then it will return Ok(false)
    pub(crate) fn fund_project<'a>(
        funding_path: &'a FundingPath,
        contributions: &'a ContributionsFor<T>,
        project_account_id: &'a T::AccountId,
        currency_id: CurrencyId,
    ) -> Result<bool, DispatchError> {
        match *funding_path {
            FundingPath::TakeFromReserved => {
                for (acc, cont) in contributions.iter() {
                    <<T as Config>::MultiCurrency as MultiReservableCurrency<
                        AccountIdOf<T>,
                    >>::unreserve(currency_id, acc, cont.value);
                    <T as Config>::MultiCurrency::transfer(
                        currency_id,
                        acc,
                        project_account_id,
                        cont.value,
                    )?;
                }
                Ok(true)
            }
            FundingPath::WaitForFunding => Ok(false),
        }
    }

    /// Try and convert some proposed milestones to milestones.
    /// Will never fail so long as proposed_milestones and BoundedBTreeMilestones<T> have the same bound.
    pub(crate) fn try_convert_to_milestones(
        proposed_milestones: BoundedVec<ProposedMilestone, T::MaxMilestonesPerProject>,
        project_key: ProjectKey,
    ) -> Result<BoundedBTreeMilestones<T>, DispatchError> {
        let mut milestone_key: u32 = 0;
        let mut milestones: BoundedBTreeMilestones<T> = BoundedBTreeMap::new();
        for milestone in proposed_milestones {
            let milestone = Milestone {
                project_key,
                milestone_key,
                percentage_to_unlock: milestone.percentage_to_unlock,
                is_approved: false,
                can_refund: false,
                is_refunded: false,
            };
            milestones
                .try_insert(milestone_key, milestone)
                .map_err(|_| Error::<T>::TooManyMilestones)?;
            milestone_key = milestone_key.saturating_add(1);
        }
        Ok(milestones)
    }

    pub(crate) fn try_auto_finalise_milestone_voting(
        project_key: ProjectKey,
        vote: &Vote<BalanceOf<T>>,
        funding_threshold: BalanceOf<T>,
        user_has_voted_key: (ProjectKey, RoundType, MilestoneKey),
        who: AccountIdOf<T>,
    ) -> Result<(), DispatchError> {
        // If the yay votes is over the funding threshold then the milestone is approved.
        if vote.yay >= funding_threshold {
            Projects::<T>::mutate(project_key, |maybe_project| {
                if let Some(p) = maybe_project {
                    if let Some(ms) = p.milestones.get_mut(&user_has_voted_key.2) {
                        ms.is_approved = true
                    }
                }
            });

            Self::close_voting_round(project_key, user_has_voted_key)?;

            Self::deposit_event(Event::MilestoneApproved(
                who,
                project_key,
                user_has_voted_key.2,
                <frame_system::Pallet<T>>::block_number(),
            ));
        }

        if vote.nay >= funding_threshold {
            Self::close_voting_round(project_key, user_has_voted_key)?;
            Self::deposit_event(Event::MilestoneRejected(
                user_has_voted_key.0,
                user_has_voted_key.2,
            ));
        }
        Ok(())
    }

    pub(crate) fn close_voting_round(
        project_key: ProjectKey,
        user_has_voted_key: (ProjectKey, RoundType, MilestoneKey),
    ) -> Result<(), DispatchError> {
        // Prevent further voting.
        let exp_block =
            Rounds::<T>::take((project_key, user_has_voted_key.2), RoundType::VotingRound)
                .ok_or(Error::<T>::VotingRoundNotStarted)?;
        // Prevent hook from calling.
        RoundsExpiring::<T>::remove(exp_block);
        // Allow future votes to occur on this milestone
        IndividualVoteStore::<T>::try_mutate(project_key, |maybe_individual_votes| {
            if let Some(individual_votes) = maybe_individual_votes {
                individual_votes.clear_milestone_votes(user_has_voted_key.2);
            } else {
                return Err(Error::<T>::IndividualVoteNotFound.into());
            }
            Ok::<(), DispatchError>(())
        })?;

        Ok(())
    }


}

impl<T: Config> DisputeHooks<ProjectKey, MilestoneKey> for Pallet<T> {
    fn on_dispute_complete(
        project_key: ProjectKey,
        specifics: Vec<MilestoneKey>,
        dispute_result: pallet_disputes::pallet::DisputeResult,
    ) -> Weight {
        let mut weight: Weight = <Weight as Zero>::zero();
        weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
        ProjectsInDispute::<T>::remove(project_key);

        Projects::<T>::mutate(project_key, |maybe_project|{
                match maybe_project {
                    Some(project) => {
                    match dispute_result {
                        DisputeResult::Success => {
                            for milestone_key in specifics.iter() {
                                if let Some(milestone) = project.milestones.get_mut(milestone_key) {
                                    milestone.can_refund = true;
                                }
                            }
                        },
                        DisputeResult::Failure => {

                        // OnFailure
                        // should not modify the state of the project except perhaps recording the amount of disputes previosuly handled.
                        // Emit event for failure?? check if pallet_disputes does this.
                        // revert anything that has happened as a result of calling a dispute (should be nothing as the 2 are independant.)
                        },
                    };
                    weight
                },
                // Looks like the project was deleted somehow during the dispute. 
                // The only way this is possible is through a refund or final withdraw.
                // Not a massive issue as either way the project has been finalised.
                // Just ignore and return weight.
                None => {
                    weight
                }
            }
        })
    }
}
