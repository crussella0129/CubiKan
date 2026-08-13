//! Bounded, provider-neutral SCALE values used by `pallet-cubikan`.
//!
//! The constructors in this module are the only way to create the text and
//! topology values. Their fields are private, and their SCALE decoders repeat
//! the same validation, so malformed bytes cannot manufacture an invalid value.

use core::str;

use frame_support::{traits::ConstU32, BoundedVec};
use parity_scale_codec::{
    Decode, DecodeWithMemTracking, Encode, Error as CodecError, Input, MaxEncodedLen,
};
use scale_info::TypeInfo;

/// Maximum UTF-8 byte length of every free-text domain value.
pub const MAX_TEXT_BYTES: usize = 256;
/// Maximum byte length of a reference namespace or definition identifier.
pub const MAX_NAMESPACE_BYTES: usize = 64;
/// Maximum number of declared workflow phases.
pub const MAX_WORKFLOW_PHASES: usize = 32;
/// Maximum number of declared directed workflow edges.
pub const MAX_WORKFLOW_EDGES: usize = 128;
/// Maximum number of completion-eligible workflow phases.
pub const MAX_COMPLETION_PHASES: usize = 32;
/// Maximum number of lifecycle records retained by one unit.
pub const MAX_LIFECYCLE_RECORDS: usize = 256;
/// Maximum number of live edges for one exact relationship definition.
pub const MAX_RELATIONSHIP_EDGES: usize = 128;
/// Maximum number of live provenance associations for one unit.
pub const MAX_ACTIVE_ASSOCIATIONS: usize = 128;
/// Maximum number of technical submitters in the runtime allowlist.
pub const MAX_AUTHORIZED_SUBMITTERS: usize = 16;
/// Maximum encoded bytes accepted for one canonical event payload.
pub const MAX_ACCEPTED_EVENT_BYTES: usize = 1_048_576;

type TextBytes = BoundedVec<u8, ConstU32<256>>;
type NamespaceBytes = BoundedVec<u8, ConstU32<64>>;

/// Validation failure for nonblank, NUL-free, bounded UTF-8 text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextError {
    /// The input has no bytes.
    Empty,
    /// The input exceeds the exact byte ceiling.
    TooLong { length: usize, maximum: usize },
    /// The input is not UTF-8; `index` and `length` identify the invalid bytes.
    InvalidUtf8 { index: usize, length: usize },
    /// The input contains a NUL byte at `index`.
    Nul { index: usize },
    /// The UTF-8 input contains only Unicode whitespace.
    Blank,
}

/// Validation failure for `[a-z][a-z0-9._-]{0,63}` namespace bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamespaceError {
    /// The input has no bytes.
    Empty,
    /// The input exceeds the exact byte ceiling.
    TooLong { length: usize, maximum: usize },
    /// The input is not UTF-8; `index` and `length` identify the invalid bytes.
    InvalidUtf8 { index: usize, length: usize },
    /// The input contains a NUL byte at `index`.
    Nul { index: usize },
    /// Byte zero is not a lowercase ASCII letter.
    InvalidStart { index: usize, byte: u8 },
    /// A later byte is outside the locked namespace grammar.
    InvalidByte { index: usize, byte: u8 },
}

fn bounded_text_bytes(bytes: &[u8]) -> Result<TextBytes, TextError> {
    if bytes.is_empty() {
        return Err(TextError::Empty);
    }
    if bytes.len() > MAX_TEXT_BYTES {
        return Err(TextError::TooLong {
            length: bytes.len(),
            maximum: MAX_TEXT_BYTES,
        });
    }

    let text = str::from_utf8(bytes).map_err(|error| {
        let index = error.valid_up_to();
        TextError::InvalidUtf8 {
            index,
            length: error.error_len().unwrap_or(bytes.len() - index),
        }
    })?;
    if let Some(index) = bytes.iter().position(|byte| *byte == 0) {
        return Err(TextError::Nul { index });
    }
    if text.trim().is_empty() {
        return Err(TextError::Blank);
    }

    let mut bounded = TextBytes::default();
    for byte in bytes {
        bounded.try_push(*byte).map_err(|_| TextError::TooLong {
            length: bytes.len(),
            maximum: MAX_TEXT_BYTES,
        })?;
    }
    Ok(bounded)
}

fn bounded_namespace_bytes(bytes: &[u8]) -> Result<NamespaceBytes, NamespaceError> {
    if bytes.is_empty() {
        return Err(NamespaceError::Empty);
    }
    if bytes.len() > MAX_NAMESPACE_BYTES {
        return Err(NamespaceError::TooLong {
            length: bytes.len(),
            maximum: MAX_NAMESPACE_BYTES,
        });
    }
    str::from_utf8(bytes).map_err(|error| {
        let index = error.valid_up_to();
        NamespaceError::InvalidUtf8 {
            index,
            length: error.error_len().unwrap_or(bytes.len() - index),
        }
    })?;
    if let Some(index) = bytes.iter().position(|byte| *byte == 0) {
        return Err(NamespaceError::Nul { index });
    }
    if !bytes[0].is_ascii_lowercase() {
        return Err(NamespaceError::InvalidStart {
            index: 0,
            byte: bytes[0],
        });
    }
    if let Some((index, byte)) = bytes.iter().copied().enumerate().skip(1).find(|(_, byte)| {
        !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-'))
    }) {
        return Err(NamespaceError::InvalidByte { index, byte });
    }

    let mut bounded = NamespaceBytes::default();
    for byte in bytes {
        bounded
            .try_push(*byte)
            .map_err(|_| NamespaceError::TooLong {
                length: bytes.len(),
                maximum: MAX_NAMESPACE_BYTES,
            })?;
    }
    Ok(bounded)
}

/// Exact nonblank, NUL-free UTF-8 text containing at most 256 bytes.
#[derive(Clone, Debug, Encode, Eq, MaxEncodedLen, Ord, PartialEq, PartialOrd, TypeInfo)]
pub struct Text256(TextBytes);

impl Text256 {
    /// Validates bytes without trimming, folding, or normalization.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, TextError> {
        bounded_text_bytes(bytes).map(Self)
    }

    /// Returns the exact validated bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Returns the exact validated UTF-8 text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        str::from_utf8(self.as_bytes()).expect("Text256 can contain only validated UTF-8")
    }
}

impl Decode for Text256 {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        let bytes = TextBytes::decode(input)?;
        Self::try_from_bytes(bytes.as_slice()).map_err(|_| "invalid bounded UTF-8 text".into())
    }
}

impl DecodeWithMemTracking for Text256 {}

impl core::hash::Hash for Text256 {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(self.as_bytes(), state);
    }
}

impl TryFrom<&str> for Text256 {
    type Error = TextError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from_bytes(value.as_bytes())
    }
}

/// Exact namespace text with grammar `[a-z][a-z0-9._-]{0,63}`.
#[derive(Clone, Debug, Encode, Eq, MaxEncodedLen, Ord, PartialEq, PartialOrd, TypeInfo)]
pub struct Namespace(NamespaceBytes);

impl Namespace {
    /// Validates bytes without trimming, folding, or normalization.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, NamespaceError> {
        bounded_namespace_bytes(bytes).map(Self)
    }

    /// Returns the exact validated bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Returns the exact validated ASCII text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        str::from_utf8(self.as_bytes()).expect("Namespace can contain only validated ASCII")
    }
}

impl Decode for Namespace {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        let bytes = NamespaceBytes::decode(input)?;
        Self::try_from_bytes(bytes.as_slice()).map_err(|_| "invalid namespace".into())
    }
}

impl DecodeWithMemTracking for Namespace {}

impl core::hash::Hash for Namespace {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(self.as_bytes(), state);
    }
}

impl TryFrom<&str> for Namespace {
    type Error = NamespaceError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from_bytes(value.as_bytes())
    }
}

macro_rules! text_value {
    ($(#[$attribute:meta])* $name:ident) => {
        $(#[$attribute])*
        #[derive(
            Clone,
            Debug,
            Decode,
            DecodeWithMemTracking,
            Encode,
            Eq,
            Hash,
            MaxEncodedLen,
            Ord,
            PartialEq,
            PartialOrd,
            TypeInfo,
        )]
        pub struct $name(Text256);

        impl $name {
            /// Validates and preserves the supplied bytes exactly.
            pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, TextError> {
                Text256::try_from_bytes(bytes).map(Self)
            }

            /// Returns the original UTF-8 bytes.
            #[must_use]
            pub fn as_bytes(&self) -> &[u8] {
                self.0.as_bytes()
            }

            /// Returns the original UTF-8 text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl TryFrom<&str> for $name {
            type Error = TextError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::try_from_bytes(value.as_bytes())
            }
        }
    };
}

text_value!(
    /// Provider-owned scope component of an external reference.
    ReferenceScope
);
text_value!(
    /// Provider-owned value component of an external reference.
    ReferenceValue
);
text_value!(
    /// Identity of one immutable workflow snapshot.
    WorkflowId
);
text_value!(
    /// Identity of one workflow phase.
    PhaseId
);
text_value!(
    /// Caller-defined species provenance for one Intent Unit.
    IntentSpecies
);

/// Complete provider-neutral external reference identity.
#[derive(
    Clone,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    Hash,
    MaxEncodedLen,
    Ord,
    PartialEq,
    PartialOrd,
    TypeInfo,
)]
pub struct ExternalReference {
    namespace: Namespace,
    scope: ReferenceScope,
    value: ReferenceValue,
}

impl ExternalReference {
    /// Constructs one complete validated reference identity.
    #[must_use]
    pub const fn new(namespace: Namespace, scope: ReferenceScope, value: ReferenceValue) -> Self {
        Self {
            namespace,
            scope,
            value,
        }
    }

    #[must_use]
    pub const fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    #[must_use]
    pub const fn scope(&self) -> &ReferenceScope {
        &self.scope
    }

    #[must_use]
    pub const fn value(&self) -> &ReferenceValue {
        &self.value
    }
}

/// Caller-supplied UUID bytes. The runtime never generates an identifier.
#[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    Hash,
    MaxEncodedLen,
    Ord,
    PartialEq,
    PartialOrd,
    TypeInfo,
)]
pub struct IntentUnitId([u8; 16]);

impl IntentUnitId {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// One directed edge in an immutable workflow snapshot.
#[derive(
    Clone,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    Hash,
    MaxEncodedLen,
    PartialEq,
    TypeInfo,
)]
pub struct WorkflowEdge {
    from: PhaseId,
    to: PhaseId,
}

impl WorkflowEdge {
    #[must_use]
    pub const fn new(from: PhaseId, to: PhaseId) -> Self {
        Self { from, to }
    }

    #[must_use]
    pub const fn from(&self) -> &PhaseId {
        &self.from
    }

    #[must_use]
    pub const fn to(&self) -> &PhaseId {
        &self.to
    }
}

/// Typed validation failure for an immutable workflow snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkflowError {
    EmptyPhases,
    TooManyPhases { length: usize, maximum: usize },
    TooManyEdges { length: usize, maximum: usize },
    TooManyCompletionPhases { length: usize, maximum: usize },
    DuplicatePhase { first: usize, duplicate: usize },
    UnknownInitialPhase,
    UnknownEdgeSource { edge: usize },
    UnknownEdgeTarget { edge: usize },
    DuplicateEdge { first: usize, duplicate: usize },
    UnknownCompletionPhase { completion: usize },
    DuplicateCompletionPhase { first: usize, duplicate: usize },
}

/// Bounded phase collection used by a stored workflow.
pub type WorkflowPhases = BoundedVec<PhaseId, ConstU32<32>>;
/// Bounded edge collection used by a stored workflow.
pub type WorkflowEdges = BoundedVec<WorkflowEdge, ConstU32<128>>;
/// Bounded completion-phase collection used by a stored workflow.
pub type CompletionPhases = BoundedVec<PhaseId, ConstU32<32>>;

/// Complete immutable and bounded workflow snapshot.
#[derive(Clone, Debug, Encode, Eq, MaxEncodedLen, PartialEq, TypeInfo)]
pub struct Workflow {
    id: WorkflowId,
    phases: WorkflowPhases,
    initial_phase: PhaseId,
    edges: WorkflowEdges,
    completion_phases: CompletionPhases,
}

impl Workflow {
    /// Validates collection bounds and topology without reordering any value.
    pub fn try_new(
        id: WorkflowId,
        phases: &[PhaseId],
        initial_phase: PhaseId,
        edges: &[WorkflowEdge],
        completion_phases: &[PhaseId],
    ) -> Result<Self, WorkflowError> {
        if phases.is_empty() {
            return Err(WorkflowError::EmptyPhases);
        }
        if phases.len() > MAX_WORKFLOW_PHASES {
            return Err(WorkflowError::TooManyPhases {
                length: phases.len(),
                maximum: MAX_WORKFLOW_PHASES,
            });
        }
        if edges.len() > MAX_WORKFLOW_EDGES {
            return Err(WorkflowError::TooManyEdges {
                length: edges.len(),
                maximum: MAX_WORKFLOW_EDGES,
            });
        }
        if completion_phases.len() > MAX_COMPLETION_PHASES {
            return Err(WorkflowError::TooManyCompletionPhases {
                length: completion_phases.len(),
                maximum: MAX_COMPLETION_PHASES,
            });
        }

        for duplicate in 0..phases.len() {
            if let Some(first) = phases[..duplicate]
                .iter()
                .position(|phase| phase == &phases[duplicate])
            {
                return Err(WorkflowError::DuplicatePhase { first, duplicate });
            }
        }
        if !phases.contains(&initial_phase) {
            return Err(WorkflowError::UnknownInitialPhase);
        }
        for (index, edge) in edges.iter().enumerate() {
            if !phases.contains(edge.from()) {
                return Err(WorkflowError::UnknownEdgeSource { edge: index });
            }
            if !phases.contains(edge.to()) {
                return Err(WorkflowError::UnknownEdgeTarget { edge: index });
            }
            if let Some(first) = edges[..index]
                .iter()
                .position(|candidate| candidate == edge)
            {
                return Err(WorkflowError::DuplicateEdge {
                    first,
                    duplicate: index,
                });
            }
        }
        for (index, phase) in completion_phases.iter().enumerate() {
            if !phases.contains(phase) {
                return Err(WorkflowError::UnknownCompletionPhase { completion: index });
            }
            if let Some(first) = completion_phases[..index]
                .iter()
                .position(|candidate| candidate == phase)
            {
                return Err(WorkflowError::DuplicateCompletionPhase {
                    first,
                    duplicate: index,
                });
            }
        }

        let mut bounded_phases = WorkflowPhases::default();
        for phase in phases {
            bounded_phases
                .try_push(phase.clone())
                .map_err(|_| WorkflowError::TooManyPhases {
                    length: phases.len(),
                    maximum: MAX_WORKFLOW_PHASES,
                })?;
        }
        let mut bounded_edges = WorkflowEdges::default();
        for edge in edges {
            bounded_edges
                .try_push(edge.clone())
                .map_err(|_| WorkflowError::TooManyEdges {
                    length: edges.len(),
                    maximum: MAX_WORKFLOW_EDGES,
                })?;
        }
        let mut bounded_completion = CompletionPhases::default();
        for phase in completion_phases {
            bounded_completion.try_push(phase.clone()).map_err(|_| {
                WorkflowError::TooManyCompletionPhases {
                    length: completion_phases.len(),
                    maximum: MAX_COMPLETION_PHASES,
                }
            })?;
        }

        Ok(Self {
            id,
            phases: bounded_phases,
            initial_phase,
            edges: bounded_edges,
            completion_phases: bounded_completion,
        })
    }

    #[must_use]
    pub const fn id(&self) -> &WorkflowId {
        &self.id
    }

    #[must_use]
    pub fn phases(&self) -> &[PhaseId] {
        self.phases.as_slice()
    }

    #[must_use]
    pub const fn initial_phase(&self) -> &PhaseId {
        &self.initial_phase
    }

    #[must_use]
    pub fn edges(&self) -> &[WorkflowEdge] {
        self.edges.as_slice()
    }

    #[must_use]
    pub fn completion_phases(&self) -> &[PhaseId] {
        self.completion_phases.as_slice()
    }

    #[must_use]
    pub fn contains_phase(&self, phase: &PhaseId) -> bool {
        self.phases.contains(phase)
    }

    #[must_use]
    pub fn allows_transition(&self, from: &PhaseId, to: &PhaseId) -> bool {
        self.edges
            .iter()
            .any(|edge| edge.from() == from && edge.to() == to)
    }

    #[must_use]
    pub fn allows_completion(&self, phase: &PhaseId) -> bool {
        self.completion_phases.contains(phase)
    }

    fn from_bounded(
        id: WorkflowId,
        phases: WorkflowPhases,
        initial_phase: PhaseId,
        edges: WorkflowEdges,
        completion_phases: CompletionPhases,
    ) -> Result<Self, WorkflowError> {
        Self::try_new(
            id,
            phases.as_slice(),
            initial_phase,
            edges.as_slice(),
            completion_phases.as_slice(),
        )
    }
}

impl Decode for Workflow {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        let id = WorkflowId::decode(input)?;
        let phases = WorkflowPhases::decode(input)?;
        let initial_phase = PhaseId::decode(input)?;
        let edges = WorkflowEdges::decode(input)?;
        let completion_phases = CompletionPhases::decode(input)?;
        Self::from_bounded(id, phases, initial_phase, edges, completion_phases)
            .map_err(|_| "invalid workflow topology".into())
    }
}

impl DecodeWithMemTracking for Workflow {}

/// Whether an Intent Unit can continue through its workflow.
#[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    MaxEncodedLen,
    PartialEq,
    TypeInfo,
)]
pub enum IntentUnitStatus {
    #[codec(index = 0)]
    Active,
    #[codec(index = 1)]
    Completed,
}

/// Immutable record of one accepted phase transition.
#[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, MaxEncodedLen, PartialEq, TypeInfo,
)]
pub struct TransitionRecord {
    sequence: u64,
    from: PhaseId,
    to: PhaseId,
}

impl TransitionRecord {
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub const fn from(&self) -> &PhaseId {
        &self.from
    }

    #[must_use]
    pub const fn to(&self) -> &PhaseId {
        &self.to
    }
}

/// Immutable record of one accepted terminal completion.
#[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, MaxEncodedLen, PartialEq, TypeInfo,
)]
pub struct CompletionRecord {
    sequence: u64,
    phase: PhaseId,
}

impl CompletionRecord {
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub const fn phase(&self) -> &PhaseId {
        &self.phase
    }
}

/// One immutable entry in bounded lifecycle history.
#[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, MaxEncodedLen, PartialEq, TypeInfo,
)]
pub enum LifecycleRecord {
    #[codec(index = 0)]
    Transition(TransitionRecord),
    #[codec(index = 1)]
    Completion(CompletionRecord),
}

impl LifecycleRecord {
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        match self {
            Self::Transition(record) => record.sequence(),
            Self::Completion(record) => record.sequence(),
        }
    }
}

/// Bounded complete lifecycle history of one current-generation unit.
pub type LifecycleHistory = BoundedVec<LifecycleRecord, ConstU32<256>>;

/// Typed lifecycle rejection used by the independent conformance model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LifecycleError {
    RevisionConflict { expected: u64, actual: u64 },
    HistoryCapacityExceeded { length: usize, maximum: usize },
    RevisionExhausted,
    AlreadyCompleted,
    UnknownTarget { target: PhaseId },
    TransitionNotAllowed { from: PhaseId, to: PhaseId },
    CompletionPhaseNotEligible { phase: PhaseId },
}

/// Required-origin bounded aggregate used as pallet storage and replay state.
#[derive(Clone, Debug, Encode, Eq, MaxEncodedLen, PartialEq, TypeInfo)]
pub struct IntentUnitState {
    id: IntentUnitId,
    origin: ExternalReference,
    species: IntentSpecies,
    workflow: Workflow,
    phase: PhaseId,
    status: IntentUnitStatus,
    history: LifecycleHistory,
    revision: u64,
}

#[derive(Decode)]
struct IntentUnitStateParts {
    id: IntentUnitId,
    origin: ExternalReference,
    species: IntentSpecies,
    workflow: Workflow,
    phase: PhaseId,
    status: IntentUnitStatus,
    history: LifecycleHistory,
    revision: u64,
}

impl IntentUnitState {
    /// Creates revision zero at the workflow's declared initial phase.
    #[must_use]
    pub fn new(
        id: IntentUnitId,
        origin: ExternalReference,
        species: IntentSpecies,
        workflow: Workflow,
    ) -> Self {
        let phase = workflow.initial_phase().clone();
        Self {
            id,
            origin,
            species,
            workflow,
            phase,
            status: IntentUnitStatus::Active,
            history: LifecycleHistory::default(),
            revision: 0,
        }
    }

    #[must_use]
    pub const fn id(&self) -> IntentUnitId {
        self.id
    }

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
    pub const fn phase(&self) -> &PhaseId {
        &self.phase
    }

    #[must_use]
    pub const fn status(&self) -> IntentUnitStatus {
        self.status
    }

    #[must_use]
    pub fn history(&self) -> &[LifecycleRecord] {
        self.history.as_slice()
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Applies a revision-conditioned transition with stale/capacity precedence.
    pub fn transition_to(
        &mut self,
        target: &PhaseId,
        expected_revision: u64,
    ) -> Result<u64, LifecycleError> {
        self.require_revision_and_capacity(expected_revision)?;
        if self.status == IntentUnitStatus::Completed {
            return Err(LifecycleError::AlreadyCompleted);
        }
        if !self.workflow.contains_phase(target) {
            return Err(LifecycleError::UnknownTarget {
                target: target.clone(),
            });
        }
        if !self.workflow.allows_transition(&self.phase, target) {
            return Err(LifecycleError::TransitionNotAllowed {
                from: self.phase.clone(),
                to: target.clone(),
            });
        }

        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(LifecycleError::RevisionExhausted)?;
        let record = LifecycleRecord::Transition(TransitionRecord {
            sequence: next_revision,
            from: self.phase.clone(),
            to: target.clone(),
        });
        self.history
            .try_push(record)
            .map_err(|_| LifecycleError::HistoryCapacityExceeded {
                length: self.history.len(),
                maximum: MAX_LIFECYCLE_RECORDS,
            })?;
        self.phase = target.clone();
        self.revision = next_revision;
        Ok(next_revision)
    }

    /// Applies revision-conditioned completion with stale/capacity precedence.
    pub fn complete(&mut self, expected_revision: u64) -> Result<u64, LifecycleError> {
        self.require_revision_and_capacity(expected_revision)?;
        if self.status == IntentUnitStatus::Completed {
            return Err(LifecycleError::AlreadyCompleted);
        }
        if !self.workflow.allows_completion(&self.phase) {
            return Err(LifecycleError::CompletionPhaseNotEligible {
                phase: self.phase.clone(),
            });
        }

        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(LifecycleError::RevisionExhausted)?;
        let record = LifecycleRecord::Completion(CompletionRecord {
            sequence: next_revision,
            phase: self.phase.clone(),
        });
        self.history
            .try_push(record)
            .map_err(|_| LifecycleError::HistoryCapacityExceeded {
                length: self.history.len(),
                maximum: MAX_LIFECYCLE_RECORDS,
            })?;
        self.status = IntentUnitStatus::Completed;
        self.revision = next_revision;
        Ok(next_revision)
    }

    fn require_revision_and_capacity(&self, expected_revision: u64) -> Result<(), LifecycleError> {
        if expected_revision != self.revision {
            return Err(LifecycleError::RevisionConflict {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        if self.history.len() >= MAX_LIFECYCLE_RECORDS {
            return Err(LifecycleError::HistoryCapacityExceeded {
                length: self.history.len(),
                maximum: MAX_LIFECYCLE_RECORDS,
            });
        }
        Ok(())
    }

    fn from_parts(parts: IntentUnitStateParts) -> Result<Self, CodecError> {
        if usize::try_from(parts.revision).ok() != Some(parts.history.len()) {
            return Err("lifecycle revision and history length disagree".into());
        }
        let mut replay = Self::new(parts.id, parts.origin, parts.species, parts.workflow);
        for record in parts.history {
            let expected_sequence = replay
                .revision
                .checked_add(1)
                .ok_or_else(|| CodecError::from("lifecycle revision exhausted"))?;
            if record.sequence() != expected_sequence {
                return Err("lifecycle sequence is not consecutive".into());
            }
            match record {
                LifecycleRecord::Transition(record) => {
                    if replay.phase != record.from {
                        return Err("lifecycle transition source mismatch".into());
                    }
                    replay
                        .transition_to(&record.to, replay.revision)
                        .map_err(|_| CodecError::from("invalid lifecycle transition"))?;
                }
                LifecycleRecord::Completion(record) => {
                    if replay.phase != record.phase {
                        return Err("lifecycle completion phase mismatch".into());
                    }
                    replay
                        .complete(replay.revision)
                        .map_err(|_| CodecError::from("invalid lifecycle completion"))?;
                }
            }
        }
        if replay.phase != parts.phase
            || replay.status != parts.status
            || replay.revision != parts.revision
        {
            return Err("lifecycle terminal projections disagree with replay".into());
        }
        Ok(replay)
    }
}

impl Decode for IntentUnitState {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        Self::from_parts(IntentUnitStateParts::decode(input)?)
    }
}

impl DecodeWithMemTracking for IntentUnitState {}

/// Positive immutable version of one relationship definition.
#[derive(
    Clone, Copy, Debug, Encode, Eq, Hash, MaxEncodedLen, Ord, PartialEq, PartialOrd, TypeInfo,
)]
pub struct DefinitionVersion(u64);

impl DefinitionVersion {
    pub const fn try_new(value: u64) -> Result<Self, DefinitionVersionError> {
        if value == 0 {
            Err(DefinitionVersionError::Zero)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl Decode for DefinitionVersion {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        Self::try_new(u64::decode(input)?).map_err(|_| "definition version must be nonzero".into())
    }
}

impl DecodeWithMemTracking for DefinitionVersion {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionVersionError {
    Zero,
}

/// Complete immutable identity of one relationship definition.
#[derive(
    Clone,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    Hash,
    MaxEncodedLen,
    Ord,
    PartialEq,
    PartialOrd,
    TypeInfo,
)]
pub struct DefinitionKey {
    id: Namespace,
    version: DefinitionVersion,
}

impl DefinitionKey {
    #[must_use]
    pub const fn new(id: Namespace, version: DefinitionVersion) -> Self {
        Self { id, version }
    }

    #[must_use]
    pub const fn id(&self) -> &Namespace {
        &self.id
    }

    #[must_use]
    pub const fn version(&self) -> DefinitionVersion {
        self.version
    }
}

/// Fixed direction of relationship contract version one.
#[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    Hash,
    MaxEncodedLen,
    PartialEq,
    TypeInfo,
)]
pub enum RelationshipDirection {
    #[codec(index = 0)]
    Directed,
}

/// Independent allow/reject relationship policy.
#[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    Hash,
    MaxEncodedLen,
    PartialEq,
    TypeInfo,
)]
pub enum RelationshipPolicy {
    #[codec(index = 0)]
    Allow,
    #[codec(index = 1)]
    Reject,
}

/// Immutable definition payload shared by storage and replay-complete events.
#[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, MaxEncodedLen, PartialEq, TypeInfo,
)]
pub struct RelationshipDefinition {
    key: DefinitionKey,
    direction: RelationshipDirection,
    source_species: Option<IntentSpecies>,
    target_species: Option<IntentSpecies>,
    self_policy: RelationshipPolicy,
    cycle_policy: RelationshipPolicy,
}

impl RelationshipDefinition {
    #[must_use]
    pub const fn new(
        key: DefinitionKey,
        source_species: Option<IntentSpecies>,
        target_species: Option<IntentSpecies>,
        self_policy: RelationshipPolicy,
        cycle_policy: RelationshipPolicy,
    ) -> Self {
        Self {
            key,
            direction: RelationshipDirection::Directed,
            source_species,
            target_species,
            self_policy,
            cycle_policy,
        }
    }

    #[must_use]
    pub const fn key(&self) -> &DefinitionKey {
        &self.key
    }

    #[must_use]
    pub const fn direction(&self) -> RelationshipDirection {
        self.direction
    }

    #[must_use]
    pub const fn source_species(&self) -> Option<&IntentSpecies> {
        self.source_species.as_ref()
    }

    #[must_use]
    pub const fn target_species(&self) -> Option<&IntentSpecies> {
        self.target_species.as_ref()
    }

    #[must_use]
    pub const fn self_policy(&self) -> RelationshipPolicy {
        self.self_policy
    }

    #[must_use]
    pub const fn cycle_policy(&self) -> RelationshipPolicy {
        self.cycle_policy
    }
}

/// Complete directed identity of one relationship edge.
#[derive(
    Clone,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    Hash,
    MaxEncodedLen,
    Ord,
    PartialEq,
    PartialOrd,
    TypeInfo,
)]
pub struct RelationshipKey {
    definition: DefinitionKey,
    source_id: IntentUnitId,
    target_id: IntentUnitId,
}

impl RelationshipKey {
    #[must_use]
    pub const fn new(
        definition: DefinitionKey,
        source_id: IntentUnitId,
        target_id: IntentUnitId,
    ) -> Self {
        Self {
            definition,
            source_id,
            target_id,
        }
    }

    #[must_use]
    pub const fn definition(&self) -> &DefinitionKey {
        &self.definition
    }

    #[must_use]
    pub const fn source_id(&self) -> IntentUnitId {
        self.source_id
    }

    #[must_use]
    pub const fn target_id(&self) -> IntentUnitId {
        self.target_id
    }
}

/// Exact subject of a provenance association.
#[derive(
    Clone,
    Copy,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    Hash,
    MaxEncodedLen,
    Ord,
    PartialEq,
    PartialOrd,
    TypeInfo,
)]
pub enum AssociationSubject {
    #[codec(index = 0)]
    WholeUnit,
    #[codec(index = 1)]
    Revision(u64),
}

/// Complete immutable identity of one provenance association.
#[derive(
    Clone,
    Debug,
    Decode,
    DecodeWithMemTracking,
    Encode,
    Eq,
    Hash,
    MaxEncodedLen,
    Ord,
    PartialEq,
    PartialOrd,
    TypeInfo,
)]
pub struct AssociationKey {
    unit_id: IntentUnitId,
    subject: AssociationSubject,
    reference: ExternalReference,
}

impl AssociationKey {
    #[must_use]
    pub const fn new(
        unit_id: IntentUnitId,
        subject: AssociationSubject,
        reference: ExternalReference,
    ) -> Self {
        Self {
            unit_id,
            subject,
            reference,
        }
    }

    #[must_use]
    pub const fn unit_id(&self) -> IntentUnitId {
        self.unit_id
    }

    #[must_use]
    pub const fn subject(&self) -> AssociationSubject {
        self.subject
    }

    #[must_use]
    pub const fn reference(&self) -> &ExternalReference {
        &self.reference
    }
}

/// Required-origin create payload used to prove structural SCALE rejection.
#[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, MaxEncodedLen, PartialEq, TypeInfo,
)]
pub struct CreateUnitPayload {
    pub command_schema_version: u16,
    pub id: IntentUnitId,
    pub origin: ExternalReference,
    pub species: IntentSpecies,
    pub workflow: Workflow,
}

/// Representative finite envelope of every foundational domain payload.
#[derive(
    Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, MaxEncodedLen, PartialEq, TypeInfo,
)]
pub enum DomainPayload {
    #[codec(index = 0)]
    UnitCreated(CreateUnitPayload),
    #[codec(index = 1)]
    UnitTransitioned {
        unit_id: IntentUnitId,
        committed_revision: u64,
        from: PhaseId,
        to: PhaseId,
    },
    #[codec(index = 2)]
    UnitCompleted {
        unit_id: IntentUnitId,
        committed_revision: u64,
        phase: PhaseId,
    },
    #[codec(index = 3)]
    RelationshipDefinitionCreated(RelationshipDefinition),
    #[codec(index = 4)]
    RelationshipCreated(RelationshipKey),
    #[codec(index = 5)]
    RelationshipDeleted(RelationshipKey),
    #[codec(index = 6)]
    AssociationRecorded(AssociationKey),
    #[codec(index = 7)]
    AssociationRevoked(AssociationKey),
}
