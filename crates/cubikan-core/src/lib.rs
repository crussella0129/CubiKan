//! Chain-agnostic domain primitives for CubiKan.
//!
//! Workflows are caller-declared. The core does not provide default phases or
//! infer transition, completion, KPI, persistence, or blockchain policy.
//!
//! # Example
//!
//! ```
//! use cubikan_core::{
//!     IntentSpecies, IntentUnit, IntentUnitId, IntentUnitStatus, PhaseId, Workflow,
//!     WorkflowEdge, WorkflowId,
//! };
//!
//! let queued = PhaseId::new("queued")?;
//! let doing = PhaseId::new("doing")?;
//! let done = PhaseId::new("done")?;
//! let workflow = Workflow::new(
//!     WorkflowId::new("delivery")?,
//!     vec![queued.clone(), doing.clone(), done.clone()],
//!     queued.clone(),
//!     vec![
//!         WorkflowEdge::new(queued, doing.clone()),
//!         WorkflowEdge::new(doing.clone(), done.clone()),
//!     ],
//!     vec![done.clone()],
//! )?;
//! let mut unit = IntentUnit::new(
//!     IntentUnitId::generate(),
//!     IntentSpecies::new("feature")?,
//!     workflow,
//! );
//!
//! unit.transition_to(&doing)?;
//! unit.transition_to(&done)?;
//! unit.complete()?;
//!
//! assert_eq!(unit.status(), IntentUnitStatus::Completed);
//! assert_eq!(unit.phase(), &done);
//! assert_eq!(unit.history().len(), 3);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![forbid(unsafe_code)]

mod id;
mod intent_unit;
mod vocabulary;
mod workflow;

pub use id::{IntentUnitId, ParseIntentUnitIdError};
pub use intent_unit::{
    CompletionError, CompletionRecord, IntentUnit, IntentUnitRevision, IntentUnitStatus,
    LifecycleRecord, RevisionConflict, RevisionedCompletionError, RevisionedTransitionError,
    TransitionError, TransitionRecord,
};
pub use vocabulary::{IntentSpecies, PhaseId, VocabularyError, WorkflowId};
pub use workflow::{Workflow, WorkflowEdge, WorkflowError};
