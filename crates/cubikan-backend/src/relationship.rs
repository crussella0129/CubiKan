use std::{error::Error, fmt, str::FromStr};

use cubikan_core::{
    IntentSpecies, IntentUnitId, ReferenceNamespace,
    RelationshipDefinition as CoreRelationshipDefinition,
    RelationshipDefinitionKey as CoreRelationshipDefinitionKey,
    RelationshipDefinitionVersion as CoreRelationshipDefinitionVersion,
    RelationshipIdentity as CoreRelationshipIdentity, RelationshipPolicy as CoreRelationshipPolicy,
};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::{
    BackendError, DirectRelationshipPredicate, LedgerCoordinate, ListCursor, PageLimit,
    ProjectedProjectionPage, ProjectedUnit, ProjectedUnitSummary, ProjectionCheckpoint,
    ProjectionQueryV1, ReadError, VerifiedReadSnapshot, query,
    sqlite::{classify_runtime_error, status_projection},
    stored,
};

const SELECT_PROJECTED_DEFINITION_SQL: &str = "SELECT definition.definition_id,definition.definition_version,definition.directed,definition.source_species,definition.target_species,definition.self_policy,definition.cycle_policy,anchor.parachain_genesis_hash,event.deployment_id,event.block_number,block.block_hash,event.extrinsic_index,event.extrinsic_hash,event.system_event_index,event.global_sequence FROM relationship_definitions AS definition JOIN projected_events AS event ON event.global_sequence=definition.created_global_sequence AND event.event_schema_version=1 AND event.event_kind='relationship_definition_created' COLLATE BINARY JOIN projected_blocks AS block ON block.block_number=event.block_number JOIN projection_anchor AS anchor ON anchor.singleton=block.anchor_singleton AND anchor.deployment_id=event.deployment_id WHERE definition.definition_id=?1 COLLATE BINARY AND definition.definition_version=?2";
const SELECT_DEFINITION_EXISTS_SQL: &str = "SELECT 1 FROM relationship_definitions WHERE definition_id=?1 COLLATE BINARY AND definition_version=?2";
const SELECT_PROJECTED_RELATIONSHIPS_SQL: &str = "SELECT edge.definition_id,edge.definition_version,edge.source_id,edge.target_id,anchor.parachain_genesis_hash,event.deployment_id,event.block_number,block.block_hash,event.extrinsic_index,event.extrinsic_hash,event.system_event_index,event.global_sequence,event.event_schema_version,event.event_kind FROM intent_unit_relationships AS edge LEFT JOIN projected_events AS event ON event.global_sequence=edge.created_global_sequence LEFT JOIN projected_blocks AS block ON block.block_number=event.block_number LEFT JOIN projection_anchor AS anchor ON anchor.singleton=block.anchor_singleton AND anchor.deployment_id=event.deployment_id WHERE edge.definition_id=?1 COLLATE BINARY AND edge.definition_version=?2 AND (?3 IS NULL OR edge.source_id=?3 COLLATE BINARY) AND (?4 IS NULL OR edge.target_id=?4 COLLATE BINARY) AND (?5 IS NULL OR edge.source_id>?5 COLLATE BINARY OR (edge.source_id=?5 COLLATE BINARY AND edge.target_id>?6 COLLATE BINARY)) ORDER BY edge.source_id COLLATE BINARY,edge.target_id COLLATE BINARY LIMIT ?7";
const SELECT_OUTGOING_PROJECTION_SQL: &str = "SELECT edge.definition_id,edge.definition_version,edge.source_id,edge.target_id,anchor.parachain_genesis_hash,event.deployment_id,event.block_number,block.block_hash,event.extrinsic_index,event.extrinsic_hash,event.system_event_index,event.global_sequence,event.event_schema_version,event.event_kind,unit.id FROM intent_unit_relationships AS edge JOIN intent_units AS unit ON unit.id=edge.target_id COLLATE BINARY LEFT JOIN projected_events AS event ON event.global_sequence=edge.created_global_sequence LEFT JOIN projected_blocks AS block ON block.block_number=event.block_number LEFT JOIN projection_anchor AS anchor ON anchor.singleton=block.anchor_singleton AND anchor.deployment_id=event.deployment_id WHERE edge.definition_id=?1 COLLATE BINARY AND edge.definition_version=?2 AND edge.source_id=?3 COLLATE BINARY AND (?4 IS NULL OR unit.workflow_id=?4 COLLATE BINARY) AND (?5 IS NULL OR unit.species=?5 COLLATE BINARY) AND (?6 IS NULL OR unit.phase=?6 COLLATE BINARY) AND (?7 IS NULL OR unit.status=?7 COLLATE BINARY) AND (?8 IS NULL OR unit.id>?8 COLLATE BINARY) ORDER BY unit.id COLLATE BINARY LIMIT ?9";
const SELECT_INCOMING_PROJECTION_SQL: &str = "SELECT edge.definition_id,edge.definition_version,edge.source_id,edge.target_id,anchor.parachain_genesis_hash,event.deployment_id,event.block_number,block.block_hash,event.extrinsic_index,event.extrinsic_hash,event.system_event_index,event.global_sequence,event.event_schema_version,event.event_kind,unit.id FROM intent_unit_relationships AS edge JOIN intent_units AS unit ON unit.id=edge.source_id COLLATE BINARY LEFT JOIN projected_events AS event ON event.global_sequence=edge.created_global_sequence LEFT JOIN projected_blocks AS block ON block.block_number=event.block_number LEFT JOIN projection_anchor AS anchor ON anchor.singleton=block.anchor_singleton AND anchor.deployment_id=event.deployment_id WHERE edge.definition_id=?1 COLLATE BINARY AND edge.definition_version=?2 AND edge.target_id=?3 COLLATE BINARY AND (?4 IS NULL OR unit.workflow_id=?4 COLLATE BINARY) AND (?5 IS NULL OR unit.species=?5 COLLATE BINARY) AND (?6 IS NULL OR unit.phase=?6 COLLATE BINARY) AND (?7 IS NULL OR unit.status=?7 COLLATE BINARY) AND (?8 IS NULL OR unit.id>?8 COLLATE BINARY) ORDER BY unit.id COLLATE BINARY LIMIT ?9";

/// Exact durable-schema capability cached by an open backend handle.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BackendSchemaVersion {
    V1,
    V2,
}

impl BackendSchemaVersion {
    /// Returns the SQLite `user_version` represented by this capability.
    #[must_use]
    pub const fn value(self) -> i64 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
        }
    }
}

/// Canonical adapter-owned identifier for a relationship definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelationshipDefinitionId(String);

impl RelationshipDefinitionId {
    pub const MIN_BYTES: usize = 1;
    pub const MAX_BYTES: usize = 64;

    /// Validates the canonical lowercase ASCII definition-ID grammar.
    pub fn new(value: impl Into<String>) -> Result<Self, RelationshipDefinitionIdError> {
        let value = value.into();
        let bytes = value.as_bytes();

        if bytes.is_empty() {
            return Err(RelationshipDefinitionIdError::Empty);
        }
        if bytes.len() > Self::MAX_BYTES {
            return Err(RelationshipDefinitionIdError::TooLong { bytes: bytes.len() });
        }
        if !bytes[0].is_ascii_lowercase() {
            return Err(RelationshipDefinitionIdError::InvalidStart);
        }
        if let Some((index, _)) = bytes.iter().enumerate().skip(1).find(|(_, byte)| {
            !(byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-'))
        }) {
            return Err(RelationshipDefinitionIdError::InvalidCharacter { index });
        }

        Ok(Self(value))
    }

    /// Borrows the canonical identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for RelationshipDefinitionId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for RelationshipDefinitionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for RelationshipDefinitionId {
    type Err = RelationshipDefinitionIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<String> for RelationshipDefinitionId {
    type Error = RelationshipDefinitionIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for RelationshipDefinitionId {
    type Error = RelationshipDefinitionIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Rejection from constructing a relationship-definition identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationshipDefinitionIdError {
    Empty,
    TooLong { bytes: usize },
    InvalidStart,
    InvalidCharacter { index: usize },
}

impl fmt::Display for RelationshipDefinitionIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("relationship definition ID is empty"),
            Self::TooLong { bytes } => write!(
                formatter,
                "relationship definition ID is {bytes} bytes; maximum is 64"
            ),
            Self::InvalidStart => formatter
                .write_str("relationship definition ID must start with a lowercase ASCII letter"),
            Self::InvalidCharacter { index } => write!(
                formatter,
                "relationship definition ID has an invalid byte at index {index}"
            ),
        }
    }
}

impl Error for RelationshipDefinitionIdError {}

/// Positive, full-width immutable relationship-definition version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelationshipDefinitionVersion(u64);

impl RelationshipDefinitionVersion {
    /// Validates a positive relationship-definition version.
    pub const fn new(value: u64) -> Result<Self, RelationshipDefinitionVersionError> {
        if value == 0 {
            Err(RelationshipDefinitionVersionError::Zero)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Rejection from constructing a relationship-definition version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationshipDefinitionVersionError {
    Zero,
}

impl fmt::Display for RelationshipDefinitionVersionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("relationship definition version must be positive")
    }
}

impl Error for RelationshipDefinitionVersionError {}

/// Complete immutable identity of one relationship definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelationshipDefinitionKey {
    id: RelationshipDefinitionId,
    version: RelationshipDefinitionVersion,
}

impl RelationshipDefinitionKey {
    #[must_use]
    pub const fn new(id: RelationshipDefinitionId, version: RelationshipDefinitionVersion) -> Self {
        Self { id, version }
    }

    #[must_use]
    pub const fn id(&self) -> &RelationshipDefinitionId {
        &self.id
    }

    #[must_use]
    pub const fn version(&self) -> RelationshipDefinitionVersion {
        self.version
    }
}

/// Direction selected by relationship contract version 1.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RelationshipDirection {
    Directed,
}

/// Independent allow/reject policy attached to a definition.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RelationshipPolicy {
    Allow,
    Reject,
}

/// Endpoint role used by typed validation failures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RelationshipEndpoint {
    Source,
    Target,
}

/// Input for creating one immutable relationship definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateRelationshipDefinition {
    key: RelationshipDefinitionKey,
    direction: RelationshipDirection,
    source_species: Option<IntentSpecies>,
    target_species: Option<IntentSpecies>,
    self_policy: RelationshipPolicy,
    cycle_policy: RelationshipPolicy,
}

impl CreateRelationshipDefinition {
    #[must_use]
    pub const fn new(
        key: RelationshipDefinitionKey,
        direction: RelationshipDirection,
        source_species: Option<IntentSpecies>,
        target_species: Option<IntentSpecies>,
        self_policy: RelationshipPolicy,
        cycle_policy: RelationshipPolicy,
    ) -> Self {
        Self {
            key,
            direction,
            source_species,
            target_species,
            self_policy,
            cycle_policy,
        }
    }

    #[must_use]
    pub const fn key(&self) -> &RelationshipDefinitionKey {
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

/// Validated adapter-owned view of one immutable definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipDefinitionView {
    key: RelationshipDefinitionKey,
    direction: RelationshipDirection,
    source_species: Option<IntentSpecies>,
    target_species: Option<IntentSpecies>,
    self_policy: RelationshipPolicy,
    cycle_policy: RelationshipPolicy,
}

impl RelationshipDefinitionView {
    #[must_use]
    pub const fn new(
        key: RelationshipDefinitionKey,
        direction: RelationshipDirection,
        source_species: Option<IntentSpecies>,
        target_species: Option<IntentSpecies>,
        self_policy: RelationshipPolicy,
        cycle_policy: RelationshipPolicy,
    ) -> Self {
        Self {
            key,
            direction,
            source_species,
            target_species,
            self_policy,
            cycle_policy,
        }
    }

    #[must_use]
    pub const fn key(&self) -> &RelationshipDefinitionKey {
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
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RelationshipIdentity {
    definition: RelationshipDefinitionKey,
    source: IntentUnitId,
    target: IntentUnitId,
}

impl RelationshipIdentity {
    #[must_use]
    pub const fn new(
        definition: RelationshipDefinitionKey,
        source: IntentUnitId,
        target: IntentUnitId,
    ) -> Self {
        Self {
            definition,
            source,
            target,
        }
    }

    #[must_use]
    pub const fn definition(&self) -> &RelationshipDefinitionKey {
        &self.definition
    }

    #[must_use]
    pub const fn source(&self) -> IntentUnitId {
        self.source
    }

    #[must_use]
    pub const fn target(&self) -> IntentUnitId {
        self.target
    }
}

/// Input for creating one exact directed relationship.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateRelationship {
    relationship: RelationshipIdentity,
}

impl CreateRelationship {
    #[must_use]
    pub const fn new(relationship: RelationshipIdentity) -> Self {
        Self { relationship }
    }

    #[must_use]
    pub const fn relationship(&self) -> &RelationshipIdentity {
        &self.relationship
    }
}

/// Input for deleting one exact directed relationship.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeleteRelationship {
    relationship: RelationshipIdentity,
}

impl DeleteRelationship {
    #[must_use]
    pub const fn new(relationship: RelationshipIdentity) -> Self {
        Self { relationship }
    }

    #[must_use]
    pub const fn relationship(&self) -> &RelationshipIdentity {
        &self.relationship
    }
}

/// Validated adapter-owned view of one relationship.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipView {
    relationship: RelationshipIdentity,
}

impl RelationshipView {
    #[must_use]
    pub const fn new(relationship: RelationshipIdentity) -> Self {
        Self { relationship }
    }

    #[must_use]
    pub const fn relationship(&self) -> &RelationshipIdentity {
        &self.relationship
    }
}

/// Exclusive relationship keyset cursor retaining one complete edge identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipCursor(RelationshipIdentity);

impl RelationshipCursor {
    #[must_use]
    pub const fn new(relationship: RelationshipIdentity) -> Self {
        Self(relationship)
    }

    #[must_use]
    pub const fn relationship(&self) -> &RelationshipIdentity {
        &self.0
    }
}

/// Input for one bounded exact-version direct-relationship query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListRelationships {
    definition: RelationshipDefinitionKey,
    source: Option<IntentUnitId>,
    target: Option<IntentUnitId>,
    limit: PageLimit,
    after: Option<RelationshipCursor>,
}

impl ListRelationships {
    pub fn new(
        definition: RelationshipDefinitionKey,
        source: Option<IntentUnitId>,
        target: Option<IntentUnitId>,
        limit: PageLimit,
        after: Option<RelationshipCursor>,
    ) -> Result<Self, RelationshipQueryError> {
        if let Some(cursor) = &after
            && cursor.relationship().definition() != &definition
        {
            return Err(RelationshipQueryError::CursorDefinitionMismatch {
                expected: definition,
                actual: cursor.relationship().definition().clone(),
            });
        }

        Ok(Self {
            definition,
            source,
            target,
            limit,
            after,
        })
    }

    #[must_use]
    pub const fn definition(&self) -> &RelationshipDefinitionKey {
        &self.definition
    }

    #[must_use]
    pub const fn source(&self) -> Option<IntentUnitId> {
        self.source
    }

    #[must_use]
    pub const fn target(&self) -> Option<IntentUnitId> {
        self.target
    }

    #[must_use]
    pub const fn limit(&self) -> PageLimit {
        self.limit
    }

    #[must_use]
    pub const fn after(&self) -> Option<&RelationshipCursor> {
        self.after.as_ref()
    }
}

/// One bounded page of validated direct relationships.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipPage {
    query: ListRelationships,
    items: Vec<RelationshipView>,
    next_cursor: Option<RelationshipCursor>,
}

impl RelationshipPage {
    #[must_use]
    pub const fn new(
        query: ListRelationships,
        items: Vec<RelationshipView>,
        next_cursor: Option<RelationshipCursor>,
    ) -> Self {
        Self {
            query,
            items,
            next_cursor,
        }
    }

    #[must_use]
    pub const fn query(&self) -> &ListRelationships {
        &self.query
    }

    #[must_use]
    pub fn items(&self) -> &[RelationshipView] {
        &self.items
    }

    #[must_use]
    pub const fn next_cursor(&self) -> Option<&RelationshipCursor> {
        self.next_cursor.as_ref()
    }
}

/// One immutable canonical relationship definition and its accepted event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedDefinition {
    definition: CoreRelationshipDefinition,
    created_coordinate: LedgerCoordinate,
}

impl ProjectedDefinition {
    pub(crate) const fn new(
        definition: CoreRelationshipDefinition,
        created_coordinate: LedgerCoordinate,
    ) -> Self {
        Self {
            definition,
            created_coordinate,
        }
    }

    #[must_use]
    pub const fn definition(&self) -> &CoreRelationshipDefinition {
        &self.definition
    }

    /// Relationship contract version 1 is directed by construction.
    #[must_use]
    pub const fn directed(&self) -> bool {
        true
    }

    #[must_use]
    pub const fn created_coordinate(&self) -> &LedgerCoordinate {
        &self.created_coordinate
    }
}

/// One exact projected definition paired with its attested checkpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedDefinitionResult {
    definition: ProjectedDefinition,
    checkpoint: ProjectionCheckpoint,
}

impl ProjectedDefinitionResult {
    pub(crate) const fn new(
        definition: ProjectedDefinition,
        checkpoint: ProjectionCheckpoint,
    ) -> Self {
        Self {
            definition,
            checkpoint,
        }
    }

    #[must_use]
    pub const fn definition(&self) -> &ProjectedDefinition {
        &self.definition
    }

    #[must_use]
    pub const fn checkpoint(&self) -> &ProjectionCheckpoint {
        &self.checkpoint
    }
}

/// One active direct relationship and the event that created it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedRelationship {
    key: CoreRelationshipIdentity,
    created_coordinate: LedgerCoordinate,
}

impl ProjectedRelationship {
    pub(crate) const fn new(
        key: CoreRelationshipIdentity,
        created_coordinate: LedgerCoordinate,
    ) -> Self {
        Self {
            key,
            created_coordinate,
        }
    }

    #[must_use]
    pub const fn key(&self) -> &CoreRelationshipIdentity {
        &self.key
    }

    #[must_use]
    pub const fn created_coordinate(&self) -> &LedgerCoordinate {
        &self.created_coordinate
    }
}

/// One bounded exact-version direct-relationship page at an attested checkpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedRelationshipPage {
    query: ListRelationships,
    items: Vec<ProjectedRelationship>,
    next_cursor: Option<RelationshipCursor>,
    checkpoint: ProjectionCheckpoint,
}

impl ProjectedRelationshipPage {
    pub(crate) const fn new(
        query: ListRelationships,
        items: Vec<ProjectedRelationship>,
        next_cursor: Option<RelationshipCursor>,
        checkpoint: ProjectionCheckpoint,
    ) -> Self {
        Self {
            query,
            items,
            next_cursor,
            checkpoint,
        }
    }

    #[must_use]
    pub const fn query(&self) -> &ListRelationships {
        &self.query
    }

    #[must_use]
    pub fn items(&self) -> &[ProjectedRelationship] {
        &self.items
    }

    #[must_use]
    pub const fn next_cursor(&self) -> Option<&RelationshipCursor> {
        self.next_cursor.as_ref()
    }

    #[must_use]
    pub const fn checkpoint(&self) -> &ProjectionCheckpoint {
        &self.checkpoint
    }
}

/// Rejection from constructing a direct-relationship query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationshipQueryError {
    CursorDefinitionMismatch {
        expected: RelationshipDefinitionKey,
        actual: RelationshipDefinitionKey,
    },
}

impl fmt::Display for RelationshipQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CursorDefinitionMismatch { expected, actual } => write!(
                formatter,
                "relationship cursor definition {actual:?} does not match query definition {expected:?}"
            ),
        }
    }
}

impl Error for RelationshipQueryError {}

/// Typed failure returned by relationship and projection operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationshipError {
    MigrationRequired {
        found: BackendSchemaVersion,
        required: BackendSchemaVersion,
    },
    DefinitionAlreadyExists {
        definition: RelationshipDefinitionKey,
    },
    DefinitionNotFound {
        definition: RelationshipDefinitionKey,
    },
    CorruptDefinition {
        definition: RelationshipDefinitionKey,
    },
    EndpointNotFound {
        endpoint: RelationshipEndpoint,
        id: IntentUnitId,
    },
    EndpointCorrupt {
        endpoint: RelationshipEndpoint,
        id: IntentUnitId,
        source: BackendError,
    },
    EndpointSpeciesMismatch {
        endpoint: RelationshipEndpoint,
        id: IntentUnitId,
        expected: IntentSpecies,
        actual: IntentSpecies,
    },
    SelfEdgeRejected {
        relationship: RelationshipIdentity,
    },
    CycleRejected {
        relationship: RelationshipIdentity,
    },
    DuplicateRelationship {
        relationship: RelationshipIdentity,
    },
    RelationshipNotFound {
        relationship: RelationshipIdentity,
    },
    CorruptRelationship {
        definition: RelationshipDefinitionKey,
    },
    Backend(BackendError),
}

impl From<BackendError> for RelationshipError {
    fn from(error: BackendError) -> Self {
        Self::Backend(error)
    }
}

impl fmt::Display for RelationshipError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MigrationRequired { found, required } => write!(
                formatter,
                "relationship operations require schema version {}, found {}",
                required.value(),
                found.value()
            ),
            Self::DefinitionAlreadyExists { definition } => {
                write!(
                    formatter,
                    "relationship definition {definition:?} already exists"
                )
            }
            Self::DefinitionNotFound { definition } => {
                write!(
                    formatter,
                    "relationship definition {definition:?} was not found"
                )
            }
            Self::CorruptDefinition { definition } => {
                write!(
                    formatter,
                    "relationship definition {definition:?} is corrupt"
                )
            }
            Self::EndpointNotFound { endpoint, id } => {
                write!(
                    formatter,
                    "relationship {endpoint:?} endpoint `{id}` was not found"
                )
            }
            Self::EndpointCorrupt { endpoint, id, .. } => {
                write!(
                    formatter,
                    "relationship {endpoint:?} endpoint `{id}` is corrupt"
                )
            }
            Self::EndpointSpeciesMismatch {
                endpoint,
                id,
                expected,
                actual,
            } => write!(
                formatter,
                "relationship {endpoint:?} endpoint `{id}` has species `{actual}`, expected `{expected}`"
            ),
            Self::SelfEdgeRejected { relationship } => {
                write!(formatter, "self relationship {relationship:?} is rejected")
            }
            Self::CycleRejected { relationship } => {
                write!(
                    formatter,
                    "cyclic relationship {relationship:?} is rejected"
                )
            }
            Self::DuplicateRelationship { relationship } => {
                write!(formatter, "relationship {relationship:?} already exists")
            }
            Self::RelationshipNotFound { relationship } => {
                write!(formatter, "relationship {relationship:?} was not found")
            }
            Self::CorruptRelationship { definition } => {
                write!(
                    formatter,
                    "relationship state for {definition:?} is corrupt"
                )
            }
            Self::Backend(error) => error.fmt(formatter),
        }
    }
}

impl Error for RelationshipError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EndpointCorrupt { source, .. } | Self::Backend(source) => Some(source),
            _ => None,
        }
    }
}

/// Typed failure returned by explicit schema-v1-to-v2 migration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationError {
    SourceVersionNotOne { found: i64 },
    Backend(BackendError),
}

impl From<BackendError> for MigrationError {
    fn from(error: BackendError) -> Self {
        Self::Backend(error)
    }
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceVersionNotOne { found } => {
                write!(
                    formatter,
                    "migration requires schema version 1, found {found}"
                )
            }
            Self::Backend(error) => error.fmt(formatter),
        }
    }
}

impl Error for MigrationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::SourceVersionNotOne { .. } => None,
        }
    }
}

impl VerifiedReadSnapshot {
    /// Reads one exact immutable definition from this attested snapshot.
    pub fn get_relationship_definition(
        self,
        key: RelationshipDefinitionKey,
    ) -> Result<ProjectedDefinitionResult, ReadError> {
        self.consume(move |connection, checkpoint| {
            let definition = select_projected_definition(connection, &key, checkpoint)?;
            Ok(ProjectedDefinitionResult::new(
                definition,
                checkpoint.clone(),
            ))
        })
    }

    /// Reads one bounded direct-edge page from this attested snapshot.
    pub fn list_relationships(
        self,
        query: ListRelationships,
    ) -> Result<ProjectedRelationshipPage, ReadError> {
        self.consume(move |connection, checkpoint| {
            list_projected_relationships(connection, query, checkpoint)
        })
    }
}

fn select_projected_definition(
    connection: &Connection,
    key: &RelationshipDefinitionKey,
    checkpoint: &ProjectionCheckpoint,
) -> Result<ProjectedDefinition, ReadError> {
    let canonical_key = canonical_definition_key(key)?;
    let version = stored::encode_u64_blob(key.version().value());
    let mut statement = connection
        .prepare(SELECT_PROJECTED_DEFINITION_SQL)
        .map_err(classify_runtime_error)?;
    let mut rows = statement
        .query(params![key.id().as_str(), version.as_slice()])
        .map_err(classify_runtime_error)?;
    let Some(row) = rows.next().map_err(classify_runtime_error)? else {
        drop(rows);
        drop(statement);
        if definition_exists(connection, key, &version)? {
            return Err(BackendError::ProjectionMismatch.into());
        }
        return Err(ReadError::RelationshipDefinitionNotFound {
            definition: canonical_key,
        });
    };
    let definition = decode_projected_definition(row, &canonical_key)?;
    query::validate_projected_coordinate(definition.created_coordinate(), checkpoint)?;
    if rows.next().map_err(classify_runtime_error)?.is_some() {
        return Err(BackendError::ProjectionMismatch.into());
    }
    Ok(definition)
}

fn definition_exists(
    connection: &Connection,
    key: &RelationshipDefinitionKey,
    version: &[u8; 8],
) -> Result<bool, BackendError> {
    connection
        .query_row(
            SELECT_DEFINITION_EXISTS_SQL,
            params![key.id().as_str(), version.as_slice()],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(classify_runtime_error)
}

fn decode_projected_definition(
    row: &Row<'_>,
    expected: &CoreRelationshipDefinitionKey,
) -> Result<ProjectedDefinition, BackendError> {
    let definition_id = row.get::<_, String>(0).map_err(classify_runtime_error)?;
    let definition_version = row.get::<_, Vec<u8>>(1).map_err(classify_runtime_error)?;
    let directed = row.get::<_, i64>(2).map_err(classify_runtime_error)?;
    let source_species = row
        .get::<_, Option<String>>(3)
        .map_err(classify_runtime_error)?
        .map(IntentSpecies::new)
        .transpose()
        .map_err(|_| BackendError::ProjectionMismatch)?;
    let target_species = row
        .get::<_, Option<String>>(4)
        .map_err(classify_runtime_error)?
        .map(IntentSpecies::new)
        .transpose()
        .map_err(|_| BackendError::ProjectionMismatch)?;
    let self_policy =
        decode_core_policy(&row.get::<_, String>(5).map_err(classify_runtime_error)?)?;
    let cycle_policy =
        decode_core_policy(&row.get::<_, String>(6).map_err(classify_runtime_error)?)?;
    let actual = CoreRelationshipDefinitionKey::new(
        ReferenceNamespace::new(definition_id).map_err(|_| BackendError::ProjectionMismatch)?,
        CoreRelationshipDefinitionVersion::new(
            stored::decode_u64_blob(&definition_version)
                .map_err(|_| BackendError::ProjectionMismatch)?,
        )
        .map_err(|_| BackendError::ProjectionMismatch)?,
    );
    if directed != 1 || &actual != expected {
        return Err(BackendError::ProjectionMismatch);
    }
    let coordinate = query::decode_ledger_coordinate(row, 7)?;
    Ok(ProjectedDefinition::new(
        CoreRelationshipDefinition::new(
            actual,
            source_species,
            target_species,
            self_policy,
            cycle_policy,
        ),
        coordinate,
    ))
}

fn decode_core_policy(value: &str) -> Result<CoreRelationshipPolicy, BackendError> {
    match value {
        "allow" => Ok(CoreRelationshipPolicy::Allow),
        "reject" => Ok(CoreRelationshipPolicy::Reject),
        _ => Err(BackendError::ProjectionMismatch),
    }
}

fn canonical_definition_key(
    key: &RelationshipDefinitionKey,
) -> Result<CoreRelationshipDefinitionKey, BackendError> {
    Ok(CoreRelationshipDefinitionKey::new(
        ReferenceNamespace::new(key.id().as_str()).map_err(|_| BackendError::ProjectionMismatch)?,
        CoreRelationshipDefinitionVersion::new(key.version().value())
            .map_err(|_| BackendError::ProjectionMismatch)?,
    ))
}

pub(crate) fn list_projected_relationships(
    connection: &Connection,
    query: ListRelationships,
    checkpoint: &ProjectionCheckpoint,
) -> Result<ProjectedRelationshipPage, ReadError> {
    let definition = select_projected_definition(connection, query.definition(), checkpoint)?;
    let version = stored::encode_u64_blob(query.definition().version().value());
    let source = query.source().map(|id| id.to_string());
    let target = query.target().map(|id| id.to_string());
    let cursor_source = query
        .after()
        .map(|cursor| cursor.relationship().source().to_string());
    let cursor_target = query
        .after()
        .map(|cursor| cursor.relationship().target().to_string());
    let fetch_limit = query
        .limit()
        .value()
        .checked_add(1)
        .ok_or(BackendError::ProjectionMismatch)?;
    let fetch_limit = i64::try_from(fetch_limit).map_err(|_| BackendError::ProjectionMismatch)?;

    let mut statement = connection
        .prepare(SELECT_PROJECTED_RELATIONSHIPS_SQL)
        .map_err(classify_runtime_error)?;
    let mut rows = statement
        .query(params![
            query.definition().id().as_str(),
            version.as_slice(),
            source.as_deref(),
            target.as_deref(),
            cursor_source.as_deref(),
            cursor_target.as_deref(),
            fetch_limit,
        ])
        .map_err(classify_runtime_error)?;
    let mut items = Vec::with_capacity(
        usize::try_from(fetch_limit).map_err(|_| BackendError::ProjectionMismatch)?,
    );
    let mut previous_key = query.after().map(|cursor| {
        (
            cursor.relationship().source().to_string(),
            cursor.relationship().target().to_string(),
        )
    });
    while let Some(row) = rows.next().map_err(classify_runtime_error)? {
        let projected = decode_projected_relationship(row, definition.definition().key())?;
        query::validate_projected_coordinate(projected.created_coordinate(), checkpoint)?;
        let current_key = (
            projected.key().source().to_string(),
            projected.key().target().to_string(),
        );
        if query
            .source()
            .is_some_and(|expected| expected != projected.key().source())
            || query
                .target()
                .is_some_and(|expected| expected != projected.key().target())
            || previous_key.as_ref().is_some_and(|previous| {
                previous.0.as_bytes() > current_key.0.as_bytes()
                    || (previous.0.as_bytes() == current_key.0.as_bytes()
                        && previous.1.as_bytes() >= current_key.1.as_bytes())
            })
        {
            return Err(BackendError::ProjectionMismatch.into());
        }
        validate_projected_endpoints(
            connection,
            definition.definition(),
            projected.key(),
            checkpoint,
        )?;
        previous_key = Some(current_key);
        items.push(projected);
    }

    let limit = query.limit().value();
    let has_more = items.len() > limit;
    items.truncate(limit);
    let next_cursor = if has_more {
        items.last().map(|item| {
            RelationshipCursor::new(RelationshipIdentity::new(
                query.definition().clone(),
                item.key().source(),
                item.key().target(),
            ))
        })
    } else {
        None
    };
    Ok(ProjectedRelationshipPage::new(
        query,
        items,
        next_cursor,
        checkpoint.clone(),
    ))
}

fn decode_projected_relationship(
    row: &Row<'_>,
    expected_definition: &CoreRelationshipDefinitionKey,
) -> Result<ProjectedRelationship, BackendError> {
    let definition_id = row.get::<_, String>(0).map_err(classify_runtime_error)?;
    let definition_version = row.get::<_, Vec<u8>>(1).map_err(classify_runtime_error)?;
    let source_text = row.get::<_, String>(2).map_err(classify_runtime_error)?;
    let target_text = row.get::<_, String>(3).map_err(classify_runtime_error)?;
    let definition = CoreRelationshipDefinitionKey::new(
        ReferenceNamespace::new(definition_id).map_err(|_| BackendError::ProjectionMismatch)?,
        CoreRelationshipDefinitionVersion::new(
            stored::decode_u64_blob(&definition_version)
                .map_err(|_| BackendError::ProjectionMismatch)?,
        )
        .map_err(|_| BackendError::ProjectionMismatch)?,
    );
    let source = source_text
        .parse::<IntentUnitId>()
        .map_err(|_| BackendError::ProjectionMismatch)?;
    let target = target_text
        .parse::<IntentUnitId>()
        .map_err(|_| BackendError::ProjectionMismatch)?;
    if &definition != expected_definition
        || source.to_string() != source_text
        || target.to_string() != target_text
    {
        return Err(BackendError::ProjectionMismatch);
    }
    query::validate_projected_event_binding(row, 12, "relationship_created")?;
    let coordinate = query::decode_ledger_coordinate(row, 4)?;
    Ok(ProjectedRelationship::new(
        CoreRelationshipIdentity::new(definition, source, target),
        coordinate,
    ))
}

fn validate_projected_endpoints(
    connection: &Connection,
    definition: &CoreRelationshipDefinition,
    relationship: &CoreRelationshipIdentity,
    checkpoint: &ProjectionCheckpoint,
) -> Result<(), BackendError> {
    let source = query::load_projected_unit(connection, relationship.source())
        .map_err(|_| BackendError::ProjectionMismatch)?;
    let target = query::load_projected_unit(connection, relationship.target())
        .map_err(|_| BackendError::ProjectionMismatch)?;
    query::validate_projected_coordinate(source.last_coordinate(), checkpoint)?;
    query::validate_projected_coordinate(target.last_coordinate(), checkpoint)?;
    if definition
        .source_species()
        .is_some_and(|expected| expected != source.intent_unit().species())
        || definition
            .target_species()
            .is_some_and(|expected| expected != target.intent_unit().species())
    {
        return Err(BackendError::ProjectionMismatch);
    }
    Ok(())
}

pub(crate) fn project_relationships(
    connection: &Connection,
    projection_query: ProjectionQueryV1,
    checkpoint: &ProjectionCheckpoint,
) -> Result<ProjectedProjectionPage, ReadError> {
    let predicate = projection_query
        .predicate()
        .cloned()
        .ok_or(BackendError::ProjectionMismatch)?;
    let definition = select_projected_definition(connection, predicate.definition(), checkpoint)?;
    let anchor = query::load_projected_unit(connection, predicate.anchor())?;
    query::validate_projected_coordinate(anchor.last_coordinate(), checkpoint)?;
    let (sql, anchor_species, candidate_species) = match &predicate {
        DirectRelationshipPredicate::Outgoing { .. } => (
            SELECT_OUTGOING_PROJECTION_SQL,
            definition.definition().source_species(),
            definition.definition().target_species(),
        ),
        DirectRelationshipPredicate::Incoming { .. } => (
            SELECT_INCOMING_PROJECTION_SQL,
            definition.definition().target_species(),
            definition.definition().source_species(),
        ),
    };
    validate_projected_species(&anchor, anchor_species)?;

    let filters = projection_query.filters();
    let workflow = filters.workflow_id().map(|value| value.as_str().to_owned());
    let species = filters.species().map(|value| value.as_str().to_owned());
    let phase = filters.phase().map(|value| value.as_str().to_owned());
    let status = filters
        .status()
        .map(|value| status_projection(value).to_owned());
    let after = projection_query.after().map(|cursor| cursor.to_string());
    let version = stored::encode_u64_blob(predicate.definition().version().value());
    let fetch_limit = projection_query
        .limit()
        .value()
        .checked_add(1)
        .ok_or(BackendError::ProjectionMismatch)?;
    let fetch_limit_sql =
        i64::try_from(fetch_limit).map_err(|_| BackendError::ProjectionMismatch)?;

    let mut statement = connection.prepare(sql).map_err(classify_runtime_error)?;
    let mut rows = statement
        .query(params![
            predicate.definition().id().as_str(),
            version.as_slice(),
            predicate.anchor().to_string(),
            workflow.as_deref(),
            species.as_deref(),
            phase.as_deref(),
            status.as_deref(),
            after.as_deref(),
            fetch_limit_sql,
        ])
        .map_err(classify_runtime_error)?;
    let mut items = Vec::with_capacity(fetch_limit);
    let mut previous_id = after;
    while let Some(row) = rows.next().map_err(classify_runtime_error)? {
        let relationship = decode_projected_relationship(row, definition.definition().key())?;
        query::validate_projected_coordinate(relationship.created_coordinate(), checkpoint)?;
        let candidate_text = row.get::<_, String>(14).map_err(classify_runtime_error)?;
        let candidate_id = candidate_text
            .parse::<IntentUnitId>()
            .map_err(|_| BackendError::ProjectionMismatch)?;
        if candidate_id.to_string() != candidate_text
            || !projection_edge_matches(&predicate, relationship.key(), candidate_id)
            || previous_id
                .as_ref()
                .is_some_and(|previous| previous.as_bytes() >= candidate_text.as_bytes())
        {
            return Err(BackendError::ProjectionMismatch.into());
        }

        let candidate = query::load_projected_unit(connection, candidate_id)
            .map_err(|_| BackendError::ProjectionMismatch)?;
        query::validate_projected_coordinate(candidate.last_coordinate(), checkpoint)?;
        validate_projected_species(&candidate, candidate_species)?;
        validate_projection_filters(&candidate, &projection_query)?;
        previous_id = Some(candidate_text);
        items.push(ProjectedUnitSummary::from_projected_unit(&candidate));
    }

    let limit = projection_query.limit().value();
    let has_more = items.len() > limit;
    items.truncate(limit);
    let next_cursor = if has_more {
        items.last().map(|item| ListCursor::from_id(item.id()))
    } else {
        None
    };
    Ok(ProjectedProjectionPage::new(
        projection_query,
        items,
        next_cursor,
        checkpoint.clone(),
    ))
}

fn projection_edge_matches(
    predicate: &DirectRelationshipPredicate,
    relationship: &CoreRelationshipIdentity,
    candidate: IntentUnitId,
) -> bool {
    match predicate {
        DirectRelationshipPredicate::Outgoing { anchor, .. } => {
            relationship.source() == *anchor && relationship.target() == candidate
        }
        DirectRelationshipPredicate::Incoming { anchor, .. } => {
            relationship.target() == *anchor && relationship.source() == candidate
        }
    }
}

fn validate_projected_species(
    unit: &ProjectedUnit,
    expected: Option<&IntentSpecies>,
) -> Result<(), BackendError> {
    if expected.is_some_and(|expected| expected != unit.intent_unit().species()) {
        return Err(BackendError::ProjectionMismatch);
    }
    Ok(())
}

fn validate_projection_filters(
    unit: &ProjectedUnit,
    query: &ProjectionQueryV1,
) -> Result<(), BackendError> {
    let view = unit.intent_unit();
    let filters = query.filters();
    if filters
        .workflow_id()
        .is_some_and(|expected| expected != view.workflow_id())
        || filters
            .species()
            .is_some_and(|expected| expected != view.species())
        || filters
            .phase()
            .is_some_and(|expected| expected != view.phase())
        || filters
            .status()
            .is_some_and(|expected| expected != view.status())
    {
        return Err(BackendError::ProjectionMismatch);
    }
    Ok(())
}
