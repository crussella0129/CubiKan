#![cfg(target_os = "linux")]

use std::{
    error::Error,
    ffi::OsStr,
    fmt, fs,
    future::Future,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

use rusqlite::{Connection, params};
use serde_json::Value;

use super::*;
use crate::{
    projector::{
        synchronize_prepared,
        tests::{fixture_archive_through, full_fixture_archive},
    },
    stored,
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
static REAL_ATTESTATION_BRANCHES: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

#[derive(Clone)]
enum ScriptedAttestationOutcome {
    Archive(PreparedArchive),
    Interrupted,
}

struct ScriptedAttestationSource {
    outcome: ScriptedAttestationOutcome,
    calls: Arc<Mutex<Vec<String>>>,
}

impl AttestationArchiveSource for ScriptedAttestationSource {
    async fn fetch_prepared_through(
        &self,
        block_number: u64,
    ) -> Result<PreparedArchive, ProjectionError> {
        self.calls
            .lock()
            .expect("scripted attestation call log")
            .push(format!("fetch_through:{block_number}"));
        match &self.outcome {
            ScriptedAttestationOutcome::Archive(archive) => Ok(archive.clone()),
            ScriptedAttestationOutcome::Interrupted => {
                Err(ProjectionError::Archive(ArchiveError::Rpc {
                    operation: "scripted attestation fetch",
                    source: Box::new(ScriptedRpcInterruption),
                }))
            }
        }
    }
}

struct NoopWake;

#[derive(Debug)]
struct ScriptedRpcInterruption;

impl fmt::Display for ScriptedRpcInterruption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("scripted archive connection closed")
    }
}

impl Error for ScriptedRpcInterruption {}

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn ready<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => {
            panic!("the private scripted attestation source must be immediately ready")
        }
    }
}

impl TestDirectory {
    fn supported() -> Option<Self> {
        let root = std::env::var_os("CUBIKAN_TEST_SUPPORTED_ROOT")?;
        let path = Path::new(&root).join(format!(
            "t1110-attestation-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create supported-root attestation directory");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
            .expect("secure supported-root attestation directory");
        REAL_ATTESTATION_BRANCHES.fetch_add(1, Ordering::Relaxed);
        Some(Self(path))
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_full_rpc_stream_attestation_mints_one_pinned_read_or_nothing() {
    let Some(directory) = TestDirectory::supported() else {
        return;
    };
    let basename = OsStr::new("projection.sqlite3");
    let path = directory.0.join(basename);
    let projector = FinalizedProjector::create(&path).expect("fresh projection");
    let archive = full_fixture_archive();
    synchronize_prepared(&projector, &archive).expect("project full independent fixture");

    let calls = Arc::new(Mutex::new(Vec::new()));
    let source = ScriptedAttestationSource {
        outcome: ScriptedAttestationOutcome::Archive(archive.clone()),
        calls: Arc::clone(&calls),
    };
    let before_pin_calls = Arc::clone(&calls);
    let snapshot = ready(attest_finalized_projection_from(
        &projector,
        &source,
        move || {
            before_pin_calls
                .lock()
                .expect("scripted attestation call log")
                .push("before_pin".to_owned());
            Ok(())
        },
    ))
    .expect("the production source orchestration mints only after exact comparison");
    assert_eq!(
        *calls.lock().expect("scripted attestation call log"),
        ["fetch_through:5", "before_pin"]
    );
    let debug = format!("{snapshot:?}");
    assert!(debug.contains("block_number: 5"));
    assert!(debug.contains("last_global_sequence: Some(11)"));
    assert!(debug.contains("opaque single-use read"));

    let lagging_directory = TestDirectory::supported().expect("supported ext4 lagging directory");
    let lagging_path = lagging_directory.0.join(basename);
    let lagging_projector =
        FinalizedProjector::create(&lagging_path).expect("fresh lagging projection");
    let through_three = fixture_archive_through(3);
    synchronize_prepared(&lagging_projector, &through_three).expect("lagging checkpoint C=3");
    let lagging_snapshot = attest_prepared_projection(&lagging_projector, &through_three)
        .expect("checkpoint C below current fixture head H attests");
    assert!(format!("{lagging_snapshot:?}").contains("block_number: 3"));

    let interrupted_calls = Arc::new(Mutex::new(Vec::new()));
    let interrupted = ScriptedAttestationSource {
        outcome: ScriptedAttestationOutcome::Interrupted,
        calls: Arc::clone(&interrupted_calls),
    };
    let unexpected_pin = Arc::clone(&interrupted_calls);
    let interruption = ready(attest_finalized_projection_from(
        &projector,
        &interrupted,
        move || {
            unexpected_pin
                .lock()
                .expect("scripted interruption call log")
                .push("unexpected_pin".to_owned());
            Ok(())
        },
    ))
    .expect_err("scripted RPC interruption must fail before pinning");
    let archive_source = interruption
        .source()
        .expect("attestation error retains its archive source")
        .downcast_ref::<ArchiveError>()
        .expect("first source is the typed archive error");
    assert!(
        archive_source
            .source()
            .expect("archive RPC error retains the transport source")
            .downcast_ref::<ScriptedRpcInterruption>()
            .is_some()
    );
    let AttestationError::Archive(ArchiveError::Rpc { operation, source }) = interruption else {
        panic!("scripted interruption must retain the typed archive RPC error");
    };
    assert_eq!(operation, "scripted attestation fetch");
    assert!(source.downcast_ref::<ScriptedRpcInterruption>().is_some());
    assert_eq!(
        *interrupted_calls
            .lock()
            .expect("scripted interruption call log"),
        ["fetch_through:5"]
    );
    assert_eq!(
        read_candidate_checkpoint(&projector).expect("unchanged candidate after interruption"),
        archive.checkpoint().expect("fixture checkpoint")
    );

    let unavailable_directory =
        TestDirectory::supported().expect("supported ext4 unavailable directory");
    let unavailable_projector = FinalizedProjector::create(unavailable_directory.0.join(basename))
        .expect("schema-only projection");
    assert!(matches!(
        read_candidate_checkpoint(&unavailable_projector),
        Err(AttestationError::ProjectionUnavailable)
    ));

    for forgery in ["raw", "derived", "coherent"] {
        let forged_directory = TestDirectory::supported().expect("supported forgery directory");
        let forged_path = forged_directory.0.join(basename);
        let forged_projector =
            FinalizedProjector::create(&forged_path).expect("fresh forged projection");
        synchronize_prepared(&forged_projector, &archive).expect("forge baseline");
        let connection = Connection::open(&forged_path).expect("test-only corruption connection");
        match forgery {
            "raw" => {
                connection
                    .execute(
                        "UPDATE projected_events SET signer=zeroblob(32) WHERE global_sequence=?1",
                        [stored::encode_u64_blob(7).as_slice()],
                    )
                    .expect("forge raw accepted event");
            }
            "derived" => {
                connection
                    .execute(
                        "UPDATE intent_units SET phase='queued' WHERE id='00112233-4455-4677-8899-aabbccddeeff'",
                        [],
                    )
                    .expect("forge derived scalar while retaining envelope");
            }
            "coherent" => {
                let envelope: String = connection
                    .query_row(
                        "SELECT envelope FROM intent_units WHERE id='00112233-4455-4677-8899-aabbccddeeff'",
                        [],
                        |row| row.get(0),
                    )
                    .expect("read unit A envelope");
                let mut envelope: Value =
                    serde_json::from_str(&envelope).expect("parse canonical envelope");
                envelope["status"] = Value::String("active".to_owned());
                envelope["revision"] = Value::String("1".to_owned());
                envelope["history"]
                    .as_array_mut()
                    .expect("history array")
                    .pop()
                    .expect("completion history record");
                let envelope = serde_json::to_string(&envelope).expect("encode forged envelope");
                connection
                    .execute_batch("BEGIN IMMEDIATE")
                    .expect("begin coherent forgery transaction");
                connection
                    .execute(
                        "UPDATE intent_units SET status='active',revision=?1,last_global_sequence=?2,envelope=?3 WHERE id='00112233-4455-4677-8899-aabbccddeeff'",
                        params![
                            stored::encode_u64_blob(1).as_slice(),
                            stored::encode_u64_blob(6).as_slice(),
                            envelope,
                        ],
                    )
                    .expect("coherently rewind derived unit before removing its former event");
                connection
                    .execute(
                        "DELETE FROM projected_events WHERE global_sequence=?1",
                        [stored::encode_u64_blob(7).as_slice()],
                    )
                    .expect("remove completion event");
                connection
                    .execute(
                        "UPDATE projected_blocks SET cubikan_event_count=4,first_global_sequence=?1 WHERE block_number=?2",
                        params![
                            stored::encode_u64_blob(8).as_slice(),
                            stored::encode_u64_blob(4).as_slice(),
                        ],
                    )
                    .expect("coherently update block event range");
                connection
                    .execute_batch("COMMIT")
                    .expect("commit coherent forgery transaction");
            }
            _ => unreachable!(),
        }
        drop(connection);
        assert!(
            matches!(
                attest_prepared_projection(&forged_projector, &archive),
                Err(AttestationError::ProjectionMismatch)
            ),
            "{forgery}"
        );
    }

    let refresh_directory = TestDirectory::supported().expect("supported refresh directory");
    let refresh_path = refresh_directory.0.join(basename);
    let refresh_projector =
        FinalizedProjector::create(&refresh_path).expect("fresh refresh projection");
    synchronize_prepared(&refresh_projector, &through_three).expect("refresh baseline C=3");
    let candidate = through_three.checkpoint().expect("candidate checkpoint");
    let result = attest_prepared_projection_at_candidate(
        &refresh_projector,
        &through_three,
        candidate,
        || {
            synchronize_prepared(&refresh_projector, &archive)
                .map(|_| ())
                .map_err(AttestationError::from)
        },
    );
    assert!(matches!(result, Err(AttestationError::RefreshRequired)));
    assert!(REAL_ATTESTATION_BRANCHES.load(Ordering::Relaxed) >= 7);
}
