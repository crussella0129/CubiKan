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
}

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
}
