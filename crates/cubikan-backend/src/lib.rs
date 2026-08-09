//! Durable-adapter values for CubiKan.
//!
//! This crate remains above [`cubikan_core`]. Its public model carries typed
//! domain values, while storage and external wire representations stay
//! adapter-owned and versioned.

#![forbid(unsafe_code)]

mod error;
mod model;
// T-802 deliberately lands the envelope before the SQLite tasks consume it.
// Remove this allowance once persistence integration makes the module live.
#[allow(dead_code)]
mod stored;

pub use error::{BackendError, ListCursorError, PageLimitError};
pub use model::{
    CompleteIntentUnit, CreateIntentUnit, GetIntentUnit, IntentUnitPage, IntentUnitSummary,
    IntentUnitView, ListCursor, ListFilters, ListIntentUnits, MutationResult, PageLimit,
    TransitionIntentUnit,
};
