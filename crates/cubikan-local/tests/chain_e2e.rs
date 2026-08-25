use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    os::fd::OwnedFd,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::OnceLock,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use cubikan_chain_client::DevSigner;
use cubikan_core::{
    AssociationSubject, ExternalReference, IntentUnitId, RecordedAssociation, ReferenceNamespace,
    ReferenceText,
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::Value;

const FIXTURE_BYTES: &[u8] = include_bytes!("../../../tests/chain-e2e/journey-v1.json");
const T1114_ASSOCIATION_BYTES: &[u8] =
    include_bytes!("../../../tests/fixtures/git/recorded-association-v1.json");
const SUPPORTED_ROOT_ENV: &str = "CUBIKAN_TEST_SUPPORTED_ROOT";
const SHA256SUM: &str = "/usr/lib/cargo/bin/coreutils/sha256sum";
const STAT: &str = "/usr/lib/cargo/bin/coreutils/stat";
const TIMEOUT: &str = "/usr/lib/cargo/bin/coreutils/timeout";
const SETSID: &str = "/usr/bin/setsid";
const KILL: &str = "/usr/bin/kill";
const REQUIRED_FILESYSTEM_MAGIC: &str = "ef53";
const REQUIRED_ROOT_MODE: u32 = 0o700;
const REQUIRED_FILE_MODE: u32 = 0o600;
const O_NOFOLLOW_FLAG: i32 = 0o400_000;
const O_CLOEXEC_FLAG: i32 = 0o2_000_000;
const O_DIRECTORY_FLAG: i32 = 0o200_000;
const MAX_LAUNCHER_STDOUT_BYTES: usize = 4096;
const MAX_LAUNCHER_STDERR_BYTES: usize = 65_536;
const MAX_AUDIT_ARTIFACT_BYTES: u64 = 4_194_304;
const MAX_RAW_CHAIN_SPEC_BYTES: u64 = 8_388_608;
const MAX_AUDIT_TOTAL_BYTES: u64 = 25_165_824;
const IGNORED_REASON: &str = "requires the pinned four-node local-chain gate";
const PROHIBITED_OUTPUT_KEYS: [&str; 14] = [
    "credential",
    "credentials",
    "mnemonic",
    "passphrase",
    "password",
    "private_key",
    "private_locator",
    "prompt",
    "provider_secret",
    "secret",
    "seed",
    "source_body",
    "token",
    "transcript",
];

static JOURNEY: OnceLock<Result<JourneyRun, String>> = OnceLock::new();

macro_rules! require {
    ($condition:expr, $($argument:tt)*) => {
        if !$condition {
            return Err(format!($($argument)*));
        }
    };
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JourneyFixture {
    format: String,
    version: u64,
    launcher: LauncherFixture,
    pinned_inputs: Vec<PinnedInput>,
    topology: TopologyFixture,
    pvf_workers: PvfWorkerFixture,
    endpoints: BTreeMap<String, String>,
    checkpoints: CheckpointsFixture,
    exact_recorded_association: ExactAssociationFixture,
    mutations: Vec<MutationFixture>,
    reads: Vec<ReadFixture>,
    archive_probe: ArchiveProbeFixture,
    semantic_projection: SemanticProjectionFixture,
    audit: AuditFixture,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LauncherFixture {
    path: String,
    success_stdout: String,
    evidence_format: String,
    timeout_seconds: u64,
    kill_after_seconds: u64,
    max_evidence_bytes: u64,
    bootstrap_log_max_bytes: u64,
    genesis_head_decoded_size: u64,
    genesis_head_decoded_sha256: String,
    materialized_toolchain_root: String,
    materialized_toolchain_mode: String,
    materialized_toolchain_filesystem_magic: String,
    materialized_toolchain_write_probe_errno: String,
    materialized_toolchain_mount_flags: Vec<String>,
    materialized_node_path: String,
    node_executable_size: u64,
    node_executable_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct PinnedInput {
    path: String,
    size: u64,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TopologyFixture {
    orchestrator_count: u64,
    node_count: u64,
    nodes: Vec<NodeFixture>,
    runtime_equal_groups: Vec<Vec<String>>,
    runtime_distinct_groups: Vec<Vec<String>>,
    normalizer_rejection_cases: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeFixture {
    role: String,
    kind: String,
    binary: String,
    generated_name: String,
    dev_seed: String,
    node_key: String,
    peer_id: String,
    primary_bootnodes: Vec<String>,
    relay_side_bootnodes: Vec<String>,
    primary_spec_bootnodes: Vec<String>,
    relay_side_spec_bootnodes: Vec<String>,
    listeners: Vec<String>,
    archive: bool,
    relay_side: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PvfWorkerFixture {
    root: String,
    filesystem_magic: String,
    directory_mode: String,
    write_probe_errno: String,
    mount_flags: Vec<String>,
    workers_path_flag: String,
    pool_caps: BTreeMap<String, String>,
    execution_sides: Vec<String>,
    host_path_prefix: String,
    node_impl_version: String,
    database_backend: String,
    database_path_components: Vec<String>,
    nested_namespace_capability_mask: String,
    maximum_supervisors_per_role_kind: u64,
    maximum_job_children_per_supervisor: u64,
    maximum_total_worker_processes: u64,
    monitor_interval_milliseconds: u64,
    maximum_observed_generations: u64,
    maximum_observed_host_paths: u64,
    maximum_observed_unix_sockets: u64,
    assets: Vec<PvfWorkerAssetFixture>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PvfWorkerAssetFixture {
    name: String,
    cache_path: String,
    materialized_path: String,
    size: u64,
    sha256: String,
    mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointsFixture {
    pre_mutation_readiness: PreMutationReadinessFixture,
    catch_up: String,
    #[serde(rename = "final")]
    final_: String,
    stop_role: String,
    survivor_role: String,
    before_c_last_mutation: String,
    after_c_first_mutation: String,
    final_mutation: String,
    stability_interval_milliseconds: u64,
    stability_interval_count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreMutationReadinessFixture {
    para_id: u32,
    minimum_finalized_number: u64,
    required_progress_blocks: u64,
    expected_scheduler_cores: u64,
    expected_distinct_aura_authorities: u64,
    maximum_best_finalized_gap: u64,
    timeout_milliseconds: u64,
    rpc_timeout_milliseconds: u64,
    poll_interval_milliseconds: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactAssociationFixture {
    fixture_path: String,
    fixture_size: u64,
    fixture_sha256: String,
    trailing_byte: u8,
    core_json: String,
    unit_id: String,
    subject_kind: String,
    subject_revision: u64,
    namespace: String,
    scope: String,
    value: String,
    required_evidence_sources: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationFixture {
    id: String,
    phase: String,
    signer: String,
    endpoint: String,
    work_category: String,
    operation: String,
    request: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadFixture {
    id: String,
    request: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchiveProbeFixture {
    range_start: u64,
    methods_per_block: Vec<String>,
    system_events_storage_key: String,
    runtime_code_storage_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SemanticProjectionFixture {
    snapshot_contracts: Vec<SnapshotContractFixture>,
    section_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotContractFixture {
    label: String,
    source_endpoint: String,
    read_endpoint: String,
    fresh_database: bool,
    database_deleted_before_build: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditFixture {
    chain_census: ChainCensusFixture,
    relay_chain_census: ChainCensusFixture,
    required_artifacts: Vec<AuditArtifactFixture>,
    zero_count_keys: Vec<String>,
    dev_signers: Vec<String>,
    dev_signer_accounts: BTreeMap<String, String>,
    synthetic_origins: Vec<String>,
    allowed_hosts: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChainCensusFixture {
    range_start: u64,
    maximum_retained_bytes: u64,
    minimum_session_rotation_count: Option<u64>,
    expected_initial_spot_price: Option<u128>,
    expected_grandpa_authorities: Option<Vec<(String, u64)>>,
    allowed_unsigned_calls: Vec<String>,
    allowed_events: Vec<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct AuditArtifactFixture {
    kind: String,
    logical_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JourneyEvidence {
    format: String,
    version: u64,
    isolation: IsolationEvidence,
    pins: Vec<ObservedPin>,
    runtime_executables: RuntimeExecutablesEvidence,
    bootstrap: BootstrapEvidence,
    topology: TopologyEvidence,
    pvf_workers: PvfWorkerEvidence,
    submissions: Vec<SubmissionEvidence>,
    checkpoints: CheckpointsEvidence,
    restart: RestartEvidence,
    snapshots: Vec<SnapshotEvidence>,
    audit: AuditEvidence,
    cleanup: CleanupEvidence,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeExecutablesEvidence {
    local_binary: RuntimeExecutableEvidence,
    orchestrator_node: OrchestratorExecutableEvidence,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeExecutableEvidence {
    path: String,
    before: OpenedFileEvidence,
    after: OpenedFileEvidence,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrchestratorExecutableEvidence {
    toolchain_root: String,
    toolchain_root_identity_before: FileIdentityEvidence,
    toolchain_root_identity_after: FileIdentityEvidence,
    toolchain_filesystem_magic: String,
    toolchain_write_probe_errno: String,
    toolchain_mount_before: PvfMountEvidence,
    toolchain_mount_after: PvfMountEvidence,
    path: String,
    expected_size: u64,
    expected_sha256: String,
    materialized_before: OpenedFileEvidence,
    materialized_after: OpenedFileEvidence,
    proc_exe_before: OpenedFileEvidence,
    proc_exe_after: OpenedFileEvidence,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapEvidence {
    log_max_bytes: u64,
    materializer_log: StableBootstrapFileEvidence,
    genesis_export_log: StableBootstrapFileEvidence,
    genesis_head: GenesisHeadExportEvidence,
    genesis_wasm: GenesisWasmExportEvidence,
    materializer_command: Vec<String>,
    export_commands: Vec<Vec<String>>,
    chain_spec_path: String,
    parachain_binary_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StableBootstrapFileEvidence {
    path: String,
    before: OpenedFileEvidence,
    after: OpenedFileEvidence,
    bytes_hex: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GenesisHeadExportEvidence {
    path: String,
    before: OpenedFileEvidence,
    after: OpenedFileEvidence,
    encoding: String,
    decoded_size: u64,
    decoded_sha256: String,
    chain_hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GenesisWasmExportEvidence {
    path: String,
    before: OpenedFileEvidence,
    after: OpenedFileEvidence,
    encoding: String,
    decoded_size: u64,
    decoded_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IsolationEvidence {
    loopback_network_namespace: bool,
    private_pid_namespace: bool,
    fresh_procfs: bool,
    pid_one_reaper_present: bool,
    external_connectivity_denied: bool,
    synthetic_chain: bool,
    dev_only: bool,
    supported_root: String,
    supported_root_filesystem_magic: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservedPin {
    path: String,
    read: OpenedFileEvidence,
    verified: bool,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FileIdentityEvidence {
    device: String,
    inode: String,
    size: u64,
    mode: String,
    link_count: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct OpenedFileEvidence {
    path_before: FileIdentityEvidence,
    descriptor_before: FileIdentityEvidence,
    descriptor_after: FileIdentityEvidence,
    path_after: FileIdentityEvidence,
    bytes_read: u64,
    sha256: String,
    regular_file: bool,
    symbolic_link: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TopologyEvidence {
    orchestrator: OrchestratorEvidence,
    nodes: Vec<NodeEvidence>,
    node_processes: Vec<NodeProcessEvidence>,
    ss_records: Vec<SocketRecordEvidence>,
    unexpected_node_processes: u64,
    unexpected_listeners: Vec<String>,
    normalizer_mutations: Vec<NormalizerMutationEvidence>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct NodeProcessEvidence {
    pid: u32,
    parent_pid: u32,
    start_time_ticks: u64,
    proc_exe_link: String,
    proc_cmdline_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrchestratorEvidence {
    pid: u32,
    executable: String,
    argv: Vec<String>,
    proc_cmdline_sha256: String,
    process_start_time_ticks_before: u64,
    process_start_time_ticks_after: u64,
    proc_directory_device_before: String,
    proc_directory_inode_before: String,
    proc_directory_device_after: String,
    proc_directory_inode_after: String,
    data_directory: String,
    data_directory_mode: String,
    listener_addresses: Vec<String>,
    node_child_pids: Vec<u32>,
    privileges: ProcessPrivilegeEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct ProcessPrivilegeEvidence {
    no_new_privileges: bool,
    capabilities: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeEvidence {
    role: String,
    kind: String,
    generated_name: String,
    dev_seed: String,
    node_key: String,
    peer_id: String,
    pid: u32,
    parent_pid: u32,
    reviewed_source_asset: String,
    argv: Vec<String>,
    proc_cmdline_sha256: String,
    environment: BTreeMap<String, String>,
    proc_environ_entries: Vec<String>,
    proc_environ_sha256: String,
    privileges: ProcessPrivilegeEvidence,
    proc_exe_link: String,
    proc_exe_size: u64,
    proc_exe_sha256: String,
    proc_exe_mode: String,
    proc_exe_seals: Vec<String>,
    post_seal_write_denied: bool,
    process_start_time_ticks_before: u64,
    process_start_time_ticks_after: u64,
    proc_directory_device_before: String,
    proc_directory_inode_before: String,
    proc_directory_device_after: String,
    proc_directory_inode_after: String,
    proc_directory_descriptor_device_before: String,
    proc_directory_descriptor_inode_before: String,
    proc_directory_descriptor_device_after: String,
    proc_directory_descriptor_inode_after: String,
    executable_device_before: String,
    executable_inode_before: String,
    executable_device_after: String,
    executable_inode_after: String,
    executable_descriptor_device_before: String,
    executable_descriptor_inode_before: String,
    executable_descriptor_device_after: String,
    executable_descriptor_inode_after: String,
    authenticated_archive_node_evidence: bool,
    data_directory: String,
    data_directory_mode: String,
    data_directories: Vec<DataDirectoryEvidence>,
    listener_addresses: Vec<String>,
    primary_bootnodes: Vec<String>,
    relay_side_bootnodes: Vec<String>,
    archive_flags: Vec<String>,
    primary_runtime_sha256: String,
    primary_chain_spec: ChainSpecEvidence,
    relay_side_runtime_sha256: Option<String>,
    relay_side_chain_spec: Option<ChainSpecEvidence>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct DataDirectoryEvidence {
    side: String,
    path: String,
    mode: String,
    device: String,
    inode: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ChainSpecEvidence {
    raw_path: String,
    raw_size: u64,
    raw_sha256: String,
    chain_spec_id: String,
    duplicate_keys_rejected: bool,
    top_level_bootnodes: Vec<String>,
    removed_top_level_members: Vec<String>,
    canonicalization: String,
    genesis_payload_sha256: String,
    live_genesis_hash: String,
    live_runtime_code_sha256: String,
    observed_rpc_endpoint: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PvfWorkerEvidence {
    root: String,
    root_identity: FileIdentityEvidence,
    root_owner_uid: u32,
    filesystem_magic: String,
    mount: PvfMountEvidence,
    write_probe_errno: String,
    assets: Vec<PvfWorkerAssetEvidence>,
    execution_sides: Vec<PvfExecutionSideEvidence>,
    runtime_monitor: PvfRuntimeMonitorEvidence,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct PvfMountEvidence {
    raw_mountinfo_line: String,
    mount_id: u64,
    parent_id: u64,
    major_minor: String,
    root: String,
    mount_point: String,
    mount_options: Vec<String>,
    optional_fields: Vec<String>,
    filesystem_type: String,
    mount_source: String,
    super_options: Vec<String>,
    bind: bool,
    read_only: bool,
    nodev: bool,
    nosuid: bool,
    executable: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PvfWorkerAssetEvidence {
    name: String,
    path: String,
    owner_uid: u32,
    read: OpenedFileEvidence,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PvfExecutionSideEvidence {
    role: String,
    side: String,
    workers_path_values: Vec<String>,
    database_values: Vec<String>,
    execute_workers_max_num_values: Vec<String>,
    prepare_workers_soft_max_num_values: Vec<String>,
    prepare_workers_hard_max_num_values: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PvfRuntimeMonitorEvidence {
    interval_milliseconds: u64,
    sample_count: u64,
    maximum_total_workers: u64,
    maximum_prepare_workers: u64,
    maximum_execute_workers: u64,
    maximum_by_role_kind: BTreeMap<String, u64>,
    maximum_job_children_by_supervisor: BTreeMap<String, u64>,
    observed_generations: Vec<PvfWorkerGenerationEvidence>,
    observed_host_paths: Vec<PvfHostPathEvidence>,
    observed_unix_sockets: Vec<PvfUnixSocketEvidence>,
    post_stop_processes: Vec<PvfWorkerGenerationEvidence>,
    post_stop_host_paths: Vec<PvfHostPathEvidence>,
    post_stop_unix_sockets: Vec<PvfUnixSocketEvidence>,
    overflowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct PvfWorkerGenerationEvidence {
    role: String,
    process_class: String,
    supervisor_pid: u32,
    kind: String,
    pid: u32,
    parent_pid: u32,
    start_time_ticks: u64,
    proc_exe_link: String,
    proc_comm: String,
    socket_path: String,
    worker_directory: String,
    database_path: String,
    artifacts_cache_path: String,
    argv: Vec<String>,
    proc_cmdline_sha256: String,
    environment: BTreeMap<String, String>,
    proc_environ_entries: Vec<String>,
    proc_environ_sha256: String,
    security: PvfProcessSecurityEvidence,
    proc_directory_device_before: String,
    proc_directory_inode_before: String,
    proc_directory_device_after: String,
    proc_directory_inode_after: String,
    proc_directory_descriptor_device_before: String,
    proc_directory_descriptor_inode_before: String,
    proc_directory_descriptor_device_after: String,
    proc_directory_descriptor_inode_after: String,
    executable_device_before: String,
    executable_inode_before: String,
    executable_device_after: String,
    executable_inode_after: String,
    executable_descriptor_device_before: String,
    executable_descriptor_inode_before: String,
    executable_descriptor_device_after: String,
    executable_descriptor_inode_after: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct PvfProcessSecurityEvidence {
    profile: String,
    no_new_privileges: bool,
    capabilities: BTreeMap<String, String>,
    status_uids: Vec<u32>,
    status_gids: Vec<u32>,
    uid_map_before: String,
    uid_map_after: String,
    gid_map_before: String,
    gid_map_after: String,
    user_namespace_before: String,
    user_namespace_after: String,
    mount_namespace_before: String,
    mount_namespace_after: String,
    orchestrator_user_namespace: String,
    orchestrator_mount_namespace: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct PvfHostPathEvidence {
    path: String,
    file_type: String,
    mode: String,
    device: String,
    inode: String,
    owner_uid: u32,
    mode_after: String,
    device_after: String,
    inode_after: String,
    descriptor_device_before: Option<String>,
    descriptor_inode_before: Option<String>,
    descriptor_device_after: Option<String>,
    descriptor_inode_after: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct PvfUnixSocketEvidence {
    path: String,
    socket_type: String,
    state: String,
    inode: String,
    raw_line: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SocketRecordEvidence {
    protocol: String,
    address: String,
    pid: u32,
    raw_line: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NormalizerMutationEvidence {
    case: String,
    rejected: bool,
    exit_code: i32,
    launched_node_processes: u64,
    executable: String,
    argv_sha256: String,
    environment_sha256: String,
    stdout_sha256: String,
    stderr_sha256: String,
    node_processes_before: Vec<NodeProcessEvidence>,
    node_processes_after: Vec<NodeProcessEvidence>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SubmissionEvidence {
    id: String,
    phase: String,
    signer: String,
    endpoint: String,
    executable: String,
    argv: Vec<String>,
    environment: BTreeMap<String, String>,
    environment_sha256: String,
    work_category: String,
    operation: String,
    request_sha256: String,
    response: Value,
    response_body_hex: String,
    response_body_size: u64,
    response_sha256: String,
    response_stdout_size: u64,
    response_stdout_sha256: String,
    result_kind: String,
    submission_count: u64,
    inclusion_count: u64,
    finalization_count: u64,
    accepted_event_count: u64,
    projection_applied: bool,
    block_number: u64,
    block_hash: String,
    extrinsic_hash: String,
    extrinsic_index: u32,
    event_index: u32,
    recorded_association_core_json: Option<String>,
    endpoint_observations: Vec<FinalizedObservationEvidence>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FinalizedObservationEvidence {
    endpoint: String,
    finalized_head_number: u64,
    finalized_head_hash: String,
    block_number: u64,
    block_hash: String,
    extrinsic_hash: String,
    extrinsic_index: u32,
    event_index: u32,
    matching_extrinsic_count: u64,
    matching_event_count: u64,
    accepted_pallet: String,
    accepted_event: String,
    accepted_deployment_id: String,
    accepted_event_schema_version: u64,
    accepted_global_sequence: String,
    accepted_signer: String,
    accepted_payload_variant: String,
    accepted_payload_scale_hex: String,
    accepted_payload_sha256: String,
    accepted_payload: Value,
    accepted_effect: Value,
    accepted_effect_sha256: String,
    finalized_header_sha256: String,
    block_extrinsics_sha256: String,
    system_events_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointsEvidence {
    pre_mutation: PreMutationReadinessEvidence,
    c: NamedCheckpointEvidence,
    f: NamedCheckpointEvidence,
    stopped_role: String,
    survivor_role: String,
    stopped_at_c: bool,
    survivor_finalized_mutation_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreMutationReadinessEvidence {
    para_id: u32,
    baseline: PreMutationBaselineEvidence,
    elapsed_milliseconds: u64,
    sample_count: u64,
    relay_finalized: ChainHeadEvidence,
    relay_para_header: RelayParaHeadEvidence,
    para_registered: bool,
    relay_endpoints_equal: bool,
    scheduler_cores: u64,
    active_config_scale_sha256: String,
    collator_finalized: ChainHeadEvidence,
    collator_finalized_endpoints_equal: bool,
    historical_block_hashes: HistoricalBlockHashesEvidence,
    collator_a_best: BestHeadEvidence,
    collator_b_best: BestHeadEvidence,
    authorities_a: Vec<String>,
    authorities_b: Vec<String>,
    authorities_equal: bool,
    distinct_authority_count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreMutationBaselineEvidence {
    relay_finalized: ChainHeadEvidence,
    relay_para_header: RelayParaHeadEvidence,
    collator_a_finalized: ChainHeadEvidence,
    collator_b_finalized: ChainHeadEvidence,
    para_registered: bool,
    relay_endpoints_equal: bool,
    scheduler_cores: u64,
    active_config_scale_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChainHeadEvidence {
    number: u64,
    hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RelayParaHeadEvidence {
    number: u64,
    hash: String,
    head_data_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoricalBlockHashesEvidence {
    collator_a: String,
    collator_b: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BestHeadEvidence {
    number: u64,
    hash: String,
    finalized_gap: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NamedCheckpointEvidence {
    name: String,
    number: u64,
    hash: String,
    projection_coordinate_sha256: String,
    stability_intervals: Vec<FinalityStabilityIntervalEvidence>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FinalityStabilityIntervalEvidence {
    index: u64,
    start_number: u64,
    start_hash: String,
    end_number: u64,
    end_hash: String,
    elapsed_milliseconds: u64,
    sample_count: u64,
    unchanged: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RestartEvidence {
    role: String,
    endpoint: String,
    pid_before: u32,
    pid_after: u32,
    process_start_time_ticks_after: u64,
    data_directory_before: String,
    data_directory_after: String,
    data_directory_mode_before: String,
    data_directory_mode_after: String,
    data_directory_device_before: String,
    data_directory_inode_before: String,
    data_directory_device_after: String,
    data_directory_inode_after: String,
    config_sha256_before: String,
    config_sha256_after: String,
    frozen_command: Vec<String>,
    frozen_command_sha256: String,
    frozen_log_path: String,
    spawn_executable: String,
    spawn_args: Vec<String>,
    teardown_exit_code: Option<i32>,
    teardown_signal: String,
    listener_records_after: Vec<SocketRecordEvidence>,
    archive_flags_before: Vec<String>,
    archive_flags_after: Vec<String>,
    synchronized_number: u64,
    synchronized_hash: String,
    source_eligible_before_probes: bool,
    source_use_count_before_probes: u64,
    source_eligible_after_probes: bool,
    source_use_count_after_probes: u64,
    source_gate: RebuildSourceGateEvidence,
    identity_a: EndpointIdentityEvidence,
    identity_b: EndpointIdentityEvidence,
    restarted_node: NodeEvidence,
    stop: StopEvidence,
    archive_probes: ArchiveProbeEvidence,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RebuildSourceGateEvidence {
    format: String,
    gate_sequence: u64,
    identity_a_sha256: String,
    identity_b_sha256: String,
    archive_probes_sha256: String,
    transcript: Vec<RebuildSourceGateEntryEvidence>,
    transcript_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RebuildSourceGateEntryEvidence {
    sequence: u64,
    kind: String,
    label: String,
    source_endpoint: Option<String>,
    gate_open: bool,
    identity_prerequisites_complete: bool,
    archive_prerequisites_complete: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StopEvidence {
    transcript: StopTranscriptEvidence,
    transcript_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StopTranscriptEvidence {
    signal: String,
    signal_send_succeeded: bool,
    pid_cleared_before_signal: bool,
    sent_to_pid: u32,
    sent_to_start_time_ticks: u64,
    pre_signal_proc_device: String,
    pre_signal_proc_inode: String,
    wait_observed: bool,
    exit_code: Option<i32>,
    termination_signal: Option<i32>,
    generation_wait_observed: bool,
    proc_probe_errno: String,
    listener_probe_errors: BTreeMap<String, String>,
    observed_before_first_survivor_mutation: bool,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct EndpointIdentityEvidence {
    relay_genesis_hash: String,
    relay_genesis_payload_sha256: String,
    parachain_genesis_hash: String,
    parachain_genesis_payload_sha256: String,
    deployment_id: String,
    checkpoint_hash: String,
    runtime_spec_version: u64,
    event_schema_version: u64,
    pallet_storage_version: u64,
    runtime_code_sha256: String,
    runtime_code_chain_hash: String,
    metadata_sha256: String,
    projection_checkpoint_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ArchiveProbeEvidence {
    range_start: u64,
    range_end: u64,
    methods_per_block: Vec<String>,
    system_events_storage_key: String,
    runtime_code_storage_key: String,
    probed_block_numbers: Vec<u64>,
    expected_per_endpoint: u64,
    endpoint_a_probe_count: u64,
    endpoint_b_probe_count: u64,
    missing_count: u64,
    mismatch_count: u64,
    completed_before_source_use: bool,
    transcript: Vec<ArchiveProbeRecordEvidence>,
    transcript_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ArchiveProbeRecordEvidence {
    block_number: u64,
    block_hash: String,
    method: String,
    endpoint_a_observed_height: u64,
    endpoint_b_observed_height: u64,
    endpoint_a_block_hash: String,
    endpoint_b_block_hash: String,
    endpoint_a_sha256: String,
    endpoint_b_sha256: String,
    raw_present: bool,
    canonical_genesis_default: bool,
    present: bool,
    equal: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SnapshotEvidence {
    label: String,
    source_endpoint: String,
    read_endpoint: String,
    read_rpc_url: String,
    read_executable: String,
    read_argvs: BTreeMap<String, Vec<String>>,
    read_environments: BTreeMap<String, BTreeMap<String, String>>,
    read_environment_sha256s: BTreeMap<String, String>,
    projection_path: String,
    projection_file_mode: String,
    projection_file_device: String,
    projection_file_inode: String,
    projection_file_size: u64,
    projection_file_sha256: String,
    sqlite_sidecar_absence: BTreeMap<String, String>,
    fresh_database: bool,
    database_deleted_before_build: bool,
    creation_probe_errno: Option<String>,
    replacement: Option<ProjectionReplacementEvidence>,
    full_stream_attested: bool,
    source_eligible_at_open: bool,
    source_gate_sequence: u64,
    checkpoint_number: u64,
    checkpoint_hash: String,
    read_ids: Vec<String>,
    read_responses: BTreeMap<String, Value>,
    read_response_body_hex: BTreeMap<String, String>,
    read_response_body_sizes: BTreeMap<String, u64>,
    read_response_sha256s: BTreeMap<String, String>,
    read_stdout_sizes: BTreeMap<String, u64>,
    read_stdout_sha256s: BTreeMap<String, String>,
    sections: BTreeMap<String, Vec<Value>>,
    semantic_sha256: String,
    attestation_identity_sha256: String,
    exact_association_core_json: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProjectionReplacementEvidence {
    predelete_device: String,
    predelete_inode: String,
    predelete_size: u64,
    predelete_sha256: String,
    retained_descriptor_device_before: String,
    retained_descriptor_inode_before: String,
    retained_descriptor_size_before: u64,
    retained_descriptor_link_count_before: u64,
    retained_descriptor_device_after: String,
    retained_descriptor_inode_after: String,
    retained_descriptor_size_after: u64,
    retained_descriptor_link_count_after: u64,
    retained_descriptor_sha256_after: String,
    deletion_probe_errno: String,
    postcreate_device: String,
    postcreate_inode: String,
    postcreate_size: u64,
    postcreate_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditEvidence {
    loopback_only: bool,
    synthetic_only: bool,
    dev_only: bool,
    dev_signers: Vec<String>,
    synthetic_origins: Vec<String>,
    allowed_hosts: Vec<String>,
    socket_addresses: Vec<String>,
    artifact_root: String,
    artifact_root_device: String,
    artifact_root_inode: String,
    recursive_walk_complete: bool,
    walked_directories: Vec<String>,
    symbolic_link_count: u64,
    hard_link_alias_count: u64,
    non_regular_count: u64,
    artifacts: Vec<AuditArtifactEvidence>,
    submission_lanes: SubmissionLaneAuditEvidence,
    chain_census: FinalizedChainCensusEvidence,
    relay_chain_census: FinalizedRelayChainCensusEvidence,
    node_log_capture: NodeLogCaptureEvidence,
    orchestrator_log: OrchestratorLogEvidence,
    process_inventory: AuditProcessInventoryEvidence,
    listener_inventory: AuditListenerInventoryEvidence,
    scanned_byte_count: u64,
    zero_counts: BTreeMap<String, u64>,
    secret_environment_names_present: Vec<String>,
    public_urls: Vec<String>,
    external_actions: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FinalizedChainCensusEvidence {
    format: String,
    range_start: u64,
    range_end: u64,
    maximum_retained_bytes: u64,
    allowed_unsigned_calls: Vec<String>,
    allowed_events: Vec<String>,
    endpoint_a: EndpointChainCensusEvidence,
    endpoint_b: EndpointChainCensusAttestationEvidence,
    endpoints_equal: bool,
    expected_submission_hashes: Vec<String>,
    expected_signed_extrinsic_count: u64,
    signed_extrinsic_count: u64,
    unsigned_inherent_count: u64,
    accepted_event_count: u64,
    extrinsic_success_count: u64,
    extrinsic_failed_count: u64,
    event_counts: BTreeMap<String, u64>,
    forbidden_counts: BTreeMap<String, u64>,
    external_actions: Vec<String>,
    forbidden_events: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FinalizedRelayChainCensusEvidence {
    format: String,
    range_start: u64,
    range_end: u64,
    checkpoint_hash: String,
    maximum_retained_bytes: u64,
    minimum_session_rotation_count: u64,
    expected_initial_spot_price: u128,
    expected_grandpa_authorities: Vec<(String, u64)>,
    allowed_unsigned_calls: Vec<String>,
    allowed_events: Vec<String>,
    endpoint_a: EndpointChainCensusEvidence,
    endpoint_b: EndpointChainCensusAttestationEvidence,
    endpoints_equal: bool,
    runtime_identity: RelayRuntimeIdentityEvidence,
    signed_extrinsic_count: u64,
    unsigned_inherent_count: u64,
    timestamp_inherent_count: u64,
    para_inherent_count: u64,
    extrinsic_success_count: u64,
    extrinsic_failed_count: u64,
    session_rotation_count: u64,
    historical_root_session_indices: Vec<u64>,
    new_session_indices: Vec<u64>,
    new_queued_count: u64,
    grandpa_new_authorities_count: u64,
    initial_spot_price: u128,
    backed_candidate_count: u64,
    candidate_para_ids: Vec<u64>,
    event_candidate_para_ids: Vec<u64>,
    new_validation_code_field_count: u64,
    new_validation_code_count: u64,
    upward_message_field_count: u64,
    upward_message_count: u64,
    upward_signal_separator_count: u64,
    upward_signal_count: u64,
    horizontal_message_field_count: u64,
    horizontal_message_count: u64,
    processed_downward_message_field_count: u64,
    processed_downward_message_count: u64,
    dispute_field_count: u64,
    dispute_statement_count: u64,
    event_counts: BTreeMap<String, u64>,
    forbidden_counts: BTreeMap<String, u64>,
    external_actions: Vec<String>,
    forbidden_events: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RelayRuntimeIdentityEvidence {
    endpoint_a: RelayRuntimeEndpointIdentityEvidence,
    endpoint_b: RelayRuntimeEndpointIdentityEvidence,
    endpoints_equal: bool,
    unchanged: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RelayRuntimeEndpointIdentityEvidence {
    endpoint: String,
    genesis: RelayRuntimePointIdentityEvidence,
    range_end: RelayRuntimePointIdentityEvidence,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct RelayRuntimePointIdentityEvidence {
    block_hash: String,
    runtime_spec_version: u64,
    runtime_code_sha256: String,
    metadata_sha256: String,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct EndpointChainCensusEvidence {
    endpoint: String,
    range_start: u64,
    range_end: u64,
    retained_bytes: u64,
    transcript_sha256: String,
    blocks: Vec<ChainBlockCensusEvidence>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct EndpointChainCensusAttestationEvidence {
    endpoint: String,
    range_start: u64,
    range_end: u64,
    block_count: u64,
    transcript_sha256: String,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ChainBlockCensusEvidence {
    number: u64,
    hash: String,
    parent_hash: String,
    state_root: String,
    extrinsics_root: String,
    header_scale_hex: String,
    header_sha256: String,
    extrinsics: Vec<ChainExtrinsicCensusEvidence>,
    system_events_scale_hex: String,
    system_events_sha256: String,
    events: Vec<ChainEventCensusEvidence>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ChainExtrinsicCensusEvidence {
    index: u64,
    hash: String,
    signed: bool,
    signer: Option<String>,
    section: String,
    method: String,
    call_index: String,
    args: Vec<Value>,
    args_sha256: String,
    scale_hex: String,
    scale_sha256: String,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ChainEventCensusEvidence {
    index: u64,
    phase: ChainEventPhaseEvidence,
    section: String,
    method: String,
    data: Value,
    data_sha256: String,
    topics: Vec<String>,
    scale_hex: String,
    scale_sha256: String,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ChainEventPhaseEvidence {
    kind: String,
    extrinsic_index: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeLogCaptureEvidence {
    format: String,
    per_file_limit_bytes: u64,
    aggregate_limit_bytes: u64,
    total_bytes: u64,
    discarded_bytes: u64,
    overflowed: bool,
    files: Vec<NodeLogFileEvidence>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeLogFileEvidence {
    file_path: String,
    size: u64,
    device: String,
    inode: String,
    mode: String,
    link_count: u64,
    owner_uid: u32,
    sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuditProcessInventoryEvidence {
    format: String,
    orchestrator: AuditOrchestratorProcessEvidence,
    initial_node_processes: Vec<NodeProcessEvidence>,
    stopped_node_processes: Vec<NodeProcessEvidence>,
    restarted_node_processes: Vec<NodeProcessEvidence>,
    remaining_node_processes: Vec<NodeProcessEvidence>,
    pvf_workers: PvfWorkerEvidence,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuditOrchestratorProcessEvidence {
    pid: u32,
    privileges: ProcessPrivilegeEvidence,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuditListenerInventoryEvidence {
    format: String,
    initial: AuditInitialSocketInventoryEvidence,
    stopped: AuditCapturedSocketInventoryEvidence,
    restarted: AuditCapturedSocketInventoryEvidence,
    released: AuditCapturedSocketInventoryEvidence,
    pvf_unix_sockets: AuditPvfSocketInventoryEvidence,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuditInitialSocketInventoryEvidence {
    command: Vec<String>,
    stdout_sha256: String,
    stdout_utf8: String,
    records: Vec<SocketRecordEvidence>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuditCapturedSocketInventoryEvidence {
    command: Vec<String>,
    stdout_sha256: String,
    stdout_utf8: String,
    records: Vec<AuditDetailedSocketRecordEvidence>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuditDetailedSocketRecordEvidence {
    protocol: String,
    state: String,
    local_address: String,
    pids: Vec<u32>,
    raw_line: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuditPvfSocketInventoryEvidence {
    observed: Vec<PvfUnixSocketEvidence>,
    post_stop: Vec<PvfUnixSocketEvidence>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OrchestratorLogEvidence {
    format: String,
    pid: u32,
    node_roles: Vec<String>,
    diagnostics: OrchestratorDiagnosticsEvidence,
    normalizer_rejection_matrix: NormalizerRejectionSummaryEvidence,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OrchestratorDiagnosticsEvidence {
    format: String,
    limit_bytes: u64,
    total_bytes: u64,
    discarded_bytes: u64,
    overflowed: bool,
    aggregate_sha256: String,
    stdout_sha256: String,
    stderr_sha256: String,
    forbidden_hits: Vec<String>,
    nonloopback_hits: Vec<String>,
    records: Vec<OrchestratorDiagnosticRecordEvidence>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OrchestratorDiagnosticRecordEvidence {
    sequence: u64,
    stream: String,
    byte_length: u64,
    bytes_hex: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NormalizerRejectionSummaryEvidence {
    case_count: u64,
    transcript_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SubmissionLaneAuditEvidence {
    format: String,
    directory: String,
    deployment_id: String,
    locks: Vec<SubmissionLaneLockEvidence>,
    prohibited_residue_names: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SubmissionLaneLockEvidence {
    signer: String,
    account: String,
    name: String,
    path: String,
    owner_uid: u32,
    identity: FileIdentityEvidence,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditArtifactEvidence {
    kind: String,
    logical_path: String,
    path: String,
    read: OpenedFileEvidence,
    owner_only: bool,
    forbidden_hits: Vec<String>,
    nonloopback_hits: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CleanupEvidence {
    node_network_cleanup_complete: bool,
    orchestrator_exit_observation: String,
    terminated_node_processes: Vec<ProcessLifetimeEvidence>,
    listener_addresses_released: Vec<String>,
    work_root_removed: bool,
    remaining_node_processes: Vec<ProcessLifetimeEvidence>,
    remaining_listeners: Vec<String>,
}

#[derive(Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
struct ProcessLifetimeEvidence {
    pid: u32,
    start_time_ticks: u64,
}

struct JourneyRun {
    repo_root: PathBuf,
    supported_root: PathBuf,
    fixture: JourneyFixture,
    evidence: JourneyEvidence,
}

struct BoundedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct DirectorySpan {
    path: PathBuf,
    handle: fs::File,
    before: FileIdentityEvidence,
}

struct BoundedStream {
    label: &'static str,
    stream: UnixStream,
    retained: Vec<u8>,
    limit: usize,
    eof: bool,
    overflowed: bool,
}

struct DuplicateCheckedJson;

impl<'de> Deserialize<'de> for DuplicateCheckedJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DuplicateCheckedJsonVisitor)
    }
}

struct DuplicateCheckedJsonVisitor;

impl<'de> Visitor<'de> for DuplicateCheckedJsonVisitor {
    type Value = DuplicateCheckedJson;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("JSON without duplicate object members")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedJson)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedJson)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedJson)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedJson)
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedJson)
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedJson)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedJson)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedJson)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        DuplicateCheckedJson::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<DuplicateCheckedJson>()?.is_some() {}
        Ok(DuplicateCheckedJson)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut names = BTreeSet::new();
        while let Some(name) = map.next_key::<String>()? {
            if !names.insert(name.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object member `{name}`"
                )));
            }
            map.next_value::<DuplicateCheckedJson>()?;
        }
        Ok(DuplicateCheckedJson)
    }
}

struct SessionDirectory {
    path: Option<PathBuf>,
}

impl SessionDirectory {
    fn create(supported_root: &Path) -> Result<Self, String> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock cannot name the test session: {error}"))?
            .as_nanos();
        for attempt in 0..100_u32 {
            let path = supported_root.join(format!(
                "cubikan-t1115-{}-{nonce}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::set_permissions(&path, fs::Permissions::from_mode(REQUIRED_ROOT_MODE))
                        .map_err(|error| {
                            format!(
                                "cannot protect session directory {}: {error}",
                                path.display()
                            )
                        })?;
                    return Ok(Self { path: Some(path) });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => {
                    return Err(format!(
                        "cannot create session below {}: {error}",
                        supported_root.display()
                    ));
                }
            }
        }
        Err("could not allocate a unique T-1115 session directory".to_owned())
    }

    fn path(&self) -> &Path {
        self.path
            .as_deref()
            .expect("an armed session directory always has a path")
    }

    fn remove(mut self) -> Result<(), String> {
        let path = self
            .path
            .take()
            .expect("an armed session directory always has a path");
        fs::remove_dir(&path)
            .map_err(|error| format!("session directory was not empty after cleanup: {error}"))
    }
}

impl Drop for SessionDirectory {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = fs::remove_dir_all(path);
        }
    }
}

#[test]
fn test_forbidden_diagnostic_marker_has_exact_key_boundaries() {
    for key in PROHIBITED_OUTPUT_KEYS {
        let assignment = format!("{key}=must-not-leak");
        assert_eq!(
            first_forbidden_diagnostic_marker(&assignment),
            Some(key.to_owned()),
            "bare assignment for {key} was not rejected"
        );

        let json = format!(r#"{{"{key}" : "must-not-leak"}}"#);
        assert_eq!(
            first_forbidden_diagnostic_marker(&json),
            Some(key.to_owned()),
            "JSON key for {key} was not rejected"
        );
        let parsed = serde_json::from_str::<Value>(&json)
            .unwrap_or_else(|error| panic!("cannot parse {key} scanner case: {error}"));
        assert_eq!(
            forbidden_json_member(&parsed),
            Some(format!("/{key}")),
            "structured JSON member for {key} was not rejected"
        );
    }

    for safe in [
        "password_hash=public-digest",
        "notpassword=public-value",
        "transcript_sha256=public-digest",
        r#"{"zero_count_keys":["secret"]}"#,
    ] {
        assert_eq!(
            first_forbidden_diagnostic_marker(safe),
            None,
            "safe diagnostic was rejected: {safe}"
        );
    }
    assert_eq!(
        forbidden_json_member(&serde_json::json!({"zero_count_keys": ["secret"]})),
        None,
        "the semantic zero-count proof was mistaken for a secret member"
    );

    for name in [
        "AWS_SECRET_ACCESS_KEY",
        "GIT_ASKPASS",
        "GIT_CONFIG_GLOBAL",
        "GIT_CONFIG_SYSTEM",
        "SSH_AUTH_SOCK",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
    ] {
        let assignment = format!("{name}=must-not-leak");
        assert_eq!(
            first_forbidden_diagnostic_marker(&assignment),
            Some(name.to_owned()),
            "hostile environment assignment {name} was not rejected"
        );
        assert_eq!(
            first_forbidden_diagnostic_marker(&format!("SAFE_{assignment}")),
            None,
            "environment-name substring {name} was not boundary-safe"
        );
    }
}

#[test]
fn test_protocol_operations_map_to_exact_runtime_dispatch_methods() {
    let expected = [
        ("create_intent_unit", "createUnit"),
        ("transition_intent_unit", "transitionUnit"),
        ("complete_intent_unit", "completeUnit"),
        (
            "create_relationship_definition",
            "createRelationshipDefinition",
        ),
        ("create_relationship", "createRelationship"),
        ("delete_relationship", "deleteRelationship"),
        ("record_association", "recordAssociation"),
        ("revoke_association", "revokeAssociation"),
    ];
    for (operation, method) in expected {
        assert_eq!(exact_dispatch_method(operation), Ok(method));
    }
    assert!(exact_dispatch_method("replace_authorized_submitters").is_err());
    assert!(exact_dispatch_method("unknown").is_err());
}

#[test]
#[ignore = "requires the pinned four-node local-chain gate"]
fn test_four_node_topology_ports_archive_flags_and_cleanup_are_exact() {
    let journey = journey();
    assert_e1(journey).unwrap_or_else(|error| panic!("T-1115-E1 failed: {error}"));
}

#[test]
#[ignore = "requires the pinned four-node local-chain gate"]
fn test_both_submitters_and_collator_endpoints_converge() {
    let journey = journey();
    assert_e2(journey).unwrap_or_else(|error| panic!("T-1115-E2 failed: {error}"));
}

#[test]
#[ignore = "requires the pinned four-node local-chain gate"]
fn test_restarted_collator_catches_up_and_passes_archive_probes_before_use() {
    let journey = journey();
    assert_e3(journey).unwrap_or_else(|error| panic!("T-1115-E3 failed: {error}"));
}

#[test]
#[ignore = "requires the pinned four-node local-chain gate"]
fn test_dual_archive_rebuilds_equal_uninterrupted_projection() {
    let journey = journey();
    assert_e4(journey).unwrap_or_else(|error| panic!("T-1115-E4 failed: {error}"));
}

#[test]
#[ignore = "requires the pinned four-node local-chain gate"]
fn test_local_journey_has_no_public_or_secret_action() {
    let journey = journey();
    assert_e5(journey).unwrap_or_else(|error| panic!("T-1115-E5 failed: {error}"));
}

fn journey() -> &'static JourneyRun {
    match JOURNEY.get_or_init(run_journey) {
        Ok(journey) => journey,
        Err(error) => panic!(
            "the explicit T-1115 gate cannot skip or degrade: {error}; ignored reason is `{IGNORED_REASON}`"
        ),
    }
}

fn run_journey() -> Result<JourneyRun, String> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|error| format!("cannot resolve repository root: {error}"))?;
    let runtime_fixture_path = repo_root.join("tests/chain-e2e/journey-v1.json");
    let runtime_fixture_metadata = fs::symlink_metadata(&runtime_fixture_path)
        .map_err(|error| format!("cannot stat runtime journey fixture: {error}"))?;
    require!(
        runtime_fixture_metadata.file_type().is_file()
            && !runtime_fixture_metadata.file_type().is_symlink()
            && fs::read(&runtime_fixture_path)
                .map_err(|error| format!("cannot read runtime journey fixture: {error}"))?
                == FIXTURE_BYTES,
        "runtime journey fixture bytes differ from the independently compiled include_bytes oracle"
    );
    validate_unique_json(FIXTURE_BYTES, "journey fixture")?;
    let fixture: JourneyFixture = serde_json::from_slice(FIXTURE_BYTES)
        .map_err(|error| format!("independent journey fixture is invalid: {error}"))?;
    validate_fixture(&fixture)?;
    let prelaunch_pins = verify_pinned_inputs(&repo_root, &fixture.pinned_inputs)?;
    verify_exact_t1114_association(&fixture.exact_recorded_association)?;
    let local_binary = PathBuf::from(env!("CARGO_BIN_EXE_cubikan-local"))
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize cubikan-local candidate: {error}"))?;
    let prelaunch_local_binary =
        read_opened_regular_file(&local_binary, "cubikan-local candidate")?;
    let input_directory_spans =
        retain_input_parent_directories(&repo_root, &fixture.pinned_inputs, &local_binary)?;

    let supported_root = validate_supported_root()?;
    let session = SessionDirectory::create(&supported_root)?;
    let work_root = session.path().join("work");
    let evidence_path = session.path().join("evidence-v1.json");
    let launcher = repo_root.join(&fixture.launcher.path);
    let launcher_metadata = fs::symlink_metadata(&launcher).map_err(|error| {
        format!(
            "required T-1115 launcher {} is unavailable: {error}",
            launcher.display()
        )
    })?;
    require!(
        launcher_metadata.file_type().is_file() && !launcher_metadata.file_type().is_symlink(),
        "T-1115 launcher must be a non-symbolic regular file"
    );
    require!(
        launcher_metadata.permissions().mode() & 0o111 != 0,
        "T-1115 launcher must be executable"
    );

    let timeout = format!("{}s", fixture.launcher.timeout_seconds);
    let kill_after = format!("{}s", fixture.launcher.kill_after_seconds);
    let mut command = Command::new(SETSID);
    command
        .arg(TIMEOUT)
        .arg("--signal=TERM")
        .arg(format!("--kill-after={kill_after}"))
        .arg(timeout)
        .arg(&launcher)
        .arg("--fixture")
        .arg(repo_root.join("tests/chain-e2e/journey-v1.json"))
        .arg("--evidence")
        .arg(&evidence_path)
        .arg("--work-root")
        .arg(&work_root)
        .current_dir(&repo_root)
        .env_clear()
        .env("HOME", "/home/charles")
        .env("CARGO_HOME", repo_root.join("chain/.cache/cargo-home"))
        .env("RUSTUP_HOME", "/home/charles/.rustup")
        .env("PATH", "/home/charles/.cargo/bin:/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .env("TMPDIR", &supported_root)
        .env(SUPPORTED_ROOT_ENV, &supported_root)
        .env("CUBIKAN_LOCAL_TEST_BINARY", &local_binary)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let outer_bound = Duration::from_secs(
        fixture
            .launcher
            .timeout_seconds
            .checked_add(fixture.launcher.kill_after_seconds)
            .and_then(|seconds| seconds.checked_add(10))
            .ok_or_else(|| "launcher timeout bound overflowed".to_owned())?,
    );
    let output = bounded_command_output(
        command,
        outer_bound,
        MAX_LAUNCHER_STDOUT_BYTES,
        MAX_LAUNCHER_STDERR_BYTES,
    )?;
    require!(
        output.status.success(),
        "bounded launcher exited {:?}; stdout={:?}; stderr={:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    require!(
        output.stdout == fixture.launcher.success_stdout.as_bytes(),
        "launcher stdout was not the one pinned success line: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    require!(
        output.stderr.is_empty(),
        "launcher stderr must be empty, got {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    require!(
        !work_root.exists(),
        "launcher left its test-owned work root behind: {}",
        work_root.display()
    );

    let evidence_metadata = fs::symlink_metadata(&evidence_path)
        .map_err(|error| format!("launcher did not produce evidence: {error}"))?;
    require!(
        evidence_metadata.file_type().is_file() && !evidence_metadata.file_type().is_symlink(),
        "evidence must be a non-symbolic regular file"
    );
    require!(
        evidence_metadata.permissions().mode() & 0o777 == REQUIRED_FILE_MODE,
        "evidence mode must be 0600"
    );
    require!(
        evidence_metadata.len() > 0
            && evidence_metadata.len() <= fixture.launcher.max_evidence_bytes,
        "evidence size {} is outside 1..={} bytes",
        evidence_metadata.len(),
        fixture.launcher.max_evidence_bytes
    );
    let evidence_bytes = fs::read(&evidence_path)
        .map_err(|error| format!("cannot read launcher evidence: {error}"))?;
    validate_unique_json(&evidence_bytes, "launcher evidence")?;
    let evidence_value: Value = serde_json::from_slice(&evidence_bytes)
        .map_err(|error| format!("cannot parse launcher evidence value: {error}"))?;
    let mut canonical_evidence = serde_json::to_vec(&evidence_value)
        .map_err(|error| format!("cannot canonicalize launcher evidence: {error}"))?;
    canonical_evidence.push(b'\n');
    require!(
        evidence_bytes == canonical_evidence,
        "launcher evidence must be exact compact key-sorted JSON plus one LF"
    );
    let evidence: JourneyEvidence = serde_json::from_slice(&evidence_bytes)
        .map_err(|error| format!("launcher evidence violates its closed schema: {error}"))?;
    require!(
        evidence.format == fixture.launcher.evidence_format && evidence.version == 1,
        "launcher evidence format/version mismatch"
    );
    verify_pinned_input_continuity(
        &repo_root,
        &fixture.pinned_inputs,
        &prelaunch_pins,
        &evidence.pins,
    )?;
    verify_runtime_executable_continuity(
        &fixture,
        &evidence,
        &local_binary,
        &prelaunch_local_binary,
    )?;
    verify_input_parent_directories(&input_directory_spans)?;

    fs::remove_file(&evidence_path)
        .map_err(|error| format!("cannot remove consumed evidence: {error}"))?;
    session.remove()?;

    Ok(JourneyRun {
        repo_root,
        supported_root,
        fixture,
        evidence,
    })
}

fn bounded_command_output(
    mut command: Command,
    outer_bound: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
) -> Result<BoundedOutput, String> {
    let (stdout_reader, stdout_writer) = UnixStream::pair()
        .map_err(|error| format!("cannot create bounded launcher stdout socket: {error}"))?;
    let (stderr_reader, stderr_writer) = UnixStream::pair()
        .map_err(|error| format!("cannot create bounded launcher stderr socket: {error}"))?;
    stdout_reader
        .set_nonblocking(true)
        .map_err(|error| format!("cannot make launcher stdout nonblocking: {error}"))?;
    stderr_reader
        .set_nonblocking(true)
        .map_err(|error| format!("cannot make launcher stderr nonblocking: {error}"))?;
    let stdout_writer: OwnedFd = stdout_writer.into();
    let stderr_writer: OwnedFd = stderr_writer.into();
    command
        .stdout(Stdio::from(stdout_writer))
        .stderr(Stdio::from(stderr_writer));
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot execute bounded T-1115 launcher: {error}"))?;
    drop(command);
    let mut stdout = BoundedStream::new("stdout", stdout_reader, stdout_limit);
    let mut stderr = BoundedStream::new("stderr", stderr_reader, stderr_limit);
    let pid = child.id();
    let process = match wait_for_owned_process_group(pid) {
        Ok(process) => process,
        Err(error) => {
            let cleanup = terminate_unowned_child(&mut child, &mut stdout, &mut stderr);
            return Err(match cleanup {
                Ok(()) => error,
                Err(cleanup_error) => format!("{error}; cleanup also failed: {cleanup_error}"),
            });
        }
    };
    let lifetime = ProcessLifetimeEvidence {
        pid,
        start_time_ticks: process.start_time_ticks,
    };

    let started = Instant::now();
    let mut status = None;
    let mut drain_deadline = None;
    let failure = loop {
        if let Err(error) = stdout.drain() {
            break Some(error);
        }
        if let Err(error) = stderr.drain() {
            break Some(error);
        }
        if stdout.overflowed || stderr.overflowed {
            let label = if stdout.overflowed {
                "stdout"
            } else {
                "stderr"
            };
            break Some(format!("launcher {label} exceeded its retained byte bound"));
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(observed) => {
                    status = observed;
                    if status.is_some() {
                        drain_deadline = Some(Instant::now() + Duration::from_secs(5));
                    }
                }
                Err(error) => break Some(format!("cannot poll bounded launcher: {error}")),
            }
        }
        if status.is_some() && stdout.eof && stderr.eof {
            match owned_process_group_has_members(&lifetime) {
                Ok(false) => break None,
                Ok(true) => {
                    break Some(
                        "launcher exited while its owned process group still had members"
                            .to_owned(),
                    );
                }
                Err(error) => break Some(error),
            }
        }
        if started.elapsed() >= outer_bound {
            break Some("launcher exceeded the independent outer deadline".to_owned());
        }
        if drain_deadline
            .is_some_and(|deadline| Instant::now() >= deadline && (!stdout.eof || !stderr.eof))
        {
            break Some(
                "launcher descendant retained an output socket after process exit".to_owned(),
            );
        }
        thread::sleep(Duration::from_millis(10));
    };

    if let Some(failure) = failure {
        let cleanup = terminate_owned_child(&mut child, &lifetime, &mut stdout, &mut stderr);
        let mut message = format!(
            "{failure}; stdout-prefix={:?}; stderr-prefix={:?}",
            String::from_utf8_lossy(&stdout.retained),
            String::from_utf8_lossy(&stderr.retained)
        );
        if let Err(cleanup_error) = cleanup {
            message.push_str(&format!("; cleanup also failed: {cleanup_error}"));
        }
        return Err(message);
    }

    Ok(BoundedOutput {
        status: status.expect("successful loop exit requires a reaped launcher"),
        stdout: stdout.retained,
        stderr: stderr.retained,
    })
}

impl BoundedStream {
    fn new(label: &'static str, stream: UnixStream, limit: usize) -> Self {
        Self {
            label,
            stream,
            retained: Vec::with_capacity(limit.min(8192)),
            limit,
            eof: false,
            overflowed: false,
        }
    }

    fn drain(&mut self) -> Result<(), String> {
        let mut buffer = [0_u8; 8192];
        loop {
            match self.stream.read(&mut buffer) {
                Ok(0) => {
                    self.eof = true;
                    return Ok(());
                }
                Ok(count) => {
                    let remaining = self.limit.saturating_sub(self.retained.len());
                    self.retained
                        .extend_from_slice(&buffer[..count.min(remaining)]);
                    self.overflowed |= count > remaining;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    return Err(format!(
                        "cannot read bounded launcher {}: {error}",
                        self.label
                    ));
                }
            }
        }
    }

    fn close(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Read);
        self.eof = true;
    }
}

fn terminate_unowned_child(
    child: &mut Child,
    stdout: &mut BoundedStream,
    stderr: &mut BoundedStream,
) -> Result<(), String> {
    if child
        .try_wait()
        .map_err(|error| format!("cannot poll unowned launcher during cleanup: {error}"))?
        .is_none()
    {
        child
            .kill()
            .map_err(|error| format!("cannot kill unowned launcher during cleanup: {error}"))?;
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let _ = stdout.drain();
        let _ = stderr.drain();
        if child
            .try_wait()
            .map_err(|error| format!("cannot reap unowned launcher: {error}"))?
            .is_some()
        {
            stdout.close();
            stderr.close();
            return Ok(());
        }
        require!(
            Instant::now() < deadline,
            "unowned launcher did not exit after SIGKILL"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn terminate_owned_child(
    child: &mut Child,
    process: &ProcessLifetimeEvidence,
    stdout: &mut BoundedStream,
    stderr: &mut BoundedStream,
) -> Result<(), String> {
    signal_owned_process_group(process, "-TERM")?;
    let term_deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < term_deadline {
        let _ = stdout.drain();
        let _ = stderr.drain();
        let child_done = child
            .try_wait()
            .map_err(|error| format!("cannot poll launcher during TERM cleanup: {error}"))?
            .is_some();
        if child_done && !owned_process_group_has_members(process)? {
            stdout.close();
            stderr.close();
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    }
    signal_owned_process_group(process, "-KILL")?;
    let kill_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let _ = stdout.drain();
        let _ = stderr.drain();
        let child_done = child
            .try_wait()
            .map_err(|error| format!("cannot reap launcher after SIGKILL: {error}"))?
            .is_some();
        let group_done = !owned_process_group_has_members(process)?;
        if child_done && group_done {
            stdout.close();
            stderr.close();
            return Ok(());
        }
        require!(
            Instant::now() < kill_deadline,
            "launcher process group did not become empty after SIGKILL"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn signal_owned_process_group(
    process: &ProcessLifetimeEvidence,
    signal: &str,
) -> Result<(), String> {
    if !owned_process_group_has_members(process)? {
        return Ok(());
    }
    require!(
        process.pid > 1,
        "refusing to signal an unsafe launcher process-group identifier"
    );
    let status = Command::new(KILL)
        .arg(signal)
        .arg("--")
        .arg(format!("-{}", process.pid))
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("cannot signal launcher process group: {error}"))?;
    if !status.success() && owned_process_group_has_members(process)? {
        return Err(format!(
            "cannot deliver {signal} to launcher process group {}",
            process.pid
        ));
    }
    Ok(())
}

struct ProcStat {
    process_group: u32,
    session_id: u32,
    start_time_ticks: u64,
}

fn wait_for_owned_process_group(pid: u32) -> Result<ProcStat, String> {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let stat = read_proc_stat(pid)?;
        if stat.process_group == pid && stat.session_id == pid {
            return Ok(stat);
        }
        require!(
            Instant::now() < deadline,
            "setsid did not create a launcher-owned process group"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn owned_process_group_has_members(process: &ProcessLifetimeEvidence) -> Result<bool, String> {
    for entry in fs::read_dir("/proc")
        .map_err(|error| format!("cannot enumerate /proc for launcher cleanup: {error}"))?
    {
        let entry = entry.map_err(|error| format!("cannot read /proc entry: {error}"))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        let stat = match read_proc_stat(pid) {
            Ok(stat) => stat,
            Err(_) if !entry.path().exists() => continue,
            Err(error) => return Err(error),
        };
        if stat.process_group == process.pid && stat.session_id == process.pid {
            return Ok(true);
        }
    }
    Ok(false)
}

fn read_proc_stat(pid: u32) -> Result<ProcStat, String> {
    let path = PathBuf::from(format!("/proc/{pid}/stat"));
    let bytes =
        fs::read(&path).map_err(|error| format!("cannot read process stat for {pid}: {error}"))?;
    require!(
        bytes.len() <= 4096 && !bytes.is_empty() && !bytes.contains(&0),
        "process stat for {pid} is malformed or over bound"
    );
    let open_parenthesis = bytes
        .windows(2)
        .position(|pair| pair == b" (")
        .ok_or_else(|| format!("process stat for {pid} lacks command opening"))?;
    require!(
        bytes[..open_parenthesis] == *pid.to_string().as_bytes(),
        "process stat PID mismatch for {pid}"
    );
    let close_parenthesis = bytes
        .iter()
        .rposition(|byte| *byte == b')')
        .ok_or_else(|| format!("process stat for {pid} lacks command closing"))?;
    require!(
        close_parenthesis > open_parenthesis + 1 && bytes.get(close_parenthesis + 1) == Some(&b' '),
        "process stat command framing is malformed for {pid}"
    );
    let fields = bytes[close_parenthesis + 2..]
        .split(u8::is_ascii_whitespace)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    require!(
        fields.len() > 19,
        "process stat lacks process-group/start-time fields for {pid}"
    );
    let parse = |field: &[u8], name: &str| -> Result<u64, String> {
        std::str::from_utf8(field)
            .ok()
            .and_then(|text| text.parse::<u64>().ok())
            .ok_or_else(|| format!("process stat {name} is invalid for {pid}"))
    };
    let process_group = u32::try_from(parse(fields[2], "process group")?)
        .map_err(|error| format!("process group overflows u32 for {pid}: {error}"))?;
    let session_id = u32::try_from(parse(fields[3], "session")?)
        .map_err(|error| format!("session ID overflows u32 for {pid}: {error}"))?;
    let start_time_ticks = parse(fields[19], "start time")?;
    Ok(ProcStat {
        process_group,
        session_id,
        start_time_ticks,
    })
}

fn validate_fixture(fixture: &JourneyFixture) -> Result<(), String> {
    require!(
        fixture.format == "cubikan-chain-e2e-journey-v1" && fixture.version == 1,
        "journey fixture format/version drifted"
    );
    require!(
        fixture.launcher.path == "chain/tools/run-four-node-journey.sh"
            && fixture.launcher.success_stdout == "verified cubikan four-node journey v1\n"
            && fixture.launcher.evidence_format == "cubikan-chain-e2e-evidence-v1"
            && fixture.launcher.timeout_seconds == 1740
            && fixture.launcher.kill_after_seconds == 20
            && fixture.launcher.max_evidence_bytes == 16_777_216
            && fixture.launcher.bootstrap_log_max_bytes == 1_048_576
            && fixture.launcher.genesis_head_decoded_size == 98
            && fixture.launcher.genesis_head_decoded_sha256
                == "a5712144e8320d7ede466cbb02f05c62b8bc5167312b50358f5ce601d5790113"
            && fixture.launcher.materialized_toolchain_root == "/run/cubikan-exec/toolchain"
            && fixture.launcher.materialized_toolchain_mode == "0700"
            && fixture.launcher.materialized_toolchain_filesystem_magic == "1021994"
            && fixture.launcher.materialized_toolchain_write_probe_errno == "EROFS"
            && fixture.launcher.materialized_toolchain_mount_flags
                == ["bind", "ro", "nodev", "nosuid", "exec"]
            && fixture.launcher.materialized_node_path
                == "/run/cubikan-exec/toolchain/node/bin/node"
            && fixture.launcher.node_executable_size == 124_835_376
            && fixture.launcher.node_executable_sha256
                == "93956de2e59480474a7b46571da1651180b1a050cdf32641ebec4ce6e478e068",
        "launcher contract drifted"
    );

    let expected_pin_paths = [
        "/usr/lib/cargo/bin/coreutils/sha256sum",
        "/usr/lib/cargo/bin/coreutils/stat",
        "/usr/lib/cargo/bin/coreutils/timeout",
        "/usr/lib/cargo/bin/coreutils/env",
        "/usr/lib/cargo/bin/coreutils/dd",
        "/usr/lib/cargo/bin/coreutils/chmod",
        "/usr/lib/cargo/bin/coreutils/dirname",
        "/usr/lib/cargo/bin/coreutils/mkdir",
        "/usr/lib/cargo/bin/coreutils/sort",
        "/usr/lib/cargo/bin/coreutils/uname",
        "/home/charles/.rustup/toolchains/1.93.0-x86_64-unknown-linux-gnu/bin/cargo",
        "/usr/bin/bash",
        "/usr/bin/cmp",
        "/usr/lib/cargo/bin/coreutils/false",
        "/usr/bin/find",
        "/usr/bin/gawk",
        "/usr/bin/git",
        "/usr/bin/grep",
        "/usr/bin/iconv",
        "/usr/bin/ip",
        "/usr/lib/cargo/bin/coreutils/mkfifo",
        "/usr/lib/cargo/bin/coreutils/mktemp",
        "/usr/bin/mount",
        "/usr/bin/gnumv",
        "/usr/bin/nc.openbsd",
        "/usr/bin/python3.14",
        "/usr/lib/cargo/bin/coreutils/readlink",
        "/usr/lib/cargo/bin/coreutils/realpath",
        "/usr/bin/gnurm",
        "/usr/lib/cargo/bin/coreutils/rmdir",
        "/usr/lib/cargo/bin/coreutils/sleep",
        "/usr/bin/ss",
        "/usr/bin/umount",
        "/usr/bin/unshare",
        "/usr/lib/cargo/bin/coreutils/wc",
        "/usr/bin/setpriv",
        "/usr/bin/setsid",
        "/usr/bin/kill",
        "/usr/bin/ps",
        "chain/pins.toml",
        "chain/tools/verify-pins.sh",
        "chain/tools/loopback-netns.sh",
        "chain/tools/loopback-netns.test.sh",
        "chain/tools/sealed-exec.py",
        "chain/tools/node-argv-grammar-v1.txt",
        "chain/tools/normalize-node-argv.sh",
        "chain/tools/normalize-node-argv.test.sh",
        "chain/config/zombienet.toml",
        "chain/tools/materialize-zombienet.sh",
        "chain/tools/zombienet-node-launcher.sh",
        "chain/tools/zombienet-t1115.test.sh",
        "chain/tools/run-four-node-journey.sh",
        "chain/tools/run-zombienet-e2e.sh",
        "tests/chain-e2e/driver.mjs",
        "chain/config/cubikan-local.json",
        "chain/artifacts/local-deployment-anchor-v1.json",
        "chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm",
        "chain/.cache/downloads/zombienet-a7c434271f094320d17cf94f7a2f95fdef417379.tar.gz",
        "chain/.cache/downloads/node-v22.23.1-linux-x64.tar.xz",
        "chain/.cache/downloads/polkadot-sdk-8ae9775dc43c0d8cdd0f6d87700596e14278b1e1.tar.gz",
        "chain/.cache/downloads/polkadot",
        "chain/.cache/downloads/polkadot-parachain",
        "chain/.cache/downloads/polkadot-omni-node",
        "chain/.cache/downloads/polkadot-prepare-worker",
        "chain/.cache/downloads/polkadot-execute-worker",
        "tests/fixtures/git/recorded-association-v1.json",
    ];
    require!(
        fixture
            .pinned_inputs
            .iter()
            .map(|pin| pin.path.as_str())
            .eq(expected_pin_paths),
        "pinned input inventory drifted"
    );
    require!(
        fixture
            .pinned_inputs
            .iter()
            .all(|pin| pin.size > 0 && is_sha256(&pin.sha256)),
        "pinned inputs must contain nonzero size and lowercase SHA-256"
    );

    let expected_endpoints = BTreeMap::from([
        ("collator-a".to_owned(), "ws://127.0.0.1:9988/".to_owned()),
        ("collator-b".to_owned(), "ws://127.0.0.1:9989/".to_owned()),
    ]);
    require!(
        fixture.endpoints == expected_endpoints,
        "collator endpoint fixture drifted"
    );
    require!(
        fixture.topology.orchestrator_count == 1 && fixture.topology.node_count == 4,
        "fixture must describe one orchestrator and exactly four nodes"
    );
    let expected_nodes = [
        (
            "relay-a",
            "relay-validator",
            "chain/.cache/downloads/polkadot",
            ["127.0.0.1:9944", "127.0.0.1:30333", "127.0.0.1:9615"].as_slice(),
            false,
            false,
        ),
        (
            "relay-b",
            "relay-validator",
            "chain/.cache/downloads/polkadot",
            ["127.0.0.1:9945", "127.0.0.1:30334", "127.0.0.1:9616"].as_slice(),
            false,
            false,
        ),
        (
            "collator-a",
            "parachain-collator",
            "chain/.cache/downloads/polkadot-omni-node",
            [
                "127.0.0.1:9988",
                "127.0.0.1:30335",
                "127.0.0.1:9617",
                "127.0.0.1:9990",
                "127.0.0.1:30337",
                "127.0.0.1:9619",
            ]
            .as_slice(),
            true,
            true,
        ),
        (
            "collator-b",
            "parachain-collator",
            "chain/.cache/downloads/polkadot-omni-node",
            [
                "127.0.0.1:9989",
                "127.0.0.1:30336",
                "127.0.0.1:9618",
                "127.0.0.1:9991",
                "127.0.0.1:30338",
                "127.0.0.1:9620",
            ]
            .as_slice(),
            true,
            true,
        ),
    ];
    require!(
        fixture.topology.nodes.len() == expected_nodes.len(),
        "fixture node inventory length drifted"
    );
    for (actual, expected) in fixture.topology.nodes.iter().zip(expected_nodes) {
        require!(
            actual.role == expected.0
                && actual.kind == expected.1
                && actual.binary == expected.2
                && actual
                    .listeners
                    .iter()
                    .map(String::as_str)
                    .eq(expected.3.iter().copied())
                && actual.archive == expected.4
                && actual.relay_side == expected.5,
            "fixture node contract drifted for {}",
            actual.role
        );
    }
    let expected_node_identities = [
        (
            "relay-a",
            "alice",
            "Alice",
            "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90",
            "12D3KooWQCkBm1BYtkHpocxCwMgR8yjitEeHGx8spzcDLGt2gkBm",
            "/ip4/127.0.0.1/tcp/30334/ws/p2p/12D3KooWRkZhiRhsqmrQ28rt73K7V3aCBpqKrLGSXmZ99PTcTZby",
            None,
            None::<&str>,
            None::<&str>,
        ),
        (
            "relay-b",
            "bob",
            "Bob",
            "81b637d8fcd2c6da6359e6963113a1170de795e4b725b84d1e0b4cfd9ec58ce9",
            "12D3KooWRkZhiRhsqmrQ28rt73K7V3aCBpqKrLGSXmZ99PTcTZby",
            "/ip4/127.0.0.1/tcp/30333/ws/p2p/12D3KooWQCkBm1BYtkHpocxCwMgR8yjitEeHGx8spzcDLGt2gkBm",
            None,
            None,
            None,
        ),
        (
            "collator-a",
            "alice-1",
            "Alice",
            "a42ac5108869b599bcbac21069f63fb47f07452fcc4b87e89b3c06a945612d0b",
            "12D3KooWHhaSXEhWFi3LibWRNgF9PezoqB9Xeae4fS3dxCowJEg3",
            "/ip4/127.0.0.1/tcp/30336/ws/p2p/12D3KooWDV1yAeEGiye3t2CQpW7MJ5TJV3TTKpTUUxv4xtaggSnA",
            Some(
                "/ip4/127.0.0.1/tcp/30333/ws/p2p/12D3KooWQCkBm1BYtkHpocxCwMgR8yjitEeHGx8spzcDLGt2gkBm",
            ),
            None,
            None,
        ),
        (
            "collator-b",
            "bob-1",
            "Bob",
            "a5fc3eac9107fe9b449965916d54b334233b8077a37f782d9b59e6e98e04def8",
            "12D3KooWDV1yAeEGiye3t2CQpW7MJ5TJV3TTKpTUUxv4xtaggSnA",
            "/ip4/127.0.0.1/tcp/30335/ws/p2p/12D3KooWHhaSXEhWFi3LibWRNgF9PezoqB9Xeae4fS3dxCowJEg3",
            Some(
                "/ip4/127.0.0.1/tcp/30333/ws/p2p/12D3KooWQCkBm1BYtkHpocxCwMgR8yjitEeHGx8spzcDLGt2gkBm",
            ),
            None,
            None,
        ),
    ];
    for (node, expected) in fixture.topology.nodes.iter().zip(expected_node_identities) {
        require!(
            node.role == expected.0
                && node.generated_name == expected.1
                && node.dev_seed == expected.2
                && node.node_key == expected.3
                && node.peer_id == expected.4
                && node.primary_bootnodes == [expected.5]
                && node.relay_side_bootnodes
                    == expected
                        .6
                        .map_or_else(Vec::new, |bootnode| vec![bootnode.to_owned()])
                && node.primary_spec_bootnodes
                    == expected
                        .7
                        .map_or_else(Vec::new, |bootnode| vec![bootnode.to_owned()])
                && node.relay_side_spec_bootnodes
                    == expected
                        .8
                        .map_or_else(Vec::new, |bootnode| vec![bootnode.to_owned()]),
            "node key/peer/bootnode identity drifted for {}",
            node.role
        );
    }
    require!(
        fixture.topology.runtime_equal_groups
            == [
                [
                    "relay-a.primary",
                    "relay-b.primary",
                    "collator-a.relay-side",
                    "collator-b.relay-side",
                ]
                .map(str::to_owned)
                .to_vec(),
                ["collator-a.primary", "collator-b.primary"]
                    .map(str::to_owned)
                    .to_vec(),
            ]
            && fixture.topology.runtime_distinct_groups
                == [["relay-a.primary", "collator-a.primary"]
                    .map(str::to_owned)
                    .to_vec()],
        "runtime equality/inequality fixture drifted"
    );

    let expected_rejection_cases = [
        "grammar-hash-mismatch",
        "unexpected-positional",
        "unknown-flag-primary",
        "unknown-flag-relay-side",
        "duplicate-value-flag-primary",
        "duplicate-value-flag-relay-side",
        "duplicate-switch-primary",
        "duplicate-switch-relay-side",
        "both-rpc-spellings-primary",
        "both-rpc-spellings-relay-side",
        "generated-structured-rpc-endpoint",
        "missing-value-primary",
        "missing-value-relay-side",
        "missing-primary-rpc-cors",
        "missing-primary-rpc-methods",
        "wrong-primary-rpc-methods",
        "primary-rpc-policy-on-relay-side",
        "missing-required-primary",
        "missing-required-relay-side",
        "nonloopback-bootnode-primary",
        "nonloopback-bootnode-relay-side",
        "relay-separator-on-validator",
        "missing-relay-separator-on-collator",
        "missing-blocks-pruning-archive",
        "missing-state-pruning-archive",
        "missing-worker-path",
        "wrong-worker-path",
        "missing-execute-worker-cap",
        "wrong-soft-worker-cap",
        "duplicate-hard-worker-cap",
        "missing-relay-side-workers",
        "wrong-relay-side-worker-cap",
        "worker-path-on-collator-primary",
        "unsafe-chain-path",
        "unsafe-base-path",
        "substituted-node-path",
        "nonregular-node-asset",
    ];
    require!(
        fixture
            .topology
            .normalizer_rejection_cases
            .iter()
            .map(String::as_str)
            .eq(expected_rejection_cases),
        "normalizer rejection matrix drifted"
    );

    let pvf = &fixture.pvf_workers;
    require!(
        pvf.root == "/run/cubikan-exec/pvf-workers"
            && pvf.filesystem_magic == "1021994"
            && pvf.directory_mode == "0500"
            && pvf.write_probe_errno == "EROFS"
            && pvf.mount_flags == ["bind", "ro", "nodev", "nosuid", "exec"].map(str::to_owned)
            && pvf.workers_path_flag == "--workers-path"
            && pvf.pool_caps
                == BTreeMap::from([
                    ("--execute-workers-max-num".to_owned(), "1".to_owned()),
                    ("--prepare-workers-hard-max-num".to_owned(), "1".to_owned()),
                    ("--prepare-workers-soft-max-num".to_owned(), "1".to_owned()),
                ])
            && pvf.execution_sides
                == [
                    "relay-a.primary",
                    "relay-b.primary",
                    "collator-a.relay-side",
                    "collator-b.relay-side",
                ]
                .map(str::to_owned)
            && pvf.host_path_prefix == "/tmp/pvf-host-"
            && pvf.node_impl_version == "1.24.1"
            && pvf.database_backend == "rocksdb"
            && pvf.database_path_components == ["db", "full"].map(str::to_owned)
            && pvf.nested_namespace_capability_mask == "000001ffffffffff"
            && pvf.maximum_supervisors_per_role_kind == 1
            && pvf.maximum_job_children_per_supervisor == 1
            && pvf.maximum_total_worker_processes == 16
            && pvf.monitor_interval_milliseconds == 100
            && pvf.maximum_observed_generations == 64
            && pvf.maximum_observed_host_paths == 256
            && pvf.maximum_observed_unix_sockets == 256,
        "PVF worker root/mount/pool/monitor fixture drifted"
    );
    let expected_pvf_assets = [
        (
            "polkadot-execute-worker",
            "chain/.cache/downloads/polkadot-execute-worker",
            "/run/cubikan-exec/pvf-workers/polkadot-execute-worker",
            19_463_336,
            "cc642041ef2582d972071cd4f7122e9803703bc7775e8d432b2d7626f5011b21",
        ),
        (
            "polkadot-prepare-worker",
            "chain/.cache/downloads/polkadot-prepare-worker",
            "/run/cubikan-exec/pvf-workers/polkadot-prepare-worker",
            21_387_400,
            "5e67a05516e24d5e9b9616bacb3a2d58235beb3392de14dfbe51ff6914244267",
        ),
    ];
    require!(
        pvf.assets.len() == expected_pvf_assets.len()
            && pvf
                .assets
                .iter()
                .zip(expected_pvf_assets)
                .all(|(asset, expected)| asset.name == expected.0
                    && asset.cache_path == expected.1
                    && asset.materialized_path == expected.2
                    && asset.size == expected.3
                    && asset.sha256 == expected.4
                    && asset.mode == "0500"),
        "PVF worker asset fixture drifted"
    );

    let readiness = &fixture.checkpoints.pre_mutation_readiness;
    require!(
        readiness.para_id == 1000
            && readiness.minimum_finalized_number == 1
            && readiness.required_progress_blocks == 1
            && readiness.expected_scheduler_cores == 1
            && readiness.expected_distinct_aura_authorities == 2
            && readiness.maximum_best_finalized_gap == 8
            && readiness.timeout_milliseconds == 180_000
            && readiness.rpc_timeout_milliseconds == 10_000
            && readiness.poll_interval_milliseconds == 250
            && fixture.checkpoints.catch_up == "C"
            && fixture.checkpoints.final_ == "F"
            && fixture.checkpoints.stop_role == "collator-b"
            && fixture.checkpoints.survivor_role == "collator-a"
            && fixture.checkpoints.before_c_last_mutation == "m07"
            && fixture.checkpoints.after_c_first_mutation == "m08"
            && fixture.checkpoints.final_mutation == "m21"
            && fixture.checkpoints.stability_interval_milliseconds == 12_000
            && fixture.checkpoints.stability_interval_count == 2,
        "checkpoint/readiness fixture drifted"
    );

    let expected_mutations = [
        (
            "m01",
            "before-c",
            "charlie",
            "collator-a",
            "lifecycle",
            "create_intent_unit",
        ),
        (
            "m02",
            "before-c",
            "dave",
            "collator-b",
            "lifecycle",
            "create_intent_unit",
        ),
        (
            "m03",
            "before-c",
            "charlie",
            "collator-a",
            "relationship",
            "create_relationship_definition",
        ),
        (
            "m04",
            "before-c",
            "dave",
            "collator-b",
            "relationship",
            "create_relationship",
        ),
        (
            "m05",
            "before-c",
            "charlie",
            "collator-a",
            "lifecycle",
            "transition_intent_unit",
        ),
        (
            "m06",
            "before-c",
            "dave",
            "collator-b",
            "lifecycle",
            "transition_intent_unit",
        ),
        (
            "m07",
            "before-c",
            "charlie",
            "collator-a",
            "lifecycle",
            "transition_intent_unit",
        ),
        (
            "m08",
            "after-c",
            "dave",
            "collator-a",
            "lifecycle",
            "transition_intent_unit",
        ),
        (
            "m09",
            "after-c",
            "charlie",
            "collator-a",
            "lifecycle",
            "transition_intent_unit",
        ),
        (
            "m10",
            "after-c",
            "dave",
            "collator-a",
            "lifecycle",
            "transition_intent_unit",
        ),
        (
            "m11",
            "after-c",
            "charlie",
            "collator-a",
            "lifecycle",
            "transition_intent_unit",
        ),
        (
            "m12",
            "after-c",
            "dave",
            "collator-a",
            "provenance-git",
            "record_association",
        ),
        (
            "m13",
            "after-c",
            "charlie",
            "collator-a",
            "lifecycle",
            "transition_intent_unit",
        ),
        (
            "m14",
            "after-c",
            "dave",
            "collator-a",
            "lifecycle",
            "complete_intent_unit",
        ),
        (
            "m15",
            "after-c",
            "charlie",
            "collator-a",
            "lifecycle",
            "transition_intent_unit",
        ),
        (
            "m16",
            "after-c",
            "charlie",
            "collator-a",
            "provenance-git",
            "record_association",
        ),
        (
            "m17",
            "after-c",
            "dave",
            "collator-a",
            "lifecycle",
            "complete_intent_unit",
        ),
        (
            "m18",
            "after-c",
            "charlie",
            "collator-a",
            "relationship",
            "delete_relationship",
        ),
        (
            "m19",
            "after-c",
            "dave",
            "collator-a",
            "relationship",
            "create_relationship",
        ),
        (
            "m20",
            "after-c",
            "charlie",
            "collator-a",
            "provenance-git",
            "revoke_association",
        ),
        (
            "m21",
            "after-c",
            "dave",
            "collator-a",
            "provenance-git",
            "record_association",
        ),
    ];
    require!(
        fixture.mutations.len() == expected_mutations.len(),
        "mutation fixture must contain exactly twenty-one operations"
    );
    for (mutation, expected) in fixture.mutations.iter().zip(expected_mutations) {
        require!(
            mutation.id == expected.0
                && mutation.phase == expected.1
                && mutation.signer == expected.2
                && mutation.endpoint == expected.3
                && mutation.work_category == expected.4
                && mutation.operation == expected.5,
            "mutation metadata drifted for {}",
            mutation.id
        );
        require!(
            mutation.request.pointer("/protocol_version") == Some(&Value::from(2))
                && mutation
                    .request
                    .pointer("/operation/type")
                    .and_then(Value::as_str)
                    == Some(mutation.operation.as_str()),
            "mutation request shape drifted for {}",
            mutation.id
        );
    }
    let mutation_ids = fixture
        .mutations
        .iter()
        .map(|mutation| mutation.id.as_str())
        .collect::<BTreeSet<_>>();
    require!(
        mutation_ids.len() == fixture.mutations.len(),
        "mutation IDs must be unique"
    );
    for signer in ["charlie", "dave"] {
        let categories = fixture
            .mutations
            .iter()
            .filter(|mutation| mutation.signer == signer)
            .map(|mutation| mutation.work_category.as_str())
            .collect::<BTreeSet<_>>();
        require!(
            categories == BTreeSet::from(["lifecycle", "relationship", "provenance-git"]),
            "{signer} must execute lifecycle, relationship, and provenance/Git work"
        );
    }

    let expected_reads = [
        ("get-primary", "get_intent_unit"),
        ("get-secondary", "get_intent_unit"),
        ("list-units-page-1", "list_intent_units"),
        ("list-units-page-2", "list_intent_units"),
        ("get-definition", "get_relationship_definition"),
        ("list-relationships", "list_relationships"),
        ("project-completed-page-1", "project_intent_units_v1"),
        ("project-completed-page-2", "project_intent_units_v1"),
        ("associations-primary", "list_associations_by_unit"),
        ("associations-secondary", "list_associations_by_unit"),
        (
            "associations-reference-page-1",
            "list_associations_by_reference",
        ),
        (
            "associations-reference-page-2",
            "list_associations_by_reference",
        ),
    ];
    require!(
        fixture.reads.len() == expected_reads.len(),
        "read fixture length drifted"
    );
    for (read, expected) in fixture.reads.iter().zip(expected_reads) {
        require!(
            read.id == expected.0
                && read.request.pointer("/protocol_version") == Some(&Value::from(2))
                && read
                    .request
                    .pointer("/operation/type")
                    .and_then(Value::as_str)
                    == Some(expected.1),
            "read request fixture drifted for {}",
            read.id
        );
    }

    let expected_snapshots = [
        ("uninterrupted", "both", "collator-a", false, false),
        ("fresh-a", "collator-a", "collator-a", true, false),
        ("fresh-b", "collator-b", "collator-b", true, false),
        ("rebuild-a", "collator-a", "collator-a", true, true),
        ("rebuild-b", "collator-b", "collator-b", true, true),
    ];
    require!(
        fixture.semantic_projection.snapshot_contracts.len() == expected_snapshots.len(),
        "snapshot contract length drifted"
    );
    for (actual, expected) in fixture
        .semantic_projection
        .snapshot_contracts
        .iter()
        .zip(expected_snapshots)
    {
        require!(
            actual.label == expected.0
                && actual.source_endpoint == expected.1
                && actual.read_endpoint == expected.2
                && actual.fresh_database == expected.3
                && actual.database_deleted_before_build == expected.4,
            "snapshot contract drifted for {}",
            actual.label
        );
    }
    require!(
        fixture.semantic_projection.section_counts
            == BTreeMap::from([
                ("checkpoint".to_owned(), 1),
                ("coordinates".to_owned(), 6),
                ("definitions".to_owned(), 1),
                ("edges".to_owned(), 1),
                ("history".to_owned(), 11),
                ("origins".to_owned(), 2),
                ("pages".to_owned(), 12),
                ("projections".to_owned(), 2),
                ("provenance".to_owned(), 2),
                ("units".to_owned(), 2),
            ]),
        "semantic section/count fixture drifted"
    );
    require!(
        fixture.archive_probe.range_start == 0
            && fixture.archive_probe.methods_per_block
                == [
                    "chain_getBlockHash",
                    "chain_getHeader",
                    "chain_getBlock",
                    "state_getStorage:System.Events",
                    "state_getStorage::code",
                ]
                .map(str::to_owned)
            && fixture.archive_probe.system_events_storage_key
                == "0x26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7"
            && fixture.archive_probe.runtime_code_storage_key == "0x3a636f6465",
        "archive probe fixture drifted"
    );
    let expected_artifacts = [
        ("action", "action/finalized-chain-census.json"),
        ("action", "action/finalized-relay-chain-census.json"),
        ("action", "action/reads.jsonl"),
        ("action", "action/submissions.jsonl"),
        ("config", "config/collator-a.raw.json"),
        ("config", "config/collator-b.raw.json"),
        ("config", "config/environment-inventory.json"),
        ("config", "config/materialized-zombienet.toml"),
        ("config", "config/process-inventory.json"),
        ("config", "config/pvf-worker-inventory.json"),
        ("config", "config/relay-a.raw.json"),
        ("config", "config/relay-b.raw.json"),
        ("fixture", "fixture/journey-v1.json"),
        ("fixture", "fixture/recorded-association-v1.json"),
        ("journal", "journal/archive-probes.jsonl"),
        ("journal", "journal/finality.jsonl"),
        ("journal", "journal/submission-lanes.json"),
        ("log", "log/collator-a.log"),
        ("log", "log/collator-b.log"),
        ("log", "log/genesis-export.log"),
        ("log", "log/materializer.log"),
        ("log", "log/orchestrator.log"),
        ("log", "log/relay-a.log"),
        ("log", "log/relay-b.log"),
        ("socket", "socket/listeners.txt"),
        ("socket", "socket/pvf-unix-sockets.json"),
    ];
    let expected_relay_grandpa_authorities = [
        (
            "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee".to_owned(),
            1_u64,
        ),
        (
            "0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69".to_owned(),
            1_u64,
        ),
    ];
    require!(
        fixture
            .audit
            .required_artifacts
            .iter()
            .map(|artifact| (artifact.kind.as_str(), artifact.logical_path.as_str()))
            .eq(expected_artifacts)
            && fixture.audit.chain_census.range_start == 0
            && fixture.audit.chain_census.maximum_retained_bytes == 4_194_304
            && fixture
                .audit
                .chain_census
                .minimum_session_rotation_count
                .is_none()
            && fixture
                .audit
                .chain_census
                .expected_initial_spot_price
                .is_none()
            && fixture
                .audit
                .chain_census
                .expected_grandpa_authorities
                .is_none()
            && fixture.audit.chain_census.allowed_unsigned_calls
                == ["parachainSystem.setValidationData", "timestamp.set"].map(str::to_owned)
            && fixture.audit.chain_census.allowed_events
                == [
                    "balances.BurnedDebt",
                    "balances.Withdraw",
                    "cubikan.Accepted",
                    "system.ExtrinsicSuccess",
                    "transactionPayment.TransactionFeePaid",
                ]
                .map(str::to_owned)
            && fixture.audit.relay_chain_census.range_start == 0
            && fixture.audit.relay_chain_census.maximum_retained_bytes == 4_194_304
            && fixture
                .audit
                .relay_chain_census
                .minimum_session_rotation_count
                == Some(2)
            && fixture.audit.relay_chain_census.expected_initial_spot_price == Some(10_000_000)
            && fixture
                .audit
                .relay_chain_census
                .expected_grandpa_authorities
                .as_deref()
                == Some(expected_relay_grandpa_authorities.as_slice())
            && fixture.audit.relay_chain_census.allowed_unsigned_calls
                == ["timestamp.set", "paraInherent.enter"].map(str::to_owned)
            && fixture.audit.relay_chain_census.allowed_events
                == [
                    "onDemandAssignmentProvider.SpotPriceSet",
                    "system.ExtrinsicSuccess",
                    "paraInclusion.CandidateBacked",
                    "paraInclusion.CandidateIncluded",
                    "paraInclusion.CandidateTimedOut",
                    "session.NewSession",
                    "session.NewQueued",
                    "historical.RootStored",
                    "grandpa.NewAuthorities",
                ]
                .map(str::to_owned)
            && fixture.audit.zero_count_keys
                == [
                    "allowlist_mutation",
                    "public_rpc",
                    "account_creation",
                    "key_import",
                    "faucet",
                    "transfer",
                    "para_id_registration",
                    "coretime",
                    "runtime_upload",
                    "deployment",
                    "release",
                    "governance",
                    "secret",
                ]
                .map(str::to_owned)
            && fixture.audit.dev_signers == ["charlie", "dave"].map(str::to_owned)
            && fixture.audit.dev_signer_accounts
                == BTreeMap::from([
                    (
                        "charlie".to_owned(),
                        "0x90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22"
                            .to_owned(),
                    ),
                    (
                        "dave".to_owned(),
                        "0x306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20"
                            .to_owned(),
                    ),
                ])
            && decode_lower_hex(&fixture.audit.dev_signer_accounts["charlie"])?
                == DevSigner::Charlie.account_id()
            && decode_lower_hex(&fixture.audit.dev_signer_accounts["dave"])?
                == DevSigner::Dave.account_id()
            && fixture.audit.synthetic_origins == ["INT-0008", "INT-0014"].map(str::to_owned)
            && fixture.audit.allowed_hosts == ["127.0.0.1", "localhost", "::1"].map(str::to_owned),
        "audit fixture drifted"
    );
    Ok(())
}

fn verify_exact_t1114_association(fixture: &ExactAssociationFixture) -> Result<(), String> {
    validate_unique_json(T1114_ASSOCIATION_BYTES, "T-1114 association fixture")?;
    require!(
        fixture.fixture_path == "tests/fixtures/git/recorded-association-v1.json"
            && fixture.fixture_size == 217
            && fixture.fixture_sha256
                == "ab5e568c937254e1848e31d35203ce60f34ea6bb8029b988d167fedee24d7dcd"
            && fixture.trailing_byte == b'\n'
            && fixture.subject_kind == "revision"
            && fixture.required_evidence_sources
                == ["fresh-a", "fresh-b", "rebuild-a", "rebuild-b"].map(str::to_owned),
        "T-1114 association transfer metadata drifted"
    );
    require!(
        T1114_ASSOCIATION_BYTES.len() as u64 == fixture.fixture_size
            && T1114_ASSOCIATION_BYTES.last() == Some(&fixture.trailing_byte)
            && T1114_ASSOCIATION_BYTES[..T1114_ASSOCIATION_BYTES.len() - 1]
                == *fixture.core_json.as_bytes(),
        "T-1114 association bytes no longer equal the independently pinned core JSON plus LF"
    );
    require!(
        sha256_bytes(T1114_ASSOCIATION_BYTES)? == fixture.fixture_sha256,
        "T-1114 association SHA-256 drifted"
    );

    let association = RecordedAssociation::new(
        fixture
            .unit_id
            .parse::<IntentUnitId>()
            .map_err(|error| format!("fixture unit ID is invalid: {error}"))?,
        AssociationSubject::Revision(fixture.subject_revision),
        ExternalReference::new(
            ReferenceNamespace::new(fixture.namespace.clone())
                .map_err(|error| format!("fixture namespace is invalid: {error}"))?,
            ReferenceText::new(fixture.scope.clone())
                .map_err(|error| format!("fixture scope is invalid: {error}"))?,
            ReferenceText::new(fixture.value.clone())
                .map_err(|error| format!("fixture value is invalid: {error}"))?,
        ),
    );
    let typed_bytes = serde_json::to_vec(&association)
        .map_err(|error| format!("cannot serialize typed T-1114 association: {error}"))?;
    require!(
        typed_bytes == fixture.core_json.as_bytes(),
        "typed RecordedAssociation bytes differ from the T-1114 fixture"
    );

    let exact_mutation = fixture_exact_mutation_request()?;
    require!(
        exact_mutation
            .pointer("/operation/association/unit_id")
            .and_then(Value::as_str)
            == Some(fixture.unit_id.as_str())
            && exact_mutation
                .pointer("/operation/association/subject/revision")
                .and_then(Value::as_str)
                == Some("7")
            && exact_mutation
                .pointer("/operation/association/reference/namespace")
                .and_then(Value::as_str)
                == Some(fixture.namespace.as_str())
            && exact_mutation
                .pointer("/operation/association/reference/scope")
                .and_then(Value::as_str)
                == Some(fixture.scope.as_str())
            && exact_mutation
                .pointer("/operation/association/reference/value")
                .and_then(Value::as_str)
                == Some(fixture.value.as_str()),
        "T-1115 exact mutation is not the T-1114 association"
    );
    Ok(())
}

fn fixture_exact_mutation_request() -> Result<Value, String> {
    let fixture: JourneyFixture = serde_json::from_slice(FIXTURE_BYTES)
        .map_err(|error| format!("cannot reparse journey fixture: {error}"))?;
    fixture
        .mutations
        .into_iter()
        .find(|mutation| mutation.id == "m12")
        .map(|mutation| mutation.request)
        .ok_or_else(|| "journey fixture lacks exact m12 association".to_owned())
}

fn validate_unique_json(bytes: &[u8], label: &str) -> Result<(), String> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    DuplicateCheckedJson::deserialize(&mut deserializer)
        .map_err(|error| format!("{label} has invalid or duplicate-key JSON: {error}"))?;
    deserializer
        .end()
        .map_err(|error| format!("{label} has trailing invalid JSON bytes: {error}"))
}

fn file_identity(metadata: &fs::Metadata) -> FileIdentityEvidence {
    FileIdentityEvidence {
        device: metadata.dev().to_string(),
        inode: metadata.ino().to_string(),
        size: metadata.len(),
        mode: format!("{:04o}", metadata.mode() & 0o7777),
        link_count: metadata.nlink(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

fn sha256_open_file(file: &mut fs::File, label: &Path) -> Result<(String, u64), String> {
    let mut child = Command::new(SHA256SUM)
        .arg("-")
        .env_clear()
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot start sha256sum for {}: {error}", label.display()))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| format!("sha256sum stdin is unavailable for {}", label.display()))?;
    let bytes_read = std::io::copy(file, &mut stdin)
        .map_err(|error| format!("cannot stream {} into sha256sum: {error}", label.display()))?;
    drop(stdin);
    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot wait for sha256sum for {}: {error}", label.display()))?;
    require!(
        output.status.success() && output.stderr.is_empty(),
        "sha256sum failed for opened input {}",
        label.display()
    );
    Ok((parse_sha256sum(&output.stdout)?, bytes_read))
}

fn read_opened_regular_file(path: &Path, label: &str) -> Result<OpenedFileEvidence, String> {
    let path_before = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "required {label} {} is unavailable: {error}",
            path.display()
        )
    })?;
    require!(
        path_before.file_type().is_file() && !path_before.file_type().is_symlink(),
        "{label} {} is not a non-symbolic regular file",
        path.display()
    );
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW_FLAG | O_CLOEXEC_FLAG)
        .open(path)
        .map_err(|error| format!("cannot open {label} {}: {error}", path.display()))?;
    let descriptor_before = file
        .metadata()
        .map_err(|error| format!("cannot stat opened {label} {}: {error}", path.display()))?;
    let (sha256, bytes_read) = sha256_open_file(&mut file, path)?;
    let descriptor_after = file.metadata().map_err(|error| {
        format!(
            "cannot restat opened {label} {} after hashing: {error}",
            path.display()
        )
    })?;
    let path_after = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "cannot restat {label} {} after hashing: {error}",
            path.display()
        )
    })?;
    let evidence = OpenedFileEvidence {
        path_before: file_identity(&path_before),
        descriptor_before: file_identity(&descriptor_before),
        descriptor_after: file_identity(&descriptor_after),
        path_after: file_identity(&path_after),
        bytes_read,
        sha256,
        regular_file: path_before.file_type().is_file(),
        symbolic_link: path_before.file_type().is_symlink(),
    };
    assert_opened_file_read(&evidence, false, label)?;
    Ok(evidence)
}

fn read_pinned_input(repo_root: &Path, pin: &PinnedInput) -> Result<OpenedFileEvidence, String> {
    let path = repo_root.join(&pin.path);
    let evidence = read_opened_regular_file(&path, &format!("pin {}", pin.path))?;
    require!(
        evidence.bytes_read == pin.size && evidence.sha256 == pin.sha256,
        "pinned input {} size or SHA-256 mismatch",
        pin.path
    );
    Ok(evidence)
}

fn verify_pinned_inputs(
    repo_root: &Path,
    pins: &[PinnedInput],
) -> Result<Vec<OpenedFileEvidence>, String> {
    pins.iter()
        .map(|pin| read_pinned_input(repo_root, pin))
        .collect()
}

fn verify_pinned_input_continuity(
    repo_root: &Path,
    pins: &[PinnedInput],
    prelaunch: &[OpenedFileEvidence],
    observed: &[ObservedPin],
) -> Result<(), String> {
    require!(
        pins.len() == prelaunch.len() && pins.len() == observed.len(),
        "prelaunch, producer, and fixture pin inventories differ in length"
    );
    for ((pin, before), produced) in pins.iter().zip(prelaunch).zip(observed) {
        require!(
            produced.path == pin.path && produced.verified && produced.read == *before,
            "pin {} changed between the Rust prelaunch read and producer use",
            pin.path
        );
        let after = read_pinned_input(repo_root, pin)?;
        require!(
            after == *before,
            "pin {} changed between prelaunch, producer use, and post-run read",
            pin.path
        );
    }
    Ok(())
}

fn retain_input_parent_directories(
    repo_root: &Path,
    pins: &[PinnedInput],
    local_binary: &Path,
) -> Result<Vec<DirectorySpan>, String> {
    let mut paths = BTreeSet::new();
    for file in pins
        .iter()
        .map(|pin| repo_root.join(&pin.path))
        .chain(std::iter::once(local_binary.to_path_buf()))
    {
        let parent = file
            .parent()
            .ok_or_else(|| format!("input {} lacks a parent directory", file.display()))?;
        paths.extend(parent.ancestors().map(Path::to_path_buf));
    }
    paths
        .into_iter()
        .map(|path| {
            let canonical = path.canonicalize().map_err(|error| {
                format!(
                    "cannot canonicalize input parent {}: {error}",
                    path.display()
                )
            })?;
            require!(
                canonical == path,
                "input parent {} is not already canonical",
                path.display()
            );
            let path_metadata = fs::symlink_metadata(&path)
                .map_err(|error| format!("cannot stat input parent {}: {error}", path.display()))?;
            require!(
                path_metadata.file_type().is_dir() && !path_metadata.file_type().is_symlink(),
                "input parent {} is not a non-symbolic directory",
                path.display()
            );
            let handle = fs::OpenOptions::new()
                .read(true)
                .custom_flags(O_NOFOLLOW_FLAG | O_CLOEXEC_FLAG | O_DIRECTORY_FLAG)
                .open(&path)
                .map_err(|error| {
                    format!("cannot retain input parent {}: {error}", path.display())
                })?;
            let before = file_identity(&path_metadata);
            require!(
                file_identity(&handle.metadata().map_err(|error| {
                    format!("cannot stat retained parent {}: {error}", path.display())
                })?) == before,
                "input parent {} path/descriptor identity diverged",
                path.display()
            );
            Ok(DirectorySpan {
                path,
                handle,
                before,
            })
        })
        .collect()
}

fn verify_input_parent_directories(spans: &[DirectorySpan]) -> Result<(), String> {
    for span in spans {
        let descriptor_after = file_identity(&span.handle.metadata().map_err(|error| {
            format!(
                "cannot restat retained input parent {}: {error}",
                span.path.display()
            )
        })?);
        let path_after = file_identity(&fs::symlink_metadata(&span.path).map_err(|error| {
            format!(
                "cannot restat input parent path {}: {error}",
                span.path.display()
            )
        })?);
        require!(
            descriptor_after == span.before && path_after == span.before,
            "input parent {} changed while an executable or producer pin could be used",
            span.path.display()
        );
    }
    Ok(())
}

fn verify_runtime_executable_continuity(
    fixture: &JourneyFixture,
    evidence: &JourneyEvidence,
    local_binary: &Path,
    prelaunch_local_binary: &OpenedFileEvidence,
) -> Result<(), String> {
    let runtime = &evidence.runtime_executables;
    let local_path = local_binary
        .to_str()
        .ok_or_else(|| "cubikan-local candidate path is not UTF-8".to_owned())?;
    require!(
        runtime.local_binary.path == local_path
            && runtime.local_binary.before == *prelaunch_local_binary
            && runtime.local_binary.after == *prelaunch_local_binary,
        "cubikan-local candidate path, identity, or bytes changed across producer use"
    );
    let postlaunch_local_binary =
        read_opened_regular_file(local_binary, "post-run cubikan-local candidate")?;
    require!(
        postlaunch_local_binary == *prelaunch_local_binary,
        "cubikan-local candidate changed between prelaunch, producer use, and post-run read"
    );

    let node = &runtime.orchestrator_node;
    let expected_node_path = PathBuf::from(&fixture.launcher.materialized_node_path);
    require!(
        node.toolchain_root == fixture.launcher.materialized_toolchain_root
            && node.toolchain_root_identity_before == node.toolchain_root_identity_after
            && is_nonzero_canonical_identity(&node.toolchain_root_identity_before.device)
            && is_nonzero_canonical_identity(&node.toolchain_root_identity_before.inode)
            && node.toolchain_root_identity_before.mode
                == fixture.launcher.materialized_toolchain_mode
            && node.toolchain_root_identity_before.link_count > 0
            && node.toolchain_filesystem_magic
                == fixture.launcher.materialized_toolchain_filesystem_magic
            && node.toolchain_write_probe_errno
                == fixture.launcher.materialized_toolchain_write_probe_errno
            && node.toolchain_mount_before == node.toolchain_mount_after,
        "private materialized toolchain identity/filesystem/write denial changed during use"
    );
    assert_exact_tmpfs_bind_mount(
        &node.toolchain_mount_before,
        &fixture.launcher.materialized_toolchain_root,
        "materialized toolchain",
    )?;
    require!(
        node.path == expected_node_path.to_string_lossy()
            && node.expected_size == fixture.launcher.node_executable_size
            && node.expected_sha256 == fixture.launcher.node_executable_sha256
            && node.materialized_before == node.materialized_after,
        "materialized orchestrator Node path or independent executable oracle drifted"
    );
    assert_opened_file_read(
        &node.materialized_before,
        true,
        "materialized orchestrator Node",
    )?;
    assert_proc_executable_read(&node.proc_exe_before, "orchestrator /proc/self/exe")?;
    assert_proc_executable_read(&node.proc_exe_after, "post-run orchestrator /proc/self/exe")?;
    require!(
        node.proc_exe_before == node.proc_exe_after
            && node.materialized_before.path_before == node.proc_exe_before.path_before
            && node.materialized_before.bytes_read == node.expected_size
            && node.proc_exe_before.bytes_read == node.expected_size
            && node.materialized_before.sha256 == node.expected_sha256
            && node.proc_exe_before.sha256 == node.expected_sha256
            && evidence.topology.orchestrator.executable == node.path,
        "materialized Node and retained /proc/self/exe bytes/object identity diverged"
    );
    let toolchain_root = Path::new(&fixture.launcher.materialized_toolchain_root);
    require!(
        !toolchain_root.exists()
            && fs::symlink_metadata(toolchain_root)
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "private materialized toolchain root remained after launcher exit"
    );
    Ok(())
}

fn assert_exact_tmpfs_bind_mount(
    mount: &PvfMountEvidence,
    expected_root: &str,
    label: &str,
) -> Result<(), String> {
    let fields = mount
        .raw_mountinfo_line
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
    let separator = fields
        .iter()
        .position(|field| *field == "-")
        .ok_or_else(|| format!("{label} mountinfo evidence lacks separator"))?;
    require!(
        separator >= 6 && fields.len() == separator + 4,
        "{label} mountinfo evidence is malformed"
    );
    let mut raw_mount_options = fields[5].split(',').collect::<Vec<_>>();
    raw_mount_options.sort_unstable();
    let mut raw_super_options = fields[separator + 3].split(',').collect::<Vec<_>>();
    raw_super_options.sort_unstable();
    require!(
        fields[0].parse::<u64>().ok() == Some(mount.mount_id)
            && fields[1].parse::<u64>().ok() == Some(mount.parent_id)
            && fields[2] == mount.major_minor
            && fields[3] == mount.root
            && fields[4] == mount.mount_point
            && raw_mount_options
                == mount
                    .mount_options
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
            && fields[6..separator] == *mount.optional_fields
            && fields[separator + 1] == mount.filesystem_type
            && fields[separator + 2] == mount.mount_source
            && raw_super_options
                == mount
                    .super_options
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
            && mount.mount_id > 0
            && mount.parent_id > 0
            && mount
                .major_minor
                .split_once(':')
                .is_some_and(|(major, minor)| {
                    !major.is_empty()
                        && !minor.is_empty()
                        && major.bytes().all(|byte| byte.is_ascii_digit())
                        && minor.bytes().all(|byte| byte.is_ascii_digit())
                })
            && mount.root != "/"
            && mount.mount_point == expected_root
            && mount.filesystem_type == "tmpfs"
            && mount.bind
            && mount.read_only
            && mount.nodev
            && mount.nosuid
            && mount.executable
            && mount.mount_options.windows(2).all(|pair| pair[0] < pair[1])
            && mount.super_options.windows(2).all(|pair| pair[0] < pair[1])
            && mount.mount_options.iter().any(|option| option == "ro")
            && mount.mount_options.iter().any(|option| option == "nodev")
            && mount.mount_options.iter().any(|option| option == "nosuid")
            && !mount
                .mount_options
                .iter()
                .any(|option| option == "rw" || option == "noexec"),
        "{label} is not the exact independently parsed read-only executable nodev/nosuid tmpfs bind"
    );
    Ok(())
}

fn validate_supported_root() -> Result<PathBuf, String> {
    let configured = env::var_os(SUPPORTED_ROOT_ENV)
        .ok_or_else(|| format!("{SUPPORTED_ROOT_ENV} is required by the explicit chain gate"))?;
    let configured = PathBuf::from(configured);
    require!(
        configured.is_absolute(),
        "{SUPPORTED_ROOT_ENV} must be absolute"
    );
    let metadata = fs::symlink_metadata(&configured).map_err(|error| {
        format!(
            "supported root {} is unavailable: {error}",
            configured.display()
        )
    })?;
    require!(
        metadata.file_type().is_dir() && !metadata.file_type().is_symlink(),
        "supported root must be a non-symbolic directory"
    );
    require!(
        metadata.permissions().mode() & 0o777 == REQUIRED_ROOT_MODE,
        "supported root must have mode 0700"
    );
    let current_uid = fs::metadata("/proc/self")
        .map_err(|error| format!("cannot inspect current process owner: {error}"))?
        .uid();
    require!(
        metadata.uid() == current_uid,
        "supported root must be owned by the test process user"
    );
    let canonical = configured
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize supported root: {error}"))?;
    require!(
        canonical == configured,
        "{SUPPORTED_ROOT_ENV} must already be canonical"
    );
    let output = Command::new(STAT)
        .args(["-f", "-c", "%t", "--"])
        .arg(&canonical)
        .env_clear()
        .env("LC_ALL", "C")
        .output()
        .map_err(|error| format!("cannot inspect supported filesystem: {error}"))?;
    require!(
        output.status.success() && output.stderr.is_empty(),
        "filesystem inspection failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    require!(
        output.stdout == format!("{REQUIRED_FILESYSTEM_MAGIC}\n").as_bytes(),
        "supported root filesystem must have magic {REQUIRED_FILESYSTEM_MAGIC}, got {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(canonical)
}

fn sha256_bytes(bytes: &[u8]) -> Result<String, String> {
    let mut child = Command::new(SHA256SUM)
        .arg("-")
        .env_clear()
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot start sha256sum: {error}"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| "sha256sum stdin was not piped".to_owned())?
        .write_all(bytes)
        .map_err(|error| format!("cannot stream bytes to sha256sum: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot wait for sha256sum: {error}"))?;
    require!(
        output.status.success() && output.stderr.is_empty(),
        "sha256sum failed while hashing bytes"
    );
    parse_sha256sum(&output.stdout)
}

fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    fn sort(value: Value) -> Value {
        match value {
            Value::Array(values) => Value::Array(values.into_iter().map(sort).collect()),
            Value::Object(values) => {
                let mut entries = values.into_iter().collect::<Vec<_>>();
                entries.sort_by(|left, right| left.0.cmp(&right.0));
                Value::Object(
                    entries
                        .into_iter()
                        .map(|(key, value)| (key, sort(value)))
                        .collect(),
                )
            }
            scalar => scalar,
        }
    }

    let value = serde_json::to_value(value)
        .map_err(|error| format!("cannot construct canonical JSON: {error}"))?;
    serde_json::to_vec(&sort(value))
        .map_err(|error| format!("cannot serialize canonical JSON: {error}"))
}

fn canonical_json_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    sha256_bytes(&canonical_json_bytes(value)?)
}

fn canonical_json_lines<T: Serialize>(values: &[T]) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    for value in values {
        bytes.extend_from_slice(&canonical_json_bytes(value)?);
        bytes.push(b'\n');
    }
    Ok(bytes)
}

fn parse_sha256sum(stdout: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(stdout)
        .map_err(|error| format!("sha256sum returned non-UTF-8: {error}"))?;
    let hash = text
        .split_ascii_whitespace()
        .next()
        .ok_or_else(|| "sha256sum returned no digest".to_owned())?;
    require!(is_sha256(hash), "sha256sum returned an invalid digest");
    Ok(hash.to_owned())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}

fn is_chain_hash(value: &str) -> bool {
    value.strip_prefix("0x").is_some_and(is_sha256)
}

fn is_nonzero_canonical_identity(value: &str) -> bool {
    value
        .parse::<u64>()
        .ok()
        .is_some_and(|parsed| parsed > 0 && parsed.to_string() == value)
}

fn decode_lower_hex(value: &str) -> Result<Vec<u8>, String> {
    let digits = value
        .strip_prefix("0x")
        .ok_or_else(|| "hex value lacks 0x prefix".to_owned())?;
    require!(
        !digits.is_empty() && digits.len().is_multiple_of(2),
        "hex value has invalid length"
    );
    digits
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let nibble = |byte: u8| match byte {
                b'0'..=b'9' => Ok(byte - b'0'),
                b'a'..=b'f' => Ok(byte - b'a' + 10),
                _ => Err("hex value is not lowercase hexadecimal".to_owned()),
            };
            Ok((nibble(pair[0])? << 4) | nibble(pair[1])?)
        })
        .collect()
}

fn assert_raw_cli_json_output(
    body_hex: &str,
    body_size: u64,
    body_sha256: &str,
    stdout_size: u64,
    stdout_sha256: &str,
    expected: &Value,
    label: &str,
) -> Result<(), String> {
    let body = decode_lower_hex(body_hex)
        .map_err(|error| format!("{label} body is not exact lowercase hex: {error}"))?;
    let exact_body_size =
        u64::try_from(body.len()).map_err(|_| format!("{label} body length overflowed u64"))?;
    require!(
        body.first() == Some(&b'{') && body.last() == Some(&b'}'),
        "{label} body has leading/trailing framing outside its JSON object"
    );
    require!(
        body_size == exact_body_size
            && is_sha256(body_sha256)
            && body_sha256 == sha256_bytes(&body)?,
        "{label} body size/SHA-256 does not match its retained bytes"
    );
    validate_unique_json(&body, label)?;
    let parsed: Value = serde_json::from_slice(&body)
        .map_err(|error| format!("{label} body is not valid JSON: {error}"))?;
    require!(
        &parsed == expected,
        "{label} retained bytes do not equal the typed response evidence"
    );

    let mut stdout = body;
    stdout.push(b'\n');
    let exact_stdout_size =
        u64::try_from(stdout.len()).map_err(|_| format!("{label} stdout length overflowed u64"))?;
    require!(
        stdout_size == exact_stdout_size
            && is_sha256(stdout_sha256)
            && stdout_sha256 == sha256_bytes(&stdout)?,
        "{label} stdout is not the exact retained JSON body plus one terminal LF"
    );
    Ok(())
}

fn exact_exec_lines<'a>(bytes: &'a [u8], label: &str) -> Result<Vec<&'a str>, String> {
    require!(
        bytes.last() == Some(&b'\n'),
        "{label} is not terminal-LF-framed"
    );
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("{label} is not exact UTF-8: {error}"))?;
    Ok(text
        .split_terminator('\n')
        .filter(|line| line.starts_with("exec: "))
        .collect())
}

fn assert_bootstrap_evidence(
    journey: &JourneyRun,
    collator_a: &NodeEvidence,
    collator_b: &NodeEvidence,
) -> Result<(), String> {
    let fixture = &journey.fixture;
    let evidence = &journey.evidence.bootstrap;
    let network_root = Path::new(&journey.evidence.topology.orchestrator.data_directory);
    let work_root = network_root
        .parent()
        .ok_or_else(|| "orchestrator network root lacks a work-root parent".to_owned())?;
    let bootstrap_root = work_root.join("bootstrap");
    let materializer_log_path = bootstrap_root.join("materializer.log");
    let genesis_export_log_path = bootstrap_root.join("genesis-export.log");
    let genesis_head_path = bootstrap_root.join("parachain-genesis-head");
    let genesis_wasm_path = bootstrap_root.join("parachain-genesis-wasm");
    let chain_spec_path = journey.repo_root.join("chain/config/cubikan-local.json");
    let parachain_binary_path = journey
        .repo_root
        .join("chain/.cache/downloads/polkadot-parachain");
    let materializer_path = journey
        .repo_root
        .join("chain/tools/materialize-zombienet.sh");

    require!(
        evidence.log_max_bytes == fixture.launcher.bootstrap_log_max_bytes
            && evidence.log_max_bytes == 1_048_576,
        "bootstrap evidence retained an unexpected log bound"
    );
    let materializer_log_bytes = decode_lower_hex(&evidence.materializer_log.bytes_hex)
        .map_err(|error| format!("materializer log bytes are not exact lowercase hex: {error}"))?;
    let genesis_export_log_bytes = decode_lower_hex(&evidence.genesis_export_log.bytes_hex)
        .map_err(|error| {
            format!("genesis export log bytes are not exact lowercase hex: {error}")
        })?;
    for (observed, expected, bytes, label) in [
        (
            &evidence.materializer_log,
            materializer_log_path.as_path(),
            materializer_log_bytes.as_slice(),
            "materializer log",
        ),
        (
            &evidence.genesis_export_log,
            genesis_export_log_path.as_path(),
            genesis_export_log_bytes.as_slice(),
            "genesis export log",
        ),
    ] {
        let bytes_read = u64::try_from(bytes.len())
            .map_err(|_| format!("{label} retained byte length overflowed u64"))?;
        require!(
            Path::new(&observed.path) == expected
                && observed.before == observed.after
                && observed.before.path_before.mode == "0600"
                && observed.before.path_before.link_count == 1
                && observed.before.bytes_read > 0
                && observed.before.bytes_read <= evidence.log_max_bytes
                && observed.before.bytes_read == bytes_read
                && observed.before.sha256 == sha256_bytes(bytes)?,
            "{label} path, stability, permissions, or retained bound drifted"
        );
        assert_opened_file_read(&observed.before, true, label)?;
    }

    let runtime_pin = pinned_input(
        &fixture.pinned_inputs,
        "chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm",
    )?;
    let head = &evidence.genesis_head;
    let wasm = &evidence.genesis_wasm;
    require!(
        Path::new(&head.path) == genesis_head_path
            && Path::new(&wasm.path) == genesis_wasm_path
            && head.before == head.after
            && wasm.before == wasm.after
            && head.before.path_before.mode == "0600"
            && wasm.before.path_before.mode == "0600"
            && head.before.path_before.link_count == 1
            && wasm.before.path_before.link_count == 1
            && head.encoding == "lowercase-0x-hex-no-whitespace-v1"
            && wasm.encoding == "lowercase-0x-hex-no-whitespace-v1"
            && head.decoded_size == fixture.launcher.genesis_head_decoded_size
            && head.decoded_sha256 == fixture.launcher.genesis_head_decoded_sha256
            && head.before.bytes_read
                == head
                    .decoded_size
                    .checked_mul(2)
                    .and_then(|size| size.checked_add(2))
                    .ok_or_else(|| "genesis-head encoded size overflowed".to_owned())?
            && wasm.decoded_size == runtime_pin.size
            && wasm.decoded_sha256 == runtime_pin.sha256
            && wasm.before.bytes_read
                == wasm
                    .decoded_size
                    .checked_mul(2)
                    .and_then(|size| size.checked_add(2))
                    .ok_or_else(|| "genesis-wasm encoded size overflowed".to_owned())?
            && is_sha256(&head.decoded_sha256)
            && is_sha256(&wasm.decoded_sha256)
            && is_chain_hash(&head.chain_hash)
            && head.chain_hash == collator_a.primary_chain_spec.live_genesis_hash
            && head.chain_hash == collator_b.primary_chain_spec.live_genesis_hash
            && wasm.decoded_sha256 == collator_a.primary_runtime_sha256
            && wasm.decoded_sha256 == collator_b.primary_runtime_sha256,
        "bootstrap genesis exports do not bind exact raw encoding, runtime, and live genesis"
    );
    assert_opened_file_read(&head.before, true, "parachain genesis-head export")?;
    assert_opened_file_read(&wasm.before, true, "parachain genesis-wasm export")?;

    let expected_materializer_command = vec![
        materializer_path.to_string_lossy().into_owned(),
        "--output-dir".to_owned(),
        fixture.launcher.materialized_toolchain_root.clone(),
    ];
    let expected_export_commands = vec![
        vec![
            parachain_binary_path.to_string_lossy().into_owned(),
            "export-genesis-head".to_owned(),
            "--chain".to_owned(),
            chain_spec_path.to_string_lossy().into_owned(),
            genesis_head_path.to_string_lossy().into_owned(),
        ],
        vec![
            parachain_binary_path.to_string_lossy().into_owned(),
            "export-genesis-wasm".to_owned(),
            "--chain".to_owned(),
            chain_spec_path.to_string_lossy().into_owned(),
            genesis_wasm_path.to_string_lossy().into_owned(),
        ],
    ];
    require!(
        evidence.materializer_command == expected_materializer_command
            && evidence.export_commands == expected_export_commands
            && Path::new(&evidence.chain_spec_path) == chain_spec_path
            && Path::new(&evidence.parachain_binary_path) == parachain_binary_path,
        "bootstrap producer/export argv and pinned input paths drifted"
    );
    let materializer_exec_line = format!("exec: {}", expected_materializer_command.join(" "));
    let expected_genesis_exec_lines = expected_export_commands
        .iter()
        .map(|command| format!("exec: {}", command.join(" ")))
        .collect::<Vec<_>>();
    let materializer_exec_lines = exact_exec_lines(&materializer_log_bytes, "materializer log")?;
    let genesis_exec_lines = exact_exec_lines(&genesis_export_log_bytes, "genesis export log")?;
    require!(
        materializer_log_bytes.starts_with(format!("{materializer_exec_line}\n").as_bytes())
            && materializer_exec_lines == [materializer_exec_line.as_str()]
            && genesis_export_log_bytes
                .starts_with(format!("{}\n", expected_genesis_exec_lines[0]).as_bytes())
            && genesis_exec_lines.len() == expected_genesis_exec_lines.len()
            && genesis_exec_lines
                .iter()
                .zip(&expected_genesis_exec_lines)
                .all(|(observed, expected)| *observed == expected.as_str()),
        "bootstrap retained logs do not bind the exact closed execution transcript"
    );
    let allowed_hosts = fixture
        .audit
        .allowed_hosts
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let allowed_ports = fixture
        .topology
        .nodes
        .iter()
        .flat_map(|node| node.listeners.iter())
        .filter_map(|address| address.rsplit_once(':').map(|(_, port)| port))
        .collect::<BTreeSet<_>>();
    for (label, bytes) in [
        ("materializer", materializer_log_bytes.as_slice()),
        ("genesis export", genesis_export_log_bytes.as_slice()),
    ] {
        let text = std::str::from_utf8(bytes)
            .map_err(|error| format!("{label} log is not exact UTF-8: {error}"))?;
        require!(
            first_forbidden_diagnostic_marker(text).is_none()
                && first_disallowed_diagnostic_url(text, &allowed_hosts, &allowed_ports).is_none(),
            "{label} log contains a secret/helper marker or public URL"
        );
    }
    let materializer_artifact = journey
        .evidence
        .audit
        .artifacts
        .iter()
        .find(|artifact| artifact.logical_path == "log/materializer.log")
        .ok_or_else(|| "audit lacks the materializer log copy".to_owned())?;
    let genesis_artifact = journey
        .evidence
        .audit
        .artifacts
        .iter()
        .find(|artifact| artifact.logical_path == "log/genesis-export.log")
        .ok_or_else(|| "audit lacks the genesis export log copy".to_owned())?;
    require!(
        materializer_artifact.read.bytes_read == evidence.materializer_log.before.bytes_read
            && materializer_artifact.read.sha256 == evidence.materializer_log.before.sha256
            && genesis_artifact.read.bytes_read == evidence.genesis_export_log.before.bytes_read
            && genesis_artifact.read.sha256 == evidence.genesis_export_log.before.sha256,
        "bootstrap audit log copies do not bind exact source bytes"
    );
    for source in [
        materializer_log_path,
        genesis_export_log_path,
        genesis_head_path,
        genesis_wasm_path,
    ] {
        require!(
            fs::symlink_metadata(&source)
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
            "bootstrap source {} remained after work-root cleanup",
            source.display()
        );
    }
    Ok(())
}

fn assert_e1(journey: &JourneyRun) -> Result<(), String> {
    let fixture = &journey.fixture;
    let evidence = &journey.evidence;
    require!(
        evidence.isolation.loopback_network_namespace
            && evidence.isolation.private_pid_namespace
            && evidence.isolation.fresh_procfs
            && evidence.isolation.pid_one_reaper_present
            && evidence.isolation.external_connectivity_denied
            && evidence.isolation.synthetic_chain
            && evidence.isolation.dev_only,
        "the journey did not prove a loopback-only synthetic dev namespace"
    );
    require!(
        evidence.isolation.supported_root == journey.supported_root.to_string_lossy()
            && evidence.isolation.supported_root_filesystem_magic == REQUIRED_FILESYSTEM_MAGIC,
        "launcher did not attest the exact supported root/filesystem"
    );
    require!(
        evidence.pins.len() == fixture.pinned_inputs.len(),
        "observed pin inventory length differs from the independent fixture"
    );
    for (observed, expected) in evidence.pins.iter().zip(&fixture.pinned_inputs) {
        require!(
            observed.verified
                && observed.path == expected.path
                && observed.read.path_before.size == expected.size
                && observed.read.sha256 == expected.sha256,
            "launcher pin evidence drifted for {}",
            expected.path
        );
        assert_opened_file_read(&observed.read, false, &format!("pin {}", expected.path))?;
    }

    let topology = &evidence.topology;
    require!(
        topology.unexpected_node_processes == 0 && topology.unexpected_listeners.is_empty(),
        "unexpected node process or listener was present"
    );
    require!(
        topology.nodes.len() == fixture.topology.node_count as usize,
        "live node count was not exactly four"
    );
    require!(
        topology.orchestrator.pid > 1
            && Path::new(&topology.orchestrator.executable).is_absolute()
            && topology.orchestrator.argv.first() == Some(&topology.orchestrator.executable)
            && topology.orchestrator.process_start_time_ticks_before > 0
            && topology.orchestrator.process_start_time_ticks_before
                == topology.orchestrator.process_start_time_ticks_after
            && is_nonzero_canonical_identity(&topology.orchestrator.proc_directory_device_before)
            && is_nonzero_canonical_identity(&topology.orchestrator.proc_directory_inode_before)
            && topology.orchestrator.proc_directory_device_before
                == topology.orchestrator.proc_directory_device_after
            && topology.orchestrator.proc_directory_inode_before
                == topology.orchestrator.proc_directory_inode_after
            && topology.orchestrator.data_directory_mode == "0700"
            && topology.orchestrator.listener_addresses.is_empty(),
        "orchestrator inventory is not separate, absolute, owner-only, and listener-free"
    );
    require!(
        Path::new(&topology.orchestrator.data_directory).starts_with(&journey.supported_root),
        "orchestrator data directory escaped the supported root"
    );
    require!(
        proc_cmdline_sha256(&topology.orchestrator.argv)?
            == topology.orchestrator.proc_cmdline_sha256,
        "orchestrator /proc cmdline digest does not match its exact argv"
    );
    assert_unprivileged_process(&topology.orchestrator.privileges, "orchestrator")?;

    let mut pids = BTreeSet::new();
    require!(
        pids.insert(topology.orchestrator.pid),
        "orchestrator PID was duplicated"
    );
    let mut node_by_role = BTreeMap::new();
    for node in &topology.nodes {
        require!(
            node.pid > 1 && node.parent_pid == topology.orchestrator.pid && pids.insert(node.pid),
            "node PID was invalid or duplicated for {}",
            node.role
        );
        require!(
            node_by_role.insert(node.role.as_str(), node).is_none(),
            "node role {} appeared more than once",
            node.role
        );
    }
    require!(
        node_by_role.len() == fixture.topology.nodes.len(),
        "node role inventory was incomplete"
    );

    for expected in &fixture.topology.nodes {
        let node = node_by_role
            .get(expected.role.as_str())
            .ok_or_else(|| format!("missing node role {}", expected.role))?;
        let expected_executable = journey.repo_root.join(&expected.binary);
        let expected_asset = pinned_input(&fixture.pinned_inputs, &expected.binary)?;
        require!(
            node.kind == expected.kind
                && node.generated_name == expected.generated_name
                && node.dev_seed == expected.dev_seed
                && node.node_key == expected.node_key
                && node.peer_id == expected.peer_id
                && Path::new(&node.reviewed_source_asset) == expected_executable
                && node.argv.first() == Some(&node.reviewed_source_asset),
            "role/name/key/peer/reviewed-source inventory drifted for {}",
            expected.role
        );
        require!(
            proc_cmdline_sha256(&node.argv)? == node.proc_cmdline_sha256,
            "{} /proc cmdline digest does not match its exact argv",
            expected.role
        );
        assert_exact_node_environment(node, &journey.repo_root)?;
        assert_unprivileged_process(&node.privileges, &expected.role)?;
        require!(
            node.proc_exe_link == "/memfd:cubikan-sealed-exec-v1 (deleted)"
                && node.proc_exe_size == expected_asset.size
                && node.proc_exe_sha256 == expected_asset.sha256
                && node.proc_exe_mode == "0500"
                && node.proc_exe_seals == ["seal", "shrink", "grow", "write"].map(str::to_owned)
                && node.post_seal_write_denied,
            "{} live /proc/exe was not the exact pinned, write-sealed memfd",
            expected.role
        );
        require!(
            node.process_start_time_ticks_before > 0
                && node.process_start_time_ticks_before == node.process_start_time_ticks_after
                && is_nonzero_canonical_identity(&node.proc_directory_device_before)
                && is_nonzero_canonical_identity(&node.proc_directory_inode_before)
                && node.proc_directory_device_before == node.proc_directory_device_after
                && node.proc_directory_inode_before == node.proc_directory_inode_after
                && node.proc_directory_descriptor_device_before
                    == node.proc_directory_device_before
                && node.proc_directory_descriptor_inode_before == node.proc_directory_inode_before
                && node.proc_directory_descriptor_device_after == node.proc_directory_device_after
                && node.proc_directory_descriptor_inode_after == node.proc_directory_inode_after
                && is_nonzero_canonical_identity(&node.executable_device_before)
                && is_nonzero_canonical_identity(&node.executable_inode_before)
                && node.executable_device_before == node.executable_device_after
                && node.executable_inode_before == node.executable_inode_after
                && node.executable_descriptor_device_before == node.executable_device_before
                && node.executable_descriptor_inode_before == node.executable_inode_before
                && node.executable_descriptor_device_after == node.executable_device_after
                && node.executable_descriptor_inode_after == node.executable_inode_after
                && node.authenticated_archive_node_evidence == expected.archive,
            "{} process/executable object was not stable across evidence capture",
            expected.role
        );
        require!(
            node.data_directory_mode == "0700"
                && Path::new(&node.data_directory).is_absolute()
                && Path::new(&node.data_directory).starts_with(&journey.supported_root),
            "{} data root was not owner-only below the supported root",
            expected.role
        );
        let argv_data_directories = flag_values(&node.argv, "--base-path");
        let expected_sides = if expected.relay_side {
            vec!["primary", "relay-side"]
        } else {
            vec!["primary"]
        };
        require!(
            node.data_directories.len() == expected_sides.len()
                && argv_data_directories.len() == expected_sides.len()
                && node
                    .data_directories
                    .iter()
                    .zip(expected_sides)
                    .zip(argv_data_directories)
                    .all(|((directory, side), argv_path)| directory.side == side
                        && directory.path == argv_path
                        && directory.mode == "0700"
                        && is_nonzero_canonical_identity(&directory.device)
                        && is_nonzero_canonical_identity(&directory.inode)
                        && Path::new(&directory.path).is_absolute()
                        && Path::new(&directory.path).starts_with(&journey.supported_root))
                && node.data_directory == node.data_directories[0].path,
            "{} did not prove exact primary/relay-side base-path objects",
            expected.role
        );
        require!(
            node.listener_addresses == expected.listeners,
            "{} normalized listener inventory drifted",
            expected.role
        );
        require!(
            node.argv.iter().all(|argument| {
                !argument.contains("0.0.0.0")
                    && !argument.contains("[::]")
                    && !argument.contains("--rpc-external")
                    && !argument.contains("--unsafe-rpc-external")
                    && !argument.contains("--ws-external")
                    && !argument.contains("--unsafe-ws-external")
                    && !argument.contains("--prometheus-external")
            }),
            "{} retained a wildcard/public bind flag",
            expected.role
        );
        assert_quiet_execution_flags(
            &node.argv,
            if expected.relay_side { 2 } else { 1 },
            &expected.role,
        )?;
        assert_exact_bootnodes(node, expected)?;
        assert_role_ports(node)?;
        if expected.archive {
            require!(
                node.archive_flags
                    == ["--blocks-pruning", "archive", "--state-pruning", "archive",]
                        .map(str::to_owned)
                    && has_adjacent_pair(&node.argv, "--blocks-pruning", "archive")
                    && has_adjacent_pair(&node.argv, "--state-pruning", "archive"),
                "{} lost one or both archive flags",
                expected.role
            );
        } else {
            require!(
                node.archive_flags.is_empty(),
                "relay validator unexpectedly claimed collator archive flags"
            );
        }
        require!(
            is_sha256(&node.primary_runtime_sha256),
            "{} runtime identity is malformed",
            expected.role
        );
        assert_chain_spec_evidence(
            &node.primary_chain_spec,
            &expected.primary_spec_bootnodes,
            &expected.primary_bootnodes,
            &node.primary_runtime_sha256,
            &journey.supported_root,
            &format!("{} primary", expected.role),
        )?;
        require!(
            node.primary_chain_spec.observed_rpc_endpoint.is_none(),
            "{} primary spec claimed a relay-side raw RPC probe",
            expected.role
        );
        if expected.relay_side {
            require!(
                node.relay_side_runtime_sha256
                    .as_deref()
                    .is_some_and(is_sha256),
                "{} relay-side runtime/spec identities are missing",
                expected.role
            );
            let relay_spec = node
                .relay_side_chain_spec
                .as_ref()
                .ok_or_else(|| format!("{} relay-side spec evidence is missing", expected.role))?;
            assert_chain_spec_evidence(
                relay_spec,
                &expected.relay_side_spec_bootnodes,
                &expected.relay_side_bootnodes,
                node.relay_side_runtime_sha256
                    .as_deref()
                    .expect("checked above"),
                &journey.supported_root,
                &format!("{} relay-side", expected.role),
            )?;
            let expected_relay_endpoint = match expected.role.as_str() {
                "collator-a" => "ws://127.0.0.1:9990/",
                "collator-b" => "ws://127.0.0.1:9991/",
                role => return Err(format!("unexpected relay-side role {role}")),
            };
            require!(
                relay_spec.observed_rpc_endpoint.as_deref() == Some(expected_relay_endpoint),
                "{} relay-side spec was not queried through its exact live RPC",
                expected.role
            );
        } else {
            require!(
                node.relay_side_runtime_sha256.is_none() && node.relay_side_chain_spec.is_none(),
                "relay validator reported a second relay side"
            );
        }
    }

    let relay_a = node_by_role["relay-a"];
    let relay_b = node_by_role["relay-b"];
    let collator_a = node_by_role["collator-a"];
    let collator_b = node_by_role["collator-b"];
    let relay_runtime = &relay_a.primary_runtime_sha256;
    let relay_payload = &relay_a.primary_chain_spec.genesis_payload_sha256;
    let relay_genesis = &relay_a.primary_chain_spec.live_genesis_hash;
    require!(
        relay_b.primary_runtime_sha256 == *relay_runtime
            && collator_a.relay_side_runtime_sha256.as_ref() == Some(relay_runtime)
            && collator_b.relay_side_runtime_sha256.as_ref() == Some(relay_runtime)
            && relay_b.primary_chain_spec.genesis_payload_sha256 == *relay_payload
            && relay_b.primary_chain_spec.chain_spec_id == relay_a.primary_chain_spec.chain_spec_id
            && collator_a
                .relay_side_chain_spec
                .as_ref()
                .is_some_and(|spec| {
                    spec.genesis_payload_sha256 == *relay_payload
                        && spec.chain_spec_id == relay_a.primary_chain_spec.chain_spec_id
                })
            && collator_b
                .relay_side_chain_spec
                .as_ref()
                .is_some_and(|spec| {
                    spec.genesis_payload_sha256 == *relay_payload
                        && spec.chain_spec_id == relay_a.primary_chain_spec.chain_spec_id
                })
            && relay_b.primary_chain_spec.live_genesis_hash == *relay_genesis
            && collator_a
                .relay_side_chain_spec
                .as_ref()
                .is_some_and(|spec| spec.live_genesis_hash == *relay_genesis)
            && collator_b
                .relay_side_chain_spec
                .as_ref()
                .is_some_and(|spec| spec.live_genesis_hash == *relay_genesis),
        "relay runtime/canonical genesis identity was not equal across all four relay roles"
    );
    require!(
        relay_a.primary_chain_spec.raw_size == relay_b.primary_chain_spec.raw_size
            && relay_a.primary_chain_spec.raw_sha256 == relay_b.primary_chain_spec.raw_sha256
            && collator_a
                .relay_side_chain_spec
                .as_ref()
                .is_some_and(|spec| {
                    spec.raw_size == relay_a.primary_chain_spec.raw_size
                        && spec.raw_sha256 == relay_a.primary_chain_spec.raw_sha256
                })
            && collator_b
                .relay_side_chain_spec
                .as_ref()
                .is_some_and(|spec| {
                    spec.raw_size == relay_a.primary_chain_spec.raw_size
                        && spec.raw_sha256 == relay_a.primary_chain_spec.raw_sha256
                }),
        "relay whole-file spec bytes were not identical across all four relay roles"
    );
    let cubikan_runtime = pinned_sha256(
        &fixture.pinned_inputs,
        "chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm",
    )?;
    require!(
        collator_a.primary_runtime_sha256 == cubikan_runtime
            && collator_b.primary_runtime_sha256 == cubikan_runtime
            && collator_a.primary_chain_spec.genesis_payload_sha256
                == collator_b.primary_chain_spec.genesis_payload_sha256
            && collator_a.primary_chain_spec.chain_spec_id
                == collator_b.primary_chain_spec.chain_spec_id
            && collator_a.primary_chain_spec.live_genesis_hash
                == collator_b.primary_chain_spec.live_genesis_hash
            && collator_a.primary_chain_spec.raw_size == collator_b.primary_chain_spec.raw_size
            && collator_a.primary_chain_spec.raw_sha256 == collator_b.primary_chain_spec.raw_sha256
            && collator_a.primary_chain_spec.raw_sha256 != relay_a.primary_chain_spec.raw_sha256
            && collator_a.primary_runtime_sha256 != *relay_runtime
            && collator_a.primary_chain_spec.genesis_payload_sha256 != *relay_payload
            && collator_a.primary_chain_spec.live_genesis_hash != *relay_genesis,
        "parachain runtime/canonical genesis was not equal across collators and distinct from relay"
    );

    assert_bootstrap_evidence(journey, collator_a, collator_b)?;

    assert_pvf_worker_evidence(journey, &node_by_role)?;

    let expected_addresses = expected_listener_addresses(fixture);
    require!(
        topology.ss_records.len() == expected_addresses.len(),
        "ss inventory must contain exactly one normalized record per expected listener"
    );
    let mut observed_addresses = BTreeSet::new();
    for record in &topology.ss_records {
        require!(
            observed_addresses.insert(record.address.as_str()),
            "ss inventory duplicated {}",
            record.address
        );
        let expected_pid = expected_pid_for_address(&record.address, fixture, &node_by_role)?;
        require!(
            record.protocol == "tcp"
                && record.pid == expected_pid
                && record.raw_line.contains(&record.address)
                && record.raw_line.contains(&format!("pid={}", record.pid))
                && !record.raw_line.contains("0.0.0.0")
                && !record.raw_line.contains("[::]"),
            "ss record for {} did not prove exact PID ownership and loopback bind",
            record.address
        );
    }
    require!(
        observed_addresses == expected_addresses.iter().map(String::as_str).collect(),
        "ss listener address set drifted"
    );

    let child_pids = topology
        .nodes
        .iter()
        .map(|node| node.pid)
        .collect::<BTreeSet<_>>();
    require!(
        topology.orchestrator.node_child_pids.len() == child_pids.len()
            && topology
                .orchestrator
                .node_child_pids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                == child_pids,
        "orchestrator child PID inventory was not exactly the four nodes"
    );
    require!(
        topology.node_processes.len() == child_pids.len()
            && topology
                .node_processes
                .windows(2)
                .all(|rows| rows[0].pid < rows[1].pid)
            && topology.node_processes.iter().all(|process| {
                topology.nodes.iter().any(|node| {
                    process.pid == node.pid
                        && process.parent_pid == topology.orchestrator.pid
                        && process.start_time_ticks == node.process_start_time_ticks_before
                        && process.proc_exe_link == node.proc_exe_link
                        && process.proc_cmdline_sha256 == node.proc_cmdline_sha256
                })
            }),
        "recursive /proc inventory was not exactly the four orchestrator-owned node generations"
    );
    require!(
        topology.normalizer_mutations.len() == fixture.topology.normalizer_rejection_cases.len(),
        "normalizer mutation evidence length drifted"
    );
    for (observed, expected) in topology
        .normalizer_mutations
        .iter()
        .zip(&fixture.topology.normalizer_rejection_cases)
    {
        require!(
            observed.case == *expected
                && observed.rejected
                && observed.exit_code != 0
                && observed.launched_node_processes == 0
                && Path::new(&observed.executable).is_absolute()
                && (observed.executable == "/usr/bin/bash"
                    || observed.executable
                        == journey
                            .repo_root
                            .join("chain/tools/normalize-node-argv.sh")
                            .to_string_lossy()
                    || observed.executable
                        == journey
                            .repo_root
                            .join("chain/tools/zombienet-node-launcher.sh")
                            .to_string_lossy())
                && is_sha256(&observed.argv_sha256)
                && is_sha256(&observed.environment_sha256)
                && is_sha256(&observed.stdout_sha256)
                && is_sha256(&observed.stderr_sha256)
                && observed.node_processes_before == topology.node_processes
                && observed.node_processes_after == topology.node_processes,
            "normalizer mutation `{expected}` was not rejected before launch"
        );
    }

    assert_pre_mutation_readiness(journey)?;

    let cleanup = &evidence.cleanup;
    let expected_terminated_nodes = topology
        .nodes
        .iter()
        .map(|node| ProcessLifetimeEvidence {
            pid: node.pid,
            start_time_ticks: node.process_start_time_ticks_before,
        })
        .chain(std::iter::once(ProcessLifetimeEvidence {
            pid: evidence.restart.pid_after,
            start_time_ticks: evidence.restart.process_start_time_ticks_after,
        }))
        .collect::<BTreeSet<_>>();
    require!(
        cleanup.node_network_cleanup_complete
            && cleanup.orchestrator_exit_observation == "pending-parent-after-publication"
            && cleanup.work_root_removed
            && cleanup.remaining_node_processes.is_empty()
            && cleanup.remaining_listeners.is_empty()
            && cleanup.terminated_node_processes.len() == expected_terminated_nodes.len()
            && cleanup
                .terminated_node_processes
                .iter()
                .collect::<BTreeSet<_>>()
                == expected_terminated_nodes.iter().collect::<BTreeSet<_>>()
            && cleanup.listener_addresses_released.len() == expected_addresses.len()
            && cleanup
                .listener_addresses_released
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                == expected_addresses.iter().map(String::as_str).collect(),
        "cleanup evidence did not cover every terminated node lifetime and released listener"
    );
    let expected_terminated = expected_terminated_nodes
        .into_iter()
        .chain(std::iter::once(ProcessLifetimeEvidence {
            pid: topology.orchestrator.pid,
            start_time_ticks: topology.orchestrator.process_start_time_ticks_before,
        }))
        .collect::<BTreeSet<_>>();
    for process in &expected_terminated {
        require!(
            !same_process_generation_is_live(process)?,
            "journey process {} generation {} remained live after cleanup",
            process.pid,
            process.start_time_ticks
        );
    }
    for address in &expected_addresses {
        let address = address
            .parse::<SocketAddr>()
            .map_err(|error| format!("fixture listener address is invalid: {error}"))?;
        require!(
            TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_err(),
            "listener {address} remained reachable after cleanup"
        );
    }
    Ok(())
}

fn assert_pre_mutation_readiness(journey: &JourneyRun) -> Result<(), String> {
    let fixture = &journey.fixture.checkpoints.pre_mutation_readiness;
    let readiness = &journey.evidence.checkpoints.pre_mutation;
    let baseline = &readiness.baseline;
    require!(
        readiness.para_id == fixture.para_id
            && readiness.elapsed_milliseconds > 0
            && readiness.elapsed_milliseconds <= fixture.timeout_milliseconds
            && readiness.sample_count >= 2
            && readiness.sample_count
                <= fixture.timeout_milliseconds / fixture.poll_interval_milliseconds + 2
            && readiness.para_registered
            && readiness.relay_endpoints_equal
            && readiness.collator_finalized_endpoints_equal
            && baseline.para_registered
            && baseline.relay_endpoints_equal
            && baseline.scheduler_cores == fixture.expected_scheduler_cores
            && is_sha256(&baseline.active_config_scale_sha256)
            && readiness.scheduler_cores == fixture.expected_scheduler_cores
            && is_sha256(&readiness.active_config_scale_sha256),
        "pre-mutation readiness policy/result binding is malformed"
    );
    let baseline_heads = [
        &baseline.relay_finalized,
        &baseline.collator_a_finalized,
        &baseline.collator_b_finalized,
    ];
    require!(
        baseline_heads
            .iter()
            .all(|head| head.number <= 100_000 && is_chain_hash(&head.hash))
            && baseline.relay_para_header.number <= 100_000
            && is_chain_hash(&baseline.relay_para_header.hash)
            && is_sha256(&baseline.relay_para_header.head_data_sha256)
            && readiness.relay_finalized.number <= 100_000
            && is_chain_hash(&readiness.relay_finalized.hash)
            && readiness.relay_para_header.number <= 100_000
            && is_chain_hash(&readiness.relay_para_header.hash)
            && is_sha256(&readiness.relay_para_header.head_data_sha256)
            && readiness.collator_finalized.number <= 100_000
            && is_chain_hash(&readiness.collator_finalized.hash),
        "pre-mutation baseline or ready head evidence is malformed"
    );
    let relay_progress_minimum = baseline
        .relay_finalized
        .number
        .checked_add(fixture.required_progress_blocks)
        .ok_or_else(|| "pre-mutation relay progress minimum overflowed".to_owned())?;
    let para_progress_minimum = baseline
        .relay_para_header
        .number
        .checked_add(fixture.required_progress_blocks)
        .ok_or_else(|| "pre-mutation para progress minimum overflowed".to_owned())?;
    let collator_progress_minimum = baseline
        .collator_a_finalized
        .number
        .max(baseline.collator_b_finalized.number)
        .checked_add(fixture.required_progress_blocks)
        .ok_or_else(|| "pre-mutation collator progress minimum overflowed".to_owned())?;
    require!(
        readiness.relay_finalized.number >= relay_progress_minimum
            && readiness.relay_para_header.number >= fixture.minimum_finalized_number
            && readiness.relay_para_header.number >= para_progress_minimum
            && readiness.collator_finalized.number >= fixture.minimum_finalized_number
            && readiness.collator_finalized.number >= collator_progress_minimum
            && readiness.relay_para_header.number <= readiness.collator_finalized.number,
        "relay, parachain, or collator did not make the required post-baseline finalized progress"
    );
    require!(
        readiness.historical_block_hashes.collator_a == readiness.relay_para_header.hash
            && readiness.historical_block_hashes.collator_b == readiness.relay_para_header.hash
            && is_chain_hash(&readiness.historical_block_hashes.collator_a)
            && is_chain_hash(&readiness.historical_block_hashes.collator_b),
        "relay parachain head did not bind both collator canonical histories"
    );
    for (role, best) in [
        ("collator-a", &readiness.collator_a_best),
        ("collator-b", &readiness.collator_b_best),
    ] {
        require!(
            best.number >= readiness.collator_finalized.number
                && best.number - readiness.collator_finalized.number == best.finalized_gap
                && best.finalized_gap <= fixture.maximum_best_finalized_gap
                && is_chain_hash(&best.hash)
                && ((best.finalized_gap == 0) == (best.hash == readiness.collator_finalized.hash)),
            "{role} best/finalized readiness gap is malformed or over bound"
        );
    }
    let distinct_authorities = readiness
        .authorities_a
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    require!(
        readiness.authorities_equal
            && readiness.authorities_a == readiness.authorities_b
            && readiness.authorities_a.len() == fixture.expected_distinct_aura_authorities as usize
            && readiness.distinct_authority_count == fixture.expected_distinct_aura_authorities
            && distinct_authorities.len() == fixture.expected_distinct_aura_authorities as usize
            && readiness
                .authorities_a
                .iter()
                .all(|authority| is_chain_hash(authority)),
        "Aura runtime API authority evidence is unequal, duplicated, or malformed"
    );
    let first_submission = journey
        .evidence
        .submissions
        .first()
        .ok_or_else(|| "pre-mutation readiness lacks a following submission".to_owned())?;
    require!(
        first_submission.id == "m01"
            && first_submission.block_number > readiness.collator_finalized.number,
        "m01 was not finalized strictly after the pre-mutation readiness gate"
    );
    Ok(())
}

fn proc_cmdline_sha256(argv: &[String]) -> Result<String, String> {
    require!(!argv.is_empty(), "captured argv cannot be empty");
    let mut bytes = Vec::new();
    for argument in argv {
        require!(
            !argument.as_bytes().contains(&0),
            "captured argv member contains NUL"
        );
        bytes.extend_from_slice(argument.as_bytes());
        bytes.push(0);
    }
    sha256_bytes(&bytes)
}

fn assert_opened_file_read(
    read: &OpenedFileEvidence,
    require_single_link: bool,
    label: &str,
) -> Result<(), String> {
    assert_opened_file_read_with_empty(read, require_single_link, false, label)
}

fn assert_opened_file_read_with_empty(
    read: &OpenedFileEvidence,
    require_single_link: bool,
    allow_empty: bool,
    label: &str,
) -> Result<(), String> {
    let identity = &read.path_before;
    require!(
        is_nonzero_canonical_identity(&identity.device)
            && is_nonzero_canonical_identity(&identity.inode)
            && (allow_empty || identity.size > 0)
            && identity.mode.len() == 4
            && identity
                .mode
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'7'))
            && identity.link_count > 0
            && (0..1_000_000_000).contains(&identity.modified_nanoseconds)
            && (0..1_000_000_000).contains(&identity.changed_nanoseconds)
            && read.path_before == read.descriptor_before
            && read.descriptor_before == read.descriptor_after
            && read.descriptor_after == read.path_after
            && read.bytes_read == identity.size
            && is_sha256(&read.sha256)
            && read.regular_file
            && !read.symbolic_link
            && (!require_single_link || identity.link_count == 1),
        "{label} did not preserve one opened regular-file identity across stat/read/hash/stat"
    );
    Ok(())
}

fn assert_proc_executable_read(read: &OpenedFileEvidence, label: &str) -> Result<(), String> {
    let identity = &read.path_before;
    require!(
        is_nonzero_canonical_identity(&identity.device)
            && is_nonzero_canonical_identity(&identity.inode)
            && identity.size > 0
            && identity.mode.len() == 4
            && identity
                .mode
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'7'))
            && identity.link_count > 0
            && read.path_before == read.descriptor_before
            && read.descriptor_before == read.descriptor_after
            && read.descriptor_after == read.path_after
            && read.bytes_read == identity.size
            && is_sha256(&read.sha256)
            && read.regular_file
            && read.symbolic_link,
        "{label} did not preserve one opened executable identity across read/hash"
    );
    Ok(())
}

fn assert_unprivileged_process(
    privileges: &ProcessPrivilegeEvidence,
    label: &str,
) -> Result<(), String> {
    let expected = BTreeMap::from([
        ("ambient".to_owned(), "0000000000000000".to_owned()),
        ("bounding".to_owned(), "0000000000000000".to_owned()),
        ("effective".to_owned(), "0000000000000000".to_owned()),
        ("inheritable".to_owned(), "0000000000000000".to_owned()),
        ("permitted".to_owned(), "0000000000000000".to_owned()),
    ]);
    require!(
        privileges.no_new_privileges && privileges.capabilities == expected,
        "{label} retained privileges or Linux capabilities"
    );
    Ok(())
}

fn is_namespace_identity(value: &str, kind: &str) -> bool {
    let prefix = format!("{kind}:[");
    value
        .strip_prefix(&prefix)
        .and_then(|inode| inode.strip_suffix(']'))
        .is_some_and(|inode| {
            inode
                .parse::<u64>()
                .is_ok_and(|number| number > 0 && inode == number.to_string())
        })
}

fn is_proc_id_map(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && value.ends_with('\n')
        && !value.contains('\0')
        && value.lines().all(|line| {
            let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
            fields.len() == 3
                && fields.iter().all(|field| field.parse::<u32>().is_ok())
                && fields[2].parse::<u32>().is_ok_and(|length| length > 0)
        })
}

fn assert_pvf_process_security(
    security: &PvfProcessSecurityEvidence,
    fixture: &PvfWorkerFixture,
    label: &str,
) -> Result<(), String> {
    let zero = "0000000000000000";
    let outer_capabilities = BTreeMap::from([
        ("ambient".to_owned(), zero.to_owned()),
        ("bounding".to_owned(), zero.to_owned()),
        ("effective".to_owned(), zero.to_owned()),
        ("inheritable".to_owned(), zero.to_owned()),
        ("permitted".to_owned(), zero.to_owned()),
    ]);
    let nested_capabilities = BTreeMap::from([
        ("ambient".to_owned(), zero.to_owned()),
        (
            "bounding".to_owned(),
            fixture.nested_namespace_capability_mask.clone(),
        ),
        (
            "effective".to_owned(),
            fixture.nested_namespace_capability_mask.clone(),
        ),
        ("inheritable".to_owned(), zero.to_owned()),
        (
            "permitted".to_owned(),
            fixture.nested_namespace_capability_mask.clone(),
        ),
    ]);
    require!(
        security.no_new_privileges
            && security.status_uids.len() == 4
            && security.status_gids.len() == 4
            && security.uid_map_before == security.uid_map_after
            && security.gid_map_before == security.gid_map_after
            && security.user_namespace_before == security.user_namespace_after
            && security.mount_namespace_before == security.mount_namespace_after
            && is_namespace_identity(&security.user_namespace_before, "user")
            && is_namespace_identity(&security.mount_namespace_before, "mnt")
            && is_namespace_identity(&security.orchestrator_user_namespace, "user")
            && is_namespace_identity(&security.orchestrator_mount_namespace, "mnt"),
        "{label} did not preserve canonical PVF security evidence"
    );
    match security.profile.as_str() {
        "outer-zero-capability" => require!(
            security.capabilities == outer_capabilities
                && security.status_uids.iter().all(|id| *id == 0)
                && security.status_gids.iter().all(|id| *id == 0)
                && security.user_namespace_before == security.orchestrator_user_namespace
                && security.mount_namespace_before == security.orchestrator_mount_namespace
                && is_proc_id_map(&security.uid_map_before)
                && is_proc_id_map(&security.gid_map_before),
            "{label} did not retain the exact zero-capability outer profile"
        ),
        "nested-unmapped-user-mount" => require!(
            security.capabilities == nested_capabilities
                && security.status_uids.iter().all(|id| *id == 0)
                && security.status_gids.iter().all(|id| *id == 0)
                && security.user_namespace_before != security.orchestrator_user_namespace
                && security.mount_namespace_before != security.orchestrator_mount_namespace
                && security.uid_map_before.is_empty()
                && security.gid_map_before.is_empty(),
            "{label} did not retain the exact unmapped nested user/mount profile"
        ),
        profile => {
            return Err(format!(
                "{label} used unknown PVF security profile {profile}"
            ));
        }
    }
    Ok(())
}

fn exact_sanitized_environment(repo_root: &Path) -> Result<BTreeMap<String, String>, String> {
    Ok(BTreeMap::from([
        ("HOME".to_owned(), "/home/charles".to_owned()),
        ("LANG".to_owned(), "C".to_owned()),
        ("LC_ALL".to_owned(), "C".to_owned()),
        ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
        (
            "PWD".to_owned(),
            repo_root
                .to_str()
                .ok_or_else(|| "canonical repository root is not UTF-8".to_owned())?
                .to_owned(),
        ),
        ("SHLVL".to_owned(), "0".to_owned()),
        ("TZ".to_owned(), "UTC".to_owned()),
    ]))
}

fn assert_exact_node_environment(node: &NodeEvidence, repo_root: &Path) -> Result<(), String> {
    let expected = exact_sanitized_environment(repo_root)?;
    let mut observed = BTreeMap::new();
    let mut raw = Vec::new();
    for entry in &node.proc_environ_entries {
        require!(
            !entry.is_empty() && !entry.as_bytes().contains(&0),
            "{} retained an empty or NUL-bearing environment entry",
            node.role
        );
        let (name, value) = entry
            .split_once('=')
            .ok_or_else(|| format!("{} environment entry lacks '='", node.role))?;
        require!(
            !name.is_empty() && observed.insert(name.to_owned(), value.to_owned()).is_none(),
            "{} environment contains an empty or duplicate name",
            node.role
        );
        raw.extend_from_slice(entry.as_bytes());
        raw.push(0);
    }
    require!(
        observed == expected && node.environment == expected,
        "{} did not retain exactly the seven normalized environment variables",
        node.role
    );
    require!(
        node.proc_environ_sha256 == sha256_bytes(&raw)?,
        "{} /proc environ digest does not match its exact ordered entries",
        node.role
    );
    Ok(())
}

fn same_process_generation_is_live(process: &ProcessLifetimeEvidence) -> Result<bool, String> {
    let stat_path = PathBuf::from(format!("/proc/{}/stat", process.pid));
    let bytes = match fs::read(&stat_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "cannot inspect cleaned process generation {}: {error}",
                process.pid
            ));
        }
    };
    require!(
        bytes.len() <= 4096 && !bytes.is_empty() && !bytes.contains(&0),
        "cleaned process stat is malformed or over bound"
    );
    let open_parenthesis = bytes
        .windows(2)
        .position(|pair| pair == b" (")
        .ok_or_else(|| "cleaned process stat lacks command opening".to_owned())?;
    require!(
        bytes[..open_parenthesis] == *process.pid.to_string().as_bytes(),
        "cleaned process stat PID mismatch"
    );
    let close_parenthesis = bytes
        .iter()
        .rposition(|byte| *byte == b')')
        .ok_or_else(|| "cleaned process stat lacks command closing".to_owned())?;
    require!(
        close_parenthesis > open_parenthesis + 1 && bytes.get(close_parenthesis + 1) == Some(&b' '),
        "cleaned process stat command framing is malformed"
    );
    let start_time = bytes[close_parenthesis + 2..]
        .split(u8::is_ascii_whitespace)
        .filter(|field| !field.is_empty())
        .nth(19)
        .and_then(|field| std::str::from_utf8(field).ok())
        .and_then(|field| field.parse::<u64>().ok())
        .ok_or_else(|| "cleaned process stat lacks start time".to_owned())?;
    Ok(start_time == process.start_time_ticks)
}

fn flag_values<'a>(argv: &'a [String], flag: &str) -> Vec<&'a str> {
    argv.windows(2)
        .filter(|window| window[0] == flag)
        .map(|window| window[1].as_str())
        .collect()
}

fn has_adjacent_pair(argv: &[String], flag: &str, value: &str) -> bool {
    argv.windows(2)
        .any(|window| window[0] == flag && window[1] == value)
}

fn assert_quiet_execution_flags(
    argv: &[String],
    expected_sides: usize,
    label: &str,
) -> Result<(), String> {
    let separator_indices = argv
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| (argument == "--").then_some(index))
        .collect::<Vec<_>>();
    require!(
        separator_indices.len() + 1 == expected_sides,
        "{label} execution-side separator inventory drifted"
    );
    let sides = if let Some(separator) = separator_indices.first().copied() {
        vec![&argv[..separator], &argv[separator + 1..]]
    } else {
        vec![argv]
    };
    require!(
        sides.iter().all(|side| {
            ["--no-mdns", "--no-telemetry", "--no-hardware-benchmarks"]
                .into_iter()
                .all(|flag| side.iter().filter(|argument| *argument == flag).count() == 1)
        }),
        "{label} did not retain each closed no-network/benchmark flag exactly once per execution side"
    );
    Ok(())
}

fn assert_role_ports(node: &NodeEvidence) -> Result<(), String> {
    let (rpc, p2p, metrics): (&[&str], &[&str], &[&str]) = match node.role.as_str() {
        "relay-a" => (&["9944"], &["30333"], &["9615"]),
        "relay-b" => (&["9945"], &["30334"], &["9616"]),
        "collator-a" => (&["9988", "9990"], &["30335", "30337"], &["9617", "9619"]),
        "collator-b" => (&["9989", "9991"], &["30336", "30338"], &["9618", "9620"]),
        role => return Err(format!("unknown live role {role}")),
    };
    require!(
        flag_values(&node.argv, "--experimental-rpc-endpoint")
            == rpc
                .iter()
                .map(|port| { format!("listen-addr=127.0.0.1:{port},methods=unsafe,cors=all") })
                .collect::<Vec<_>>()
            && flag_values(&node.argv, "--rpc-port").is_empty()
            && flag_values(&node.argv, "--ws-port").is_empty()
            && flag_values(&node.argv, "--rpc-cors").is_empty()
            && flag_values(&node.argv, "--rpc-methods").is_empty()
            && flag_values(&node.argv, "--port").is_empty()
            && flag_values(&node.argv, "--prometheus-port") == metrics
            && flag_values(&node.argv, "--listen-addr")
                == p2p
                    .iter()
                    .map(|port| format!("/ip4/127.0.0.1/tcp/{port}/ws"))
                    .collect::<Vec<_>>(),
        "{} argv did not contain its exact normalized ports",
        node.role
    );
    Ok(())
}

fn assert_exact_bootnodes(node: &NodeEvidence, expected: &NodeFixture) -> Result<(), String> {
    require!(
        !expected.primary_bootnodes.is_empty()
            && node.primary_bootnodes == expected.primary_bootnodes
            && node.relay_side_bootnodes == expected.relay_side_bootnodes,
        "{} bootnode evidence differs from the exact role-specific fixture",
        expected.role
    );
    let separators = node
        .argv
        .iter()
        .enumerate()
        .filter(|(_, argument)| argument.as_str() == "--")
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let (primary_argv, relay_argv) = if expected.relay_side {
        require!(
            separators.len() == 1,
            "{} must have exactly one relay-side separator",
            expected.role
        );
        (
            &node.argv[..separators[0]],
            Some(&node.argv[separators[0] + 1..]),
        )
    } else {
        require!(
            separators.is_empty(),
            "relay validator {} must not have a relay-side separator",
            expected.role
        );
        (node.argv.as_slice(), None)
    };
    require!(
        flag_values(primary_argv, "--name") == [expected.generated_name.as_str()]
            && flag_values(primary_argv, "--node-key") == [expected.node_key.as_str()]
            && flag_values(primary_argv, "--bootnodes")
                == expected
                    .primary_bootnodes
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
        "{} primary argv lacks its exact name, node key, or one bootnode flag",
        expected.role
    );
    match relay_argv {
        Some(relay_argv) => require!(
            !expected.relay_side_bootnodes.is_empty()
                && flag_values(relay_argv, "--bootnodes")
                    == expected
                        .relay_side_bootnodes
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
            "{} relay-side argv lacks its exact one bootnode flag",
            expected.role
        ),
        None => require!(
            expected.relay_side_bootnodes.is_empty(),
            "relay validator unexpectedly has relay-side bootnodes"
        ),
    }
    Ok(())
}

fn assert_chain_spec_evidence(
    spec: &ChainSpecEvidence,
    expected_spec_bootnodes: &[String],
    expected_cli_bootnodes: &[String],
    expected_runtime_sha256: &str,
    supported_root: &Path,
    label: &str,
) -> Result<(), String> {
    require!(
        Path::new(&spec.raw_path).is_absolute()
            && Path::new(&spec.raw_path).starts_with(supported_root)
            && spec.raw_size > 0
            && spec.raw_size <= MAX_RAW_CHAIN_SPEC_BYTES
            && is_sha256(&spec.raw_sha256)
            && !spec.chain_spec_id.is_empty()
            && spec.chain_spec_id.len() <= 128
            && spec
                .chain_spec_id
                .bytes()
                .enumerate()
                .all(|(index, byte)| if index == 0 {
                    byte.is_ascii_alphanumeric()
                } else {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                })
            && spec.duplicate_keys_rejected
            && spec.top_level_bootnodes == expected_spec_bootnodes
            && spec.removed_top_level_members == ["bootNodes"]
            && spec.canonicalization
                == "remove-one-top-level-bootNodes-recursive-key-sort-compact-json-v1"
            && is_sha256(&spec.genesis_payload_sha256)
            && is_chain_hash(&spec.live_genesis_hash)
            && spec.live_runtime_code_sha256 == expected_runtime_sha256,
        "{label} chain-spec evidence is malformed or escaped the supported root"
    );
    let mut effective = Vec::new();
    for address in expected_spec_bootnodes.iter().chain(expected_cli_bootnodes) {
        require!(
            address.starts_with("/ip4/127.0.0.1/tcp/")
                && address.contains("/ws/p2p/")
                && !address.as_bytes().contains(&0),
            "{label} chain spec or CLI contains a non-loopback/invalid bootnode"
        );
        if !effective.contains(address) {
            effective.push(address.clone());
        }
    }
    require!(
        effective.len() == 1,
        "{label} effective chain-spec plus CLI bootnodes were empty or had extras"
    );
    require!(
        expected_spec_bootnodes
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            == expected_spec_bootnodes.len()
            && expected_cli_bootnodes.iter().collect::<BTreeSet<_>>().len()
                == expected_cli_bootnodes.len(),
        "{label} chain-spec or CLI bootnode inventory contained duplicates"
    );
    Ok(())
}

fn node_argv_segment<'a>(node: &'a NodeEvidence, side: &str) -> Result<&'a [String], String> {
    let separators = node
        .argv
        .iter()
        .enumerate()
        .filter(|(_, argument)| argument.as_str() == "--")
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    match (side, separators.as_slice()) {
        ("primary", []) => Ok(&node.argv),
        ("primary", [separator]) => Ok(&node.argv[..*separator]),
        ("relay-side", [separator]) => Ok(&node.argv[*separator + 1..]),
        _ => Err(format!("{} has no exact {side} argv segment", node.role)),
    }
}

fn is_ascii_alphanumeric_suffix(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn assert_pvf_worker_evidence(
    journey: &JourneyRun,
    node_by_role: &BTreeMap<&str, &NodeEvidence>,
) -> Result<(), String> {
    let fixture = &journey.fixture.pvf_workers;
    let evidence = &journey.evidence.pvf_workers;
    let owner_uid = fs::metadata(&journey.supported_root)
        .map_err(|error| format!("cannot stat supported-root owner for PVF evidence: {error}"))?
        .uid();
    require!(
        evidence.root == fixture.root
            && evidence.filesystem_magic == fixture.filesystem_magic
            && evidence.write_probe_errno == fixture.write_probe_errno
            && evidence.root_owner_uid == owner_uid
            && is_nonzero_canonical_identity(&evidence.root_identity.device)
            && is_nonzero_canonical_identity(&evidence.root_identity.inode)
            && evidence.root_identity.mode == fixture.directory_mode
            && evidence.root_identity.size > 0
            && evidence.root_identity.link_count > 0
            && (0..1_000_000_000).contains(&evidence.root_identity.modified_nanoseconds)
            && (0..1_000_000_000).contains(&evidence.root_identity.changed_nanoseconds),
        "PVF root identity/mode/filesystem/owner/write denial drifted"
    );

    let mount = &evidence.mount;
    let fields = mount
        .raw_mountinfo_line
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
    let separator = fields
        .iter()
        .position(|field| *field == "-")
        .ok_or_else(|| "PVF mountinfo evidence lacks separator".to_owned())?;
    let mut raw_mount_options = fields[5].split(',').collect::<Vec<_>>();
    raw_mount_options.sort_unstable();
    let mut raw_super_options = fields[separator + 3].split(',').collect::<Vec<_>>();
    raw_super_options.sort_unstable();
    require!(
        separator >= 6
            && fields.len() == separator + 4
            && fields[0].parse::<u64>().ok() == Some(mount.mount_id)
            && fields[1].parse::<u64>().ok() == Some(mount.parent_id)
            && fields[2] == mount.major_minor
            && fields[3] == mount.root
            && fields[4] == mount.mount_point
            && raw_mount_options
                == mount
                    .mount_options
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
            && fields[6..separator] == *mount.optional_fields
            && fields[separator + 1] == mount.filesystem_type
            && fields[separator + 2] == mount.mount_source
            && raw_super_options
                == mount
                    .super_options
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
            && mount.mount_id > 0
            && mount.parent_id > 0
            && mount
                .major_minor
                .split_once(':')
                .is_some_and(|(major, minor)| {
                    !major.is_empty()
                        && !minor.is_empty()
                        && major.bytes().all(|byte| byte.is_ascii_digit())
                        && minor.bytes().all(|byte| byte.is_ascii_digit())
                })
            && mount.root != "/"
            && mount.mount_point == fixture.root
            && mount.filesystem_type == "tmpfs"
            && mount.bind
            && mount.read_only
            && mount.nodev
            && mount.nosuid
            && mount.executable
            && mount.mount_options.windows(2).all(|pair| pair[0] < pair[1])
            && mount.super_options.windows(2).all(|pair| pair[0] < pair[1])
            && mount.mount_options.iter().any(|option| option == "ro")
            && mount.mount_options.iter().any(|option| option == "nodev")
            && mount.mount_options.iter().any(|option| option == "nosuid")
            && !mount
                .mount_options
                .iter()
                .any(|option| option == "rw" || option == "noexec"),
        "PVF mount evidence is not the exact independently parsed read-only executable nodev/nosuid tmpfs bind"
    );

    require!(
        evidence.assets.len() == fixture.assets.len(),
        "PVF materialized asset inventory length drifted"
    );
    for (observed, expected) in evidence.assets.iter().zip(&fixture.assets) {
        let cached = pinned_input(&journey.fixture.pinned_inputs, &expected.cache_path)?;
        require!(
            observed.name == expected.name
                && observed.path == expected.materialized_path
                && observed.owner_uid == owner_uid
                && observed.read.path_before.mode == expected.mode
                && observed.read.path_before.size == expected.size
                && observed.read.sha256 == expected.sha256
                && cached.size == expected.size
                && cached.sha256 == expected.sha256,
            "PVF materialized/cache asset identity drifted for {}",
            expected.name
        );
        assert_opened_file_read(
            &observed.read,
            true,
            &format!("PVF worker asset {}", expected.name),
        )?;
    }

    require!(
        evidence.execution_sides.len() == fixture.execution_sides.len(),
        "PVF execution-side inventory length drifted"
    );
    let assert_segment = |node: &NodeEvidence, side: &str, expected: bool| {
        let segment = node_argv_segment(node, side)?;
        let workers = flag_values(segment, &fixture.workers_path_flag);
        let database = flag_values(segment, "--database");
        let execute = flag_values(segment, "--execute-workers-max-num");
        let prepare_soft = flag_values(segment, "--prepare-workers-soft-max-num");
        let prepare_hard = flag_values(segment, "--prepare-workers-hard-max-num");
        if expected {
            require!(
                workers == [fixture.root.as_str()]
                    && database.is_empty()
                    && execute == ["1"]
                    && prepare_soft == ["1"]
                    && prepare_hard == ["1"],
                "{}.{} lacks one exact PVF workers path/pool-cap tuple",
                node.role,
                side
            );
        } else {
            require!(
                workers.is_empty()
                    && database.is_empty()
                    && execute.is_empty()
                    && prepare_soft.is_empty()
                    && prepare_hard.is_empty(),
                "{}.{} unexpectedly contains PVF worker flags",
                node.role,
                side
            );
        }
        Ok::<(), String>(())
    };
    for (observed, contract) in evidence
        .execution_sides
        .iter()
        .zip(&fixture.execution_sides)
    {
        let (role, side) = contract
            .split_once('.')
            .ok_or_else(|| format!("malformed PVF execution-side fixture {contract}"))?;
        let node = node_by_role
            .get(role)
            .ok_or_else(|| format!("PVF execution side has unknown role {role}"))?;
        require!(
            observed.role == role
                && observed.side == side
                && observed.workers_path_values == [fixture.root.as_str()]
                && observed.database_values.is_empty()
                && observed.execute_workers_max_num_values == ["1"]
                && observed.prepare_workers_soft_max_num_values == ["1"]
                && observed.prepare_workers_hard_max_num_values == ["1"],
            "PVF execution-side evidence drifted for {contract}"
        );
        assert_segment(node, side, true)?;
    }
    assert_segment(node_by_role["collator-a"], "primary", false)?;
    assert_segment(node_by_role["collator-b"], "primary", false)?;

    let monitor = &evidence.runtime_monitor;
    let expected_role_kind_keys = fixture
        .execution_sides
        .iter()
        .flat_map(|side| {
            let role = side.split_once('.').expect("fixture validated").0;
            [format!("{role}.execute"), format!("{role}.prepare")]
        })
        .collect::<BTreeSet<_>>();
    require!(
        monitor.interval_milliseconds == fixture.monitor_interval_milliseconds
            && monitor.sample_count > 0
            && monitor.maximum_total_workers > 0
            && monitor.maximum_total_workers <= fixture.maximum_total_worker_processes
            && monitor.maximum_prepare_workers > 0
            && monitor.maximum_prepare_workers <= monitor.maximum_total_workers
            && monitor.maximum_execute_workers > 0
            && monitor.maximum_execute_workers <= monitor.maximum_total_workers
            && monitor
                .maximum_by_role_kind
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
                == expected_role_kind_keys
            && monitor
                .maximum_by_role_kind
                .values()
                .all(|count| { *count <= fixture.maximum_supervisors_per_role_kind })
            && monitor.observed_generations.len() <= fixture.maximum_observed_generations as usize
            && monitor.observed_host_paths.len() <= fixture.maximum_observed_host_paths as usize
            && monitor.observed_unix_sockets.len()
                <= fixture.maximum_observed_unix_sockets as usize
            && !monitor.overflowed,
        "PVF runtime monitor bounds/counters drifted"
    );

    let mut parent_pids = BTreeMap::<&str, BTreeSet<u32>>::new();
    for (role, node) in node_by_role {
        parent_pids.entry(role).or_default().insert(node.pid);
    }
    parent_pids
        .entry("collator-b")
        .or_default()
        .insert(journey.evidence.restart.pid_after);
    let mut generations_by_pid = BTreeMap::new();
    let mut kinds = BTreeSet::new();
    for generation in &monitor.observed_generations {
        require!(
            generations_by_pid
                .insert(generation.pid, generation)
                .is_none(),
            "PVF monitor reused PID {} across retained generations",
            generation.pid
        );
        kinds.insert(generation.kind.as_str());
    }
    require!(
        kinds == BTreeSet::from(["execute", "prepare"]),
        "PVF monitor did not observe both exact worker kinds"
    );
    require!(
        monitor
            .observed_generations
            .iter()
            .any(|generation| { generation.security.profile == "nested-unmapped-user-mount" }),
        "PVF monitor did not observe the pinned nested user/mount sandbox"
    );

    for generation in &monitor.observed_generations {
        let node = node_by_role
            .get(generation.role.as_str())
            .ok_or_else(|| format!("PVF generation has unknown role {}", generation.role))?;
        let (data_side, spec) = if generation.role.starts_with("collator") {
            (
                "relay-side",
                node.relay_side_chain_spec.as_ref().ok_or_else(|| {
                    format!("{} lacks relay-side spec for PVF path", generation.role)
                })?,
            )
        } else {
            ("primary", &node.primary_chain_spec)
        };
        let data_root = node
            .data_directories
            .iter()
            .find(|directory| directory.side == data_side)
            .ok_or_else(|| format!("{} lacks {data_side} PVF base path", generation.role))?;
        let database = Path::new(&data_root.path)
            .join("chains")
            .join(&spec.chain_spec_id)
            .join(&fixture.database_path_components[0])
            .join(&fixture.database_path_components[1]);
        let artifacts = database.join("pvf-artifacts");
        let expected_asset = fixture
            .assets
            .iter()
            .find(|asset| asset.name == format!("polkadot-{}-worker", generation.kind))
            .ok_or_else(|| format!("unknown PVF worker kind {}", generation.kind))?;
        let expected_comm = expected_asset.name.chars().take(15).collect::<String>();
        let worker_name = Path::new(&generation.worker_directory)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "PVF worker directory basename is not UTF-8".to_owned())?;
        let worker_suffix = worker_name
            .strip_prefix(&format!("worker-dir-{}-", generation.kind))
            .ok_or_else(|| format!("PVF worker directory has wrong kind: {worker_name}"))?;
        let socket_suffix = generation
            .socket_path
            .strip_prefix(&format!("/tmp/pvf-host-{}-", generation.kind))
            .ok_or_else(|| format!("PVF socket path has wrong kind: {}", generation.socket_path))?;
        assert_pvf_process_security(
            &generation.security,
            fixture,
            &format!("PVF worker {}", generation.pid),
        )?;
        require!(
            generation.pid > 1
                && generation.parent_pid > 0
                && generation.start_time_ticks > 0
                && matches!(generation.process_class.as_str(), "supervisor" | "job")
                && matches!(generation.kind.as_str(), "prepare" | "execute")
                && generation.proc_exe_link == expected_asset.materialized_path
                && generation.proc_comm == expected_comm
                && generation.database_path == database.to_string_lossy()
                && generation.artifacts_cache_path == artifacts.to_string_lossy()
                && Path::new(&generation.worker_directory).parent() == Some(artifacts.as_path())
                && is_ascii_alphanumeric_suffix(worker_suffix)
                && socket_suffix.len() == 10
                && is_ascii_alphanumeric_suffix(socket_suffix)
                && generation.argv
                    == [
                        generation.proc_exe_link.as_str(),
                        format!("{}-worker", generation.kind).as_str(),
                        "--node-impl-version",
                        fixture.node_impl_version.as_str(),
                        "--socket-path",
                        generation.socket_path.as_str(),
                        "--worker-dir-path",
                        generation.worker_directory.as_str(),
                    ]
                    .map(str::to_owned)
                && proc_cmdline_sha256(&generation.argv)? == generation.proc_cmdline_sha256
                && generation.environment.len() == 1
                && generation.environment.contains_key("RUST_LOG")
                && generation.environment["RUST_LOG"].len() <= 4096
                && generation.proc_environ_entries
                    == [format!("RUST_LOG={}", generation.environment["RUST_LOG"])]
                && {
                    let mut bytes = generation.proc_environ_entries[0].as_bytes().to_vec();
                    bytes.push(0);
                    sha256_bytes(&bytes)? == generation.proc_environ_sha256
                }
                && is_nonzero_canonical_identity(&generation.proc_directory_device_before)
                && is_nonzero_canonical_identity(&generation.proc_directory_inode_before)
                && generation.proc_directory_device_before
                    == generation.proc_directory_device_after
                && generation.proc_directory_inode_before == generation.proc_directory_inode_after
                && generation.proc_directory_descriptor_device_before
                    == generation.proc_directory_device_before
                && generation.proc_directory_descriptor_inode_before
                    == generation.proc_directory_inode_before
                && generation.proc_directory_descriptor_device_after
                    == generation.proc_directory_device_after
                && generation.proc_directory_descriptor_inode_after
                    == generation.proc_directory_inode_after
                && is_nonzero_canonical_identity(&generation.executable_device_before)
                && is_nonzero_canonical_identity(&generation.executable_inode_before)
                && generation.executable_device_before == generation.executable_device_after
                && generation.executable_inode_before == generation.executable_inode_after
                && generation.executable_descriptor_device_before
                    == generation.executable_device_before
                && generation.executable_descriptor_inode_before
                    == generation.executable_inode_before
                && generation.executable_descriptor_device_after
                    == generation.executable_device_after
                && generation.executable_descriptor_inode_after
                    == generation.executable_inode_after,
            "PVF generation {}:{}:{} failed exact argv/env/path/object validation",
            generation.role,
            generation.kind,
            generation.pid
        );
        if generation.process_class == "supervisor" {
            require!(
                generation.supervisor_pid == generation.pid
                    && parent_pids
                        .get(generation.role.as_str())
                        .is_some_and(|pids| pids.contains(&generation.parent_pid)),
                "PVF supervisor did not belong to its exact node generation"
            );
        } else {
            require!(
                generation.supervisor_pid != generation.pid,
                "PVF job process claimed itself as supervisor"
            );
            let mut parent = generation.parent_pid;
            let mut visited = BTreeSet::new();
            while parent != generation.supervisor_pid {
                require!(visited.insert(parent), "PVF job ancestry contained a cycle");
                let ancestor = generations_by_pid.get(&parent).ok_or_else(|| {
                    format!("PVF job {} lacks retained parent {parent}", generation.pid)
                })?;
                require!(
                    ancestor.role == generation.role && ancestor.kind == generation.kind,
                    "PVF job ancestry crossed a role or worker kind"
                );
                parent = ancestor.parent_pid;
            }
            require!(
                generations_by_pid
                    .get(&generation.supervisor_pid)
                    .is_some_and(|supervisor| supervisor.process_class == "supervisor"
                        && supervisor.role == generation.role
                        && supervisor.kind == generation.kind),
                "PVF job did not resolve to one retained supervisor"
            );
        }
    }

    let supervisors = monitor
        .observed_generations
        .iter()
        .filter(|generation| generation.process_class == "supervisor")
        .collect::<Vec<_>>();
    require!(
        !supervisors.is_empty(),
        "PVF monitor observed no supervisor"
    );
    for (key, maximum) in &monitor.maximum_by_role_kind {
        let (role, kind) = key
            .split_once('.')
            .ok_or_else(|| format!("malformed PVF role/kind maximum key {key}"))?;
        let observed = supervisors
            .iter()
            .any(|generation| generation.role == role && generation.kind == kind);
        require!(
            *maximum == u64::from(observed),
            "PVF role/kind maximum {key} was not derived from retained generations"
        );
    }
    let expected_supervisor_keys = supervisors
        .iter()
        .map(|generation| format!("{}.{}.{}", generation.role, generation.kind, generation.pid))
        .collect::<BTreeSet<_>>();
    require!(
        monitor
            .maximum_job_children_by_supervisor
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            == expected_supervisor_keys
            && monitor
                .maximum_job_children_by_supervisor
                .values()
                .all(|count| *count <= fixture.maximum_job_children_per_supervisor),
        "PVF subordinate worker maximum inventory drifted"
    );
    for supervisor in supervisors {
        let key = format!("{}.{}.{}", supervisor.role, supervisor.kind, supervisor.pid);
        let observed_job = monitor.observed_generations.iter().any(|generation| {
            generation.process_class == "job" && generation.supervisor_pid == supervisor.pid
        });
        require!(
            monitor.maximum_job_children_by_supervisor[&key] >= u64::from(observed_job),
            "PVF subordinate maximum did not cover retained jobs for {key}"
        );
    }

    require!(
        !monitor.observed_unix_sockets.is_empty(),
        "PVF monitor retained no live Unix-socket evidence"
    );
    for host in &monitor.observed_host_paths {
        let relative = host
            .path
            .strip_prefix(&fixture.host_path_prefix)
            .ok_or_else(|| format!("PVF host path escaped exact prefix: {}", host.path))?;
        let (root_suffix, _) = relative.split_once('/').unwrap_or((relative, ""));
        let suffix = root_suffix
            .strip_prefix("prepare-")
            .or_else(|| root_suffix.strip_prefix("execute-"))
            .ok_or_else(|| format!("PVF host root has wrong kind: {}", host.path))?;
        let descriptor_expected = matches!(host.file_type.as_str(), "directory" | "regular");
        require!(
            suffix.len() == 10
                && is_ascii_alphanumeric_suffix(suffix)
                && !host.path.contains("/../")
                && !host.path.contains("/./")
                && matches!(host.file_type.as_str(), "directory" | "regular" | "socket")
                && host.owner_uid == owner_uid
                && is_nonzero_canonical_identity(&host.device)
                && is_nonzero_canonical_identity(&host.inode)
                && host.device == host.device_after
                && host.inode == host.inode_after
                && host.mode == host.mode_after
                && host.mode.len() == 4
                && host.mode.bytes().all(|byte| matches!(byte, b'0'..=b'7'))
                && (host.descriptor_device_before.is_some() == descriptor_expected)
                && (host.descriptor_inode_before.is_some() == descriptor_expected)
                && (host.descriptor_device_after.is_some() == descriptor_expected)
                && (host.descriptor_inode_after.is_some() == descriptor_expected)
                && (!descriptor_expected
                    || (host.descriptor_device_before.as_ref() == Some(&host.device)
                        && host.descriptor_inode_before.as_ref() == Some(&host.inode)
                        && host.descriptor_device_after.as_ref() == Some(&host.device_after)
                        && host.descriptor_inode_after.as_ref() == Some(&host.inode_after))),
            "PVF host path {} failed stable no-follow identity validation",
            host.path
        );
    }
    for socket in &monitor.observed_unix_sockets {
        let suffix = socket
            .path
            .strip_prefix("/tmp/pvf-host-prepare-")
            .or_else(|| socket.path.strip_prefix("/tmp/pvf-host-execute-"))
            .ok_or_else(|| format!("PVF Unix socket path has wrong prefix: {}", socket.path))?;
        let fields = socket.raw_line.split_ascii_whitespace().collect::<Vec<_>>();
        require!(
            suffix.len() == 10
                && is_ascii_alphanumeric_suffix(suffix)
                && socket.socket_type == "0001"
                && socket.state.len() == 2
                && socket.state.bytes().all(|byte| byte.is_ascii_hexdigit())
                && is_nonzero_canonical_identity(&socket.inode)
                && fields.len() == 8
                && fields[4] == socket.socket_type
                && fields[5] == socket.state
                && fields[6] == socket.inode
                && fields[7] == socket.path
                && monitor
                    .observed_generations
                    .iter()
                    .any(|generation| generation.socket_path == socket.path),
            "PVF Unix socket {} failed exact /proc/path identity validation",
            socket.path
        );
    }
    require!(
        monitor
            .observed_generations
            .iter()
            .filter(|generation| generation.process_class == "supervisor")
            .all(|generation| monitor
                .observed_unix_sockets
                .iter()
                .any(|socket| socket.path == generation.socket_path)),
        "a retained PVF supervisor lacked its concurrently sampled Unix socket"
    );
    require!(
        monitor.post_stop_processes.is_empty()
            && monitor.post_stop_host_paths.is_empty()
            && monitor.post_stop_unix_sockets.is_empty(),
        "PVF worker process, host path, or Unix socket remained after teardown"
    );
    let root = Path::new(&fixture.root);
    require!(
        !root.exists()
            && fs::symlink_metadata(root)
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "PVF worker materialization root remains after launcher cleanup"
    );
    Ok(())
}

fn expected_listener_addresses(fixture: &JourneyFixture) -> Vec<String> {
    fixture
        .topology
        .nodes
        .iter()
        .flat_map(|node| node.listeners.iter().cloned())
        .collect()
}

fn expected_pid_for_address(
    address: &str,
    fixture: &JourneyFixture,
    nodes: &BTreeMap<&str, &NodeEvidence>,
) -> Result<u32, String> {
    let role = fixture
        .topology
        .nodes
        .iter()
        .find(|node| node.listeners.iter().any(|listener| listener == address))
        .map(|node| node.role.as_str())
        .ok_or_else(|| format!("ss evidence contains unexpected address {address}"))?;
    nodes
        .get(role)
        .map(|node| node.pid)
        .ok_or_else(|| format!("ss evidence address {address} has no live role"))
}

fn pinned_sha256<'a>(pins: &'a [PinnedInput], path: &str) -> Result<&'a str, String> {
    pinned_input(pins, path).map(|pin| pin.sha256.as_str())
}

fn pinned_input<'a>(pins: &'a [PinnedInput], path: &str) -> Result<&'a PinnedInput, String> {
    pins.iter()
        .find(|pin| pin.path == path)
        .ok_or_else(|| format!("missing independent pin for {path}"))
}

fn expected_accepted_payload(
    mutation: &MutationFixture,
    lifecycle_phases: &mut BTreeMap<String, String>,
) -> Result<Value, String> {
    let operation = mutation
        .request
        .get("operation")
        .ok_or_else(|| format!("mutation {} lacks operation", mutation.id))?;
    let required = |name: &str| {
        operation
            .get(name)
            .cloned()
            .ok_or_else(|| format!("mutation {} lacks {name}", mutation.id))
    };
    match mutation.operation.as_str() {
        "create_intent_unit" => {
            let unit = operation
                .get("intent_unit")
                .ok_or_else(|| format!("mutation {} lacks intent_unit", mutation.id))?;
            let workflow = operation
                .get("workflow")
                .ok_or_else(|| format!("mutation {} lacks workflow", mutation.id))?;
            let id = unit
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("mutation {} lacks unit id", mutation.id))?;
            let initial = workflow
                .get("initial_phase")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("mutation {} lacks initial phase", mutation.id))?;
            lifecycle_phases.insert(id.to_owned(), initial.to_owned());
            Ok(serde_json::json!({
                "command_schema_version": 1,
                "id": id,
                "origin": unit.get("origin").cloned().ok_or_else(|| format!("mutation {} lacks origin", mutation.id))?,
                "species": unit.get("species").cloned().ok_or_else(|| format!("mutation {} lacks species", mutation.id))?,
                "workflow": workflow,
            }))
        }
        "transition_intent_unit" => {
            let id = operation
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("mutation {} lacks unit id", mutation.id))?;
            let target = operation
                .get("target")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("mutation {} lacks target", mutation.id))?;
            let expected_revision = operation
                .get("expected_revision")
                .and_then(Value::as_str)
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or_else(|| format!("mutation {} lacks expected revision", mutation.id))?;
            let from = lifecycle_phases
                .insert(id.to_owned(), target.to_owned())
                .ok_or_else(|| format!("mutation {} has no prior phase", mutation.id))?;
            Ok(serde_json::json!({
                "unit_id": id,
                "committed_revision": expected_revision.checked_add(1).ok_or_else(|| format!("mutation {} revision overflowed", mutation.id))?.to_string(),
                "from": from,
                "to": target,
            }))
        }
        "complete_intent_unit" => {
            let id = operation
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("mutation {} lacks unit id", mutation.id))?;
            let expected_revision = operation
                .get("expected_revision")
                .and_then(Value::as_str)
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or_else(|| format!("mutation {} lacks expected revision", mutation.id))?;
            let phase = lifecycle_phases
                .get(id)
                .ok_or_else(|| format!("mutation {} has no completion phase", mutation.id))?;
            Ok(serde_json::json!({
                "unit_id": id,
                "committed_revision": expected_revision.checked_add(1).ok_or_else(|| format!("mutation {} revision overflowed", mutation.id))?.to_string(),
                "phase": phase,
            }))
        }
        "create_relationship_definition" => Ok(serde_json::json!({
            "definition": required("definition")?,
            "direction": "directed",
            "source_species": operation.get("source_species").cloned().unwrap_or(Value::Null),
            "target_species": operation.get("target_species").cloned().unwrap_or(Value::Null),
            "self_policy": required("self_policy")?,
            "cycle_policy": required("cycle_policy")?,
        })),
        "create_relationship" | "delete_relationship" => {
            Ok(serde_json::json!({ "relationship": required("relationship")? }))
        }
        "record_association" | "revoke_association" => {
            Ok(serde_json::json!({ "association": required("association")? }))
        }
        operation => Err(format!("unmapped mutation operation {operation}")),
    }
}

fn assert_e2(journey: &JourneyRun) -> Result<(), String> {
    let fixture = &journey.fixture;
    let evidence = &journey.evidence;
    assert_pre_mutation_readiness(journey)?;
    require!(
        evidence.submissions.len() == fixture.mutations.len(),
        "submission evidence length differs from the exact mutation fixture"
    );
    let mut coordinates = BTreeSet::new();
    let mut previous_coordinate = None;
    let mut deployment_anchor = None;
    let mut lifecycle_phases = BTreeMap::new();
    let local_binary = &evidence.runtime_executables.local_binary.path;
    let uninterrupted_database = &snapshot(journey, "uninterrupted")?.projection_path;
    let expected_environment = exact_sanitized_environment(&journey.repo_root)?;
    let expected_environment_sha256 = canonical_json_sha256(&expected_environment)?;
    for (index, (submission, mutation)) in evidence
        .submissions
        .iter()
        .zip(&fixture.mutations)
        .enumerate()
    {
        let expected_payload = expected_accepted_payload(mutation, &mut lifecycle_phases)?;
        require!(
            submission.id == mutation.id
                && submission.phase == mutation.phase
                && submission.signer == mutation.signer
                && submission.endpoint == mutation.endpoint
                && submission.work_category == mutation.work_category
                && submission.operation == mutation.operation,
            "submission metadata drifted for {}",
            mutation.id
        );
        let expected_rpc_url = fixture.endpoints.get(&mutation.endpoint).ok_or_else(|| {
            format!(
                "submission {} fixture names an unknown endpoint {}",
                mutation.id, mutation.endpoint
            )
        })?;
        let expected_argv = vec![
            local_binary.clone(),
            "--database".to_owned(),
            uninterrupted_database.clone(),
            "--rpc".to_owned(),
            expected_rpc_url.clone(),
            "--dev-signer".to_owned(),
            mutation.signer.clone(),
        ];
        require!(
            submission.executable == *local_binary
                && submission.argv == expected_argv
                && submission.environment == expected_environment
                && submission.environment_sha256 == expected_environment_sha256,
            "submission {} did not execute the exact binary/database/RPC/signer/environment contract",
            mutation.id
        );
        let request_bytes = canonical_json_bytes(&mutation.request)?;
        require!(
            submission.request_sha256 == sha256_bytes(&request_bytes)?,
            "submission {} did not use the exact independent request bytes",
            mutation.id
        );
        assert_raw_cli_json_output(
            &submission.response_body_hex,
            submission.response_body_size,
            &submission.response_sha256,
            submission.response_stdout_size,
            &submission.response_stdout_sha256,
            &submission.response,
            &format!("submission {} response", mutation.id),
        )?;
        require!(
            submission.result_kind == "finalized_accepted"
                && submission.submission_count == 1
                && submission.inclusion_count == 1
                && submission.finalization_count == 1
                && submission.accepted_event_count == 1
                && submission.projection_applied,
            "submission {} was not accepted, finalized, emitted, and projected exactly once",
            mutation.id
        );
        require!(
            submission.response.pointer("/protocol_version") == Some(&Value::from(2))
                && submission
                    .response
                    .pointer("/outcome")
                    .and_then(Value::as_str)
                    == Some("finalized_accepted")
                && submission
                    .response
                    .pointer("/operation")
                    .and_then(Value::as_str)
                    == Some(mutation.operation.as_str())
                && submission.response.pointer("/effect").is_some()
                && submission.response.pointer("/projection").is_some(),
            "submission {} retained response is not a successful local-v2 response",
            mutation.id
        );
        require!(
            is_chain_hash(&submission.block_hash) && is_chain_hash(&submission.extrinsic_hash),
            "submission {} retained malformed chain identity",
            mutation.id
        );
        let coordinate_block = submission
            .response
            .pointer("/coordinate/block_number")
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<u64>().ok());
        let coordinate_extrinsic = submission
            .response
            .pointer("/coordinate/extrinsic_index")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok());
        let coordinate_event = submission
            .response
            .pointer("/coordinate/system_event_index")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok());
        require!(
            coordinate_block == Some(submission.block_number)
                && submission
                    .response
                    .pointer("/coordinate/block_hash")
                    .and_then(Value::as_str)
                    == Some(submission.block_hash.as_str())
                && submission
                    .response
                    .pointer("/coordinate/extrinsic_hash")
                    .and_then(Value::as_str)
                    == Some(submission.extrinsic_hash.as_str())
                && coordinate_extrinsic == Some(submission.extrinsic_index)
                && coordinate_event == Some(submission.event_index),
            "submission {} retained coordinate differs from its evidence fields",
            mutation.id
        );
        require!(
            coordinates.insert((
                submission.block_hash.as_str(),
                submission.extrinsic_index,
                submission.event_index,
            )),
            "submission {} reused an accepted event coordinate",
            mutation.id
        );
        let ordered_coordinate = (
            submission.block_number,
            submission.extrinsic_index,
            submission.event_index,
        );
        require!(
            previous_coordinate.is_none_or(|previous| previous < ordered_coordinate),
            "submission {} finalized coordinate is not strictly ordered",
            mutation.id
        );
        previous_coordinate = Some(ordered_coordinate);
        require!(
            submission.endpoint_observations.len() == 2
                && submission.endpoint_observations[0].endpoint == "collator-a"
                && submission.endpoint_observations[1].endpoint == "collator-b",
            "submission {} lacks the exact dual-endpoint finalized observation pair",
            mutation.id
        );
        let response_effect = submission
            .response
            .pointer("/effect")
            .ok_or_else(|| format!("submission {} response lacks effect", mutation.id))?;
        let response_deployment = submission
            .response
            .pointer("/coordinate/deployment_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("submission {} response lacks deployment", mutation.id))?;
        let response_global_sequence = submission
            .response
            .pointer("/coordinate/global_sequence")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("submission {} response lacks global sequence", mutation.id))?;
        let expected_signer = fixture
            .audit
            .dev_signer_accounts
            .get(&mutation.signer)
            .ok_or_else(|| format!("submission {} uses an unpinned signer", mutation.id))?;
        let expected_payload_variant = match mutation.operation.as_str() {
            "create_intent_unit" => "unit_created",
            "transition_intent_unit" => "unit_transitioned",
            "complete_intent_unit" => "unit_completed",
            "create_relationship_definition" => "relationship_definition_created",
            "create_relationship" => "relationship_created",
            "delete_relationship" => "relationship_deleted",
            "record_association" => "association_recorded",
            "revoke_association" => "association_revoked",
            operation => return Err(format!("unmapped mutation operation {operation}")),
        };
        for observation in &submission.endpoint_observations {
            let effect_bytes = canonical_json_bytes(&observation.accepted_effect)?;
            let payload_bytes = decode_lower_hex(&observation.accepted_payload_scale_hex)?;
            require!(
                observation.finalized_head_number == evidence.checkpoints.f.number
                    && observation.finalized_head_hash == evidence.checkpoints.f.hash
                    && observation.block_number == submission.block_number
                    && observation.block_hash == submission.block_hash
                    && observation.extrinsic_hash == submission.extrinsic_hash
                    && observation.extrinsic_index == submission.extrinsic_index
                    && observation.event_index == submission.event_index
                    && observation.matching_extrinsic_count == 1
                    && observation.matching_event_count == 1
                    && observation.accepted_pallet == "Cubikan"
                    && observation.accepted_event == "Accepted"
                    && observation.accepted_deployment_id == response_deployment
                    && observation.accepted_event_schema_version == 1
                    && observation.accepted_global_sequence == response_global_sequence
                    && &observation.accepted_signer == expected_signer
                    && observation.accepted_payload_variant == expected_payload_variant
                    && observation.accepted_payload_sha256 == sha256_bytes(&payload_bytes)?
                    && observation.accepted_payload == expected_payload
                    && &observation.accepted_effect == response_effect
                    && observation.accepted_effect_sha256 == sha256_bytes(&effect_bytes)?
                    && is_sha256(&observation.finalized_header_sha256)
                    && is_sha256(&observation.block_extrinsics_sha256)
                    && is_sha256(&observation.system_events_sha256),
                "submission {} was not independently found exactly once in finalized block/events through {}",
                mutation.id,
                observation.endpoint
            );
        }
        let endpoint_a = &submission.endpoint_observations[0];
        let endpoint_b = &submission.endpoint_observations[1];
        require!(
            endpoint_a.accepted_effect_sha256 == endpoint_b.accepted_effect_sha256
                && endpoint_a.accepted_payload_scale_hex == endpoint_b.accepted_payload_scale_hex
                && endpoint_a.accepted_payload_sha256 == endpoint_b.accepted_payload_sha256
                && endpoint_a.accepted_signer == endpoint_b.accepted_signer
                && endpoint_a.finalized_header_sha256 == endpoint_b.finalized_header_sha256
                && endpoint_a.block_extrinsics_sha256 == endpoint_b.block_extrinsics_sha256
                && endpoint_a.system_events_sha256 == endpoint_b.system_events_sha256,
            "submission {} finalized block/event transcripts diverge across endpoints",
            mutation.id
        );
        let global_sequence = submission
            .response
            .pointer("/coordinate/global_sequence")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("submission {} lacks global sequence", mutation.id))?;
        let expected_sequence = (index + 1).to_string();
        require!(
            global_sequence == expected_sequence,
            "submission {} global sequence is not exact canonical sequence {}",
            mutation.id,
            expected_sequence
        );
        let parachain_genesis_hash = submission
            .response
            .pointer("/coordinate/parachain_genesis_hash")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("submission {} lacks parachain genesis", mutation.id))?;
        let deployment_id = submission
            .response
            .pointer("/coordinate/deployment_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("submission {} lacks deployment ID", mutation.id))?;
        require!(
            is_chain_hash(parachain_genesis_hash) && is_chain_hash(deployment_id),
            "submission {} deployment anchor is malformed",
            mutation.id
        );
        match &deployment_anchor {
            Some((expected_genesis, expected_deployment)) => require!(
                parachain_genesis_hash == *expected_genesis
                    && deployment_id == *expected_deployment,
                "submission {} deployment anchor diverged",
                mutation.id
            ),
            None => {
                deployment_anchor =
                    Some((parachain_genesis_hash.to_owned(), deployment_id.to_owned()));
            }
        }
        if mutation.phase == "before-c" {
            require!(
                submission.block_number <= evidence.checkpoints.c.number,
                "before-C submission {} finalized after C",
                mutation.id
            );
        } else {
            require!(
                submission.endpoint == "collator-a"
                    && submission.block_number > evidence.checkpoints.c.number
                    && submission.block_number <= evidence.checkpoints.f.number,
                "after-C submission {} did not finalize through survivor A between C and F",
                mutation.id
            );
        }
    }

    let endpoint_set = evidence
        .submissions
        .iter()
        .filter(|submission| submission.phase == "before-c")
        .map(|submission| submission.endpoint.as_str())
        .collect::<BTreeSet<_>>();
    require!(
        endpoint_set == BTreeSet::from(["collator-a", "collator-b"]),
        "both collator endpoints were not used before failover"
    );
    for signer in ["charlie", "dave"] {
        let categories = evidence
            .submissions
            .iter()
            .filter(|submission| submission.signer == signer)
            .map(|submission| submission.work_category.as_str())
            .collect::<BTreeSet<_>>();
        require!(
            categories == BTreeSet::from(["lifecycle", "relationship", "provenance-git"]),
            "{signer} did not finalize all required work categories"
        );
    }

    let exact_submission = evidence
        .submissions
        .iter()
        .find(|submission| submission.id == "m12")
        .ok_or_else(|| "finalized evidence lacks exact association mutation m12".to_owned())?;
    let association_bytes = protocol_association_core_bytes(&exact_submission.response)?;
    require!(
        association_bytes == fixture.exact_recorded_association.core_json.as_bytes()
            && exact_submission.recorded_association_core_json.as_deref()
                == Some(fixture.exact_recorded_association.core_json.as_str()),
        "finalized m12 response did not contain the exact T-1114 RecordedAssociation bytes"
    );
    let secondary_submission = evidence
        .submissions
        .iter()
        .find(|submission| submission.id == "m16")
        .ok_or_else(|| {
            "finalized evidence lacks Charlie's provenance/Git mutation m16".to_owned()
        })?;
    require!(
        secondary_submission
            .recorded_association_core_json
            .as_deref()
            == Some(
                std::str::from_utf8(&protocol_association_core_bytes(
                    &secondary_submission.response,
                )?)
                .map_err(|error| format!("secondary core association is not UTF-8: {error}"))?
            ),
        "Charlie's finalized Git association was not reconstructed from the accepted effect"
    );

    let fresh_a = snapshot(journey, "fresh-a")?;
    let fresh_b = snapshot(journey, "fresh-b")?;
    assert_snapshot_contract(journey, fresh_a)?;
    assert_snapshot_contract(journey, fresh_b)?;
    require!(
        fresh_a.full_stream_attested
            && fresh_b.full_stream_attested
            && fresh_a.checkpoint_number == evidence.checkpoints.f.number
            && fresh_b.checkpoint_number == evidence.checkpoints.f.number
            && fresh_a.checkpoint_hash == evidence.checkpoints.f.hash
            && fresh_b.checkpoint_hash == evidence.checkpoints.f.hash
            && fresh_a.sections == fresh_b.sections
            && fresh_a.semantic_sha256 == fresh_b.semantic_sha256,
        "fresh attested reads through collators A and B did not converge at F"
    );
    for snapshot in [fresh_a, fresh_b] {
        require!(
            snapshot.exact_association_core_json == fixture.exact_recorded_association.core_json,
            "{} fresh query did not preserve the finalized T-1114 association bytes",
            snapshot.label
        );
    }
    Ok(())
}

fn protocol_association_core_bytes(response: &Value) -> Result<Vec<u8>, String> {
    let association = response.pointer("/effect/association").ok_or_else(|| {
        "record-association response lacks accepted association effect".to_owned()
    })?;
    let unit_id = association
        .pointer("/unit_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "accepted association lacks unit_id".to_owned())?
        .parse::<IntentUnitId>()
        .map_err(|error| format!("accepted association unit ID is invalid: {error}"))?;
    let subject_kind = association
        .pointer("/subject/type")
        .and_then(Value::as_str)
        .ok_or_else(|| "accepted association lacks subject type".to_owned())?;
    let subject = match subject_kind {
        "whole_unit" => AssociationSubject::WholeUnit,
        "revision" => AssociationSubject::Revision(
            association
                .pointer("/subject/revision")
                .and_then(Value::as_str)
                .ok_or_else(|| "accepted revision association lacks revision".to_owned())?
                .parse::<u64>()
                .map_err(|error| format!("accepted association revision is invalid: {error}"))?,
        ),
        other => return Err(format!("accepted association has unknown subject {other}")),
    };
    let reference = association
        .pointer("/reference")
        .ok_or_else(|| "accepted association lacks reference".to_owned())?;
    let reference_member = |name: &str| -> Result<&str, String> {
        reference
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("accepted association reference lacks {name}"))
    };
    let typed = RecordedAssociation::new(
        unit_id,
        subject,
        ExternalReference::new(
            ReferenceNamespace::new(reference_member("namespace")?)
                .map_err(|error| format!("accepted namespace is invalid: {error}"))?,
            ReferenceText::new(reference_member("scope")?)
                .map_err(|error| format!("accepted scope is invalid: {error}"))?,
            ReferenceText::new(reference_member("value")?)
                .map_err(|error| format!("accepted value is invalid: {error}"))?,
        ),
    );
    serde_json::to_vec(&typed)
        .map_err(|error| format!("cannot serialize accepted association: {error}"))
}

fn snapshot<'a>(journey: &'a JourneyRun, label: &str) -> Result<&'a SnapshotEvidence, String> {
    journey
        .evidence
        .snapshots
        .iter()
        .find(|snapshot| snapshot.label == label)
        .ok_or_else(|| format!("missing semantic snapshot {label}"))
}

fn assert_snapshot_contract(
    journey: &JourneyRun,
    snapshot: &SnapshotEvidence,
) -> Result<(), String> {
    let fixture = &journey.fixture;
    let contract = fixture
        .semantic_projection
        .snapshot_contracts
        .iter()
        .find(|contract| contract.label == snapshot.label)
        .ok_or_else(|| format!("snapshot {} has no independent contract", snapshot.label))?;
    let expected_rpc_url = fixture
        .endpoints
        .get(&contract.read_endpoint)
        .ok_or_else(|| {
            format!(
                "snapshot {} contract names an unknown read endpoint {}",
                snapshot.label, contract.read_endpoint
            )
        })?;
    require!(
        snapshot.source_endpoint == contract.source_endpoint
            && snapshot.read_endpoint == contract.read_endpoint
            && snapshot.read_rpc_url == *expected_rpc_url
            && snapshot.read_executable == journey.evidence.runtime_executables.local_binary.path
            && snapshot.fresh_database == contract.fresh_database
            && snapshot.database_deleted_before_build == contract.database_deleted_before_build
            && snapshot.projection_file_mode == "0600"
            && is_nonzero_canonical_identity(&snapshot.projection_file_device)
            && is_nonzero_canonical_identity(&snapshot.projection_file_inode)
            && snapshot.projection_file_size > 0
            && is_sha256(&snapshot.projection_file_sha256)
            && snapshot.sqlite_sidecar_absence
                == BTreeMap::from([
                    ("-journal".to_owned(), "ENOENT".to_owned()),
                    ("-shm".to_owned(), "ENOENT".to_owned()),
                    ("-wal".to_owned(), "ENOENT".to_owned()),
                ])
            && Path::new(&snapshot.projection_path).is_absolute()
            && Path::new(&snapshot.projection_path).starts_with(&journey.supported_root),
        "snapshot {} source/database ownership contract drifted",
        snapshot.label
    );
    if contract.fresh_database {
        require!(
            snapshot.creation_probe_errno.as_deref() == Some("ENOENT"),
            "snapshot {} did not prove an absent path before first creation",
            snapshot.label
        );
    } else {
        require!(
            snapshot.creation_probe_errno.is_none(),
            "uninterrupted snapshot carried an inapplicable creation probe"
        );
    }
    if contract.database_deleted_before_build {
        let replacement = snapshot.replacement.as_ref().ok_or_else(|| {
            format!(
                "snapshot {} lacks pre-delete/ENOENT/post-create evidence",
                snapshot.label
            )
        })?;
        require!(
            is_nonzero_canonical_identity(&replacement.predelete_device)
                && is_nonzero_canonical_identity(&replacement.predelete_inode)
                && replacement.predelete_size > 0
                && is_sha256(&replacement.predelete_sha256)
                && replacement.retained_descriptor_device_before == replacement.predelete_device
                && replacement.retained_descriptor_inode_before == replacement.predelete_inode
                && replacement.retained_descriptor_size_before == replacement.predelete_size
                && replacement.retained_descriptor_link_count_before == 1
                && replacement.retained_descriptor_device_after
                    == replacement.retained_descriptor_device_before
                && replacement.retained_descriptor_inode_after
                    == replacement.retained_descriptor_inode_before
                && replacement.retained_descriptor_size_after
                    == replacement.retained_descriptor_size_before
                && replacement.retained_descriptor_link_count_after == 0
                && replacement.retained_descriptor_sha256_after == replacement.predelete_sha256
                && replacement.deletion_probe_errno == "ENOENT"
                && replacement.postcreate_device == snapshot.projection_file_device
                && replacement.postcreate_inode == snapshot.projection_file_inode
                && replacement.postcreate_size == snapshot.projection_file_size
                && replacement.postcreate_sha256 == snapshot.projection_file_sha256
                && (&replacement.predelete_device, &replacement.predelete_inode)
                    != (
                        &replacement.postcreate_device,
                        &replacement.postcreate_inode
                    ),
            "snapshot {} did not replace its deleted projection with a distinct inode",
            snapshot.label
        );
    } else {
        require!(
            snapshot.replacement.is_none(),
            "snapshot {} carried inapplicable rebuild-replacement evidence",
            snapshot.label
        );
    }
    let expected_read_ids = fixture
        .reads
        .iter()
        .map(|read| read.id.as_str())
        .collect::<Vec<_>>();
    require!(
        snapshot
            .read_ids
            .iter()
            .map(String::as_str)
            .eq(expected_read_ids.iter().copied()),
        "snapshot {} did not execute the exact read matrix",
        snapshot.label
    );
    require!(
        snapshot.read_responses.len() == expected_read_ids.len()
            && snapshot.read_argvs.len() == expected_read_ids.len()
            && snapshot.read_environments.len() == expected_read_ids.len()
            && snapshot.read_environment_sha256s.len() == expected_read_ids.len()
            && snapshot.read_response_body_hex.len() == expected_read_ids.len()
            && snapshot.read_response_body_sizes.len() == expected_read_ids.len()
            && snapshot.read_response_sha256s.len() == expected_read_ids.len()
            && snapshot.read_stdout_sizes.len() == expected_read_ids.len()
            && snapshot.read_stdout_sha256s.len() == expected_read_ids.len(),
        "snapshot {} read response inventory length drifted",
        snapshot.label
    );
    let pages = snapshot
        .sections
        .get("pages")
        .ok_or_else(|| format!("snapshot {} lacks DB-backed result pages", snapshot.label))?;
    require!(
        pages.len() == expected_read_ids.len(),
        "snapshot {} result-page inventory length drifted",
        snapshot.label
    );
    let expected_environment = exact_sanitized_environment(&journey.repo_root)?;
    let expected_environment_sha256 = canonical_json_sha256(&expected_environment)?;
    for (index, id) in expected_read_ids.iter().enumerate() {
        let expected_argv = vec![
            snapshot.read_executable.clone(),
            "--database".to_owned(),
            snapshot.projection_path.clone(),
            "--rpc".to_owned(),
            snapshot.read_rpc_url.clone(),
        ];
        require!(
            snapshot.read_argvs.get(*id) == Some(&expected_argv),
            "snapshot {} read {id} did not execute the exact pinned binary/database/RPC argv",
            snapshot.label
        );
        require!(
            snapshot.read_environments.get(*id) == Some(&expected_environment)
                && snapshot.read_environment_sha256s.get(*id) == Some(&expected_environment_sha256),
            "snapshot {} read {id} did not execute with the exact seven-entry child environment",
            snapshot.label
        );
        let response = snapshot
            .read_responses
            .get(*id)
            .ok_or_else(|| format!("snapshot {} lacks response for {id}", snapshot.label))?;
        let body_hex = snapshot
            .read_response_body_hex
            .get(*id)
            .ok_or_else(|| format!("snapshot {} lacks response body for {id}", snapshot.label))?;
        let body_size = *snapshot.read_response_body_sizes.get(*id).ok_or_else(|| {
            format!(
                "snapshot {} lacks response body size for {id}",
                snapshot.label
            )
        })?;
        let body_sha256 = snapshot.read_response_sha256s.get(*id).ok_or_else(|| {
            format!(
                "snapshot {} lacks response body digest for {id}",
                snapshot.label
            )
        })?;
        let stdout_size = *snapshot
            .read_stdout_sizes
            .get(*id)
            .ok_or_else(|| format!("snapshot {} lacks stdout size for {id}", snapshot.label))?;
        let stdout_sha256 = snapshot
            .read_stdout_sha256s
            .get(*id)
            .ok_or_else(|| format!("snapshot {} lacks stdout digest for {id}", snapshot.label))?;
        assert_raw_cli_json_output(
            body_hex,
            body_size,
            body_sha256,
            stdout_size,
            stdout_sha256,
            response,
            &format!("snapshot {} read {id}", snapshot.label),
        )?;
        require!(
            response.pointer("/protocol_version") == Some(&Value::from(2))
                && response.pointer("/outcome").and_then(Value::as_str) == Some("success")
                && response.pointer("/result") == pages.get(index),
            "snapshot {} read {id} response/result join drifted",
            snapshot.label
        );
    }
    require!(
        snapshot.sections.len() == fixture.semantic_projection.section_counts.len(),
        "snapshot {} semantic section inventory length drifted",
        snapshot.label
    );
    for (section, count) in &fixture.semantic_projection.section_counts {
        require!(
            snapshot.sections.get(section).map(Vec::len) == Some(*count),
            "snapshot {} section {section} count drifted",
            snapshot.label
        );
    }
    let semantic_bytes = canonical_json_bytes(&snapshot.sections)?;
    require!(
        snapshot.semantic_sha256 == sha256_bytes(&semantic_bytes)?
            && is_sha256(&snapshot.attestation_identity_sha256),
        "snapshot {} semantic/attestation identity is malformed",
        snapshot.label
    );
    Ok(())
}

fn assert_e3(journey: &JourneyRun) -> Result<(), String> {
    let fixture = &journey.fixture;
    let evidence = &journey.evidence;
    let checkpoints = &evidence.checkpoints;
    require!(
        checkpoints.c.name == fixture.checkpoints.catch_up
            && checkpoints.f.name == fixture.checkpoints.final_
            && checkpoints.c.number < checkpoints.f.number
            && checkpoints.f.number <= 100_000
            && is_chain_hash(&checkpoints.c.hash)
            && is_chain_hash(&checkpoints.f.hash)
            && is_sha256(&checkpoints.c.projection_coordinate_sha256)
            && is_sha256(&checkpoints.f.projection_coordinate_sha256),
        "named checkpoints C/F are malformed or unordered"
    );
    require!(
        checkpoints.c.stability_intervals.is_empty()
            && checkpoints.f.stability_intervals.len()
                == fixture.checkpoints.stability_interval_count as usize,
        "F does not carry the exact independently bounded stability intervals"
    );
    for (offset, interval) in checkpoints.f.stability_intervals.iter().enumerate() {
        require!(
            interval.index == (offset + 1) as u64
                && interval.start_number == checkpoints.f.number
                && interval.end_number == checkpoints.f.number
                && interval.start_hash == checkpoints.f.hash
                && interval.end_hash == checkpoints.f.hash
                && interval.elapsed_milliseconds
                    >= fixture.checkpoints.stability_interval_milliseconds
                && interval.elapsed_milliseconds
                    <= fixture.checkpoints.stability_interval_milliseconds + 10_000
                && interval.sample_count >= 2
                && interval.unchanged,
            "F stability interval {} did not retain the exact finalized head",
            interval.index
        );
    }
    require!(
        checkpoints.stopped_role == fixture.checkpoints.stop_role
            && checkpoints.survivor_role == fixture.checkpoints.survivor_role
            && checkpoints.stopped_at_c,
        "collator B was not stopped exactly at C with A as survivor"
    );
    let expected_survivor_ids = fixture
        .mutations
        .iter()
        .filter(|mutation| mutation.phase == "after-c")
        .map(|mutation| mutation.id.as_str())
        .collect::<Vec<_>>();
    require!(
        checkpoints
            .survivor_finalized_mutation_ids
            .iter()
            .map(String::as_str)
            .eq(expected_survivor_ids.iter().copied()),
        "survivor A did not finalize the exact remaining mutation sequence"
    );

    let restart = &evidence.restart;
    let collator_b = evidence
        .topology
        .nodes
        .iter()
        .find(|node| node.role == "collator-b")
        .ok_or_else(|| "topology lacks collator-b".to_owned())?;
    require!(
        restart.role == fixture.checkpoints.stop_role
            && restart.endpoint == "collator-b"
            && restart.pid_before == collator_b.pid
            && restart.pid_after > 1
            && restart.pid_after != restart.pid_before
            && restart.process_start_time_ticks_after > 0
            && restart.restarted_node.role == "collator-b"
            && restart.restarted_node.pid == restart.pid_after
            && restart.restarted_node.process_start_time_ticks_before
                == restart.process_start_time_ticks_after,
        "restart did not replace collator B's process identity"
    );
    require!(
        restart.data_directory_before == collator_b.data_directory
            && restart.data_directory_after == restart.data_directory_before
            && restart.data_directory_mode_before == "0700"
            && restart.data_directory_mode_after == "0700"
            && Path::new(&restart.data_directory_after).starts_with(&journey.supported_root)
            && is_nonzero_canonical_identity(&restart.data_directory_device_before)
            && is_nonzero_canonical_identity(&restart.data_directory_inode_before)
            && restart.data_directory_device_before == restart.data_directory_device_after
            && restart.data_directory_inode_before == restart.data_directory_inode_after
            && restart.config_sha256_before == restart.config_sha256_after
            && is_sha256(&restart.config_sha256_before)
            && !restart.frozen_command.is_empty()
            && restart.config_sha256_before
                == sha256_bytes(
                    &serde_json::to_vec(&collator_b.argv)
                        .map_err(|error| format!("cannot serialize original argv: {error}"))?
                )?
            && restart.config_sha256_after
                == sha256_bytes(
                    &serde_json::to_vec(&restart.restarted_node.argv)
                        .map_err(|error| format!("cannot serialize restarted argv: {error}"))?
                )?
            && restart.frozen_command_sha256
                == sha256_bytes(
                    &serde_json::to_vec(&restart.frozen_command)
                        .map_err(|error| format!("cannot serialize frozen command: {error}"))?
                )?
            && Path::new(&restart.frozen_log_path).is_absolute()
            && restart.spawn_executable == "bash"
            && restart.spawn_args
                == std::iter::once("-c".to_owned())
                    .chain(restart.frozen_command.iter().cloned())
                    .collect::<Vec<_>>()
            && restart.teardown_exit_code.is_none()
            && restart.teardown_signal == "SIGKILL",
        "collator B did not restart with its original owner-only archive data/config"
    );
    let expected_archive_flags =
        ["--blocks-pruning", "archive", "--state-pruning", "archive"].map(str::to_owned);
    require!(
        restart.archive_flags_before == expected_archive_flags
            && restart.archive_flags_after == expected_archive_flags,
        "collator B restart lost an archive flag"
    );
    let restarted = &restart.restarted_node;
    let collator_fixture = fixture
        .topology
        .nodes
        .iter()
        .find(|node| node.role == "collator-b")
        .ok_or_else(|| "fixture lacks collator-b".to_owned())?;
    let collator_asset = pinned_input(&fixture.pinned_inputs, &collator_fixture.binary)?;
    assert_unprivileged_process(&restarted.privileges, "restarted collator-b")?;
    require!(
        restarted.kind == collator_fixture.kind
            && restarted.generated_name == collator_fixture.generated_name
            && restarted.dev_seed == collator_fixture.dev_seed
            && restarted.node_key == collator_fixture.node_key
            && restarted.peer_id == collator_fixture.peer_id
            && restarted.parent_pid == evidence.topology.orchestrator.pid
            && restarted.data_directory == restart.data_directory_after
            && restarted.data_directory_mode == "0700"
            && restarted.data_directories.len() == 2
            && restarted
                .data_directories
                .iter()
                .all(|directory| directory.mode == "0700"
                    && is_nonzero_canonical_identity(&directory.device)
                    && is_nonzero_canonical_identity(&directory.inode)
                    && Path::new(&directory.path).starts_with(&journey.supported_root))
            && restarted.data_directories == collator_b.data_directories
            && restarted.primary_chain_spec == collator_b.primary_chain_spec
            && restarted.relay_side_chain_spec == collator_b.relay_side_chain_spec
            && restarted.archive_flags == expected_archive_flags
            && restarted.proc_exe_link == "/memfd:cubikan-sealed-exec-v1 (deleted)"
            && restarted.proc_exe_size == collator_asset.size
            && restarted.proc_exe_sha256 == collator_asset.sha256
            && restarted.proc_exe_mode == "0500"
            && restarted.proc_exe_seals == ["seal", "shrink", "grow", "write"].map(str::to_owned)
            && restarted.post_seal_write_denied
            && restarted.authenticated_archive_node_evidence
            && restarted.process_start_time_ticks_before
                == restarted.process_start_time_ticks_after
            && is_nonzero_canonical_identity(&restarted.proc_directory_device_before)
            && is_nonzero_canonical_identity(&restarted.proc_directory_inode_before)
            && restarted.proc_directory_device_before == restarted.proc_directory_device_after
            && restarted.proc_directory_inode_before == restarted.proc_directory_inode_after
            && restarted.proc_directory_descriptor_device_before
                == restarted.proc_directory_device_before
            && restarted.proc_directory_descriptor_inode_before
                == restarted.proc_directory_inode_before
            && restarted.proc_directory_descriptor_device_after
                == restarted.proc_directory_device_after
            && restarted.proc_directory_descriptor_inode_after
                == restarted.proc_directory_inode_after
            && is_nonzero_canonical_identity(&restarted.executable_device_before)
            && is_nonzero_canonical_identity(&restarted.executable_inode_before)
            && restarted.executable_device_before == restarted.executable_device_after
            && restarted.executable_inode_before == restarted.executable_inode_after
            && restarted.executable_descriptor_device_before == restarted.executable_device_before
            && restarted.executable_descriptor_inode_before == restarted.executable_inode_before
            && restarted.executable_descriptor_device_after == restarted.executable_device_after
            && restarted.executable_descriptor_inode_after == restarted.executable_inode_after
            && proc_cmdline_sha256(&restarted.argv)? == restarted.proc_cmdline_sha256,
        "restarted collator B did not re-establish exact sealed process/archive evidence"
    );
    assert_exact_node_environment(restarted, &journey.repo_root)?;
    assert_quiet_execution_flags(&restarted.argv, 2, "restarted collator-b")?;
    assert_exact_bootnodes(restarted, collator_fixture)?;
    require!(
        restart.listener_records_after.len() == collator_fixture.listeners.len()
            && restart
                .listener_records_after
                .iter()
                .zip(&collator_fixture.listeners)
                .all(|(record, address)| record.protocol == "tcp"
                    && record.address == *address
                    && record.pid == restart.pid_after
                    && record.raw_line.contains(address)
                    && record
                        .raw_line
                        .contains(&format!("pid={},", restart.pid_after))),
        "restarted collator B did not own all six exact listeners"
    );

    let stop = &restart.stop;
    let stopped_addresses = collator_fixture
        .listeners
        .iter()
        .map(|address| (address.clone(), "ECONNREFUSED".to_owned()))
        .collect::<BTreeMap<_, _>>();
    require!(
        stop.transcript.signal == "SIGTERM"
            && stop.transcript.signal_send_succeeded
            && stop.transcript.pid_cleared_before_signal
            && stop.transcript.sent_to_pid == restart.pid_before
            && stop.transcript.sent_to_start_time_ticks
                == collator_b.process_start_time_ticks_before
            && stop.transcript.pre_signal_proc_device == collator_b.proc_directory_device_after
            && stop.transcript.pre_signal_proc_inode == collator_b.proc_directory_inode_after
            && stop.transcript.wait_observed
            && stop.transcript.generation_wait_observed
            && stop.transcript.exit_code.is_none()
            && stop.transcript.termination_signal.is_none()
            && stop.transcript.proc_probe_errno == "ENOENT"
            && stop.transcript.listener_probe_errors == stopped_addresses
            && stop.transcript.observed_before_first_survivor_mutation,
        "collator B stop lacks kill/wait/proc-absence/socket-absence proof before failover work"
    );
    let stop_bytes = canonical_json_bytes(&stop.transcript)?;
    require!(
        stop.transcript_sha256 == sha256_bytes(&stop_bytes)?,
        "collator B stop transcript digest does not match retained evidence"
    );
    require!(
        restart.synchronized_number == checkpoints.f.number
            && restart.synchronized_hash == checkpoints.f.hash
            && !restart.source_eligible_before_probes
            && restart.source_use_count_before_probes == 0
            && restart.source_eligible_after_probes
            && restart.source_use_count_after_probes == 3,
        "collator B was used before proof or failed to become eligible after syncing to F"
    );

    let source_gate = &restart.source_gate;
    let expected_gate_transcript = [
        (1, "source_open", "uninterrupted", Some("both"), false),
        (2, "gate_open", "dual-endpoint-identity-archive", None, true),
        (3, "source_open", "fresh-a", Some("collator-a"), true),
        (4, "source_open", "fresh-b", Some("collator-b"), true),
        (
            5,
            "source_open",
            "rebuild-a-predelete",
            Some("collator-a"),
            true,
        ),
        (6, "source_open", "rebuild-a", Some("collator-a"), true),
        (
            7,
            "source_open",
            "rebuild-b-predelete",
            Some("collator-b"),
            true,
        ),
        (8, "source_open", "rebuild-b", Some("collator-b"), true),
    ];
    require!(
        source_gate.format == "cubikan-rebuild-source-gate-v1"
            && source_gate.gate_sequence == 2
            && source_gate.identity_a_sha256 == canonical_json_sha256(&restart.identity_a)?
            && source_gate.identity_b_sha256 == canonical_json_sha256(&restart.identity_b)?
            && source_gate.identity_a_sha256 == source_gate.identity_b_sha256
            && source_gate.archive_probes_sha256 == canonical_json_sha256(&restart.archive_probes)?
            && source_gate.transcript_sha256 == canonical_json_sha256(&source_gate.transcript)?
            && source_gate.transcript.len() == expected_gate_transcript.len()
            && source_gate
                .transcript
                .iter()
                .zip(&expected_gate_transcript)
                .all(|(observed, expected)| {
                    observed.sequence == expected.0
                        && observed.kind == expected.1
                        && observed.label == expected.2
                        && observed.source_endpoint.as_deref() == expected.3
                        && observed.gate_open == expected.4
                        && observed.identity_prerequisites_complete == expected.4
                        && observed.archive_prerequisites_complete == expected.4
                },),
        "rebuild source gate transcript/order/prerequisite identity drifted"
    );
    let source_open_count = |endpoint: &str| {
        source_gate
            .transcript
            .iter()
            .filter(|entry| {
                entry.kind == "source_open" && entry.source_endpoint.as_deref() == Some(endpoint)
            })
            .count()
    };
    require!(
        source_open_count("collator-a") == 3
            && source_open_count("collator-b") == 3
            && evidence.snapshots[0].source_gate_sequence == 1
            && !evidence.snapshots[0].source_eligible_at_open
            && evidence.snapshots[1].source_gate_sequence == 3
            && evidence.snapshots[2].source_gate_sequence == 4
            && evidence.snapshots[3].source_gate_sequence == 6
            && evidence.snapshots[4].source_gate_sequence == 8
            && evidence.snapshots[1..]
                .iter()
                .all(|snapshot| snapshot.source_eligible_at_open),
        "fresh/prime/rebuild source opens were not strictly ordered after the gate"
    );

    require!(
        restart.identity_a == restart.identity_b,
        "full endpoint identity differs after collator B restart"
    );
    let db_checkpoint = evidence
        .snapshots
        .first()
        .and_then(|snapshot| snapshot.sections.get("checkpoint"))
        .and_then(|section| section.first())
        .ok_or_else(|| "snapshots lack a DB-backed checkpoint".to_owned())?;
    let db_checkpoint_bytes = canonical_json_bytes(db_checkpoint)?;
    require!(
        restart.identity_a.projection_checkpoint_sha256 == sha256_bytes(&db_checkpoint_bytes)?
            && evidence.snapshots.iter().all(|snapshot| snapshot
                .sections
                .get("checkpoint")
                .and_then(|section| section.first())
                == Some(db_checkpoint)),
        "endpoint projection identity is not bound to every fresh DB-backed checkpoint"
    );
    let first_submission = evidence
        .submissions
        .first()
        .ok_or_else(|| "journey lacks finalized submissions".to_owned())?;
    let response_genesis = first_submission
        .response
        .pointer("/coordinate/parachain_genesis_hash")
        .and_then(Value::as_str)
        .ok_or_else(|| "first finalized response lacks parachain genesis".to_owned())?;
    let response_deployment = first_submission
        .response
        .pointer("/coordinate/deployment_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "first finalized response lacks deployment ID".to_owned())?;
    let relay_a = evidence
        .topology
        .nodes
        .iter()
        .find(|node| node.role == "relay-a")
        .ok_or_else(|| "topology lacks relay-a".to_owned())?;
    let initial_collator_a = evidence
        .topology
        .nodes
        .iter()
        .find(|node| node.role == "collator-a")
        .ok_or_else(|| "topology lacks collator-a".to_owned())?;
    for identity in [&restart.identity_a, &restart.identity_b] {
        require!(
            is_chain_hash(&identity.relay_genesis_hash)
                && identity.relay_genesis_hash
                    == "0x2a682650fa14e635c7ec4c2b7e4f043017a08b58d586d0d2dc50a64e223a6d27"
                && is_sha256(&identity.relay_genesis_payload_sha256)
                && is_chain_hash(&identity.parachain_genesis_hash)
                && identity.parachain_genesis_hash
                    == "0x627f53b3abc01130ec273ef85759f90779e8497614a428a66d862a624ee01a17"
                && is_sha256(&identity.parachain_genesis_payload_sha256)
                && is_chain_hash(&identity.deployment_id)
                && identity.deployment_id
                    == "0x3046cb2cf3f5f9c565a85493cfff10fee94d12d950a0d6f54d7c1ff32a6afc42"
                && identity.checkpoint_hash == checkpoints.f.hash
                && identity.runtime_spec_version > 0
                && identity.event_schema_version == 1
                && identity.pallet_storage_version == 1
                && is_sha256(&identity.runtime_code_sha256)
                && is_chain_hash(&identity.runtime_code_chain_hash)
                && is_sha256(&identity.metadata_sha256)
                && is_sha256(&identity.projection_checkpoint_sha256)
                && identity.relay_genesis_hash != identity.parachain_genesis_hash
                && identity.relay_genesis_payload_sha256
                    == relay_a.primary_chain_spec.genesis_payload_sha256
                && identity.parachain_genesis_payload_sha256
                    == initial_collator_a.primary_chain_spec.genesis_payload_sha256
                && identity.relay_genesis_hash == relay_a.primary_chain_spec.live_genesis_hash
                && identity.parachain_genesis_hash
                    == initial_collator_a.primary_chain_spec.live_genesis_hash
                && identity.parachain_genesis_hash == response_genesis
                && identity.deployment_id == response_deployment,
            "post-restart endpoint identity is malformed or not anchored at F"
        );
    }
    let cubikan_runtime = pinned_sha256(
        &fixture.pinned_inputs,
        "chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm",
    )?;
    require!(
        restart.identity_a.runtime_code_sha256 == cubikan_runtime,
        "historical :code identity differs from the pinned CubiKan Wasm"
    );
    require!(
        db_checkpoint
            .get("runtime_code_hash")
            .and_then(Value::as_str)
            == Some(restart.identity_a.runtime_code_chain_hash.as_str()),
        "DB checkpoint runtime chain hash differs from live Substrate BlakeTwo256(:code)"
    );

    let probes = &restart.archive_probes;
    require!(
        probes.range_start == fixture.archive_probe.range_start
            && probes.range_end == checkpoints.f.number
            && probes.methods_per_block == fixture.archive_probe.methods_per_block
            && probes.system_events_storage_key == fixture.archive_probe.system_events_storage_key
            && probes.runtime_code_storage_key == fixture.archive_probe.runtime_code_storage_key,
        "archive probe range/method/key contract drifted"
    );
    let expected_blocks =
        (fixture.archive_probe.range_start..=checkpoints.f.number).collect::<Vec<_>>();
    require!(
        probes.probed_block_numbers == expected_blocks,
        "archive probe did not cover every historical block from genesis through F"
    );
    let method_count = u64::try_from(fixture.archive_probe.methods_per_block.len())
        .map_err(|error| format!("archive method count overflowed: {error}"))?;
    let expected_per_endpoint = (checkpoints.f.number + 1)
        .checked_mul(method_count)
        .ok_or_else(|| "archive probe count overflowed".to_owned())?;
    require!(
        probes.expected_per_endpoint == expected_per_endpoint
            && probes.endpoint_a_probe_count == expected_per_endpoint
            && probes.endpoint_b_probe_count == expected_per_endpoint
            && probes.missing_count == 0
            && probes.mismatch_count == 0
            && probes.completed_before_source_use,
        "archive probe counts show missing, mismatched, or post-use checks"
    );
    require!(
        probes.transcript.len() == expected_per_endpoint as usize,
        "archive probe transcript length drifted"
    );
    for (record, (block_number, method)) in
        probes
            .transcript
            .iter()
            .zip(expected_blocks.iter().flat_map(|block| {
                fixture
                    .archive_probe
                    .methods_per_block
                    .iter()
                    .map(move |method| (*block, method.as_str()))
            }))
    {
        let is_genesis_events = block_number == 0 && method == "state_getStorage:System.Events";
        require!(
            record.block_number == block_number
                && is_chain_hash(&record.block_hash)
                && record.method == method
                && record.endpoint_a_observed_height == block_number
                && record.endpoint_b_observed_height == block_number
                && record.endpoint_a_block_hash == record.block_hash
                && record.endpoint_b_block_hash == record.block_hash
                && record.present
                && if is_genesis_events {
                    !record.raw_present && record.canonical_genesis_default
                } else {
                    record.raw_present && !record.canonical_genesis_default
                }
                && record.equal
                && is_sha256(&record.endpoint_a_sha256)
                && record.endpoint_a_sha256 == record.endpoint_b_sha256,
            "archive probe transcript diverged at block {block_number} method {method}"
        );
    }
    let transcript_bytes = canonical_json_bytes(&probes.transcript)?;
    require!(
        probes.transcript_sha256 == sha256_bytes(&transcript_bytes)?,
        "archive probe transcript digest does not match retained records"
    );
    Ok(())
}

fn assert_e4(journey: &JourneyRun) -> Result<(), String> {
    let fixture = &journey.fixture;
    let evidence = &journey.evidence;
    require!(
        evidence.snapshots.len() == fixture.semantic_projection.snapshot_contracts.len(),
        "semantic snapshot inventory must contain exactly five databases"
    );
    let labels = evidence
        .snapshots
        .iter()
        .map(|snapshot| snapshot.label.as_str())
        .collect::<Vec<_>>();
    require!(
        labels
            == fixture
                .semantic_projection
                .snapshot_contracts
                .iter()
                .map(|contract| contract.label.as_str())
                .collect::<Vec<_>>(),
        "semantic snapshot order/labels drifted"
    );
    let paths = evidence
        .snapshots
        .iter()
        .map(|snapshot| snapshot.projection_path.as_str())
        .collect::<BTreeSet<_>>();
    require!(
        paths.len() == evidence.snapshots.len(),
        "semantic snapshots did not use independent projection paths"
    );

    for snapshot in &evidence.snapshots {
        assert_snapshot_contract(journey, snapshot)?;
        require!(
            snapshot.full_stream_attested
                && (if snapshot.label == "uninterrupted" {
                    !snapshot.source_eligible_at_open
                } else {
                    snapshot.source_eligible_at_open
                })
                && snapshot.checkpoint_number == evidence.checkpoints.f.number
                && snapshot.checkpoint_hash == evidence.checkpoints.f.hash,
            "snapshot {} was not full-stream-attested at F from an eligible source",
            snapshot.label
        );
    }
    let uninterrupted = snapshot(journey, "uninterrupted")?;
    for label in ["fresh-a", "fresh-b", "rebuild-a", "rebuild-b"] {
        let rebuilt = snapshot(journey, label)?;
        require!(
            rebuilt.sections == uninterrupted.sections
                && rebuilt.semantic_sha256 == uninterrupted.semantic_sha256
                && rebuilt.read_response_sha256s == uninterrupted.read_response_sha256s
                && rebuilt.attestation_identity_sha256 == uninterrupted.attestation_identity_sha256,
            "snapshot {label} differs semantically from uninterrupted projection"
        );
    }
    let rebuild_a = snapshot(journey, "rebuild-a")?;
    let rebuild_b = snapshot(journey, "rebuild-b")?;
    require!(
        rebuild_a.fresh_database
            && rebuild_a.database_deleted_before_build
            && rebuild_b.fresh_database
            && rebuild_b.database_deleted_before_build
            && rebuild_a.source_endpoint == "collator-a"
            && rebuild_b.source_endpoint == "collator-b",
        "A/B rebuilds were not disposable, deleted-first, and independently sourced"
    );
    for label in &fixture.exact_recorded_association.required_evidence_sources {
        let rebuilt = snapshot(journey, label)?;
        require!(
            rebuilt.exact_association_core_json == fixture.exact_recorded_association.core_json,
            "snapshot {label} did not reconstruct the exact finalized T-1114 association bytes"
        );
    }
    require!(
        uninterrupted.exact_association_core_json == fixture.exact_recorded_association.core_json,
        "uninterrupted projection did not retain the exact finalized T-1114 association"
    );
    Ok(())
}

fn expected_submission_lane_name(
    directory: &str,
    deployment_id: &str,
    signer: &str,
) -> Result<String, String> {
    let directory_length = u32::try_from(directory.len())
        .map_err(|error| format!("submission lane directory is over bound: {error}"))?;
    let mut preimage = b"CubiKan signer lane v1\0".to_vec();
    preimage.extend_from_slice(&directory_length.to_be_bytes());
    preimage.extend_from_slice(directory.as_bytes());
    preimage.extend_from_slice(&decode_lower_hex(deployment_id)?);
    preimage.extend_from_slice(&decode_lower_hex(signer)?);
    Ok(format!(
        "cubikan-submission-{}.lock",
        sha256_bytes(&preimage)?
    ))
}

fn exact_dispatch_method(operation: &str) -> Result<&'static str, String> {
    match operation {
        "create_intent_unit" => Ok("createUnit"),
        "transition_intent_unit" => Ok("transitionUnit"),
        "complete_intent_unit" => Ok("completeUnit"),
        "create_relationship_definition" => Ok("createRelationshipDefinition"),
        "create_relationship" => Ok("createRelationship"),
        "delete_relationship" => Ok("deleteRelationship"),
        "record_association" => Ok("recordAssociation"),
        "revoke_association" => Ok("revokeAssociation"),
        _ => Err(format!("unknown protocol operation {operation}")),
    }
}

fn canonical_json_u128(value: &Value, label: &str) -> Result<u128, String> {
    if let Some(value) = value.as_u64() {
        return Ok(u128::from(value));
    }
    if let Some(value) = value.as_str() {
        let canonical = value == "0"
            || value
                .as_bytes()
                .first()
                .is_some_and(|byte| (b'1'..=b'9').contains(byte))
                && value.as_bytes().iter().all(u8::is_ascii_digit);
        if canonical {
            return value
                .parse::<u128>()
                .map_err(|error| format!("{label} overflows u128: {error}"));
        }
    }
    Err(format!("{label} is not a canonical unsigned integer"))
}

fn exact_single_event_u64(data: &Value, label: &str) -> Result<u64, String> {
    let fields = data
        .as_array()
        .filter(|fields| fields.len() == 1)
        .ok_or_else(|| format!("{label} is not one event field"))?;
    u64::try_from(canonical_json_u128(&fields[0], label)?)
        .map_err(|error| format!("{label} overflows u64: {error}"))
}

fn assert_chain_census(journey: &JourneyRun) -> Result<(), String> {
    let fixture = &journey.fixture;
    let evidence = &journey.evidence;
    let census = &evidence.audit.chain_census;
    let final_checkpoint = &evidence.checkpoints.f;
    require!(
        census.format == "cubikan-finalized-chain-census-v1"
            && census.range_start == fixture.audit.chain_census.range_start
            && census.range_start == 0
            && census.range_end == final_checkpoint.number
            && census.maximum_retained_bytes == fixture.audit.chain_census.maximum_retained_bytes
            && census.allowed_unsigned_calls == fixture.audit.chain_census.allowed_unsigned_calls
            && census.allowed_events == fixture.audit.chain_census.allowed_events
            && census.endpoints_equal
            && census.endpoint_a.endpoint == "collator-a"
            && census.endpoint_b.endpoint == "collator-b"
            && census.endpoint_a.range_start == 0
            && census.endpoint_b.range_start == 0
            && census.endpoint_a.range_end == census.range_end
            && census.endpoint_b.range_end == census.range_end,
        "finalized chain census format/range/endpoints diverged"
    );
    let canonical_census = canonical_json_bytes(census)?;
    require!(
        canonical_census.len() as u64 <= census.maximum_retained_bytes
            && census.endpoint_a.retained_bytes <= census.maximum_retained_bytes,
        "finalized chain census exceeded an independent retained byte bound"
    );
    let census_artifact = evidence
        .audit
        .artifacts
        .iter()
        .find(|artifact| artifact.logical_path == "action/finalized-chain-census.json")
        .ok_or_else(|| "audit lacks the finalized chain census artifact".to_owned())?;
    require!(
        census_artifact.kind == "action"
            && census_artifact.read.bytes_read == canonical_census.len() as u64
            && census_artifact.read.sha256 == sha256_bytes(&canonical_census)?,
        "finalized chain census artifact is not the exact canonical evidence bytes"
    );

    let expected_block_count = usize::try_from(census.range_end)
        .map_err(|error| format!("census range end overflows usize: {error}"))?
        .checked_add(1)
        .ok_or_else(|| "census block count overflowed".to_owned())?;
    require!(
        census.endpoint_a.blocks.len() == expected_block_count,
        "finalized chain census did not retain every block from genesis through F"
    );
    let retained_a = census
        .endpoint_a
        .blocks
        .iter()
        .try_fold(0_u64, |total, block| {
            total
                .checked_add(canonical_json_bytes(block)?.len() as u64)
                .ok_or_else(|| "census retained byte count overflowed".to_owned())
        })?;
    require!(
        retained_a == census.endpoint_a.retained_bytes
            && census.endpoint_a.transcript_sha256
                == canonical_json_sha256(&census.endpoint_a.blocks)?
            && census.endpoint_b.transcript_sha256 == census.endpoint_a.transcript_sha256
            && census.endpoint_b.block_count
                == u64::try_from(expected_block_count)
                    .map_err(|error| format!("census block count overflows u64: {error}"))?,
        "finalized chain census retained byte/transcript attestation drifted"
    );

    let allowed_unsigned_calls = fixture
        .audit
        .chain_census
        .allowed_unsigned_calls
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let allowed_events = fixture
        .audit
        .chain_census
        .allowed_events
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut observed_event_counts = fixture
        .audit
        .chain_census
        .allowed_events
        .iter()
        .map(|key| (key.clone(), 0_u64))
        .collect::<BTreeMap<_, _>>();
    let mut expected_by_hash = BTreeMap::new();
    let mut expected_event_coordinates = BTreeMap::new();
    for (submission, mutation) in evidence.submissions.iter().zip(&fixture.mutations) {
        require!(
            expected_by_hash
                .insert(submission.extrinsic_hash.as_str(), (submission, mutation))
                .is_none(),
            "expected submission hash is duplicated"
        );
        require!(
            expected_event_coordinates
                .insert(
                    (submission.block_number, u64::from(submission.event_index)),
                    submission,
                )
                .is_none(),
            "expected submission event coordinate is duplicated"
        );
    }
    let mut seen_submission_hashes = BTreeSet::new();
    let mut signed_count = 0_u64;
    let mut unsigned_count = 0_u64;
    let mut accepted_count = 0_u64;
    let mut success_count = 0_u64;
    let mut previous_hash: Option<&str> = None;
    for (expected_number, block) in census.endpoint_a.blocks.iter().enumerate() {
        require!(
            block.number == expected_number as u64
                && is_chain_hash(&block.hash)
                && is_chain_hash(&block.state_root)
                && is_chain_hash(&block.extrinsics_root)
                && if let Some(previous_hash) = previous_hash {
                    block.parent_hash == previous_hash
                } else {
                    block.parent_hash == format!("0x{}", "0".repeat(64))
                }
                && sha256_bytes(&decode_lower_hex(&block.header_scale_hex)?)?
                    == block.header_sha256
                && sha256_bytes(&decode_lower_hex(&block.system_events_scale_hex)?)?
                    == block.system_events_sha256,
            "finalized chain census block/header linkage drifted at {}",
            block.number
        );
        previous_hash = Some(&block.hash);
        let mut success_by_extrinsic = vec![0_u64; block.extrinsics.len()];
        let mut fee_counts_by_extrinsic = vec![[0_u64; 3]; block.extrinsics.len()];
        let mut fee_amounts_by_extrinsic = vec![[None; 3]; block.extrinsics.len()];
        for (expected_index, extrinsic) in block.extrinsics.iter().enumerate() {
            let call = format!("{}.{}", extrinsic.section, extrinsic.method);
            require!(
                extrinsic.index == expected_index as u64
                    && is_chain_hash(&extrinsic.hash)
                    && decode_lower_hex(&extrinsic.call_index)?.len() == 2
                    && extrinsic.args_sha256 == canonical_json_sha256(&extrinsic.args)?
                    && extrinsic.scale_sha256
                        == sha256_bytes(&decode_lower_hex(&extrinsic.scale_hex)?)?
                    && extrinsic.signed == extrinsic.signer.is_some(),
                "finalized chain census extrinsic row drifted at {}:{}",
                block.number,
                extrinsic.index
            );
            if extrinsic.signed {
                signed_count += 1;
                let (submission, mutation) = expected_by_hash
                    .get(extrinsic.hash.as_str())
                    .ok_or_else(|| {
                        format!(
                            "unexpected signed action at {}:{}: {call}",
                            block.number, extrinsic.index
                        )
                    })?;
                let expected_signer = fixture
                    .audit
                    .dev_signer_accounts
                    .get(&submission.signer)
                    .ok_or_else(|| format!("unknown dev signer {}", submission.signer))?;
                require!(
                    seen_submission_hashes.insert(extrinsic.hash.as_str())
                        && block.number == submission.block_number
                        && extrinsic.index == u64::from(submission.extrinsic_index)
                        && extrinsic.signer.as_deref() == Some(expected_signer.as_str())
                        && extrinsic.section == "cubikan"
                        && extrinsic.method == exact_dispatch_method(&mutation.operation)?,
                    "signed census action did not join exact submission metadata at {}:{}",
                    block.number,
                    extrinsic.index
                );
            } else {
                require!(
                    allowed_unsigned_calls.contains(call.as_str()),
                    "unexpected unsigned action at {}:{}: {call}",
                    block.number,
                    extrinsic.index
                );
                unsigned_count += 1;
            }
        }
        for (expected_index, event) in block.events.iter().enumerate() {
            let event_key = format!("{}.{}", event.section, event.method);
            require!(
                event.index == expected_index as u64
                    && event.data_sha256 == canonical_json_sha256(&event.data)?
                    && event.scale_sha256 == sha256_bytes(&decode_lower_hex(&event.scale_hex)?)?
                    && event.topics.iter().all(|topic| is_chain_hash(topic))
                    && allowed_events.contains(event_key.as_str()),
                "unexpected or malformed finalized event at {}:{}: {event_key}",
                block.number,
                event.index
            );
            *observed_event_counts
                .get_mut(event_key.as_str())
                .ok_or_else(|| format!("allowed event lacks a counter: {event_key}"))? += 1;
            match event.phase.kind.as_str() {
                "apply_extrinsic" => {
                    let extrinsic_index = event.phase.extrinsic_index.ok_or_else(|| {
                        format!(
                            "apply event lacks extrinsic index at {}:{}",
                            block.number, event.index
                        )
                    })?;
                    require!(
                        usize::try_from(extrinsic_index)
                            .ok()
                            .is_some_and(|index| index < block.extrinsics.len()),
                        "event extrinsic index is out of range at {}:{}",
                        block.number,
                        event.index
                    );
                    if event_key == "system.ExtrinsicSuccess" {
                        success_count += 1;
                        success_by_extrinsic[extrinsic_index as usize] += 1;
                    }
                    if event_key == "cubikan.Accepted" {
                        accepted_count += 1;
                        let submission = expected_event_coordinates
                            .get(&(block.number, event.index))
                            .ok_or_else(|| {
                                format!(
                                    "Accepted event lacks an expected coordinate at {}:{}",
                                    block.number, event.index
                                )
                            })?;
                        require!(
                            extrinsic_index == u64::from(submission.extrinsic_index),
                            "Accepted event/extrinsic join drifted at {}:{}",
                            block.number,
                            event.index
                        );
                    }
                    let fee_event_index = match event_key.as_str() {
                        "balances.Withdraw" => Some(0),
                        "balances.BurnedDebt" => Some(1),
                        "transactionPayment.TransactionFeePaid" => Some(2),
                        _ => None,
                    };
                    if let Some(fee_event_index) = fee_event_index {
                        let extrinsic = &block.extrinsics[extrinsic_index as usize];
                        require!(
                            extrinsic.signed
                                && expected_by_hash.contains_key(extrinsic.hash.as_str()),
                            "fee event was not tied to an expected signed submission"
                        );
                        let data = event.data.as_array().ok_or_else(|| {
                            format!(
                                "fee event data is not an array at {}:{}",
                                block.number, event.index
                            )
                        })?;
                        let amount = match fee_event_index {
                            0 => {
                                require!(
                                    data.len() == 2
                                        && data[0].as_str() == extrinsic.signer.as_deref(),
                                    "balances.Withdraw shape or payer drifted at {}:{}",
                                    block.number,
                                    event.index
                                );
                                canonical_json_u128(
                                    &data[1],
                                    &format!(
                                        "balances.Withdraw amount at {}:{}",
                                        block.number, event.index
                                    ),
                                )?
                            }
                            1 => {
                                require!(
                                    data.len() == 1,
                                    "balances.BurnedDebt shape drifted at {}:{}",
                                    block.number,
                                    event.index
                                );
                                canonical_json_u128(
                                    &data[0],
                                    &format!(
                                        "balances.BurnedDebt amount at {}:{}",
                                        block.number, event.index
                                    ),
                                )?
                            }
                            2 => {
                                require!(
                                    data.len() == 3
                                        && data[0].as_str() == extrinsic.signer.as_deref()
                                        && canonical_json_u128(
                                            &data[2],
                                            &format!(
                                                "transaction fee tip at {}:{}",
                                                block.number, event.index
                                            ),
                                        )? == 0,
                                    "transaction fee shape, payer, or zero tip drifted at {}:{}",
                                    block.number,
                                    event.index
                                );
                                canonical_json_u128(
                                    &data[1],
                                    &format!(
                                        "transaction actual fee at {}:{}",
                                        block.number, event.index
                                    ),
                                )?
                            }
                            _ => unreachable!("fee event index is closed above"),
                        };
                        fee_counts_by_extrinsic[extrinsic_index as usize][fee_event_index] += 1;
                        fee_amounts_by_extrinsic[extrinsic_index as usize][fee_event_index] =
                            Some(amount);
                    }
                }
                "initialization" | "finalization" => require!(
                    event.phase.extrinsic_index.is_none(),
                    "non-apply event retained an extrinsic index"
                ),
                phase => return Err(format!("unknown finalized event phase {phase}")),
            }
        }
        require!(
            success_by_extrinsic.iter().all(|count| *count == 1),
            "one or more extrinsics at block {} lacked exactly one success event",
            block.number
        );
        for (extrinsic_index, extrinsic) in block.extrinsics.iter().enumerate() {
            let expected_fee_count = u64::from(extrinsic.signed);
            require!(
                fee_counts_by_extrinsic[extrinsic_index]
                    .iter()
                    .all(|count| *count == expected_fee_count),
                "extrinsic {}:{} lacked exact withdraw/burn/payment event counts",
                block.number,
                extrinsic_index
            );
            if extrinsic.signed {
                let amounts = fee_amounts_by_extrinsic[extrinsic_index];
                let withdraw = amounts[0].ok_or_else(|| {
                    format!(
                        "signed extrinsic lacks a withdraw amount at {}:{extrinsic_index}",
                        block.number
                    )
                })?;
                let burned_debt = amounts[1].ok_or_else(|| {
                    format!(
                        "signed extrinsic lacks a burned debt amount at {}:{extrinsic_index}",
                        block.number
                    )
                })?;
                let actual_fee = amounts[2].ok_or_else(|| {
                    format!(
                        "signed extrinsic lacks an actual fee at {}:{extrinsic_index}",
                        block.number
                    )
                })?;
                require!(
                    withdraw > 0 && withdraw == burned_debt && withdraw == actual_fee,
                    "fee withdrawal/burn/payment identity drifted at {}:{}",
                    block.number,
                    extrinsic_index
                );
            }
        }
    }
    let expected_submission_count = u64::try_from(evidence.submissions.len())
        .map_err(|error| format!("submission count overflows u64: {error}"))?;
    let expected_event_counts = BTreeMap::from([
        ("balances.BurnedDebt".to_owned(), expected_submission_count),
        ("balances.Withdraw".to_owned(), expected_submission_count),
        ("cubikan.Accepted".to_owned(), expected_submission_count),
        ("system.ExtrinsicSuccess".to_owned(), success_count),
        (
            "transactionPayment.TransactionFeePaid".to_owned(),
            expected_submission_count,
        ),
    ]);
    require!(
        census
            .endpoint_a
            .blocks
            .last()
            .map(|block| block.hash.as_str())
            == Some(final_checkpoint.hash.as_str())
            && seen_submission_hashes.len() == evidence.submissions.len()
            && census.expected_submission_hashes
                == evidence
                    .submissions
                    .iter()
                    .map(|submission| submission.extrinsic_hash.clone())
                    .collect::<Vec<_>>()
            && census.expected_signed_extrinsic_count == evidence.submissions.len() as u64
            && census.signed_extrinsic_count == signed_count
            && signed_count == evidence.submissions.len() as u64
            && census.unsigned_inherent_count == unsigned_count
            && census.accepted_event_count == accepted_count
            && accepted_count == evidence.submissions.len() as u64
            && census.extrinsic_success_count == success_count
            && census.extrinsic_failed_count == 0
            && census.event_counts == observed_event_counts
            && observed_event_counts == expected_event_counts
            && census.external_actions.is_empty()
            && census.forbidden_events.is_empty(),
        "finalized chain census counts/actions/events drifted"
    );
    let expected_forbidden_keys = fixture
        .audit
        .zero_count_keys
        .iter()
        .filter(|key| !matches!(key.as_str(), "public_rpc" | "secret"))
        .map(|key| (key.clone(), 0_u64))
        .collect::<BTreeMap<_, _>>();
    require!(
        census.forbidden_counts == expected_forbidden_keys,
        "finalized chain census forbidden category inventory was not exact zero"
    );
    Ok(())
}

#[derive(Default)]
struct RelayInherentObservation {
    backed_candidate_count: u64,
    candidate_para_ids: Vec<u64>,
    new_validation_code_field_count: u64,
    new_validation_code_count: u64,
    upward_message_field_count: u64,
    upward_message_count: u64,
    upward_signal_separator_count: u64,
    upward_signal_count: u64,
    horizontal_message_field_count: u64,
    horizontal_message_count: u64,
    processed_downward_message_field_count: u64,
    processed_downward_message_count: u64,
    dispute_field_count: u64,
    dispute_statement_count: u64,
}

fn checked_increment(total: &mut u64, amount: u64, label: &str) -> Result<(), String> {
    *total = total
        .checked_add(amount)
        .ok_or_else(|| format!("{label} overflowed"))?;
    Ok(())
}

fn normalized_codec_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| *character != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn relay_nonnegative_integer(value: &Value, path: &str) -> Result<u64, String> {
    if let Some(number) = value.as_u64() {
        return Ok(number);
    }
    if let Some(text) = value.as_str() {
        require!(
            (text == "0"
                || (!text.starts_with('0')
                    && text.chars().all(|character| character.is_ascii_digit()))),
            "{path} is not a canonical nonnegative integer"
        );
        return text
            .parse::<u64>()
            .map_err(|error| format!("{path} is outside the u64 range: {error}"));
    }
    Err(format!("{path} is not a nonnegative integer"))
}

fn inspect_relay_inherent_value(
    value: &Value,
    observation: &mut RelayInherentObservation,
    path: &str,
) -> Result<(), String> {
    match value {
        Value::Array(entries) => {
            for (index, entry) in entries.iter().enumerate() {
                inspect_relay_inherent_value(entry, observation, &format!("{path}/{index}"))?;
            }
        }
        Value::Object(entries) => {
            for (key, child) in entries {
                let child_path = format!("{path}/{key}");
                match normalized_codec_key(key).as_str() {
                    "backedcandidates" => {
                        let candidates = child
                            .as_array()
                            .ok_or_else(|| format!("{child_path} is not an array"))?;
                        checked_increment(
                            &mut observation.backed_candidate_count,
                            candidates.len() as u64,
                            "relay backed candidate count",
                        )?;
                    }
                    "paraid" => observation
                        .candidate_para_ids
                        .push(relay_nonnegative_integer(child, &child_path)?),
                    "newvalidationcode" => {
                        checked_increment(
                            &mut observation.new_validation_code_field_count,
                            1,
                            "relay new validation code field count",
                        )?;
                        if !child.is_null() {
                            checked_increment(
                                &mut observation.new_validation_code_count,
                                1,
                                "relay new validation code count",
                            )?;
                        }
                    }
                    "upwardmessages" => {
                        let messages = child
                            .as_array()
                            .ok_or_else(|| format!("{child_path} is not an array"))?;
                        let byte_lengths = messages
                            .iter()
                            .enumerate()
                            .map(|(index, message)| {
                                let message_path = format!("{child_path}/{index}");
                                let text = message
                                    .as_str()
                                    .ok_or_else(|| format!("{message_path} is not a hex string"))?;
                                if text == "0x" {
                                    Ok(0)
                                } else {
                                    decode_lower_hex(text)
                                        .map(|bytes| bytes.len())
                                        .map_err(|error| format!("{message_path}: {error}"))
                                }
                            })
                            .collect::<Result<Vec<_>, String>>()?;
                        let separator_indexes = byte_lengths
                            .iter()
                            .enumerate()
                            .filter_map(|(index, length)| (*length == 0).then_some(index))
                            .collect::<Vec<_>>();
                        require!(
                            separator_indexes.len() <= 1,
                            "{child_path} contains duplicate UMP signal separators"
                        );
                        let separator_index =
                            separator_indexes.first().copied().unwrap_or(messages.len());
                        let signal_count = if separator_indexes.is_empty() {
                            0
                        } else {
                            messages.len() - separator_index - 1
                        };
                        require!(
                            separator_indexes.is_empty()
                                || ((1..=2).contains(&signal_count)
                                    && byte_lengths[separator_index + 1..]
                                        .iter()
                                        .all(|length| *length > 0)),
                            "{child_path} contains a malformed UMP protocol-signal suffix"
                        );
                        checked_increment(
                            &mut observation.upward_message_field_count,
                            1,
                            "relay upward message field count",
                        )?;
                        checked_increment(
                            &mut observation.upward_message_count,
                            separator_index as u64,
                            "relay upward message count",
                        )?;
                        checked_increment(
                            &mut observation.upward_signal_separator_count,
                            separator_indexes.len() as u64,
                            "relay UMP signal separator count",
                        )?;
                        checked_increment(
                            &mut observation.upward_signal_count,
                            signal_count as u64,
                            "relay UMP protocol signal count",
                        )?;
                    }
                    "horizontalmessages" => {
                        let messages = child
                            .as_array()
                            .ok_or_else(|| format!("{child_path} is not an array"))?;
                        checked_increment(
                            &mut observation.horizontal_message_field_count,
                            1,
                            "relay horizontal message field count",
                        )?;
                        checked_increment(
                            &mut observation.horizontal_message_count,
                            messages.len() as u64,
                            "relay horizontal message count",
                        )?;
                    }
                    "processeddownwardmessages" => {
                        checked_increment(
                            &mut observation.processed_downward_message_field_count,
                            1,
                            "relay processed downward message field count",
                        )?;
                        checked_increment(
                            &mut observation.processed_downward_message_count,
                            relay_nonnegative_integer(child, &child_path)?,
                            "relay processed downward message count",
                        )?;
                    }
                    "disputes" => {
                        let disputes = child
                            .as_array()
                            .ok_or_else(|| format!("{child_path} is not an array"))?;
                        checked_increment(
                            &mut observation.dispute_field_count,
                            1,
                            "relay dispute field count",
                        )?;
                        checked_increment(
                            &mut observation.dispute_statement_count,
                            disputes.len() as u64,
                            "relay dispute statement count",
                        )?;
                    }
                    _ => {}
                }
                inspect_relay_inherent_value(child, observation, &child_path)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn collect_relay_named_integers(
    value: &Value,
    normalized_name: &str,
    result: &mut Vec<u64>,
    path: &str,
) -> Result<(), String> {
    match value {
        Value::Array(entries) => {
            for (index, entry) in entries.iter().enumerate() {
                collect_relay_named_integers(
                    entry,
                    normalized_name,
                    result,
                    &format!("{path}/{index}"),
                )?;
            }
        }
        Value::Object(entries) => {
            for (key, child) in entries {
                let child_path = format!("{path}/{key}");
                if normalized_codec_key(key) == normalized_name {
                    result.push(relay_nonnegative_integer(child, &child_path)?);
                }
                collect_relay_named_integers(child, normalized_name, result, &child_path)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn assert_relay_chain_census(journey: &JourneyRun) -> Result<(), String> {
    let fixture = &journey.fixture;
    let evidence = &journey.evidence;
    let census = &evidence.audit.relay_chain_census;
    let census_fixture = &fixture.audit.relay_chain_census;
    let minimum_session_rotation_count = census_fixture
        .minimum_session_rotation_count
        .ok_or_else(|| "relay census fixture lacks its session rotation minimum".to_owned())?;
    let expected_initial_spot_price = census_fixture
        .expected_initial_spot_price
        .ok_or_else(|| "relay census fixture lacks its initial spot price".to_owned())?;
    let expected_grandpa_authorities = census_fixture
        .expected_grandpa_authorities
        .as_ref()
        .ok_or_else(|| "relay census fixture lacks its exact Grandpa authorities".to_owned())?;
    require!(
        census.format == "cubikan-finalized-relay-chain-census-v1"
            && census.range_start == census_fixture.range_start
            && census.range_start == 0
            && census.maximum_retained_bytes == census_fixture.maximum_retained_bytes
            && census.minimum_session_rotation_count == minimum_session_rotation_count
            && census.expected_initial_spot_price == expected_initial_spot_price
            && expected_initial_spot_price == 10_000_000
            && &census.expected_grandpa_authorities == expected_grandpa_authorities
            && census.allowed_unsigned_calls == census_fixture.allowed_unsigned_calls
            && census.allowed_events == census_fixture.allowed_events
            && census.endpoints_equal
            && census.endpoint_a.endpoint == "relay-a"
            && census.endpoint_b.endpoint == "relay-b"
            && census.endpoint_a.range_start == 0
            && census.endpoint_b.range_start == 0
            && census.endpoint_a.range_end == census.range_end
            && census.endpoint_b.range_end == census.range_end,
        "finalized relay chain census format/range/endpoints diverged"
    );
    let canonical_census = canonical_json_bytes(census)?;
    require!(
        canonical_census.len() as u64 <= census.maximum_retained_bytes
            && census.endpoint_a.retained_bytes <= census.maximum_retained_bytes,
        "finalized relay chain census exceeded an independent retained byte bound"
    );
    let census_artifact = evidence
        .audit
        .artifacts
        .iter()
        .find(|artifact| artifact.logical_path == "action/finalized-relay-chain-census.json")
        .ok_or_else(|| "audit lacks the finalized relay chain census artifact".to_owned())?;
    require!(
        census_artifact.kind == "action"
            && census_artifact.read.bytes_read == canonical_census.len() as u64
            && census_artifact.read.sha256 == sha256_bytes(&canonical_census)?,
        "finalized relay chain census artifact is not the exact canonical evidence bytes"
    );

    let expected_block_count = usize::try_from(census.range_end)
        .map_err(|error| format!("relay census range end overflows usize: {error}"))?
        .checked_add(1)
        .ok_or_else(|| "relay census block count overflowed".to_owned())?;
    require!(
        census.endpoint_a.blocks.len() == expected_block_count,
        "finalized relay census did not retain every block from genesis through its checkpoint"
    );
    let retained_a = census
        .endpoint_a
        .blocks
        .iter()
        .try_fold(0_u64, |total, block| {
            total
                .checked_add(canonical_json_bytes(block)?.len() as u64)
                .ok_or_else(|| "relay census retained byte count overflowed".to_owned())
        })?;
    require!(
        retained_a == census.endpoint_a.retained_bytes
            && census.endpoint_a.transcript_sha256
                == canonical_json_sha256(&census.endpoint_a.blocks)?
            && census.endpoint_b.transcript_sha256 == census.endpoint_a.transcript_sha256
            && census.endpoint_b.block_count
                == u64::try_from(expected_block_count)
                    .map_err(|error| format!("relay census block count overflows u64: {error}"))?,
        "finalized relay census retained byte/transcript attestation drifted"
    );

    let allowed_calls = census_fixture
        .allowed_unsigned_calls
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let allowed_events = census_fixture
        .allowed_events
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut event_counts = census_fixture
        .allowed_events
        .iter()
        .map(|event| (event.clone(), 0_u64))
        .collect::<BTreeMap<_, _>>();
    let expected_grandpa_event_data = serde_json::to_value([expected_grandpa_authorities])
        .map_err(|error| format!("cannot encode expected Grandpa authority event data: {error}"))?;
    let mut observation = RelayInherentObservation::default();
    let mut event_candidate_para_ids = Vec::new();
    let mut historical_root_session_indices = Vec::new();
    let mut new_session_indices = Vec::new();
    let mut new_queued_count = 0_u64;
    let mut grandpa_new_authorities_count = 0_u64;
    let mut initial_spot_price = None;
    let mut signed_count = 0_u64;
    let mut unsigned_count = 0_u64;
    let mut timestamp_count = 0_u64;
    let mut para_inherent_count = 0_u64;
    let mut success_count = 0_u64;
    let mut failed_count = 0_u64;
    let mut previous_hash: Option<&str> = None;
    for (expected_number, block) in census.endpoint_a.blocks.iter().enumerate() {
        require!(
            block.number == expected_number as u64
                && is_chain_hash(&block.hash)
                && is_chain_hash(&block.state_root)
                && is_chain_hash(&block.extrinsics_root)
                && if let Some(previous_hash) = previous_hash {
                    block.parent_hash == previous_hash
                } else {
                    block.parent_hash == format!("0x{}", "0".repeat(64))
                }
                && sha256_bytes(&decode_lower_hex(&block.header_scale_hex)?)?
                    == block.header_sha256
                && sha256_bytes(&decode_lower_hex(&block.system_events_scale_hex)?)?
                    == block.system_events_sha256,
            "finalized relay census block/header linkage drifted at {}",
            block.number
        );
        previous_hash = Some(&block.hash);
        if block.number == 0 {
            require!(
                block.extrinsics.is_empty() && block.events.is_empty(),
                "relay genesis block retained an extrinsic or event"
            );
        } else {
            require!(
                block
                    .extrinsics
                    .iter()
                    .map(|extrinsic| format!("{}.{}", extrinsic.section, extrinsic.method))
                    .eq(census_fixture.allowed_unsigned_calls.iter().cloned()),
                "relay block {} did not contain the exact ordered inherent set",
                block.number
            );
        }

        let mut success_by_extrinsic = vec![0_u64; block.extrinsics.len()];
        for (expected_index, extrinsic) in block.extrinsics.iter().enumerate() {
            let call = format!("{}.{}", extrinsic.section, extrinsic.method);
            require!(
                extrinsic.index == expected_index as u64
                    && is_chain_hash(&extrinsic.hash)
                    && decode_lower_hex(&extrinsic.call_index)?.len() == 2
                    && extrinsic.args_sha256 == canonical_json_sha256(&extrinsic.args)?
                    && extrinsic.scale_sha256
                        == sha256_bytes(&decode_lower_hex(&extrinsic.scale_hex)?)?
                    && !extrinsic.signed
                    && extrinsic.signer.is_none()
                    && allowed_calls.contains(call.as_str()),
                "unexpected or malformed finalized relay call at {}:{}: {call}",
                block.number,
                extrinsic.index
            );
            if extrinsic.signed {
                checked_increment(&mut signed_count, 1, "relay signed extrinsic count")?;
            } else {
                checked_increment(&mut unsigned_count, 1, "relay unsigned inherent count")?;
            }
            match call.as_str() {
                "timestamp.set" => {
                    checked_increment(&mut timestamp_count, 1, "relay timestamp inherent count")?
                }
                "paraInherent.enter" => {
                    checked_increment(&mut para_inherent_count, 1, "relay para inherent count")?;
                    for (index, argument) in extrinsic.args.iter().enumerate() {
                        inspect_relay_inherent_value(
                            argument,
                            &mut observation,
                            &format!("$args/{index}"),
                        )?;
                    }
                }
                _ => return Err(format!("unexpected relay inherent {call}")),
            }
        }

        for (expected_index, event) in block.events.iter().enumerate() {
            let event_key = format!("{}.{}", event.section, event.method);
            require!(
                event.index == expected_index as u64
                    && event.data_sha256 == canonical_json_sha256(&event.data)?
                    && event.scale_sha256 == sha256_bytes(&decode_lower_hex(&event.scale_hex)?)?
                    && event.topics.iter().all(|topic| is_chain_hash(topic))
                    && allowed_events.contains(event_key.as_str()),
                "unexpected or malformed finalized relay event at {}:{}: {event_key}",
                block.number,
                event.index
            );
            checked_increment(
                event_counts
                    .get_mut(&event_key)
                    .ok_or_else(|| format!("relay event count key is absent: {event_key}"))?,
                1,
                "relay event count",
            )?;
            match event_key.as_str() {
                "onDemandAssignmentProvider.SpotPriceSet" => {
                    let fields = event
                        .data
                        .as_array()
                        .filter(|fields| fields.len() == 1)
                        .ok_or_else(|| {
                            format!(
                                "relay initial spot-price event shape drifted at {}:{}",
                                block.number, event.index
                            )
                        })?;
                    let spot_price = canonical_json_u128(
                        &fields[0],
                        &format!(
                            "onDemandAssignmentProvider.SpotPriceSet at {}:{}",
                            block.number, event.index
                        ),
                    )?;
                    require!(
                        block.number == 1
                            && event.index == 0
                            && event.phase.kind == "initialization"
                            && event.phase.extrinsic_index.is_none()
                            && event.topics.is_empty()
                            && spot_price == expected_initial_spot_price
                            && initial_spot_price.is_none(),
                        "relay initial spot-price event payload/position drifted at {}:{}",
                        block.number,
                        event.index
                    );
                    initial_spot_price = Some(spot_price);
                }
                "system.ExtrinsicSuccess" => {
                    require!(
                        event.phase.kind == "apply_extrinsic"
                            && event.phase.extrinsic_index.is_some_and(|index| {
                                usize::try_from(index)
                                    .ok()
                                    .is_some_and(|index| index < block.extrinsics.len())
                            }),
                        "relay success event phase drifted at {}:{}",
                        block.number,
                        event.index
                    );
                    let extrinsic_index = event
                        .phase
                        .extrinsic_index
                        .expect("the preceding predicate established an extrinsic index")
                        as usize;
                    checked_increment(
                        &mut success_by_extrinsic[extrinsic_index],
                        1,
                        "per-inherent relay success count",
                    )?;
                    checked_increment(&mut success_count, 1, "relay success event count")?;
                }
                "paraInclusion.CandidateBacked"
                | "paraInclusion.CandidateIncluded"
                | "paraInclusion.CandidateTimedOut" => {
                    let mut para_ids = Vec::new();
                    collect_relay_named_integers(&event.data, "paraid", &mut para_ids, "$event")?;
                    require!(
                        event.phase.kind == "apply_extrinsic"
                            && event.phase.extrinsic_index == Some(1)
                            && !para_ids.is_empty()
                            && para_ids.iter().all(|para_id| {
                                *para_id
                                    == u64::from(fixture.checkpoints.pre_mutation_readiness.para_id)
                            }),
                        "relay candidate event payload/phase drifted at {}:{}",
                        block.number,
                        event.index
                    );
                    event_candidate_para_ids.extend(para_ids);
                }
                "historical.RootStored" => {
                    require!(
                        event.phase.kind == "initialization"
                            && event.phase.extrinsic_index.is_none(),
                        "relay historical root event phase drifted at {}:{}",
                        block.number,
                        event.index
                    );
                    historical_root_session_indices.push(exact_single_event_u64(
                        &event.data,
                        &format!("historical.RootStored at {}:{}", block.number, event.index),
                    )?);
                }
                "session.NewSession" => {
                    require!(
                        event.phase.kind == "initialization"
                            && event.phase.extrinsic_index.is_none(),
                        "relay new-session event phase drifted at {}:{}",
                        block.number,
                        event.index
                    );
                    new_session_indices.push(exact_single_event_u64(
                        &event.data,
                        &format!("session.NewSession at {}:{}", block.number, event.index),
                    )?);
                }
                "session.NewQueued" => {
                    require!(
                        event.phase.kind == "initialization"
                            && event.phase.extrinsic_index.is_none()
                            && event.data.as_array().is_some_and(Vec::is_empty),
                        "relay new-queued event payload/phase drifted at {}:{}",
                        block.number,
                        event.index
                    );
                    checked_increment(&mut new_queued_count, 1, "relay new-queued event count")?;
                }
                "grandpa.NewAuthorities" => {
                    require!(
                        event.phase.kind == "finalization"
                            && event.phase.extrinsic_index.is_none()
                            && event.data == expected_grandpa_event_data,
                        "relay Grandpa authority event payload/phase drifted at {}:{}",
                        block.number,
                        event.index
                    );
                    checked_increment(
                        &mut grandpa_new_authorities_count,
                        1,
                        "relay Grandpa authority event count",
                    )?;
                }
                "system.ExtrinsicFailed" => {
                    checked_increment(&mut failed_count, 1, "relay failed event count")?;
                    return Err(format!(
                        "relay census retained an extrinsic failure at {}:{}",
                        block.number, event.index
                    ));
                }
                _ => return Err(format!("unexpected relay event {event_key}")),
            }
        }
        require!(
            success_by_extrinsic.iter().all(|count| *count == 1),
            "one or more relay inherents at block {} lacked exactly one success event",
            block.number
        );
    }

    let candidate_para_ids = observation
        .candidate_para_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let unique_event_para_ids = event_candidate_para_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let session_rotation_count = u64::try_from(new_session_indices.len())
        .map_err(|error| format!("relay session rotation count overflows u64: {error}"))?;
    let historical_root_indices_match = historical_root_session_indices.len()
        == new_session_indices.len()
        && historical_root_session_indices
            .iter()
            .zip(&new_session_indices)
            .all(|(historical_index, session_index)| {
                session_index.checked_add(1) == Some(*historical_index)
            });
    let expected_grandpa_new_authorities_count = session_rotation_count
        .checked_sub(1)
        .ok_or_else(|| "relay census lacks its first session rotation".to_owned())?;
    let initial_spot_price = initial_spot_price
        .ok_or_else(|| "relay census lacks its exact initial spot-price event".to_owned())?;
    let initial_spot_price_set_count = *event_counts
        .get("onDemandAssignmentProvider.SpotPriceSet")
        .ok_or_else(|| "relay census lacks its initial spot-price event count".to_owned())?;
    let non_genesis_blocks = census.range_end;
    let expected_unsigned_count = non_genesis_blocks
        .checked_mul(census_fixture.allowed_unsigned_calls.len() as u64)
        .ok_or_else(|| "relay expected inherent count overflowed".to_owned())?;
    require!(
        census
            .endpoint_a
            .blocks
            .last()
            .map(|block| block.hash.as_str())
            == Some(census.checkpoint_hash.as_str())
            && signed_count == 0
            && unsigned_count == expected_unsigned_count
            && timestamp_count == non_genesis_blocks
            && para_inherent_count == non_genesis_blocks
            && success_count == unsigned_count
            && failed_count == 0
            && initial_spot_price_set_count == 1
            && initial_spot_price == expected_initial_spot_price
            && session_rotation_count >= minimum_session_rotation_count
            && historical_root_indices_match
            && new_queued_count == session_rotation_count
            && grandpa_new_authorities_count == expected_grandpa_new_authorities_count
            && observation.backed_candidate_count > 0
            && observation.candidate_para_ids.len() as u64 == observation.backed_candidate_count
            && candidate_para_ids
                == [u64::from(
                    fixture.checkpoints.pre_mutation_readiness.para_id,
                )]
            && unique_event_para_ids
                == [u64::from(
                    fixture.checkpoints.pre_mutation_readiness.para_id,
                )]
            && observation.new_validation_code_field_count == observation.backed_candidate_count
            && observation.upward_message_field_count == observation.backed_candidate_count
            && observation.horizontal_message_field_count == observation.backed_candidate_count
            && observation.processed_downward_message_field_count
                == observation.backed_candidate_count
            && observation.dispute_field_count == para_inherent_count
            && observation.new_validation_code_count == 0
            && observation.upward_message_count == 0
            && observation.upward_signal_separator_count == observation.backed_candidate_count
            && observation.upward_signal_count >= observation.backed_candidate_count
            && observation.upward_signal_count
                <= observation
                    .backed_candidate_count
                    .checked_mul(2)
                    .ok_or_else(|| "relay maximum UMP signal count overflowed".to_owned())?
            && observation.horizontal_message_count == 0
            && observation.processed_downward_message_count == 0
            && observation.dispute_statement_count == 0,
        "relay census contains an unexpected action, failure, candidate, message, dispute, or runtime update"
    );
    require!(
        census.signed_extrinsic_count == signed_count
            && census.unsigned_inherent_count == unsigned_count
            && census.timestamp_inherent_count == timestamp_count
            && census.para_inherent_count == para_inherent_count
            && census.extrinsic_success_count == success_count
            && census.extrinsic_failed_count == failed_count
            && census.session_rotation_count == session_rotation_count
            && census.historical_root_session_indices == historical_root_session_indices
            && census.new_session_indices == new_session_indices
            && census.new_queued_count == new_queued_count
            && census.grandpa_new_authorities_count == grandpa_new_authorities_count
            && census.initial_spot_price == initial_spot_price
            && census.backed_candidate_count == observation.backed_candidate_count
            && census.candidate_para_ids == candidate_para_ids
            && census.event_candidate_para_ids == unique_event_para_ids
            && census.new_validation_code_field_count
                == observation.new_validation_code_field_count
            && census.new_validation_code_count == observation.new_validation_code_count
            && census.upward_message_field_count == observation.upward_message_field_count
            && census.upward_message_count == observation.upward_message_count
            && census.upward_signal_separator_count == observation.upward_signal_separator_count
            && census.upward_signal_count == observation.upward_signal_count
            && census.horizontal_message_field_count == observation.horizontal_message_field_count
            && census.horizontal_message_count == observation.horizontal_message_count
            && census.processed_downward_message_field_count
                == observation.processed_downward_message_field_count
            && census.processed_downward_message_count
                == observation.processed_downward_message_count
            && census.dispute_field_count == observation.dispute_field_count
            && census.dispute_statement_count == observation.dispute_statement_count
            && census.event_counts == event_counts
            && census.external_actions.is_empty()
            && census.forbidden_events.is_empty(),
        "relay census retained summary counters/maps drifted from its full transcript"
    );

    let expected_forbidden_keys = fixture
        .audit
        .zero_count_keys
        .iter()
        .filter(|key| !matches!(key.as_str(), "public_rpc" | "secret"))
        .map(|key| (key.clone(), 0_u64))
        .collect::<BTreeMap<_, _>>();
    require!(
        census.forbidden_counts == expected_forbidden_keys,
        "finalized relay census forbidden category inventory was not exact zero"
    );

    let runtime = &census.runtime_identity;
    let runtime_a = &runtime.endpoint_a;
    let runtime_b = &runtime.endpoint_b;
    let genesis_hash = census
        .endpoint_a
        .blocks
        .first()
        .map(|block| block.hash.as_str())
        .ok_or_else(|| "relay census lacks genesis".to_owned())?;
    require!(
        runtime.endpoints_equal
            && runtime.unchanged
            && runtime_a.endpoint == "relay-a"
            && runtime_b.endpoint == "relay-b"
            && runtime_a.genesis == runtime_b.genesis
            && runtime_a.range_end == runtime_b.range_end
            && runtime_a.genesis.block_hash == genesis_hash
            && runtime_b.genesis.block_hash == genesis_hash
            && runtime_a.range_end.block_hash == census.checkpoint_hash
            && runtime_b.range_end.block_hash == census.checkpoint_hash
            && runtime_a.genesis.runtime_spec_version > 0
            && runtime_a.genesis.runtime_spec_version == runtime_a.range_end.runtime_spec_version
            && runtime_a.genesis.runtime_code_sha256 == runtime_a.range_end.runtime_code_sha256
            && runtime_a.genesis.metadata_sha256 == runtime_a.range_end.metadata_sha256
            && runtime_b.genesis.runtime_spec_version == runtime_b.range_end.runtime_spec_version
            && runtime_b.genesis.runtime_code_sha256 == runtime_b.range_end.runtime_code_sha256
            && runtime_b.genesis.metadata_sha256 == runtime_b.range_end.metadata_sha256
            && is_sha256(&runtime_a.genesis.runtime_code_sha256)
            && is_sha256(&runtime_a.genesis.metadata_sha256),
        "relay runtime identity changed, diverged, or did not bind census endpoints"
    );
    Ok(())
}

fn first_disallowed_diagnostic_url(
    text: &str,
    allowed_hosts: &BTreeSet<&str>,
    allowed_ports: &BTreeSet<&str>,
) -> Option<String> {
    for (separator, _) in text.match_indices("://") {
        let scheme_start = text[..separator]
            .char_indices()
            .rev()
            .take_while(|(_, character)| character.is_ascii_alphanumeric() || *character == '+')
            .last()
            .map_or(separator, |(index, _)| index);
        let scheme = text[scheme_start..separator].to_ascii_lowercase();
        if !matches!(
            scheme.as_str(),
            "git+ssh" | "ssh" | "git" | "http" | "https" | "ws" | "wss"
        ) {
            continue;
        }
        let candidate = text[scheme_start..]
            .split(|character: char| {
                character.is_ascii_whitespace() || matches!(character, '"' | '\'' | '<' | '>')
            })
            .next()
            .unwrap_or_default();
        let authority = candidate
            .split_once("://")
            .map(|(_, remainder)| remainder)
            .unwrap_or_default()
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default();
        if authority.is_empty() || authority.contains('@') {
            return Some(candidate.to_owned());
        }
        let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
            let Some((host, port)) = bracketed.split_once("]:") else {
                return Some(candidate.to_owned());
            };
            (host, port)
        } else {
            let Some((host, port)) = authority.rsplit_once(':') else {
                return Some(candidate.to_owned());
            };
            (host, port)
        };
        if !allowed_hosts.contains(host) || !allowed_ports.contains(port) {
            return Some(candidate.to_owned());
        }
    }
    None
}

fn first_forbidden_diagnostic_marker(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    for name in [
        "aws_secret_access_key",
        "git_askpass",
        "git_config_global",
        "git_config_system",
        "ssh_auth_sock",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
    ] {
        if contains_exact_environment_assignment(&lower, name) {
            return Some(name.to_ascii_uppercase());
        }
    }
    for name in PROHIBITED_OUTPUT_KEYS {
        if contains_exact_json_key_or_bare_assignment(&lower, name) {
            return Some(name.to_owned());
        }
    }
    None
}

fn contains_exact_environment_assignment(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(start, _)| {
        let end = start + name.len();
        has_identifier_boundary_before(text.as_bytes(), start)
            && text.as_bytes().get(end) == Some(&b'=')
    })
}

fn contains_exact_json_key_or_bare_assignment(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(start, _)| {
        let end = start + name.len();
        is_exact_json_key(text.as_bytes(), start, end)
            || is_exact_bare_assignment(text.as_bytes(), start, end)
    })
}

fn is_exact_json_key(bytes: &[u8], start: usize, end: usize) -> bool {
    let Some(opening_quote) = start.checked_sub(1) else {
        return false;
    };
    if bytes.get(opening_quote) != Some(&b'"') || bytes.get(end) != Some(&b'"') {
        return false;
    }
    if bytes[..opening_quote]
        .iter()
        .rfind(|byte| !is_json_whitespace(**byte))
        .is_some_and(|byte| !matches!(*byte, b'{' | b','))
    {
        return false;
    }
    let mut cursor = end + 1;
    while bytes
        .get(cursor)
        .is_some_and(|byte| is_json_whitespace(*byte))
    {
        cursor += 1;
    }
    bytes.get(cursor) == Some(&b':')
}

fn is_exact_bare_assignment(bytes: &[u8], start: usize, end: usize) -> bool {
    if !has_identifier_boundary_before(bytes, start) {
        return false;
    }
    let mut cursor = end;
    while matches!(bytes.get(cursor), Some(b' ' | b'\t')) {
        cursor += 1;
    }
    matches!(bytes.get(cursor), Some(b':' | b'='))
}

fn has_identifier_boundary_before(bytes: &[u8], start: usize) -> bool {
    start == 0 || !bytes[start - 1].is_ascii_alphanumeric() && bytes[start - 1] != b'_'
}

fn is_json_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

fn assert_orchestrator_diagnostics(journey: &JourneyRun) -> Result<(), String> {
    let audit = &journey.evidence.audit;
    let log = &audit.orchestrator_log;
    let diagnostics = &log.diagnostics;
    require!(
        log.format == "cubikan-orchestrator-log-v1"
            && log.pid == journey.evidence.topology.orchestrator.pid
            && log.node_roles
                == journey
                    .fixture
                    .topology
                    .nodes
                    .iter()
                    .map(|node| node.role.clone())
                    .collect::<Vec<_>>()
            && log.normalizer_rejection_matrix.case_count
                == journey.fixture.topology.normalizer_rejection_cases.len() as u64
            && is_sha256(&log.normalizer_rejection_matrix.transcript_sha256)
            && diagnostics.format == "cubikan-bounded-orchestrator-diagnostics-v1"
            && diagnostics.limit_bytes == 1_048_576
            && diagnostics.total_bytes <= diagnostics.limit_bytes
            && diagnostics.discarded_bytes == 0
            && !diagnostics.overflowed
            && diagnostics.forbidden_hits.is_empty()
            && diagnostics.nonloopback_hits.is_empty(),
        "orchestrator log/diagnostic format, roles, bound, or zero-hit claims drifted"
    );
    let mut aggregate = Vec::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    for (index, record) in diagnostics.records.iter().enumerate() {
        let bytes = decode_lower_hex(&record.bytes_hex)?;
        require!(
            record.sequence == index as u64
                && record.byte_length == bytes.len() as u64
                && record.byte_length > 0
                && matches!(record.stream.as_str(), "stdout" | "stderr"),
            "orchestrator diagnostic record {index} is reordered, empty, or malformed"
        );
        aggregate.extend_from_slice(&bytes);
        match record.stream.as_str() {
            "stdout" => stdout.extend_from_slice(&bytes),
            "stderr" => stderr.extend_from_slice(&bytes),
            _ => unreachable!("validated above"),
        }
    }
    require!(
        aggregate.len() as u64 == diagnostics.total_bytes
            && sha256_bytes(&aggregate)? == diagnostics.aggregate_sha256
            && sha256_bytes(&stdout)? == diagnostics.stdout_sha256
            && sha256_bytes(&stderr)? == diagnostics.stderr_sha256,
        "orchestrator diagnostic records do not reproduce exact stream/aggregate hashes"
    );
    let allowed_hosts = journey
        .fixture
        .audit
        .allowed_hosts
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let allowed_ports = journey
        .fixture
        .topology
        .nodes
        .iter()
        .flat_map(|node| node.listeners.iter())
        .filter_map(|address| address.rsplit_once(':').map(|(_, port)| port))
        .collect::<BTreeSet<_>>();
    for (label, bytes) in [
        ("aggregate", aggregate.as_slice()),
        ("stdout", stdout.as_slice()),
        ("stderr", stderr.as_slice()),
    ] {
        let text = String::from_utf8_lossy(bytes);
        require!(
            first_forbidden_diagnostic_marker(&text).is_none()
                && first_disallowed_diagnostic_url(&text, &allowed_hosts, &allowed_ports).is_none(),
            "orchestrator {label} diagnostics contain a secret/helper marker or public URL"
        );
    }
    let log_bytes = canonical_json_bytes(log)?;
    let artifact = audit
        .artifacts
        .iter()
        .find(|artifact| artifact.logical_path == "log/orchestrator.log")
        .ok_or_else(|| "audit lacks orchestrator log artifact".to_owned())?;
    require!(
        artifact.read.bytes_read == log_bytes.len() as u64
            && artifact.read.sha256 == sha256_bytes(&log_bytes)?,
        "orchestrator log artifact does not bind the independently consumed diagnostics"
    );
    Ok(())
}

fn bind_audit_artifact_bytes(
    audit: &AuditEvidence,
    logical_path: &str,
    expected: &[u8],
) -> Result<(), String> {
    let artifact = audit
        .artifacts
        .iter()
        .find(|artifact| artifact.logical_path == logical_path)
        .ok_or_else(|| format!("audit lacks required artifact {logical_path}"))?;
    require!(
        artifact.read.bytes_read == expected.len() as u64
            && artifact.read.sha256 == sha256_bytes(expected)?,
        "audit artifact {logical_path} does not bind the validated source/evidence bytes"
    );
    Ok(())
}

fn assert_audit_process_inventory(journey: &JourneyRun) -> Result<(), String> {
    let evidence = &journey.evidence;
    let inventory = &evidence.audit.process_inventory;
    let old_pid = evidence.restart.pid_before;
    let restarted = &evidence.restart.restarted_node;
    let expected_stopped = evidence
        .topology
        .node_processes
        .iter()
        .filter(|process| process.pid != old_pid)
        .collect::<Vec<_>>();
    let mut expected_restarted = expected_stopped.clone();
    let restarted_process = inventory
        .restarted_node_processes
        .iter()
        .find(|process| process.pid == evidence.restart.pid_after)
        .ok_or_else(|| "process inventory lacks restarted collator generation".to_owned())?;
    expected_restarted.push(restarted_process);
    expected_restarted.sort_by_key(|process| process.pid);
    require!(
        inventory.format == "cubikan-process-inventory-v1"
            && inventory.orchestrator.pid == evidence.topology.orchestrator.pid
            && inventory.orchestrator.privileges == evidence.topology.orchestrator.privileges
            && inventory.initial_node_processes == evidence.topology.node_processes
            && inventory.stopped_node_processes
                == expected_stopped.into_iter().cloned().collect::<Vec<_>>()
            && inventory.restarted_node_processes
                == expected_restarted.into_iter().cloned().collect::<Vec<_>>()
            && restarted_process.parent_pid == evidence.topology.orchestrator.pid
            && restarted_process.start_time_ticks == restarted.process_start_time_ticks_before
            && restarted_process.proc_exe_link == restarted.proc_exe_link
            && restarted_process.proc_cmdline_sha256 == restarted.proc_cmdline_sha256
            && inventory.remaining_node_processes.is_empty()
            && canonical_json_sha256(&inventory.pvf_workers)?
                == canonical_json_sha256(&evidence.pvf_workers)?,
        "typed process audit does not join exact initial/stopped/restarted/PVF generations"
    );
    bind_audit_artifact_bytes(
        &evidence.audit,
        "config/process-inventory.json",
        &canonical_json_bytes(inventory)?,
    )
}

fn validate_captured_socket_inventory(
    inventory: &AuditCapturedSocketInventoryEvidence,
    expected_addresses: &BTreeSet<String>,
    expected_pids: &BTreeSet<u32>,
    label: &str,
) -> Result<(), String> {
    require!(
        inventory.command == ["/usr/bin/ss", "-H", "-lntup"].map(str::to_owned)
            && inventory.stdout_sha256 == sha256_bytes(inventory.stdout_utf8.as_bytes())?
            && inventory.records.len() == expected_addresses.len(),
        "{label} socket transcript command/hash/count drifted"
    );
    let mut addresses = BTreeSet::new();
    for row in &inventory.records {
        require!(
            addresses.insert(row.local_address.clone())
                && expected_addresses.contains(&row.local_address)
                && matches!(
                    (row.protocol.as_str(), row.state.as_str()),
                    ("tcp", "LISTEN") | ("udp", "UNCONN")
                )
                && row.local_address.parse::<SocketAddr>().is_ok()
                && !row.pids.is_empty()
                && row.pids.windows(2).all(|pair| pair[0] < pair[1])
                && row.pids.iter().all(|pid| expected_pids.contains(pid))
                && row.raw_line.contains(&row.local_address)
                && row
                    .pids
                    .iter()
                    .all(|pid| row.raw_line.contains(&format!("pid={pid}"))),
            "{label} contains an unjoined/malformed listener row"
        );
    }
    require!(
        addresses == *expected_addresses,
        "{label} listener address set drifted"
    );
    Ok(())
}

fn assert_audit_listener_inventory(journey: &JourneyRun) -> Result<(), String> {
    let fixture = &journey.fixture;
    let evidence = &journey.evidence;
    let inventory = &evidence.audit.listener_inventory;
    let all_addresses = expected_listener_addresses(fixture);
    let all_address_set = all_addresses.into_iter().collect::<BTreeSet<_>>();
    let all_pids = evidence
        .topology
        .nodes
        .iter()
        .map(|node| node.pid)
        .collect::<BTreeSet<_>>();
    require!(
        inventory.format == "cubikan-listener-inventory-v1"
            && inventory.initial.command == ["/usr/bin/ss", "-H", "-lntup"].map(str::to_owned)
            && inventory.initial.stdout_sha256
                == sha256_bytes(inventory.initial.stdout_utf8.as_bytes())?
            && inventory.initial.records == evidence.topology.ss_records,
        "initial typed listener transcript diverged from E1 topology evidence"
    );
    let stopped_role = fixture
        .topology
        .nodes
        .iter()
        .find(|node| node.role == evidence.restart.role)
        .ok_or_else(|| "restart role lacks fixture topology".to_owned())?;
    let stopped_addresses = all_address_set
        .iter()
        .filter(|address| !stopped_role.listeners.contains(address))
        .cloned()
        .collect::<BTreeSet<_>>();
    let stopped_pids = all_pids
        .iter()
        .filter(|pid| **pid != evidence.restart.pid_before)
        .copied()
        .collect::<BTreeSet<_>>();
    validate_captured_socket_inventory(
        &inventory.stopped,
        &stopped_addresses,
        &stopped_pids,
        "stopped",
    )?;
    let mut restarted_pids = stopped_pids;
    require!(
        restarted_pids.insert(evidence.restart.pid_after),
        "restarted PID duplicated one survivor PID"
    );
    validate_captured_socket_inventory(
        &inventory.restarted,
        &all_address_set,
        &restarted_pids,
        "restarted",
    )?;
    validate_captured_socket_inventory(
        &inventory.released,
        &BTreeSet::new(),
        &BTreeSet::new(),
        "released",
    )?;
    require!(
        inventory.pvf_unix_sockets.observed
            == evidence.pvf_workers.runtime_monitor.observed_unix_sockets
            && inventory.pvf_unix_sockets.post_stop
                == evidence.pvf_workers.runtime_monitor.post_stop_unix_sockets,
        "listener audit PVF Unix socket inventory diverged"
    );
    bind_audit_artifact_bytes(
        &evidence.audit,
        "socket/listeners.txt",
        &canonical_json_bytes(inventory)?,
    )
}

fn assert_simple_audit_artifact_bindings(journey: &JourneyRun) -> Result<(), String> {
    let fixture = &journey.fixture;
    let evidence = &journey.evidence;
    let audit = &evidence.audit;
    bind_audit_artifact_bytes(
        audit,
        "action/reads.jsonl",
        &canonical_json_lines(&evidence.snapshots)?,
    )?;
    bind_audit_artifact_bytes(
        audit,
        "action/submissions.jsonl",
        &canonical_json_lines(&evidence.submissions)?,
    )?;
    let environments = evidence
        .topology
        .nodes
        .iter()
        .map(|node| (node.role.clone(), node.environment.clone()))
        .collect::<BTreeMap<_, _>>();
    bind_audit_artifact_bytes(
        audit,
        "config/environment-inventory.json",
        &canonical_json_bytes(&environments)?,
    )?;
    let config_pin = pinned_input(&fixture.pinned_inputs, "chain/config/zombienet.toml")?;
    let materialized_config = audit
        .artifacts
        .iter()
        .find(|artifact| artifact.logical_path == "config/materialized-zombienet.toml")
        .ok_or_else(|| "audit lacks materialized Zombienet config".to_owned())?;
    require!(
        materialized_config.read.bytes_read == config_pin.size
            && materialized_config.read.sha256 == config_pin.sha256,
        "materialized Zombienet audit copy differs from the pinned config"
    );
    bind_audit_artifact_bytes(audit, "fixture/journey-v1.json", FIXTURE_BYTES)?;
    bind_audit_artifact_bytes(
        audit,
        "fixture/recorded-association-v1.json",
        T1114_ASSOCIATION_BYTES,
    )?;
    bind_audit_artifact_bytes(
        audit,
        "journal/archive-probes.jsonl",
        &canonical_json_lines(&evidence.restart.archive_probes.transcript)?,
    )?;
    let finality = evidence
        .submissions
        .iter()
        .flat_map(|submission| submission.endpoint_observations.iter())
        .collect::<Vec<_>>();
    bind_audit_artifact_bytes(
        audit,
        "journal/finality.jsonl",
        &canonical_json_lines(&finality)?,
    )?;
    bind_audit_artifact_bytes(
        audit,
        "journal/submission-lanes.json",
        &canonical_json_bytes(&audit.submission_lanes)?,
    )?;
    assert_audit_process_inventory(journey)?;
    assert_audit_listener_inventory(journey)?;
    Ok(())
}

fn assert_e5(journey: &JourneyRun) -> Result<(), String> {
    let fixture = &journey.fixture;
    let evidence = &journey.evidence;
    let audit = &evidence.audit;
    assert_chain_census(journey)?;
    assert_relay_chain_census(journey)?;
    assert_orchestrator_diagnostics(journey)?;
    require!(
        audit.loopback_only
            && audit.synthetic_only
            && audit.dev_only
            && evidence.isolation.loopback_network_namespace
            && evidence.isolation.external_connectivity_denied,
        "audit did not prove loopback/synthetic/dev-only execution"
    );
    require!(
        audit.dev_signers == fixture.audit.dev_signers
            && audit.synthetic_origins == fixture.audit.synthetic_origins
            && audit.allowed_hosts == fixture.audit.allowed_hosts,
        "audit signer/origin/host inventory drifted"
    );
    let log_capture = &audit.node_log_capture;
    let frozen_log_parent = Path::new(&evidence.restart.frozen_log_path)
        .parent()
        .ok_or_else(|| "frozen collator log lacks a parent directory".to_owned())?;
    let expected_log_paths = fixture
        .topology
        .nodes
        .iter()
        .map(|node| {
            frozen_log_parent
                .join(format!("{}.log", node.generated_name))
                .to_string_lossy()
                .into_owned()
        })
        .collect::<BTreeSet<_>>();
    let captured_log_paths = log_capture
        .files
        .iter()
        .map(|file| file.file_path.clone())
        .collect::<BTreeSet<_>>();
    let captured_logs = log_capture
        .files
        .iter()
        .map(|file| (file.file_path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let log_owner_uid = fs::metadata(&journey.supported_root)
        .map_err(|error| format!("cannot stat supported root owner: {error}"))?
        .uid();
    require!(
        log_capture.format == "cubikan-bounded-node-log-capture-v1"
            && log_capture.per_file_limit_bytes == 1_048_576
            && log_capture.aggregate_limit_bytes == 4_194_304
            && !log_capture.overflowed
            && log_capture.discarded_bytes == 0
            && frozen_log_parent == Path::new(&evidence.topology.orchestrator.data_directory)
            && log_capture.files.len() == fixture.topology.nodes.len()
            && captured_log_paths == expected_log_paths
            && log_capture
                .files
                .windows(2)
                .all(|rows| rows[0].file_path < rows[1].file_path)
            && log_capture
                .files
                .iter()
                .map(|file| (&file.device, &file.inode))
                .collect::<BTreeSet<_>>()
                .len()
                == log_capture.files.len()
            && log_capture
                .files
                .iter()
                .all(
                    |file| Path::new(&file.file_path).starts_with(frozen_log_parent)
                        && file.size > 0
                        && file.size <= log_capture.per_file_limit_bytes
                        && is_nonzero_canonical_identity(&file.device)
                        && is_nonzero_canonical_identity(&file.inode)
                        && file.mode == "0600"
                        && file.link_count == 1
                        && file.owner_uid == log_owner_uid
                        && is_sha256(&file.sha256)
                )
            && log_capture.total_bytes
                == log_capture.files.iter().map(|file| file.size).sum::<u64>()
            && log_capture.total_bytes <= log_capture.aggregate_limit_bytes,
        "node log capture was not exact, continuously bounded, and overflow-free"
    );
    for node in &fixture.topology.nodes {
        let source = frozen_log_parent.join(format!("{}.log", node.generated_name));
        let copied = audit
            .artifacts
            .iter()
            .find(|artifact| artifact.logical_path == format!("log/{}.log", node.role))
            .ok_or_else(|| format!("audit lacks copied {} node log", node.role))?;
        require!(
            captured_logs
                .get(source.to_string_lossy().as_ref())
                .is_some_and(|captured| {
                    copied.read.bytes_read == captured.size && copied.read.sha256 == captured.sha256
                }),
            "copied {} log bytes/object differ from the bounded source",
            node.role
        );
    }
    for node in &evidence.topology.nodes {
        let copied = audit
            .artifacts
            .iter()
            .find(|artifact| artifact.logical_path == format!("config/{}.raw.json", node.role))
            .ok_or_else(|| format!("audit lacks copied {} raw chain spec", node.role))?;
        require!(
            copied.read.bytes_read == node.primary_chain_spec.raw_size
                && copied.read.sha256 == node.primary_chain_spec.raw_sha256,
            "copied {} raw chain spec differs from the E1 live spec identity",
            node.role
        );
    }
    let expected_addresses = expected_listener_addresses(fixture);
    require!(
        audit.socket_addresses.len() == expected_addresses.len()
            && audit
                .socket_addresses
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                == expected_addresses.iter().map(String::as_str).collect(),
        "audited socket inventory differs from the exact loopback topology"
    );

    let artifact_root = Path::new(&audit.artifact_root);
    require!(
        artifact_root.is_absolute()
            && artifact_root.starts_with(&journey.supported_root)
            && is_nonzero_canonical_identity(&audit.artifact_root_device)
            && is_nonzero_canonical_identity(&audit.artifact_root_inode)
            && audit.recursive_walk_complete
            && audit.walked_directories
                == [
                    "", "action", "config", "fixture", "journal", "log", "socket"
                ]
                .map(str::to_owned)
            && audit.symbolic_link_count == 0
            && audit.hard_link_alias_count == 0
            && audit.non_regular_count == 0,
        "audit root/walk did not prove one complete regular-file-only tree"
    );
    let observed_inventory = audit
        .artifacts
        .iter()
        .map(|artifact| (artifact.kind.clone(), artifact.logical_path.clone()))
        .collect::<Vec<_>>();
    assert_audit_artifact_inventory(&observed_inventory, &fixture.audit.required_artifacts)?;
    assert_simple_audit_artifact_bindings(journey)?;
    let pvf_inventory_bytes = canonical_json_bytes(&evidence.pvf_workers)?;
    let pvf_inventory_artifact = audit
        .artifacts
        .iter()
        .find(|artifact| artifact.logical_path == "config/pvf-worker-inventory.json")
        .ok_or_else(|| "audit lacks exact PVF worker inventory artifact".to_owned())?;
    require!(
        pvf_inventory_artifact.read.bytes_read == pvf_inventory_bytes.len() as u64
            && pvf_inventory_artifact.read.sha256 == sha256_bytes(&pvf_inventory_bytes)?,
        "retained PVF worker inventory artifact does not bind the consumed evidence"
    );
    let pvf_socket_value = serde_json::json!({
        "observed": &evidence.pvf_workers.runtime_monitor.observed_unix_sockets,
        "post_stop": &evidence.pvf_workers.runtime_monitor.post_stop_unix_sockets,
    });
    let pvf_socket_bytes = canonical_json_bytes(&pvf_socket_value)?;
    let pvf_socket_artifact = audit
        .artifacts
        .iter()
        .find(|artifact| artifact.logical_path == "socket/pvf-unix-sockets.json")
        .ok_or_else(|| "audit lacks exact PVF Unix-socket inventory artifact".to_owned())?;
    require!(
        pvf_socket_artifact.read.bytes_read == pvf_socket_bytes.len() as u64
            && pvf_socket_artifact.read.sha256 == sha256_bytes(&pvf_socket_bytes)?,
        "retained PVF Unix-socket artifact does not bind the consumed evidence"
    );
    let lanes = &audit.submission_lanes;
    let projection_directory = evidence
        .snapshots
        .first()
        .and_then(|snapshot| Path::new(&snapshot.projection_path).parent())
        .ok_or_else(|| "uninterrupted snapshot lacks a projection directory".to_owned())?;
    let projection_directory_text = projection_directory
        .to_str()
        .ok_or_else(|| "projection directory is not UTF-8".to_owned())?;
    let expected_uid = fs::metadata(&journey.supported_root)
        .map_err(|error| format!("cannot stat supported root owner: {error}"))?
        .uid();
    let mut expected_lanes = fixture
        .audit
        .dev_signer_accounts
        .iter()
        .map(|(signer, account)| {
            Ok((
                expected_submission_lane_name(
                    projection_directory_text,
                    &lanes.deployment_id,
                    account,
                )?,
                signer,
                account,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    expected_lanes.sort_by(|left, right| left.0.cmp(&right.0));
    require!(
        lanes.format == "cubikan-submission-lane-audit-v1"
            && lanes.directory == projection_directory_text
            && lanes.deployment_id == evidence.restart.identity_a.deployment_id
            && lanes.prohibited_residue_names.is_empty()
            && lanes.locks.len() == expected_lanes.len()
            && lanes
                .locks
                .iter()
                .zip(expected_lanes)
                .all(|(lock, expected)| {
                    lock.name == expected.0
                        && lock.signer == *expected.1
                        && lock.account == *expected.2
                        && Path::new(&lock.path) == projection_directory.join(&lock.name)
                        && lock.owner_uid == expected_uid
                        && lock.identity.mode == "0600"
                        && lock.identity.size == 0
                        && lock.identity.link_count == 1
                        && is_nonzero_canonical_identity(&lock.identity.device)
                        && is_nonzero_canonical_identity(&lock.identity.inode)
                }),
        "submission lanes were not exactly two canonical empty owner-only locks with no residue"
    );
    let mut omission = observed_inventory.clone();
    omission.pop();
    require!(
        assert_audit_artifact_inventory(&omission, &fixture.audit.required_artifacts).is_err(),
        "audit inventory validator accepted an omitted artifact"
    );
    let mut extra = observed_inventory.clone();
    extra.push(("log".to_owned(), "log/unexpected.log".to_owned()));
    require!(
        assert_audit_artifact_inventory(&extra, &fixture.audit.required_artifacts).is_err(),
        "audit inventory validator accepted an extra artifact"
    );
    let mut duplicate = observed_inventory.clone();
    duplicate.push(observed_inventory[0].clone());
    require!(
        assert_audit_artifact_inventory(&duplicate, &fixture.audit.required_artifacts).is_err(),
        "audit inventory validator accepted a duplicate artifact"
    );
    let mut scanned_bytes = 0_u64;
    let mut file_objects = BTreeSet::new();
    for artifact in &audit.artifacts {
        let maximum_bytes = match artifact.logical_path.as_str() {
            "config/collator-a.raw.json"
            | "config/collator-b.raw.json"
            | "config/relay-a.raw.json"
            | "config/relay-b.raw.json" => MAX_RAW_CHAIN_SPEC_BYTES,
            _ => MAX_AUDIT_ARTIFACT_BYTES,
        };
        require!(
            Path::new(&artifact.path) == artifact_root.join(&artifact.logical_path)
                && artifact.owner_only
                && artifact.read.bytes_read <= maximum_bytes
                && artifact.forbidden_hits.is_empty()
                && artifact.nonloopback_hits.is_empty(),
            "audited {} artifact {} has invalid identity, permissions, or a prohibited hit",
            artifact.kind,
            artifact.path
        );
        assert_opened_file_read(
            &artifact.read,
            true,
            &format!("audit artifact {}", artifact.logical_path),
        )?;
        require!(
            file_objects.insert((
                artifact.read.path_before.device.as_str(),
                artifact.read.path_before.inode.as_str(),
            )),
            "audit artifact {} aliases another file object",
            artifact.logical_path
        );
        scanned_bytes = scanned_bytes
            .checked_add(artifact.read.bytes_read)
            .ok_or_else(|| "audited byte count overflowed".to_owned())?;
    }
    require!(
        scanned_bytes == audit.scanned_byte_count
            && audit.scanned_byte_count > 0
            && audit.scanned_byte_count <= MAX_AUDIT_TOTAL_BYTES,
        "audited byte count does not equal retained artifact sizes"
    );
    require!(
        audit.zero_counts.len() == fixture.audit.zero_count_keys.len()
            && fixture
                .audit
                .zero_count_keys
                .iter()
                .all(|key| audit.zero_counts.get(key) == Some(&0)),
        "one or more forbidden action counters was missing or nonzero"
    );
    require!(
        audit.secret_environment_names_present.is_empty()
            && audit.public_urls.is_empty()
            && audit.external_actions.is_empty(),
        "audit retained a secret environment name, public URL, or external action"
    );
    let allowed_urls = fixture.endpoints.values().cloned().collect::<BTreeSet<_>>();

    for submission in &evidence.submissions {
        require!(
            forbidden_json_member(&submission.response).is_none(),
            "submission {} response contains a prohibited secret/source member at {}",
            submission.id,
            forbidden_json_member(&submission.response)
                .expect("the preceding predicate established a forbidden member")
        );
        require!(
            first_public_url(&submission.response, &allowed_urls).is_none(),
            "submission {} response contains public URL {}",
            submission.id,
            first_public_url(&submission.response, &allowed_urls)
                .expect("the preceding predicate established a public URL")
        );
    }
    for snapshot in &evidence.snapshots {
        let sections = serde_json::to_value(&snapshot.sections)
            .map_err(|error| format!("cannot inspect {} sections: {error}", snapshot.label))?;
        require!(
            forbidden_json_member(&sections).is_none(),
            "snapshot {} contains a prohibited secret/source member at {}",
            snapshot.label,
            forbidden_json_member(&sections)
                .expect("the preceding predicate established a forbidden member")
        );
        require!(
            first_public_url(&sections, &allowed_urls).is_none(),
            "snapshot {} contains public URL {}",
            snapshot.label,
            first_public_url(&sections, &allowed_urls)
                .expect("the preceding predicate established a public URL")
        );
    }
    Ok(())
}

fn assert_audit_artifact_inventory(
    observed: &[(String, String)],
    expected: &[AuditArtifactFixture],
) -> Result<(), String> {
    require!(
        observed.len() == expected.len()
            && observed
                .iter()
                .zip(expected)
                .all(|((kind, path), expected)| {
                    kind == &expected.kind && path == &expected.logical_path
                }),
        "audit artifact inventory omitted, added, reordered, or duplicated a required path"
    );
    require!(
        observed.iter().collect::<BTreeSet<_>>().len() == observed.len(),
        "audit artifact inventory contains a duplicate kind/path"
    );
    Ok(())
}

fn forbidden_json_member(value: &Value) -> Option<String> {
    fn visit(value: &Value, path: &str) -> Option<String> {
        match value {
            Value::Object(entries) => entries.iter().find_map(|(key, child)| {
                let child_path = format!("{path}/{key}");
                PROHIBITED_OUTPUT_KEYS
                    .contains(&key.as_str())
                    .then_some(child_path.clone())
                    .or_else(|| visit(child, &child_path))
            }),
            Value::Array(entries) => entries
                .iter()
                .enumerate()
                .find_map(|(index, child)| visit(child, &format!("{path}/{index}"))),
            _ => None,
        }
    }
    visit(value, "")
}

fn first_public_url(value: &Value, allowed_urls: &BTreeSet<String>) -> Option<String> {
    fn visit(value: &Value, allowed_urls: &BTreeSet<String>) -> Option<String> {
        match value {
            Value::String(text) => {
                let recognized_url = text.split_once(':').is_some_and(|(scheme, remainder)| {
                    matches!(
                        scheme.to_ascii_lowercase().as_str(),
                        "git+ssh" | "ssh" | "git" | "http" | "https" | "ws" | "wss"
                    ) && remainder.starts_with("//")
                });
                (recognized_url && !allowed_urls.contains(text)).then(|| text.clone())
            }
            Value::Array(entries) => entries.iter().find_map(|entry| visit(entry, allowed_urls)),
            Value::Object(entries) => entries
                .values()
                .find_map(|entry| visit(entry, allowed_urls)),
            _ => None,
        }
    }
    visit(value, allowed_urls)
}
