use crate::mock::*;
use crate::test_utils::gen_grant_id;
use crate::tests::{get_approvers, get_milestones};
use common_types::{CurrencyId, TreasuryOrigin};
use frame_support::assert_ok;
use pallet_projects::Projects;

#[test]
fn create_project_from_grant() {
    new_test_ext().execute_with(|| {
        let grant_id = gen_grant_id(100);
        let contribution_value: Balance = 10_000;

        assert_ok!(Grant::submit_initial_grant(
            RuntimeOrigin::signed(*ALICE),
            get_milestones(10),
            get_approvers(10),
            CurrencyId::Native,
            contribution_value,
            TreasuryOrigin::Imbue,
            grant_id,
        ));
        assert_ok!(Grant::convert_to_project(
            RuntimeOrigin::signed(*ALICE),
            grant_id
        ));
        assert!(Projects::<Test>::get(1).is_some());
    });
}
