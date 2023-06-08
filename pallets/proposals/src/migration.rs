
use crate::*;
use frame_support::{pallet_prelude::OptionQuery, storage_alias, traits::Get, weights::Weight};
pub use pallet::*;
pub type TimestampOf<T> = <T as pallet_timestamp::Config>::Moment;

#[allow(unused)]
mod v0 {
    use super::*;
    pub type ProjectV0Of<T> = ProjectV0<AccountIdOf<T>, BalanceOf<T>, BlockNumberFor<T>>;

    #[derive(Encode, Clone, Decode, Debug)]
    pub struct MilestoneV0 {
        pub project_key: u32,
        pub milestone_key: u32,
        pub name: Vec<u8>,
        pub percentage_to_unlock: u32,
        pub is_approved: bool,
    }

    #[derive(Encode, Clone, Decode)]
    pub struct ContributionV0<AccountId, Balance> {
        pub account_id: AccountId,
        pub value: Balance,
    }

    #[derive(Encode, Clone, Decode)]
    pub struct ProjectV0<AccountId, Balance, BlockNumber> {
        pub name: Vec<u8>,
        pub logo: Vec<u8>,
        pub description: Vec<u8>,
        pub website: Vec<u8>,
        pub milestones: Vec<MilestoneV0>,
        /// A collection of the accounts which have contributed and their contributions.
        pub contributions: Vec<ContributionV0<AccountId, Balance>>,
        pub currency_id: common_types::CurrencyId,
        pub required_funds: Balance,
        pub withdrawn_funds: Balance,
        /// The account that will receive the funds if the campaign is successful
        pub initiator: AccountId,
        pub create_block_number: BlockNumber,
        pub approved_for_funding: bool,
        pub funding_threshold_met: bool,
        pub cancelled: bool,
    }

    #[storage_alias]
    pub type Projects<T: Config> =
        StorageMap<Pallet<T>, Identity, ProjectKey, ProjectV0Of<T>, OptionQuery>;
}

#[allow(unused)]
mod v1 {
    use super::*;
    use crate::migration::v0::MilestoneV0;

    #[derive(Encode, Clone, Decode)]
    pub struct ProjectV1<AccountId, Balance, BlockNumber, Timestamp> {
        pub name: Vec<u8>,
        pub logo: Vec<u8>,
        pub description: Vec<u8>,
        pub website: Vec<u8>,
        pub milestones: BTreeMap<MilestoneKey, MilestoneV0>,
        pub contributions: BTreeMap<AccountId, Contribution<Balance, Timestamp>>,
        pub currency_id: common_types::CurrencyId,
        pub required_funds: Balance,
        pub withdrawn_funds: Balance,
        pub raised_funds: Balance,
        pub initiator: AccountId,
        pub create_block_number: BlockNumber,
        pub approved_for_funding: bool,
        pub funding_threshold_met: bool,
        pub cancelled: bool,
    }

    pub type ProjectV1Of<T> =
        ProjectV1<AccountIdOf<T>, BalanceOf<T>, BlockNumberFor<T>, TimestampOf<T>>;

    #[storage_alias]
    pub type Projects<T: Config> =
        StorageMap<Pallet<T>, Identity, ProjectKey, ProjectV1Of<T>, OptionQuery>;

    pub fn migrate<T: Config>() -> Weight {
        let mut weight = T::DbWeight::get().reads_writes(1, 1);

        v1::Projects::<T>::translate(|_project_key, project: v0::ProjectV0Of<T>| {

            weight += T::DbWeight::get().reads_writes(1, 1);

            let mut migrated_contributions: BTreeMap<
                AccountIdOf<T>,
                Contribution<BalanceOf<T>, TimestampOf<T>>,
            > = BTreeMap::new();
            let mut migrated_milestones: BTreeMap<MilestoneKey, MilestoneV0> = BTreeMap::new();
            let mut raised_funds: BalanceOf<T> = (0u32).into();
            let _ = project
                .contributions
                .into_iter()
                .map(|contribution| {
                    let migrated_contribution = Contribution {
                        value: contribution.value,
                        timestamp: TimestampOf::<T>::default(),
                    };
                    migrated_contributions.insert(contribution.account_id, migrated_contribution);
                    raised_funds += contribution.value
                })
                .collect::<Vec<_>>();

            let _ = project
                .milestones
                .into_iter()
                .map(|milestone| migrated_milestones.insert(milestone.milestone_key, milestone))
                .collect::<Vec<_>>();

            let migrated_project: ProjectV1<
                T::AccountId,
                BalanceOf<T>,
                T::BlockNumber,
                TimestampOf<T>,
            > = ProjectV1 {
                name: project.name,
                logo: project.logo,
                description: project.description,
                website: project.website,
                milestones: migrated_milestones,
                contributions: migrated_contributions,
                required_funds: project.required_funds,
                currency_id: project.currency_id,
                withdrawn_funds: project.withdrawn_funds,
                initiator: project.initiator,
                create_block_number: project.create_block_number,
                approved_for_funding: project.approved_for_funding,
                funding_threshold_met: project.funding_threshold_met,
                cancelled: project.cancelled,
                raised_funds,
            };
            Some(migrated_project)
        });
        weight
    }
}

mod v2 {
    use super::*;

    #[storage_alias]
    pub type Projects<T: Config> =
        StorageMap<Pallet<T>, Identity, ProjectKey, ProjectV2Of<T>, OptionQuery>;

    pub type ProjectV2Of<T> =
        ProjectV2<AccountIdOf<T>, BalanceOf<T>, BlockNumberFor<T>, TimestampOf<T>>;

    #[derive(Encode, Clone, Decode, TypeInfo)]
    pub struct ProjectV2<AccountId, Balance, BlockNumber, Timestamp> {
        pub agreement_hash: H256,
        pub milestones: BTreeMap<MilestoneKey, MilestoneV1>,
        pub contributions: BTreeMap<AccountId, Contribution<Balance, Timestamp>>,
        pub currency_id: common_types::CurrencyId,
        pub required_funds: Balance,
        pub withdrawn_funds: Balance,
        pub raised_funds: Balance,
        pub initiator: AccountId,
        pub created_on: BlockNumber,
        pub approved_for_funding: bool,
        pub funding_threshold_met: bool,
        pub cancelled: bool,
        pub funding_type: FundingType,
    }

    #[derive(Clone, Debug, Encode, Decode, TypeInfo)]
    pub struct MilestoneV1 {
        pub project_key: ProjectKey,
        pub milestone_key: MilestoneKey,
        pub percentage_to_unlock: u32,
        pub is_approved: bool,
    }

    pub fn migrate<T: Config + pallet_timestamp::Config>() -> Weight {
        let mut weight = T::DbWeight::get().reads_writes(1, 1);
        let mut migrated_milestones: BTreeMap<MilestoneKey, MilestoneV1> = BTreeMap::new();
        v2::Projects::<T>::translate(|_project_key, project: v1::ProjectV1Of<T>| {
            let _ = project
                .milestones
                .into_values()
                .map(|milestone| {
                    let migrated_milestone = MilestoneV1 {
                        project_key: milestone.project_key,
                        milestone_key: milestone.milestone_key,
                        percentage_to_unlock: milestone.percentage_to_unlock,
                        is_approved: milestone.is_approved,
                    };
                    migrated_milestones.insert(milestone.milestone_key, migrated_milestone)
                })
                .collect::<Vec<_>>();

            weight += T::DbWeight::get().reads_writes(1, 1);
            let migrated_project: ProjectV2Of<T> = ProjectV2 {
                milestones: migrated_milestones.clone(),
                contributions: project.contributions,
                required_funds: project.required_funds,
                currency_id: project.currency_id,
                withdrawn_funds: project.withdrawn_funds,
                initiator: project.initiator,
                created_on: project.create_block_number,
                agreement_hash: Default::default(),
                approved_for_funding: project.approved_for_funding,
                funding_threshold_met: project.funding_threshold_met,
                cancelled: project.cancelled,
                raised_funds: project.raised_funds,
                funding_type: FundingType::Proposal,
            };
            Some(migrated_project)
        });
        weight
    }   
}

// 1. --DONE Use blocknumber instead of timestamp for contribution.
// 2. --DONE Project has had required_funds, approved_for_funding and funding_threshold_met removed.
// 3. --DONE UserVotes storage map needs removing + placing into UserHasVoted
// 4. --DONE Milestone votes is now a double map (project_key, milestone_key) +
// 5. --DONE Rounds is also a DoubleMap
// 6. --DONE Round type has had contribution_round removed
// 7, --DONE percent_to_unlock changed from u32 to Percent. (cuteolaf) 
// 8  --DONE Project new field deposit_id 
// 9, --DONE Binding Contrbutions and milestones in project.
pub mod v3 {    
    use super::*;

    #[derive(Encode, Decode, Clone)]
    pub struct ProjectV3<AccountId, Balance, BlockNumber> {
        pub agreement_hash: H256,
        pub milestones: BTreeMap<MilestoneKey, Milestone>,
        pub contributions: BTreeMap<AccountId, Contribution<Balance, BlockNumber>>,
        pub currency_id: common_types::CurrencyId,
        pub withdrawn_funds: Balance,
        pub raised_funds: Balance,
        pub initiator: AccountId,
        pub created_on: BlockNumber,
        pub cancelled: bool,
        pub funding_type: FundingType,
    }

    pub fn migrate_contribution_and_project<T: Config + pallet_timestamp::Config>(
        weight: &mut Weight,
    ) {
        // Migration #1 + #2 + #7 + #8 + #9
        let mut migrated_contributions = BTreeMap::new();
        let mut migrated_milestones = BTreeMap::new();

        Projects::<T>::translate(|_project_key, project: v2::ProjectV2Of<T>| {
            project.contributions.iter().for_each(|(key, cont)| {
                *weight += T::DbWeight::get().reads_writes(1, 1);
                migrated_contributions.insert(
                    key.clone(),
                    Contribution {
                        value: cont.value,
                        timestamp: frame_system::Pallet::<T>::block_number(),
                    },
                );
            });
            project.milestones.iter().for_each(|(key, milestone)| {
                *weight += T::DbWeight::get().reads_writes(1, 1);
                migrated_milestones.insert(
                    key.clone(),
                    Milestone {
                        project_key: milestone.project_key,
                        milestone_key: milestone.milestone_key,
                        percentage_to_unlock: Percent::from_percent(
                            milestone.percentage_to_unlock as u8,
                        ),
                        is_approved: milestone.is_approved,
                    },
                );
            });
            let bounded_milestone: Result<BoundedBTreeMilestones<T>, _> = migrated_milestones.clone().try_into();
            let bounded_contributions: Result<ContributionsFor<T>, _> = migrated_contributions.clone().try_into();
            if let Ok(ms) = bounded_milestone {
                if let Ok(cont) = bounded_contributions {
                    *weight += T::DbWeight::get().reads_writes(1, 1);
                    let migrated_project: Project<T> =
                        Project {
                            milestones: ms,
                            contributions: cont,
                            currency_id: project.currency_id,
                            withdrawn_funds: project.withdrawn_funds,
                            initiator: project.initiator,
                            created_on: project.created_on,
                            agreement_hash: Default::default(),
                            cancelled: project.cancelled,
                            raised_funds: project.raised_funds,
                            funding_type: FundingType::Proposal,
                            // A deposit_id of u32::MAX is ignored.
                            deposit_id: u32::MAX.into(),
                        };
                    Some(migrated_project)
                } else {
                    None
                }
            } else {
                None
            }
        });
    }

    // Migration #3
    #[storage_alias]
    pub type UserVotes<T: Config> = StorageMap<
        Pallet<T>,
        Identity,
        (AccountIdOf<T>, ProjectKey, MilestoneKey, RoundType),
        bool,
        ValueQuery,
    >;
    fn migrate_user_votes<T: Config>(weight: &mut Weight) {
        UserVotes::<T>::translate(|key, value| {
            let (account_id, project_key, milestone_key, round_type) = key;
            let _ = UserHasVoted::<T>::try_mutate(
                (project_key, round_type.into_new(), milestone_key),
                |btree| {
                    // Mutate UserHasVoted per k/v.
                    *weight += T::DbWeight::get().reads_writes(1, 1);
                    // TODO:
                    // If this insert fails it is because the MaxContributors bound has been violated.
                    // Shankar has been working on the bound in the project struct.
                    let _ = btree.try_insert(account_id, value);
                    Ok::<(), ()>(())
                },
            );
            // Mutate UserVotes per k/v.
            *weight += T::DbWeight::get().reads_writes(1, 1);
            None
        });
    }

    // Migration #4
    #[storage_alias]
    pub type OldMilestoneVotes<T: Config> =
        StorageMap<Pallet<T>, Identity, (ProjectKey, MilestoneKey), Vote<BalanceOf<T>>, ValueQuery>;
    
        fn migrate_milestone_votes<T: Config>(weight: &mut Weight) {
        v3::OldMilestoneVotes::<T>::drain().for_each(|(old_key, vote)| {
            *weight += T::DbWeight::get().reads(1);
            let (project_key, milestone_key) = old_key;
            crate::MilestoneVotes::<T>::insert(project_key, milestone_key, vote);
            *weight += T::DbWeight::get().reads_writes(1, 1);
        });

    }

    //Migration #5 + #6
    #[derive(Clone, Encode, Decode, Eq, PartialEq, Debug)]
    pub enum RoundType {
        ContributionRound,
        VotingRound,
        VoteOfNoConfidence,
    }
    impl v3::RoundType {
        pub fn into_new(&self) -> crate::RoundType {
            match &self {
                v3::RoundType::VotingRound => crate::RoundType::VotingRound,
                v3::RoundType::VoteOfNoConfidence => crate::RoundType::VoteOfNoConfidence,
                _ => crate::RoundType::VotingRound,
            }
        }
    }
    #[derive(Encode, Decode, Clone, Debug)]
    pub struct Round<BlockNumber> {
        pub start: BlockNumber,
        pub end: BlockNumber,
        pub project_keys: Vec<ProjectKey>,
        pub round_type: v3::RoundType,
        pub is_canceled: bool,
    }

    #[storage_alias]
    pub type OldRounds<T: pallet::Config> =
        StorageMap<Pallet<T>, Identity, u32, Option<Round<BlockNumberFor<T>>>, ValueQuery>;
    fn migrate_rounds_and_round_type<T: Config>(weight: &mut Weight) {
        
        OldRounds::<T>::translate(|_, r: Option<Round<BlockNumberFor<T>>>| {
            if let Some(round) = r {
                if !round.is_canceled
                    && round.end >= frame_system::Pallet::<T>::block_number()
                    && round.round_type != v3::RoundType::ContributionRound
                {
                    round.project_keys.iter().for_each(|k| {
                        // Insert per project_key
                        *weight += T::DbWeight::get().reads_writes(1, 1);
                        Rounds::<T>::insert(k, round.round_type.into_new(), round.end);
                    })
                }
            }
            // Remove the old round.
            *weight += T::DbWeight::get().reads_writes(1, 1);
            None
        });
    }

    pub fn migrate_all<T: Config>() -> Weight {
        let mut weight = T::DbWeight::get().reads_writes(1, 1);
        migrate_contribution_and_project::<T>(&mut weight);
        migrate_user_votes::<T>(&mut weight);
        migrate_milestone_votes::<T>(&mut weight);
        migrate_rounds_and_round_type::<T>(&mut weight);
        weight
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use mock::*;

    use v0::{ContributionV0, MilestoneV0, ProjectV0};

    #[test]
    fn migrate_v0_to_v1() {
        let contribution_value = 1_000_000_u64;

        build_test_externality().execute_with(|| {
            let project_key = 1;
            let old_milestones = vec![
                MilestoneV0 {
                    project_key,
                    name: b"milestone 1".to_vec(),
                    milestone_key: 0,
                    percentage_to_unlock: 40,
                    is_approved: true,
                },
                MilestoneV0 {
                    project_key,
                    name: b"milestone 2".to_vec(),
                    milestone_key: 1,
                    percentage_to_unlock: 60,
                    is_approved: true,
                },
            ];

            let old_contributions = vec![
                ContributionV0 {
                    account_id: *ALICE,
                    value: contribution_value,
                },
                ContributionV0 {
                    account_id: *BOB,
                    value: contribution_value,
                },
            ];

            let old_project = ProjectV0 {
                name: b"Project Pre-migrations".to_vec(),
                logo: b"logo".to_vec(),
                description: b"description".to_vec(),
                website: b"https://imbue.network".to_vec(),
                milestones: old_milestones,
                contributions: old_contributions,
                currency_id: CurrencyId::KSM,
                required_funds: (100_000_000u32).into(),
                withdrawn_funds: (0u32).into(),
                initiator: *ALICE,
                create_block_number: 100u64,
                approved_for_funding: true,
                funding_threshold_met: true,
                cancelled: false,
            };

            v0::Projects::<Test>::insert(project_key, &old_project);
            let _ = v1::migrate::<Test>();
            let migrated_project =
                v1::Projects::<Test>::get(project_key).expect("project should exist");

            assert_eq!(old_project.name, migrated_project.name);

            assert_eq!(
                old_project.milestones[0].percentage_to_unlock,
                migrated_project
                    .milestones
                    .get(&0)
                    .unwrap()
                    .percentage_to_unlock
            );

            assert_eq!(
                old_project.milestones[0].name,
                migrated_project.milestones.get(&0).unwrap().name
            );

            assert_eq!(
                &old_project.contributions[0].value,
                &migrated_project.contributions.get(&ALICE).unwrap().value
            );

            assert_eq!(
                contribution_value.saturating_mul(2),
                migrated_project.raised_funds
            );
        })
    }

    #[test]
    fn migrate_v1_to_v2() {
        build_test_externality().execute_with(|| {
            let project_key = 1;
            let contribution_value = 1_000_000_u64;
            let mut contributions: BTreeMap<
                AccountIdOf<Test>,
                Contribution<BalanceOf<Test>, TimestampOf<Test>>,
            > = BTreeMap::new();

            contributions.insert(
                *ALICE,
                Contribution {
                    value: contribution_value,
                    timestamp: TimestampOf::<Test>::default(),
                },
            );

            contributions.insert(
                *BOB,
                Contribution {
                    value: contribution_value,
                    timestamp: TimestampOf::<Test>::default(),
                },
            );

            let old_project = v1::ProjectV1 {
                name: b"Project Pre-migrations".to_vec(),
                logo: b"logo".to_vec(),
                description: b"description".to_vec(),
                website: b"https://imbue.network".to_vec(),
                milestones: BTreeMap::new(),
                contributions,
                currency_id: CurrencyId::KSM,
                required_funds: (100_000_000u32).into(),
                raised_funds: (100_000_000u32).into(),
                withdrawn_funds: (0u32).into(),
                initiator: *ALICE,
                create_block_number: 100u64,
                approved_for_funding: true,
                funding_threshold_met: true,
                cancelled: false,
            };
            v1::Projects::<Test>::insert(project_key, &old_project);
            let _ = v2::migrate::<Test>();
            dbg!(&v1::Projects::<Test>::iter_keys().collect::<Vec<u32>>());
            let migrated_project = v2::Projects::<Test>::get(1).unwrap();

            assert_eq!(old_project.create_block_number, migrated_project.created_on);

            assert_eq!(
                &old_project.contributions.get(&*ALICE).unwrap().value,
                &migrated_project.contributions.get(&*ALICE).unwrap().value
            );

            assert_eq!(H256::default(), migrated_project.agreement_hash);
            assert_eq!(FundingType::Proposal, migrated_project.funding_type);
        })
    }

    #[test]
    fn migrate_v2_to_v3() {
        build_test_externality().execute_with(|| {
            use v3::*;

            // 1. -- TESTED - Use blocknumber instead of timestamp for contribution.
            // 2. -- TESTED - Project has had required_funds, approved_for_funding and funding_threshold_met removed.
            // 3. -- TESTED - UserVotes storage map needs removing + placing into UserHasVoted
            // 4. -- TESTED - Milestone votes is now a double map (project_key, milestone_key) +
            // 5. -- TESTED - Rounds is also a DoubleMap
            // 6. -- UNTESTED: Round type has had contribution_round removed. Cannot test as the contribution rounds dont exist anymore.
            // 7. -- TESTED - percent_to_unlock changed from u32 to Percent. (cuteolaf) 
            // 8. -- TESTED - Project new field deposit_id 
            // 9. -- // - milestones and contributions have been bound, on overflow the project is removed.
            let mut old_milestones = BTreeMap::new();
               old_milestones.insert(0,v2::MilestoneV1 {
                    project_key: 0,
                    milestone_key: 0,
                    percentage_to_unlock: 40u32,
                    is_approved: true,
                });
                old_milestones.insert(1, v2::MilestoneV1 {
                    project_key: 0,
                    milestone_key: 1,
                    percentage_to_unlock: 60u32,
                    is_approved: true,
                });
            let mut contributions = BTreeMap::new();
            contributions.insert(
                *CHARLIE,
                Contribution {
                    value: 100_000,
                    timestamp: TimestampOf::<Test>::default(),
                },
            );
            contributions.insert(
                *BOB,
                Contribution {
                    value: 900_000,
                    timestamp: TimestampOf::<Test>::default(),
                },
            );
            let project = v2::ProjectV2 {
                agreement_hash: Default::default(),
                milestones: old_milestones,
                contributions: contributions ,
                currency_id: CurrencyId::Native,
                required_funds: 1_000_000 ,
                withdrawn_funds: 0,
                raised_funds: 1_000_000,
                initiator: *ALICE,
                created_on: frame_system::Pallet::<Test>::block_number(),
                approved_for_funding: false,
                funding_threshold_met: false,
                cancelled: false,
                funding_type: FundingType::Brief,
            };
            v2::Projects::<Test>::insert(0, &project);
            v3::UserVotes::<Test>::insert((*ALICE, 10u32, 10u32, v3::RoundType::VotingRound), true);
            v3::UserVotes::<Test>::insert((*ALICE, 10u32, 10u32, v3::RoundType::VoteOfNoConfidence), true);
            let v = Vote {
                yay: 100_000u64,
                nay: 50_000u64,
                is_approved: false,
            };
            v3::OldMilestoneVotes::<Test>::insert((10, 10), v);
            let end_block_number = frame_system::Pallet::<Test>::block_number() + 100;
            let old_round: v3::Round<BlockNumberFor<Test>> = v3::Round {
                start: frame_system::Pallet::<Test>::block_number() - 1,
                end: end_block_number,
                project_keys: vec![1,2,3],
                round_type: v3::RoundType::VotingRound,
                is_canceled: false,
            };
            v3::OldRounds::<Test>::insert(0, Some(old_round));

            let w = v3::migrate_all::<Test>();

            let project_apres = crate::Projects::<Test>::get(0).unwrap();
            // #1, 2, 7 & 8
            assert_eq!(project.agreement_hash, project_apres.agreement_hash);
            assert_eq!(project.contributions[&CHARLIE].value, project_apres.contributions[&CHARLIE].value);
            assert_eq!(project.contributions[&BOB].value, project_apres.contributions[&BOB].value);
            assert_eq!(project_apres.contributions.iter().len(), 2usize);
            assert_eq!(project.milestones[&0].milestone_key, project_apres.milestones[&0].milestone_key);
            assert_eq!(project.milestones[&0].project_key, project_apres.milestones[&0].project_key);
            assert_eq!(Percent::from_percent(project.milestones[&0].percentage_to_unlock as u8), project_apres.milestones[&0].percentage_to_unlock);
            assert_eq!(project_apres.deposit_id, u32::MAX as u64);

            // #3
            let new_votes = UserHasVoted::<Test>::get((10u32, crate::RoundType::VotingRound, 10u32));
            assert!(new_votes[&ALICE]);
            let new_votes = UserHasVoted::<Test>::get((10u32, crate::RoundType::VoteOfNoConfidence, 10u32));
            assert!(new_votes[&ALICE]);

            // #4
            assert_eq!(v3::OldMilestoneVotes::<Test>::get((10, 10)), Default::default());
            assert!(crate::MilestoneVotes::<Test>::contains_key(10, 10));
            let v = crate::MilestoneVotes::<Test>::get(10, 10).unwrap();
            assert_eq!(v.yay, 100_000);
            assert_eq!(v.nay, 50_000);
            assert!(!v.is_approved);

            // #5
            assert!(OldRounds::<Test>::get(0).is_none());
            [1,2,3].iter().for_each(|k| {

                let end = crate::Rounds::<Test>::get(k, crate::RoundType::VotingRound).unwrap();
                assert_eq!(end, end_block_number);
            });
        })
    }
}
