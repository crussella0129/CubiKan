//! Provisional conservative lifecycle dispatch weights.
//!
//! These nonzero values intentionally cover the largest bounded aggregate and
//! keep every call usable before runtime benchmarking. They are not represented
//! as generated output: T-1106 runs `src/benchmarking.rs` against the composed
//! benchmark-capable runtime, records the command and hardware provenance, and
//! replaces this file with the generated stable2606 weights.

use core::marker::PhantomData;

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};

/// Weight functions required by the lifecycle pallet.
pub trait WeightInfo {
    fn create_unit() -> Weight;
    fn transition_unit() -> Weight;
    fn complete_unit() -> Weight;
    fn replace_authorized_submitters(accounts: u32) -> Weight;
}

/// Conservative database-aware weights used until T-1106 generation.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn create_unit() -> Weight {
        Weight::from_parts(185_000_000, 310_000)
            .saturating_add(T::DbWeight::get().reads(6))
            .saturating_add(T::DbWeight::get().writes(5))
    }

    fn transition_unit() -> Weight {
        Weight::from_parts(205_000_000, 310_000)
            .saturating_add(T::DbWeight::get().reads(6))
            .saturating_add(T::DbWeight::get().writes(5))
    }

    fn complete_unit() -> Weight {
        Weight::from_parts(195_000_000, 310_000)
            .saturating_add(T::DbWeight::get().reads(6))
            .saturating_add(T::DbWeight::get().writes(5))
    }

    fn replace_authorized_submitters(accounts: u32) -> Weight {
        Weight::from_parts(45_000_000, 4_000)
            .saturating_add(Weight::from_parts(1_000_000, 40).saturating_mul(accounts.into()))
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(3))
    }
}

impl WeightInfo for () {
    fn create_unit() -> Weight {
        Weight::from_parts(185_000_000, 310_000)
            .saturating_add(RocksDbWeight::get().reads(6))
            .saturating_add(RocksDbWeight::get().writes(5))
    }

    fn transition_unit() -> Weight {
        Weight::from_parts(205_000_000, 310_000)
            .saturating_add(RocksDbWeight::get().reads(6))
            .saturating_add(RocksDbWeight::get().writes(5))
    }

    fn complete_unit() -> Weight {
        Weight::from_parts(195_000_000, 310_000)
            .saturating_add(RocksDbWeight::get().reads(6))
            .saturating_add(RocksDbWeight::get().writes(5))
    }

    fn replace_authorized_submitters(accounts: u32) -> Weight {
        Weight::from_parts(45_000_000, 4_000)
            .saturating_add(Weight::from_parts(1_000_000, 40).saturating_mul(accounts.into()))
            .saturating_add(RocksDbWeight::get().reads(1))
            .saturating_add(RocksDbWeight::get().writes(3))
    }
}
