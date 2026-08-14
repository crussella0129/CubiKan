//! Fixed-runtime and deployment-artifact contract tests for T-1106.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use cubikan_runtime::{
    deployment_id, local_genesis_config, AccountId, Balances, Cubikan, Executive, Header,
    OriginCaller, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, Signature, System,
    TxExtension, UncheckedExtrinsic, DEPLOYMENT_ID_INPUT, EVENT_SCHEMA_VERSION, LOCAL_PARA_ID,
    PALLET_STORAGE_VERSION, VERSION,
};
use frame_support::{
    __private::metadata::{RuntimeMetadata, RuntimeMetadataPrefixed},
    dispatch::GetDispatchInfo,
    traits::{Contains, GetCallMetadata, GetCallName, OriginTrait},
};
use pallet_cubikan::types::{
    AssociationKey, AssociationSubject, ExternalReference, IntentUnitId, Namespace, ReferenceScope,
    ReferenceValue,
};
use parity_scale_codec::{Decode, Encode};
use serde_json::{json, Value};
use sp_keyring::Sr25519Keyring;
use sp_runtime::{generic::Era, traits::Dispatchable, BuildStorage, DispatchError};

const DEPLOYMENT_ID: [u8; 32] = [
    0x30, 0x46, 0xcb, 0x2c, 0xf3, 0xf5, 0xf9, 0xc5, 0x65, 0xa8, 0x54, 0x93, 0xcf, 0xff, 0x10, 0xfe,
    0xe9, 0x4d, 0x12, 0xd9, 0x50, 0xa0, 0xd6, 0xf5, 0x4d, 0x7c, 0x1f, 0xf3, 0x2a, 0x6a, 0xfc, 0x42,
];

fn chain_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("runtime crate is nested directly under chain/")
        .to_path_buf()
}

fn repository_root() -> PathBuf {
    chain_root()
        .parent()
        .expect("chain workspace is nested under the repository root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    fs::read_to_string(chain_root().join(relative))
        .unwrap_or_else(|error| panic!("read chain/{relative}: {error}"))
}

fn json_file(relative: &str) -> Value {
    serde_json::from_str(&read(relative))
        .unwrap_or_else(|error| panic!("decode chain/{relative}: {error}"))
}

fn pointer<'a>(value: &'a Value, path: &str) -> &'a Value {
    value
        .pointer(path)
        .unwrap_or_else(|| panic!("JSON path is absent: {path}"))
}

fn run_static_verifier(root: &Path) -> Output {
    Command::new("bash")
        .arg(root.join("chain/tools/verify-runtime-artifacts.sh"))
        .arg("--test-static")
        .arg(root)
        .output()
        .expect("run static runtime-artifact verifier")
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).expect("serialize mutated anchor");
    bytes.push(b'\n');
    bytes
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(*byte >> 4) as usize] as char);
        encoded.push(DIGITS[(*byte & 0x0f) as usize] as char);
    }
    encoded
}

fn normalize_tooling_apis_in_native_metadata(bytes: &[u8]) -> Vec<u8> {
    let mut input = bytes;
    let mut prefixed = RuntimeMetadataPrefixed::decode(&mut input)
        .expect("native runtime metadata must decode as FRAME metadata");
    assert!(input.is_empty(), "native metadata has trailing bytes");

    let RuntimeMetadata::V14(metadata) = &mut prefixed.1 else {
        panic!("the fixed runtime metadata must be V14");
    };
    let system = metadata
        .pallets
        .iter_mut()
        .find(|pallet| pallet.name == "System")
        .expect("native metadata contains the System pallet");
    let version_constant = system
        .constants
        .iter_mut()
        .find(|constant| constant.name == "Version")
        .expect("System metadata contains the runtime Version constant");

    let mut version_input = version_constant.value.as_slice();
    let mut runtime_version = sp_version::RuntimeVersion::decode(&mut version_input)
        .expect("System Version constant must decode as RuntimeVersion");
    assert!(
        version_input.is_empty(),
        "System Version constant has trailing bytes"
    );

    let mut fixed_apis = cubikan_runtime::fixed_runtime_api_versions_json();
    let mut expected_tooling_apis = Vec::new();
    #[cfg(feature = "runtime-benchmarks")]
    expected_tooling_apis.push((
        format!(
            "0x{}",
            lowercase_hex(
                &<dyn frame_benchmarking::Benchmark<cubikan_runtime::Block> as sp_api::RuntimeApiInfo>::ID,
            )
        ),
        <dyn frame_benchmarking::Benchmark<cubikan_runtime::Block> as sp_api::RuntimeApiInfo>::VERSION,
    ));
    #[cfg(feature = "try-runtime")]
    expected_tooling_apis.push((
        format!(
            "0x{}",
            lowercase_hex(
                &<dyn frame_try_runtime::TryRuntime<cubikan_runtime::Block> as sp_api::RuntimeApiInfo>::ID,
            )
        ),
        <dyn frame_try_runtime::TryRuntime<cubikan_runtime::Block> as sp_api::RuntimeApiInfo>::VERSION,
    ));
    let mut actual_tooling_apis: Vec<(String, u32)> = runtime_version
        .apis
        .iter()
        .map(|(api, version)| (format!("0x{}", lowercase_hex(api)), *version))
        .filter(|api| !fixed_apis.contains(api))
        .collect();
    expected_tooling_apis.sort_unstable();
    actual_tooling_apis.sort_unstable();
    assert_eq!(
        actual_tooling_apis, expected_tooling_apis,
        "only the enabled Benchmark/TryRuntime API IDs may differ in all-features metadata"
    );
    runtime_version.apis.to_mut().retain(|(api, version)| {
        let encoded_api = format!("0x{}", lowercase_hex(api));
        fixed_apis
            .iter()
            .any(|(fixed_api, fixed_version)| fixed_api == &encoded_api && fixed_version == version)
    });
    let mut normalized_apis: Vec<(String, u32)> = runtime_version
        .apis
        .iter()
        .map(|(api, version)| (format!("0x{}", lowercase_hex(api)), *version))
        .collect();
    fixed_apis.sort_unstable();
    normalized_apis.sort_unstable();
    assert_eq!(normalized_apis, fixed_apis);

    version_constant.value = runtime_version.encode();
    prefixed.encode()
}

fn decode_lowercase_hex(value: &str) -> Vec<u8> {
    let encoded = value
        .strip_prefix("0x")
        .expect("encoded artifact bytes use a 0x prefix");
    assert!(!encoded.is_empty() && encoded.len().is_multiple_of(2));
    assert!(encoded
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));
    encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => panic!("non-lowercase-hex artifact byte"),
            };
            (digit(pair[0]) << 4) | digit(pair[1])
        })
        .collect()
}

fn update_copied_anchor_pin(target: &Path, anchor_bytes: &[u8]) {
    let pins_path = target.join("chain/pins.toml");
    let mut pins = fs::read_to_string(&pins_path).expect("read copied pins");
    let marker = "anchor_sha256 = \"";
    let value_start = pins
        .find(marker)
        .map(|index| index + marker.len())
        .expect("copied pins contain the deployment-anchor digest");
    let value_end = pins[value_start..]
        .find('"')
        .map(|index| value_start + index)
        .expect("copied deployment-anchor digest is quoted");
    pins.replace_range(
        value_start..value_end,
        &lowercase_hex(&sp_io::hashing::sha2_256(anchor_bytes)),
    );
    fs::write(pins_path, pins).expect("update copied deployment-anchor pin");
}

fn copy_static_contract(target: &Path, anchor: &Value) {
    for relative in [
        "chain/tools/verify-runtime-artifacts.py",
        "chain/tools/verify-runtime-artifacts.sh",
        "chain/config/cubikan-local.json",
        "chain/pins.toml",
    ] {
        let source = repository_root().join(relative);
        let destination = target.join(relative);
        fs::create_dir_all(destination.parent().expect("copied path has a parent"))
            .expect("create static contract directory");
        fs::copy(source, destination).expect("copy static contract file");
    }
    for relative in [
        "chain/metadata/cubikan-runtime-v1.scale",
        "chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm",
    ] {
        let source = repository_root().join(relative);
        if source.is_file() {
            let destination = target.join(relative);
            fs::create_dir_all(destination.parent().expect("artifact path has a parent"))
                .expect("create copied artifact directory");
            fs::copy(source, destination).expect("copy resolved runtime artifact");
        }
    }
    let anchor_path = target.join("chain/artifacts/local-deployment-anchor-v1.json");
    fs::create_dir_all(anchor_path.parent().expect("anchor has a parent"))
        .expect("create anchor directory");
    let anchor_bytes = canonical_json_bytes(anchor);
    fs::write(anchor_path, &anchor_bytes).expect("write mutated anchor");
    update_copied_anchor_pin(target, &anchor_bytes);
}

fn transaction_extension(nonce: u32) -> TxExtension {
    (
        frame_system::CheckNonZeroSender::<Runtime>::new(),
        frame_system::CheckSpecVersion::<Runtime>::new(),
        frame_system::CheckTxVersion::<Runtime>::new(),
        frame_system::CheckGenesis::<Runtime>::new(),
        frame_system::CheckEra::<Runtime>::from(Era::immortal()),
        frame_system::CheckNonce::<Runtime>::from(nonce),
        frame_system::CheckWeight::<Runtime>::new(),
        pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
    )
        .into()
}

fn signed_extrinsic(sender: Sr25519Keyring, nonce: u32, call: RuntimeCall) -> UncheckedExtrinsic {
    let account: AccountId = sender.to_account_id();
    let extension = transaction_extension(nonce);
    let payload = sp_runtime::generic::SignedPayload::new(call.clone(), extension.clone())
        .expect("the bounded test call must form a signed payload");
    let signature = payload.using_encoded(|bytes| sender.sign(bytes));
    UncheckedExtrinsic::new_signed(
        call,
        account.into(),
        Signature::Sr25519(signature),
        extension,
    )
}

fn rejected_domain_call() -> RuntimeCall {
    let reference = ExternalReference::new(
        Namespace::try_from("git").expect("valid namespace"),
        ReferenceScope::try_from("commit").expect("valid scope"),
        ReferenceValue::try_from("0123456789abcdef").expect("valid reference"),
    );
    RuntimeCall::Cubikan(pallet_cubikan::Call::record_association {
        command_schema_version: 1,
        association: AssociationKey::new(
            IntentUnitId::from_bytes([0x11; 16]),
            AssociationSubject::WholeUnit,
            reference,
        ),
    })
}

fn domain_state() -> Vec<u8> {
    (
        pallet_cubikan::DeploymentAnchor::<Runtime>::get(),
        pallet_cubikan::PalletStorageVersion::<Runtime>::get(),
        pallet_cubikan::EventSchemaVersion::<Runtime>::get(),
        pallet_cubikan::AuthorizedSubmitters::<Runtime>::get(),
        pallet_cubikan::GlobalSequence::<Runtime>::get(),
        pallet_cubikan::IntentUnits::<Runtime>::iter().collect::<Vec<_>>(),
        pallet_cubikan::RelationshipDefinitions::<Runtime>::iter().collect::<Vec<_>>(),
        pallet_cubikan::RelationshipEdges::<Runtime>::iter().collect::<Vec<_>>(),
        pallet_cubikan::ActiveAssociations::<Runtime>::iter().collect::<Vec<_>>(),
    )
        .encode()
}

fn cubikan_event_count() -> usize {
    System::events()
        .iter()
        .filter(|record| matches!(&record.event, RuntimeEvent::Cubikan(_)))
        .count()
}

fn block_one_header() -> Header {
    Header {
        parent_hash: Default::default(),
        number: 1,
        state_root: Default::default(),
        extrinsics_root: Default::default(),
        digest: Default::default(),
    }
}

#[test]
fn test_runtime_genesis_and_authority_contract_are_exact() {
    assert_eq!(LOCAL_PARA_ID, 1000);
    assert_eq!(DEPLOYMENT_ID_INPUT, b"CubiKan local deployment v1\n");
    assert_eq!(sp_io::hashing::sha2_256(DEPLOYMENT_ID_INPUT), DEPLOYMENT_ID);
    assert_eq!(deployment_id(), DEPLOYMENT_ID);
    assert_eq!(PALLET_STORAGE_VERSION, 1);
    assert_eq!(EVENT_SCHEMA_VERSION, 1);
    assert_eq!(VERSION.spec_version, 1);
    assert_eq!(VERSION.transaction_version, 1);
    assert_eq!(VERSION.system_version, 1);
    let mut native_apis = cubikan_runtime::fixed_runtime_api_versions_json();
    native_apis.sort_unstable();
    let mut anchored_apis: Vec<(String, u32)> = serde_json::from_value(
        pointer(
            &json_file("artifacts/local-deployment-anchor-v1.json"),
            "/runtime/apis",
        )
        .clone(),
    )
    .expect("decode the anchored runtime API inventory");
    anchored_apis.sort_unstable();
    assert_eq!(
        native_apis, anchored_apis,
        "the deployment anchor does not bind the exact normal-runtime API inventory"
    );

    let genesis = local_genesis_config();
    let alice = Sr25519Keyring::Alice.to_account_id();
    let bob = Sr25519Keyring::Bob.to_account_id();
    let charlie = Sr25519Keyring::Charlie.to_account_id();
    let dave = Sr25519Keyring::Dave.to_account_id();
    assert_eq!(
        genesis.cubikan.authorized_submitters,
        vec![charlie.clone(), dave.clone()]
    );
    assert_eq!(genesis.cubikan.deployment_id, DEPLOYMENT_ID);
    assert_eq!(genesis.cubikan.pallet_storage_version, 1);
    assert_eq!(genesis.cubikan.event_schema_version, 1);
    assert_eq!(
        genesis.collator_selection.invulnerables,
        vec![alice.clone(), bob.clone()]
    );
    assert_eq!(
        genesis.balances.balances,
        vec![
            (alice, 1u128 << 60),
            (bob, 1u128 << 60),
            (charlie, 1u128 << 60),
            (dave, 1u128 << 60),
        ]
    );

    let spec = json_file("config/cubikan-local.json");
    assert_eq!(pointer(&spec, "/relay_chain"), "rococo-local");
    assert_eq!(pointer(&spec, "/para_id"), 1000);
    let spec_code = pointer(&spec, "/genesis/runtimeGenesis/code");
    match spec_code {
        Value::Null => {
            assert_eq!(pointer(&spec, "/status"), "awaiting-runtime-wasm");
            assert_eq!(
                pointer(&spec, "/format"),
                "cubikan-local-chain-spec-bootstrap-v1"
            );
        }
        Value::String(encoded) => {
            let code = decode_lowercase_hex(encoded);
            assert!(
                !code.is_empty(),
                "resolved chain spec System :code is empty"
            );
            assert_eq!(
                code,
                fs::read(chain_root().join("artifacts/cubikan-runtime-v1.compact.compressed.wasm"))
                    .expect("resolved chain spec has a committed runtime Wasm artifact")
            );
            assert!(spec.get("status").is_none());
            assert_ne!(
                spec.get("format").and_then(Value::as_str),
                Some("cubikan-local-chain-spec-bootstrap-v1")
            );
        }
        _ => panic!("chain spec System :code is neither null nor exact lowercase hex"),
    }
    assert_eq!(
        pointer(
            &spec,
            "/genesis/runtimeGenesis/patch/parachainInfo/parachainId"
        ),
        1000
    );
    let collators = pointer(
        &spec,
        "/genesis/runtimeGenesis/patch/collatorSelection/invulnerables",
    );
    let submitters = pointer(
        &spec,
        "/genesis/runtimeGenesis/patch/cubikan/authorizedSubmitters",
    );
    assert_eq!(
        pointer(&spec, "/genesis/runtimeGenesis/patch/cubikan/deploymentId"),
        &json!(DEPLOYMENT_ID),
        "chain-spec JSON uses the canonical fixed-byte array representation"
    );
    let expected_balances = json!({
        "balances": [
            ["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 1u64 << 60],
            ["5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty", 1u64 << 60],
            ["5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y", 1u64 << 60],
            ["5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy", 1u64 << 60]
        ]
    });
    assert_eq!(
        pointer(&spec, "/genesis/runtimeGenesis/patch/balances"),
        &expected_balances
    );
    assert_eq!(
        collators,
        &json!([
            "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
            "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"
        ])
    );
    assert_eq!(
        submitters,
        &json!([
            "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
            "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy"
        ])
    );
    assert!(collators
        .as_array()
        .expect("collator list")
        .iter()
        .all(|account| !submitters
            .as_array()
            .expect("submitter list")
            .contains(account)));
    assert_eq!(
        pointer(&spec, "/genesis/runtimeGenesis/patch/session"),
        &json!({
            "keys": [
                [
                    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
                    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
                    {"aura": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"}
                ],
                [
                    "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
                    "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
                    {"aura": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"}
                ]
            ]
        })
    );
    assert!(spec.get("relay_genesis_hash").is_none());
    assert!(spec.get("parachain_genesis_hash").is_none());
}

#[test]
fn test_runtime_artifacts_are_semantically_consistent() {
    let output = run_static_verifier(&repository_root());
    assert!(
        output.status.success(),
        "static artifact verification failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let manifest = json_file("artifacts/local-deployment-anchor-v1.json");
    assert_eq!(
        pointer(&manifest, "/artifacts/runtime_wasm/path"),
        "chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm"
    );
    assert_eq!(
        pointer(&manifest, "/artifacts/metadata/path"),
        "chain/metadata/cubikan-runtime-v1.scale"
    );
    assert_eq!(pointer(&manifest, "/status"), "resolved");
    assert_eq!(
        pointer(&manifest, "/artifacts/chain_spec/sha256"),
        "dc7945fbeed5b18d21c1839f8f4f5ab13a1660ca956a3513b8a9946bab6334c7"
    );
    assert_eq!(pointer(&manifest, "/artifacts/chain_spec/size"), 1_278_643);
    assert_eq!(
        pointer(&manifest, "/artifacts/runtime_wasm/sha256"),
        "640cc616674fe7393fc93928904f0fd92d77571209c8200f08b8da6290c6a275"
    );
    assert_eq!(pointer(&manifest, "/artifacts/runtime_wasm/size"), 637_930);
    assert_eq!(
        pointer(&manifest, "/artifacts/metadata/sha256"),
        "171a323b1e6bf0122e549eecd5f5932e672a3e0835f32edf0b8808cfefd97302"
    );
    assert_eq!(pointer(&manifest, "/artifacts/metadata/size"), 63_327);
    let committed = fs::read(chain_root().join("metadata/cubikan-runtime-v1.scale"))
        .expect("read committed runtime metadata");
    let storage = local_genesis_config()
        .build_storage()
        .expect("the exact local runtime genesis must build");
    let mut externalities = sp_io::TestExternalities::new(storage);
    let native_metadata = externalities.execute_with(|| Runtime::metadata().encode());
    let normalized_native_metadata = normalize_tooling_apis_in_native_metadata(&native_metadata);
    assert_eq!(
        committed, normalized_native_metadata,
        "committed V14 metadata differs from the native runtime after excluding the two all-features tooling APIs"
    );

    // The canonical post-release gate invokes `--locked` after producing the
    // release wbuild artifact. This test stays clean-build-safe and validates
    // the committed/static/native contract without assuming an untracked
    // release artifact exists before the workspace test step.
}

#[test]
fn test_runtime_data_fee_and_fixed_code_policy() {
    let source = read("runtime/src/lib.rs");
    assert!(source.contains("ChargeTransactionPayment"));
    assert!(source.contains("TransactionByteFee"));
    assert!(!source.contains("ChargeAssetTxPayment"));
    assert!(!source.contains("pallet_sudo"));
    assert!(cubikan_runtime::TransactionByteFee::get() > 0);

    let storage = local_genesis_config()
        .build_storage()
        .expect("the exact local runtime genesis must build");
    let mut externalities = sp_io::TestExternalities::new(storage);
    externalities.execute_with(|| {
        Executive::initialize_block(&block_one_header());
        let charlie = Sr25519Keyring::Charlie.to_account_id();
        let call = rejected_domain_call();
        let extrinsic = signed_extrinsic(Sr25519Keyring::Charlie, 0, call.clone());
        let fee = cubikan_runtime::TransactionPayment::compute_fee(
            extrinsic.encoded_size() as u32,
            &call.get_dispatch_info(),
            0,
        );
        let details = cubikan_runtime::TransactionPayment::compute_fee_details(
            extrinsic.encoded_size() as u32,
            &call.get_dispatch_info(),
            0,
        );
        let inclusion = details
            .inclusion_fee
            .expect("a fee-paying signed domain call has an inclusion fee");
        assert!(fee > 0, "ordinary weight/length fee must be nonzero");
        assert_eq!(
            details.tip, 0,
            "the canonical domain transaction tip is zero"
        );
        assert!(inclusion.base_fee > 0);
        assert!(inclusion.len_fee > 0);
        assert!(inclusion.adjusted_weight_fee > 0);
        assert_eq!(fee, inclusion.inclusion_fee());
        assert!(Balances::free_balance(&charlie) > fee);
    });

    let manifest = json_file("artifacts/local-deployment-anchor-v1.json");
    assert_eq!(
        pointer(&manifest, "/runtime/code/provenance/method"),
        "state_getStorage"
    );
    for forbidden in [
        "production_key",
        "private_key",
        "secret_key",
        "mnemonic",
        "seed_phrase",
    ] {
        assert!(!manifest
            .to_string()
            .to_ascii_lowercase()
            .contains(forbidden));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeCallRoute {
    AnyOrigin,
    Inherent,
    Signed,
    AuthorizedSubmitter,
    Root,
    AuthorizedUpgrade,
    FilteredRoot,
}

/// Exhaustive over every dispatchable exposed by the outer runtime call and
/// every relevant inner call enum. A newly exposed pallet or dispatchable must
/// therefore make this test stop compiling until its authority route is
/// explicitly reviewed.
fn runtime_call_route(call: &RuntimeCall) -> RuntimeCallRoute {
    match call {
        RuntimeCall::System(call) => match call {
            frame_system::Call::remark { .. } => RuntimeCallRoute::AnyOrigin,
            frame_system::Call::remark_with_event { .. } => RuntimeCallRoute::Signed,
            frame_system::Call::set_heap_pages { .. }
            | frame_system::Call::set_code { .. }
            | frame_system::Call::set_code_without_checks { .. }
            | frame_system::Call::set_storage { .. }
            | frame_system::Call::kill_storage { .. }
            | frame_system::Call::kill_prefix { .. }
            | frame_system::Call::authorize_upgrade { .. }
            | frame_system::Call::authorize_upgrade_without_checks { .. } => RuntimeCallRoute::Root,
            frame_system::Call::apply_authorized_upgrade { .. } => {
                RuntimeCallRoute::AuthorizedUpgrade
            }
            frame_system::Call::__Ignore(_, _) => unreachable!("uninhabited system call variant"),
        },
        RuntimeCall::ParachainSystem(call) => match call {
            cumulus_pallet_parachain_system::Call::set_validation_data { .. } => {
                RuntimeCallRoute::Inherent
            }
            cumulus_pallet_parachain_system::Call::sudo_send_upward_message { .. } => {
                RuntimeCallRoute::FilteredRoot
            }
            cumulus_pallet_parachain_system::Call::__Ignore(_, _) => {
                unreachable!("uninhabited parachain-system call variant")
            }
        },
        RuntimeCall::Timestamp(call) => match call {
            pallet_timestamp::Call::set { .. } => RuntimeCallRoute::Inherent,
            pallet_timestamp::Call::__Ignore(_, _) => {
                unreachable!("uninhabited timestamp call variant")
            }
        },
        RuntimeCall::ParachainInfo(call) => match call {
            staging_parachain_info::Call::__Ignore(_, _) => {
                unreachable!("uninhabited parachain-info call variant")
            }
        },
        RuntimeCall::Balances(call) => match call {
            pallet_balances::Call::transfer_allow_death { .. }
            | pallet_balances::Call::transfer_keep_alive { .. }
            | pallet_balances::Call::transfer_all { .. }
            | pallet_balances::Call::upgrade_accounts { .. }
            | pallet_balances::Call::burn { .. } => RuntimeCallRoute::Signed,
            pallet_balances::Call::force_transfer { .. }
            | pallet_balances::Call::force_unreserve { .. }
            | pallet_balances::Call::force_set_balance { .. }
            | pallet_balances::Call::force_adjust_total_issuance { .. } => RuntimeCallRoute::Root,
            pallet_balances::Call::__Ignore(_, _) => {
                unreachable!("uninhabited balances call variant")
            }
        },
        RuntimeCall::Session(call) => match call {
            pallet_session::Call::set_keys { .. } | pallet_session::Call::purge_keys { .. } => {
                RuntimeCallRoute::Signed
            }
            pallet_session::Call::__Ignore(_, _) => {
                unreachable!("uninhabited session call variant")
            }
        },
        RuntimeCall::Cubikan(call) => match call {
            pallet_cubikan::Call::create_unit { .. }
            | pallet_cubikan::Call::transition_unit { .. }
            | pallet_cubikan::Call::complete_unit { .. }
            | pallet_cubikan::Call::create_relationship_definition { .. }
            | pallet_cubikan::Call::create_relationship { .. }
            | pallet_cubikan::Call::delete_relationship { .. }
            | pallet_cubikan::Call::record_association { .. }
            | pallet_cubikan::Call::revoke_association { .. } => {
                RuntimeCallRoute::AuthorizedSubmitter
            }
            pallet_cubikan::Call::replace_authorized_submitters { .. } => RuntimeCallRoute::Root,
            pallet_cubikan::Call::__Ignore(_, _) => {
                unreachable!("uninhabited Cubikan call variant")
            }
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeOriginRoute {
    None,
    Signed,
    Root,
    AuthorizedUnreachable,
}

/// Exhaustive over the generated runtime-origin caller. CubiKan deliberately
/// has no pallet-defined origin that could be transformed into Root.
fn runtime_origin_route(origin: RuntimeOrigin) -> RuntimeOriginRoute {
    match <RuntimeOrigin as OriginTrait>::into_caller(origin) {
        OriginCaller::system(frame_system::RawOrigin::None) => RuntimeOriginRoute::None,
        OriginCaller::system(frame_system::RawOrigin::Signed(_)) => RuntimeOriginRoute::Signed,
        OriginCaller::system(frame_system::RawOrigin::Root) => RuntimeOriginRoute::Root,
        OriginCaller::system(frame_system::RawOrigin::Authorized) => {
            RuntimeOriginRoute::AuthorizedUnreachable
        }
        OriginCaller::Void(never) => match never {},
    }
}

#[test]
fn test_runtime_has_no_origin_transform_or_root_route() {
    let source = read("runtime/src/lib.rs").to_ascii_lowercase();
    for forbidden in [
        "pallet_sudo",
        "pallet_proxy",
        "pallet_utility",
        "pallet_multisig",
        "pallet_collective",
        "pallet_democracy",
        "pallet_conviction_voting",
        "dispatch_as",
        "as_derivative",
        "authorizecall",
    ] {
        assert!(!source.contains(forbidden), "forbidden route: {forbidden}");
    }
    for required in ["type system =", "type balances =", "type cubikan ="] {
        assert!(source.contains(required), "runtime omits {required}");
    }

    assert_eq!(
        <RuntimeCall as GetCallMetadata>::get_module_names(),
        &[
            "System",
            "ParachainSystem",
            "Timestamp",
            "ParachainInfo",
            "Balances",
            "Session",
            "Cubikan",
        ],
        "the outer RuntimeCall inventory changed without an authority review"
    );
    assert_eq!(
        <frame_system::Call<Runtime> as GetCallName>::get_call_names(),
        &[
            "remark",
            "set_heap_pages",
            "set_code",
            "set_code_without_checks",
            "set_storage",
            "kill_storage",
            "kill_prefix",
            "remark_with_event",
            "authorize_upgrade",
            "authorize_upgrade_without_checks",
            "apply_authorized_upgrade",
        ]
    );
    assert_eq!(
        <cumulus_pallet_parachain_system::Call<Runtime> as GetCallName>::get_call_names(),
        &["set_validation_data", "sudo_send_upward_message"]
    );
    assert_eq!(
        <pallet_timestamp::Call<Runtime> as GetCallName>::get_call_names(),
        &["set"]
    );
    assert!(
        <staging_parachain_info::Call<Runtime> as GetCallName>::get_call_names().is_empty(),
        "the uninhabited ParachainInfo call enum unexpectedly gained a dispatchable"
    );
    assert_eq!(
        <pallet_balances::Call<Runtime> as GetCallName>::get_call_names(),
        &[
            "transfer_allow_death",
            "force_transfer",
            "transfer_keep_alive",
            "transfer_all",
            "force_unreserve",
            "upgrade_accounts",
            "force_set_balance",
            "force_adjust_total_issuance",
            "burn",
        ]
    );
    assert_eq!(
        <pallet_session::Call<Runtime> as GetCallName>::get_call_names(),
        &["set_keys", "purge_keys"]
    );
    assert_eq!(
        <pallet_cubikan::Call<Runtime> as GetCallName>::get_call_names(),
        &[
            "create_unit",
            "transition_unit",
            "complete_unit",
            "replace_authorized_submitters",
            "create_relationship_definition",
            "create_relationship",
            "delete_relationship",
            "record_association",
            "revoke_association",
        ],
        "the Cubikan RuntimeCall inventory must remain exactly nine calls"
    );

    let charlie = Sr25519Keyring::Charlie.to_account_id();
    assert_eq!(
        runtime_origin_route(RuntimeOrigin::none()),
        RuntimeOriginRoute::None
    );
    assert_eq!(
        runtime_origin_route(RuntimeOrigin::signed(charlie.clone())),
        RuntimeOriginRoute::Signed
    );
    assert_eq!(
        runtime_origin_route(RuntimeOrigin::root()),
        RuntimeOriginRoute::Root
    );
    assert_eq!(
        runtime_origin_route(RuntimeOrigin::from(frame_system::RawOrigin::Authorized)),
        RuntimeOriginRoute::AuthorizedUnreachable,
        "FRAME's internal Authorized origin exists in the enum but must have no runtime route"
    );
    let representative_routes = [
        (
            RuntimeCall::System(frame_system::Call::remark { remark: Vec::new() }),
            RuntimeCallRoute::AnyOrigin,
        ),
        (
            RuntimeCall::Timestamp(pallet_timestamp::Call::set { now: 0 }),
            RuntimeCallRoute::Inherent,
        ),
        (
            RuntimeCall::Balances(pallet_balances::Call::burn {
                value: 1,
                keep_alive: true,
            }),
            RuntimeCallRoute::Signed,
        ),
        (
            RuntimeCall::System(frame_system::Call::remark_with_event { remark: Vec::new() }),
            RuntimeCallRoute::Signed,
        ),
        (
            rejected_domain_call(),
            RuntimeCallRoute::AuthorizedSubmitter,
        ),
    ];
    for (call, expected) in representative_routes {
        assert_eq!(runtime_call_route(&call), expected);
    }

    let storage = local_genesis_config()
        .build_storage()
        .expect("the exact local runtime genesis must build");
    let mut externalities = sp_io::TestExternalities::new(storage);
    externalities.execute_with(|| {
        let signed = RuntimeOrigin::signed(charlie);
        let filtered = RuntimeCall::ParachainSystem(
            cumulus_pallet_parachain_system::Call::sudo_send_upward_message {
                message: Default::default(),
            },
        );
        assert_eq!(
            runtime_call_route(&filtered),
            RuntimeCallRoute::FilteredRoot
        );
        assert!(
            !<<Runtime as frame_system::Config>::BaseCallFilter as Contains<RuntimeCall>>::contains(
                &filtered
            )
        );

        let root_only = [
            RuntimeCall::System(frame_system::Call::set_heap_pages { pages: 1 }),
            RuntimeCall::System(frame_system::Call::set_code { code: Vec::new() }),
            RuntimeCall::System(frame_system::Call::set_code_without_checks { code: Vec::new() }),
            RuntimeCall::System(frame_system::Call::set_storage { items: Vec::new() }),
            RuntimeCall::System(frame_system::Call::kill_storage { keys: Vec::new() }),
            RuntimeCall::System(frame_system::Call::kill_prefix {
                prefix: Vec::new(),
                subkeys: 0,
            }),
            RuntimeCall::System(frame_system::Call::authorize_upgrade {
                code_hash: Default::default(),
            }),
            RuntimeCall::System(frame_system::Call::authorize_upgrade_without_checks {
                code_hash: Default::default(),
            }),
            RuntimeCall::Balances(pallet_balances::Call::force_transfer {
                source: Sr25519Keyring::Alice.to_account_id().into(),
                dest: Sr25519Keyring::Bob.to_account_id().into(),
                value: 1,
            }),
            RuntimeCall::Balances(pallet_balances::Call::force_unreserve {
                who: Sr25519Keyring::Alice.to_account_id().into(),
                amount: 1,
            }),
            RuntimeCall::Balances(pallet_balances::Call::force_set_balance {
                who: Sr25519Keyring::Alice.to_account_id().into(),
                new_free: 1,
            }),
            RuntimeCall::Balances(pallet_balances::Call::force_adjust_total_issuance {
                direction: pallet_balances::AdjustmentDirection::Increase,
                delta: 1,
            }),
            RuntimeCall::Cubikan(pallet_cubikan::Call::replace_authorized_submitters {
                accounts: Default::default(),
            }),
        ];
        for call in root_only {
            assert_eq!(runtime_call_route(&call), RuntimeCallRoute::Root);
            assert_eq!(
                call.dispatch(signed.clone())
                    .map(|_| ())
                    .map_err(|error| error.error),
                Err(DispatchError::BadOrigin),
                "a signed origin reached a Root-only call"
            );
        }
        let authorized_upgrade =
            RuntimeCall::System(frame_system::Call::apply_authorized_upgrade { code: Vec::new() });
        assert_eq!(
            runtime_call_route(&authorized_upgrade),
            RuntimeCallRoute::AuthorizedUpgrade
        );
        assert_eq!(
            authorized_upgrade
                .dispatch(signed.clone())
                .map(|_| ())
                .map_err(|error| error.error),
            Err(frame_system::Error::<Runtime>::NothingAuthorized.into()),
            "the signed upgrade-preimage route was not gated by unreachable authorization"
        );
        assert_eq!(
            filtered
                .dispatch(signed)
                .map(|_| ())
                .map_err(|error| error.error),
            Err(frame_system::Error::<Runtime>::CallFiltered.into()),
            "the parachain sudo route was not filtered"
        );
    });
}

#[test]
fn test_post_genesis_manifest_traces_every_field_source() {
    let manifest = json_file("artifacts/local-deployment-anchor-v1.json");
    let expected = [
        ("/status", json!("resolved")),
        ("/relay_genesis/block_number", json!(0)),
        (
            "/relay_genesis/hash",
            json!("0xeb2ada687ce553d3b9d695afd5d9d0a9c44a0b82e1f6eb823ac87e81638200f0"),
        ),
        (
            "/relay_genesis/provenance/rpc_url",
            json!("ws://127.0.0.1:9944/"),
        ),
        (
            "/relay_genesis/provenance/endpoint_role",
            json!("relay-validator-alice-rpc"),
        ),
        (
            "/relay_genesis/provenance/method",
            json!("chain_getBlockHash"),
        ),
        ("/relay_genesis/provenance/params", json!([0])),
        ("/parachain_genesis/block_number", json!(0)),
        (
            "/parachain_genesis/hash",
            json!("0x627f53b3abc01130ec273ef85759f90779e8497614a428a66d862a624ee01a17"),
        ),
        (
            "/parachain_genesis/provenance/rpc_url",
            json!("ws://127.0.0.1:9988/"),
        ),
        (
            "/parachain_genesis/provenance/endpoint_role",
            json!("parachain-collator-alice-rpc"),
        ),
        (
            "/parachain_genesis/provenance/method",
            json!("chain_getBlockHash"),
        ),
        ("/parachain_genesis/provenance/params", json!([0])),
        ("/deployment/para_id", json!(1000)),
        (
            "/deployment/deployment_id",
            json!("0x3046cb2cf3f5f9c565a85493cfff10fee94d12d950a0d6f54d7c1ff32a6afc42"),
        ),
        ("/deployment/pallet_storage_version", json!(1)),
        ("/deployment/event_schema_version", json!(1)),
        (
            "/deployment/source",
            json!("decoded-pallet-cubikan-genesis-state"),
        ),
        ("/runtime/authoring_version", json!(1)),
        ("/runtime/impl_name", json!("cubikan-runtime")),
        ("/runtime/impl_version", json!(0)),
        ("/runtime/spec_name", json!("cubikan-runtime")),
        ("/runtime/spec_version", json!(1)),
        ("/runtime/transaction_version", json!(1)),
        ("/runtime/state_version", json!(1)),
        ("/runtime/system_version", json!(1)),
        (
            "/runtime/provenance/endpoint_role",
            json!("parachain-collator-alice-rpc"),
        ),
        (
            "/runtime/provenance/method",
            json!("state_getRuntimeVersion"),
        ),
        (
            "/runtime/provenance/params",
            json!(["$parachain_block_0_hash"]),
        ),
        ("/runtime/provenance/rpc_url", json!("ws://127.0.0.1:9988/")),
        ("/runtime/code/provenance/method", json!("state_getStorage")),
        (
            "/runtime/code/blake2_256",
            json!("0xe95e40bb618591b98b315b7901f3586ee5899f8bf26bda01401601c4f86b8a00"),
        ),
        (
            "/runtime/code/provenance/params",
            json!(["0x3a636f6465", "$parachain_block_0_hash"]),
        ),
    ];
    for (path, value) in expected {
        assert_eq!(
            pointer(&manifest, path),
            &value,
            "wrong anchor field {path}"
        );
    }
    assert_eq!(
        pointer(&manifest, "/runtime/apis"),
        &json!([
            ["0xab3c0572291feb8b", 2],
            ["0xea93e3f16f3d6962", 3],
            ["0x37e397fc7c91f5e4", 2],
            ["0x37c8bb1350a9a2a8", 4],
            ["0xbc9d89904f5b923f", 1],
            ["0xdf6acb689907609b", 5],
            ["0xd7bdd8a272ca0d65", 2],
            ["0x075f8cd374350e84", 1],
            ["0xd2bc9897eed08f15", 3],
            ["0xfbc577b9d747efd6", 1],
            ["0xf78b278be53f454c", 2],
            ["0xa2ddb6a58477bf63", 1],
            ["0xdd718d5cc53262d4", 1],
            ["0x04e70521a0d3d2f8", 2],
            ["0x40fe3ad401f8959a", 6],
            ["0xccd9de6396c899ca", 1],
            ["0xf3ff14d5ab527059", 3]
        ])
    );
    let expected_state_records = [
        (
            "deployment_id",
            json!("0x3046cb2cf3f5f9c565a85493cfff10fee94d12d950a0d6f54d7c1ff32a6afc42"),
            "0x3046cb2cf3f5f9c565a85493cfff10fee94d12d950a0d6f54d7c1ff32a6afc42",
            "0x2609aef1a1450f1b658394d12c417d249cc1dff5d6cf049a569219f2724d2e09",
        ),
        (
            "event_schema_version",
            json!(1),
            "0x0100",
            "0x2609aef1a1450f1b658394d12c417d24be4eb7443b9de1b5f041137c841f02ac",
        ),
        (
            "pallet_storage_version",
            json!(1),
            "0x0100",
            "0x2609aef1a1450f1b658394d12c417d241601562ebcdff856cb2f34e65f3b2659",
        ),
        (
            "para_id",
            json!(1000),
            "0xe8030000",
            "0x0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f",
        ),
    ];
    for (record, value, scale_hex, storage_key) in expected_state_records {
        let prefix = format!("/deployment/state_records/{record}");
        assert_eq!(pointer(&manifest, &format!("{prefix}/value")), &value);
        assert_eq!(
            pointer(&manifest, &format!("{prefix}/scale_hex")),
            scale_hex
        );
        assert_eq!(
            pointer(&manifest, &format!("{prefix}/provenance/endpoint_role")),
            "parachain-collator-alice-rpc"
        );
        assert_eq!(
            pointer(&manifest, &format!("{prefix}/provenance/method")),
            "state_getStorage"
        );
        assert_eq!(
            pointer(&manifest, &format!("{prefix}/provenance/params")),
            &json!([storage_key, "$parachain_block_0_hash"])
        );
        assert_eq!(
            pointer(&manifest, &format!("{prefix}/provenance/rpc_url")),
            "ws://127.0.0.1:9988/"
        );
    }
    assert!(!manifest.to_string().contains("self_hash"));

    let pins = read("pins.toml");
    assert!(pins.contains("local-deployment-anchor-v1.json"));
    assert!(pins.contains("[runtime_artifacts]"));
    assert!(pins.contains("anchor_sha256"));

    let mutation_paths = [
        "/format",
        "/namespace",
        "/deployment/deployment_id",
        "/deployment/deployment_id_derivation/algorithm",
        "/deployment/deployment_id_derivation/input_utf8",
        "/deployment/event_schema_version",
        "/deployment/pallet_storage_version",
        "/deployment/para_id",
        "/deployment/source",
        "/deployment/state_records/deployment_id/value",
        "/deployment/state_records/deployment_id/scale_hex",
        "/deployment/state_records/deployment_id/provenance/endpoint_role",
        "/deployment/state_records/deployment_id/provenance/method",
        "/deployment/state_records/deployment_id/provenance/params/0",
        "/deployment/state_records/deployment_id/provenance/params/1",
        "/deployment/state_records/deployment_id/provenance/rpc_url",
        "/deployment/state_records/event_schema_version/value",
        "/deployment/state_records/event_schema_version/scale_hex",
        "/deployment/state_records/event_schema_version/provenance/endpoint_role",
        "/deployment/state_records/event_schema_version/provenance/method",
        "/deployment/state_records/event_schema_version/provenance/params/0",
        "/deployment/state_records/event_schema_version/provenance/params/1",
        "/deployment/state_records/event_schema_version/provenance/rpc_url",
        "/deployment/state_records/pallet_storage_version/value",
        "/deployment/state_records/pallet_storage_version/scale_hex",
        "/deployment/state_records/pallet_storage_version/provenance/endpoint_role",
        "/deployment/state_records/pallet_storage_version/provenance/method",
        "/deployment/state_records/pallet_storage_version/provenance/params/0",
        "/deployment/state_records/pallet_storage_version/provenance/params/1",
        "/deployment/state_records/pallet_storage_version/provenance/rpc_url",
        "/deployment/state_records/para_id/value",
        "/deployment/state_records/para_id/scale_hex",
        "/deployment/state_records/para_id/provenance/endpoint_role",
        "/deployment/state_records/para_id/provenance/method",
        "/deployment/state_records/para_id/provenance/params/0",
        "/deployment/state_records/para_id/provenance/params/1",
        "/deployment/state_records/para_id/provenance/rpc_url",
        "/relay_genesis/hash",
        "/relay_genesis/block_number",
        "/relay_genesis/provenance/endpoint_role",
        "/relay_genesis/provenance/method",
        "/relay_genesis/provenance/params/0",
        "/relay_genesis/provenance/rpc_url",
        "/parachain_genesis/block_number",
        "/parachain_genesis/hash",
        "/parachain_genesis/provenance/endpoint_role",
        "/parachain_genesis/provenance/method",
        "/parachain_genesis/provenance/params/0",
        "/parachain_genesis/provenance/rpc_url",
        "/runtime/authoring_version",
        "/runtime/apis/0/0",
        "/runtime/apis/0/1",
        "/runtime/impl_name",
        "/runtime/impl_version",
        "/runtime/spec_name",
        "/runtime/spec_version",
        "/runtime/state_version",
        "/runtime/system_version",
        "/runtime/transaction_version",
        "/runtime/provenance/endpoint_role",
        "/runtime/provenance/method",
        "/runtime/provenance/params/0",
        "/runtime/provenance/rpc_url",
        "/runtime/code/blake2_256",
        "/runtime/code/provenance/endpoint_role",
        "/runtime/code/provenance/method",
        "/runtime/code/provenance/params/0",
        "/runtime/code/provenance/params/1",
        "/runtime/code/provenance/rpc_url",
        "/artifacts/chain_spec/path",
        "/artifacts/chain_spec/sha256",
        "/artifacts/chain_spec/size",
        "/artifacts/metadata/path",
        "/artifacts/metadata/sha256",
        "/artifacts/metadata/size",
        "/artifacts/metadata/provenance/endpoint_role",
        "/artifacts/metadata/provenance/method",
        "/artifacts/metadata/provenance/params/0",
        "/artifacts/metadata/provenance/rpc_url",
        "/artifacts/runtime_wasm/path",
        "/artifacts/runtime_wasm/sha256",
        "/artifacts/runtime_wasm/size",
        "/artifacts/runtime_wasm/provenance/builder",
        "/artifacts/runtime_wasm/provenance/profile",
        "/artifacts/runtime_wasm/provenance/source_path",
        "/status",
    ];
    for (index, path) in mutation_paths.iter().enumerate() {
        let mut mutated = manifest.clone();
        *mutated
            .pointer_mut(path)
            .unwrap_or_else(|| panic!("mutation path is absent: {path}")) = json!("mutated");
        let directory = std::env::temp_dir().join(format!(
            "cubikan-t1106-anchor-mutation-{}-{index}",
            std::process::id()
        ));
        if directory.exists() {
            fs::remove_dir_all(&directory).expect("remove stale mutation fixture");
        }
        copy_static_contract(&directory, &mutated);
        let output = run_static_verifier(&directory);
        assert!(
            !output.status.success(),
            "static verifier accepted mutation at {path}"
        );
        fs::remove_dir_all(directory).expect("remove mutation fixture");
    }

    let valid_semantic_mutations = [
        (
            "/relay_genesis/hash",
            json!("0x1111111111111111111111111111111111111111111111111111111111111111"),
        ),
        (
            "/parachain_genesis/hash",
            json!("0x2222222222222222222222222222222222222222222222222222222222222222"),
        ),
        ("/runtime/apis/0/0", json!("0x0000000000000001")),
        ("/runtime/apis/0/1", json!(1)),
        ("/runtime/state_version", json!(2)),
        (
            "/runtime/code/blake2_256",
            json!("0x3333333333333333333333333333333333333333333333333333333333333333"),
        ),
        (
            "/artifacts/chain_spec/sha256",
            json!("4444444444444444444444444444444444444444444444444444444444444444"),
        ),
        (
            "/artifacts/metadata/sha256",
            json!("5555555555555555555555555555555555555555555555555555555555555555"),
        ),
        (
            "/artifacts/runtime_wasm/sha256",
            json!("6666666666666666666666666666666666666666666666666666666666666666"),
        ),
    ];
    for (index, (path, replacement)) in valid_semantic_mutations.iter().enumerate() {
        let mut mutated = manifest.clone();
        *mutated
            .pointer_mut(path)
            .unwrap_or_else(|| panic!("valid semantic mutation path is absent: {path}")) =
            replacement.clone();
        let directory = std::env::temp_dir().join(format!(
            "cubikan-t1106-anchor-valid-semantic-mutation-{}-{index}",
            std::process::id()
        ));
        if directory.exists() {
            fs::remove_dir_all(&directory).expect("remove stale semantic mutation fixture");
        }
        copy_static_contract(&directory, &mutated);
        let output = run_static_verifier(&directory);
        assert!(
            !output.status.success(),
            "static verifier accepted valid-looking semantic mutation at {path}"
        );
        fs::remove_dir_all(directory).expect("remove semantic mutation fixture");
    }

    let numeric_type_mutations = [
        ("/deployment/event_schema_version", json!(true)),
        ("/deployment/pallet_storage_version", json!(1.0)),
        (
            "/deployment/state_records/event_schema_version/value",
            json!(true),
        ),
        ("/deployment/state_records/para_id/value", json!(1000.0)),
        ("/relay_genesis/block_number", json!(false)),
        ("/parachain_genesis/block_number", json!(0.0)),
        ("/runtime/spec_version", json!(true)),
        ("/runtime/state_version", json!(1.0)),
        ("/runtime/system_version", json!(1.0)),
    ];
    for (index, (path, replacement)) in numeric_type_mutations.iter().enumerate() {
        let mut mutated = manifest.clone();
        *mutated
            .pointer_mut(path)
            .unwrap_or_else(|| panic!("numeric mutation path is absent: {path}")) =
            replacement.clone();
        let directory = std::env::temp_dir().join(format!(
            "cubikan-t1106-anchor-numeric-type-mutation-{}-{index}",
            std::process::id()
        ));
        if directory.exists() {
            fs::remove_dir_all(&directory).expect("remove stale numeric mutation fixture");
        }
        copy_static_contract(&directory, &mutated);
        let output = run_static_verifier(&directory);
        assert!(
            !output.status.success(),
            "static verifier accepted a JSON numeric type substitution at {path}"
        );
        fs::remove_dir_all(directory).expect("remove numeric mutation fixture");
    }
}

#[test]
fn test_runtime_executive_separates_preinclusion_and_dispatch_failure_effects() {
    let storage = local_genesis_config()
        .build_storage()
        .expect("the exact local runtime genesis must build");
    let mut externalities = sp_io::TestExternalities::new(storage);

    externalities.execute_with(|| {
        Executive::initialize_block(&block_one_header());
        let charlie = Sr25519Keyring::Charlie.to_account_id();
        let initial_domain = domain_state();
        let initial_nonce = System::account(&charlie).nonce;
        let initial_balance = Balances::free_balance(&charlie);
        let initial_cubikan_events = cubikan_event_count();
        let initial_system_events = System::events();

        let well_formed = signed_extrinsic(
            Sr25519Keyring::Charlie,
            initial_nonce,
            rejected_domain_call(),
        );
        let mut malformed = well_formed.encode();
        malformed.pop().expect("encoded extrinsic is nonempty");
        assert!(UncheckedExtrinsic::decode(&mut malformed.as_slice()).is_err());
        assert_eq!(System::account(&charlie).nonce, initial_nonce);
        assert_eq!(Balances::free_balance(&charlie), initial_balance);
        assert_eq!(domain_state(), initial_domain);
        assert_eq!(cubikan_event_count(), initial_cubikan_events);
        assert_eq!(System::events(), initial_system_events);

        let future = signed_extrinsic(
            Sr25519Keyring::Charlie,
            initial_nonce + 1,
            rejected_domain_call(),
        );
        assert!(Executive::apply_extrinsic(future).is_err());
        assert_eq!(System::account(&charlie).nonce, initial_nonce);
        assert_eq!(Balances::free_balance(&charlie), initial_balance);
        assert_eq!(domain_state(), initial_domain);
        assert_eq!(cubikan_event_count(), initial_cubikan_events);
        assert_eq!(System::events(), initial_system_events);

        let included = signed_extrinsic(
            Sr25519Keyring::Charlie,
            initial_nonce,
            rejected_domain_call(),
        );
        assert_eq!(
            Executive::apply_extrinsic(included)
                .expect("the well-formed extrinsic passes transaction validity"),
            Err(pallet_cubikan::Error::<Runtime>::AssociationUnitNotFound.into())
        );
        assert_eq!(System::account(&charlie).nonce, initial_nonce + 1);
        assert!(Balances::free_balance(&charlie) < initial_balance);
        assert_eq!(domain_state(), initial_domain);
        assert_eq!(cubikan_event_count(), initial_cubikan_events);
        let final_events = System::events();
        assert_eq!(
            final_events
                .iter()
                .filter(|record| matches!(
                    &record.event,
                    RuntimeEvent::System(frame_system::Event::ExtrinsicFailed { .. })
                ))
                .count(),
            1,
            "the included domain rejection emits exactly one System failure"
        );
        assert!(matches!(
            &final_events
                .last()
                .expect("one included failure event")
                .event,
            RuntimeEvent::System(frame_system::Event::ExtrinsicFailed { .. })
        ));
        assert_eq!(Cubikan::global_sequence(), None);
    });
}
