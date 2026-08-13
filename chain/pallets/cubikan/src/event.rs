//! Stable identities carried by every accepted CubiKan domain event.

/// Caller-independent command schema accepted by lifecycle dispatchables.
pub const COMMAND_SCHEMA_VERSION: u16 = 1;

/// First canonical pallet storage schema.
pub const PALLET_STORAGE_SCHEMA_VERSION: u16 = 1;

/// First canonical accepted-event schema.
pub const EVENT_SCHEMA_VERSION: u16 = 1;

/// Non-self-referential deployment identity configured at genesis.
pub type DeploymentId = [u8; 32];
