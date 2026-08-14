//! Runtime-owned weight implementations and fixed execution constants.

use frame_support::{
    parameter_types,
    weights::{constants, RuntimeDbWeight, Weight},
};

pub mod pallet_cubikan;

/// Exact stable2606-1 generic storage-reclaim weight.
///
/// The pinned SDK exposes the trait but keeps its generated
/// `cumulus/pallets/weight-reclaim/src/weights.rs::SubstrateWeight` module
/// private. This runtime therefore reproduces that same-revision nonzero
/// value instead of selecting the crate's `()` fallback.
pub struct StorageWeightReclaimWeight;

impl cumulus_pallet_weight_reclaim::WeightInfo for StorageWeightReclaimWeight {
    fn storage_weight_reclaim() -> Weight {
        Weight::from_parts(2_466_000, 0)
    }
}

parameter_types! {
    pub const BlockExecutionWeight: Weight =
        Weight::from_parts(constants::WEIGHT_REF_TIME_PER_NANOS.saturating_mul(5_000_000), 0);
    pub const ExtrinsicBaseWeight: Weight =
        Weight::from_parts(constants::WEIGHT_REF_TIME_PER_NANOS.saturating_mul(125_000), 0);
    pub const RocksDbWeight: RuntimeDbWeight = RuntimeDbWeight {
        read: 25_000 * constants::WEIGHT_REF_TIME_PER_NANOS,
        write: 100_000 * constants::WEIGHT_REF_TIME_PER_NANOS,
    };
}
