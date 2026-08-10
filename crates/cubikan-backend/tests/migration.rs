mod common;

use std::fs;

use common::{TestDatabase, initialize_exact_v1};
use cubikan_backend::{BackendError, MigrationError, SqliteBackend};
use rusqlite::Connection;

#[derive(Debug, Eq, PartialEq)]
struct LogicalSnapshot {
    version: i64,
    objects: Vec<(String, String, String, Option<String>)>,
    rows: Vec<StoredRowSnapshot>,
}

#[derive(Debug, Eq, PartialEq)]
struct StoredRowSnapshot {
    id: String,
    envelope_version: i64,
    envelope: String,
    workflow_id: String,
    species: String,
    phase: String,
    status: String,
    revision: Vec<u8>,
}

fn snapshot(connection: &Connection) -> LogicalSnapshot {
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("schema version should be readable");
    let objects = connection
        .prepare("SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY name")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .expect("schema snapshot should be readable");
    let has_units = objects
        .iter()
        .any(|row| row.0 == "table" && row.1 == "intent_units");
    let rows = if has_units {
        connection
            .prepare(
                "SELECT id,envelope_version,envelope,workflow_id,species,phase,status,revision
                 FROM intent_units ORDER BY id",
            )
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| {
                        Ok(StoredRowSnapshot {
                            id: row.get(0)?,
                            envelope_version: row.get(1)?,
                            envelope: row.get(2)?,
                            workflow_id: row.get(3)?,
                            species: row.get(4)?,
                            phase: row.get(5)?,
                            status: row.get(6)?,
                            revision: row.get(7)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()
            })
            .expect("unit snapshot should be readable")
    } else {
        Vec::new()
    };
    LogicalSnapshot {
        version,
        objects,
        rows,
    }
}

fn assert_rejected_without_logical_change(database: &TestDatabase, expected: MigrationError) {
    let before = snapshot(&database.connect());
    assert_eq!(
        SqliteBackend::migrate_v1_to_v2(database.path()),
        Err(expected)
    );
    assert_eq!(snapshot(&database.connect()), before);
}

#[test]
fn test_migration_rejects_unowned_corrupt_and_wrong_version_sources() {
    let missing = TestDatabase::new("migration-missing");
    assert!(!missing.path().exists());
    assert!(matches!(
        SqliteBackend::migrate_v1_to_v2(missing.path()),
        Err(MigrationError::Backend(BackendError::Storage(_)))
    ));
    assert!(
        !missing.path().exists(),
        "migration must not create its source"
    );

    let empty = TestDatabase::new("migration-empty");
    fs::File::create(empty.path()).expect("empty source should be created");
    assert_rejected_without_logical_change(
        &empty,
        MigrationError::SourceVersionNotOne { found: 0 },
    );

    let unowned = TestDatabase::new("migration-unowned");
    unowned
        .connect()
        .execute_batch(
            "CREATE TABLE foreign_data(value TEXT); INSERT INTO foreign_data VALUES ('keep');",
        )
        .expect("unowned fixture should create");
    assert_rejected_without_logical_change(
        &unowned,
        MigrationError::Backend(BackendError::UnownedDatabase),
    );

    let corrupt_unit = TestDatabase::new("migration-corrupt-unit");
    {
        let connection = corrupt_unit.connect();
        initialize_exact_v1(&connection);
        connection
            .execute(
                "INSERT INTO intent_units VALUES (?1,1,?2,'workflow','feature','queued','active',?3)",
                (
                    "00000000-0000-0000-0000-000000000001",
                    "not-json",
                    [0_u8; 8],
                ),
            )
            .expect("corrupt replay fixture should insert");
    }
    assert_rejected_without_logical_change(
        &corrupt_unit,
        MigrationError::Backend(BackendError::CorruptEnvelope),
    );

    let malformed_v1 = TestDatabase::new("migration-malformed-v1");
    {
        let connection = malformed_v1.connect();
        initialize_exact_v1(&connection);
        connection
            .execute("CREATE TABLE unexpected(value TEXT) STRICT", [])
            .expect("extra object should create");
    }
    assert_rejected_without_logical_change(
        &malformed_v1,
        MigrationError::Backend(BackendError::CorruptSchema),
    );

    let v2 = TestDatabase::new("migration-v2");
    drop(SqliteBackend::open(v2.path()).expect("v2 fixture should initialize"));
    assert_rejected_without_logical_change(&v2, MigrationError::SourceVersionNotOne { found: 2 });

    let unsupported = TestDatabase::new("migration-v3");
    {
        let connection = unsupported.connect();
        initialize_exact_v1(&connection);
        connection
            .pragma_update(None, "user_version", 3_i64)
            .expect("unsupported version should set");
    }
    assert_rejected_without_logical_change(
        &unsupported,
        MigrationError::Backend(BackendError::UnsupportedSchemaVersion { found: 3 }),
    );

    let non_sqlite = TestDatabase::new("migration-not-sqlite");
    let bytes = b"not a SQLite database";
    fs::write(non_sqlite.path(), bytes).expect("non-SQLite fixture should be written");
    assert_eq!(
        SqliteBackend::migrate_v1_to_v2(non_sqlite.path()),
        Err(MigrationError::Backend(BackendError::CorruptSchema))
    );
    assert_eq!(fs::read(non_sqlite.path()).unwrap(), bytes);
}
