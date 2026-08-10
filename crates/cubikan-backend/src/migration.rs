use std::path::Path;

use rusqlite::{Connection, OpenFlags, TransactionBehavior};

use crate::{
    BackendError, MigrationError,
    schema::{self, Ownership},
    sqlite::{
        StoredRow, classify_runtime_error, configure_accepted_database,
        configure_connection_local_safety, validate_database_path,
    },
};

pub(crate) fn migrate_v1_to_v2(path: &Path) -> Result<(), MigrationError> {
    migrate_v1_to_v2_inner(path, |_, _| Ok(()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MigrationStage {
    BeforeVersion,
    AfterVersion,
}

fn migrate_v1_to_v2_inner(
    path: &Path,
    mut stage_hook: impl FnMut(MigrationStage, &Connection) -> Result<(), MigrationError>,
) -> Result<(), MigrationError> {
    validate_database_path(path).map_err(MigrationError::from)?;
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let mut connection =
        Connection::open_with_flags(path, flags).map_err(classify_runtime_error)?;
    configure_connection_local_safety(&connection)?;

    classify_migration_source(schema::inspect(&connection)?)?;
    configure_accepted_database(&connection)?;

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(classify_runtime_error)?;
    classify_migration_source(schema::inspect(&transaction)?)?;
    replay_every_intent_unit(&transaction)?;

    schema::add_v2_objects(&transaction)?;
    stage_hook(MigrationStage::BeforeVersion, &transaction)?;
    // The version marker is deliberately the final persistent migration step.
    schema::set_user_version(&transaction, schema::SCHEMA_VERSION_V2)?;
    stage_hook(MigrationStage::AfterVersion, &transaction)?;
    if schema::inspect(&transaction)? != Ownership::OwnedV2 {
        return Err(MigrationError::Backend(BackendError::CorruptSchema));
    }

    transaction.commit().map_err(classify_runtime_error)?;
    Ok(())
}

fn classify_migration_source(ownership: Ownership) -> Result<(), MigrationError> {
    match ownership {
        Ownership::OwnedV1 => Ok(()),
        Ownership::Empty => Err(MigrationError::SourceVersionNotOne { found: 0 }),
        Ownership::OwnedV2 => Err(MigrationError::SourceVersionNotOne { found: 2 }),
    }
}

fn replay_every_intent_unit(connection: &Connection) -> Result<(), BackendError> {
    let mut statement = connection
        .prepare(
            "SELECT id, envelope_version, envelope, workflow_id, species, phase, status, revision
             FROM intent_units ORDER BY id COLLATE BINARY",
        )
        .map_err(classify_runtime_error)?;
    let rows = statement
        .query_map([], StoredRow::from_row)
        .map_err(classify_runtime_error)?;
    for row in rows {
        row.map_err(classify_runtime_error)?.into_validated_unit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        sync::{Arc, Barrier},
        thread,
    };

    use cubikan_core::{IntentSpecies, IntentUnitId, PhaseId, Workflow, WorkflowId};
    use rusqlite::TransactionBehavior;

    use super::*;
    use crate::{BackendSchemaVersion, CreateIntentUnit, SqliteBackend};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory {
        root: PathBuf,
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

    impl TestDirectory {
        fn new(label: &str) -> Self {
            for _ in 0..100 {
                let ordinal = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
                let root = std::env::temp_dir().join(format!(
                    "cubikan-migration-{label}-{}-{ordinal}",
                    std::process::id()
                ));
                match fs::create_dir(&root) {
                    Ok(()) => return Self { root },
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => panic!("test directory should be created: {error}"),
                }
            }
            panic!("could not allocate migration test directory")
        }

        fn path(&self, label: &str) -> PathBuf {
            self.root.join(format!("{label}.sqlite3"))
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn initialize_v1(path: &Path) {
        let connection = Connection::open(path).expect("v1 fixture should open");
        connection
            .execute(schema::CREATE_INTENT_UNITS_SQL, [])
            .expect("v1 table should create");
        for sql in schema::CREATE_INTENT_UNIT_INDEX_SQL {
            connection.execute(sql, []).expect("v1 index should create");
        }
        schema::set_user_version(&connection, 1).expect("v1 marker should set");
    }

    fn workflow() -> Workflow {
        let phase = PhaseId::new("queued").expect("fixture phase should be valid");
        Workflow::new(
            WorkflowId::new("delivery").expect("fixture workflow should be valid"),
            vec![phase.clone()],
            phase.clone(),
            Vec::new(),
            vec![phase],
        )
        .expect("fixture workflow should be valid")
    }

    #[test]
    fn test_explicit_migration_orders_version_last_and_preserves_all_unit_columns() {
        let directory = TestDirectory::new("preserve");
        let path = directory.path("preserve");
        initialize_v1(&path);
        let id: IntentUnitId = "00000000-0000-0000-0000-000000000001"
            .parse()
            .expect("fixture ID should parse");
        let mut old_handle = SqliteBackend::open(&path).expect("v1 should open");
        old_handle
            .create(CreateIntentUnit::new(
                Some(id),
                IntentSpecies::new("feature").expect("fixture species should be valid"),
                workflow(),
            ))
            .expect("fixture unit should create");
        let before = snapshot_rows(&Connection::open(&path).unwrap());

        let mut observed_stages = Vec::new();
        migrate_v1_to_v2_inner(&path, |stage, connection| {
            observed_stages.push((stage, schema::user_version(connection)?));
            Ok(())
        })
        .expect("migration should succeed");
        assert_eq!(
            observed_stages,
            [
                (MigrationStage::BeforeVersion, 1),
                (MigrationStage::AfterVersion, 2),
            ]
        );
        let after = snapshot_rows(&Connection::open(&path).unwrap());
        assert_eq!(after, before);
        assert_eq!(old_handle.schema_version(), BackendSchemaVersion::V1);
        assert_eq!(old_handle.get(id).unwrap().id(), id);
        assert!(old_handle.require_relationship_schema().is_err());
        let reopened = SqliteBackend::open(&path).expect("v2 should reopen");
        assert_eq!(reopened.schema_version(), BackendSchemaVersion::V2);
        reopened
            .require_relationship_schema()
            .expect("reopened v2 should pass relationship guard");

        drop(reopened);
        drop(old_handle);
    }

    #[test]
    fn test_busy_interrupted_and_racing_migrations_leave_one_exact_state() {
        let directory = TestDirectory::new("failure-atomicity");
        let interrupted = directory.path("interrupted");
        initialize_v1(&interrupted);
        let result = migrate_v1_to_v2_inner(&interrupted, |stage, connection| {
            if stage == MigrationStage::AfterVersion {
                assert_eq!(schema::user_version(connection).unwrap(), 2);
                Err(MigrationError::Backend(
                    BackendError::ConcurrentStorageChange,
                ))
            } else {
                Ok(())
            }
        });
        assert_eq!(
            result,
            Err(MigrationError::Backend(
                BackendError::ConcurrentStorageChange
            ))
        );
        assert_eq!(
            schema::inspect(&Connection::open(&interrupted).unwrap()).unwrap(),
            Ownership::OwnedV1
        );

        let busy = directory.path("busy");
        initialize_v1(&busy);
        let mut lock_connection = Connection::open(&busy).unwrap();
        lock_connection
            .busy_timeout(std::time::Duration::ZERO)
            .unwrap();
        let lock = lock_connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let result = migrate_v1_to_v2(&busy);
        assert!(matches!(
            result,
            Err(MigrationError::Backend(BackendError::StorageBusy(_)))
        ));
        drop(lock);
        assert_eq!(
            schema::inspect(&Connection::open(&busy).unwrap()).unwrap(),
            Ownership::OwnedV1
        );

        let racing = directory.path("racing");
        initialize_v1(&racing);
        let barrier = Arc::new(Barrier::new(3));
        let mut workers = Vec::new();
        for ordinal in 0..2 {
            let path = racing.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(
                thread::Builder::new()
                    .name(format!("migrator-{ordinal}"))
                    .spawn(move || {
                        barrier.wait();
                        migrate_v1_to_v2(&path)
                    })
                    .unwrap(),
            );
        }
        barrier.wait();
        let results = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| {
                    **result == Err(MigrationError::SourceVersionNotOne { found: 2 })
                })
                .count(),
            1
        );
        assert_eq!(
            schema::inspect(&Connection::open(&racing).unwrap()).unwrap(),
            Ownership::OwnedV2
        );
    }

    fn snapshot_rows(connection: &Connection) -> Vec<StoredRowSnapshot> {
        let mut statement = connection
            .prepare(
                "SELECT id,envelope_version,envelope,workflow_id,species,phase,status,revision
                 FROM intent_units ORDER BY id",
            )
            .unwrap();
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
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }
}
