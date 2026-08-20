#![cfg(target_os = "linux")]

#[path = "../src/submission_journal.rs"]
mod journal;

use std::{
    ffi::OsString,
    fs,
    os::unix::{
        ffi::OsStringExt,
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt, symlink},
    },
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

use journal::{
    JournalRecord, JournalState, LaneNames, MutationOperation, PublicationPoint, SignerLane,
    VfsObservation, classify_filesystem_fixture_case, validate_directory_observation_for_test,
    validate_regular_observation_for_test, validate_virtual_lane_path_for_test,
};
use sha2::{Digest, Sha256};

const FILESYSTEM_FIXTURE: &str =
    include_str!("../../../tests/fixtures/filesystem-boundary-v1.json");
const FIXTURE_MANIFEST: &[u8] =
    include_bytes!("../../../tests/fixtures/submission-journal-v1/manifest-v1.json");
const FIXTURE_INVENTORY: &[u8] =
    include_bytes!("../../../tests/fixtures/submission-journal-v1/inventory-v1.json");
const FIXTURE_VERIFIER: &[u8] =
    include_bytes!("../../../tests/fixtures/submission-journal-v1/verify_fixtures.py");
const LANE_PATH_FIXTURE: &str =
    include_str!("../../../tests/fixtures/submission-journal-v1/lane-path-vectors-v1.json");
const JOURNAL_VECTOR_FIXTURE: &str =
    include_str!("../../../tests/fixtures/submission-journal-v1/journal-vectors-v1.json");
const REJECTION_FIXTURE: &str =
    include_str!("../../../tests/fixtures/submission-journal-v1/rejection-cases-v1.json");
const TRANSITION_FIXTURE: &str =
    include_str!("../../../tests/fixtures/submission-journal-v1/transitions-v1.json");
const CRASH_FIXTURE: &str =
    include_str!("../../../tests/fixtures/submission-journal-v1/crash-points-v1.json");
const PREPARED_HEX: &str =
    include_str!("../../../tests/fixtures/submission-journal-v1/raw/journal/prepared.hex");
const ACCEPTED_HEX: &str = include_str!(
    "../../../tests/fixtures/submission-journal-v1/raw/journal/finalized-accepted.hex"
);
const DISPATCH_REJECTED_HEX: &str = include_str!(
    "../../../tests/fixtures/submission-journal-v1/raw/journal/finalized-dispatch-rejected.hex"
);
const INVARIANT_FAILED_HEX: &str = include_str!(
    "../../../tests/fixtures/submission-journal-v1/raw/journal/finalized-invariant-failed.hex"
);
const EXPIRED_HEX: &str = include_str!(
    "../../../tests/fixtures/submission-journal-v1/raw/journal/expired-not-included.hex"
);
const CHILD_TEST_NAME: &str = "test_cross_process_signer_lanes_serialize_with_explicit_nonclaims";
const CRASH_TEST_NAME: &str = "test_submission_crash_matrix_never_resends_unsafely";
const PROCESS_CHILD: &str = "CUBIKAN_JOURNAL_PROCESS_CHILD";
const CRASH_CHILD: &str = "CUBIKAN_JOURNAL_CRASH_CHILD";
const TEST_ROOT: &str = "CUBIKAN_TEST_SUPPORTED_ROOT";

static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

type SupportedOpen = fn(&Path, [u8; 32], [u8; 32]) -> Result<SignerLane, journal::JournalError>;

#[test]
fn test_submission_lane_path_lock_and_first_use_are_hardened() {
    assert_locked_fixture_identity();
    assert_every_shared_filesystem_classifier_case();
    assert_every_lane_path_vector();
    assert_canonical_journal_vectors_and_rejections();
    let _only_supported_open_signature: SupportedOpen = SignerLane::open;
    assert!(
        validate_regular_observation_for_test(true, 1_000, 1_001, 0o600, 256, Some(256)).is_err(),
        "a synthetic wrong-owner observation must reject"
    );
    assert!(
        validate_directory_observation_for_test(true, 1_000, 1_001, 0o700).is_err(),
        "a synthetic wrong-owner directory must reject"
    );

    let Some(directory) = TestDirectory::new("lane-boundary") else {
        return;
    };
    let deployment = [0x31; 32];
    let signer = [0x72; 32];
    let names = LaneNames::derive(directory.path(), &deployment, &signer)
        .expect("derive exact signer lane");
    let lane =
        SignerLane::open(directory.path(), deployment, signer).expect("open clean signer lane");
    assert!(
        lane.record().is_none(),
        "absence is the only clean first use"
    );
    assert_eq!(lane.names(), &names);
    let lock_path = directory.path().join(names.lock());
    let lock_before = fs::symlink_metadata(&lock_path).expect("persistent lock inode exists");
    assert!(lock_before.file_type().is_file());
    assert_eq!(lock_before.mode() & 0o777, 0o600);
    assert_eq!(lock_before.len(), 0);
    drop(lane);
    let lock_after = fs::symlink_metadata(&lock_path).expect("lock inode persists after unlock");
    assert_eq!(lock_after.dev(), lock_before.dev());
    assert_eq!(lock_after.ino(), lock_before.ino());

    let temp_path = directory.path().join(names.temporary());
    create_mode_0600(&temp_path, &[0x55; 117]);
    let lane = SignerLane::open(directory.path(), deployment, signer)
        .expect("clean one safely derived torn temporary under lock");
    assert!(!temp_path.exists());
    drop(lane);

    for hostile in [
        HostileFile::WrongMode,
        HostileFile::Oversized,
        HostileFile::Symlink,
        HostileFile::Directory,
    ] {
        let directory = TestDirectory::new_required("hostile-temp");
        let names = LaneNames::derive(directory.path(), &deployment, &signer)
            .expect("derive hostile temp lane");
        let temporary = directory.path().join(names.temporary());
        hostile.create(&temporary);
        assert!(
            SignerLane::open(directory.path(), deployment, signer).is_err(),
            "hostile temporary {hostile:?} must fail closed"
        );
        assert!(
            fs::symlink_metadata(&temporary).is_ok(),
            "hostile temporary must not be removed"
        );
    }

    for hostile in [
        HostileFile::WrongMode,
        HostileFile::Symlink,
        HostileFile::Directory,
    ] {
        let directory = TestDirectory::new_required("hostile-journal");
        let names = LaneNames::derive(directory.path(), &deployment, &signer)
            .expect("derive hostile journal lane");
        hostile.create(&directory.path().join(names.journal()));
        assert!(
            SignerLane::open(directory.path(), deployment, signer).is_err(),
            "hostile journal {hostile:?} must fail closed"
        );
    }

    let corrupt_directory = TestDirectory::new_required("corrupt-exact-journal");
    let corrupt_names = LaneNames::derive(corrupt_directory.path(), &deployment, &signer)
        .expect("derive corrupt journal lane");
    create_mode_0600(
        &corrupt_directory.path().join(corrupt_names.journal()),
        &[0; 256],
    );
    assert!(SignerLane::open(corrupt_directory.path(), deployment, signer).is_err());

    let insecure_directory = TestDirectory::new_required("insecure-directory");
    fs::set_permissions(insecure_directory.path(), fs::Permissions::from_mode(0o750))
        .expect("make test-owned directory insecure");
    assert!(SignerLane::open(insecure_directory.path(), deployment, signer).is_err());
}

fn assert_locked_fixture_identity() {
    assert_eq!(FIXTURE_MANIFEST.len(), 2_046);
    assert_eq!(FIXTURE_INVENTORY.len(), 3_245);
    for (bytes, expected) in [
        (
            FIXTURE_MANIFEST,
            "ef3241ee5cb7d1cda3f12c628aae1f0533fd3fb673ffeecae0dc0626c15f942c",
        ),
        (
            FIXTURE_INVENTORY,
            "590d2e4b414f11bfa2c4a6355a60bfefb4a339256fff25469a86fe19f6671e21",
        ),
        (
            FIXTURE_VERIFIER,
            "a39a121c51f902df02b7a317df25a69e57f7f6965f4709f4c3ddb707ebedfce7",
        ),
        (
            JOURNAL_VECTOR_FIXTURE.as_bytes(),
            "4b9275f79335875bd48f3086715e86224e1b744d3d2b9621acc92479d5ad54dd",
        ),
        (
            LANE_PATH_FIXTURE.as_bytes(),
            "9902716e5817c4971edf0fb37ed9ab15bd720a0cceb2db7414246382bec751ab",
        ),
        (
            REJECTION_FIXTURE.as_bytes(),
            "fddff1bf8953c2aee3d0d6cbaa14e22b4336514a502a127ceb92188c6540fef4",
        ),
        (
            TRANSITION_FIXTURE.as_bytes(),
            "6ee41515c2a7af580465938dbc5d88e0b3adf04ce4f6eead7644865bc777a7a5",
        ),
        (
            CRASH_FIXTURE.as_bytes(),
            "edb5cec4065892440b32279bdc6b7a3b9ac98575d99fcf007a6ff808d0c7ed4b",
        ),
    ] {
        assert_eq!(lower_hex(&Sha256::digest(bytes)), expected);
    }
}

#[test]
fn test_submission_crash_matrix_never_resends_unsafely() {
    if let Ok(specification) = std::env::var(CRASH_CHILD) {
        run_crash_child(&specification);
        return;
    }
    let Some(_) = supported_root() else {
        return;
    };

    assert_transition_fixture_and_real_fsm();
    assert_crash_fixture_contract();

    for point in publication_points() {
        run_crash_case(point, CrashTransition::Prepare);
        run_crash_case(point, CrashTransition::Resolve);
    }
    for point in removal_points() {
        run_crash_case(point, CrashTransition::Acknowledge);
    }
    for point in delivery_crash_points() {
        run_delivery_crash_case(point);
    }
}

#[test]
fn test_cross_process_signer_lanes_serialize_with_explicit_nonclaims() {
    if std::env::var_os(PROCESS_CHILD).is_some() {
        run_lock_child();
        return;
    }
    let Some(_) = supported_root() else {
        return;
    };

    let directory = TestDirectory::new_required("same-signer-processes");
    let ready_one = directory.path().join("ready-one");
    let ready_two = directory.path().join("ready-two");
    let release_one = directory.path().join("release-one");
    let release_two = directory.path().join("release-two");
    let mut first = spawn_lock_child(directory.path(), 0x44, &ready_one, &release_one);
    wait_for_file(&ready_one, "first same-signer process acquired lane");
    let names = LaneNames::derive(directory.path(), &[0x33; 32], &[0x44; 32])
        .expect("derive same-signer lock name");
    let lock_before = fs::metadata(directory.path().join(names.lock())).expect("lock inode");
    let mut second = spawn_lock_child(directory.path(), 0x44, &ready_two, &release_two);
    thread::sleep(Duration::from_millis(300));
    assert!(
        !ready_two.exists(),
        "second same-signer process must block on the persistent inode"
    );
    touch(&release_one);
    wait_for_file(
        &ready_two,
        "second same-signer process acquired after release",
    );
    touch(&release_two);
    assert_success(wait_child(&mut first), "first same-signer child");
    assert_success(wait_child(&mut second), "second same-signer child");
    let lock_after = fs::metadata(directory.path().join(names.lock())).expect("persistent lock");
    assert_eq!(
        (lock_before.dev(), lock_before.ino()),
        (lock_after.dev(), lock_after.ino())
    );

    let directory = TestDirectory::new_required("different-signer-processes");
    let ready_one = directory.path().join("ready-a");
    let ready_two = directory.path().join("ready-b");
    let release_one = directory.path().join("release-a");
    let release_two = directory.path().join("release-b");
    let mut first = spawn_lock_child(directory.path(), 0x51, &ready_one, &release_one);
    let mut second = spawn_lock_child(directory.path(), 0x52, &ready_two, &release_two);
    wait_for_file(&ready_one, "first different-signer lane acquired");
    wait_for_file(&ready_two, "second different-signer lane overlaps");
    let first_names = LaneNames::derive(directory.path(), &[0x33; 32], &[0x51; 32])
        .expect("derive first separate lane");
    let second_names = LaneNames::derive(directory.path(), &[0x33; 32], &[0x52; 32])
        .expect("derive second separate lane");
    assert_ne!(first_names, second_names);
    touch(&release_one);
    touch(&release_two);
    assert_success(wait_child(&mut first), "first different-signer child");
    assert_success(wait_child(&mut second), "second different-signer child");

    let other_projection = TestDirectory::new_required("alternate-projection-nonclaim");
    let alternate_names = LaneNames::derive(other_projection.path(), &[0x33; 32], &[0x51; 32])
        .expect("derive alternate-projection lane");
    assert_ne!(first_names, alternate_names);
    let source = include_str!("../src/submission_journal.rs");
    for explicit_nonclaim in [
        "External signer users and alternate projection directories are not coordinated",
        "Same-user deletion of an unresolved record is undetectable",
        "makes no exactly-once delivery claim",
    ] {
        assert!(source.contains(explicit_nonclaim));
    }
}

fn assert_every_shared_filesystem_classifier_case() {
    let fixture: serde_json::Value =
        serde_json::from_str(FILESYSTEM_FIXTURE).expect("filesystem fixture JSON");
    let cases = fixture["mount_classifier_cases"]
        .as_array()
        .expect("mount classifier cases");
    assert_eq!(
        cases.len(),
        34,
        "consume the complete locked classifier corpus"
    );
    for case in cases {
        let mountinfo = case["mountinfo_lines"]
            .as_array()
            .expect("mountinfo lines")
            .iter()
            .map(|line| line.as_str().expect("mountinfo line"))
            .collect::<Vec<_>>()
            .join("\n");
        let vfs = &case["sqlite_vfs"];
        let result = classify_filesystem_fixture_case(
            case["platform"].as_str().expect("platform"),
            Path::new(
                case["canonical_directory"]
                    .as_str()
                    .expect("canonical directory"),
            ),
            &mountinfo,
            parse_hex_u64(case["statfs_magic"].as_str().expect("statfs magic")),
            VfsObservation {
                requested_name: vfs["requested_name"].as_str().expect("VFS name"),
                registered: vfs["registered"].as_bool().expect("registered VFS"),
                built_in: vfs["built_in"].as_bool().expect("built-in VFS"),
            },
        );
        match case["expected"]["decision"]
            .as_str()
            .expect("expected decision")
        {
            "accept" => {
                let (mount_point, filesystem_type) = result.unwrap_or_else(|error| {
                    panic!("{} unexpectedly rejected: {error}", case["id"])
                });
                assert_eq!(
                    mount_point,
                    PathBuf::from(
                        case["expected"]["selected_mount_point"]
                            .as_str()
                            .expect("selected mount point")
                    ),
                    "{} selected mount",
                    case["id"]
                );
                assert_eq!(
                    filesystem_type,
                    case["expected"]["filesystem_type"]
                        .as_str()
                        .expect("filesystem type"),
                    "{} selected filesystem",
                    case["id"]
                );
            }
            "reject_before_access" => {
                assert!(result.is_err(), "{} unexpectedly accepted", case["id"]);
            }
            other => panic!("unknown classifier decision {other}"),
        }
    }
}

fn assert_every_lane_path_vector() {
    let fixture: serde_json::Value =
        serde_json::from_str(LANE_PATH_FIXTURE).expect("lane-path fixture JSON");
    let accepted = fixture["accepted"]
        .as_array()
        .expect("accepted lane vectors");
    assert_eq!(accepted.len(), 4);
    for vector in accepted {
        let raw_path = decode_hex(vector["path_hex"].as_str().expect("path hex"));
        let path = PathBuf::from(OsString::from_vec(raw_path.clone()));
        assert_eq!(
            raw_path.len(),
            vector["path_length"].as_u64().expect("path length") as usize
        );
        let deployment = decode_array(vector["deployment_id"].as_str().expect("deployment"));
        let signer = decode_array(vector["signer"].as_str().expect("signer"));
        let names = LaneNames::derive(&path, &deployment, &signer)
            .unwrap_or_else(|error| panic!("{} rejected: {error}", vector["id"]));
        assert_eq!(
            names.lock(),
            vector["basenames"]["lock"].as_str().expect("lock basename")
        );
        assert_eq!(
            names.journal(),
            vector["basenames"]["journal"]
                .as_str()
                .expect("journal basename")
        );
        assert_eq!(
            names.temporary(),
            vector["basenames"]["temporary"]
                .as_str()
                .expect("temporary basename")
        );
        assert_eq!(
            &names.lock()["cubikan-submission-".len().."cubikan-submission-".len() + 64],
            vector["digest"].as_str().expect("lane digest")
        );
    }

    let deployment = decode_array(
        accepted[0]["deployment_id"]
            .as_str()
            .expect("primary deployment"),
    );
    let signer = decode_array(accepted[0]["signer"].as_str().expect("primary signer"));
    for vector in fixture["rejected_paths"]
        .as_array()
        .expect("rejected path vectors")
    {
        let result = if let Some(virtual_length) = vector["virtual_length"].as_u64() {
            validate_virtual_lane_path_for_test(b"/", virtual_length)
                .and_then(|()| LaneNames::derive(Path::new("/"), &deployment, &signer).map(drop))
        } else {
            let raw_path = decode_hex(vector["path_hex"].as_str().expect("rejected path hex"));
            let path = PathBuf::from(OsString::from_vec(raw_path));
            LaneNames::derive(&path, &deployment, &signer).map(drop)
        };
        assert!(result.is_err(), "{} unexpectedly accepted", vector["id"]);
    }

    let primary = &accepted[0]["basenames"];
    for vector in fixture["rejected_basenames"]
        .as_array()
        .expect("rejected basename vectors")
    {
        let mut lock = primary["lock"].as_str().expect("primary lock").to_owned();
        let mut journal = primary["journal"]
            .as_str()
            .expect("primary journal")
            .to_owned();
        let mut temporary = primary["temporary"]
            .as_str()
            .expect("primary temporary")
            .to_owned();
        match vector["kind"].as_str().expect("basename kind") {
            "lock" => lock = vector["value"].as_str().expect("bad lock").to_owned(),
            "journal" => {
                journal = vector["value"].as_str().expect("bad journal").to_owned();
            }
            "temporary" => {
                temporary = vector["value"].as_str().expect("bad temp").to_owned();
            }
            other => panic!("unknown basename kind {other}"),
        }
        assert!(
            LaneNames::from_parts_for_test(lock, journal, temporary).is_err(),
            "{} unexpectedly accepted",
            vector["id"]
        );
    }
}

fn assert_canonical_journal_vectors_and_rejections() {
    let vectors: serde_json::Value =
        serde_json::from_str(JOURNAL_VECTOR_FIXTURE).expect("journal-vector fixture JSON");
    let records = vectors["records"].as_array().expect("journal records");
    assert_eq!(records.len(), 5);
    for vector in records {
        let bytes = journal_bytes(vector["state"].as_str().expect("journal state"));
        assert_eq!(bytes.len(), 256);
        assert_eq!(
            lower_hex(&Sha256::digest(&bytes)),
            vector["decoded_sha256"]
                .as_str()
                .expect("decoded record SHA-256")
        );
        let record = JournalRecord::decode(&bytes)
            .unwrap_or_else(|error| panic!("{} rejected: {error}", vector["state"]));
        assert_eq!(record.encode().expect("re-encode record").as_slice(), bytes);
        assert_eq!(
            record.deployment_id(),
            &decode_array(
                vectors["common"]["deployment_id"]
                    .as_str()
                    .expect("common deployment")
            )
        );
        assert_eq!(
            record.signer(),
            &decode_array(vectors["common"]["signer"].as_str().expect("common signer"))
        );
        assert_eq!(record.nonce(), 66_051);
        assert_eq!(record.signing_block_number(), 131);
        assert_eq!(record.birth(), 131);
        assert_eq!(record.death(), 194);
        assert_eq!(record.operation(), MutationOperation::CompleteUnit);
        assert_eq!(
            record.resolution_block_number().to_string(),
            vector["resolution_number"]
                .as_str()
                .expect("resolution number")
        );
        assert_eq!(
            lower_hex(record.resolution_block_hash()),
            vector["resolution_hash"].as_str().expect("resolution hash")
        );
    }

    let rejection: serde_json::Value =
        serde_json::from_str(REJECTION_FIXTURE).expect("rejection fixture JSON");
    let cases = rejection["cases"].as_array().expect("rejection cases");
    assert_eq!(cases.len(), 39);
    for case in cases {
        let base = journal_bytes(case["base"].as_str().expect("rejection base"));
        let mutated = apply_fixture_mutation(base, &case["mutation"]);
        match case["validation"].as_str().expect("validation boundary") {
            "codec" => assert!(
                JournalRecord::decode(&mutated).is_err(),
                "{} unexpectedly decoded",
                case["id"]
            ),
            "lane" => {
                let record = JournalRecord::decode(&mutated)
                    .unwrap_or_else(|error| panic!("{} codec failed: {error}", case["id"]));
                let deployment = decode_array(
                    vectors["common"]["deployment_id"]
                        .as_str()
                        .expect("primary deployment"),
                );
                let signer = decode_array(
                    vectors["common"]["signer"]
                        .as_str()
                        .expect("primary signer"),
                );
                assert!(
                    record.deployment_id() != &deployment || record.signer() != &signer,
                    "{} must differ from its derived lane",
                    case["id"]
                );
                if let Some(directory) = TestDirectory::new("lane-mismatch") {
                    let names = LaneNames::derive(directory.path(), &deployment, &signer)
                        .expect("derive lane mismatch path");
                    create_mode_0600(&directory.path().join(names.journal()), &mutated);
                    assert!(
                        SignerLane::open(directory.path(), deployment, signer).is_err(),
                        "{} must reject at the real lane boundary",
                        case["id"]
                    );
                }
            }
            other => panic!("unknown rejection validation boundary {other}"),
        }
    }
}

fn apply_fixture_mutation(mut bytes: Vec<u8>, mutation: &serde_json::Value) -> Vec<u8> {
    match mutation["kind"].as_str().expect("mutation kind") {
        "truncate" => {
            bytes.truncate(mutation["length"].as_u64().expect("truncate length") as usize)
        }
        "append" => bytes.extend(decode_hex(mutation["hex"].as_str().expect("append bytes"))),
        "xor" => {
            let offset = mutation["offset"].as_u64().expect("xor offset") as usize;
            bytes[offset] ^= mutation["value"].as_u64().expect("xor byte") as u8;
        }
        "patch" => {
            for patch in mutation["patches"].as_array().expect("patches") {
                let offset = patch["offset"].as_u64().expect("patch offset") as usize;
                let replacement = decode_hex(patch["hex"].as_str().expect("patch bytes"));
                bytes[offset..offset + replacement.len()].copy_from_slice(&replacement);
            }
            if mutation["recompute_checksum"].as_bool() == Some(true) {
                let mut hasher = Sha256::new();
                hasher.update(b"CubiKan submission-journal-v1\0");
                hasher.update(&bytes[..224]);
                bytes[224..256].copy_from_slice(&hasher.finalize());
            }
        }
        other => panic!("unknown fixture mutation {other}"),
    }
    bytes
}

fn assert_transition_fixture_and_real_fsm() {
    let fixture: serde_json::Value =
        serde_json::from_str(TRANSITION_FIXTURE).expect("transition fixture JSON");
    let matrix = fixture["allowed_matrix"]
        .as_object()
        .expect("allowed matrix");
    assert_eq!(matrix.len(), 6);
    assert_eq!(
        fixture["allowed"]
            .as_array()
            .expect("allowed transitions")
            .len(),
        9
    );
    assert_eq!(
        matrix["absent"],
        serde_json::json!([false, true, false, false, false, false])
    );
    assert_eq!(
        matrix["prepared"],
        serde_json::json!([false, false, true, true, true, true])
    );
    for terminal in [
        "finalized_accepted",
        "finalized_dispatch_rejected",
        "finalized_invariant_failed",
        "expired_not_included",
    ] {
        assert_eq!(
            matrix[terminal],
            serde_json::json!([true, false, false, false, false, false])
        );
    }

    let prepared = JournalRecord::decode(&journal_bytes("prepared")).expect("prepared fixture");
    for terminal_name in [
        "finalized_accepted",
        "finalized_dispatch_rejected",
        "finalized_invariant_failed",
        "expired_not_included",
    ] {
        let terminal =
            JournalRecord::decode(&journal_bytes(terminal_name)).expect("terminal record fixture");
        let directory = TestDirectory::new_required(&format!("fsm-{terminal_name}"));
        let mut lane = SignerLane::open(
            directory.path(),
            *prepared.deployment_id(),
            *prepared.signer(),
        )
        .expect("open clean FSM lane");
        assert!(lane.publish_resolved(terminal.clone()).is_err());
        lane.publish_prepared(prepared.clone())
            .expect("absent to prepared");
        assert!(lane.publish_prepared(prepared.clone()).is_err());
        lane.publish_resolved(terminal.clone())
            .expect("prepared to one terminal");
        assert!(lane.publish_resolved(terminal).is_err());
        assert!(lane.publish_prepared(prepared.clone()).is_err());
        lane.acknowledge_resolved()
            .expect("terminal to absent acknowledgement");
        let lane = SignerLane::open(
            directory.path(),
            *prepared.deployment_id(),
            *prepared.signer(),
        )
        .expect("reopen acknowledged lane");
        assert!(lane.record().is_none());
    }

    let directory = TestDirectory::new_required("prepared-removal-forbidden");
    let mut lane = SignerLane::open(
        directory.path(),
        *prepared.deployment_id(),
        *prepared.signer(),
    )
    .expect("open prepared-removal lane");
    lane.publish_prepared(prepared.clone())
        .expect("publish prepared");
    assert!(lane.acknowledge_resolved().is_err());
    let mut lane = SignerLane::open(
        directory.path(),
        *prepared.deployment_id(),
        *prepared.signer(),
    )
    .expect("prepared survives forbidden acknowledgement");
    let wrong_operation = JournalRecord::prepared(
        *prepared.deployment_id(),
        *prepared.signer(),
        prepared.nonce(),
        *prepared.extrinsic_hash(),
        prepared.signing_block_number(),
        *prepared.signing_block_hash(),
        MutationOperation::CreateUnit,
    )
    .expect("construct later incoming operation")
    .resolved(JournalState::FinalizedAccepted, 137, [0x89; 32])
    .expect("resolve later incoming operation");
    assert!(lane.publish_resolved(wrong_operation).is_err());
    assert_eq!(
        lane.record().map(JournalRecord::operation),
        Some(MutationOperation::CompleteUnit)
    );
}

fn assert_crash_fixture_contract() {
    let fixture: serde_json::Value =
        serde_json::from_str(CRASH_FIXTURE).expect("crash fixture JSON");
    assert_eq!(
        fixture["points"].as_array().expect("crash points").len(),
        23
    );
    assert_eq!(fixture["global_invariants"]["orphan_limit"], 1);
    assert_eq!(
        fixture["global_invariants"]["excluded"],
        "lying_hardware_power_loss"
    );
    let ids = fixture["points"]
        .as_array()
        .expect("crash points")
        .iter()
        .map(|point| point["id"].as_str().expect("crash point ID"))
        .collect::<std::collections::BTreeSet<_>>();
    for controlled_boundary in [
        "before_prepared_temp_create",
        "after_prepared_temp_create",
        "after_prepared_partial_write",
        "after_prepared_complete_write_and_checksum",
        "before_prepared_temp_fsync",
        "after_prepared_temp_fsync",
        "before_prepared_rename",
        "after_prepared_rename_before_parent_fsync",
        "after_prepared_parent_fsync_before_send",
        "before_resolution_temp_create",
        "after_resolution_temp_create",
        "after_resolution_partial_write",
        "after_resolution_complete_write_and_checksum",
        "after_resolution_temp_fsync",
        "after_resolution_rename_before_parent_fsync",
        "after_resolution_parent_fsync",
        "after_terminal_response_before_remove",
        "after_remove_before_parent_fsync",
        "after_remove_parent_fsync",
        "before_submit_and_watch",
        "after_submit_and_watch_begins",
        "watcher_loss",
        "response_loss",
    ] {
        assert!(ids.contains(controlled_boundary));
    }
}

fn journal_bytes(state: &str) -> Vec<u8> {
    decode_hex(match state {
        "prepared" => PREPARED_HEX,
        "finalized_accepted" => ACCEPTED_HEX,
        "finalized_dispatch_rejected" => DISPATCH_REJECTED_HEX,
        "finalized_invariant_failed" => INVARIANT_FAILED_HEX,
        "expired_not_included" => EXPIRED_HEX,
        other => panic!("unknown journal fixture state {other}"),
    })
}

#[derive(Clone, Copy, Debug)]
enum HostileFile {
    WrongMode,
    Oversized,
    Symlink,
    Directory,
}

impl HostileFile {
    fn create(self, path: &Path) {
        match self {
            Self::WrongMode => {
                let file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o640)
                    .open(path)
                    .expect("create wrong-mode file");
                file.sync_all().expect("sync wrong-mode file");
            }
            Self::Oversized => create_mode_0600(path, &[0x77; 257]),
            Self::Symlink => symlink("missing-target", path).expect("create hostile symlink"),
            Self::Directory => fs::DirBuilder::new()
                .mode(0o700)
                .create(path)
                .expect("create hostile nonregular directory"),
        }
    }
}

#[derive(Clone, Copy)]
enum CrashTransition {
    Prepare,
    Resolve,
    Acknowledge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeliveryCrashPoint {
    BeforeSubmitAndWatch,
    AfterSubmitAndWatchBegins,
    WatcherLoss,
    ResponseLoss,
}

impl DeliveryCrashPoint {
    const fn fixture_id(self) -> &'static str {
        match self {
            Self::BeforeSubmitAndWatch => "before_submit_and_watch",
            Self::AfterSubmitAndWatchBegins => "after_submit_and_watch_begins",
            Self::WatcherLoss => "watcher_loss",
            Self::ResponseLoss => "response_loss",
        }
    }

    const fn original_send_began(self) -> bool {
        !matches!(self, Self::BeforeSubmitAndWatch)
    }
}

impl CrashTransition {
    const fn name(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::Resolve => "resolve",
            Self::Acknowledge => "acknowledge",
        }
    }
}

fn run_crash_case(point: PublicationPoint, transition: CrashTransition) {
    let directory = TestDirectory::new_required(&format!("crash-{}-{point:?}", transition.name()));
    if matches!(
        transition,
        CrashTransition::Resolve | CrashTransition::Acknowledge
    ) {
        let mut lane = SignerLane::open(directory.path(), [0x31; 32], [0x72; 32])
            .expect("open lane before resolution crash");
        lane.publish_prepared(sample_record())
            .expect("publish old prepared record");
        if matches!(transition, CrashTransition::Acknowledge) {
            let resolved = lane
                .record()
                .expect("prepared record before acknowledgement setup")
                .resolved(JournalState::FinalizedAccepted, 140, [0xa5; 32])
                .expect("construct acknowledgement terminal");
            lane.publish_resolved(resolved)
                .expect("publish terminal before acknowledgement crash");
        }
    }
    let specification = format!(
        "{}|{}|{}",
        transition.name(),
        publication_point_name(point),
        directory.path().display()
    );
    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg(CRASH_TEST_NAME)
        .arg("--nocapture")
        .env(CRASH_CHILD, specification)
        .status()
        .expect("run real crash child");
    assert_eq!(
        status.code(),
        Some(86),
        "child must exit at injected boundary"
    );
    assert!(
        !directory.path().join("send-marker").exists(),
        "journal crash cannot reach a send callback"
    );
    let lane = SignerLane::open(directory.path(), [0x31; 32], [0x72; 32])
        .expect("restart accepts only an old/new complete record and cleans safe temp");
    assert!(
        !directory.path().join(lane.names().temporary()).exists(),
        "restart leaves no orphan temporary"
    );
    let after_rename = matches!(
        point,
        PublicationPoint::AfterRename
            | PublicationPoint::BeforeDirectorySync
            | PublicationPoint::AfterDirectorySync
    );
    match transition {
        CrashTransition::Prepare if after_rename => {
            assert_eq!(
                lane.record().map(JournalRecord::state),
                Some(JournalState::Prepared)
            );
        }
        CrashTransition::Prepare => assert!(lane.record().is_none()),
        CrashTransition::Resolve if after_rename => assert_eq!(
            lane.record().map(JournalRecord::state),
            Some(JournalState::FinalizedAccepted)
        ),
        CrashTransition::Resolve => {
            assert_eq!(
                lane.record().map(JournalRecord::state),
                Some(JournalState::Prepared)
            );
        }
        CrashTransition::Acknowledge if point == PublicationPoint::BeforeRemoval => assert_eq!(
            lane.record().map(JournalRecord::state),
            Some(JournalState::FinalizedAccepted)
        ),
        CrashTransition::Acknowledge => assert!(
            lane.record().is_none()
                || lane.record().map(JournalRecord::state) == Some(JournalState::FinalizedAccepted),
            "removal crash admits only the same terminal or durable absence"
        ),
    }
}

fn run_delivery_crash_case(point: DeliveryCrashPoint) {
    let directory = TestDirectory::new_required(&format!("crash-{}", point.fixture_id()));
    let mut lane = SignerLane::open(directory.path(), [0x31; 32], [0x72; 32])
        .expect("open lane before delivery-boundary crash");
    lane.publish_prepared(sample_record())
        .expect("durably publish prepared record before delivery boundary");
    drop(lane);

    let crash_specification = format!(
        "delivery|{}|{}",
        point.fixture_id(),
        directory.path().display()
    );
    let crash_status = spawn_crash_child(&crash_specification)
        .status()
        .expect("run real delivery-boundary crash child");
    assert_eq!(
        crash_status.code(),
        Some(86),
        "{} child must exit at its injected boundary",
        point.fixture_id()
    );
    assert_eq!(
        directory.path().join("original-send-marker").exists(),
        point.original_send_began(),
        "{} must preserve whether the first submit invocation began",
        point.fixture_id()
    );

    let after_crash = SignerLane::open(directory.path(), [0x31; 32], [0x72; 32])
        .expect("restart must reopen the durable prepared lane");
    assert_eq!(
        after_crash.record().map(JournalRecord::state),
        Some(JournalState::Prepared),
        "{} must retain the prepared record after the crash",
        point.fixture_id()
    );
    drop(after_crash);

    let restart_specification = format!(
        "restart-delivery|{}|{}",
        point.fixture_id(),
        directory.path().display()
    );
    let restart_status = spawn_crash_child(&restart_specification)
        .status()
        .expect("run real delivery-boundary restart child");
    assert_success(restart_status, "delivery-boundary restart child");
    assert!(
        directory.path().join("restart-observed-marker").exists(),
        "{} restart child must observe the prepared lane",
        point.fixture_id()
    );
    assert!(
        !directory.path().join("restart-send-marker").exists(),
        "{} restart must issue zero new sends",
        point.fixture_id()
    );

    let after_restart = SignerLane::open(directory.path(), [0x31; 32], [0x72; 32])
        .expect("reopen lane after delivery-boundary restart proof");
    assert_eq!(
        after_restart.record().map(JournalRecord::state),
        Some(JournalState::Prepared),
        "{} restart must leave the unresolved journal retained",
        point.fixture_id()
    );
}

fn spawn_crash_child(specification: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("current test executable"));
    command
        .arg("--exact")
        .arg(CRASH_TEST_NAME)
        .arg("--nocapture")
        .env(CRASH_CHILD, specification);
    command
}

fn run_crash_child(specification: &str) {
    let mut fields = specification.splitn(3, '|');
    let transition = fields.next().expect("crash transition");
    let point_name = fields.next().expect("crash point");
    let directory = PathBuf::from(fields.next().expect("crash directory"));
    if transition == "delivery" {
        run_delivery_crash_child(parse_delivery_crash_point(point_name), &directory);
        return;
    }
    if transition == "restart-delivery" {
        run_delivery_restart_child(parse_delivery_crash_point(point_name), &directory);
        return;
    }

    let point = parse_publication_point(point_name);
    let mut lane =
        SignerLane::open(&directory, [0x31; 32], [0x72; 32]).expect("crash child opens lane");
    let mut crash = |observed| {
        if observed == point {
            std::process::exit(86);
        }
        Ok(())
    };
    match transition {
        "prepare" => lane
            .publish_prepared_with_hook(sample_record(), &mut crash)
            .expect("all named prepare points are reached"),
        "resolve" => {
            let resolved = lane
                .record()
                .expect("prepared record before resolution crash")
                .resolved(JournalState::FinalizedAccepted, 140, [0xa5; 32])
                .expect("construct resolved record");
            lane.publish_resolved_with_hook(resolved, &mut crash)
                .expect("all named resolution points are reached");
        }
        "acknowledge" => lane
            .acknowledge_resolved_with_hook(&mut crash)
            .expect("all named acknowledgement points are reached"),
        other => panic!("unknown crash transition {other}"),
    }
    touch(&directory.join("send-marker"));
    panic!("crash point was not reached");
}

fn run_delivery_crash_child(point: DeliveryCrashPoint, directory: &Path) {
    let lane = SignerLane::open(directory, [0x31; 32], [0x72; 32])
        .expect("delivery crash child opens lane");
    assert_eq!(
        lane.record().map(JournalRecord::state),
        Some(JournalState::Prepared),
        "delivery crash child starts from a durable prepared record"
    );
    if point.original_send_began() {
        touch(&directory.join("original-send-marker"));
    }
    std::process::exit(86);
}

fn run_delivery_restart_child(point: DeliveryCrashPoint, directory: &Path) {
    let mut lane = SignerLane::open(directory, [0x31; 32], [0x72; 32])
        .expect("delivery restart child opens lane");
    assert_eq!(
        lane.record().map(JournalRecord::state),
        Some(JournalState::Prepared),
        "{} restart must observe the unresolved prepared record",
        point.fixture_id()
    );
    if lane.publish_prepared(sample_record()).is_ok() {
        touch(&directory.join("restart-send-marker"));
    }
    assert_eq!(
        lane.record().map(JournalRecord::state),
        Some(JournalState::Prepared),
        "{} restart must retain the original record",
        point.fixture_id()
    );
    touch(&directory.join("restart-observed-marker"));
}

fn delivery_crash_points() -> [DeliveryCrashPoint; 4] {
    [
        DeliveryCrashPoint::BeforeSubmitAndWatch,
        DeliveryCrashPoint::AfterSubmitAndWatchBegins,
        DeliveryCrashPoint::WatcherLoss,
        DeliveryCrashPoint::ResponseLoss,
    ]
}

fn parse_delivery_crash_point(name: &str) -> DeliveryCrashPoint {
    delivery_crash_points()
        .into_iter()
        .find(|point| point.fixture_id() == name)
        .unwrap_or_else(|| panic!("unknown delivery crash point {name}"))
}

fn publication_points() -> [PublicationPoint; 12] {
    [
        PublicationPoint::BeforeChecksum,
        PublicationPoint::AfterChecksum,
        PublicationPoint::BeforeTemporaryCreate,
        PublicationPoint::AfterTemporaryCreate,
        PublicationPoint::AfterPartialWrite,
        PublicationPoint::AfterCompleteWrite,
        PublicationPoint::BeforeFileSync,
        PublicationPoint::AfterFileSync,
        PublicationPoint::BeforeRename,
        PublicationPoint::AfterRename,
        PublicationPoint::BeforeDirectorySync,
        PublicationPoint::AfterDirectorySync,
    ]
}

fn publication_point_name(point: PublicationPoint) -> &'static str {
    match point {
        PublicationPoint::BeforeChecksum => "before-checksum",
        PublicationPoint::AfterChecksum => "after-checksum",
        PublicationPoint::BeforeTemporaryCreate => "before-temp-create",
        PublicationPoint::AfterTemporaryCreate => "after-temp-create",
        PublicationPoint::AfterPartialWrite => "after-partial-write",
        PublicationPoint::AfterCompleteWrite => "after-complete-write",
        PublicationPoint::BeforeFileSync => "before-file-fsync",
        PublicationPoint::AfterFileSync => "after-file-fsync",
        PublicationPoint::BeforeRename => "before-rename",
        PublicationPoint::AfterRename => "after-rename",
        PublicationPoint::BeforeDirectorySync => "before-directory-fsync",
        PublicationPoint::AfterDirectorySync => "after-directory-fsync",
        PublicationPoint::BeforeRemoval => "before-removal",
        PublicationPoint::AfterRemoval => "after-removal",
        PublicationPoint::BeforeRemovalDirectorySync => "before-removal-directory-fsync",
        PublicationPoint::AfterRemovalDirectorySync => "after-removal-directory-fsync",
    }
}

fn parse_publication_point(name: &str) -> PublicationPoint {
    publication_points()
        .into_iter()
        .chain(removal_points())
        .find(|point| publication_point_name(*point) == name)
        .unwrap_or_else(|| panic!("unknown publication point {name}"))
}

fn removal_points() -> [PublicationPoint; 4] {
    [
        PublicationPoint::BeforeRemoval,
        PublicationPoint::AfterRemoval,
        PublicationPoint::BeforeRemovalDirectorySync,
        PublicationPoint::AfterRemovalDirectorySync,
    ]
}

fn sample_record() -> JournalRecord {
    JournalRecord::prepared(
        [0x31; 32],
        [0x72; 32],
        66_051,
        [0x84; 32],
        131,
        [0x93; 32],
        MutationOperation::CompleteUnit,
    )
    .expect("valid sample prepared record")
}

fn run_lock_child() {
    let directory = PathBuf::from(std::env::var("CUBIKAN_JOURNAL_CHILD_DIRECTORY").expect("dir"));
    let signer = std::env::var("CUBIKAN_JOURNAL_CHILD_SIGNER")
        .expect("signer")
        .parse::<u8>()
        .expect("u8 signer");
    let ready = PathBuf::from(std::env::var("CUBIKAN_JOURNAL_CHILD_READY").expect("ready"));
    let release = PathBuf::from(std::env::var("CUBIKAN_JOURNAL_CHILD_RELEASE").expect("release"));
    let _lane =
        SignerLane::open(&directory, [0x33; 32], [signer; 32]).expect("child acquires signer lane");
    touch(&ready);
    wait_for_file(&release, "parent releases child lane");
}

fn spawn_lock_child(directory: &Path, signer: u8, ready: &Path, release: &Path) -> Child {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg(CHILD_TEST_NAME)
        .arg("--nocapture")
        .env(PROCESS_CHILD, "1")
        .env("CUBIKAN_JOURNAL_CHILD_DIRECTORY", directory)
        .env("CUBIKAN_JOURNAL_CHILD_SIGNER", signer.to_string())
        .env("CUBIKAN_JOURNAL_CHILD_READY", ready)
        .env("CUBIKAN_JOURNAL_CHILD_RELEASE", release)
        .spawn()
        .expect("spawn real signer-lane process")
}

fn wait_child(child: &mut Child) -> ExitStatus {
    child.wait().expect("wait for signer-lane process")
}

fn assert_success(status: ExitStatus, label: &str) {
    assert!(status.success(), "{label} failed with {status}");
}

fn wait_for_file(path: &Path, label: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(Instant::now() < deadline, "timed out: {label}");
        thread::sleep(Duration::from_millis(20));
    }
}

fn touch(path: &Path) {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .expect("create process marker")
        .sync_all()
        .expect("sync process marker");
}

fn parse_hex_u64(value: &str) -> u64 {
    u64::from_str_radix(value.strip_prefix("0x").expect("0x-prefixed integer"), 16)
        .expect("valid hexadecimal integer")
}

fn decode_hex(value: &str) -> Vec<u8> {
    let compact = value.trim();
    assert_eq!(compact.len() % 2, 0, "hex input must have whole bytes");
    compact
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("ASCII hex pair");
            u8::from_str_radix(pair, 16).expect("valid fixture hex")
        })
        .collect()
}

fn decode_array(value: &str) -> [u8; 32] {
    decode_hex(value)
        .try_into()
        .unwrap_or_else(|bytes: Vec<u8>| panic!("expected 32 bytes, got {}", bytes.len()))
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn create_mode_0600(path: &Path, bytes: &[u8]) {
    use std::io::Write;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .expect("create mode-0600 test file");
    file.write_all(bytes).expect("write mode-0600 test file");
    file.sync_all().expect("sync mode-0600 test file");
}

fn supported_root() -> Option<PathBuf> {
    let root = match std::env::var_os(TEST_ROOT) {
        Some(root) => PathBuf::from(root),
        None => return None,
    };
    assert!(root.is_absolute(), "{TEST_ROOT} must be absolute");
    let canonical = fs::canonicalize(&root).expect("canonicalize configured supported root");
    assert_eq!(
        canonical, root,
        "configured supported root must be canonical"
    );
    Some(root)
}

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Option<Self> {
        let root = supported_root()?;
        Some(Self::create(&root, label))
    }

    fn new_required(label: &str) -> Self {
        let root = supported_root().expect("supported root was established for this test");
        Self::create(&root, label)
    }

    fn create(root: &Path, label: &str) -> Self {
        let sequence = DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!("cubikan-journal-{}-{sequence}-{label}", std::process::id());
        let path = root.join(name);
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .expect("create owner-only test directory");
        assert_eq!(
            fs::canonicalize(&path).expect("canonicalize test directory"),
            path
        );
        Self { path }
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::set_permissions(&self.path, fs::Permissions::from_mode(0o700))
            .expect("restore test directory permissions");
        for entry in fs::read_dir(&self.path).expect("read test directory during cleanup") {
            let entry = entry.expect("read test entry during cleanup");
            let metadata = fs::symlink_metadata(entry.path()).expect("inspect test entry");
            if metadata.file_type().is_dir() {
                fs::remove_dir_all(entry.path()).expect("remove nested test directory");
            } else {
                fs::remove_file(entry.path()).expect("remove test file");
            }
        }
        fs::remove_dir(&self.path).expect("remove test directory");
    }
}
