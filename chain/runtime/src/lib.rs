#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

extern crate alloc;

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod apis;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarks;
mod configs;
mod genesis_config_presets;
pub mod weights;

use alloc::vec::Vec;
use sp_runtime::{
    generic, impl_opaque_keys,
    traits::{BlakeTwo256, IdentifyAccount, Verify},
    MultiSignature,
};
use staging_parachain_info as parachain_info;

pub use configs::TransactionByteFee;
pub use genesis_config_presets::{local_genesis_config, session_keys};
pub use pallet_cubikan;
#[cfg(feature = "std")]
pub use sp_version::NativeVersion;

pub const POLKADOT_SDK_REVISION: &str = "8ae9775dc43c0d8cdd0f6d87700596e14278b1e1";
pub const LOCAL_PARA_ID: u32 = 1000;
pub const PALLET_STORAGE_VERSION: u16 = 1;
pub const EVENT_SCHEMA_VERSION: u16 = 1;
/// Exact domain-separated input whose SHA-256 digest identifies this one local
/// deployment. The trailing newline is deliberate and prevents an ambiguous
/// concatenation with later human-readable version text.
pub const DEPLOYMENT_ID_INPUT: &[u8; 28] = b"CubiKan local deployment v1\n";
/// SHA-256(`DEPLOYMENT_ID_INPUT`), pinned as bytes so native and Wasm genesis
/// never depend on a host hashing tool or runtime randomness.
pub const DEPLOYMENT_ID: [u8; 32] = [
    0x30, 0x46, 0xcb, 0x2c, 0xf3, 0xf5, 0xf9, 0xc5, 0x65, 0xa8, 0x54, 0x93, 0xcf, 0xff, 0x10, 0xfe,
    0xe9, 0x4d, 0x12, 0xd9, 0x50, 0xa0, 0xd6, 0xf5, 0x4d, 0x7c, 0x1f, 0xf3, 0x2a, 0x6a, 0xfc, 0x42,
];

pub const fn deployment_id() -> [u8; 32] {
    DEPLOYMENT_ID
}

pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Balance = u128;
pub type Nonce = u32;
pub type Hash = sp_core::H256;
pub type BlockNumber = u32;
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
pub type SignedBlock = generic::SignedBlock<Block>;

pub type TxExtension = cumulus_pallet_weight_reclaim::StorageWeightReclaim<
    Runtime,
    (
        frame_system::CheckNonZeroSender<Runtime>,
        frame_system::CheckSpecVersion<Runtime>,
        frame_system::CheckTxVersion<Runtime>,
        frame_system::CheckGenesis<Runtime>,
        frame_system::CheckEra<Runtime>,
        frame_system::CheckNonce<Runtime>,
        frame_system::CheckWeight<Runtime>,
        pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    ),
>;
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

impl_opaque_keys! { pub struct SessionKeys { pub aura: Aura, } }

#[sp_version::runtime_version]
pub const VERSION: sp_version::RuntimeVersion = sp_version::RuntimeVersion {
    spec_name: alloc::borrow::Cow::Borrowed("cubikan-runtime"),
    impl_name: alloc::borrow::Cow::Borrowed("cubikan-runtime"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 0,
    apis: apis::RUNTIME_API_VERSIONS,
    transaction_version: 1,
    system_version: 1,
};

/// JSON-RPC representation of the fixed consensus API inventory.
///
/// All-feature test builds also compile development-only benchmark/try-runtime
/// APIs; those two IDs are excluded so this value remains identical to the
/// normal release Wasm inventory committed in the deployment anchor.
#[cfg(feature = "std")]
pub fn fixed_runtime_api_versions_json() -> alloc::vec::Vec<(alloc::string::String, u32)> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    apis::RUNTIME_API_VERSIONS
        .iter()
        .filter(|(_api, _)| {
            #[cfg(feature = "runtime-benchmarks")]
            if *_api == <dyn frame_benchmarking::Benchmark<Block> as sp_api::RuntimeApiInfo>::ID {
                return false;
            }
            #[cfg(feature = "try-runtime")]
            if *_api == <dyn frame_try_runtime::TryRuntime<Block> as sp_api::RuntimeApiInfo>::ID {
                return false;
            }
            true
        })
        .map(|(api, version)| {
            let mut encoded = alloc::string::String::from("0x");
            for byte in api {
                encoded.push(HEX[(byte >> 4) as usize] as char);
                encoded.push(HEX[(byte & 0x0f) as usize] as char);
            }
            (encoded, *version)
        })
        .collect()
}

pub const MILLI_SECS_PER_BLOCK: u64 = 6_000;
pub const SLOT_DURATION: u64 = MILLI_SECS_PER_BLOCK;
pub const UNIT: Balance = 1_000_000_000_000;
pub const MILLI_UNIT: Balance = 1_000_000_000;
pub const MICRO_UNIT: Balance = 1_000_000;
pub const EXISTENTIAL_DEPOSIT: Balance = MILLI_UNIT;
pub const HOURS: BlockNumber = (60_000 / MILLI_SECS_PER_BLOCK as BlockNumber) * 60;

type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<Runtime, 6_000, 1, 3>;

#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

#[frame_support::runtime]
mod runtime {
    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask,
        RuntimeViewFunction
    )]
    pub struct Runtime;
    #[runtime::pallet_index(0)]
    pub type System = frame_system;
    #[runtime::pallet_index(1)]
    pub type ParachainSystem = cumulus_pallet_parachain_system;
    #[runtime::pallet_index(2)]
    pub type Timestamp = pallet_timestamp;
    #[runtime::pallet_index(3)]
    pub type ParachainInfo = parachain_info;
    #[runtime::pallet_index(4)]
    pub type WeightReclaim = cumulus_pallet_weight_reclaim;
    #[runtime::pallet_index(10)]
    pub type Balances = pallet_balances;
    #[runtime::pallet_index(11)]
    pub type TransactionPayment = pallet_transaction_payment;
    #[runtime::pallet_index(20)]
    pub type Authorship = pallet_authorship;
    #[runtime::pallet_index(21)]
    #[runtime::disable_call]
    pub type CollatorSelection = pallet_collator_selection;
    #[runtime::pallet_index(22)]
    pub type Session = pallet_session;
    #[runtime::pallet_index(23)]
    pub type Aura = pallet_aura;
    #[runtime::pallet_index(24)]
    pub type AuraExt = cumulus_pallet_aura_ext;
    #[runtime::pallet_index(50)]
    pub type Cubikan = pallet_cubikan;
}

cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
    BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
