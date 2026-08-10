use std::{path::Path, time::Duration};

use cubikan_core::{
    IntentUnit, IntentUnitId, IntentUnitRevision, IntentUnitStatus, RevisionedCompletionError,
    RevisionedTransitionError,
};
use rusqlite::{
    Connection, ErrorCode, OpenFlags, OptionalExtension, Row, TransactionBehavior, params,
};

use crate::{
    BackendError, BackendSchemaVersion, CompleteIntentUnit, CreateIntentUnit,
    CreateRelationshipDefinition, IntentUnitPage, IntentUnitView, ListIntentUnits, MigrationError,
    MutationResult, RelationshipDefinitionKey, RelationshipDefinitionView, RelationshipError,
    StorageFailure, TransitionIntentUnit, migration, query, relationship_store,
    schema::{self, Ownership},
    stored::{
        ENVELOPE_VERSION, decode_envelope, decode_revision_blob, encode_envelope,
        encode_revision_blob,
    },
};

const BUSY_TIMEOUT: Duration = Duration::from_millis(5_000);
const BUSY_TIMEOUT_MILLISECONDS: i64 = 5_000;
const SYNCHRONOUS_EXTRA: i64 = 3;

/// Synchronous local SQLite backend for durable CubiKan Intent Units.
///
/// One handle owns one caller-selected on-disk database. Opening validates exact
/// v1 or v2 ownership before returning; it never adopts or implicitly migrates.
#[derive(Debug)]
pub struct SqliteBackend {
    connection: Connection,
    schema_version: BackendSchemaVersion,
}

impl SqliteBackend {
    /// Opens, initializes when truly empty, and validates an owned local store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, BackendError> {
        let path = path.as_ref();
        validate_database_path(path)?;

        // Deliberately omit URI and shared-cache flags. A caller-selected path
        // is a literal local filesystem path, not a SQLite URI.
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let mut connection =
            Connection::open_with_flags(path, flags).map_err(classify_runtime_error)?;

        configure_connection_local_safety(&connection)?;

        // Ownership/version classification must precede journal or synchronous
        // assignment because those PRAGMAs can change accepted database state.
        let before_lock = schema::inspect(&connection)?;
        configure_accepted_database(&connection)?;

        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(classify_runtime_error)?;
        let after_lock = schema::inspect(&transaction)?;
        let accepted = match (before_lock, after_lock) {
            (Ownership::Empty, Ownership::Empty) => {
                schema::initialize_v2(&transaction)?;
                Ownership::OwnedV2
            }
            (Ownership::Empty, owned @ (Ownership::OwnedV1 | Ownership::OwnedV2)) => owned,
            (Ownership::OwnedV1, owned @ (Ownership::OwnedV1 | Ownership::OwnedV2)) => owned,
            (Ownership::OwnedV2, Ownership::OwnedV2) => Ownership::OwnedV2,
            (Ownership::OwnedV1 | Ownership::OwnedV2, Ownership::Empty)
            | (Ownership::OwnedV2, Ownership::OwnedV1) => {
                return Err(BackendError::CorruptSchema);
            }
        };
        if schema::inspect(&transaction)? != accepted {
            return Err(BackendError::CorruptSchema);
        }
        let schema_version = accepted
            .capability()
            .expect("accepted owned schema must have a capability");
        transaction.commit().map_err(classify_runtime_error)?;

        let after_commit = schema::inspect(&connection)?;
        if after_commit != accepted
            && !(accepted == Ownership::OwnedV1 && after_commit == Ownership::OwnedV2)
        {
            return Err(BackendError::CorruptSchema);
        }
        verify_connection_configuration(&connection)?;

        Ok(Self {
            connection,
            schema_version,
        })
    }

    /// Returns the exact durable-schema capability cached when this handle opened.
    #[must_use]
    pub const fn schema_version(&self) -> BackendSchemaVersion {
        self.schema_version
    }

    /// Explicitly migrates one exact schema-v1 file to exact schema v2.
    pub fn migrate_v1_to_v2(path: impl AsRef<Path>) -> Result<(), MigrationError> {
        migration::migrate_v1_to_v2(path.as_ref())
    }

    pub(crate) fn require_relationship_schema(&self) -> Result<(), RelationshipError> {
        if self.schema_version == BackendSchemaVersion::V2 {
            Ok(())
        } else {
            Err(RelationshipError::MigrationRequired {
                found: self.schema_version,
                required: BackendSchemaVersion::V2,
            })
        }
    }

    /// Durably creates one immutable exact-version relationship definition.
    pub fn create_relationship_definition(
        &mut self,
        command: CreateRelationshipDefinition,
    ) -> Result<RelationshipDefinitionView, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::create_definition(&mut self.connection, command)
    }

    /// Retrieves and strictly decodes one exact relationship definition.
    pub fn get_relationship_definition(
        &self,
        key: RelationshipDefinitionKey,
    ) -> Result<RelationshipDefinitionView, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::get_definition(&self.connection, key)
    }

    /// Durably creates one revision-zero Intent Unit.
    pub fn create(&mut self, command: CreateIntentUnit) -> Result<IntentUnitView, BackendError> {
        let (id, species, workflow) = command.into_parts();
        let id = id.unwrap_or_else(IntentUnitId::generate);
        let unit = IntentUnit::new(id, species, workflow);
        let row = StoredRow::from_intent_unit(&unit)?;

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(classify_runtime_error)?;
        transaction
            .execute(
                "INSERT INTO intent_units (
                    id, envelope_version, envelope, workflow_id, species, phase, status, revision
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    &row.id,
                    row.envelope_version,
                    &row.envelope,
                    &row.workflow_id,
                    &row.species,
                    &row.phase,
                    &row.status,
                    &row.revision,
                ],
            )
            .map_err(|error| classify_insert_error(error, id))?;
        transaction.commit().map_err(classify_runtime_error)?;

        Ok(IntentUnitView::from_intent_unit(&unit))
    }

    /// Retrieves and replay-validates one Intent Unit by stable identity.
    pub fn get(&self, id: IntentUnitId) -> Result<IntentUnitView, BackendError> {
        let unit = load_validated_unit(&self.connection, id)?;
        Ok(IntentUnitView::from_intent_unit(&unit))
    }

    /// Lists one bounded, live keyset page of replay-validated summaries.
    pub fn list(&self, command: ListIntentUnits) -> Result<IntentUnitPage, BackendError> {
        query::list(&self.connection, &command)
    }

    /// Durably transitions one Intent Unit when its observed revision is current.
    pub fn transition(
        &mut self,
        command: TransitionIntentUnit,
    ) -> Result<MutationResult, BackendError> {
        let id = command.id();
        let expected_revision = command.expected_revision();
        let target = command.target().clone();
        self.mutate(id, expected_revision, |unit| {
            unit.transition_to_if_revision(&target, expected_revision)
                .map_err(classify_transition_error)
        })
    }

    /// Durably completes one Intent Unit when its observed revision is current.
    pub fn complete(
        &mut self,
        command: CompleteIntentUnit,
    ) -> Result<MutationResult, BackendError> {
        let id = command.id();
        let expected_revision = command.expected_revision();
        self.mutate(id, expected_revision, |unit| {
            unit.complete_if_revision(expected_revision)
                .map_err(classify_completion_error)
        })
    }

    fn mutate(
        &mut self,
        id: IntentUnitId,
        expected_revision: IntentUnitRevision,
        apply: impl FnOnce(&mut IntentUnit) -> Result<IntentUnitRevision, BackendError>,
    ) -> Result<MutationResult, BackendError> {
        // Acquiring the writer before load serializes competing writers. Busy
        // therefore takes precedence over stale evaluation, while the core's
        // guarded command retains stale-before-domain precedence once locked.
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(classify_runtime_error)?;
        let mut unit = load_validated_unit(&transaction, id)?;
        let committed_revision = apply(&mut unit)?;
        let successor = StoredRow::from_intent_unit(&unit)?;

        let changed = transaction
            .execute(
                "UPDATE intent_units SET
                    envelope_version=?1,
                    envelope=?2,
                    workflow_id=?3,
                    species=?4,
                    phase=?5,
                    status=?6,
                    revision=?7
                 WHERE id=?8 AND revision=?9 AND envelope_version=1",
                params![
                    successor.envelope_version,
                    &successor.envelope,
                    &successor.workflow_id,
                    &successor.species,
                    &successor.phase,
                    &successor.status,
                    &successor.revision,
                    &successor.id,
                    encode_revision_blob(expected_revision),
                ],
            )
            .map_err(classify_runtime_error)?;
        if changed != 1 {
            return Err(BackendError::ConcurrentStorageChange);
        }

        transaction.commit().map_err(classify_runtime_error)?;
        let view = IntentUnitView::from_intent_unit(&unit);
        Ok(MutationResult::new(committed_revision, view))
    }
}

fn load_validated_unit(
    connection: &Connection,
    id: IntentUnitId,
) -> Result<IntentUnit, BackendError> {
    let row = connection
        .query_row(
            "SELECT id, envelope_version, envelope, workflow_id, species, phase, status, revision
             FROM intent_units WHERE id=?1",
            [id.to_string()],
            StoredRow::from_row,
        )
        .optional()
        .map_err(classify_runtime_error)?
        .ok_or(BackendError::IntentUnitNotFound { id })?;
    row.into_validated_unit()
}

fn classify_transition_error(error: RevisionedTransitionError) -> BackendError {
    match error {
        RevisionedTransitionError::Conflict(conflict) => BackendError::RevisionConflict(conflict),
        RevisionedTransitionError::Transition(error) => BackendError::TransitionRejected(error),
    }
}

fn classify_completion_error(error: RevisionedCompletionError) -> BackendError {
    match error {
        RevisionedCompletionError::Conflict(conflict) => BackendError::RevisionConflict(conflict),
        RevisionedCompletionError::Completion(error) => BackendError::CompletionRejected(error),
    }
}

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
    fn from_intent_unit(unit: &IntentUnit) -> Result<Self, BackendError> {
        Ok(Self {
            id: unit.id().to_string(),
            envelope_version: i64::try_from(ENVELOPE_VERSION)
                .expect("envelope version 1 must fit SQLite INTEGER"),
            envelope: encode_envelope(unit)?,
            workflow_id: unit.workflow_id().as_str().to_owned(),
            species: unit.species().as_str().to_owned(),
            phase: unit.phase().as_str().to_owned(),
            status: status_projection(unit.status()).to_owned(),
            revision: encode_revision_blob(unit.revision()).to_vec(),
        })
    }

    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            envelope_version: row.get(1)?,
            envelope: row.get(2)?,
            workflow_id: row.get(3)?,
            species: row.get(4)?,
            phase: row.get(5)?,
            status: row.get(6)?,
            revision: row.get(7)?,
        })
    }

    pub(crate) fn into_validated_unit(self) -> Result<IntentUnit, BackendError> {
        let projected_version =
            u64::try_from(self.envelope_version).map_err(|_| BackendError::ProjectionMismatch)?;
        if projected_version != ENVELOPE_VERSION {
            return Err(BackendError::UnsupportedEnvelopeVersion {
                found: projected_version,
            });
        }

        let unit = decode_envelope(self.envelope.as_bytes())?;
        let projected_revision = decode_revision_blob(&self.revision)?;
        if self.id != unit.id().to_string()
            || self.workflow_id != unit.workflow_id().as_str()
            || self.species != unit.species().as_str()
            || self.phase != unit.phase().as_str()
            || self.status != status_projection(unit.status())
            || projected_revision != unit.revision()
        {
            return Err(BackendError::ProjectionMismatch);
        }
        Ok(unit)
    }
}

pub(crate) const fn status_projection(status: IntentUnitStatus) -> &'static str {
    match status {
        IntentUnitStatus::Active => "active",
        IntentUnitStatus::Completed => "completed",
    }
}

fn classify_insert_error(error: rusqlite::Error, id: IntentUnitId) -> BackendError {
    let is_duplicate = matches!(
        &error,
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(
                failure.extended_code,
                rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
                    | rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
            )
    );
    if is_duplicate {
        BackendError::DuplicateIntentUnit { id }
    } else {
        classify_runtime_error(error)
    }
}

pub(crate) fn validate_database_path(path: &Path) -> Result<(), BackendError> {
    if path.as_os_str().is_empty() || path == Path::new(":memory:") {
        Err(BackendError::storage(rusqlite::Error::InvalidPath(
            path.to_path_buf(),
        )))
    } else {
        Ok(())
    }
}

pub(crate) fn configure_connection_local_safety(
    connection: &Connection,
) -> Result<(), BackendError> {
    connection
        .busy_timeout(BUSY_TIMEOUT)
        .map_err(classify_runtime_error)?;
    connection
        .pragma_update(None, "foreign_keys", 1_i64)
        .map_err(classify_runtime_error)?;
    connection
        .pragma_update(None, "trusted_schema", 0_i64)
        .map_err(classify_runtime_error)?;
    connection
        .pragma_update(None, "read_uncommitted", 0_i64)
        .map_err(classify_runtime_error)?;
    connection
        .pragma_update(None, "locking_mode", "NORMAL")
        .map_err(classify_runtime_error)?;
    Ok(())
}

pub(crate) fn configure_accepted_database(connection: &Connection) -> Result<(), BackendError> {
    let journal_mode: String = connection
        .query_row("PRAGMA main.journal_mode = DELETE", [], |row| row.get(0))
        .map_err(classify_runtime_error)?;
    if !journal_mode.eq_ignore_ascii_case("delete") {
        return Err(BackendError::CorruptSchema);
    }
    connection
        .pragma_update(None, "synchronous", "EXTRA")
        .map_err(classify_runtime_error)?;
    Ok(())
}

pub(crate) fn verify_connection_configuration(connection: &Connection) -> Result<(), BackendError> {
    let journal_mode: String = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .map_err(classify_runtime_error)?;
    let locking_mode: String = connection
        .pragma_query_value(None, "locking_mode", |row| row.get(0))
        .map_err(classify_runtime_error)?;
    let synchronous = pragma_i64(connection, "synchronous")?;
    let foreign_keys = pragma_i64(connection, "foreign_keys")?;
    let trusted_schema = pragma_i64(connection, "trusted_schema")?;
    let read_uncommitted = pragma_i64(connection, "read_uncommitted")?;
    let busy_timeout = pragma_i64(connection, "busy_timeout")?;

    if !journal_mode.eq_ignore_ascii_case("delete")
        || !locking_mode.eq_ignore_ascii_case("normal")
        || synchronous != SYNCHRONOUS_EXTRA
        || foreign_keys != 1
        || trusted_schema != 0
        || read_uncommitted != 0
        || busy_timeout != BUSY_TIMEOUT_MILLISECONDS
    {
        return Err(BackendError::CorruptSchema);
    }
    Ok(())
}

fn pragma_i64(connection: &Connection, pragma: &str) -> Result<i64, BackendError> {
    connection
        .pragma_query_value(None, pragma, |row| row.get(0))
        .map_err(classify_runtime_error)
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
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use cubikan_core::{IntentSpecies, PhaseId, Workflow, WorkflowEdge, WorkflowId};

    use super::*;

    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

    fn test_path() -> std::path::PathBuf {
        let ordinal = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "cubikan-backend-config-{}-{ordinal}.sqlite3",
            std::process::id()
        ))
    }

    struct TestDatabase {
        root: PathBuf,
        path: PathBuf,
    }

    impl TestDatabase {
        fn new(label: &str) -> Self {
            let ordinal = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "cubikan-backend-unit-{label}-{}-{ordinal}",
                std::process::id()
            ));
            fs::create_dir(&root).expect("test directory should be created");
            let path = root.join("cubikan.sqlite3");
            Self { root, path }
        }
    }

    impl Drop for TestDatabase {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn test_exact_v1_retains_unit_operations_and_caches_relationship_migration_guard() {
        let database = TestDatabase::new("v1-capability");
        let connection = Connection::open(&database.path).expect("v1 fixture should open");
        connection
            .execute(schema::CREATE_INTENT_UNITS_SQL, [])
            .expect("v1 table should create");
        for sql in schema::CREATE_INTENT_UNIT_INDEX_SQL {
            connection.execute(sql, []).expect("v1 index should create");
        }
        schema::set_user_version(&connection, 1).expect("v1 marker should set");
        drop(connection);

        let queued = PhaseId::new("queued").unwrap();
        let done = PhaseId::new("done").unwrap();
        let workflow = Workflow::new(
            WorkflowId::new("delivery").unwrap(),
            vec![done.clone(), queued.clone()],
            queued,
            vec![WorkflowEdge::new(
                PhaseId::new("queued").unwrap(),
                done.clone(),
            )],
            vec![done.clone()],
        )
        .unwrap();
        let id: IntentUnitId = "00000000-0000-0000-0000-000000000001".parse().unwrap();
        let mut backend = SqliteBackend::open(&database.path).expect("exact v1 should open");
        assert_eq!(backend.schema_version(), BackendSchemaVersion::V1);
        backend
            .create(CreateIntentUnit::new(
                Some(id),
                IntentSpecies::new("feature").unwrap(),
                workflow,
            ))
            .unwrap();
        assert_eq!(backend.get(id).unwrap().id(), id);
        assert_eq!(
            backend
                .list(ListIntentUnits::new(
                    crate::ListFilters::default(),
                    crate::PageLimit::new(100).unwrap(),
                    None,
                ))
                .unwrap()
                .items()
                .len(),
            1
        );
        let transitioned = backend
            .transition(TransitionIntentUnit::new(
                id,
                done,
                IntentUnitRevision::INITIAL,
            ))
            .unwrap();
        backend
            .complete(CompleteIntentUnit::new(
                id,
                transitioned.committed_revision(),
            ))
            .unwrap();
        let before = backend
            .connection
            .query_row(
                "SELECT id,envelope_version,envelope,workflow_id,species,phase,status,revision
                 FROM intent_units WHERE id=?1",
                [id.to_string()],
                StoredRow::from_row,
            )
            .unwrap();
        assert_eq!(
            schema::inspect(&backend.connection).unwrap(),
            Ownership::OwnedV1
        );
        assert_eq!(
            backend.require_relationship_schema(),
            Err(RelationshipError::MigrationRequired {
                found: BackendSchemaVersion::V1,
                required: BackendSchemaVersion::V2,
            })
        );
        let after = backend
            .connection
            .query_row(
                "SELECT id,envelope_version,envelope,workflow_id,species,phase,status,revision
                 FROM intent_units WHERE id=?1",
                [id.to_string()],
                StoredRow::from_row,
            )
            .unwrap();
        assert_eq!(after, before);
        assert_eq!(
            schema::inspect(&backend.connection).unwrap(),
            Ownership::OwnedV1
        );
        verify_connection_configuration(&backend.connection).unwrap();
    }

    #[test]
    fn test_new_empty_database_initializes_exact_schema_v2_and_pragmas() {
        let path = test_path();
        let backend = SqliteBackend::open(&path).expect("new database should open");
        assert_eq!(backend.schema_version(), BackendSchemaVersion::V2);

        verify_connection_configuration(&backend.connection)
            .expect("returned connection should retain exact configuration");
        let journal_mode: String = backend
            .connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("journal mode should be readable");
        let locking_mode: String = backend
            .connection
            .pragma_query_value(None, "locking_mode", |row| row.get(0))
            .expect("locking mode should be readable");
        assert_eq!(journal_mode.to_ascii_lowercase(), "delete");
        assert_eq!(locking_mode.to_ascii_lowercase(), "normal");
        assert_eq!(pragma_i64(&backend.connection, "synchronous").unwrap(), 3);
        assert_eq!(pragma_i64(&backend.connection, "foreign_keys").unwrap(), 1);
        assert_eq!(
            pragma_i64(&backend.connection, "trusted_schema").unwrap(),
            0
        );
        assert_eq!(
            pragma_i64(&backend.connection, "read_uncommitted").unwrap(),
            0
        );
        assert_eq!(
            pragma_i64(&backend.connection, "busy_timeout").unwrap(),
            5_000
        );

        drop(backend);
        let reopened = SqliteBackend::open(&path).expect("exact v2 database should reopen");
        assert_eq!(reopened.schema_version(), BackendSchemaVersion::V2);
        verify_connection_configuration(&reopened.connection)
            .expect("reopened connection should retain exact configuration");
        drop(reopened);
        let _ = fs::remove_file(path);
    }
}
