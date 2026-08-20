use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use cubikan_local::{ResponseClass, execute_request};
use serde_json::Value;

const V1_FIXTURE: &[u8] = include_bytes!("fixtures/durable-lifecycle-v1.json");
static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

fn absent_database() -> PathBuf {
    std::env::temp_dir().join(format!(
        "cubikan-local-unsupported-protocol-{}-{}.sqlite3",
        std::process::id(),
        NEXT_PATH.fetch_add(1, Ordering::Relaxed)
    ))
}

fn assert_unsupported(database: &PathBuf, request: &[u8]) {
    assert!(!database.exists());
    let response = execute_request(database, request);
    assert_eq!(response.class(), ResponseClass::RequestRejected);
    let value: Value = serde_json::from_slice(response.body()).expect("response should be JSON");
    assert_eq!(value["protocol_version"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["error"]["code"], "unsupported_protocol_version");
    assert_eq!(
        value["error"]["message"],
        "protocol version 1 is unsupported"
    );
    assert!(!database.exists());
}

#[test]
fn every_preserved_v1_fixture_request_is_unsupported_before_path_access() {
    let fixture: Value = serde_json::from_slice(V1_FIXTURE).expect("fixture should be JSON");
    let requests = fixture
        .as_object()
        .expect("fixture root should be an object");
    let database = absent_database();

    for (name, request) in requests {
        let bytes = serde_json::to_vec(request).expect("named request should serialize");
        assert_unsupported(&database, &bytes);
        assert!(!database.exists(), "{name} accessed the database path");
    }
}

#[test]
fn test_root_consumers_reject_v1_before_removed_authority() {
    let fixture: Value = serde_json::from_slice(V1_FIXTURE).expect("fixture should be JSON");
    let request = serde_json::to_vec(&fixture["create_01"]).expect("request should serialize");
    let database = absent_database();

    assert_unsupported(&database, &request);

    let local_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cli_root = local_root
        .parent()
        .expect("local crate should have a crates parent")
        .join("cubikan-cli");
    let local_sources = [
        fs::read_to_string(local_root.join("src/execution.rs")).expect("read local execution"),
        fs::read_to_string(local_root.join("src/protocol.rs")).expect("read local protocol"),
        fs::read_to_string(local_root.join("src/runner.rs")).expect("read local runner"),
    ];
    let cli_sources = [
        fs::read_to_string(cli_root.join("src/execution.rs")).expect("read CLI execution"),
        fs::read_to_string(cli_root.join("src/protocol.rs")).expect("read CLI protocol"),
        fs::read_to_string(cli_root.join("src/runner.rs")).expect("read CLI runner"),
    ];
    for forbidden in [
        "IntentUnit::new",
        "IntentUnitId::generate",
        "SqliteBackend::open",
        "CreateIntentUnit::new",
        "synthetic_origin",
    ] {
        assert!(
            local_sources
                .iter()
                .all(|source| !source.contains(forbidden)),
            "retired local root-consumer authority remains: {forbidden}"
        );
    }

    // T-1112 deliberately gives `cubikan` an in-memory core simulator while
    // preserving the T-1107 prohibition on durable, RPC, or synthetic-origin
    // authority. `cubikan-local` remains the unsupported-only bridge here.
    for forbidden in [
        "SqliteBackend",
        "rusqlite",
        "Connection::open",
        "subxt",
        "OnlineClient",
        "CreateIntentUnit",
        "synthetic_origin",
    ] {
        assert!(
            cli_sources.iter().all(|source| !source.contains(forbidden)),
            "stateless CLI gained external authority: {forbidden}"
        );
    }
}
