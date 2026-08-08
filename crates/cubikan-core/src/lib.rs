//! Chain-agnostic domain primitives for CubiKan.

#![forbid(unsafe_code)]

mod id;
mod intent_unit;
mod vocabulary;
mod workflow;

pub use id::{IntentUnitId, ParseIntentUnitIdError};
pub use intent_unit::{
    CompletionRecord, IntentUnit, IntentUnitStatus, LifecycleRecord, TransitionError,
    TransitionRecord,
};
pub use vocabulary::{IntentSpecies, PhaseId, VocabularyError, WorkflowId};
pub use workflow::{Workflow, WorkflowEdge, WorkflowError};
