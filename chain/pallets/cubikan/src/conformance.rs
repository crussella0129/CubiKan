//! Cross-runtime conformance adapters and structural decoding instrumentation.
//!
//! This module deliberately contains no fixture parser. The checked fixture is
//! an independent oracle; tests translate its raw inputs into these typed
//! adapters rather than teaching production code about JSON.

use parity_scale_codec::Decode;

use crate::types::MAX_ACCEPTED_EVENT_BYTES;
use crate::types::{
    DefinitionVersion, DefinitionVersionError, ExternalReference, IntentSpecies, Namespace,
    NamespaceError, PhaseId, ReferenceScope, ReferenceValue, TextError, Workflow, WorkflowEdge,
    WorkflowError, WorkflowId, MAX_ACTIVE_ASSOCIATIONS, MAX_AUTHORIZED_SUBMITTERS,
    MAX_LIFECYCLE_RECORDS, MAX_RELATIONSHIP_EDGES,
};

/// Field whose exact bytes are being validated by a conformance case.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextField {
    Namespace,
    Scope,
    Value,
    WorkflowId,
    Phase,
    Species,
    DefinitionId,
}

/// Stable, allocation-free result classification shared with the fixture test.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConformanceError {
    Empty {
        field: TextField,
    },
    TooLong {
        field: TextField,
        length: usize,
        maximum: usize,
    },
    InvalidUtf8 {
        field: TextField,
        index: usize,
        length: usize,
    },
    Nul {
        field: TextField,
        index: usize,
    },
    Blank {
        field: TextField,
    },
    InvalidNamespaceStart {
        field: TextField,
        index: usize,
        byte: u8,
    },
    InvalidNamespaceByte {
        field: TextField,
        index: usize,
        byte: u8,
    },
    DefinitionVersionZero,
    EmptyPhases,
    TooManyPhases {
        length: usize,
        maximum: usize,
    },
    TooManyWorkflowEdges {
        length: usize,
        maximum: usize,
    },
    TooManyCompletionPhases {
        length: usize,
        maximum: usize,
    },
    DuplicatePhase {
        first: usize,
        duplicate: usize,
    },
    UnknownInitialPhase,
    UnknownEdgeSource {
        edge: usize,
    },
    UnknownEdgeTarget {
        edge: usize,
    },
    DuplicateWorkflowEdge {
        first: usize,
        duplicate: usize,
    },
    UnknownCompletionPhase {
        completion: usize,
    },
    DuplicateCompletionPhase {
        first: usize,
        duplicate: usize,
    },
}

fn map_text_error(field: TextField, error: TextError) -> ConformanceError {
    match error {
        TextError::Empty => ConformanceError::Empty { field },
        TextError::TooLong { length, maximum } => ConformanceError::TooLong {
            field,
            length,
            maximum,
        },
        TextError::InvalidUtf8 { index, length } => ConformanceError::InvalidUtf8 {
            field,
            index,
            length,
        },
        TextError::Nul { index } => ConformanceError::Nul { field, index },
        TextError::Blank => ConformanceError::Blank { field },
    }
}

fn map_namespace_error(field: TextField, error: NamespaceError) -> ConformanceError {
    match error {
        NamespaceError::Empty => ConformanceError::Empty { field },
        NamespaceError::TooLong { length, maximum } => ConformanceError::TooLong {
            field,
            length,
            maximum,
        },
        NamespaceError::InvalidUtf8 { index, length } => ConformanceError::InvalidUtf8 {
            field,
            index,
            length,
        },
        NamespaceError::Nul { index } => ConformanceError::Nul { field, index },
        NamespaceError::InvalidStart { index, byte } => {
            ConformanceError::InvalidNamespaceStart { field, index, byte }
        }
        NamespaceError::InvalidByte { index, byte } => {
            ConformanceError::InvalidNamespaceByte { field, index, byte }
        }
    }
}

fn map_workflow_error(error: WorkflowError) -> ConformanceError {
    match error {
        WorkflowError::EmptyPhases => ConformanceError::EmptyPhases,
        WorkflowError::TooManyPhases { length, maximum } => {
            ConformanceError::TooManyPhases { length, maximum }
        }
        WorkflowError::TooManyEdges { length, maximum } => {
            ConformanceError::TooManyWorkflowEdges { length, maximum }
        }
        WorkflowError::TooManyCompletionPhases { length, maximum } => {
            ConformanceError::TooManyCompletionPhases { length, maximum }
        }
        WorkflowError::DuplicatePhase { first, duplicate } => {
            ConformanceError::DuplicatePhase { first, duplicate }
        }
        WorkflowError::UnknownInitialPhase => ConformanceError::UnknownInitialPhase,
        WorkflowError::UnknownEdgeSource { edge } => ConformanceError::UnknownEdgeSource { edge },
        WorkflowError::UnknownEdgeTarget { edge } => ConformanceError::UnknownEdgeTarget { edge },
        WorkflowError::DuplicateEdge { first, duplicate } => {
            ConformanceError::DuplicateWorkflowEdge { first, duplicate }
        }
        WorkflowError::UnknownCompletionPhase { completion } => {
            ConformanceError::UnknownCompletionPhase { completion }
        }
        WorkflowError::DuplicateCompletionPhase { first, duplicate } => {
            ConformanceError::DuplicateCompletionPhase { first, duplicate }
        }
    }
}

/// Validates a namespace used by an external reference.
pub fn namespace(bytes: &[u8]) -> Result<Namespace, ConformanceError> {
    Namespace::try_from_bytes(bytes)
        .map_err(|error| map_namespace_error(TextField::Namespace, error))
}

/// Validates a relationship-definition identifier using the namespace grammar.
pub fn definition_id(bytes: &[u8]) -> Result<Namespace, ConformanceError> {
    Namespace::try_from_bytes(bytes)
        .map_err(|error| map_namespace_error(TextField::DefinitionId, error))
}

/// Validates an external-reference scope.
pub fn scope(bytes: &[u8]) -> Result<ReferenceScope, ConformanceError> {
    ReferenceScope::try_from_bytes(bytes).map_err(|error| map_text_error(TextField::Scope, error))
}

/// Validates an external-reference value.
pub fn reference_value(bytes: &[u8]) -> Result<ReferenceValue, ConformanceError> {
    ReferenceValue::try_from_bytes(bytes).map_err(|error| map_text_error(TextField::Value, error))
}

/// Validates caller-defined Intent Unit species text.
pub fn species(bytes: &[u8]) -> Result<IntentSpecies, ConformanceError> {
    IntentSpecies::try_from_bytes(bytes).map_err(|error| map_text_error(TextField::Species, error))
}

/// Validates an immutable workflow identifier.
pub fn workflow_id(bytes: &[u8]) -> Result<WorkflowId, ConformanceError> {
    WorkflowId::try_from_bytes(bytes).map_err(|error| map_text_error(TextField::WorkflowId, error))
}

/// Validates an immutable workflow phase identifier.
pub fn phase_id(bytes: &[u8]) -> Result<PhaseId, ConformanceError> {
    PhaseId::try_from_bytes(bytes).map_err(|error| map_text_error(TextField::Phase, error))
}

/// Validates all three components of one external reference in field order.
pub fn external_reference(
    namespace_bytes: &[u8],
    scope_bytes: &[u8],
    value_bytes: &[u8],
) -> Result<ExternalReference, ConformanceError> {
    Ok(ExternalReference::new(
        namespace(namespace_bytes)?,
        scope(scope_bytes)?,
        reference_value(value_bytes)?,
    ))
}

/// Validates a positive definition version.
pub fn definition_version(value: u64) -> Result<DefinitionVersion, ConformanceError> {
    DefinitionVersion::try_new(value).map_err(|error| match error {
        DefinitionVersionError::Zero => ConformanceError::DefinitionVersionZero,
    })
}

/// Raw workflow input whose element text has already crossed value validation.
pub struct WorkflowInput<'a> {
    pub id: WorkflowId,
    pub phases: &'a [PhaseId],
    pub initial_phase: PhaseId,
    pub edges: &'a [WorkflowEdge],
    pub completion_phases: &'a [PhaseId],
}

/// Applies the bounded workflow topology contract in deterministic order.
pub fn workflow(input: WorkflowInput<'_>) -> Result<Workflow, ConformanceError> {
    Workflow::try_new(
        input.id,
        input.phases,
        input.initial_phase,
        input.edges,
        input.completion_phases,
    )
    .map_err(map_workflow_error)
}

/// Runtime-only rejection classes that are intentionally absent from core.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainOnlyError {
    UnauthorizedSubmitter,
    AuthorizedSubmitterCapacityExceeded { length: usize, maximum: usize },
    LifecycleHistoryCapacityExceeded { length: usize, maximum: usize },
    RelationshipEdgeCapacityExceeded { length: usize, maximum: usize },
    ActiveAssociationCapacityExceeded { length: usize, maximum: usize },
}

/// Lower inclusive bound for projected read query limits.
pub const MIN_PROJECTED_QUERY_LIMIT: usize = 1;
/// Upper inclusive bound for projected read query limits.
pub const MAX_PROJECTED_QUERY_LIMIT: usize = 100;

/// Stable rejection for an out-of-range projected query limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundedLimitError {
    QueryLimitOutOfRange {
        value: usize,
        minimum: usize,
        maximum: usize,
    },
}

/// Distinguishes the technical allowlist from shared domain validation.
pub const fn require_authorized(authorized: bool) -> Result<(), ChainOnlyError> {
    if authorized {
        Ok(())
    } else {
        Err(ChainOnlyError::UnauthorizedSubmitter)
    }
}

/// Applies the complete technical submitter allowlist bound.
pub const fn require_authorized_submitter_capacity(length: usize) -> Result<(), ChainOnlyError> {
    if length <= MAX_AUTHORIZED_SUBMITTERS {
        Ok(())
    } else {
        Err(ChainOnlyError::AuthorizedSubmitterCapacityExceeded {
            length,
            maximum: MAX_AUTHORIZED_SUBMITTERS,
        })
    }
}

/// Validates the inclusive projected read query limit range.
pub const fn projected_query_limit(limit: usize) -> Result<usize, BoundedLimitError> {
    if limit < MIN_PROJECTED_QUERY_LIMIT || limit > MAX_PROJECTED_QUERY_LIMIT {
        Err(BoundedLimitError::QueryLimitOutOfRange {
            value: limit,
            minimum: MIN_PROJECTED_QUERY_LIMIT,
            maximum: MAX_PROJECTED_QUERY_LIMIT,
        })
    } else {
        Ok(limit)
    }
}

/// Applies the current-generation lifecycle storage capacity.
pub const fn require_lifecycle_capacity(length: usize) -> Result<(), ChainOnlyError> {
    if length < MAX_LIFECYCLE_RECORDS {
        Ok(())
    } else {
        Err(ChainOnlyError::LifecycleHistoryCapacityExceeded {
            length,
            maximum: MAX_LIFECYCLE_RECORDS,
        })
    }
}

/// Applies the per-definition active edge capacity.
pub const fn require_relationship_capacity(length: usize) -> Result<(), ChainOnlyError> {
    if length < MAX_RELATIONSHIP_EDGES {
        Ok(())
    } else {
        Err(ChainOnlyError::RelationshipEdgeCapacityExceeded {
            length,
            maximum: MAX_RELATIONSHIP_EDGES,
        })
    }
}

/// Applies the per-unit active association capacity.
pub const fn require_association_capacity(length: usize) -> Result<(), ChainOnlyError> {
    if length < MAX_ACTIVE_ASSOCIATIONS {
        Ok(())
    } else {
        Err(ChainOnlyError::ActiveAssociationCapacityExceeded {
            length,
            maximum: MAX_ACTIVE_ASSOCIATIONS,
        })
    }
}

/// Observable boundary counters for codec-versus-domain rejection tests.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BoundaryCounters {
    pallet_entries: u32,
    domain_reads: u32,
    mutations: u32,
    accepted_events: u32,
}

impl BoundaryCounters {
    #[must_use]
    pub const fn pallet_entries(&self) -> u32 {
        self.pallet_entries
    }

    #[must_use]
    pub const fn domain_reads(&self) -> u32 {
        self.domain_reads
    }

    #[must_use]
    pub const fn mutations(&self) -> u32 {
        self.mutations
    }

    #[must_use]
    pub const fn accepted_events(&self) -> u32 {
        self.accepted_events
    }
}

/// Structural rejection produced before any pallet-owned observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralDecodeError {
    Codec,
    TrailingBytes { remaining: usize },
}

/// Decodes an exact SCALE payload and records entry only after full success.
///
/// Failed calls leave every counter byte-identical. Successful decode records
/// only the handoff to dispatch; domain reads, mutations, and accepted events
/// remain the responsibility of the pallet operation.
pub fn decode_before_dispatch<T: Decode>(
    bytes: &[u8],
    counters: &mut BoundaryCounters,
) -> Result<T, StructuralDecodeError> {
    let mut input = bytes;
    let decoded = T::decode(&mut input).map_err(|_| StructuralDecodeError::Codec)?;
    if !input.is_empty() {
        return Err(StructuralDecodeError::TrailingBytes {
            remaining: input.len(),
        });
    }
    counters.pallet_entries = counters.pallet_entries.saturating_add(1);
    Ok(decoded)
}

/// Compile-time-independent bound check used by maximal fixture assertions.
#[must_use]
pub const fn accepted_event_bound_is_valid(max_encoded_len: usize) -> bool {
    max_encoded_len <= MAX_ACCEPTED_EVENT_BYTES
}
