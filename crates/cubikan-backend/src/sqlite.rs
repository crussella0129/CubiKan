use std::{path::Path, time::Duration};

use rusqlite::{Connection, ErrorCode, OpenFlags, TransactionBehavior};

use crate::{
    BackendError, StorageFailure,
    schema::{self, Ownership},
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
    #[allow(dead_code)] // T-804 introduces the first operations over this connection.
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
