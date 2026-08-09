use std::{path::Path, time::Duration};

use cubikan_core::{IntentUnit, IntentUnitId, IntentUnitStatus};
use rusqlite::{
    Connection, ErrorCode, OpenFlags, OptionalExtension, Row, TransactionBehavior, params,
};

use crate::{
    BackendError, CreateIntentUnit, IntentUnitPage, IntentUnitView, ListIntentUnits,
    StorageFailure, query,
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
/// Version 1 deliberately owns one caller-selected on-disk database. Opening a
/// path validates ownership and the exact schema before returning; it never
/// adopts or migrates an unknown database.
#[derive(Debug)]
pub struct SqliteBackend {
    connection: Connection,
}

impl SqliteBackend {
    /// Opens, initializes when truly empty, and validates an owned local store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, BackendError> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() || path == Path::new(":memory:") {
            return Err(BackendError::storage(rusqlite::Error::InvalidPath(
                path.to_path_buf(),
            )));
        }

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
        match (before_lock, after_lock) {
            (Ownership::Empty, Ownership::Empty) => schema::initialize(&transaction)?,
            (Ownership::Empty | Ownership::OwnedV1, Ownership::OwnedV1) => {}
            (Ownership::OwnedV1, Ownership::Empty) => {
                return Err(BackendError::CorruptSchema);
            }
        }
        if schema::inspect(&transaction)? != Ownership::OwnedV1 {
            return Err(BackendError::CorruptSchema);
        }
        transaction.commit().map_err(classify_runtime_error)?;

        if schema::inspect(&connection)? != Ownership::OwnedV1 {
            return Err(BackendError::CorruptSchema);
        }
        verify_connection_configuration(&connection)?;

        Ok(Self { connection })
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
        let row = self
            .connection
            .query_row(
                "SELECT id, envelope_version, envelope, workflow_id, species, phase, status, revision
                 FROM intent_units WHERE id=?1",
                [id.to_string()],
                StoredRow::from_row,
            )
            .optional()
            .map_err(classify_runtime_error)?
            .ok_or(BackendError::IntentUnitNotFound { id })?;
        let unit = row.into_validated_unit()?;
        Ok(IntentUnitView::from_intent_unit(&unit))
    }

    /// Lists one bounded, live keyset page of replay-validated summaries.
    pub fn list(&self, command: ListIntentUnits) -> Result<IntentUnitPage, BackendError> {
        query::list(&self.connection, &command)
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

fn configure_connection_local_safety(connection: &Connection) -> Result<(), BackendError> {
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

fn configure_accepted_database(connection: &Connection) -> Result<(), BackendError> {
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

fn verify_connection_configuration(connection: &Connection) -> Result<(), BackendError> {
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
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

    fn test_path() -> std::path::PathBuf {
        let ordinal = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "cubikan-backend-config-{}-{ordinal}.sqlite3",
            std::process::id()
        ))
    }

    #[test]
    fn test_new_empty_database_initializes_exact_schema_v1_and_pragmas() {
        let path = test_path();
        let backend = SqliteBackend::open(&path).expect("new database should open");

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
        let _ = fs::remove_file(path);
    }
}
