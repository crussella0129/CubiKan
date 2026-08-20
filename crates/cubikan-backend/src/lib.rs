//! Durable-adapter values for CubiKan.
//!
//! This crate remains above [`cubikan_core`]. Its public model carries typed
//! domain values, while storage and external wire representations stay
//! adapter-owned and versioned.

#![forbid(unsafe_code)]

mod attestation;
mod error;
mod migration;
mod model;
mod projection;
mod projection_store;
mod projector;
mod provenance;
mod query;
mod relationship;
mod relationship_store;
mod schema;
mod sqlite;
mod stored;
mod verified_read;

pub use attestation::{AttestationError, attest_finalized_projection};
pub use error::{BackendError, ListCursorError, PageLimitError, StorageFailure};
pub use model::{
    CompleteIntentUnit, CreateIntentUnit, GetIntentUnit, IntentUnitPage, IntentUnitSummary,
    IntentUnitView, LedgerCoordinate, ListCursor, ListFilters, ListIntentUnits, MutationResult,
    PageLimit, ProjectedProjectionPage, ProjectedUnit, ProjectedUnitPage, ProjectedUnitResult,
    ProjectedUnitSummary, TransitionIntentUnit,
};
pub use projection::{DirectRelationshipPredicate, ProjectionPage, ProjectionQueryV1};
pub use projector::{FinalizedProjector, ProjectionError};
pub use provenance::{
    AssociationDirection, AssociationPage, AssociationQueryError, ListAssociationsByReference,
    ListAssociationsByUnit, ProjectedAssociation,
};
pub use relationship::{
    BackendSchemaVersion, CreateRelationship, CreateRelationshipDefinition, DeleteRelationship,
    ListRelationships, MigrationError, ProjectedDefinition, ProjectedDefinitionResult,
    ProjectedRelationship, ProjectedRelationshipPage, RelationshipCursor, RelationshipDefinitionId,
    RelationshipDefinitionIdError, RelationshipDefinitionKey, RelationshipDefinitionVersion,
    RelationshipDefinitionVersionError, RelationshipDefinitionView, RelationshipDirection,
    RelationshipEndpoint, RelationshipError, RelationshipIdentity, RelationshipPage,
    RelationshipPolicy, RelationshipQueryError, RelationshipView,
};
pub use sqlite::SqliteBackend;
pub use verified_read::{ProjectionCheckpoint, ReadError, VerifiedReadSnapshot};
