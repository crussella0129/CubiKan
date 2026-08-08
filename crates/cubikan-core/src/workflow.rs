use std::{collections::HashSet, error::Error, fmt};

use crate::{PhaseId, WorkflowId};

/// A directed edge declared by a workflow.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WorkflowEdge {
    from: PhaseId,
    to: PhaseId,
}

impl WorkflowEdge {
    /// Creates a directed edge from one declared phase to another.
    #[must_use]
    pub const fn new(from: PhaseId, to: PhaseId) -> Self {
        Self { from, to }
    }

    /// Returns the source phase.
    #[must_use]
    pub const fn from(&self) -> &PhaseId {
        &self.from
    }

    /// Returns the target phase.
    #[must_use]
    pub const fn to(&self) -> &PhaseId {
        &self.to
    }
}

/// Immutable, caller-declared lifecycle policy for Intent Units.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Workflow {
    id: WorkflowId,
    phases: Vec<PhaseId>,
    initial_phase: PhaseId,
    edges: Vec<WorkflowEdge>,
    completion_phases: Vec<PhaseId>,
}

impl Workflow {
    /// Validates and constructs a directed workflow definition.
    pub fn new(
        id: WorkflowId,
        phases: impl IntoIterator<Item = PhaseId>,
        initial_phase: PhaseId,
        edges: impl IntoIterator<Item = WorkflowEdge>,
        completion_phases: impl IntoIterator<Item = PhaseId>,
    ) -> Result<Self, WorkflowError> {
        let phases: Vec<_> = phases.into_iter().collect();
        if phases.is_empty() {
            return Err(WorkflowError::EmptyPhases);
        }

        let mut declared_phases = HashSet::with_capacity(phases.len());
        for phase in &phases {
            if !declared_phases.insert(phase) {
                return Err(WorkflowError::DuplicatePhase {
                    phase: phase.clone(),
                });
            }
        }

        if !declared_phases.contains(&initial_phase) {
            return Err(WorkflowError::UnknownInitialPhase {
                phase: initial_phase,
            });
        }

        let edges: Vec<_> = edges.into_iter().collect();
        let mut declared_edges = HashSet::with_capacity(edges.len());
        for edge in &edges {
            if !declared_phases.contains(edge.from()) {
                return Err(WorkflowError::UnknownEdgeSource {
                    phase: edge.from().clone(),
                });
            }
            if !declared_phases.contains(edge.to()) {
                return Err(WorkflowError::UnknownEdgeTarget {
                    phase: edge.to().clone(),
                });
            }
            if !declared_edges.insert(edge) {
                return Err(WorkflowError::DuplicateEdge { edge: edge.clone() });
            }
        }

        let completion_phases: Vec<_> = completion_phases.into_iter().collect();
        let mut declared_completion_phases = HashSet::with_capacity(completion_phases.len());
        for phase in &completion_phases {
            if !declared_phases.contains(phase) {
                return Err(WorkflowError::UnknownCompletionPhase {
                    phase: phase.clone(),
                });
            }
            if !declared_completion_phases.insert(phase) {
                return Err(WorkflowError::DuplicateCompletionPhase {
                    phase: phase.clone(),
                });
            }
        }

        Ok(Self {
            id,
            phases,
            initial_phase,
            edges,
            completion_phases,
        })
    }

    /// Returns the workflow identity.
    #[must_use]
    pub const fn id(&self) -> &WorkflowId {
        &self.id
    }

    /// Returns declared phases in caller order.
    #[must_use]
    pub fn phases(&self) -> &[PhaseId] {
        &self.phases
    }

    /// Returns the phase assigned to newly created Intent Units.
    #[must_use]
    pub const fn initial_phase(&self) -> &PhaseId {
        &self.initial_phase
    }

    /// Returns directed edges in caller order.
    #[must_use]
    pub fn edges(&self) -> &[WorkflowEdge] {
        &self.edges
    }

    /// Returns completion-eligible phases in caller order.
    #[must_use]
    pub fn completion_phases(&self) -> &[PhaseId] {
        &self.completion_phases
    }

    /// Reports whether the phase belongs to this workflow.
    #[must_use]
    pub fn contains_phase(&self, phase: &PhaseId) -> bool {
        self.phases.contains(phase)
    }

    /// Reports whether the exact directed edge was declared.
    #[must_use]
    pub fn allows_transition(&self, from: &PhaseId, to: &PhaseId) -> bool {
        self.edges
            .iter()
            .any(|edge| edge.from() == from && edge.to() == to)
    }

    /// Reports whether an Intent Unit may complete in this phase.
    #[must_use]
    pub fn allows_completion(&self, phase: &PhaseId) -> bool {
        self.completion_phases.contains(phase)
    }
}

/// Validation failure for a caller-declared workflow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkflowError {
    /// A workflow must declare at least one phase.
    EmptyPhases,
    /// The phase list contains the same identity more than once.
    DuplicatePhase { phase: PhaseId },
    /// The initial phase does not belong to the workflow.
    UnknownInitialPhase { phase: PhaseId },
    /// An edge starts at a phase that does not belong to the workflow.
    UnknownEdgeSource { phase: PhaseId },
    /// An edge targets a phase that does not belong to the workflow.
    UnknownEdgeTarget { phase: PhaseId },
    /// The same directed edge was declared more than once.
    DuplicateEdge { edge: WorkflowEdge },
    /// A completion phase does not belong to the workflow.
    UnknownCompletionPhase { phase: PhaseId },
    /// The same completion phase was declared more than once.
    DuplicateCompletionPhase { phase: PhaseId },
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPhases => formatter.write_str("workflow must declare at least one phase"),
            Self::DuplicatePhase { phase } => {
                write!(formatter, "workflow repeats phase `{phase}`")
            }
            Self::UnknownInitialPhase { phase } => {
                write!(formatter, "initial phase `{phase}` is not declared")
            }
            Self::UnknownEdgeSource { phase } => {
                write!(formatter, "edge source phase `{phase}` is not declared")
            }
            Self::UnknownEdgeTarget { phase } => {
                write!(formatter, "edge target phase `{phase}` is not declared")
            }
            Self::DuplicateEdge { edge } => write!(
                formatter,
                "workflow repeats edge `{} -> {}`",
                edge.from(),
                edge.to()
            ),
            Self::UnknownCompletionPhase { phase } => {
                write!(formatter, "completion phase `{phase}` is not declared")
            }
            Self::DuplicateCompletionPhase { phase } => {
                write!(formatter, "workflow repeats completion phase `{phase}`")
            }
        }
    }
}

impl Error for WorkflowError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn phase(value: &str) -> PhaseId {
        PhaseId::new(value).expect("fixture phase should be valid")
    }

    fn workflow_id() -> WorkflowId {
        WorkflowId::new("support-flow").expect("fixture workflow ID should be valid")
    }

    #[test]
    fn test_workflow_accepts_explicit_topology() {
        let queued = phase("queued");
        let doing = phase("doing");
        let done = phase("done");
        let workflow = Workflow::new(
            workflow_id(),
            vec![queued.clone(), doing.clone(), done.clone()],
            queued.clone(),
            vec![
                WorkflowEdge::new(queued.clone(), doing.clone()),
                WorkflowEdge::new(doing.clone(), done.clone()),
            ],
            vec![done.clone()],
        )
        .expect("workflow should be valid");

        assert_eq!(workflow.initial_phase(), &queued);
        assert!(workflow.allows_transition(&doing, &done));
        assert!(!workflow.allows_transition(&done, &doing));
        assert!(workflow.allows_completion(&done));
        assert!(!workflow.allows_completion(&doing));
    }

    #[test]
    fn test_workflow_rejects_empty_phase_set() {
        assert_eq!(
            Workflow::new(workflow_id(), vec![], phase("missing"), vec![], vec![]),
            Err(WorkflowError::EmptyPhases)
        );
    }

    #[test]
    fn test_workflow_rejects_duplicate_phase() {
        let queued = phase("queued");
        let duplicate_phase = Workflow::new(
            workflow_id(),
            vec![queued.clone(), queued.clone()],
            queued.clone(),
            vec![],
            vec![],
        );
        let duplicate_completion = Workflow::new(
            workflow_id(),
            vec![queued.clone()],
            queued.clone(),
            vec![],
            vec![queued.clone(), queued.clone()],
        );

        assert_eq!(
            duplicate_phase,
            Err(WorkflowError::DuplicatePhase {
                phase: queued.clone()
            })
        );
        assert_eq!(
            duplicate_completion,
            Err(WorkflowError::DuplicateCompletionPhase { phase: queued })
        );
    }

    #[test]
    fn test_workflow_rejects_duplicate_edge() {
        let queued = phase("queued");
        let doing = phase("doing");
        let edge = WorkflowEdge::new(queued.clone(), doing.clone());

        assert_eq!(
            Workflow::new(
                workflow_id(),
                vec![queued.clone(), doing],
                queued,
                vec![edge.clone(), edge.clone()],
                vec![],
            ),
            Err(WorkflowError::DuplicateEdge { edge })
        );
    }

    #[test]
    fn test_workflow_rejects_unknown_initial_phase() {
        let queued = phase("queued");
        let missing = phase("missing");

        assert_eq!(
            Workflow::new(workflow_id(), vec![queued], missing.clone(), vec![], vec![],),
            Err(WorkflowError::UnknownInitialPhase { phase: missing })
        );
    }

    #[test]
    fn test_workflow_rejects_unknown_edge_source() {
        let queued = phase("queued");
        let missing = phase("missing");

        assert_eq!(
            Workflow::new(
                workflow_id(),
                vec![queued.clone()],
                queued.clone(),
                vec![WorkflowEdge::new(missing.clone(), queued)],
                vec![],
            ),
            Err(WorkflowError::UnknownEdgeSource { phase: missing })
        );
    }

    #[test]
    fn test_workflow_rejects_unknown_edge_target() {
        let queued = phase("queued");
        let missing = phase("missing");

        assert_eq!(
            Workflow::new(
                workflow_id(),
                vec![queued.clone()],
                queued.clone(),
                vec![WorkflowEdge::new(queued, missing.clone())],
                vec![],
            ),
            Err(WorkflowError::UnknownEdgeTarget { phase: missing })
        );
    }

    #[test]
    fn test_workflow_rejects_unknown_completion_phase() {
        let queued = phase("queued");
        let missing = phase("missing");

        assert_eq!(
            Workflow::new(
                workflow_id(),
                vec![queued.clone()],
                queued,
                vec![],
                vec![missing.clone()],
            ),
            Err(WorkflowError::UnknownCompletionPhase { phase: missing })
        );
    }

    #[test]
    fn test_workflow_allows_only_declared_reverse_and_self_edges() {
        let queued = phase("queued");
        let doing = phase("doing");
        let workflow = Workflow::new(
            workflow_id(),
            vec![queued.clone(), doing.clone()],
            queued.clone(),
            vec![
                WorkflowEdge::new(queued.clone(), doing.clone()),
                WorkflowEdge::new(doing.clone(), queued.clone()),
                WorkflowEdge::new(doing.clone(), doing.clone()),
            ],
            vec![],
        )
        .expect("workflow should be valid");

        assert!(workflow.allows_transition(&doing, &queued));
        assert!(workflow.allows_transition(&doing, &doing));
        assert!(!workflow.allows_transition(&queued, &queued));
    }
}
