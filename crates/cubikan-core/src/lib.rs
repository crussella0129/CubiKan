//! Chain-agnostic domain primitives for CubiKan.

#![forbid(unsafe_code)]

mod id;
mod vocabulary;

pub use id::{IntentUnitId, ParseIntentUnitIdError};
pub use vocabulary::{IntentSpecies, PhaseId, VocabularyError, WorkflowId};
