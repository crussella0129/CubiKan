use std::{error::Error, fmt, str::FromStr};

use cubikan_core::{IntentSpecies, IntentUnitId};

use crate::{BackendError, PageLimit};

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
