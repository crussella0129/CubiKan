use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    num::NonZeroU64,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
    },
    thread,
};

use cubikan_core::{
    AssociationSubject, ExternalReference, IntentSpecies, IntentUnit, IntentUnitId, PhaseId,
    RecordedAssociation, ReferenceNamespace, ReferenceText, RelationshipDefinition,
    RelationshipDefinitionKey, RelationshipDefinitionVersion, RelationshipIdentity,
    RelationshipPolicy, Workflow, WorkflowEdge, WorkflowId,
};
use rusqlite::{Connection, Params};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::*;

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
static REAL_DATABASE_BRANCHES: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn supported(label: &str) -> Option<Self> {
        let root = std::env::var_os("CUBIKAN_TEST_SUPPORTED_ROOT")?;
        let path = Path::new(&root).join(format!(
            "t1110-{label}-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create supported-root projector directory");
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                .expect("secure supported-root projector directory");
        }
        REAL_DATABASE_BRANCHES.fetch_add(1, Ordering::Relaxed);
        Some(Self(path))
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone)]
struct FakeArchiveSource {
    identity: ArchiveIdentity,
    blocks: Vec<SourceBlock>,
}

impl ArchiveSource for FakeArchiveSource {
    fn identity(&self) -> &ArchiveIdentity {
        &self.identity
    }

    fn blocks(&self) -> &[SourceBlock] {
        &self.blocks
    }
}

#[derive(Default)]
struct FaultWriter {
    pending: Vec<projection_store::ProjectionStatement>,
    committed: Vec<projection_store::ProjectionStatement>,
    fail_at: Option<usize>,
    statement_fault: InjectedStatementFault,
    fail_commit: bool,
}

#[derive(Clone, Copy, Default)]
enum InjectedStatementFault {
    #[default]
    ProjectionMismatch,
    StorageFull,
    SqliteLimit,
}

impl InjectedStatementFault {
    fn error(self) -> BackendError {
        match self {
            Self::ProjectionMismatch => BackendError::ProjectionMismatch,
            Self::StorageFull => BackendError::StorageFull(crate::StorageFailure::new(
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_FULL),
                    None,
                ),
            )),
            Self::SqliteLimit => {
                BackendError::Storage(crate::StorageFailure::new(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_TOOBIG),
                    Some("injected SQLite limit".to_owned()),
                )))
            }
        }
    }
}

impl ProjectionWriter for FaultWriter {
    fn execute<P: Params>(
        &mut self,
        statement: projection_store::ProjectionStatement,
        _parameters: P,
    ) -> Result<usize, BackendError> {
        if self.fail_at == Some(self.pending.len()) {
            return Err(self.statement_fault.error());
        }
        self.pending.push(statement);
        Ok(1)
    }
}

impl AtomicProjectionWriter for FaultWriter {
    fn commit_atomic_projection(&mut self) -> Result<(), BackendError> {
        if self.fail_commit {
            return Err(BackendError::ProjectionMismatch);
        }
        self.committed.append(&mut self.pending);
        Ok(())
    }

    fn rollback_atomic_projection(&mut self) -> Result<(), BackendError> {
        self.pending.clear();
        Ok(())
    }
}

fn identity() -> ArchiveIdentity {
    ArchiveIdentity {
        relay_genesis_hash: [0x11; 32],
        parachain_genesis_hash: [0x22; 32],
        deployment_id: [0x33; 32],
        initial_runtime_spec_version: 1,
        initial_runtime_code_hash: [0x44; 32],
    }
}

fn zero_block(identity: &ArchiveIdentity) -> SourceBlock {
    SourceBlock {
        number: 0,
        hash: identity.parachain_genesis_hash,
        parent_hash: [0; 32],
        runtime_spec_version: identity.initial_runtime_spec_version,
        runtime_code_hash: identity.initial_runtime_code_hash,
        extrinsic_hashes: Vec::new(),
        system_event_record_count: 0,
        events: Vec::new(),
    }
}

fn sample_unit() -> IntentUnit {
    let queued = PhaseId::from_bytes(b"queued").expect("phase");
    let workflow = Workflow::new_bounded(
        WorkflowId::from_bytes(b"delivery").expect("workflow"),
        [queued.clone()],
        queued.clone(),
        [],
        [queued],
    )
    .expect("workflow topology");
    IntentUnit::new(
        IntentUnitId::from_str("00000000-0000-4000-8000-000000000001").expect("unit id"),
        ExternalReference::new(
            ReferenceNamespace::from_bytes(b"book.intent").expect("namespace"),
            ReferenceText::from_bytes(b"sprint").expect("scope"),
            ReferenceText::from_bytes(b"S11").expect("value"),
        ),
        IntentSpecies::from_bytes(b"feature").expect("species"),
        workflow,
    )
}

fn unit_event(identity: &ArchiveIdentity) -> SourceEvent {
    SourceEvent {
        extrinsic_index: 1,
        system_event_index: 2,
        global_sequence: 1,
        deployment_id: identity.deployment_id,
        event_schema_version: EVENT_SCHEMA_VERSION,
        signer: [0xa1; 32],
        extrinsic_hash: [0xe1; 32],
        raw_scale_payload: vec![0, 1, 2],
        payload: SourcePayload::UnitCreated(sample_unit()),
    }
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/fixtures/finalized-events-v1")
}

fn fixture_json(relative: &str) -> Value {
    let bytes = fs::read(fixture_root().join(relative)).expect("read independent fixture JSON");
    serde_json::from_slice(&bytes).expect("parse independent fixture JSON")
}

fn decode_hex(text: &str) -> Vec<u8> {
    assert_eq!(text, text.to_ascii_lowercase());
    assert_eq!(text.len() % 2, 0);
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("hex pair");
            u8::from_str_radix(pair, 16).expect("lowercase hex")
        })
        .collect()
}

fn hash(text: &str) -> [u8; 32] {
    decode_hex(text).try_into().expect("32-byte fixture hash")
}

fn fixture_payload(relative: &str) -> Vec<u8> {
    let text = fs::read_to_string(fixture_root().join(relative)).expect("read SCALE fixture");
    assert!(text.ends_with('\n'));
    decode_hex(text.trim_end_matches('\n'))
}

fn fixture_workflow() -> Workflow {
    let queued = PhaseId::from_bytes(b"queued").expect("queued");
    let doing = PhaseId::from_bytes(b"doing").expect("doing");
    Workflow::new_bounded(
        WorkflowId::from_bytes(b"lifecycle-v1").expect("workflow id"),
        [queued.clone(), doing.clone()],
        queued.clone(),
        [WorkflowEdge::new(queued, doing.clone())],
        [doing],
    )
    .expect("fixture workflow")
}

fn fixture_unit(id: &str, intent: &str) -> IntentUnit {
    IntentUnit::new(
        IntentUnitId::from_str(id).expect("fixture unit id"),
        ExternalReference::new(
            ReferenceNamespace::from_bytes(b"book.intent").expect("origin namespace"),
            ReferenceText::from_bytes(b"sprint-11").expect("origin scope"),
            ReferenceText::from_bytes(intent.as_bytes()).expect("origin value"),
        ),
        IntentSpecies::from_bytes(b"task").expect("species"),
        fixture_workflow(),
    )
}

fn fixture_units() -> (IntentUnit, IntentUnit) {
    (
        fixture_unit("00112233-4455-4677-8899-aabbccddeeff", "INT-0008"),
        fixture_unit("10213243-5465-4767-98a9-bacbdcedfe0f", "INT-0014"),
    )
}

fn fixture_definition() -> RelationshipDefinition {
    RelationshipDefinition::new(
        RelationshipDefinitionKey::new(
            ReferenceNamespace::from_bytes(b"depends_on").expect("definition id"),
            RelationshipDefinitionVersion::new(7).expect("definition version"),
        ),
        Some(IntentSpecies::from_bytes(b"task").expect("source species")),
        Some(IntentSpecies::from_bytes(b"task").expect("target species")),
        RelationshipPolicy::Reject,
        RelationshipPolicy::Reject,
    )
}

fn fixture_relationship() -> RelationshipIdentity {
    let (unit_a, unit_b) = fixture_units();
    RelationshipIdentity::new(fixture_definition().key().clone(), unit_a.id(), unit_b.id())
}

fn fixture_reference() -> ExternalReference {
    ExternalReference::new(
        ReferenceNamespace::from_bytes(b"git.commit.sha256").expect("reference namespace"),
        ReferenceText::from_bytes(b"public-synthetic/repository").expect("reference scope"),
        ReferenceText::from_bytes(
            b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .expect("reference value"),
    )
}

fn fixture_payload_effect(sequence: u64) -> SourcePayload {
    let (unit_a, unit_b) = fixture_units();
    let relationship = fixture_relationship();
    match sequence {
        1 => SourcePayload::UnitCreated(unit_a),
        2 => SourcePayload::UnitCreated(unit_b),
        3 => SourcePayload::RelationshipDefinitionCreated(fixture_definition()),
        4 | 11 => SourcePayload::RelationshipCreated(relationship),
        5 => SourcePayload::AssociationRecorded(RecordedAssociation::new(
            unit_a.id(),
            AssociationSubject::Revision(0),
            fixture_reference(),
        )),
        6 => SourcePayload::UnitTransitioned {
            unit_id: unit_a.id(),
            committed_revision: 1,
            from: PhaseId::from_bytes(b"queued").expect("from"),
            to: PhaseId::from_bytes(b"doing").expect("to"),
        },
        7 => SourcePayload::UnitCompleted {
            unit_id: unit_a.id(),
            committed_revision: 2,
            phase: PhaseId::from_bytes(b"doing").expect("phase"),
        },
        8 => SourcePayload::RelationshipDeleted(relationship),
        9 => SourcePayload::AssociationRevoked(RecordedAssociation::new(
            unit_a.id(),
            AssociationSubject::Revision(0),
            fixture_reference(),
        )),
        10 => SourcePayload::AssociationRecorded(RecordedAssociation::new(
            unit_b.id(),
            AssociationSubject::WholeUnit,
            fixture_reference(),
        )),
        _ => panic!("unknown fixture global sequence"),
    }
}

fn fixture_source() -> FakeArchiveSource {
    let stream = fixture_json("valid-stream-v1.json");
    let expected = fixture_json("expected-projection-v1.json");
    let anchor = &expected["projection_anchor"];
    let identity = ArchiveIdentity {
        relay_genesis_hash: hash(anchor["relay_genesis_hash"].as_str().expect("relay hash")),
        parachain_genesis_hash: hash(
            anchor["parachain_genesis_hash"]
                .as_str()
                .expect("parachain hash"),
        ),
        deployment_id: hash(anchor["deployment_id"].as_str().expect("deployment id")),
        initial_runtime_spec_version: u32::try_from(
            anchor["initial_runtime_spec_version"]
                .as_u64()
                .expect("runtime spec"),
        )
        .expect("u32 runtime spec"),
        initial_runtime_code_hash: hash(
            anchor["initial_runtime_code_hash"]
                .as_str()
                .expect("runtime code hash"),
        ),
    };
    let blocks = stream["blocks"]
        .as_array()
        .expect("fixture blocks")
        .iter()
        .map(|block| {
            assert_eq!(block["finalized"], true);
            let body = fixture_json(block["body"].as_str().expect("body path"));
            let extrinsic_hashes = body["extrinsics"]
                .as_array()
                .expect("body extrinsics")
                .iter()
                .map(|extrinsic| hash(extrinsic["blake2_256"].as_str().expect("extrinsic hash")))
                .collect();
            let events = block["events"]
                .as_array()
                .expect("events")
                .iter()
                .map(|event| {
                    let sequence = event["global_sequence"]
                        .as_str()
                        .expect("sequence")
                        .parse::<u64>()
                        .expect("u64 sequence");
                    SourceEvent {
                        extrinsic_index: u32::try_from(
                            event["extrinsic_index"].as_u64().expect("extrinsic index"),
                        )
                        .expect("u32 extrinsic index"),
                        system_event_index: u32::try_from(
                            event["system_event_index"]
                                .as_u64()
                                .expect("system event index"),
                        )
                        .expect("u32 event index"),
                        global_sequence: sequence,
                        deployment_id: identity.deployment_id,
                        event_schema_version: EVENT_SCHEMA_VERSION,
                        signer: hash(event["signer"].as_str().expect("signer")),
                        extrinsic_hash: hash(
                            event["extrinsic_hash"].as_str().expect("extrinsic hash"),
                        ),
                        raw_scale_payload: fixture_payload(
                            event["payload"].as_str().expect("payload path"),
                        ),
                        payload: fixture_payload_effect(sequence),
                    }
                })
                .collect();
            SourceBlock {
                number: block["block_number"]
                    .as_str()
                    .expect("block number")
                    .parse()
                    .expect("u64 block number"),
                hash: hash(block["block_hash"].as_str().expect("block hash")),
                parent_hash: hash(block["parent_hash"].as_str().expect("parent hash")),
                runtime_spec_version: u32::try_from(
                    block["runtime_spec_version"].as_u64().expect("spec"),
                )
                .expect("u32 spec"),
                runtime_code_hash: hash(block["runtime_code_hash"].as_str().expect("runtime code")),
                extrinsic_hashes,
                system_event_record_count: u32::try_from(
                    block["system_event_record_count"]
                        .as_u64()
                        .expect("record count"),
                )
                .expect("u32 record count"),
                events,
            }
        })
        .collect();
    FakeArchiveSource { identity, blocks }
}

pub(crate) fn full_fixture_archive() -> PreparedArchive {
    prepare_from_source(&fixture_source()).expect("independent 6-block archive")
}

pub(crate) fn fixture_archive_through(block: usize) -> PreparedArchive {
    let mut source = fixture_source();
    source.blocks.truncate(block + 1);
    prepare_from_source(&source).expect("independent archive prefix")
}

fn fixture_i64(value: &Value, field: &str) -> i64 {
    value[field].as_i64().expect("fixture i64")
}

fn fixture_text<'a>(value: &'a Value, field: &str) -> &'a str {
    value[field].as_str().expect("fixture string")
}

fn fixture_optional_hex(value: &Value, field: &str) -> Option<Vec<u8>> {
    value[field].as_str().map(decode_hex)
}

fn expected_projection_fixture() -> StoredProjection {
    let expected = fixture_json("expected-projection-v1.json");
    let anchor = &expected["projection_anchor"];
    let anchor = vec![AnchorRow {
        singleton: fixture_i64(anchor, "singleton"),
        namespace: fixture_text(anchor, "namespace").to_owned(),
        relay_genesis_hash: decode_hex(fixture_text(anchor, "relay_genesis_hash")),
        parachain_genesis_hash: decode_hex(fixture_text(anchor, "parachain_genesis_hash")),
        para_id: fixture_i64(anchor, "para_id"),
        deployment_id: decode_hex(fixture_text(anchor, "deployment_id")),
        pallet_storage_version: fixture_i64(anchor, "pallet_storage_version"),
        event_schema_version: fixture_i64(anchor, "event_schema_version"),
        initial_runtime_spec_version: fixture_i64(anchor, "initial_runtime_spec_version"),
        initial_runtime_code_hash: decode_hex(fixture_text(anchor, "initial_runtime_code_hash")),
    }];
    let blocks = expected["projected_blocks"]
        .as_array()
        .expect("fixture block rows")
        .iter()
        .map(|row| BlockRow {
            anchor_singleton: fixture_i64(row, "anchor_singleton"),
            block_number: decode_hex(fixture_text(row, "block_number_be_hex")),
            block_hash: decode_hex(fixture_text(row, "block_hash")),
            parent_hash: decode_hex(fixture_text(row, "parent_hash")),
            runtime_spec_version: fixture_i64(row, "runtime_spec_version"),
            runtime_code_hash: decode_hex(fixture_text(row, "runtime_code_hash")),
            cubikan_event_count: fixture_i64(row, "cubikan_event_count"),
            first_global_sequence: fixture_optional_hex(row, "first_global_sequence_be_hex"),
            last_global_sequence: fixture_optional_hex(row, "last_global_sequence_be_hex"),
        })
        .collect::<Vec<_>>();
    let block_hashes = blocks
        .iter()
        .map(|block| (block.block_number.clone(), block.block_hash.clone()))
        .collect::<BTreeMap<_, _>>();
    let events = expected["projected_events"]
        .as_array()
        .expect("fixture event rows")
        .iter()
        .map(|row| {
            let block_number = decode_hex(fixture_text(row, "block_number_be_hex"));
            EventRow {
                block_hash: block_hashes
                    .get(&block_number)
                    .expect("event joins a fixture block")
                    .clone(),
                block_number,
                extrinsic_index: fixture_i64(row, "extrinsic_index"),
                system_event_index: fixture_i64(row, "system_event_index"),
                global_sequence: decode_hex(fixture_text(row, "global_sequence_be_hex")),
                deployment_id: decode_hex(fixture_text(row, "deployment_id")),
                event_schema_version: fixture_i64(row, "event_schema_version"),
                event_kind: fixture_text(row, "event_kind").to_owned(),
                scale_payload: fixture_payload(fixture_text(row, "scale_payload")),
                signer: decode_hex(fixture_text(row, "signer")),
                extrinsic_hash: decode_hex(fixture_text(row, "extrinsic_hash")),
            }
        })
        .collect();
    let row = &expected["projection_checkpoint"];
    let checkpoint = vec![CheckpointRow {
        singleton: fixture_i64(row, "singleton"),
        block_number: decode_hex(fixture_text(row, "block_number_be_hex")),
        block_hash: decode_hex(fixture_text(row, "block_hash")),
        last_global_sequence: fixture_optional_hex(row, "last_global_sequence_be_hex"),
        runtime_spec_version: fixture_i64(row, "runtime_spec_version"),
        runtime_code_hash: decode_hex(fixture_text(row, "runtime_code_hash")),
    }];
    let units = expected["intent_units"]
        .as_array()
        .expect("fixture unit rows")
        .iter()
        .map(|row| UnitRow {
            id: fixture_text(row, "id").to_owned(),
            envelope_version: fixture_i64(row, "envelope_version"),
            envelope: fixture_text(row, "envelope").to_owned(),
            origin_namespace: fixture_text(row, "origin_namespace").to_owned(),
            origin_scope: fixture_text(row, "origin_scope").to_owned(),
            origin_value: fixture_text(row, "origin_value").to_owned(),
            workflow_id: fixture_text(row, "workflow_id").to_owned(),
            species: fixture_text(row, "species").to_owned(),
            phase: fixture_text(row, "phase").to_owned(),
            status: fixture_text(row, "status").to_owned(),
            revision: decode_hex(fixture_text(row, "revision_be_hex")),
            last_global_sequence: decode_hex(fixture_text(row, "last_global_sequence_be_hex")),
        })
        .collect();
    let definitions = expected["relationship_definitions"]
        .as_array()
        .expect("fixture definition rows")
        .iter()
        .map(|row| DefinitionRow {
            definition_id: fixture_text(row, "definition_id").to_owned(),
            definition_version: decode_hex(fixture_text(row, "definition_version_be_hex")),
            directed: fixture_i64(row, "directed"),
            source_species: row["source_species"].as_str().map(str::to_owned),
            target_species: row["target_species"].as_str().map(str::to_owned),
            self_policy: fixture_text(row, "self_policy").to_owned(),
            cycle_policy: fixture_text(row, "cycle_policy").to_owned(),
            created_global_sequence: decode_hex(fixture_text(
                row,
                "created_global_sequence_be_hex",
            )),
        })
        .collect();
    let relationships = expected["intent_unit_relationships"]
        .as_array()
        .expect("fixture relationship rows")
        .iter()
        .map(|row| RelationshipRow {
            definition_id: fixture_text(row, "definition_id").to_owned(),
            definition_version: decode_hex(fixture_text(row, "definition_version_be_hex")),
            source_id: fixture_text(row, "source_id").to_owned(),
            target_id: fixture_text(row, "target_id").to_owned(),
            created_global_sequence: decode_hex(fixture_text(
                row,
                "created_global_sequence_be_hex",
            )),
        })
        .collect();
    let associations = expected["recorded_associations"]
        .as_array()
        .expect("fixture association rows")
        .iter()
        .map(|row| AssociationRow {
            unit_id: fixture_text(row, "unit_id").to_owned(),
            subject_kind: fixture_text(row, "subject_kind").to_owned(),
            subject_revision_key: decode_hex(fixture_text(row, "subject_revision_key_be_hex")),
            namespace: fixture_text(row, "namespace").to_owned(),
            scope: fixture_text(row, "scope").to_owned(),
            value: fixture_text(row, "value").to_owned(),
            created_global_sequence: decode_hex(fixture_text(
                row,
                "created_global_sequence_be_hex",
            )),
        })
        .collect();
    StoredProjection {
        anchor,
        blocks,
        events,
        checkpoint,
        units,
        definitions,
        relationships,
        associations,
    }
}

fn assert_fixture_coordinate(actual: &StoredProjection, coordinate: &Value) {
    let sequence = stored::encode_u64_blob(
        fixture_text(coordinate, "global_sequence")
            .parse()
            .expect("coordinate sequence"),
    )
    .to_vec();
    let event = actual
        .events
        .iter()
        .find(|event| event.global_sequence == sequence)
        .expect("derived row joins an accepted event");
    assert_eq!(
        event.block_number,
        stored::encode_u64_blob(
            fixture_text(coordinate, "block_number")
                .parse()
                .expect("coordinate block")
        )
    );
    assert_eq!(
        event.block_hash,
        decode_hex(fixture_text(coordinate, "block_hash"))
    );
    assert_eq!(
        event.extrinsic_index,
        fixture_i64(coordinate, "extrinsic_index")
    );
    assert_eq!(
        event.system_event_index,
        fixture_i64(coordinate, "system_event_index")
    );
    assert_eq!(
        event.extrinsic_hash,
        decode_hex(fixture_text(coordinate, "extrinsic_hash"))
    );
    assert_eq!(
        event.deployment_id,
        decode_hex(fixture_text(coordinate, "deployment_id"))
    );
    assert_eq!(
        actual.anchor[0].parachain_genesis_hash,
        decode_hex(fixture_text(coordinate, "parachain_genesis_hash"))
    );
}

fn assert_projection_matches_fixture(actual: &StoredProjection) {
    assert_eq!(actual, &expected_projection_fixture());
    let expected = fixture_json("expected-projection-v1.json");
    for unit in expected["intent_units"].as_array().expect("unit rows") {
        assert_fixture_coordinate(actual, &unit["last_coordinate"]);
    }
    assert_fixture_coordinate(
        actual,
        &expected["intent_unit_relationships"][0]["created_coordinate"],
    );
    assert_fixture_coordinate(
        actual,
        &expected["recorded_associations"][0]["created_coordinate"],
    );
}

fn fault_registry() -> Vec<Value> {
    fixture_json("fault-cases-v1.json")["cases"]
        .as_array()
        .expect("fault cases")
        .clone()
}

const BACKEND_REPLAY_CASE_IDS: [&str; 19] = [
    "identical_duplicate_complete_equality",
    "conflicting_duplicate_hash",
    "conflicting_duplicate_event_row",
    "skipped_block",
    "out_of_order_block",
    "wrong_parent_hash",
    "wrong_block_zero_hash",
    "wrong_deployment_id",
    "wrong_runtime_spec_version",
    "wrong_runtime_code_hash",
    "wrong_event_schema_version",
    "global_sequence_zero",
    "global_sequence_gap",
    "global_sequence_duplicate",
    "system_event_order_reversed",
    "overbound_payload",
    "replay_invalid_transition",
    "extrinsic_index_out_of_body",
    "extrinsic_hash_join_mismatch",
];

fn assert_fixture_seals_and_fault_inventory() {
    for (path, expected) in [
        (
            "manifest-v1.json",
            "90d969339a2b08d4872b7a9e4fa65d010a3c61bae6129c12a31462890bb03b71",
        ),
        (
            "inventory-v1.json",
            "feea5b6a39c0204dfb2d4be9c7cd12dc73060c6e1ca3db78809663d140885a63",
        ),
        (
            "fault-cases-v1.json",
            "fd11e2a6b55ea9aa948e62a74ba6871bae0d5fc6e980486643b3d1e390b01e29",
        ),
    ] {
        let raw = fs::read(fixture_root().join(path)).expect("read sealed fixture");
        assert_eq!(Sha256::digest(raw).to_vec(), decode_hex(expected), "{path}");
    }

    const EXPECTED_IDS: [&str; 75] = [
        "archive_blocks_flag_missing",
        "archive_state_flag_missing",
        "archive_flag_value_drift",
        "deployment_manifest_digest_mismatch",
        "metadata_digest_mismatch",
        "runtime_wasm_digest_mismatch",
        "genesis_body_probe_unavailable",
        "early_events_probe_unavailable",
        "mid_code_probe_unavailable",
        "current_header_probe_unavailable",
        "live_parachain_genesis_hash_mismatch",
        "live_relay_genesis_hash_mismatch",
        "live_para_id_storage_mismatch",
        "live_deployment_storage_mismatch",
        "live_pallet_storage_version_mismatch",
        "live_event_schema_storage_mismatch",
        "live_metadata_bytes_mismatch",
        "live_runtime_code_bytes_mismatch",
        "live_runtime_identity_mutations",
        "first_sync_statement_faults",
        "first_sync_commit_fault",
        "best_only_block",
        "displaced_finalized_block",
        "identical_duplicate_complete_equality",
        "conflicting_duplicate_hash",
        "conflicting_duplicate_event_row",
        "skipped_block",
        "out_of_order_block",
        "wrong_parent_hash",
        "wrong_block_zero_hash",
        "wrong_deployment_id",
        "wrong_runtime_spec_version",
        "wrong_runtime_spec_name",
        "wrong_runtime_impl_name",
        "wrong_runtime_authoring_version",
        "wrong_runtime_impl_version",
        "wrong_runtime_transaction_version",
        "wrong_runtime_state_version",
        "wrong_runtime_system_version",
        "wrong_runtime_api_version",
        "wrong_runtime_api_order",
        "wrong_runtime_code_hash",
        "wrong_event_schema_version",
        "wrong_cubikan_event_count",
        "global_sequence_zero",
        "global_sequence_gap",
        "global_sequence_duplicate",
        "system_event_order_reversed",
        "accepted_event_initialization_phase",
        "accepted_event_finalization_phase",
        "system_events_trailing_bytes",
        "system_events_declared_count_too_small",
        "system_events_declared_count_too_large",
        "malformed_scale_compact_length",
        "truncated_scale_payload",
        "trailing_scale_payload_bytes",
        "unknown_payload_variant",
        "overbound_payload",
        "replay_invalid_transition",
        "extrinsic_index_out_of_body",
        "extrinsic_hash_join_mismatch",
        "block_statement_faults",
        "block_commit_fault",
        "block_space_fault",
        "block_limit_fault",
        "rpc_source_interruption_mid_block",
        "attestation_raw_event_forgery",
        "attestation_derived_row_forgery",
        "attestation_coherent_event_and_derived_forgery",
        "checkpoint_advances_before_pin",
        "attestation_rpc_interrupt",
        "restart_after_block_fetch",
        "restart_before_block_commit",
        "restart_after_block_commit",
        "two_projectors_contend",
    ];
    let cases = fault_registry();
    let actual = cases
        .iter()
        .map(|case| fixture_text(case, "id"))
        .collect::<Vec<_>>();
    assert_eq!(actual, EXPECTED_IDS);
    assert_eq!(actual.iter().copied().collect::<BTreeSet<_>>().len(), 75);

    // Every sealed case is routed to the component that can exercise its
    // private seam; backend-owned routes are exercised by E2--E6 below.
    for case in &cases {
        let id = fixture_text(case, "id");
        let lane = match id {
            "archive_blocks_flag_missing"
            | "archive_state_flag_missing"
            | "archive_flag_value_drift"
            | "deployment_manifest_digest_mismatch"
            | "metadata_digest_mismatch"
            | "runtime_wasm_digest_mismatch"
            | "genesis_body_probe_unavailable"
            | "early_events_probe_unavailable"
            | "mid_code_probe_unavailable"
            | "current_header_probe_unavailable"
            | "live_parachain_genesis_hash_mismatch"
            | "live_relay_genesis_hash_mismatch"
            | "live_para_id_storage_mismatch"
            | "live_deployment_storage_mismatch"
            | "live_pallet_storage_version_mismatch"
            | "live_event_schema_storage_mismatch"
            | "live_metadata_bytes_mismatch"
            | "live_runtime_code_bytes_mismatch"
            | "live_runtime_identity_mutations"
            | "wrong_runtime_spec_name"
            | "wrong_runtime_impl_name"
            | "wrong_runtime_authoring_version"
            | "wrong_runtime_impl_version"
            | "wrong_runtime_transaction_version"
            | "wrong_runtime_state_version"
            | "wrong_runtime_system_version"
            | "wrong_runtime_api_version"
            | "wrong_runtime_api_order" => {
                "chain-client::test_rpc_archive_anchor_and_runtime_preflight_precedes_projection"
            }
            "best_only_block" | "displaced_finalized_block" => {
                "chain-client::scripted_finalized_stream_decodes_every_coordinate_and_body_hash"
            }
            "wrong_cubikan_event_count"
            | "accepted_event_initialization_phase"
            | "accepted_event_finalization_phase"
            | "system_events_trailing_bytes"
            | "system_events_declared_count_too_small"
            | "system_events_declared_count_too_large"
            | "malformed_scale_compact_length"
            | "truncated_scale_payload"
            | "trailing_scale_payload_bytes"
            | "unknown_payload_variant"
            | "rpc_source_interruption_mid_block" => {
                "chain-client::scripted_event_decoder_rejects_absence_topics_phase_count_trailing_and_body_join"
            }
            "first_sync_statement_faults"
            | "first_sync_commit_fault"
            | "block_statement_faults"
            | "block_commit_fault"
            | "block_space_fault"
            | "block_limit_fault" => "backend-writer",
            "attestation_raw_event_forgery"
            | "attestation_derived_row_forgery"
            | "attestation_coherent_event_and_derived_forgery"
            | "checkpoint_advances_before_pin"
            | "attestation_rpc_interrupt" => "backend-attestation",
            "restart_after_block_fetch"
            | "restart_before_block_commit"
            | "restart_after_block_commit"
            | "two_projectors_contend" => "backend-restart",
            _ => "backend-replay",
        };
        assert!(!lane.is_empty());
        assert!(case["expected"].is_object(), "{id}");
    }
}

#[test]
fn test_backend_stream_preflight_rejects_genesis_parent_before_database_open() {
    let identity = identity();
    let mut block = zero_block(&identity);
    block.parent_hash = [0xff; 32];
    let source = FakeArchiveSource {
        identity,
        blocks: vec![block],
    };

    assert!(matches!(
        prepare_from_source(&source),
        Err(ProjectionError::InvalidFinalizedStream)
    ));

    let nonexistent = Path::new("/definitely/not/a/cubikan/projection.sqlite3");
    let projector = FinalizedProjector::from_path(nonexistent).expect("syntactic private path");
    assert!(matches!(
        synchronize_fetched(&projector, Err(ProjectionError::InvalidFinalizedStream)),
        Err(ProjectionError::InvalidFinalizedStream)
    ));
    assert!(!nonexistent.exists());
}

#[test]
fn test_first_sync_bootstraps_anchor_block_zero_and_nullable_checkpoint() {
    let identity = identity();
    let archive = prepare_from_source(&FakeArchiveSource {
        identity: identity.clone(),
        blocks: vec![zero_block(&identity)],
    })
    .expect("zero-event archive");
    let block = &archive.blocks[0];
    let mut writer = FaultWriter::default();

    commit_prepared_block(&mut writer, &archive.identity, block, None)
        .expect("bootstrap transaction");

    assert_eq!(
        writer.committed,
        [
            projection_store::ProjectionStatement::InsertAnchor,
            projection_store::ProjectionStatement::InsertBlock,
            projection_store::ProjectionStatement::InsertCheckpoint,
        ]
    );
    assert_eq!(block.first_global_sequence, None);
    assert_eq!(block.last_global_sequence, None);
    assert_eq!(block.checkpoint_sequence, None);

    for fail_at in 0..writer.committed.len() {
        let mut writer = FaultWriter {
            fail_at: Some(fail_at),
            ..FaultWriter::default()
        };
        assert!(commit_prepared_block(&mut writer, &archive.identity, block, None).is_err());
        assert!(writer.pending.is_empty());
        assert!(writer.committed.is_empty());
    }
    let mut commit_fault = FaultWriter {
        fail_commit: true,
        ..FaultWriter::default()
    };
    assert!(commit_prepared_block(&mut commit_fault, &archive.identity, block, None).is_err());
    assert!(commit_fault.pending.is_empty());
    assert!(commit_fault.committed.is_empty());

    let mut source = FakeArchiveSource {
        identity: identity.clone(),
        blocks: vec![zero_block(&identity)],
    };
    source.blocks[0].system_event_record_count = 1;
    assert!(matches!(
        prepare_from_source(&source),
        Err(ProjectionError::InvalidFinalizedStream)
    ));
    source.blocks[0].system_event_record_count = 3;
    let event = unit_event(&identity);
    source.blocks[0].extrinsic_hashes = vec![[0; 32], event.extrinsic_hash];
    source.blocks[0].events.push(event);
    assert!(matches!(
        prepare_from_source(&source),
        Err(ProjectionError::InvalidFinalizedStream)
    ));

    let Some(directory) = TestDirectory::supported("bootstrap") else {
        return;
    };
    let basename = OsStr::new("projection.sqlite3");
    let path = directory.0.join(basename);
    let projector = FinalizedProjector::create(&path).expect("schema-v3 database");
    synchronize_prepared(&projector, &archive).expect("atomic block-zero bootstrap");
    let actual = read_projection(&directory.0, basename).expect("read bootstrapped projection");
    assert_eq!(
        actual,
        archive
            .complete_expected_projection()
            .expect("expected bootstrap")
    );
    assert_eq!(actual.blocks[0].first_global_sequence, None);
    assert_eq!(actual.blocks[0].last_global_sequence, None);
    assert_eq!(actual.checkpoint[0].last_global_sequence, None);

    let later_zero_archive = full_fixture_archive();
    let later_zero = later_zero_archive
        .blocks
        .last()
        .expect("fixture has a later zero-event block");
    assert!(later_zero.events.is_empty());
    assert_eq!(later_zero.first_global_sequence, None);
    assert_eq!(later_zero.last_global_sequence, None);
    assert_eq!(later_zero.checkpoint_sequence, Some(11));
    let later_basename = OsStr::new("later-zero.sqlite3");
    let later_projector = FinalizedProjector::create(directory.0.join(later_basename))
        .expect("fresh later-zero projection");
    synchronize_prepared(&later_projector, &later_zero_archive)
        .expect("later zero-event block retains the prior checkpoint sequence");
    let later_actual =
        read_projection(&directory.0, later_basename).expect("read later-zero projection");
    assert_eq!(later_actual.blocks[5].first_global_sequence, None);
    assert_eq!(later_actual.blocks[5].last_global_sequence, None);
    assert_eq!(
        later_actual.checkpoint[0].last_global_sequence,
        Some(stored::encode_u64_blob(11).to_vec())
    );
}

#[test]
fn test_finalized_block_projection_is_atomic_joined_and_ordered() {
    let archive = full_fixture_archive();
    let expected = archive.complete_expected_projection().expect("projection");
    assert_projection_matches_fixture(&expected);

    for (block_index, block) in archive.blocks.iter().enumerate() {
        let previous = block_index
            .checked_sub(1)
            .and_then(|index| archive.blocks.get(index));
        let mut successful = FaultWriter::default();
        write_prepared_block(&mut successful, &archive.identity, block, previous)
            .expect("enumerate block statements");
        let statement_count = successful.pending.len();
        assert!(statement_count >= 2);
        for fail_at in 0..statement_count {
            let mut writer = FaultWriter {
                fail_at: Some(fail_at),
                ..FaultWriter::default()
            };
            assert!(
                commit_prepared_block(&mut writer, &archive.identity, block, previous).is_err()
            );
            assert!(
                writer.pending.is_empty(),
                "block {block_index} statement {fail_at}"
            );
            assert!(writer.committed.is_empty());
        }
        for statement_fault in [
            InjectedStatementFault::StorageFull,
            InjectedStatementFault::SqliteLimit,
        ] {
            let mut writer = FaultWriter {
                fail_at: Some(statement_count / 2),
                statement_fault,
                ..FaultWriter::default()
            };
            let error = commit_prepared_block(&mut writer, &archive.identity, block, previous)
                .expect_err("injected resource failure must abort the whole block");
            match statement_fault {
                InjectedStatementFault::StorageFull => {
                    assert!(matches!(error, BackendError::StorageFull(_)))
                }
                InjectedStatementFault::SqliteLimit => {
                    assert!(matches!(error, BackendError::Storage(_)))
                }
                InjectedStatementFault::ProjectionMismatch => unreachable!(),
            }
            assert!(
                writer.pending.is_empty(),
                "block {block_index} resource rollback"
            );
            assert!(writer.committed.is_empty());
        }
        let mut commit_fault = FaultWriter {
            fail_commit: true,
            ..FaultWriter::default()
        };
        assert!(
            commit_prepared_block(&mut commit_fault, &archive.identity, block, previous).is_err()
        );
        assert!(
            commit_fault.pending.is_empty(),
            "block {block_index} commit"
        );
        assert!(commit_fault.committed.is_empty());
    }

    let Some(directory) = TestDirectory::supported("full-fixture") else {
        return;
    };
    let basename = OsStr::new("projection.sqlite3");
    let projector = FinalizedProjector::create(directory.0.join(basename)).expect("fresh database");
    let checkpoint = synchronize_prepared(&projector, &archive).expect("project all six blocks");
    assert_eq!(checkpoint.block_number(), 5);
    assert_eq!(
        checkpoint.last_global_sequence().map(NonZeroU64::get),
        Some(11)
    );
    let actual = read_projection(&directory.0, basename).expect("read complete projection");
    assert_projection_matches_fixture(&actual);
    assert_eq!(actual.blocks[5].cubikan_event_count, 0);
    assert_eq!(actual.blocks[5].first_global_sequence, None);
    assert_eq!(actual.blocks[5].last_global_sequence, None);
    assert_eq!(
        stored::decode_u64_blob(
            actual.checkpoint[0]
                .last_global_sequence
                .as_deref()
                .expect("retained sequence")
        ),
        Ok(11)
    );
}

#[test]
fn test_invalid_or_nonfinalized_stream_inputs_expose_no_progress() {
    assert_fixture_seals_and_fault_inventory();
    assert!(matches!(
        reserve_archive_blocks(u64::MAX),
        Err(ProjectionError::InvalidFinalizedStream)
    ));
    assert!(matches!(
        reserve_archive_blocks(u64::MAX - 1),
        Err(ProjectionError::InvalidFinalizedStream)
    ));

    let mut mutations = Vec::<(&str, FakeArchiveSource)>::new();
    let mut source = fixture_source();
    source.blocks[0].hash = [0xff; 32];
    mutations.push(("wrong_block_zero_hash", source));
    let mut source = fixture_source();
    source.blocks[0].parent_hash = [0xff; 32];
    mutations.push(("wrong_block_zero_parent", source));
    let mut source = fixture_source();
    source.blocks[0].system_event_record_count = 1;
    mutations.push(("block_zero_system_event", source));
    let mut source = fixture_source();
    source.blocks.remove(4);
    mutations.push(("skipped_block", source));
    let mut source = fixture_source();
    source.blocks.swap(4, 5);
    mutations.push(("out_of_order_block", source));
    let mut source = fixture_source();
    source.blocks[4].parent_hash = [0xff; 32];
    mutations.push(("wrong_parent_hash", source));
    let mut source = fixture_source();
    source.blocks[4].runtime_spec_version = 2;
    mutations.push(("wrong_runtime_spec_version", source));
    let mut source = fixture_source();
    source.blocks[4].runtime_code_hash = [0xff; 32];
    mutations.push(("wrong_runtime_code_hash", source));
    let mut source = fixture_source();
    source.blocks[4].events[0].deployment_id = [0xff; 32];
    mutations.push(("wrong_deployment_id", source));
    let mut source = fixture_source();
    source.blocks[4].events[0].event_schema_version = 2;
    mutations.push(("wrong_event_schema_version", source));
    for (name, replacement) in [
        ("global_sequence_zero", 0),
        ("global_sequence_gap", 8),
        ("global_sequence_duplicate", 6),
    ] {
        let mut source = fixture_source();
        source.blocks[4].events[0].global_sequence = replacement;
        mutations.push((name, source));
    }
    let mut source = fixture_source();
    source.blocks[4].events.swap(0, 1);
    mutations.push(("system_event_order_reversed", source));
    let mut source = fixture_source();
    source.blocks[4].events[0].raw_scale_payload.clear();
    mutations.push(("empty_payload", source));
    let mut source = fixture_source();
    source.blocks[4].events[0].raw_scale_payload = vec![0; MAX_SCALE_PAYLOAD_BYTES + 1];
    mutations.push(("overbound_payload", source));
    let mut source = fixture_source();
    source.blocks[4].system_event_record_count = 4;
    mutations.push(("event_count_exceeds_system_records", source));
    let mut source = fixture_source();
    source.blocks[4].events[0].system_event_index = source.blocks[4].system_event_record_count;
    mutations.push(("system_index_out_of_records", source));
    let mut source = fixture_source();
    source.blocks[4].events[0].extrinsic_index = u32::MAX;
    mutations.push(("extrinsic_index_out_of_body", source));
    let mut source = fixture_source();
    source.blocks[4].events[0].extrinsic_hash = [0xff; 32];
    mutations.push(("extrinsic_hash_join_mismatch", source));
    let mut source = fixture_source();
    if let SourcePayload::UnitCompleted { phase, .. } = &mut source.blocks[4].events[0].payload {
        *phase = PhaseId::from_bytes(b"queued").expect("invalid completion phase");
    } else {
        panic!("fixture sequence seven must be completion");
    }
    mutations.push(("replay_invalid_transition", source));

    let mutation_case_ids = mutations
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| BACKEND_REPLAY_CASE_IDS.contains(name))
        .collect::<BTreeSet<_>>();
    let expected_mutation_case_ids = BACKEND_REPLAY_CASE_IDS
        .iter()
        .copied()
        .filter(|name| {
            !matches!(
                *name,
                "identical_duplicate_complete_equality"
                    | "conflicting_duplicate_hash"
                    | "conflicting_duplicate_event_row"
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(mutation_case_ids, expected_mutation_case_ids);

    let Some(directory) = TestDirectory::supported("invalid-no-progress") else {
        for (_, source) in mutations {
            assert!(matches!(
                prepare_from_source(&source),
                Err(ProjectionError::InvalidFinalizedStream)
            ));
        }
        return;
    };
    let basename = OsStr::new("projection.sqlite3");
    let projector = FinalizedProjector::create(directory.0.join(basename)).expect("fresh baseline");
    let baseline = read_projection(&directory.0, basename).expect("schema-only baseline");
    for (name, source) in mutations {
        let fetched = prepare_from_source(&source);
        assert!(
            matches!(
                synchronize_fetched(&projector, fetched),
                Err(ProjectionError::InvalidFinalizedStream)
            ),
            "{name}"
        );
        assert_eq!(
            read_projection(&directory.0, basename).expect("unchanged baseline"),
            baseline,
            "{name}"
        );
    }

    let archive = full_fixture_archive();
    synchronize_prepared(&projector, &archive).expect("valid baseline");
    let before = read_projection(&directory.0, basename).expect("full baseline");
    synchronize_prepared(&projector, &archive).expect("identical complete no-op");
    assert_eq!(
        read_projection(&directory.0, basename).expect("no-op result"),
        before
    );

    let mut conflict_source = fixture_source();
    conflict_source.blocks[4].hash = [0x4f; 32];
    conflict_source.blocks[5].parent_hash = [0x4f; 32];
    let conflict = prepare_from_source(&conflict_source).expect("internally contiguous conflict");
    assert!(matches!(
        synchronize_prepared(&projector, &conflict),
        Err(ProjectionError::ConflictingFinalizedBlock)
    ));
    assert_eq!(
        read_projection(&directory.0, basename).expect("fatal conflict no-op"),
        before
    );

    let connection = Connection::open(directory.0.join(basename)).expect("test corruption handle");
    connection
        .execute(
            "UPDATE projected_events SET signer=zeroblob(32) WHERE global_sequence=?1",
            [stored::encode_u64_blob(10).as_slice()],
        )
        .expect("forge one accepted event row");
    drop(connection);
    assert!(matches!(
        synchronize_prepared(&projector, &archive),
        Err(ProjectionError::Backend(BackendError::ProjectionMismatch))
    ));
    let mut executed_case_ids = mutation_case_ids;
    executed_case_ids.extend([
        "identical_duplicate_complete_equality",
        "conflicting_duplicate_hash",
        "conflicting_duplicate_event_row",
    ]);
    assert_eq!(
        executed_case_ids,
        BACKEND_REPLAY_CASE_IDS.into_iter().collect::<BTreeSet<_>>()
    );
}

#[test]
fn test_archive_refresh_restart_and_projector_contention_fail_honestly() {
    let Some(directory) = TestDirectory::supported("restart-contention") else {
        return;
    };
    let basename = OsStr::new("projection.sqlite3");
    let path = directory.0.join(basename);
    let projector = FinalizedProjector::create(&path).expect("fresh projection");
    let through_three = fixture_archive_through(3);
    let full = full_fixture_archive();

    synchronize_prepared(&projector, &through_three).expect("baseline through block three");
    let through_three_state =
        read_projection(&directory.0, basename).expect("restart-before-commit baseline");
    {
        let mut interrupted_writer =
            open_projection_writer(&directory.0, basename).expect("open interrupted writer");
        interrupted_writer
            .begin_projection()
            .expect("begin block-four transaction");
        write_prepared_block(
            &mut interrupted_writer,
            &full.identity,
            &full.blocks[4],
            Some(&full.blocks[3]),
        )
        .expect("write block four before simulated process loss");
        drop(interrupted_writer);
    }
    assert_eq!(
        read_projection(&directory.0, basename).expect("dropped transaction rolls back"),
        through_three_state
    );
    drop(projector);
    let projector = FinalizedProjector::open(&path).expect("restart before block commit");
    assert_eq!(
        read_projection(&directory.0, basename).expect("restart sees committed baseline only"),
        through_three_state
    );

    let fetched_before_begin = full.clone();
    drop(projector);
    let reopened = FinalizedProjector::open(&path).expect("restart after complete fetch");
    synchronize_prepared(&reopened, &fetched_before_begin).expect("resume after fetch");
    let full_state = read_projection(&directory.0, basename).expect("post-restart state");
    assert_projection_matches_fixture(&full_state);

    drop(reopened);
    let reopened = FinalizedProjector::open(&path).expect("restart after commit before ack");
    synchronize_prepared(&reopened, &full).expect("complete equality restart no-op");
    assert_eq!(
        read_projection(&directory.0, basename).expect("restart no-op state"),
        full_state
    );

    let Some(contended_directory) = TestDirectory::supported("two-projectors") else {
        return;
    };
    let contended_path = contended_directory.0.join(basename);
    let baseline_projector =
        FinalizedProjector::create(&contended_path).expect("contended baseline schema");
    synchronize_prepared(&baseline_projector, &through_three).expect("contended baseline");
    drop(baseline_projector);
    let barrier = Arc::new(Barrier::new(2));
    let archive = Arc::new(full);
    let handles = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let archive = Arc::clone(&archive);
            let path = contended_path.clone();
            thread::spawn(move || {
                let projector = FinalizedProjector::open(path)?;
                barrier.wait();
                synchronize_prepared(&projector, &archive)
            })
        })
        .collect::<Vec<_>>();
    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().expect("projector thread"))
        .collect::<Vec<_>>();
    assert!(outcomes.iter().any(Result::is_ok));
    assert!(outcomes.iter().all(|outcome| {
        outcome.is_ok()
            || matches!(
                outcome,
                Err(ProjectionError::Backend(BackendError::StorageBusy(_)))
            )
    }));
    let final_state = read_projection(&contended_directory.0, basename)
        .expect("read serialized contention result");
    assert_projection_matches_fixture(&final_state);
    assert!(REAL_DATABASE_BRANCHES.load(Ordering::Relaxed) >= 2);
}
