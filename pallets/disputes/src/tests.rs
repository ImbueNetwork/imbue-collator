use crate::traits::*;
use crate::{mock::*, pallet, pallet::*};
use frame_support::traits::Len;
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use sp_arithmetic::traits::One;
use sp_runtime::{BoundedVec, Saturating};
use test_utils::*;

mod test_utils {
    use super::*;

    pub fn run_to_block<T: Config>(n: T::BlockNumber)
        where
            T::BlockNumber: Into<u64>,
    {
        loop {
            let mut block: T::BlockNumber = frame_system::Pallet::<T>::block_number();
            if block >= n {
                break;
            }
            block = block.saturating_add(<T::BlockNumber as One>::one());
            frame_system::Pallet::<T>::set_block_number(block);
            frame_system::Pallet::<T>::on_initialize(block);
            PalletDisputes::on_initialize(block.into());
        }
    }

    pub fn get_jury<T: Config>(
        accounts: Vec<AccountIdOf<T>>,
    ) -> BoundedVec<AccountIdOf<T>, <T as Config>::MaxJurySize> {
        accounts.try_into().expect("too many jury members")
    }

    pub fn get_specifics<T: Config>(
        specifics: Vec<T::SpecificId>,
    ) -> BoundedVec<T::SpecificId, T::MaxSpecifics> {
        specifics.try_into().expect("too many specific ids.")
    }
}

#[test]
fn raise_dispute_assert_state() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*CHARLIE, *BOB]);
        let specifics = get_specifics::<Test>(vec![0, 1]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specifics,
        ));
        assert!(PalletDisputes::disputes(dispute_key).is_some());
        assert_eq!(1, PalletDisputes::disputes(dispute_key).iter().count());
    });
}

#[test]
fn raise_dispute_assert_event() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*CHARLIE, *BOB]);
        let specifics = get_specifics::<Test>(vec![0, 1]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specifics,
        ));
        System::assert_last_event(RuntimeEvent::PalletDisputes(
            Event::<Test>::DisputeRaised { dispute_key: dispute_key },
        ));
    });
}


///testing when trying to insert more than max number of disputes allowed in a block
#[test]
fn raise_dispute_assert_event_too_many_disputes() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        (0..=1000).for_each(|i| {
            let jury = get_jury::<Test>(vec![*CHARLIE, *BOB]);
            let specifics = get_specifics::<Test>(vec![0, 1]);
            if i != 1000 {
                assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            i,
            *ALICE,
            jury,
            specifics,
              ));
                System::assert_last_event(RuntimeEvent::PalletDisputes(
                    Event::<Test>::DisputeRaised { dispute_key: i },
                ));
            } else {
                let actual_result = <PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
                    i,
                    *ALICE,
                    jury,
                    specifics,
                );
                assert_noop!(actual_result,Error::<Test>::TooManyDisputesThisBlock);
            }
        });
    });
}

#[test]
fn raise_dispute_already_exists() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*CHARLIE, *BOB]);
        let specifics = get_specifics::<Test>(vec![0, 1]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury.clone(),
            specifics.clone(),
        ));
        assert_noop!(
            <PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
                dispute_key,
                *ALICE,
                jury,
                specifics
            ),
            Error::<Test>::DisputeAlreadyExists
        );
    });
}

#[test]
fn vote_on_dispute_assert_state() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*CHARLIE, *BOB]);
        let specifics = get_specifics::<Test>(vec![0, 1]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specifics,
        ));
        let dispute_before_vote = Disputes::<Test>::get(dispute_key).expect("dispute should exist");
        assert_eq!(0, dispute_before_vote.votes.len());
        assert_ok!(PalletDisputes::vote_on_dispute(
            RuntimeOrigin::signed(*BOB),
            dispute_key,
            true
        ));
        let dispute_after_vote = Disputes::<Test>::get(dispute_key).expect("dispute should exist");
        let vote = dispute_after_vote.votes.get(&BOB).unwrap();
        assert_eq!(true, *vote);
        assert_eq!(1, dispute_after_vote.votes.len());
    });
}

// FELIX: shankar what does this mean? ^^
//SHANKAR: Just telling when auto finalization comes we could extend by making more calls to check unanimous voting
//But i think we have covered in the new test cases so we can ignore the comments above
#[test]
fn vote_on_dispute_assert_last_event() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*CHARLIE, *BOB]);
        let specifics = get_specifics::<Test>(vec![0, 1]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specifics,
        ));

        assert_ok!(PalletDisputes::vote_on_dispute(
            RuntimeOrigin::signed(*BOB),
            dispute_key,
            true
        ));
        System::assert_last_event(RuntimeEvent::PalletDisputes(
            Event::<Test>::DisputeVotedOn { who: *BOB },
        ));
    });
}

#[test]
fn vote_on_dispute_autofinalises_on_unanimous_yes() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*CHARLIE, *BOB]);
        let specifics = get_specifics::<Test>(vec![0, 1]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specifics,
        ));
        assert_ok!(PalletDisputes::vote_on_dispute(
            RuntimeOrigin::signed(*BOB),
            dispute_key,
            true
        ));
        assert_ok!(PalletDisputes::vote_on_dispute(
            RuntimeOrigin::signed(*CHARLIE),
            dispute_key,
            true
        ));
        //verify that the dispute has been removed once auto_finalization is done in case of unanimous yes
        assert_eq!(0, PalletDisputes::disputes(dispute_key).iter().count());
    });
}

#[test]
fn vote_on_dispute_autofinalises_on_unanimous_no() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*CHARLIE, *BOB]);
        let specifics = get_specifics::<Test>(vec![0, 1]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specifics,
        ));
        assert_ok!(PalletDisputes::vote_on_dispute(
            RuntimeOrigin::signed(*BOB),
            dispute_key,
            false
        ));
        assert_ok!(PalletDisputes::vote_on_dispute(
            RuntimeOrigin::signed(*CHARLIE),
            dispute_key,
            false
        ));
        //verify that the dispute has been removed once auto_finalization is done in case of unanimous no
        assert_eq!(0, PalletDisputes::disputes(dispute_key).iter().count());
    });
}

///SHANKAR: What does this mean?
#[test]
fn try_auto_finalise_removes_autofinalise() {
    new_test_ext().execute_with(|| {
        new_test_ext().execute_with(|| {
            let dispute_key = 10;
            let jury = get_jury::<Test>(vec![*CHARLIE, *BOB]);
            let specifics = get_specifics::<Test>(vec![0, 1]);
            assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specifics,
        ));
            assert_ok!(PalletDisputes::vote_on_dispute(
            RuntimeOrigin::signed(*BOB),
            dispute_key,
            false
        ));
            assert_ok!(PalletDisputes::vote_on_dispute(
            RuntimeOrigin::signed(*CHARLIE),
            dispute_key,
            false
        ));
            //verify that the dispute has been removed once auto_finalization is done in case of unanimous no
            assert_eq!(0, PalletDisputes::disputes(dispute_key).iter().count());
            //After the dispute has been autofinalized and the we again tru to autofinalize it throws an error saying that
            // the dispute doesnt exists as it has been removed
            assert_noop!(Dispute::<Test>::try_finalise_with_result(dispute_key,DisputeResult::Success),Error::<Test>::DisputeDoesNotExist);
        });
    });
}

///testing if the non jury account tries to vote it should throw the error saying its not a jury account
#[test]
fn vote_on_dispute_not_jury_account() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*BOB]);
        let specific_ids = get_specifics::<Test>(vec![0]);

        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specific_ids
        ));
        assert_noop!(
            PalletDisputes::vote_on_dispute(RuntimeOrigin::signed(*CHARLIE), dispute_key, true),
            Error::<Test>::NotAJuryAccount
        );
    });
}

///trying to vote on a dispute that doesn't exists which result in the error throwing dispute does not exists
#[test]
fn vote_on_dispute_dispute_doesnt_exist() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*BOB]);
        let specific_ids = get_specifics::<Test>(vec![0]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specific_ids
        ));

        assert_noop!(
            PalletDisputes::vote_on_dispute(RuntimeOrigin::signed(*BOB), 1, true),
            Error::<Test>::DisputeDoesNotExist
        );
    });
}

///trying to extend the voting time  on a dispute that doesn't exists which result in the error throwing dispute does not exists
#[test]
fn extend_dispute_dispute_doesnt_exist() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*BOB]);
        let specific_ids = get_specifics::<Test>(vec![0]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specific_ids
        ));
        assert_noop!(
            PalletDisputes::extend_dispute(RuntimeOrigin::signed(*BOB), 1),
            Error::<Test>::DisputeDoesNotExist
        );
    });
}

///testing to extend the time for voting from a not jury account, it should throw the error saying its not a jury account
#[test]
fn extend_dispute_not_a_jury_account() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*BOB]);
        let specific_ids = get_specifics::<Test>(vec![0]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specific_ids
        ));
        assert_noop!(
            PalletDisputes::extend_dispute(RuntimeOrigin::signed(*CHARLIE), dispute_key),
            Error::<Test>::NotAJuryAccount
        );
    });
}

/// testing trying to extend the voting on a dispute which has already been extended and should throw Dispute Already Extended error
#[test]
fn extend_dispute_already_extended() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*BOB]);
        let specific_ids = get_specifics::<Test>(vec![0]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specific_ids
        ));
        assert_ok!(PalletDisputes::extend_dispute(
            RuntimeOrigin::signed(*BOB),
            dispute_key
        ));
        assert_noop!(
            PalletDisputes::extend_dispute(RuntimeOrigin::signed(*BOB), dispute_key),
            Error::<Test>::DisputeAlreadyExtended
        );
    });
}

/// testing trying to extend the voting time and it successfully extend by setting the flag to true
#[test]
fn extend_dispute_works_assert_last_event() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*BOB]);
        let specific_ids = get_specifics::<Test>(vec![0]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specific_ids
        ));
        let d = Disputes::<Test>::get(dispute_key).expect("dispute should exist");
        assert!(!d.is_extended);
        assert_ok!(PalletDisputes::extend_dispute(
            RuntimeOrigin::signed(*BOB),
            10
        ));
        System::assert_last_event(RuntimeEvent::PalletDisputes(
            Event::<Test>::DisputeExtended { dispute_key: dispute_key },
        ));
    });
}

#[test]
fn extend_dispute_works_assert_state() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*BOB]);
        let specific_ids = get_specifics::<Test>(vec![0]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specific_ids
        ));
        let d = Disputes::<Test>::get(dispute_key).expect("dispute should exist");
        assert!(!d.is_extended);
        assert_eq!(11, d.expiration);
        assert_ok!(PalletDisputes::extend_dispute(
            RuntimeOrigin::signed(*BOB),
            10
        ));
        let d = Disputes::<Test>::get(dispute_key).expect("dispute should exist");
        assert!(d.is_extended);
        assert_eq!(21, d.expiration);
    });
}

#[test]
fn calculate_winner_works() {
    new_test_ext().execute_with(|| {});
}


///e2e
#[test]
fn e2e() {
    new_test_ext().execute_with(|| {
        let dispute_key = 10;
        let jury = get_jury::<Test>(vec![*CHARLIE, *BOB]);
        let specifics = get_specifics::<Test>(vec![0, 1]);
        assert_ok!(<PalletDisputes as DisputeRaiser<AccountId>>::raise_dispute(
            dispute_key,
            *ALICE,
            jury,
            specifics,
        ));
        assert_ok!(PalletDisputes::vote_on_dispute(
            RuntimeOrigin::signed(*BOB),
            dispute_key,
            true
        ));
        assert_ok!(PalletDisputes::vote_on_dispute(
            RuntimeOrigin::signed(*CHARLIE),
            dispute_key,
            true
        ));
        //verify that the dispute has been removed once auto_finalization is done in case of unanimous yes
        assert_eq!(0, PalletDisputes::disputes(dispute_key).iter().count());
    });
}

