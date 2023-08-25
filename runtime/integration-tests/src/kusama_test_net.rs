// Copyright 2022 Imbue Network (imbue.network).
// This file is part of Imbue chain project.

// Imbue is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version (see http://www.gnu.org/licenses).

// Imbue is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

//! Relay chain and parachains emulation.



pub use imbue_kusama_runtime::{AccountId, AuraId, Balance, BlockNumber};
use crate::constants::SAFE_XCM_VERSION;



use xcm_emulator::{
	decl_test_networks, decl_test_parachains, decl_test_relay_chains, Parachain, RelayChain,
	TestExt,
};
pub use sp_core::{sr25519, storage::Storage, Get};
use xcm::prelude::*;
use crate::constants::{imbue,kusama, accounts::{ALICE,BOB, CHARLIE, DAVE, EVE, FERDIE}};

use frame_support::{parameter_types, sp_io, sp_tracing};
use crate::setup::{
    PARA_ID_DEVELOPMENT,
    PARA_ID_SIBLING,
};
use xcm_executor::traits::Convert;
use xcm_builder::test_utils::XcmHash;
decl_test_relay_chains! {
	pub struct Kusama {
		genesis = kusama::genesis(),
		on_init = (
			// kusama_runtime::XcmPallet::force_default_xcm_version(
            // kusama_runtime::RuntimeOrigin::root(),
            // Some(SAFE_XCM_VERSION)),
			//
			// kusama_runtime::XcmPallet::force_xcm_version(
            // kusama_runtime::RuntimeOrigin::root(),
            // Box::new(MultiLocation::new(1, X1(Parachain(PARA_ID_SIBLING)))),
            // SAFE_XCM_VERSION),
			//
			// kusama_runtime::XcmPallet::force_xcm_version(
            // kusama_runtime::RuntimeOrigin::root(),
            // Box::new(MultiLocation::new(1, X1(Parachain(PARA_ID_DEVELOPMENT)))),
            // SAFE_XCM_VERSION),

			kusama_runtime::XcmPallet::force_xcm_version(
            kusama_runtime::RuntimeOrigin::root(),
            Box::new(MultiLocation::new(0, X1(Parachain(PARA_ID_SIBLING)))),
            SAFE_XCM_VERSION),

			kusama_runtime::XcmPallet::force_xcm_version(
            kusama_runtime::RuntimeOrigin::root(),
            Box::new(MultiLocation::new(0, X1(Parachain(PARA_ID_DEVELOPMENT)))),
            SAFE_XCM_VERSION),
		),
		runtime = {
			Runtime: kusama_runtime::Runtime,
			RuntimeOrigin: kusama_runtime::RuntimeOrigin,
			RuntimeCall: kusama_runtime::RuntimeCall,
			RuntimeEvent: kusama_runtime::RuntimeEvent,
			MessageQueue: kusama_runtime::MessageQueue,
			XcmConfig: kusama_runtime::xcm_config::XcmConfig,
			SovereignAccountOf: kusama_runtime::xcm_config::SovereignAccountOf,
			System: kusama_runtime::System,
			Balances: kusama_runtime::Balances,
		},
		pallets_extra = {
			XcmPallet: kusama_runtime::XcmPallet,
		}
	}
}
decl_test_parachains! {
	pub struct Development {
		genesis = imbue::genesis(PARA_ID_DEVELOPMENT),
		on_init = (
			imbue_kusama_runtime::PolkadotXcm::force_xcm_version(
            imbue_kusama_runtime::RuntimeOrigin::root(),
            Box::new(MultiLocation::new(1, Here)),
            SAFE_XCM_VERSION),
		),
		runtime = {
			Runtime: imbue_kusama_runtime::Runtime,
			RuntimeOrigin: imbue_kusama_runtime::RuntimeOrigin,
			RuntimeCall: imbue_kusama_runtime::RuntimeCall,
			RuntimeEvent: imbue_kusama_runtime::RuntimeEvent,
			XcmpMessageHandler: imbue_kusama_runtime::XcmpQueue,
			DmpMessageHandler: imbue_kusama_runtime::DmpQueue,
			LocationToAccountId: imbue_kusama_runtime::xcm_config::LocationToAccountId,
			System: imbue_kusama_runtime::System,
			Balances: imbue_kusama_runtime::Balances,
			ParachainSystem: imbue_kusama_runtime::ParachainSystem,
			ParachainInfo: imbue_kusama_runtime::ParachainInfo,
		},
		pallets_extra = {
			PolkadotXcm: imbue_kusama_runtime::PolkadotXcm,
			XTokens: imbue_kusama_runtime::XTokens,
		}
	},
	pub struct Sibling {
		genesis = imbue::genesis(PARA_ID_SIBLING),
		on_init = (
			imbue_kusama_runtime::PolkadotXcm::force_xcm_version(
            imbue_kusama_runtime::RuntimeOrigin::root(),
            Box::new(MultiLocation::new(1, Here)),
            SAFE_XCM_VERSION),
		),
		runtime = {
			Runtime: imbue_kusama_runtime::Runtime,
			RuntimeOrigin: imbue_kusama_runtime::RuntimeOrigin,
			RuntimeCall: imbue_kusama_runtime::RuntimeCall,
			RuntimeEvent: imbue_kusama_runtime::RuntimeEvent,
			XcmpMessageHandler: imbue_kusama_runtime::XcmpQueue,
			DmpMessageHandler: imbue_kusama_runtime::DmpQueue,
			LocationToAccountId: imbue_kusama_runtime::xcm_config::LocationToAccountId,
			System: imbue_kusama_runtime::System,
			Balances: imbue_kusama_runtime::Balances,
			ParachainSystem: imbue_kusama_runtime::ParachainSystem,
			ParachainInfo: imbue_kusama_runtime::ParachainInfo,
		},
		pallets_extra = {
			PolkadotXcm: imbue_kusama_runtime::PolkadotXcm,
			XTokens: imbue_kusama_runtime::XTokens,
		}
	}
}

decl_test_networks! {
	pub struct TestNet {
		relay_chain = Kusama,
		parachains = vec![
			Development,
			Sibling,
			// Karura,
		],
	}
}

parameter_types! {
	// Kusama
	pub KusamaSender: AccountId = Kusama::account_id_of(ALICE);
	pub KusamaReceiver: AccountId = Kusama::account_id_of(BOB);
	// Imbue Kusama
	pub ImbueKusamaSender: AccountId = Development::account_id_of(CHARLIE);
	pub ImbueKusamaReceiver: AccountId = Development::account_id_of(DAVE);
	// Sibling Kusama
	pub SiblingKusamaSender: AccountId = Sibling::account_id_of(EVE);
	pub SiblingKusamaReceiver: AccountId = Sibling::account_id_of(FERDIE);
}


// decl_test_parachains! {
//     pub struct Development {
//         Runtime = imbue_kusama_runtime::Runtime,
//         RuntimeOrigin = imbue_kusama_runtime::RuntimeOrigin,
//         XcmpMessageHandler = imbue_kusama_runtime::XcmpQueue,
//         DmpMessageHandler = imbue_kusama_runtime::DmpQueue,
//         new_ext = para_ext(PARA_ID_DEVELOPMENT),
//     }
// }
//
// decl_test_parachains! {
//     pub struct Sibling {
//         Runtime = imbue_kusama_runtime::Runtime,
//         RuntimeOrigin = imbue_kusama_runtime::RuntimeOrigin,
//         XcmpMessageHandler = imbue_kusama_runtime::XcmpQueue,
//         DmpMessageHandler = imbue_kusama_runtime::DmpQueue,
//         new_ext = para_ext(PARA_ID_SIBLING),
//     }
// }
//
// decl_test_parachains! {
//     pub struct Karura {
//         Runtime = imbue_kusama_runtime::Runtime,
//         RuntimeOrigin = imbue_kusama_runtime::RuntimeOrigin,
//         XcmpMessageHandler = imbue_kusama_runtime::XcmpQueue,
//         DmpMessageHandler = imbue_kusama_runtime::DmpQueue,
//         new_ext = para_ext(PARA_ID_KARURA),
//     }
// }
//
// decl_test_networks! {
//     pub struct TestNet {
//         relay_chain = KusamaNet,
//         parachains = vec![
//             // N.B: Ideally, we could use the defined para id constants but doing so
//             // fails with: "error: arbitrary expressions aren't allowed in patterns"
//
//             // Be sure to use `PARA_ID_DEVELOPMENT`
//             (2121, Development),
//             // Be sure to use `PARA_ID_SIBLING`
//             (3000, Sibling),
//             // Be sure to use `PARA_ID_KARURA`
//             (2000, Karura),
//         ],
//     }
// }
//
// pub fn kusama_ext() -> sp_io::TestExternalities {
//     use kusama_runtime::{Runtime, System};
//
//     let mut t = frame_system::GenesisConfig::default()
//         .build_storage::<Runtime>()
//         .unwrap();
//
//     pallet_balances::GenesisConfig::<Runtime> {
//         balances: vec![
//             (AccountId::from(ALICE), native_amount(2002)),
//             (
//                 ParaId::from(PARA_ID_DEVELOPMENT).into_account_truncating(),
//                 native_amount(7),
//             ),
//             (
//                 ParaId::from(PARA_ID_SIBLING).into_account_truncating(),
//                 native_amount(7),
//             ),
//         ],
//     }
//     .assimilate_storage(&mut t)
//     .unwrap();
//
//     polkadot_runtime_parachains::configuration::GenesisConfig::<Runtime> {
//         config: default_parachains_host_configuration(),
//     }
//     .assimilate_storage(&mut t)
//     .unwrap();
//
//     <pallet_xcm::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
//         &pallet_xcm::GenesisConfig {
//             safe_xcm_version: Some(2),
//         },
//         &mut t,
//     )
//     .unwrap();
//
//     let mut ext = sp_io::TestExternalities::new(t);
//     ext.execute_with(|| System::set_block_number(1));
//     ext
// }
//
// pub fn para_ext(parachain_id: u32) -> sp_io::TestExternalities {
//     ExtBuilder::default()
//         .balances(vec![
//             (
//                 AccountId::from(ALICE),
//                 CurrencyId::Native,
//                 native_amount(10),
//             ),
//             (AccountId::from(BOB), CurrencyId::Native, native_amount(10)),
//             (AccountId::from(ALICE), CurrencyId::KSM, ksm_amount(10)),
//             (
//                 imbue_kusama_runtime::TreasuryAccount::get(),
//                 CurrencyId::KSM,
//                 ksm_amount(1),
//             ),
//         ])
//         .parachain_id(parachain_id)
//         .build()
// }
//
// fn default_parachains_host_configuration() -> HostConfiguration<BlockNumber> {
//     HostConfiguration {
//         minimum_validation_upgrade_delay: 5,
//         validation_upgrade_cooldown: 5u32,
//         validation_upgrade_delay: 5,
//         code_retention_period: 1200,
//         max_code_size: MAX_CODE_SIZE,
//         max_pov_size: MAX_POV_SIZE,
//         max_head_data_size: 32 * 1024,
//         group_rotation_frequency: 20,
//         chain_availability_period: 4,
//         thread_availability_period: 4,
//         max_upward_queue_count: 8,
//         max_upward_queue_size: 1024 * 1024,
//         max_downward_message_size: 1024,
//         max_upward_message_size: 50 * 1024,
//         max_upward_message_num_per_candidate: 5,
//         hrmp_sender_deposit: 0,
//         hrmp_recipient_deposit: 0,
//         hrmp_channel_max_capacity: 8,
//         hrmp_channel_max_total_size: 8 * 1024,
//         hrmp_max_parachain_inbound_channels: 4,
//         hrmp_max_parathread_inbound_channels: 4,
//         hrmp_channel_max_message_size: 1024 * 1024,
//         hrmp_max_parachain_outbound_channels: 4,
//         hrmp_max_parathread_outbound_channels: 4,
//         hrmp_max_message_num_per_candidate: 5,
//         dispute_period: 6,
//         no_show_slots: 2,
//         n_delay_tranches: 25,
//         needed_approvals: 2,
//         relay_vrf_modulo_samples: 2,
//         zeroth_delay_tranche_width: 0,
//         ..Default::default()
//     }
// }
