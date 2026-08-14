use std::{fmt, str::FromStr};

use cubikan_core::{
    ExternalReference, IntentSpecies, IntentUnit, IntentUnitId, IntentUnitRevision,
    IntentUnitStatus, LifecycleRecord, PhaseId, Workflow, WorkflowId,
};

use crate::{ListCursorError, PageLimitError};

/// Input for creating one new revision-zero Intent Unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateIntentUnit {
    id: Option<IntentUnitId>,
    species: IntentSpecies,
    workflow: Workflow,
}

impl CreateIntentUnit {
    /// Constructs a typed create command. An absent ID requests generation.
    #[must_use]
    pub const fn new(id: Option<IntentUnitId>, species: IntentSpecies, workflow: Workflow) -> Self {
        Self {
            id,
            species,
            workflow,
        }
    }

    /// Returns the optional caller-supplied identity.
    #[must_use]
    pub const fn id(&self) -> Option<IntentUnitId> {
        self.id
    }

    /// Returns the immutable species provenance.
    #[must_use]
    pub const fn species(&self) -> &IntentSpecies {
        &self.species
    }

    /// Returns the immutable workflow snapshot to own.
    #[must_use]
    pub const fn workflow(&self) -> &Workflow {
        &self.workflow
    }

    pub(crate) fn into_parts(self) -> (Option<IntentUnitId>, IntentSpecies, Workflow) {
        (self.id, self.species, self.workflow)
    }
}

/// Input for retrieving one Intent Unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetIntentUnit {
    id: IntentUnitId,
}

impl GetIntentUnit {
    /// Constructs a typed get command.
    #[must_use]
    pub const fn new(id: IntentUnitId) -> Self {
        Self { id }
    }

    /// Returns the requested identity.
    #[must_use]
    pub const fn id(self) -> IntentUnitId {
        self.id
    }
}

/// Input for a revision-conditioned transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionIntentUnit {
    id: IntentUnitId,
    target: PhaseId,
    expected_revision: IntentUnitRevision,
}

impl TransitionIntentUnit {
    /// Constructs a typed transition command.
    #[must_use]
    pub const fn new(
        id: IntentUnitId,
        target: PhaseId,
        expected_revision: IntentUnitRevision,
    ) -> Self {
        Self {
            id,
            target,
            expected_revision,
        }
    }

    /// Returns the target identity.
    #[must_use]
    pub const fn id(&self) -> IntentUnitId {
        self.id
    }

    /// Returns the requested target phase.
    #[must_use]
    pub const fn target(&self) -> &PhaseId {
        &self.target
    }

    /// Returns the caller-observed aggregate revision.
    #[must_use]
    pub const fn expected_revision(&self) -> IntentUnitRevision {
        self.expected_revision
    }
}

/// Input for revision-conditioned terminal completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompleteIntentUnit {
    id: IntentUnitId,
    expected_revision: IntentUnitRevision,
}

impl CompleteIntentUnit {
    /// Constructs a typed completion command.
    #[must_use]
    pub const fn new(id: IntentUnitId, expected_revision: IntentUnitRevision) -> Self {
        Self {
            id,
            expected_revision,
        }
    }

    /// Returns the target identity.
    #[must_use]
    pub const fn id(self) -> IntentUnitId {
        self.id
    }

    /// Returns the caller-observed aggregate revision.
    #[must_use]
    pub const fn expected_revision(self) -> IntentUnitRevision {
        self.expected_revision
    }
}

/// Optional exact-match projections for a bounded collection query.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListFilters {
    workflow_id: Option<WorkflowId>,
    species: Option<IntentSpecies>,
    phase: Option<PhaseId>,
    status: Option<IntentUnitStatus>,
}

impl ListFilters {
    /// Constructs exact-match filters. Every field may be omitted.
    #[must_use]
    pub const fn new(
        workflow_id: Option<WorkflowId>,
        species: Option<IntentSpecies>,
        phase: Option<PhaseId>,
        status: Option<IntentUnitStatus>,
    ) -> Self {
        Self {
            workflow_id,
            species,
            phase,
            status,
        }
    }

    #[must_use]
    pub const fn workflow_id(&self) -> Option<&WorkflowId> {
        self.workflow_id.as_ref()
    }

    #[must_use]
    pub const fn species(&self) -> Option<&IntentSpecies> {
        self.species.as_ref()
    }

    #[must_use]
    pub const fn phase(&self) -> Option<&PhaseId> {
        self.phase.as_ref()
    }

    #[must_use]
    pub const fn status(&self) -> Option<IntentUnitStatus> {
        self.status
    }
}

/// Validated per-page result ceiling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageLimit(usize);

impl PageLimit {
    pub const MIN: usize = 1;
    pub const MAX: usize = 100;

    /// Validates a limit in the inclusive range 1 through 100.
    pub const fn new(value: usize) -> Result<Self, PageLimitError> {
        if value >= Self::MIN && value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(PageLimitError::new(value))
        }
    }

    #[must_use]
    pub const fn value(self) -> usize {
        self.0
    }
}

/// Canonical exclusive Intent Unit ID keyset cursor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ListCursor(IntentUnitId);

impl ListCursor {
    pub(crate) const fn from_id(id: IntentUnitId) -> Self {
        Self(id)
    }

    /// Returns the typed identity carried by this cursor.
    #[must_use]
    pub const fn id(self) -> IntentUnitId {
        self.0
    }
}

impl fmt::Display for ListCursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for ListCursor {
    type Err = ListCursorError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let id = value
            .parse::<IntentUnitId>()
            .map_err(|_| ListCursorError::Malformed)?;
        if id.to_string() != value {
            return Err(ListCursorError::NonCanonical);
        }
        Ok(Self(id))
    }
}

/// Input for one bounded live collection query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListIntentUnits {
    filters: ListFilters,
    limit: PageLimit,
    after: Option<ListCursor>,
}

impl ListIntentUnits {
    #[must_use]
    pub const fn new(filters: ListFilters, limit: PageLimit, after: Option<ListCursor>) -> Self {
        Self {
            filters,
            limit,
            after,
        }
    }

    #[must_use]
    pub const fn filters(&self) -> &ListFilters {
        &self.filters
    }

    #[must_use]
    pub const fn limit(&self) -> PageLimit {
        self.limit
    }

    #[must_use]
    pub const fn after(&self) -> Option<ListCursor> {
        self.after
    }
}

/// Complete adapter-owned semantic view of one validated Intent Unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentUnitView {
    id: IntentUnitId,
    origin: ExternalReference,
    species: IntentSpecies,
    workflow: Workflow,
    phase: PhaseId,
    status: IntentUnitStatus,
    revision: IntentUnitRevision,
    history: Vec<LifecycleRecord>,
}

impl IntentUnitView {
    #[must_use]
    pub fn from_intent_unit(unit: &IntentUnit) -> Self {
        Self {
            id: unit.id(),
            origin: unit.origin().clone(),
            species: unit.species().clone(),
            workflow: unit.workflow().clone(),
            phase: unit.phase().clone(),
            status: unit.status(),
            revision: unit.revision(),
            history: unit.history().to_vec(),
        }
    }

    #[must_use]
    pub const fn id(&self) -> IntentUnitId {
        self.id
    }
    /// Returns the immutable external origin required by the aggregate.
    #[must_use]
    pub const fn origin(&self) -> &ExternalReference {
        &self.origin
    }
    #[must_use]
    pub const fn species(&self) -> &IntentSpecies {
        &self.species
    }
    #[must_use]
    pub const fn workflow(&self) -> &Workflow {
        &self.workflow
    }
    #[must_use]
    pub const fn workflow_id(&self) -> &WorkflowId {
        self.workflow.id()
    }
    #[must_use]
    pub const fn phase(&self) -> &PhaseId {
        &self.phase
    }
    #[must_use]
    pub const fn status(&self) -> IntentUnitStatus {
        self.status
    }
    #[must_use]
    pub const fn revision(&self) -> IntentUnitRevision {
        self.revision
    }
    #[must_use]
    pub fn history(&self) -> &[LifecycleRecord] {
        &self.history
    }
}

/// Validated collection projection for one Intent Unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentUnitSummary {
    id: IntentUnitId,
    origin: ExternalReference,
    workflow_id: WorkflowId,
    species: IntentSpecies,
    phase: PhaseId,
    status: IntentUnitStatus,
    revision: IntentUnitRevision,
}

impl IntentUnitSummary {
    pub(crate) fn from_intent_unit(unit: &IntentUnit) -> Self {
        Self {
            id: unit.id(),
            origin: unit.origin().clone(),
            workflow_id: unit.workflow_id().clone(),
            species: unit.species().clone(),
            phase: unit.phase().clone(),
            status: unit.status(),
            revision: unit.revision(),
        }
    }

    #[must_use]
    pub fn from_view(view: &IntentUnitView) -> Self {
        Self {
            id: view.id,
            origin: view.origin.clone(),
            workflow_id: view.workflow.id().clone(),
            species: view.species.clone(),
            phase: view.phase.clone(),
            status: view.status,
            revision: view.revision,
        }
    }

    #[must_use]
    pub const fn id(&self) -> IntentUnitId {
        self.id
    }
    /// Returns the immutable external origin required by the aggregate.
    #[must_use]
    pub const fn origin(&self) -> &ExternalReference {
        &self.origin
    }
    #[must_use]
    pub const fn workflow_id(&self) -> &WorkflowId {
        &self.workflow_id
    }
    #[must_use]
    pub const fn species(&self) -> &IntentSpecies {
        &self.species
    }
    #[must_use]
    pub const fn phase(&self) -> &PhaseId {
        &self.phase
    }
    #[must_use]
    pub const fn status(&self) -> IntentUnitStatus {
        self.status
    }
    #[must_use]
    pub const fn revision(&self) -> IntentUnitRevision {
        self.revision
    }
}

/// One bounded page of validated collection summaries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentUnitPage {
    items: Vec<IntentUnitSummary>,
    next_cursor: Option<ListCursor>,
}

impl IntentUnitPage {
    #[must_use]
    pub const fn new(items: Vec<IntentUnitSummary>, next_cursor: Option<ListCursor>) -> Self {
        Self { items, next_cursor }
    }

    #[must_use]
    pub fn items(&self) -> &[IntentUnitSummary] {
        &self.items
    }
    #[must_use]
    pub const fn next_cursor(&self) -> Option<ListCursor> {
        self.next_cursor
    }
}

/// Adapter-owned result of one committed lifecycle mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationResult {
    committed_revision: IntentUnitRevision,
    intent_unit: IntentUnitView,
}

impl MutationResult {
    #[must_use]
    pub const fn new(committed_revision: IntentUnitRevision, intent_unit: IntentUnitView) -> Self {
        Self {
            committed_revision,
            intent_unit,
        }
    }

    #[must_use]
    pub const fn committed_revision(&self) -> IntentUnitRevision {
        self.committed_revision
    }
    #[must_use]
    pub const fn intent_unit(&self) -> &IntentUnitView {
        &self.intent_unit
    }
}

#[cfg(test)]
mod tests {
    use cubikan_core::{
        ReferenceNamespace, ReferenceText, RevisionedTransitionError, WorkflowEdge,
    };

    use super::*;
    use crate::BackendError;

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
        .expect("workflow should be valid")
    }

    fn id() -> IntentUnitId {
        "00000000-0000-0000-0000-000000000001"
            .parse()
            .expect("fixture UUID should be valid")
    }

    fn origin() -> ExternalReference {
        ExternalReference::new(
            ReferenceNamespace::new("github").expect("fixture namespace should be valid"),
            ReferenceText::new("crussella0129/CubiKan").expect("fixture scope should be valid"),
            ReferenceText::new("issue:1107").expect("fixture value should be valid"),
        )
    }

    #[test]
    fn test_public_backend_model_preserves_typed_u64_revisions() {
        for value in [0, i64::MAX as u64 + 1, u64::MAX] {
            let revision = IntentUnitRevision::new(value);
            let transition = TransitionIntentUnit::new(id(), phase("done"), revision);
            let completion = CompleteIntentUnit::new(id(), revision);
            let view = IntentUnitView {
                id: id(),
                origin: origin(),
                species: IntentSpecies::new("feature").expect("species should be valid"),
                workflow: workflow(),
                phase: phase("queued"),
                status: IntentUnitStatus::Active,
                revision,
                history: Vec::new(),
            };
            let summary = IntentUnitSummary::from_view(&view);
            let page = IntentUnitPage::new(vec![summary.clone()], None);
            let mutation = MutationResult::new(revision, view.clone());

            assert_eq!(transition.expected_revision().value(), value);
            assert_eq!(completion.expected_revision().value(), value);
            assert_eq!(view.revision().value(), value);
            assert_eq!(summary.origin(), view.origin());
            assert_eq!(summary.revision().value(), value);
            assert_eq!(page.items()[0].revision().value(), value);
            assert_eq!(mutation.committed_revision().value(), value);
            assert_eq!(mutation.intent_unit().revision().value(), value);

            let mut aggregate = IntentUnit::new(
                id(),
                origin(),
                IntentSpecies::new("feature").expect("species should be valid"),
                workflow(),
            );
            if value == 0 {
                aggregate
                    .transition_to(&phase("done"))
                    .expect("fixture transition should succeed");
            }
            let error = aggregate
                .transition_to_if_revision(&phase("done"), revision)
                .expect_err("fixture expectation should be stale");
            let RevisionedTransitionError::Conflict(conflict) = error else {
                panic!("expected a revision conflict");
            };
            assert_eq!(conflict.expected().value(), value);
            assert_eq!(conflict.actual().value(), if value == 0 { 1 } else { 0 });
            assert_eq!(
                BackendError::RevisionConflict(conflict.clone()),
                BackendError::RevisionConflict(conflict)
            );
        }
    }
}
