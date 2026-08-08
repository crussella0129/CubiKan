use std::{error::Error, fmt};

use crate::{IntentSpecies, IntentUnitId, PhaseId, Workflow, WorkflowId};

/// Whether an Intent Unit can still move through its workflow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntentUnitStatus {
    /// The unit may transition or complete according to its workflow snapshot.
    Active,
    /// The unit is terminal and cannot change again.
    Completed,
}

/// Immutable record of one successful phase transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionRecord {
    sequence: usize,
    from: PhaseId,
    to: PhaseId,
}

impl TransitionRecord {
    /// Returns the one-based lifecycle sequence number.
    #[must_use]
    pub const fn sequence(&self) -> usize {
        self.sequence
    }

    /// Returns the phase occupied before the transition.
    #[must_use]
    pub const fn from(&self) -> &PhaseId {
        &self.from
    }

    /// Returns the phase occupied after the transition.
    #[must_use]
    pub const fn to(&self) -> &PhaseId {
        &self.to
    }
}

/// Immutable record of terminal completion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionRecord {
    sequence: usize,
    final_phase: PhaseId,
}

impl CompletionRecord {
    /// Returns the one-based lifecycle sequence number.
    #[must_use]
    pub const fn sequence(&self) -> usize {
        self.sequence
    }

    /// Returns the phase occupied when the unit completed.
    #[must_use]
    pub const fn final_phase(&self) -> &PhaseId {
        &self.final_phase
    }
}

/// One immutable entry in an Intent Unit's in-memory domain history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LifecycleRecord {
    /// A successful directed phase transition.
    Transition(TransitionRecord),
    /// Terminal completion in an eligible phase.
    Completion(CompletionRecord),
}

impl LifecycleRecord {
    /// Returns the record's one-based sequence number.
    #[must_use]
    pub const fn sequence(&self) -> usize {
        match self {
            Self::Transition(record) => record.sequence(),
            Self::Completion(record) => record.sequence(),
        }
    }
}

/// A chain-agnostic unit of intent moving through caller-declared phases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentUnit {
    id: IntentUnitId,
    species: IntentSpecies,
    workflow: Workflow,
    phase: PhaseId,
    status: IntentUnitStatus,
    history: Vec<LifecycleRecord>,
}

impl IntentUnit {
    /// Creates an active unit at the owned workflow snapshot's initial phase.
    #[must_use]
    pub fn new(id: IntentUnitId, species: IntentSpecies, workflow: Workflow) -> Self {
        let phase = workflow.initial_phase().clone();
        Self {
            id,
            species,
            workflow,
            phase,
            status: IntentUnitStatus::Active,
            history: Vec::new(),
        }
    }

    /// Returns the immutable Intent Unit identity.
    #[must_use]
    pub const fn id(&self) -> IntentUnitId {
        self.id
    }

    /// Returns the immutable species provenance.
    #[must_use]
    pub const fn species(&self) -> &IntentSpecies {
        &self.species
    }

    /// Returns the owned immutable workflow snapshot.
    #[must_use]
    pub const fn workflow(&self) -> &Workflow {
        &self.workflow
    }

    /// Returns the identity of the owned workflow snapshot.
    #[must_use]
    pub fn workflow_id(&self) -> &WorkflowId {
        self.workflow.id()
    }

    /// Returns the unit's current phase.
    #[must_use]
    pub const fn phase(&self) -> &PhaseId {
        &self.phase
    }

    /// Returns whether the unit is active or completed.
    #[must_use]
    pub const fn status(&self) -> IntentUnitStatus {
        self.status
    }

    /// Returns immutable lifecycle records in sequence order.
    #[must_use]
    pub fn history(&self) -> &[LifecycleRecord] {
        &self.history
    }

    /// Moves the unit across an edge declared by its workflow snapshot.
    pub fn transition_to(&mut self, target: &PhaseId) -> Result<(), TransitionError> {
        if self.status == IntentUnitStatus::Completed {
            return Err(TransitionError::AlreadyCompleted);
        }
        if !self.workflow.contains_phase(target) {
            return Err(TransitionError::UnknownTarget {
                target: target.clone(),
            });
        }

        let from = self.phase.clone();
        if !self.workflow.allows_transition(&from, target) {
            return Err(TransitionError::NotAllowed {
                from,
                to: target.clone(),
            });
        }

        let record = TransitionRecord {
            sequence: self.history.len() + 1,
            from,
            to: target.clone(),
        };
        self.history.push(LifecycleRecord::Transition(record));
        self.phase = target.clone();
        Ok(())
    }

    /// Completes the unit when its current phase is marked eligible.
    pub fn complete(&mut self) -> Result<(), CompletionError> {
        if self.status == IntentUnitStatus::Completed {
            return Err(CompletionError::AlreadyCompleted);
        }
        if !self.workflow.allows_completion(&self.phase) {
            return Err(CompletionError::PhaseNotEligible {
                phase: self.phase.clone(),
            });
        }

        let record = CompletionRecord {
            sequence: self.history.len() + 1,
            final_phase: self.phase.clone(),
        };
        self.history.push(LifecycleRecord::Completion(record));
        self.status = IntentUnitStatus::Completed;
        Ok(())
    }
}

/// Rejection from an attempted phase transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionError {
    /// Terminal Intent Units cannot move again.
    AlreadyCompleted,
    /// The requested target does not belong to the workflow snapshot.
    UnknownTarget { target: PhaseId },
    /// Both phases exist, but their directed edge was not declared.
    NotAllowed { from: PhaseId, to: PhaseId },
}

impl fmt::Display for TransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyCompleted => formatter.write_str("Intent Unit is already completed"),
            Self::UnknownTarget { target } => {
                write!(formatter, "target phase `{target}` is not declared")
            }
            Self::NotAllowed { from, to } => {
                write!(formatter, "transition `{from} -> {to}` is not declared")
            }
        }
    }
}

impl Error for TransitionError {}

/// Rejection from an attempted terminal completion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompletionError {
    /// Terminal Intent Units cannot complete again.
    AlreadyCompleted,
    /// The current phase is not marked completion-eligible.
    PhaseNotEligible { phase: PhaseId },
}

impl fmt::Display for CompletionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyCompleted => formatter.write_str("Intent Unit is already completed"),
            Self::PhaseNotEligible { phase } => {
                write!(formatter, "phase `{phase}` is not eligible for completion")
            }
        }
    }
}

impl Error for CompletionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorkflowEdge;
    use std::str::FromStr;

    fn phase(value: &str) -> PhaseId {
        PhaseId::new(value).expect("fixture phase should be valid")
    }

    fn workflow() -> Workflow {
        let queued = phase("queued");
        let done = phase("done");
        Workflow::new(
            WorkflowId::new("delivery").expect("workflow ID should be valid"),
            vec![queued.clone(), done.clone()],
            queued.clone(),
            vec![WorkflowEdge::new(queued, done.clone())],
            vec![done],
        )
        .expect("fixture workflow should be valid")
    }

    fn transition_workflow() -> Workflow {
        let queued = phase("queued");
        let doing = phase("doing");
        let done = phase("done");
        Workflow::new(
            WorkflowId::new("delivery-rework").expect("workflow ID should be valid"),
            vec![queued.clone(), doing.clone(), done.clone()],
            queued.clone(),
            vec![
                WorkflowEdge::new(queued.clone(), doing.clone()),
                WorkflowEdge::new(doing.clone(), queued),
                WorkflowEdge::new(doing.clone(), doing.clone()),
                WorkflowEdge::new(doing, done.clone()),
            ],
            vec![done],
        )
        .expect("transition workflow should be valid")
    }

    fn fixed_id() -> IntentUnitId {
        IntentUnitId::from_str("67e55044-10b1-426f-9247-bb680e5fe0c8")
            .expect("fixed ID should parse")
    }

    fn species() -> IntentSpecies {
        IntentSpecies::new("feature").expect("species should be valid")
    }

    #[test]
    fn test_intent_unit_starts_active_at_initial_phase() {
        let workflow = workflow();
        let expected_phase = workflow.initial_phase().clone();
        let unit = IntentUnit::new(fixed_id(), species(), workflow);

        assert_eq!(unit.id(), fixed_id());
        assert_eq!(unit.species().as_str(), "feature");
        assert_eq!(unit.phase(), &expected_phase);
        assert_eq!(unit.status(), IntentUnitStatus::Active);
        assert!(unit.history().is_empty());
    }

    #[test]
    fn test_intent_unit_owns_workflow_snapshot() {
        let workflow = workflow();
        let expected = workflow.clone();
        let unit = IntentUnit::new(fixed_id(), species(), workflow);

        assert_eq!(unit.workflow(), &expected);
        assert_eq!(unit.workflow_id(), expected.id());
    }

    #[test]
    fn test_intent_unit_identity_accessors_are_stable() {
        let unit = IntentUnit::new(fixed_id(), species(), workflow());

        assert_eq!(unit.id(), unit.id());
        assert_eq!(unit.species(), unit.species());
        assert_eq!(unit.workflow_id(), unit.workflow_id());
    }

    #[test]
    fn test_allowed_transition_moves_and_appends_record() {
        let mut unit = IntentUnit::new(fixed_id(), species(), transition_workflow());
        let queued = phase("queued");
        let doing = phase("doing");

        unit.transition_to(&doing)
            .expect("declared transition should succeed");

        assert_eq!(unit.phase(), &doing);
        assert_eq!(unit.history().len(), 1);
        let LifecycleRecord::Transition(record) = &unit.history()[0] else {
            panic!("first history entry should be a transition");
        };
        assert_eq!(record.sequence(), 1);
        assert_eq!(record.from(), &queued);
        assert_eq!(record.to(), &doing);
    }

    #[test]
    fn test_disallowed_transition_is_atomic() {
        let mut unit = IntentUnit::new(fixed_id(), species(), transition_workflow());
        let before = unit.clone();
        let done = phase("done");

        let error = unit
            .transition_to(&done)
            .expect_err("undeclared edge should fail");

        assert_eq!(
            error,
            TransitionError::NotAllowed {
                from: phase("queued"),
                to: done
            }
        );
        assert_eq!(unit, before);
    }

    #[test]
    fn test_unknown_target_transition_is_atomic() {
        let mut unit = IntentUnit::new(fixed_id(), species(), transition_workflow());
        let before = unit.clone();
        let missing = phase("missing");

        let error = unit
            .transition_to(&missing)
            .expect_err("unknown target should fail");

        assert_eq!(error, TransitionError::UnknownTarget { target: missing });
        assert_eq!(unit, before);
    }

    #[test]
    fn test_configured_reverse_transition_succeeds() {
        let mut unit = IntentUnit::new(fixed_id(), species(), transition_workflow());

        unit.transition_to(&phase("doing"))
            .expect("forward edge should succeed");
        unit.transition_to(&phase("queued"))
            .expect("declared reverse edge should succeed");

        assert_eq!(unit.phase(), &phase("queued"));
        assert_eq!(unit.history().len(), 2);
    }

    #[test]
    fn test_transition_history_preserves_order() {
        let mut unit = IntentUnit::new(fixed_id(), species(), transition_workflow());
        let doing = phase("doing");
        let done = phase("done");

        unit.transition_to(&doing)
            .expect("first edge should succeed");
        unit.transition_to(&doing)
            .expect("declared self edge should succeed");
        unit.transition_to(&done)
            .expect("final edge should succeed");

        let sequences: Vec<_> = unit
            .history()
            .iter()
            .map(LifecycleRecord::sequence)
            .collect();
        assert_eq!(sequences, vec![1, 2, 3]);
        assert_eq!(unit.phase(), &done);
    }

    #[test]
    fn test_transition_preserves_identity() {
        let mut unit = IntentUnit::new(fixed_id(), species(), transition_workflow());
        let id = unit.id();
        let species = unit.species().clone();
        let workflow_id = unit.workflow_id().clone();

        unit.transition_to(&phase("doing"))
            .expect("declared edge should succeed");

        assert_eq!(unit.id(), id);
        assert_eq!(unit.species(), &species);
        assert_eq!(unit.workflow_id(), &workflow_id);
    }

    #[test]
    fn test_completion_from_eligible_phase_is_terminal() {
        let mut unit = IntentUnit::new(fixed_id(), species(), transition_workflow());
        unit.transition_to(&phase("doing"))
            .expect("first edge should succeed");
        unit.transition_to(&phase("done"))
            .expect("completion phase should be reachable");

        unit.complete().expect("eligible completion should succeed");

        assert_eq!(unit.status(), IntentUnitStatus::Completed);
        assert_eq!(unit.phase(), &phase("done"));
        assert_eq!(unit.history().len(), 3);
        let LifecycleRecord::Completion(record) = &unit.history()[2] else {
            panic!("last history entry should be completion");
        };
        assert_eq!(record.sequence(), 3);
        assert_eq!(record.final_phase(), &phase("done"));
    }

    #[test]
    fn test_completion_from_ineligible_phase_is_atomic() {
        let mut unit = IntentUnit::new(fixed_id(), species(), transition_workflow());
        let before = unit.clone();

        let error = unit
            .complete()
            .expect_err("ineligible completion should fail");

        assert_eq!(
            error,
            CompletionError::PhaseNotEligible {
                phase: phase("queued")
            }
        );
        assert_eq!(unit, before);
    }

    #[test]
    fn test_second_completion_is_rejected_without_mutation() {
        let mut unit = IntentUnit::new(fixed_id(), species(), workflow());
        unit.transition_to(&phase("done"))
            .expect("completion phase should be reachable");
        unit.complete().expect("first completion should succeed");
        let before = unit.clone();

        assert_eq!(unit.complete(), Err(CompletionError::AlreadyCompleted));
        assert_eq!(unit, before);
    }

    #[test]
    fn test_transition_after_completion_is_rejected_without_mutation() {
        let mut unit = IntentUnit::new(fixed_id(), species(), workflow());
        unit.transition_to(&phase("done"))
            .expect("completion phase should be reachable");
        unit.complete().expect("completion should succeed");
        let before = unit.clone();

        assert_eq!(
            unit.transition_to(&phase("queued")),
            Err(TransitionError::AlreadyCompleted)
        );
        assert_eq!(unit, before);
    }

    #[test]
    fn test_completion_preserves_identity_and_species() {
        let mut unit = IntentUnit::new(fixed_id(), species(), workflow());
        let id = unit.id();
        let species = unit.species().clone();
        let workflow_id = unit.workflow_id().clone();
        unit.transition_to(&phase("done"))
            .expect("completion phase should be reachable");

        unit.complete().expect("completion should succeed");

        assert_eq!(unit.id(), id);
        assert_eq!(unit.species(), &species);
        assert_eq!(unit.workflow_id(), &workflow_id);
    }
}
