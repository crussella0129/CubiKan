//! Durable-adapter values for CubiKan.
//!
//! This crate remains above [`cubikan_core`]. Its public model carries typed
//! domain values, while storage and external wire representations stay
//! adapter-owned and versioned.

#![forbid(unsafe_code)]

mod error;
mod migration;
mod model;
mod projection;
mod query;
mod relationship;
mod schema;
mod sqlite;
mod stored;

pub use error::{BackendError, ListCursorError, PageLimitError, StorageFailure};
pub use model::{
    CompleteIntentUnit, CreateIntentUnit, GetIntentUnit, IntentUnitPage, IntentUnitSummary,
    IntentUnitView, ListCursor, ListFilters, ListIntentUnits, MutationResult, PageLimit,
    TransitionIntentUnit,
};
pub use projection::{DirectRelationshipPredicate, ProjectionPage, ProjectionQueryV1};
pub use relationship::{
    BackendSchemaVersion, CreateRelationship, CreateRelationshipDefinition, DeleteRelationship,
    ListRelationships, MigrationError, RelationshipCursor, RelationshipDefinitionId,
    RelationshipDefinitionIdError, RelationshipDefinitionKey, RelationshipDefinitionVersion,
    RelationshipDefinitionVersionError, RelationshipDefinitionView, RelationshipDirection,
    RelationshipEndpoint, RelationshipError, RelationshipIdentity, RelationshipPage,
    RelationshipPolicy, RelationshipQueryError, RelationshipView,
};
pub use sqlite::SqliteBackend;
