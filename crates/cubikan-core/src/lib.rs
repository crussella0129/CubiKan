//! Chain-agnostic domain primitives for CubiKan.
//!
//! Workflows are caller-declared. The core does not provide default phases or
//! infer transition, completion, KPI, persistence, or blockchain policy.
//!
//! Every [`IntentUnit`] has an aggregate-local [`IntentUnitRevision`]. New
//! units begin at [`IntentUnitRevision::INITIAL`] (`0`), and each accepted
//! lifecycle mutation advances that revision exactly once. Revision-conditioned
//! operations provide optimistic conflict detection by rejecting a stale
//! observation before applying normal lifecycle validation; they do not provide
//! persistence-level compare-and-set or ordering between different Intent Units.
//!
//! # Example
//!
//! ```
//! use cubikan_core::{
//!     IntentSpecies, IntentUnit, IntentUnitId, IntentUnitRevision, IntentUnitStatus, PhaseId,
//!     Workflow, WorkflowEdge, WorkflowId,
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
//! assert_eq!(unit.revision(), IntentUnitRevision::INITIAL);
//! assert_eq!(unit.revision().value(), 0);
//!
//! let doing_revision = unit.transition_to_if_revision(&doing, unit.revision())?;
//! assert_eq!(doing_revision.value(), 1);
//! assert_eq!(unit.revision(), doing_revision);
//!
//! let done_revision = unit.transition_to_if_revision(&done, doing_revision)?;
//! let completed_revision = unit.complete_if_revision(done_revision)?;
//!
//! assert_eq!(unit.status(), IntentUnitStatus::Completed);
//! assert_eq!(unit.phase(), &done);
//! assert_eq!(unit.history().len(), 3);
//! assert_eq!(completed_revision.value(), 3);
//! assert_eq!(unit.revision(), completed_revision);
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
