#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

pub use pallet_cubikan;

/// Exact Polkadot SDK source used by this runtime foundation.
pub const POLKADOT_SDK_REVISION: &str = "8ae9775dc43c0d8cdd0f6d87700596e14278b1e1";

/// Local CubiKan parachain identifier reserved by the locked Sprint 11 design.
pub const LOCAL_PARA_ID: u32 = 1000;

/// Pallet storage identity for the first canonical chain generation.
pub const PALLET_STORAGE_VERSION: u16 = 1;

/// Accepted-event schema identity for the first canonical chain generation.
pub const EVENT_SCHEMA_VERSION: u16 = 1;

/// Exact native/Wasm identity for the pinned runtime foundation.
#[sp_version::runtime_version]
pub const VERSION: sp_version::RuntimeVersion = sp_version::RuntimeVersion {
    spec_name: alloc::borrow::Cow::Borrowed("cubikan-runtime"),
    impl_name: alloc::borrow::Cow::Borrowed("cubikan-runtime"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 0,
    apis: sp_version::create_apis_vec!([]),
    transaction_version: 1,
    system_version: 1,
};

/// SCALE and metadata seed shared by the later full runtime composition.
#[derive(Clone, Copy, Debug, Decode, Encode, Eq, MaxEncodedLen, PartialEq, TypeInfo)]
pub struct RuntimeMetadataFoundation {
    pub para_id: u32,
    pub pallet_storage_version: u16,
    pub event_schema_version: u16,
}

impl RuntimeMetadataFoundation {
    pub const fn local() -> Self {
        Self {
            para_id: LOCAL_PARA_ID,
            pallet_storage_version: PALLET_STORAGE_VERSION,
            event_schema_version: EVENT_SCHEMA_VERSION,
        }
    }
}

/// Account representation selected from the pinned SDK runtime family.
pub type AccountId = sp_runtime::AccountId32;

/// Relay-chain block-number representation selected from the pinned Cumulus family.
pub type RelayBlockNumber = cumulus_primitives_core::relay_chain::BlockNumber;

/// Storage-version representation selected from the pinned FRAME family.
pub const FRAME_STORAGE_VERSION: frame_support::traits::StorageVersion =
    frame_support::traits::StorageVersion::new(PALLET_STORAGE_VERSION);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_metadata_foundation_is_exact() {
        assert_eq!(
            RuntimeMetadataFoundation::local(),
            RuntimeMetadataFoundation {
                para_id: 1000,
                pallet_storage_version: 1,
                event_schema_version: 1,
            }
        );
        assert_eq!(POLKADOT_SDK_REVISION.len(), 40);
        assert_eq!(VERSION.spec_version, 1);
        assert_eq!(VERSION.transaction_version, 1);
    }
}
