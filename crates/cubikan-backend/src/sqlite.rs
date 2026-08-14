use std::path::Path;

use cubikan_core::{IntentUnit, IntentUnitId, IntentUnitStatus};
use rusqlite::{Connection, ErrorCode, Row};

use crate::{
    BackendError, BackendSchemaVersion, CompleteIntentUnit, CreateIntentUnit, CreateRelationship,
    CreateRelationshipDefinition, DeleteRelationship, IntentUnitPage, IntentUnitView,
    ListIntentUnits, ListRelationships, MigrationError, MutationResult, ProjectionPage,
    ProjectionQueryV1, RelationshipDefinitionKey, RelationshipDefinitionView, RelationshipError,
    RelationshipPage, RelationshipView, StorageFailure, TransitionIntentUnit, migration, query,
    relationship_store, schema, stored,
};

const RETIRED_SCHEMA_VERSION: i64 = 2;

/// Temporary fail-closed bridge for the retired schema-v1/v2 backend.
///
/// This type is deliberately unconstructible outside this crate. T-1108
/// replaces it with the fresh-only schema-v3 projection reader; until then no
/// root process can regain the removed SQLite write authority.
#[derive(Debug)]
pub struct SqliteBackend {
    connection: Connection,
    _private: (),
}

impl SqliteBackend {
    /// Rejects the retired backend generation before inspecting or creating a path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, BackendError> {
        reject_retired_schema()?;
        let _ = path.as_ref();
        retain_retired_implementation_symbols();
        Err(retired_schema())
    }

    /// Returns the last historical schema identity represented by this bridge.
    #[must_use]
    pub const fn schema_version(&self) -> BackendSchemaVersion {
        BackendSchemaVersion::V2
    }

    /// Rejects the retired originless migration before inspecting its path.
    pub fn migrate_v1_to_v2(path: impl AsRef<Path>) -> Result<(), MigrationError> {
        migration::migrate_v1_to_v2(path.as_ref())
    }

    pub(crate) fn require_relationship_schema(&self) -> Result<(), RelationshipError> {
        Err(RelationshipError::Backend(retired_schema()))
    }

    pub fn create_relationship_definition(
        &mut self,
        _command: CreateRelationshipDefinition,
    ) -> Result<RelationshipDefinitionView, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::create_definition(&mut self.connection, _command)
    }

    pub fn get_relationship_definition(
        &self,
        _key: RelationshipDefinitionKey,
    ) -> Result<RelationshipDefinitionView, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::get_definition(&self.connection, _key)
    }

    pub fn create_relationship(
        &mut self,
        _command: CreateRelationship,
    ) -> Result<RelationshipView, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::create_relationship(&mut self.connection, _command)
    }

    pub fn delete_relationship(
        &mut self,
        _command: DeleteRelationship,
    ) -> Result<RelationshipView, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::delete_relationship(&mut self.connection, _command)
    }

    pub fn list_relationships(
        &self,
        _query: ListRelationships,
    ) -> Result<RelationshipPage, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::list_relationships(&self.connection, _query)
    }

    pub fn project(&self, _query: ProjectionQueryV1) -> Result<ProjectionPage, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::project(&self.connection, _query)
    }

    /// Rejects the originless historical create command before ID generation or SQL.
    pub fn create(&mut self, command: CreateIntentUnit) -> Result<IntentUnitView, BackendError> {
        reject_retired_schema()?;
        let _ = command.into_parts();
        retain_retired_envelope_symbols();
        Err(retired_schema())
    }

    pub fn get(&self, id: IntentUnitId) -> Result<IntentUnitView, BackendError> {
        reject_retired_schema()?;
        let unit = load_validated_unit(&self.connection, id)?;
        Ok(IntentUnitView::from_intent_unit(&unit))
    }

    pub fn list(&self, command: ListIntentUnits) -> Result<IntentUnitPage, BackendError> {
        reject_retired_schema()?;
        query::list(&self.connection, &command)
    }

    /// Rejects the historical canonical mutation surface before SQLite access.
    pub fn transition(
        &mut self,
        _command: TransitionIntentUnit,
    ) -> Result<MutationResult, BackendError> {
        reject_retired_schema()?;
        Err(retired_schema())
    }

    /// Rejects the historical canonical mutation surface before SQLite access.
    pub fn complete(
        &mut self,
        _command: CompleteIntentUnit,
    ) -> Result<MutationResult, BackendError> {
        reject_retired_schema()?;
        Err(retired_schema())
    }
}

const fn retired_schema() -> BackendError {
    BackendError::UnsupportedSchemaVersion {
        found: RETIRED_SCHEMA_VERSION,
    }
}

fn reject_retired_schema() -> Result<(), BackendError> {
    Err(retired_schema())
}

fn retain_retired_implementation_symbols() {
    let _ = (
        schema::SCHEMA_VERSION_V1,
        schema::SCHEMA_VERSION_V2,
        schema::inspect,
        schema::user_version,
        schema::initialize_v2,
        schema::add_v2_objects,
        schema::set_user_version,
    );
    let _ = schema::Ownership::Empty.capability();
}

fn retain_retired_envelope_symbols() {
    let _ = (
        stored::ENVELOPE_VERSION,
        stored::encode_envelope,
        stored::decode_envelope,
        stored::encode_revision_text,
        stored::decode_revision_text,
        stored::encode_revision_blob,
        stored::decode_revision_blob,
    );
}

/// Retained row decoder used only to classify historical query results as
/// unsupported. It never replays an originless envelope.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StoredRow {
    id: String,
    envelope_version: i64,
    envelope: String,
    workflow_id: String,
    species: String,
    phase: String,
    status: String,
    revision: Vec<u8>,
}

impl StoredRow {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Self::from_row_at(row, 0)
    }

    pub(crate) fn from_row_at(row: &Row<'_>, offset: usize) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(offset)?,
            envelope_version: row.get(offset + 1)?,
            envelope: row.get(offset + 2)?,
            workflow_id: row.get(offset + 3)?,
            species: row.get(offset + 4)?,
            phase: row.get(offset + 5)?,
            status: row.get(offset + 6)?,
            revision: row.get(offset + 7)?,
        })
    }

    pub(crate) fn optional_from_row_at(
        row: &Row<'_>,
        offset: usize,
    ) -> rusqlite::Result<Option<Self>> {
        let Some(id) = row.get::<_, Option<String>>(offset)? else {
            return Ok(None);
        };
        Ok(Some(Self {
            id,
            envelope_version: row.get(offset + 1)?,
            envelope: row.get(offset + 2)?,
            workflow_id: row.get(offset + 3)?,
            species: row.get(offset + 4)?,
            phase: row.get(offset + 5)?,
            status: row.get(offset + 6)?,
            revision: row.get(offset + 7)?,
        }))
    }

    pub(crate) fn into_validated_unit(self) -> Result<IntentUnit, BackendError> {
        let Self {
            id,
            envelope_version,
            envelope,
            workflow_id,
            species,
            phase,
            status,
            revision,
        } = self;
        let _ = (
            id,
            envelope_version,
            envelope,
            workflow_id,
            species,
            phase,
            status,
            revision,
        );
        Err(BackendError::UnsupportedEnvelopeVersion { found: 1 })
    }
}

pub(crate) fn load_validated_unit(
    _connection: &rusqlite::Connection,
    _id: IntentUnitId,
) -> Result<IntentUnit, BackendError> {
    Err(BackendError::UnsupportedEnvelopeVersion { found: 1 })
}

pub(crate) const fn status_projection(status: IntentUnitStatus) -> &'static str {
    match status {
        IntentUnitStatus::Active => "active",
        IntentUnitStatus::Completed => "completed",
    }
}

pub(crate) fn classify_runtime_error(error: rusqlite::Error) -> BackendError {
    if is_busy_error(&error) {
        BackendError::StorageBusy(StorageFailure::new(error))
    } else {
        BackendError::storage(error)
    }
}

fn is_busy_error(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(failure.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
    )
}

pub(crate) fn is_corrupt_database_error(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(failure.code, ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase)
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_open_rejects_before_creating_a_database_or_parent() {
        let path = PathBuf::from(format!(
            "{}/cubikan-retired-open-{}-missing/parent/database.sqlite3",
            std::env::temp_dir().display(),
            std::process::id()
        ));
        assert!(!path.exists());

        assert_eq!(
            SqliteBackend::open(&path).expect_err("retired schema must reject"),
            BackendError::UnsupportedSchemaVersion { found: 2 }
        );
        assert!(!path.exists());
        assert!(!path.parent().expect("fixture has a parent").exists());
    }
}
