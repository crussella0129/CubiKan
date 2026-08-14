use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use cubikan_backend::{BackendError, MigrationError, SqliteBackend};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    root: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        for _ in 0..100 {
            let ordinal = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "cubikan-t1107-backend-{}-{ordinal}",
                std::process::id()
            ));
            match fs::create_dir(&root) {
                Ok(()) => return Self { root },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("test directory should be created: {error}"),
            }
        }
        panic!("could not allocate a unique test directory")
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn directory_entries(path: &Path) -> Vec<String> {
    let mut names = fs::read_dir(path)
        .expect("fixture directory should remain readable")
        .map(|entry| {
            entry
                .expect("fixture directory entry should be readable")
                .file_name()
                .into_string()
                .expect("fixture names are UTF-8")
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn test_root_consumers_reject_v1_before_removed_authority() {
    let directory = TestDirectory::new();

    let missing = directory.path("missing-parent/database.sqlite3");
    let before_missing_entries = directory_entries(&directory.root);
    assert_eq!(
        SqliteBackend::open(&missing).expect_err("retired open must fail closed"),
        BackendError::UnsupportedSchemaVersion { found: 2 }
    );
    assert_eq!(
        SqliteBackend::migrate_v1_to_v2(&missing).expect_err("retired migration must fail closed"),
        MigrationError::Backend(BackendError::UnsupportedSchemaVersion { found: 1 })
    );
    assert!(!missing.exists());
    assert_eq!(directory_entries(&directory.root), before_missing_entries);

    let existing = directory.path("legacy.sqlite3");
    let legacy_bytes = b"retired-schema-v1-byte-snapshot\0\xff";
    fs::write(&existing, legacy_bytes).expect("fixture bytes should be written");
    let before_bytes = fs::read(&existing).expect("fixture bytes should be readable");
    let before_entries = directory_entries(&directory.root);

    assert_eq!(
        SqliteBackend::open(&existing).expect_err("retired open must reject existing bytes"),
        BackendError::UnsupportedSchemaVersion { found: 2 }
    );
    assert_eq!(
        SqliteBackend::migrate_v1_to_v2(&existing)
            .expect_err("retired migration must reject existing bytes"),
        MigrationError::Backend(BackendError::UnsupportedSchemaVersion { found: 1 })
    );
    assert_eq!(fs::read(&existing).unwrap(), before_bytes);
    assert_eq!(directory_entries(&directory.root), before_entries);

    let model_source = include_str!("../src/model.rs");
    let create_start = model_source
        .find("pub struct CreateIntentUnit")
        .expect("historical create command should remain declared");
    let create_end = model_source[create_start..]
        .find("/// Input for retrieving")
        .map(|offset| create_start + offset)
        .expect("create command should have a stable following boundary");
    let create_source = &model_source[create_start..create_end];
    assert!(!create_source.contains("origin"));

    let stored_source = include_str!("../src/stored.rs");
    let envelope_start = stored_source
        .find("struct StoredEnvelopeV1")
        .expect("historical envelope should remain declared");
    let envelope_end = stored_source[envelope_start..]
        .find("enum StoredStatusV1")
        .map(|offset| envelope_start + offset)
        .expect("envelope should have a stable following boundary");
    assert!(!stored_source[envelope_start..envelope_end].contains("origin"));

    for source in [
        include_str!("../src/sqlite.rs"),
        include_str!("../src/migration.rs"),
        stored_source,
    ] {
        assert!(!source.contains("IntentUnit::new"));
        assert!(!source.contains("synthetic_origin"));
    }
}
