use crate as pallet_deposits;
use crate::traits::{DepositCalculator, DepositHandler};
use common_types::CurrencyId;
use frame_support::once_cell::sync::Lazy;
use frame_support::traits::{ConstU16, ConstU64};
use frame_support::{pallet_prelude::*, parameter_types};
use sp_core::sr25519::{Public, Signature};
use sp_core::H256;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Deposits: pallet_deposits,
        Balances: pallet_balances::{Pallet, Call, Storage, Config<Test>, Event<Test>},
    }
);

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Balance = u64;
pub type BlockNumber = u64;

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

orml_traits::parameter_type_with_key! {
    pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
        100
    };
}

parameter_types! {
    pub MaxLocks: u32 = 2;
    pub const ExistentialDeposit: u64 = 5;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type AccountStore = System;
    type Balance = u64;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type WeightInfo = ();
}

parameter_types! {
    pub DepositSlashAccount: AccountId = Public::from_raw([66u8; 32]);

}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug, MaxEncodedLen, TypeInfo, Copy)]
pub enum StorageItem {
    CrowdFund,
    Brief,
    Grant,
    Project,
    Unsupported,
}
pub(crate) type DepositId = u64;

impl pallet_deposits::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type StorageItem = StorageItem;
    type DepositId = DepositId;
    type DepositCalculator = MockDepositCalculator;
    type DepositSlashAccount = DepositSlashAccount;
}

pub struct MockDepositCalculator;
impl DepositCalculator<Balance> for MockDepositCalculator {
    type StorageItem = StorageItem;
    fn calculate_deposit(item: Self::StorageItem) -> Result<Balance, DispatchError> {
        if item == StorageItem::Unsupported {
            return Err(crate::pallet::Error::<Test>::UnsupportedStorageType.into());
        }
        Ok(10_000u64)
    }
}

struct MockDepositHandler<T>(T);
impl<T: crate::Config> DepositHandler<crate::BalanceOf<T>, crate::AccountIdOf<T>>
    for MockDepositHandler<T>
{
    type DepositId = T::DepositId;
    type StorageItem = T::StorageItem;
    fn take_deposit(
        _who: crate::AccountIdOf<T>,
        _storage_item: Self::StorageItem,
    ) -> Result<T::DepositId, DispatchError> {
        todo!()
    }
    fn return_deposit(_deposit_id: Self::DepositId) -> DispatchResult {
        todo!()
    }
    fn slash_reserve_deposit(_deposit_id: Self::DepositId) -> DispatchResult {
        todo!()
    }
}

pub static ALICE: Lazy<Public> = Lazy::new(|| Public::from_raw([125u8; 32]));
pub static BOB: Lazy<Public> = Lazy::new(|| Public::from_raw([126u8; 32]));
pub static CHARLIE: Lazy<Public> = Lazy::new(|| Public::from_raw([127u8; 32]));

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let initial_balance = 10_000_000u64;
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (*ALICE, initial_balance),
            (*BOB, initial_balance),
            (*CHARLIE, initial_balance),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
    });
    ext
}

impl<T: crate::Config> DepositHandler<crate::BalanceOf<T>, crate::AccountIdOf<T>> for T {
    type DepositId = u64;
    type StorageItem = StorageItem;
    fn take_deposit(
        _who: crate::AccountIdOf<T>,
        _storage_item: Self::StorageItem,
    ) -> Result<Self::DepositId, DispatchError> {
        Ok(0u64)
    }
    fn return_deposit(_deposit_id: Self::DepositId) -> DispatchResult {
        Ok(().into())
    }
    fn slash_reserve_deposit(_deposit_id: Self::DepositId) -> DispatchResult {
        Ok(().into())
    }
}
