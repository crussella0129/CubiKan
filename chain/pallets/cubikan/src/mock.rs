use crate as pallet_cubikan;
use frame_support::{derive_impl, sp_runtime::BuildStorage};

pub type AccountId = u64;
pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const DEPLOYMENT_ID: [u8; 32] = [0xa5; 32];

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        CubiKan: pallet_cubikan,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
}

impl pallet_cubikan::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

pub fn new_test_ext(
    authorized_submitters: Vec<AccountId>,
) -> frame_support::__private::TestExternalities {
    let mut storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("mock System genesis must build");
    pallet_cubikan::GenesisConfig::<Test> {
        deployment_id: DEPLOYMENT_ID,
        pallet_storage_version: 1,
        event_schema_version: 1,
        authorized_submitters,
    }
    .assimilate_storage(&mut storage)
    .expect("mock CubiKan genesis must assimilate");

    let mut externalities = frame_support::__private::TestExternalities::new(storage);
    externalities.execute_with(|| System::set_block_number(1));
    externalities
}
