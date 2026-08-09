//! Durable-adapter values for CubiKan.
//!
//! This crate remains above [`cubikan_core`]. Its public model carries typed
//! domain values, while storage and external wire representations stay
//! adapter-owned and versioned.

#![forbid(unsafe_code)]

mod error;
mod model;
mod schema;
mod sqlite;
mod stored;

pub use error::{BackendError, ListCursorError, PageLimitError, StorageFailure};
pub use model::{
    CompleteIntentUnit, CreateIntentUnit, GetIntentUnit, IntentUnitPage, IntentUnitSummary,
    IntentUnitView, ListCursor, ListFilters, ListIntentUnits, MutationResult, PageLimit,
    TransitionIntentUnit,
};
pub use sqlite::SqliteBackend;
