use frame_support::{
    traits::{ConstU16, ConstU64},
    weights::Weight,
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use crate as pallet_auto_tasks;
use crate::weights::WeightInfo;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test,>;
type Block = frame_system::mocking::MockBlock<Test,>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        AutoTasks: pallet_auto_tasks,
    }
);

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId,>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250,>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ConstU16<42,>;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16,>;
    type RuntimeTask = RuntimeTask;
}

// Task types and implementations for the mock runtime
pub enum RuntimeTask {
    AutoTask(pallet_auto_tasks::Task<Test,>,),
}

impl From<pallet_auto_tasks::Task<Test,>,> for RuntimeTask {
    fn from(task: pallet_auto_tasks::Task<Test,>,) -> Self {
        RuntimeTask::AutoTask(task,)
    }
}

impl frame_support::traits::Task for RuntimeTask {
    fn run(&self,) -> frame_support::dispatch::DispatchResultWithInfo<(),> {
        match self {
            RuntimeTask::AutoTask(task,) => match task {
                pallet_auto_tasks::Task::<Test,>::AddNumberIntoTotal { i, } => {
                    pallet_auto_tasks::Pallet::<Test,>::add_number_into_total(*i,)?;
                    Ok(().into(),)
                }
            },
        }
    }
}

// Define TestWeightInfo for the tests
pub struct TestWeightInfo;

impl WeightInfo for TestWeightInfo {
    fn add_number_into_total() -> Weight {
        Weight::from_parts(10_000, 0,)
    }

    fn store_number() -> Weight {
        Weight::from_parts(5_000, 0,)
    }

    fn get_totals() -> Weight {
        Weight::from_parts(2_000, 0,)
    }
}

// Update Config to use TestWeightInfo instead of SubstrateWeight
impl pallet_auto_tasks::Config for Test {
    type RuntimeTask = RuntimeTask;
    type WeightInfo = TestWeightInfo;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into()
}

// Helper function to run to a specific block
pub fn run_to_block(n: u64,) {
    while System::block_number() < n {
        System::on_finalize(System::block_number(),);
        System::set_block_number(System::block_number() + 1,);
        System::on_initialize(System::block_number(),);
    }
}
