use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use cubikan_backend::{BackendError, SqliteBackend};
use serde_json::Value;
use sha2::{Digest, Sha256};

const FILESYSTEM_FIXTURE: &[u8] =
    include_bytes!("../../../tests/fixtures/filesystem-boundary-v1.json");
const AUTHORIZER_FIXTURE: &[u8] =
    include_bytes!("../../../tests/fixtures/sqlite-authorizer-v1.json");
const SQLITE_SOURCE: &str = include_str!("../src/sqlite.rs");
const SCHEMA_SOURCE: &str = include_str!("../src/schema.rs");

static TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

fn fixture(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("locked fixture must be valid JSON")
}

fn array_length(value: &Value, key: &str) -> usize {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} must be an array"))
        .len()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn unique_temporary_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cubikan-t1108-{label}-{}-{}",
        std::process::id(),
        TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn test_public_fresh_projection_contract_is_closed_and_fixture_pinned() {
    let filesystem = fixture(FILESYSTEM_FIXTURE);
    assert_eq!(
        filesystem["fixture_format"],
        "cubikan-filesystem-boundary-v1"
    );
    assert_eq!(array_length(&filesystem, "mount_classifier_cases"), 34);
    assert_eq!(array_length(&filesystem, "path_cases"), 12);
    assert_eq!(array_length(&filesystem, "inode_cases"), 17);
    assert_eq!(array_length(&filesystem, "sidecar_cases"), 7);
    assert_eq!(
        filesystem["classifier_contract"]["allowed_filesystems"]
            .as_array()
            .expect("filesystem list")
            .iter()
            .map(|entry| entry["filesystem_type"].as_str().expect("filesystem name"))
            .collect::<Vec<_>>(),
        ["ext2", "ext3", "ext4", "xfs", "btrfs"]
    );

    assert!(SQLITE_SOURCE.contains("OFlags::RDWR | OFlags::CREATE | OFlags::EXCL"));
    assert!(SQLITE_SOURCE.contains("OFlags::NOFOLLOW | OFlags::CLOEXEC"));
    assert!(SQLITE_SOURCE.contains("Mode::from_raw_mode(0o600)"));
    assert!(
        SQLITE_SOURCE
            .contains("OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW")
    );
    assert!(SQLITE_SOURCE.contains("schema::initialize_v3_scoped"));
    assert_eq!(SCHEMA_SOURCE.matches("CREATE TABLE ").count(), 8);
    assert_eq!(SCHEMA_SOURCE.matches("CREATE INDEX ").count(), 8);
    assert_eq!(SCHEMA_SOURCE.matches("CREATE UNIQUE INDEX ").count(), 3);
    assert!(SCHEMA_SOURCE.contains("pub(crate) const SCHEMA_VERSION: i64 = 3"));

    let authorizer = fixture(AUTHORIZER_FIXTURE);
    let creator = &authorizer["roles"]["creator"];
    assert_eq!(array_length(creator, "statement_inventory"), 54);
    assert_eq!(array_length(creator, "allowed_tuples"), 196);

    let missing = unique_temporary_path("fresh-reject");
    assert!(!missing.exists());
    assert_eq!(
        SqliteBackend::open(&missing).expect_err("retired authority must reject"),
        BackendError::UnsupportedSchemaVersion { found: 2 }
    );
    assert!(!missing.exists());
}

#[test]
fn test_public_retired_open_is_byte_preserving_and_preflight_contract_is_pinned() {
    let path = unique_temporary_path("existing-byte-equality");
    let original = b"SQLite format 3\0fixed-adversarial-bytes";
    fs::write(&path, original).expect("write test-owned fixture");

    assert_eq!(
        SqliteBackend::open(&path).expect_err("retired authority must reject"),
        BackendError::UnsupportedSchemaVersion { found: 2 }
    );
    assert_eq!(fs::read(&path).expect("read unchanged fixture"), original);
    for suffix in ["-journal", "-wal", "-shm"] {
        assert!(!PathBuf::from(format!("{}{suffix}", path.display())).exists());
    }
    fs::remove_file(&path).expect("remove test-owned fixture");

    let open_writer = SQLITE_SOURCE
        .find("pub(crate) fn open_projection_writer")
        .expect("writer open boundary");
    let path_validation = SQLITE_SOURCE[open_writer..]
        .find("SecurePath::for_existing")
        .expect("path validation");
    let runtime_validation = SQLITE_SOURCE[open_writer..]
        .find("validate_runtime_boundary")
        .expect("runtime validation");
    let sqlite_preflight = SQLITE_SOURCE[open_writer..]
        .find("open_preflight_at")
        .expect("SQLite preflight");
    assert!(path_validation < runtime_validation && runtime_validation < sqlite_preflight);
    assert!(SQLITE_SOURCE.contains("path.validate_existing_file(None, true)?"));
    assert!(
        SQLITE_SOURCE
            .contains("OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW")
    );
    assert!(SQLITE_SOURCE.contains("schema::validate_v3_scoped"));

    let authorizer = fixture(AUTHORIZER_FIXTURE);
    let roles = &authorizer["roles"];
    for (role, inventory, tuples) in [
        ("preflight_reader", 86, 137),
        ("projector_writer", 99, 227),
        ("public_reader", 27, 205),
    ] {
        assert_eq!(array_length(&roles[role], "statement_inventory"), inventory);
        assert_eq!(array_length(&roles[role], "allowed_tuples"), tuples);
    }
}

#[test]
fn test_public_path_feature_and_uri_contract_is_pinned() {
    assert_eq!(
        sha256(FILESYSTEM_FIXTURE),
        "959f21e68c2b3010d4a477b102810ef2b908a97d0aea97bd3da751ad45b168e5"
    );
    assert_eq!(
        sha256(AUTHORIZER_FIXTURE),
        "cc7014c65b0384a39672cda01d45160ebf45af3f148dcbe34330142310423441"
    );

    let authorizer = fixture(AUTHORIZER_FIXTURE);
    let compile = &authorizer["sqlite_contract"]["compile_options_by_target"][0];
    let options = compile["options"]
        .as_array()
        .expect("compile option vector");
    assert_eq!(options.len(), 51);
    let canonical = options
        .iter()
        .map(|option| option.as_str().expect("compile option"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(canonical.len(), 1_053);
    assert_eq!(
        sha256(canonical.as_bytes()),
        "bc8270fff13f91d62b52a3d41f8c43ce3ca99287525c3a7f9a6ce28889bd11b0"
    );

    assert!(SQLITE_SOURCE.contains("let identities = rusqlite::registered_vfses()"));
    assert!(
        SQLITE_SOURCE
            .contains("[\"unix\", \"memdb\", \"unix-excl\", \"unix-dotfile\", \"unix-none\"]")
    );
    assert!(SQLITE_SOURCE.contains("if index == 1 { (2, 1_024) } else { (3, 512) }"));
    assert!(SQLITE_SOURCE.contains("identity.maximum_pathname != expected_maximum_pathname"));
    assert!(SQLITE_SOURCE.contains("for suffix in [\"-journal\", \"-wal\", \"-shm\"]"));
    assert!(SQLITE_SOURCE.contains("SQLITE_OPEN_NOFOLLOW"));
    assert!(!SQLITE_SOURCE.contains("OpenFlags::SQLITE_OPEN_URI"));
    assert!(!SQLITE_SOURCE.contains("OpenFlags::SQLITE_OPEN_CREATE"));
    assert!(!SQLITE_SOURCE.contains("OpenFlags::SQLITE_OPEN_EXCLUSIVE"));

    let filesystem = fixture(FILESYSTEM_FIXTURE);
    assert!(
        filesystem["path_cases"]
            .as_array()
            .expect("path cases")
            .iter()
            .any(|case| case.to_string().contains("file:"))
    );
}

#[test]
fn test_public_page_budget_and_busy_contract_is_pinned() {
    assert!(SQLITE_SOURCE.contains("const DATABASE_PAGE_SIZE: i64 = 4_096"));
    assert!(SQLITE_SOURCE.contains("const MAX_DATABASE_PAGES: i64 = 262_144"));
    assert!(SQLITE_SOURCE.contains("const BUSY_TIMEOUT_MILLISECONDS: i64 = 5_000"));
    assert!(SQLITE_SOURCE.contains("ConfigurationStatement::MaxPageCountSet262144"));
    assert!(SQLITE_SOURCE.contains("ConfigurationStatement::BusyTimeoutSet5000"));
    assert!(SQLITE_SOURCE.contains("failure.code == ErrorCode::DiskFull"));
    assert!(SQLITE_SOURCE.contains("ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked"));
    assert!(!SQLITE_SOURCE.contains("thread::sleep"));

    let authorizer = fixture(AUTHORIZER_FIXTURE);
    for role in [
        "creator",
        "preflight_reader",
        "projector_writer",
        "public_reader",
    ] {
        let statements = authorizer["roles"][role]["statement_inventory"]
            .as_array()
            .expect("statement inventory");
        assert!(statements.iter().any(|statement| {
            statement.to_string().contains("busy_timeout") && statement.to_string().contains("5000")
        }));
    }
}
