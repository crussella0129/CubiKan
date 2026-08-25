#!/usr/bin/bash
set -euo pipefail

readonly TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_ROOT="$(cd -- "$TOOL_DIR/../.." && pwd -P)"
readonly CONFIG="$PROJECT_ROOT/chain/config/zombienet.toml"
readonly MATERIALIZER="$TOOL_DIR/materialize-zombienet.sh"
readonly LAUNCHER="$TOOL_DIR/zombienet-node-launcher.sh"
readonly RUNNER="$TOOL_DIR/run-zombienet-e2e.sh"
readonly JOURNEY="$TOOL_DIR/run-four-node-journey.sh"
readonly DRIVER="$PROJECT_ROOT/tests/chain-e2e/driver.mjs"
readonly VERIFIER="$TOOL_DIR/verify-pins.sh"
readonly LOOPBACK="$TOOL_DIR/loopback-netns.sh"
readonly ZOMBIENET_ARCHIVE="$PROJECT_ROOT/chain/.cache/downloads/zombienet-a7c434271f094320d17cf94f7a2f95fdef417379.tar.gz"
readonly TEST_ROOT="$(mktemp -d)"
readonly RUN_DIR="$TEST_ROOT/run"
trap '/usr/bin/rm -rf -- "$TEST_ROOT"' EXIT

die() {
    printf 'zombienet-t1115.test: %s\n' "$*" >&2
    exit 1
}

for subject in "$CONFIG" "$MATERIALIZER" "$LAUNCHER" "$RUNNER" "$JOURNEY" \
    "$DRIVER" "$VERIFIER" "$LOOPBACK"; do
    [[ -f "$subject" && ! -L "$subject" ]] || die "missing or symbolic subject ${subject##*/}"
done
[[ "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%s' -- "$LOOPBACK")" == 38972 ]] ||
    die 'loopback namespace wrapper size drifted'
loopback_digest="$(/usr/lib/cargo/bin/coreutils/sha256sum -- "$LOOPBACK")"
[[ "${loopback_digest%% *}" == 8c5a8bc685888c506ea189f32205810c80c6c58170dca0ffcbcc914efd2ecb36 ]] ||
    die 'loopback namespace wrapper hash drifted'
unset loopback_digest
[[ -x "$MATERIALIZER" && -x "$LAUNCHER" && -x "$RUNNER" && -x "$JOURNEY" ]] ||
    die 'one or more T-1115 launch tools are not executable'

test_config_is_the_closed_four_node_topology() {
    /usr/bin/python3.14 -I -S - "$CONFIG" <<'PY'
import pathlib
import re
import sys
import tomllib

path = pathlib.Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
config = tomllib.loads(text)

assert "rpc_port fields are Zombienet orchestration allocations only" in text

assert set(config) == {"settings", "relaychain", "parachains"}
assert set(config["settings"]) == {
    "provider", "bootnode", "timeout", "node_spawn_timeout", "local_ip",
    "telemetry", "prometheus", "enable_tracing", "polkadot_introspector",
    "backchannel", "node_verifier", "isolate_env",
}
assert config["settings"] == {
    "provider": "native",
    "bootnode": False,
    "timeout": 1200,
    "node_spawn_timeout": 180,
    "local_ip": "127.0.0.1",
    "telemetry": False,
    "prometheus": True,
    "enable_tracing": False,
    "polkadot_introspector": False,
    "backchannel": False,
    "node_verifier": "Metric",
    "isolate_env": False,
}

relay = config["relaychain"]
assert set(relay) == {
    "default_image", "default_command", "default_substrate_cli_args_version",
    "chain", "nodes",
}
assert relay["default_image"] == "localhost/cubikan/pinned:local"
assert relay["default_command"] == "{{CUBIKAN_POLKADOT}}"
assert relay["default_substrate_cli_args_version"] == "2"
assert relay["chain"] == "rococo-local"
nodes = relay["nodes"]
assert len(nodes) == 2
expected_relay = [
    ("alice", "relay-a", 9944, 30333, 9615),
    ("bob", "relay-b", 9945, 30334, 9616),
]
for node, (name, role, rpc, p2p, metrics) in zip(nodes, expected_relay, strict=True):
    expected_keys = {
        "name", "command", "validator", "invulnerable", "rpc_port",
        "p2p_port", "prometheus_port", "args",
    }
    assert set(node) == expected_keys
    assert node["name"] == name
    assert node["command"] == (
        "exec {{CUBIKAN_ZOMBIENET_NODE_LAUNCHER}} "
        f"--role {role} -- --port {p2p} "
        "--execute-workers-max-num 1 --prepare-workers-soft-max-num 1 "
        "--prepare-workers-hard-max-num 1"
    )
    assert node["validator"] is True and node["invulnerable"] is True
    assert (node["rpc_port"], node["p2p_port"], node["prometheus_port"]) == (
        rpc, p2p, metrics
    )
assert nodes[0]["args"] == [
    "--bootnodes",
    "/ip4/127.0.0.1/tcp/30334/ws/p2p/"
    "12D3KooWRkZhiRhsqmrQ28rt73K7V3aCBpqKrLGSXmZ99PTcTZby",
    "--workers-path",
    "{{CUBIKAN_POLKADOT_WORKERS_DIR}}",
]
assert nodes[1]["args"] == [
    "--workers-path",
    "{{CUBIKAN_POLKADOT_WORKERS_DIR}}",
]

paras = config["parachains"]
assert len(paras) == 1
para = paras[0]
assert set(para) == {
    "id", "chain", "chain_spec_path", "chain_spec_command",
    "genesis_state_path", "genesis_wasm_path",
    "default_substrate_cli_args_version", "cumulus_based",
    "add_to_genesis", "register_para", "onboard_as_parachain",
    "with_custom_props", "collators",
}
assert para["id"] == 1000 and para["chain"] == "cubikan-local"
assert para["chain_spec_path"] == "{{CUBIKAN_PARACHAIN_CHAIN_SPEC}}"
assert para["chain_spec_command"] == (
    "{{CUBIKAN_POLKADOT_OMNI_NODE}} build-spec --disable-default-bootnode"
)
assert para["genesis_state_path"] == "{{CUBIKAN_PARACHAIN_GENESIS_HEAD}}"
assert para["genesis_wasm_path"] == "{{CUBIKAN_PARACHAIN_GENESIS_WASM}}"
assert para["default_substrate_cli_args_version"] == "2"
assert para["cumulus_based"] is True and para["add_to_genesis"] is True
assert para["register_para"] is False and para["onboard_as_parachain"] is True
assert para["with_custom_props"] is False

collators = para["collators"]
assert len(collators) == 2
expected_collators = [
    ("alice", "collator-a", 9988, 30335, 9617, 30336,
     "12D3KooWDV1yAeEGiye3t2CQpW7MJ5TJV3TTKpTUUxv4xtaggSnA"),
    ("bob", "collator-b", 9989, 30336, 9618, 30335,
     "12D3KooWHhaSXEhWFi3LibWRNgF9PezoqB9Xeae4fS3dxCowJEg3"),
]
alice_relay_peer = (
    "/ip4/127.0.0.1/tcp/30333/ws/p2p/"
    "12D3KooWQCkBm1BYtkHpocxCwMgR8yjitEeHGx8spzcDLGt2gkBm"
)
for node, expected in zip(collators, expected_collators, strict=True):
    assert set(node) == {
        "name", "image", "command", "validator", "invulnerable",
        "rpc_port", "p2p_port", "prometheus_port", "args",
    }
    name, role, rpc, p2p, metrics, peer_port, peer_id = expected
    assert node["name"] == name
    assert node["image"] == "localhost/cubikan/pinned:local"
    assert node["command"] == (
        "exec {{CUBIKAN_ZOMBIENET_NODE_LAUNCHER}} "
        f"--role {role} -- --port {p2p}"
    )
    assert node["validator"] is True and node["invulnerable"] is True
    assert (node["rpc_port"], node["p2p_port"], node["prometheus_port"]) == (
        rpc, p2p, metrics
    )
    args = node["args"]
    assert args[:6] == [
        "--blocks-pruning", "archive", "--state-pruning", "archive",
        "--bootnodes", f"/ip4/127.0.0.1/tcp/{peer_port}/ws/p2p/{peer_id}",
    ]
    assert args[6:] == [
        "--", "--bootnodes", alice_relay_peer,
        "--workers-path", "{{CUBIKAN_POLKADOT_WORKERS_DIR}}",
        "--execute-workers-max-num", "1",
        "--prepare-workers-soft-max-num", "1",
        "--prepare-workers-hard-max-num", "1",
    ]
    assert args.count("--blocks-pruning") == 1
    assert args.count("--state-pruning") == 1
    assert args.count("--") == 1

tokens = set(re.findall(r"{{([A-Z][A-Z0-9_]*)}}", text))
assert tokens == {
    "CUBIKAN_ZOMBIENET_NODE_LAUNCHER",
    "CUBIKAN_POLKADOT",
    "CUBIKAN_POLKADOT_WORKERS_DIR",
    "CUBIKAN_POLKADOT_OMNI_NODE",
    "CUBIKAN_PARACHAIN_CHAIN_SPEC",
    "CUBIKAN_PARACHAIN_GENESIS_HEAD",
    "CUBIKAN_PARACHAIN_GENESIS_WASM",
}
assert "{{chainName}}" not in text
assert "30337" not in text and "30338" not in text
assert "9990" not in text and "9991" not in text
assert "9619" not in text and "9620" not in text

# Model the pinned readNetworkConfig Nunjucks pass with exact test values.
render_values = {
    "CUBIKAN_ZOMBIENET_NODE_LAUNCHER": "/sealed/node-launcher",
    "CUBIKAN_POLKADOT": "/sealed/polkadot",
    "CUBIKAN_POLKADOT_WORKERS_DIR": "/run/cubikan-exec/pvf-workers",
    "CUBIKAN_POLKADOT_OMNI_NODE": "/sealed/polkadot-omni-node",
    "CUBIKAN_PARACHAIN_CHAIN_SPEC": "/sealed/cubikan-local.json",
    "CUBIKAN_PARACHAIN_GENESIS_HEAD": "/sealed/genesis-head",
    "CUBIKAN_PARACHAIN_GENESIS_WASM": "/sealed/genesis-wasm",
}
rendered_text = text
for token, value in render_values.items():
    rendered_text = rendered_text.replace("{{" + token + "}}", value)
assert "{{" not in rendered_text and "}}" not in rendered_text
rendered = tomllib.loads(rendered_text)
assert rendered["relaychain"]["default_command"] == "/sealed/polkadot"
relay_default_template = (
    rendered["relaychain"]["default_command"]
    + " build-spec --chain {{chainName}} --disable-default-bootnode"
)
relay_plain_command = relay_default_template.replace("{{chainName}}", "rococo-local")
relay_raw_command = relay_default_template.replace(
    "{{chainName}}", "/sealed/modified-relay-plain.json"
)
assert relay_plain_command == (
    "/sealed/polkadot build-spec --chain rococo-local --disable-default-bootnode"
)
assert relay_raw_command == (
    "/sealed/polkadot build-spec --chain /sealed/modified-relay-plain.json "
    "--disable-default-bootnode"
)
assert relay_raw_command.count("--chain") == 1
assert "--chain rococo-local" not in relay_raw_command
rendered_para_command = rendered["parachains"][0]["chain_spec_command"]
assert rendered_para_command == (
    "/sealed/polkadot-omni-node build-spec --disable-default-bootnode"
)
generated_para_command = (
    rendered_para_command + " --chain /sealed/copied-plain-spec.json"
)
assert generated_para_command.count("--chain") == 1
assert generated_para_command.endswith("--chain /sealed/copied-plain-spec.json")
PY
}

test_pinned_provider_compatibility_is_explicit() {
    /usr/bin/python3.14 -I -S - "$ZOMBIENET_ARCHIVE" <<'PY'
import sys
import tarfile

archive = sys.argv[1]
root = "zombienet-a7c434271f094320d17cf94f7a2f95fdef417379"
with tarfile.open(archive, "r:gz") as source:
    cmd = source.extractfile(
        f"{root}/javascript/packages/orchestrator/src/cmdGenerator.ts"
    ).read().decode()
    boot = source.extractfile(
        f"{root}/javascript/packages/orchestrator/src/bootnode.ts"
    ).read().decode()
    config = source.extractfile(
        f"{root}/javascript/packages/orchestrator/src/configGenerator.ts"
    ).read().decode()
    constants = source.extractfile(
        f"{root}/javascript/packages/orchestrator/src/constants.ts"
    ).read().decode()
    native_chain_spec = source.extractfile(
        f"{root}/javascript/packages/orchestrator/src/providers/native/chainSpec.ts"
    ).read().decode()
    orchestrator = source.extractfile(
        f"{root}/javascript/packages/orchestrator/src/orchestrator.ts"
    ).read().decode()

start = cmd.index("const getPortFlagsByCliArgsVersion")
end = cmd.index("return portFlags", start)
port_helper = cmd[start:end]
assert 'portFlags["--rpc-port"]' in port_helper
assert '"--prometheus-port"' in port_helper
assert 'portFlags["--port"]' not in port_helper
assert '"--rpc-cors"' in cmd and '"--rpc-methods"' in cmd
relay_start = cmd.index("const collatorPorts")
relay_end = cmd.index("if (nodeSetup.args.length", relay_start)
relay_port_helper = cmd[relay_start:relay_end]
assert '"--port"' in relay_port_helper
assert '"--rpc-port"' in relay_port_helper
assert '"--prometheus-port"' in relay_port_helper
assert '"--rpc-cors"' not in relay_port_helper
assert '"--rpc-methods"' not in relay_port_helper
assert "useWs = true" in boot
assert 'useWs ? "ws/" : "/"' in boot
assert "const uniqueArgs = [...new Set(args)]" in config
assert '"{{DEFAULT_COMMAND}} build-spec --chain {{chainName}} --disable-default-bootnode"' in constants
assert "config.relaychain.chain_spec_command" in config
assert ": DEFAULT_CHAIN_SPEC_COMMAND.replace(" in config
assert '"{{DEFAULT_COMMAND}}",' in config
assert "networkSpec.relaychain.defaultCommand," in config
assert "chainSpecCommand.replace(" in native_chain_spec
assert "/{{chainName}}/gi," in native_chain_spec
assert "remoteChainSpecFullPath," in native_chain_spec
assert "`${chainSpecCommandRaw}  --raw > ${remoteChainSpecRawFullPath}`" in native_chain_spec
add_para = orchestrator.index("await addParachainToGenesis(")
customize_relay = orchestrator.index(
    "await customizePlainRelayChain(chainSpecFullPathPlain, networkSpec);"
)
raw_relay = orchestrator.index("await getChainSpecRaw(", customize_relay)
assert add_para < customize_relay < raw_relay
raw_relay_call = orchestrator[raw_relay:orchestrator.index(");", raw_relay) + 2]
assert "networkSpec.relaychain.chainSpecCommand!" in raw_relay_call
assert "chainSpecFullPath" in raw_relay_call
assert "const command = node.command\n    ? node.command\n    : networkSpec.relaychain.defaultCommand;" in config
assert "collatorConfig.command || DEFAULT_CUMULUS_COLLATOR_BIN" in config
assert "command: collatorBinary" in config
PY
    /usr/bin/grep -Fq 'pinned native provider' "$LAUNCHER" ||
        die 'launcher omits the pinned-provider compatibility explanation'
    /usr/bin/grep -Fq -- '-- --port 30333' "$CONFIG" ||
        die 'config omits the missing generated primary p2p flag'
}

test_materializer_and_candidate_gate_are_closed() {
    /usr/bin/grep -Fq 'source.extractall(destination_path, members=members, filter="data")' \
        "$MATERIALIZER" || die 'materializer lacks filtered exact extraction'
    /usr/bin/grep -Fq 'archive contains a hardlink or special member' "$MATERIALIZER" ||
        die 'materializer lacks special-member rejection'
    /usr/bin/grep -Fq 'archive symlink inventory mismatch' "$MATERIALIZER" ||
        die 'materializer lacks exact Node symlink inventory validation'
    /usr/bin/grep -Fq 'flags |= os.O_NOFOLLOW' "$MATERIALIZER" ||
        die 'materializer does not open archives with no-follow semantics'
    /usr/bin/grep -Fq 'tarfile.open(fileobj=archive_file, mode="r:*")' \
        "$MATERIALIZER" || die 'materializer reopens archives by pathname'
    [[ "$(/usr/bin/grep -Fc 'hash_open_file(archive_file) != expected_sha256' \
        "$MATERIALIZER")" == 2 ]] ||
        die 'materializer does not hash the same archive descriptor before and after use'
    /usr/bin/grep -Fq 'identity(os.fstat(archive_file.fileno())) != identity(initial)' \
        "$MATERIALIZER" || die 'materializer does not recheck opened archive identity'
    /usr/bin/grep -Fq 'run_npm ci --offline --ignore-scripts --no-audit --no-fund' \
        "$MATERIALIZER" || die 'materializer lacks the offline ignore-scripts npm ci gate'
    /usr/bin/grep -Fq 'readonly NODE="$NODE_HOME/bin/node"' "$MATERIALIZER" ||
        die 'materializer lacks the deterministic pinned Node path'
    /usr/bin/grep -Fq 'readonly NPM_CLI="$NODE_HOME/lib/node_modules/npm/bin/npm-cli.js"' \
        "$MATERIALIZER" || die 'materializer lacks the direct pinned npm CLI path'
    /usr/bin/grep -Fq 'readonly TOML_GIT_COMMIT=5e17114f1af5b5b70e4f2ec10cd007623c928988' \
        "$MATERIALIZER" || die 'materializer lacks the exact toml Git commit'
    /usr/bin/grep -Fq 'readonly TOML_INSTALLED_TREE_SHA256=2faea9de33ef0b6a95e7823a17c8beddb514f10873b8988fa65755ecff9114c7' \
        "$MATERIALIZER" || die 'materializer lacks the installed toml tree identity'
    /usr/bin/grep -Fq 'entry = lock.get("packages", {}).get("node_modules/toml")' \
        "$MATERIALIZER" || die 'materializer does not validate the Git lock entry'
    /usr/bin/grep -Fq 'git_entries != [("node_modules/toml", expected["resolved"])]' \
        "$MATERIALIZER" || die 'materializer does not reject additional Git dependencies'
    [[ "$(/usr/bin/grep -Fc 'verify_toml_install' "$MATERIALIZER")" == 3 ]] ||
        die 'materializer does not verify the toml tree after install and build'
    /usr/bin/grep -Fq 'tree contains a symbolic link' "$MATERIALIZER" ||
        die 'materializer does not reject symbolic installed-tree entries'
    /usr/bin/grep -Fq 'tree contains a nonregular entry' "$MATERIALIZER" ||
        die 'materializer does not reject nonregular installed-tree entries'
    /usr/bin/grep -Fq '"$NODE" "$NPM_CLI" "$@"' "$MATERIALIZER" ||
        die 'materializer does not execute npm through pinned Node and npm-cli bytes'
    /usr/bin/grep -Fq 'readonly GIT=/usr/bin/git' "$MATERIALIZER" ||
        die 'materializer lacks the canonical pinned Git path'
    /usr/bin/grep -Fq '"$(sha256_file "$GIT")" == "$GIT_SHA256"' "$MATERIALIZER" ||
        die 'materializer does not authenticate the Git executable bytes'
    /usr/bin/grep -Fq 'npm_config_userconfig="$USER_NPMRC" npm_config_globalconfig="$GLOBAL_NPMRC"' \
        "$MATERIALIZER" || die 'materializer lacks distinct explicit npm configuration inputs'
    for cache_cap in \
        'readonly NPM_CACHE_SNAPSHOT_MAX_ENTRIES=10000' \
        'readonly NPM_CACHE_SNAPSHOT_MAX_BYTES=268435456' \
        'readonly NPM_CACHE_SNAPSHOT_MAX_DEPTH=32' \
        'readonly NPM_CACHE_SNAPSHOT_MAX_PATH_BYTES=512'; do
        /usr/bin/grep -Fq "$cache_cap" "$MATERIALIZER" ||
            die "materializer lacks private npm-cache cap: $cache_cap"
    done
    /usr/bin/grep -Fq 'snapshot_npm_cache "$NPM_CACHE" "$PRIVATE_NPM_CACHE"' \
        "$MATERIALIZER" || die 'materializer does not snapshot the shared npm cache'
    /usr/bin/grep -Fq 'os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW' \
        "$MATERIALIZER" || die 'materializer npm-cache snapshot is not descriptor/no-follow based'
    /usr/bin/grep -Fq 'cache snapshot source contains a symbolic or special entry' \
        "$MATERIALIZER" || die 'materializer npm-cache snapshot does not reject special entries'
    /usr/bin/grep -Fq 'cache directory inventory drift after copy' \
        "$MATERIALIZER" || die 'materializer npm-cache snapshot does not reject identity drift'
    /usr/bin/grep -Fq 'npm_config_cache="$PRIVATE_NPM_CACHE"' "$MATERIALIZER" ||
        die 'materializer npm boundary does not use the private cache snapshot'
    /usr/bin/grep -Fq 'npm_config_loglevel=error' "$MATERIALIZER" ||
        die 'materializer npm boundary does not suppress unauditable advisory chatter'
    if /usr/bin/grep -Fq 'npm_config_cache="$NPM_CACHE"' "$MATERIALIZER"; then
        die 'materializer npm boundary still writes the shared cache'
    fi
    /usr/bin/grep -Fq 'run_npm cache verify --offline --no-audit --no-fund' \
        "$MATERIALIZER" || die 'materializer does not verify its private cache before install'
    [[ "$(/usr/bin/grep -Fc 'verify_npm_cache_tree "$PRIVATE_NPM_CACHE"' \
        "$MATERIALIZER")" == 4 ]] ||
        die 'materializer does not verify its private cache at every boundary'
    for git_rule in \
        'GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null' \
        'GIT_TERMINAL_PROMPT=0 GIT_ASKPASS=/nonexistent SSH_ASKPASS=/nonexistent' \
        'GIT_SSH=/nonexistent GIT_SSH_COMMAND=/nonexistent GIT_PROXY_COMMAND=/nonexistent' \
        'GIT_EXEC_PATH=/nonexistent GIT_ALLOW_PROTOCOL= GIT_PROTOCOL_FROM_USER=0' \
        'GIT_NO_REPLACE_OBJECTS=1 GIT_NO_LAZY_FETCH=1 GIT_OPTIONAL_LOCKS=0'; do
        /usr/bin/grep -Fq "$git_rule" "$MATERIALIZER" ||
            die "materializer lacks Git scrub rule: $git_rule"
    done
    /usr/bin/grep -Fq 'for workspace in utils orchestrator cli; do' "$MATERIALIZER" ||
        die 'materializer lacks the exact direct TypeScript workspace inventory'
    /usr/bin/grep -Fq 'run_node "$JAVASCRIPT/node_modules/typescript/bin/tsc"' \
        "$MATERIALIZER" || die 'materializer does not execute the pinned TypeScript compiler directly'
    /usr/bin/grep -Fq 'fs.rmSync(directory, { recursive: true, force: true });' \
        "$MATERIALIZER" || die 'materializer lacks the direct Node build-output cleanup'
    /usr/bin/grep -Fq 'fs.cpSync(process.argv[1], process.argv[2], {' \
        "$MATERIALIZER" || die 'materializer lacks the direct Node resource copy'
    if /usr/bin/grep -Fq 'npm_config_script_shell' "$MATERIALIZER" ||
        /usr/bin/grep -Fq 'run_npm run build' "$MATERIALIZER"; then
        die 'materializer reintroduced an npm script-shell build dependency'
    fi
    /usr/bin/grep -Fq 'readonly CLI="$JAVASCRIPT/packages/cli/dist/cli.js"' "$MATERIALIZER" ||
        die 'materializer lacks the deterministic built CLI path'

    /usr/bin/grep -Fq 'generated chain spec lacks raw genesis top storage' "$LAUNCHER" ||
        die 'launcher does not require raw genesis chain-spec structure'
    /usr/bin/grep -Fq 'generated chain spec changed before canonical publication' "$LAUNCHER" ||
        die 'launcher does not recheck the canonicalization source before publication'
    /usr/bin/grep -Fq 'generated chain spec changed across canonical publication' "$LAUNCHER" ||
        die 'launcher does not recheck the canonicalization source after publication'
    /usr/bin/grep -Fq 'generated chain spec changed across the canonicalizer handoff' "$LAUNCHER" ||
        die 'launcher does not bind canonicalizer output to the Bash pin handoff'
    /usr/bin/grep -Fq 'hashlib.sha256(final_destination).hexdigest()' "$LAUNCHER" ||
        die 'launcher canonicalizer does not return its exact destination hash'

    /usr/bin/grep -Fq -- '--assert-current-isolated' "$RUNNER" ||
        die 'candidate runner omits the live namespace proof'
    /usr/bin/grep -Fq '/var/tmp/cubikan-t1115.XXXXXX' "$RUNNER" ||
        die 'candidate runner omits the supported ext4 root'
    /usr/bin/grep -Fq "'700:0:directory'" "$RUNNER" ||
        die 'candidate runner omits owner-only root validation'
    /usr/bin/grep -Fq -- '--kill-after=30s 1770s' "$RUNNER" ||
        die 'candidate runner omits the bounded 30-minute terminate/kill limit'
    [[ "$(/usr/bin/grep -Fc -- '--test chain_e2e --release --locked --offline' "$RUNNER")" == 2 ]] ||
        die 'candidate runner must compile and execute the exact gate in the release profile'
    /usr/bin/grep -Fq \
        'readonly RUSTC=/home/charles/.rustup/toolchains/1.93.0-x86_64-unknown-linux-gnu/bin/rustc' \
        "$RUNNER" || die 'candidate runner does not pin the exact rustc child executable'
    /usr/bin/grep -Fq \
        'readonly RUSTDOC=/home/charles/.rustup/toolchains/1.93.0-x86_64-unknown-linux-gnu/bin/rustdoc' \
        "$RUNNER" || die 'candidate runner does not pin the exact rustdoc child executable'
    /usr/bin/grep -Fq 'export RUSTC RUSTDOC' "$RUNNER" ||
        die 'candidate runner does not export the pinned compiler children'
    /usr/bin/grep -Fq -- '--ignored --test-threads=1' "$RUNNER" ||
        die 'candidate runner does not make ignored tests actual and serial'
}

test_archive_checker_executes_on_exact_opened_bytes() {
    local checker safe_archive unsafe_archive safe_destination unsafe_destination
    local size digest
    checker="$TEST_ROOT/archive-checker.py"
    safe_archive="$TEST_ROOT/safe.tar"
    unsafe_archive="$TEST_ROOT/unsafe.tar"
    safe_destination="$TEST_ROOT/safe-extract"
    unsafe_destination="$TEST_ROOT/unsafe-extract"
    /usr/bin/gawk '
        /^read -r -d.*ARCHIVE_CHECKER.*<<.PY./ { capture=1; next }
        capture && $0 == "PY" { exit }
        capture { print }
    ' "$MATERIALIZER" >"$checker"
    [[ -s "$checker" ]] || die 'could not bind the materializer archive checker'
    /usr/bin/python3.14 -I -S - "$safe_archive" "$unsafe_archive" <<'PY'
import io
import sys
import tarfile

safe_path, unsafe_path = sys.argv[1:]
payload = b"pinned fixture\n"
with tarfile.open(safe_path, "w") as archive:
    root = tarfile.TarInfo("fixture-root")
    root.type = tarfile.DIRTYPE
    root.mode = 0o700
    archive.addfile(root)
    member = tarfile.TarInfo("fixture-root/value")
    member.size = len(payload)
    member.mode = 0o600
    archive.addfile(member, io.BytesIO(payload))
with tarfile.open(unsafe_path, "w") as archive:
    root = tarfile.TarInfo("fixture-root")
    root.type = tarfile.DIRTYPE
    archive.addfile(root)
    escaping = tarfile.TarInfo("fixture-root/../escape")
    escaping.size = len(payload)
    archive.addfile(escaping, io.BytesIO(payload))
PY
    /usr/bin/mkdir -m 0700 -- "$safe_destination" "$unsafe_destination"
    size="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%s' -- "$safe_archive")"
    digest="$(/usr/lib/cargo/bin/coreutils/sha256sum -- "$safe_archive")"
    digest=${digest%% *}
    /usr/bin/python3.14 -I -S "$checker" "$safe_archive" "$safe_destination" \
        fixture-root '' "$size" "$digest" || die 'exact archive checker rejected safe bytes'
    [[ "$(<"$safe_destination/fixture-root/value")" == 'pinned fixture' ]] ||
        die 'exact archive checker extracted the wrong bytes'
    size="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%s' -- "$unsafe_archive")"
    digest="$(/usr/lib/cargo/bin/coreutils/sha256sum -- "$unsafe_archive")"
    digest=${digest%% *}
    if /usr/bin/python3.14 -I -S "$checker" "$unsafe_archive" "$unsafe_destination" \
        fixture-root '' "$size" "$digest" >/dev/null 2>&1; then
        die 'exact archive checker accepted an escaping member'
    fi
}

validate_fresh_npm_closure_source() {
    /usr/bin/python3.14 -I -S - "$1" <<'PY'
import pathlib
import sys

source = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")

def function(name, next_name):
    start = source.index(name + "() {")
    end = source.index("\n" + next_name + "() {", start)
    return source[start:end]

assert (
    'readonly EXPECTED_PINS_SHA256="'
    '641a3b8f325dcd20cf40831c4ed0f7969a912f0647fbd716bf12c33dba6d5383"'
) in source
fetch = function("fetch_all", "verify_downloads")
assert fetch.index('download_exact "$(pin zombienet archive_url)"') < fetch.index(
    "populate_zombienet_npm_cache"
)
assert fetch.index('download_exact "$(pin node archive_url)"') < fetch.index(
    "populate_zombienet_npm_cache"
)
populate = function("populate_zombienet_npm_cache", "verify_sdk_and_scaffold")
assert 'mktemp -d "$CACHE/npm.incoming.XXXXXX"' in populate
assert "verify_zombienet_npm_build" in populate and "online" in populate
assert "cache verify --offline --no-audit --no-fund" in populate
assert "verify_zombienet_npm_build" in populate and "offline" in populate
online_call = '"$incoming" online'
offline_call = '"$incoming" offline'
assert populate.index(online_call) < populate.index(offline_call)
assert populate.index(offline_call) < populate.index("publish_zombienet_npm_cache")
publish = function("publish_zombienet_npm_cache", "populate_zombienet_npm_cache")
assert '/usr/bin/gnumv -T -- "$incoming" "$NPM_CACHE"' in publish
assert 'npm.backup.XXXXXX' in publish
verify = function("verify_node_and_zombienet", "verify_asset_capabilities")
assert '/usr/bin/gnucp -a -- "$NPM_CACHE/." "$verification_cache/"' in verify
assert "cache verify --offline --no-audit --no-fund" in verify
assert "verify_zombienet_npm_build" in verify and "offline" in verify
assert (
    'require_exact_literal zombienet toml_git_commit '
    '5e17114f1af5b5b70e4f2ec10cd007623c928988'
) in source
assert (
    'require_exact_literal zombienet toml_installed_tree_sha256 '
    '2faea9de33ef0b6a95e7823a17c8beddb514f10873b8988fa65755ecff9114c7'
) in source
lock = function("verify_zombienet_toml_lock", "verify_zombienet_toml_install")
assert 'entry = lock.get("packages", {}).get("node_modules/toml")' in lock
assert '"resolved": f"git+ssh://git@github.com/pepoviola/toml-node.git#{commit}"' in lock
assert 'entry != expected' in lock and 'or "integrity" in entry' in lock
assert 'git_entries != [("node_modules/toml", expected["resolved"])]' in lock
installed = function("verify_zombienet_toml_install", "prepare_pinned_git_https_helper")
assert 'actual="$(tree_sha256 "$installed")"' in installed
assert "installed toml Git dependency tree hash mismatch" in installed
helper = function("prepare_pinned_git_https_helper", "run_pinned_npm")
assert 'source="$(pin host_tools git_https_helper_path)"' in helper
assert 'helper="$helper_root/git-remote-https"' in helper
assert 'require_size "$source" "$(pin host_tools git_https_helper_size)"' in helper
assert 'require_hash "$source" "$(pin host_tools git_https_helper_sha256)"' in helper
assert '/usr/bin/gnucp -- "$source" "$helper"' in helper
assert "private Git HTTPS helper inventory drifted" in helper
runner = function("run_pinned_npm", "run_pinned_node")
assert "/usr/lib/cargo/bin/coreutils/env -i" in runner
assert 'git_bin="$(pin host_tools git_path)"' in runner
assert '"$git_bin" == /usr/bin/git' in runner
assert 'require_hash "$git_bin" "$(pin host_tools git_sha256)"' in runner
assert 'npm_config_userconfig="$user_config"' in runner
assert 'npm_config_globalconfig="$global_config"' in runner
assert "npm_config_loglevel=error" in runner
assert "npm configuration inputs are not distinct empty regular files" in runner
assert "npm_config_offline=true" in runner
for rule in (
    "GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null",
    "GIT_TERMINAL_PROMPT=0 GIT_ASKPASS=/nonexistent SSH_ASKPASS=/nonexistent",
    "GIT_SSH=/nonexistent GIT_SSH_COMMAND=/nonexistent GIT_PROXY_COMMAND=/nonexistent",
    "GIT_PROTOCOL_FROM_USER=0",
    "GIT_NO_REPLACE_OBJECTS=1 GIT_NO_LAZY_FETCH=1 GIT_OPTIONAL_LOCKS=0",
):
    assert rule in runner
assert "GIT_EXEC_PATH=/nonexistent" in runner
assert "GIT_ALLOW_PROTOCOL=" in runner
assert "GIT_ALLOW_PROTOCOL=https" in runner
assert "GIT_CONFIG_COUNT=4" in runner
assert "GIT_CONFIG_KEY_3=credential.helper" in runner
assert "GIT_CONFIG_VALUE_3=" in runner
for exact_url in (
    "url.https://github.com/pepoviola/toml-node.git.insteadOf",
    "ssh://git@github.com/pepoviola/toml-node.git",
    "git@github.com:pepoviola/toml-node.git",
    "git+ssh://git@github.com/pepoviola/toml-node.git",
):
    assert exact_url in runner
node_runner = function("run_pinned_node", "build_zombienet_javascript")
assert "/usr/lib/cargo/bin/coreutils/env -i" in node_runner
builder = function("build_zombienet_javascript", "verify_zombienet_npm_build")
assert "fs.rmSync(directory, { recursive: true, force: true });" in builder
assert "for workspace in utils orchestrator cli; do" in builder
assert '"$javascript/node_modules/typescript/bin/tsc"' in builder
assert "fs.cpSync(process.argv[1], process.argv[2], {" in builder
build = function("verify_zombienet_npm_build", "publish_zombienet_npm_cache")
assert 'verify_zombienet_toml_lock "$package_lock"' in build
assert 'prepare_pinned_git_https_helper "$home"' in build
assert build.count('verify_zombienet_toml_install "$zombie_root"') == 2
assert 'build_zombienet_javascript "$node_root" "$javascript" "$home"' in build
assert "npm run build" not in build
PY
}

test_fresh_npm_cache_contract_is_closed() {
    local git_mutated="$TEST_ROOT/verify-pins-open-git-protocol.sh"
    local mutated="$TEST_ROOT/verify-pins-missing-npm-fetch.sh"
    local toml_mutated="$TEST_ROOT/verify-pins-missing-toml-tree.sh"
    validate_fresh_npm_closure_source "$VERIFIER" ||
        die 'verifier lacks the fresh npm cache closure contract'
    /usr/bin/sed '/^    populate_zombienet_npm_cache$/d' "$VERIFIER" >"$mutated"
    if validate_fresh_npm_closure_source "$mutated" >/dev/null 2>&1; then
        die 'fresh npm cache source oracle accepted a missing population stage'
    fi
    /usr/bin/sed '/^    verify_zombienet_toml_install "\$zombie_root"$/d' \
        "$VERIFIER" >"$toml_mutated"
    if validate_fresh_npm_closure_source "$toml_mutated" >/dev/null 2>&1; then
        die 'fresh npm cache source oracle accepted missing Git dependency tree checks'
    fi
    /usr/bin/sed 's/GIT_ALLOW_PROTOCOL=https/GIT_ALLOW_PROTOCOL=http/' \
        "$VERIFIER" >"$git_mutated"
    if validate_fresh_npm_closure_source "$git_mutated" >/dev/null 2>&1; then
        die 'fresh npm cache source oracle accepted an alternate online Git protocol'
    fi
}

validate_journey_driver_interface() {
    local driver=${2:-$DRIVER}
    /usr/bin/python3.14 -I -S - "$1" "$driver" "$LOOPBACK" <<'PY'
import pathlib
import re
import subprocess
import sys

source = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
driver_source = pathlib.Path(sys.argv[2]).read_text(encoding="utf-8")
loopback_source = pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
assert 'readonly TOOLCHAIN_ROOT=/run/cubikan-exec/toolchain' in source
assert 'readonly TOOLCHAIN_ROOT="$WORK_ROOT/toolchain"' not in source
assert 'readonly BOOTSTRAP_LOG_MAX_BYTES=1048576' in source
assert 'readonly BOOTSTRAP_LOG_RETAIN_LIMIT=1048577' in source
assert 'readonly MATERIALIZER_LOG="$BOOTSTRAP_ROOT/materializer.log"' in source
assert 'readonly GENESIS_LOG="$BOOTSTRAP_ROOT/genesis-export.log"' in source
assert 'run_unprivileged "$MATERIALIZER" --output-dir "$TOOLCHAIN_ROOT"' in source
assert 'readonly PINNED_NODE="$TOOLCHAIN_ROOT/node/bin/node"' in source
assert 'readonly EXPECTED_CLI="$ZOMBIENET_ROOT/javascript/packages/cli/dist/cli.js"' in source
assert 'readonly PVF_WORKERS_DIR=/run/cubikan-exec/pvf-workers' in source
assert 'readonly POLKADOT_PREPARE_WORKER=' in source
assert 'readonly POLKADOT_EXECUTE_WORKER=' in source
assert 'readonly UMOUNT=/usr/bin/umount' in source
assert 'readonly SETPRIV=/usr/bin/setpriv' in source
assert source.count(
    '"$SETPRIV" --no-new-privs --inh-caps=-all --ambient-caps=-all'
) == 4
assert source.count('--bounding-set=-all --') == 4
assert source.count('run_unprivileged "$ENV" -i HOME=/home/charles') == 2
assert "os.O_NOFOLLOW | os.O_NONBLOCK" in source
assert 'or stat.S_IMODE(opened.st_mode) & 0o022 != 0' in source
assert 'or stat.S_IMODE(opened.st_mode) & 0o111 != 0o111' in source
assert 'destination_flags = (' in source and "os.O_EXCL" in source
assert 'pvf_workers_created=1' in source
assert 'pvf_workers_mounted=1' in source
assert '"$MOUNT" -o remount,bind,ro,nodev,nosuid "$PVF_WORKERS_DIR"' in source
assert '"$UMOUNT" -- "$PVF_WORKERS_DIR"' in source
assert '/usr/bin/gnurm -rf -- "$PVF_WORKERS_DIR"' in source
assert 'toolchain_created=1' in source
assert 'toolchain_mounted=1' in source
assert '"$MOUNT" -o remount,bind,ro,nodev,nosuid "$TOOLCHAIN_ROOT"' in source
assert '"$UMOUNT" -- "$TOOLCHAIN_ROOT"' in source
assert '/usr/bin/gnurm -rf -- "$TOOLCHAIN_ROOT"' in source
assert source.count("verify_read_only_toolchain") == 3
assert '"$actual_identity" == "$toolchain_root_identity"' in source
assert '"$($STAT -fLc \'%t\' -- "$TOOLCHAIN_ROOT")" == 1021994' in source
assert 'materialized toolchain mount is not one exact executable read-only tmpfs bind' in source
assert '"$TOOLCHAIN_ROOT/.write-probe"' in source
assert 'materialized toolchain did not reject creation with EROFS' in source
assert source.index("trap cleanup EXIT") < source.index(
    "\nmaterialize_read_only_pvf_workers\n"
)
trap_index = source.index("trap cleanup EXIT")
creation = source.index("\ntoolchain_created=2\n", trap_index)
materialize = source.index('run_unprivileged "$MATERIALIZER"', creation)
seal = source.index("\nseal_read_only_toolchain\n", materialize)
assert trap_index < creation < materialize < seal
assert source.count('bs="$BOOTSTRAP_LOG_RETAIN_LIMIT" count=1 iflag=fullblock status=none &') == 2
assert source.count("finish_bootstrap_log_capture") == 3
assert '"$EUID:600:1:regular file"' in source
assert '[[ "$size" =~ ^[0-9]+$ && $size -le $BOOTSTRAP_LOG_MAX_BYTES ]]' in source
assert 'printf \'exec: %s --output-dir %s\\n\'' in source
assert 'printf \'exec: %s export-genesis-head --chain %s %s\\n\'' in source
assert 'printf \'exec: %s export-genesis-wasm --chain %s %s\\n\'' in source
assert '"$POLKADOT_PARACHAIN" export-genesis-head \\' in source
assert '--chain "$PARACHAIN_SPEC" "$GENESIS_HEAD" &&' in source
assert '"$POLKADOT_PARACHAIN" export-genesis-wasm \\' in source
assert '--chain "$PARACHAIN_SPEC" "$GENESIS_WASM"' in source

driver_start = source.index(
    '"$SETSID" "$SETPRIV" --no-new-privs --inh-caps=-all --ambient-caps=-all'
)
driver_end = source.index(
    '    >"$DRIVER_STDOUT_PIPE" 2>"$DRIVER_STDERR_PIPE" &', driver_start
) + len('    >"$DRIVER_STDOUT_PIPE" 2>"$DRIVER_STDERR_PIPE" &')
driver = source[driver_start:driver_end]
expected_driver = '''"$SETSID" "$SETPRIV" --no-new-privs --inh-caps=-all --ambient-caps=-all \\
    --bounding-set=-all -- \\
    "$ENV" -i HOME=/home/charles LANG=C LC_ALL=C PATH=/usr/bin:/bin \\
    PWD="$PROJECT_ROOT" SHLVL=0 TZ=UTC \\
    CUBIKAN_LOCAL_TEST_BINARY="$LOCAL_BINARY" \\
    CUBIKAN_TEST_SUPPORTED_ROOT="$SUPPORTED_ROOT" \\
    CUBIKAN_ZOMBIENET_RUN_DIR="$NETWORK_ROOT" \\
    CUBIKAN_ZOMBIENET_NODE_LAUNCHER="$NODE_LAUNCHER" \\
    CUBIKAN_POLKADOT="$POLKADOT" \\
    CUBIKAN_POLKADOT_WORKERS_DIR="$PVF_WORKERS_DIR" \\
    CUBIKAN_POLKADOT_OMNI_NODE="$POLKADOT_OMNI_NODE" \\
    CUBIKAN_POLKADOT_PARACHAIN="$POLKADOT_PARACHAIN" \\
    CUBIKAN_PARACHAIN_CHAIN_SPEC="$PARACHAIN_SPEC" \\
    CUBIKAN_PARACHAIN_GENESIS_HEAD="$GENESIS_HEAD" \\
    CUBIKAN_PARACHAIN_GENESIS_WASM="$GENESIS_WASM" \\
    "$PINNED_NODE" "$DRIVER" \\
    --fixture "$FIXTURE" \\
    --evidence "$EVIDENCE_PATH" \\
    --work-root "$WORK_ROOT" \\
    --config "$CONFIG" \\
    --zombienet-root "$ZOMBIENET_ROOT" \\
    >"$DRIVER_STDOUT_PIPE" 2>"$DRIVER_STDERR_PIPE" &'''
assert driver == expected_driver

cleanup_start = source.index("cleanup() {")
cleanup_end = source.index("\nterminate() {", cleanup_start)
cleanup = source[cleanup_start:cleanup_end]
for required in (
    '/usr/bin/kill -TERM -- "-$driver_group"',
    '/usr/bin/kill -KILL -- "-$driver_group"',
    'wait "$driver_pid"',
    '"${bootstrap_reader_pid:-}"',
    '"${stdout_reader_pid:-}" "${stderr_reader_pid:-}"',
    '/usr/bin/kill -KILL -- "$reader_pid"',
    'wait "$reader_pid"',
    '"$UMOUNT" -- "$TOOLCHAIN_ROOT"',
    '[[ "$TOOLCHAIN_ROOT" == /run/cubikan-exec/toolchain',
    '/usr/bin/gnurm -rf -- "$TOOLCHAIN_ROOT"',
    '"$UMOUNT" -- "$PVF_WORKERS_DIR"',
    '/usr/bin/gnurm -rf -- "$PVF_WORKERS_DIR"',
    '[[ "$WORK_ROOT" == "$SESSION_ROOT/work"',
    '/usr/bin/gnurm -rf -- "$WORK_ROOT"',
    '((journey_succeeded == 0))',
    '[[ "$EVIDENCE_PATH" == "$SESSION_ROOT/evidence-v1.json"',
    '/usr/bin/gnurm -f -- "$EVIDENCE_PATH"',
    '"$SESSION_ROOT/.driver.stdout.pipe" "$SESSION_ROOT/.driver.stderr.pipe"',
):
    assert required in cleanup
assert "trap cleanup EXIT" in source
assert "trap terminate HUP INT TERM" in source
assert "readonly DRIVER_OUTPUT_RETAIN_LIMIT=65536" in source
assert "readonly DRIVER_CAPTURE_PROGRAM='import os" in source
assert '"$MKFIFO" -m 0600 -- "$DRIVER_STDOUT_PIPE" "$DRIVER_STDERR_PIPE"' in source
assert source.count(
    '--bounding-set=-all -- "$PYTHON" -I -S -c "$DRIVER_CAPTURE_PROGRAM"'
) == 2
assert 'stdout "$DRIVER_OUTPUT_RETAIN_LIMIT" <"$DRIVER_STDOUT_PIPE" &' in source
assert 'stderr "$DRIVER_OUTPUT_RETAIN_LIMIT" <"$DRIVER_STDERR_PIPE" &' in source
assert source.count("count=1 iflag=fullblock status=none &") == 2
assert 'readonly DRIVER_STDOUT=' not in source
assert 'readonly DRIVER_STDERR=' not in source
assert '[[ "$driver_group" == "$driver_pid" ]]' in source
assert "setsid did not create the driver-owned session" in source
assert "journey descendant retained a bounded output pipe after driver exit" in source
assert 'quiesce_driver_group() {' in source
assert '/usr/bin/kill -TERM -- "-$group"' in source
assert '/usr/bin/kill -KILL -- "-$group"' in source
assert 'forward_bounded_driver_failure' not in source
assert 'payload_limit = output_limit - len(safe_prefix) - len(safe_suffix) - shell_trailer_budget - 1' in source
assert 'shell_trailer_budget = 256' in source
assert 'def write_all(fd, payload):' in source
assert 'chunk = os.read(0, 65_536)' in source
assert 'if total > payload_limit:' in source
assert 'data.decode("ascii", "strict")' in source
assert 'byte not in (0x09, 0x0A) and not 0x20 <= byte <= 0x7E' in source
assert 'forbidden_environment.search(data)' in source
assert 'forbidden_secret.search(data)' in source
assert 'public_url.search(data)' in source
assert 'run-four-node-journey: driver output exceeded its bound' in source
assert 'bounded driver stderr rejected by scan' in source
assert 'run-four-node-journey: bounded driver stderr follows' in source
assert 'run-four-node-journey: end bounded driver stderr' in source
assert 'raise SystemExit(0 if not data else 3)' in source
capture_prefix = "readonly DRIVER_CAPTURE_PROGRAM='"
capture_start = source.index(capture_prefix) + len(capture_prefix)
capture_end = source.index("'\nreadonly BOOTSTRAP_LOG_MAX_BYTES=", capture_start)
capture_program = source[capture_start:capture_end]
compile(capture_program, "DRIVER_CAPTURE_PROGRAM", "exec")

def scan(stream, payload):
    return subprocess.run(
        [sys.executable, "-I", "-S", "-c", capture_program, stream, "65536"],
        input=payload,
        capture_output=True,
        check=False,
    )

empty_stdout = scan("stdout", b"")
assert empty_stdout.returncode == 0 and not empty_stdout.stdout and not empty_stdout.stderr
unexpected_stdout = scan("stdout", b"unexpected\n")
assert unexpected_stdout.returncode == 3 and not unexpected_stdout.stdout and not unexpected_stdout.stderr
safe_stderr = scan("stderr", b"safe diagnostic\n")
assert safe_stderr.returncode == 3 and not safe_stderr.stdout
assert safe_stderr.stderr == (
    b"run-four-node-journey: bounded driver stderr follows\n"
    b"safe diagnostic\n"
    b"run-four-node-journey: end bounded driver stderr\n"
)
phase_marker = scan(
    "stderr",
    b"run-four-node-journey: last-phase=relay-census\n",
)
assert phase_marker.returncode == 3 and not phase_marker.stdout
assert b"last-phase=relay-census\n" in phase_marker.stderr
launcher_phase_marker = scan(
    "stderr",
    b"run-four-node-journey: launcher-phase=driver\n",
)
assert launcher_phase_marker.returncode == 3 and not launcher_phase_marker.stdout
assert b"launcher-phase=driver\n" in launcher_phase_marker.stderr
driver_phase_marker = scan(
    "stderr",
    b"run-four-node-journey: driver-phase=projection-snapshot-fresh-pair\n",
)
assert driver_phase_marker.returncode == 3 and not driver_phase_marker.stdout
assert b"driver-phase=projection-snapshot-fresh-pair\n" in driver_phase_marker.stderr
safe_prefix = b"run-four-node-journey: bounded driver stderr follows\n"
safe_suffix = b"run-four-node-journey: end bounded driver stderr\n"
maximum_payload = 65536 - len(safe_prefix) - len(safe_suffix) - 256 - 1
maximum_stderr = scan("stderr", b"x" * maximum_payload)
assert maximum_stderr.returncode == 3 and not maximum_stderr.stdout
stdout_overflow = b"run-four-node-journey: driver output exceeded its bound\n"
stdout_failure = b"run-four-node-journey: journey driver wrote unexpected stdout or its scanner failed\n"
assert len(maximum_stderr.stderr) + len(stdout_overflow) + len(stdout_failure) <= 65536
over_maximum = scan("stderr", b"x" * (maximum_payload + 1))
assert over_maximum.returncode == 2
sensitive_keys = (
    b"credential",
    b"credentials",
    b"mnemonic",
    b"passphrase",
    b"password",
    b"private_key",
    b"private_locator",
    b"prompt",
    b"provider_secret",
    b"secret",
    b"seed",
    b"source_body",
    b"token",
    b"transcript",
)
unsafe_payloads = [
    payload
    for key in sensitive_keys
    for payload in (key.upper() + b"=must-not-leak\n", b'{"' + key + b'":"must-not-leak"}\n')
]
unsafe_payloads.extend((
    b"https://public.example.invalid/path\n",
    b"carriage\rreturn\n",
    b"non-ascii=\xc2\x9b\n",
    b"x" * 65537,
))
for unsafe in unsafe_payloads:
    rejected = scan("stderr", unsafe)
    assert rejected.returncode == 2 and not rejected.stdout
    assert b"must-not-leak" not in rejected.stderr
    assert rejected.stderr in (
        b"run-four-node-journey: bounded driver stderr rejected by scan\n",
        b"run-four-node-journey: driver output exceeded its bound\n",
    )
for safe in (
    b"password_hash=safe\n",
    b"notpassword=safe\n",
    b"transcript_sha256=safe\n",
    b'zero_count_keys:["secret"]\n',
):
    accepted = scan("stderr", safe)
    assert accepted.returncode == 3 and not accepted.stdout
    assert safe in accepted.stderr
assert source.index('quiesce_driver_group "$driver_group"', driver_end) < source.index(
    "die 'four-node journey driver failed'", driver_end
)
assert 'if ! process_is_live "${bootstrap_reader_pid:-}" &&' in source
assert 'if process_is_live "$reader_pid"; then' in source
assert "preflight|toolchain-materialization|genesis-export|driver|post-driver" in source
assert "printf 'run-four-node-journey: launcher-phase=%s\\n'" in source
assert 'readonly DRIVER_PHASE_PATH="$SESSION_ROOT/.driver.phase"' in source
assert 'readonly DRIVER_PHASE_TEMP="$SESSION_ROOT/.driver.phase.tmp"' in source
assert "printf 'run-four-node-journey: driver-phase=%s\\n'" in source
assert '"$EUID:600:1:regular file"' in source
phase_check = source.index("driver did not publish the exact complete phase marker", driver_end)
phase_remove = source.index('/usr/bin/gnurm -f -- "$DRIVER_PHASE_PATH"', phase_check)
success_flag = source.index("journey_succeeded=1", phase_remove)
assert phase_check < phase_remove < success_flag
assert '[[ $stdout_reader_status -eq 0 ]] ||' in source
assert '$stderr_reader_status -eq 0 || $stderr_reader_status -eq 2 ||' in source
assert 'process_group_has_members' not in source
assert 'process_group_is_empty() {' in source
assert 'inventory="$(/usr/bin/ps -e -o pgid=)" || return 1' in source
assert 'if ! process_group_is_empty "$driver_group"; then' in source
assert 'journey driver exited with process-group members or an invalid census' in source
assert source.index("driver_group=''", driver_end) < source.index(
    "verify_read_only_toolchain after-driver", driver_end
)
assert '[[ $stdout_reader_status -eq 0 && $stderr_reader_status -eq 0 ]] ||' in source
assert 'journey driver capture pipe identity drifted' in source
assert '[[ ! -e "$WORK_ROOT" && ! -L "$WORK_ROOT" ]]' in source
assert '"$EUID:600:regular file"' in source
assert "journey_succeeded=1" in source
assert "readonly SUCCESS_LINE='verified cubikan four-node journey v1'" in source
assert "printf '%s\\n' \"$SUCCESS_LINE\"" in source
assert 'process.env.CUBIKAN_POLKADOT_PARACHAIN ?? ""' in driver_source
assert "CUBIKAN_PARACHAIN_BINARY" not in driver_source
assert 'path.join(bootstrapRoot, "materializer.log")' in driver_source
assert 'path.join(bootstrapRoot, "genesis-export.log")' in driver_source
assert 'const exactExecLines = (bytes, label) =>' in driver_source
assert 'bytes.length > 0 && bytes.at(-1) === 0x0a' in driver_source
assert '.filter((line) => line.startsWith("exec: "))' in driver_source
assert 'materializerExecLines.length === 1' in driver_source
assert 'genesisExecLines.length === expectedGenesisExecLines.length' in driver_source
assert 'bytes_hex: `0x${materializerLogBeforeRead.bytes.toString("hex")}`' in driver_source
assert 'bytes_hex: `0x${genesisExportLogBeforeRead.bytes.toString("hex")}`' in driver_source
assert 'async function procSeals(pid, executableHandle, executableIdentity)' in driver_source
assert 'fcntl.F_GET_SEALS' in driver_source
assert 'procPath(pid, "exe")' in driver_source
assert 'probe.stdout.equals(Buffer.from("seal-mask=0f write=denied\\n"))' in driver_source
assert 'procSeals(pid, exeHandle, exeBefore)' in driver_source
assert 'has no retained sealed executable descriptor' not in driver_source
assert 'const expectedNamespaceInitArgv = [' in driver_source
assert '"/usr/bin/bash",' in driver_source
assert 'async function stableProcessAncestry()' in driver_source
assert 'ancestors.size < 64' in driver_source
assert 'ancestors.has(1)' in driver_source
assert 'return { self, generations: [...ancestors.entries()] };' in driver_source
assert 'const namespaceInitGeneration = ancestors.get(1);' in driver_source
assert 'PID 1 was not the exact stable namespace init' in driver_source
assert 'if (error?.code !== "EACCES") throw error;' in driver_source
assert 'const expectedAncestor = ancestors.get(pid);' in driver_source
assert 'expectedAncestor !== undefined' in driver_source
assert '!nodeShaped' in driver_source
assert 'before.parent_pid === expectedAncestor.parent_pid' in driver_source
assert 'before.process_group === expectedAncestor.process_group' in driver_source
assert 'before.session_id === expectedAncestor.session_id' in driver_source
assert 'before.start_time_ticks === expectedAncestor.start_time_ticks' in driver_source
assert 'const argvAfterBytes = await fsp.readFile(procPath(pid, "cmdline"));' in driver_source
assert 'const commAfter = (await fsp.readFile(procPath(pid, "comm"), "utf8")).trim();' in driver_source
assert 'argvBytes.equals(argvAfterBytes)' in driver_source
assert 'comm === commAfter' in driver_source
assert 'nodeAssets.has(argv[0])' in driver_source
assert 'comm === "memfd:cubikan-s"' in driver_source
assert 'before.parent_pid === 0' in driver_source
assert 'before.process_group === 0' in driver_source
assert 'before.session_id === 0' in driver_source
assert 'before.start_time_ticks === after.start_time_ticks' in driver_source
assert 'comm === "bash"' in driver_source
assert 'JSON.stringify(argv) === JSON.stringify(expectedNamespaceInitArgv)' in driver_source
assert 'inaccessible process ${pid} was not an exact stable non-node harness ancestor' in driver_source
assert 'JSON.stringify(ancestryBefore) === JSON.stringify(ancestryAfter)' in driver_source
assert 'driver process ancestry changed during /proc inventory' in driver_source
assert 'let observedGeneration = null;' in driver_source
assert 'live process ${pid} became uninspectable during /proc inventory' in driver_source
assert 'const workerComms = new Set(workerFixture.assets.map((asset) => asset.name.slice(0, 15)));' in driver_source
assert 'const workerSubcommands = new Set(["prepare-worker", "execute-worker"]);' in driver_source
assert 'const workerShaped =' in driver_source
assert 'if (!workerShaped) continue;' in driver_source
assert 'PVF-shaped process ${pid} used an unknown executable' in driver_source
assert 'async function procStatWithState(pid) {' in driver_source
assert 'async function processGenerationExited(pid, observedGeneration) {' in driver_source
assert 'async function processGenerationExitedWithin(pid, observedGeneration) {' in driver_source
assert driver_source.count('await processGenerationExitedWithin(pid, before)') == 3
assert 'for (let retry = 0; retry < 8; retry += 1)' in driver_source
assert 'before.state !== "Z"' in driver_source
assert 'classificationAfter.state === "Z"' in driver_source
assert 'argvBefore.length === 0' in driver_source
assert 'argvAfter.length === 0' in driver_source
assert 'argv[0] === executable' in driver_source
assert 'comm === workerName.slice(0, 15)' in driver_source
assert 'argv[1] === expectedSubcommand' in driver_source
assert 'proc_comm: comm' in driver_source
assert 'async function processPrivilegeStatus(pid) {' in driver_source
assert 'async function pvfProcessSecurityEvidence(pid, workerFixture) {' in driver_source
assert 'workerFixture.nested_namespace_capability_mask' in driver_source
assert 'readBoundedSpecialFile(procPath(pid, "uid_map"), 4096)' in driver_source
assert 'readBoundedSpecialFile(procPath(pid, "gid_map"), 4096)' in driver_source
assert 'userNamespaceBefore !== orchestratorUserNamespaceBefore' in driver_source
assert 'mountNamespaceBefore !== orchestratorMountNamespaceBefore' in driver_source
assert 'uidMapBefore.length === 0' in driver_source
assert 'gidMapBefore.length === 0' in driver_source
assert '"nested-unmapped-user-mount"' in driver_source
assert 'const security = await pvfProcessSecurityEvidence(pid, workerFixture);' in driver_source
assert 'security,' in driver_source
assert 'const finalStat = await procStatWithState(pid);' in driver_source
assert 'sameProcessGeneration(before, finalStat)' in driver_source
assert 'for (let retry = 0; !classificationStable && retry < 8; retry += 1)' in driver_source
assert 'await new Promise((resolve) => setTimeout(resolve, 1));' in driver_source
assert 'function isCompleteProcVector(bytes) {' in driver_source
assert driver_source.count('isCompleteProcVector(argvBytes)') >= 1
assert driver_source.count('isCompleteProcVector(classificationArgvAfter)') == 1
assert driver_source.count('isCompleteProcVector(retryArgv)') == 1
assert driver_source.count('isCompleteProcVector(retryArgvAfter)') == 1
assert 'observedGeneration !== null &&' in driver_source
assert '(await processGenerationExited(pid, observedGeneration))' in driver_source
assert 'continue pid_loop;' in driver_source
assert 'process ${pid} did not reach a stable PVF worker classification' in driver_source
assert 'argvBytes.equals(argvAfterBytes)' in driver_source
assert 'environmentBytes.equals(environmentAfterBytes)' in driver_source
assert 'executable === executableAfter' in driver_source
assert 'live process ${pid} became uninspectable during PVF worker inventory' in driver_source
assert 'knownReparentedGenerations = null' in driver_source
assert 'if (parentWorker === undefined) {' in driver_source
assert 'ancestor.parent_pid === 1 ||' in driver_source
assert 'knownAncestorGeneration?.parent_pid === ancestor.parent_pid' in driver_source
assert 'PVF worker ${raw.pid} has an unowned parent' in driver_source
assert 'canonicalBytes(knownComparable).equals(canonicalBytes(currentComparable))' in driver_source
assert '!canonicalBytes(knownComparable).equals(canonicalBytes(currentComparable))' not in driver_source
assert 'PVF worker ${raw.pid} did not match its exact retained generation after reparenting' in driver_source
assert 'parentRoles.set(1' not in driver_source
assert '/unowned parent/' not in driver_source
monitor_start = driver_source.index("async function createPvfRuntimeMonitor(")
monitor_end = driver_source.index("\nasync function waitFor(", monitor_start)
monitor_source = driver_source[monitor_start:monitor_end]
monitor_stop = monitor_source.index("stopped = true;")
monitor_drain = monitor_source.index("await inFlight;", monitor_stop)
monitor_failure = monitor_source.index("if (failure) throw failure;", monitor_drain)
monitor_final_live_sample = monitor_source.index("await sample(true);", monitor_failure)
assert monitor_stop < monitor_drain < monitor_failure < monitor_final_live_sample
assert 'knownReparentedGenerations.size === generations.size' in monitor_source
assert '"retained PVF generation identities collided"' in monitor_source
finalizer_start = monitor_source.index("async stopAndEvidence()")
finalizer_source = monitor_source[finalizer_start:]
assert '"PVF process/path/socket teardown"' in finalizer_source
assert "30_000," in finalizer_source
assert "workerFixture.monitor_interval_milliseconds," in finalizer_source
assert finalizer_source.count("postStop.processes.length === 0") == 2
assert finalizer_source.count("postStop.paths.length === 0") == 2
assert finalizer_source.count("postStop.sockets.length === 0") == 2
assert '"PVF process/path/socket teardown did not reach an exact empty inventory"' in finalizer_source
assert '(?:git\\+ssh|https?|wss?|ssh|git):\\/\\/' in driver_source
assert 'path.join(auditRoot, "log/materializer.log")' in driver_source
assert 'path.join(auditRoot, "log/genesis-export.log")' in driver_source
assert 'let failureResponseClass = null;' in driver_source
assert 'error_code:' in driver_source and 'error_field:' in driver_source
assert 'stdout_sha256=${sha256(result.stdout)}' in driver_source
assert 'stderr_sha256=${sha256(result.stderr)}' in driver_source
assert 'stderr=${result.stderr}' not in driver_source
assert 'const SENSITIVE_PAYLOAD_KEYS = [' in driver_source
assert 'SENSITIVE_PAYLOAD_KEY_ALTERNATION' in driver_source
assert 'new RegExp(' in driver_source
assert 'spec.raw_size === relaySpecs[0].raw_size' in driver_source
assert 'spec.raw_sha256 === relaySpecs[0].raw_sha256' in driver_source
assert 'spec.raw_size === parachainSpecs[0].raw_size' in driver_source
assert 'spec.raw_sha256 === parachainSpecs[0].raw_sha256' in driver_source
assert 'spec.top_level_bootnodes.length === 0' in driver_source
assert "function installTerminationPhaseMarker(filePath)" in driver_source
assert 'fs.constants.O_EXCL' in driver_source
assert 'fs.constants.O_NOFOLLOW' in driver_source
assert 'fs.fsyncSync(descriptor);' in driver_source
assert 'fs.renameSync(temporaryPath, filePath);' in driver_source
assert 'fs.fsyncSync(directory);' in driver_source
assert 'marker.nlink === 1n' in driver_source
assert 'marker.uid === BigInt(process.geteuid())' in driver_source
assert 'marker.size <= 64n' in driver_source
assert 'path.join(path.dirname(workRoot), ".driver.phase")' in driver_source
assert 'terminationPhase.set("projection-snapshot-uninterrupted")' in driver_source
assert 'terminationPhase.set("projection-snapshot-fresh-pair")' in driver_source
assert 'terminationPhase.set("projection-snapshot-rebuild-a")' in driver_source
assert 'terminationPhase.set("projection-snapshot-rebuild-b")' in driver_source
assert 'terminationPhase.set("relay-resume-api-reconnect")' in driver_source
assert 'terminationPhase.set("relay-resume-convergence")' in driver_source
assert 'terminationPhase.set("relay-census")' in driver_source
assert 'terminationPhase.set("complete")' in driver_source
assert 'terminationPhase.set(`initial-submission-${mutation.id}`)' in driver_source
assert 'terminationPhase.set(`survivor-submission-${mutation.id}`)' in driver_source
assert "initial-submission-m0[1-7]" in source
assert "survivor-submission-m2[0-1]" in source

read_start = driver_source.index("async function readSnapshot(")
read_end = driver_source.index("\nasync function primeSnapshotDatabase(", read_start)
read_snapshot = driver_source[read_start:read_end]
authorize = read_snapshot.index("sourceGate.authorize(")
database_open = read_snapshot.index("await fsp.lstat(database)")
serial_primer = read_snapshot.index("outputs[0] = await runSnapshotPage({")
batch_start = read_snapshot.index("for (let start = 1;")
batch_settle = read_snapshot.index("const settlements = await Promise.allSettled(", batch_start)
ordered_aggregate = read_snapshot.index("outputs[start + offset] = settlement.value", batch_settle)
evidence_aggregate = read_snapshot.index("for (let index = 0;", ordered_aggregate)
assert authorize < database_open < serial_primer < batch_start < batch_settle
assert batch_settle < ordered_aggregate < evidence_aggregate
assert "const SNAPSHOT_READ_CONCURRENCY = 4;" in driver_source
assert 'fixture.reads[0]?.id === "get-primary"' in read_snapshot
assert "reads.slice(start, start + SNAPSHOT_READ_CONCURRENCY)" in read_snapshot

primer_start = driver_source.index("async function primeSnapshotDatabase(", read_end)
primer_end = driver_source.index("\nasync function replacementSnapshot(", primer_start)
primer = driver_source[primer_start:primer_end]
primer_authorize = primer.index("sourceGate.authorize(")
primer_database_open = primer.index("await fsp.lstat(database)")
primer_read = primer.index("const output = await runSnapshotPage({")
assert primer_authorize < primer_database_open < primer_read
assert primer.count("runSnapshotPage({") == 1
assert 'creationProbeErrno === "ENOENT"' in primer
assert 'fixture.reads[0]?.id === "get-primary"' in primer

replacement_start = driver_source.index("async function replacementSnapshot(", primer_end)
replacement_end = driver_source.index("\nasync function probeArchive(", replacement_start)
replacement = driver_source[replacement_start:replacement_end]
one_page_primer = replacement.index("await primeSnapshotDatabase(")
predelete_open = replacement.index("await fsp.lstat(database", one_page_primer)
database_delete = replacement.index("await fsp.unlink(database)", predelete_open)
full_rebuild = replacement.index("await readSnapshot({", database_delete)
assert one_page_primer < predelete_open < database_delete < full_rebuild
assert replacement.count("await readSnapshot({") == 1
assert "primedCheckpoint.block_hash === rebuilt.checkpoint_hash" in replacement

main_start = driver_source.index("async function main()")
pre_stop_pvf_quiesce = driver_source.index(
    'await withTimeout("pre-stop PVF runtime monitor quiescence"', main_start
)
network_stop_phase = driver_source.index('terminationPhase.set("network-stop")', pre_stop_pvf_quiesce)
network_stop = driver_source.index("network.stop()", network_stop_phase)
post_stop_phase = driver_source.index(
    'terminationPhase.set("post-stop-verification")', network_stop
)
post_stop_pvf_evidence = driver_source.index(
    "pvfRuntimeMonitor.stopAndEvidence()", post_stop_phase
)
assert pre_stop_pvf_quiesce < network_stop_phase < network_stop
assert network_stop < post_stop_phase < post_stop_pvf_evidence
uninterrupted = driver_source.index("snapshotByLabel.uninterrupted = await readSnapshot({", main_start)
identity_a = driver_source.index("const identityA = await endpointIdentity(", uninterrupted)
identity_b = driver_source.index("const identityB = await endpointIdentity(", identity_a)
identity_equal = driver_source.index(
    "canonicalSha256(identityA) === canonicalSha256(identityB)", identity_b
)
gate_open = driver_source.index("sourceGate.open(identityA, identityB, archiveProbes);", identity_equal)
fresh_phase = driver_source.index('terminationPhase.set("projection-snapshot-fresh-pair")', gate_open)
fresh_parallel = driver_source.index("await orderedParallelOperations([", fresh_phase)
fresh_a = driver_source.index('contract: contracts["fresh-a"]', fresh_parallel)
fresh_b = driver_source.index('contract: contracts["fresh-b"]', fresh_a)
fresh_a_assign = driver_source.index('snapshotByLabel["fresh-a"] = freshA;', fresh_b)
fresh_b_assign = driver_source.index('snapshotByLabel["fresh-b"] = freshB;', fresh_a_assign)
rebuild_a = driver_source.index('snapshotByLabel["rebuild-a"] = await replacementSnapshot({', fresh_b_assign)
rebuild_b = driver_source.index('snapshotByLabel["rebuild-b"] = await replacementSnapshot({', rebuild_a)
assert uninterrupted < identity_a < identity_b < identity_equal < gate_open
assert gate_open < fresh_phase < fresh_parallel < fresh_a < fresh_b
assert fresh_b < fresh_a_assign < fresh_b_assign < rebuild_a < rebuild_b
parallel_start = driver_source.index("async function orderedParallelOperations(")
parallel_end = driver_source.index("\nasync function readSnapshot(", parallel_start)
parallel = driver_source[parallel_start:parallel_end]
assert "Promise.allSettled(operations.map((operation) => operation()))" in parallel
assert parallel.index('settlement.status === "rejected"') < parallel.index(
    "settlements.map((settlement) => settlement.value)"
)
assert "sourceGateTranscript.length === 8" in driver_source
assert "sourceUseCount === 3" in driver_source

log_start = driver_source.index("function installBoundedNodeLogCapture(")
log_end = driver_source.index("\nfunction captureProcessDiagnostics(", log_start)
log_capture = driver_source[log_start:log_end]
assert 'this.on("error", (error) =>' in log_capture
assert "if (captureError === null) captureError = error;" in log_capture
assert 'if (error?.code !== "ENOENT") throw error;' in log_capture
assert "allowFailureCleanupUnlink && descriptor.nlink === 0n" in log_capture
assert "async quiesce()" in log_capture
assert "activeStreams.size === 0" in log_capture
assert "captureError === null" in log_capture
success_quiesce = driver_source.index('await withTimeout("bounded node log quiescence"')
success_health = driver_source.index("boundedNodeLogs.assertHealthy()", success_quiesce)
success_evidence = driver_source.index("boundedNodeLogs.evidence()", success_health)
assert success_quiesce < success_health < success_evidence
failure_quiesce = driver_source.index('await withTimeout("failure cleanup node log quiescence"')
failure_unlink_gate = driver_source.index("boundedNodeLogs.allowFailureCleanupUnlink()", failure_quiesce)
failure_remove = driver_source.index("await fsp.rm(workRoot", failure_unlink_gate)
assert failure_quiesce < failure_unlink_gate < failure_remove
assert "} catch {}" in driver_source[failure_remove:failure_remove + 120]

assert 'networkNodes["collator-b"].connectApi()' not in driver_source
assert "async function connectRestartPolkadotApi(" in driver_source
assert 'networkNodes["collator-b"].apiInstance = apiB;' in driver_source
assert 'terminationPhase.set("collator-restart-api-reconnect")' in driver_source
assert 'terminationPhase.set("collator-restart-primary-listener")' in driver_source
assert 'terminationPhase.set("collator-restart-relay-api")' in driver_source
assert 'terminationPhase.set("collator-restart-convergence")' in driver_source
assert 'terminationPhase.set("collator-restart-evidence")' in driver_source
assert "restart.closed.then((exit)" in driver_source
assert 'const mismatches = [' in driver_source
assert '["signer", signer !== expectedSigner]' in driver_source
assert '["payload", canonicalSha256(decoded.payload) !== canonicalSha256(expectedPayload)]' in driver_source
assert '["effect", canonicalSha256(decoded.effect) !== canonicalSha256(submission.response.effect)]' in driver_source
assert 'mismatches.length === 0' in driver_source
assert "function exactCallIndexHex(callIndex)" in driver_source
assert 'callIndex instanceof Uint8Array' in driver_source
assert 'bytes.length === 2' in driver_source
assert 'call_index: exactCallIndexHex(extrinsic.method.callIndex)' in driver_source
assert 'extrinsic.method.callIndex.toHex()' not in driver_source
assert 'function exactSignerAccountHex(signer)' in driver_source
assert 'signer?.type === "Id"' in driver_source
assert 'typeof signer.value?.toHex === "function"' in driver_source
assert '/^0x[0-9a-f]{64}$/.test(account)' in driver_source
assert 'signer: extrinsic.isSigned ? exactSignerAccountHex(extrinsic.signer) : null' in driver_source
assert 'extrinsic.signer.toHex()' not in driver_source
assert 'function eventDataEvidence(event)' in driver_source
assert 'key === "balances.Withdraw" || key === "transactionPayment.TransactionFeePaid"' in driver_source
assert 'typeof event.data[0]?.toHex === "function"' in driver_source
assert 'data[0] = payer;' in driver_source
assert 'const data = eventDataEvidence(record.event);' in driver_source
dispatch_start = driver_source.index('const DISPATCH_METHOD_BY_OPERATION = Object.freeze({')
dispatch_end = driver_source.index('\n});', dispatch_start) + len('\n});')
dispatch_source = driver_source[dispatch_start:dispatch_end]
dispatch_entries = re.findall(r'^  ([a-z_]+): "([A-Za-z]+)",$', dispatch_source, re.MULTILINE)
expected_dispatch_entries = {
    "create_intent_unit": "createUnit",
    "transition_intent_unit": "transitionUnit",
    "complete_intent_unit": "completeUnit",
    "create_relationship_definition": "createRelationshipDefinition",
    "create_relationship": "createRelationship",
    "delete_relationship": "deleteRelationship",
    "record_association": "recordAssociation",
    "revoke_association": "revokeAssociation",
}
assert len(dispatch_entries) == len(expected_dispatch_entries) == 8
assert dict(dispatch_entries) == expected_dispatch_entries
assert "replace_authorized_submitters" not in dispatch_source
assert "replaceAuthorizedSubmitters" not in dispatch_source
assert 'extrinsic.method !== exactDispatchMethod(expected.operation)' in driver_source
assert '["section", extrinsic.section !== "cubikan"]' in driver_source
assert 'snakeToLowerCamel' not in driver_source
assert '"balances.BurnedDebt"' in driver_source
assert 'const feeEventKeys = [' in driver_source
assert 'const feeCountsByExtrinsic = Array.from(' in driver_source
assert 'successByExtrinsic[event.phase.extrinsic_index] += 1;' in driver_source
assert 'withdrawAmount === burnedDebtAmount' in driver_source
assert 'withdrawAmount === actualFee' in driver_source
assert 'TransactionFeePaid payer or zero tip drifted' in driver_source
assert 'event_counts: eventCounts' in driver_source
assert 'const aggregateMismatches = aggregateChecks' in driver_source
assert 'forbidden_event_histogram:' in driver_source
assert '.slice(0, 4096)' in driver_source
listener_phase = driver_source.index('terminationPhase.set("collator-restart-primary-listener")')
listener_probe = driver_source.index("socketRefused(restartedPrimaryAddress)", listener_phase)
api_phase = driver_source.index('terminationPhase.set("collator-restart-api-reconnect")', listener_probe)
assert listener_phase < listener_probe < api_phase
assert 'async function finalizedRelayChainCensus(' in driver_source
assert 'const MAX_AUDIT_ARTIFACT_BYTES = 4_194_304;' in driver_source
assert 'const MAX_RAW_CHAIN_SPEC_BYTES = 8_388_608;' in driver_source
assert 'const MAX_AUDIT_TOTAL_BYTES = 25_165_824;' in driver_source
assert 'openedFileEvidence(specPath, MAX_RAW_CHAIN_SPEC_BYTES)' in driver_source
assert r'/^config\/(?:collator|relay)-[ab]\.raw\.json$/.test(logicalPath)' in driver_source
assert '? MAX_RAW_CHAIN_SPEC_BYTES' in driver_source
assert 'maximumBytes <= MAX_RAW_CHAIN_SPEC_BYTES' in driver_source
assert 'const ARCHIVE_PROBE_BLOCK_CONCURRENCY = 4;' in driver_source
assert 'const CENSUS_BLOCK_CONCURRENCY = 4;' in driver_source
assert 'const FINALIZED_OBSERVATION_CONCURRENCY = 4;' in driver_source
assert 'async function probeArchiveBlock(apiA, apiB, fixture, blockNumber)' in driver_source
assert 'batchStart += ARCHIVE_PROBE_BLOCK_CONCURRENCY' in driver_source
assert 'probeArchiveBlock(apiA, apiB, fixture, batchStart + offset)' in driver_source
assert 'for (const rows of rowsByBlock) transcript.push(...rows);' in driver_source
assert 'batchStart += FINALIZED_OBSERVATION_CONCURRENCY' in driver_source
assert 'submissions[batchStart + offset].endpoint_observations = observations[offset];' in driver_source
assert 'async function endpointChainCensusBlock(api, endpoint, blockNumber)' in driver_source
assert 'batchStart += CENSUS_BLOCK_CONCURRENCY' in driver_source
assert 'const rows = await orderedParallelOperations(' in driver_source
assert 'endpointChainCensusBlock(api, endpoint, batchStart + offset)' in driver_source
assert 'function attestEqualEndpointCensus(endpointA, endpointB, finalCheckpoint)' in driver_source
assert 'const transcriptA = canonicalBytes(endpointA.blocks);' in driver_source
assert 'const transcriptB = canonicalBytes(endpointB.blocks);' in driver_source
assert 'transcriptA.equals(transcriptB)' in driver_source
assert 'endpointRetainedLimit = fixture.audit.chain_census.maximum_retained_bytes' in driver_source
assert 'endpointRetainedLimit = censusFixture.maximum_retained_bytes' in driver_source
assert 'const endpointBAttestation = attestEqualEndpointCensus(' in driver_source
assert 'block_count: endpointB.blocks.length' in driver_source
assert 'endpoint_b: endpointBAttestation' in driver_source
assert 'endpoint_b: endpointB,' not in driver_source
para_census_start = driver_source.index("async function finalizedChainCensus(")
relay_census_start = driver_source.index("async function finalizedRelayChainCensus(")
para_census = driver_source[para_census_start:relay_census_start]
para_fetch_a = para_census.index('endpointChainCensus(apiA, "collator-a"')
para_fetch_b = para_census.index('endpointChainCensus(apiB, "collator-b"', para_fetch_a)
para_attest = para_census.index("attestEqualEndpointCensus(", para_fetch_b)
para_compact = para_census.index("endpoint_b: endpointBAttestation", para_attest)
assert para_fetch_a < para_fetch_b < para_attest < para_compact
relay_census_end = driver_source.index("\nasync function runSnapshotPage(", relay_census_start)
relay_census_source = driver_source[relay_census_start:relay_census_end]
relay_fetch_a = relay_census_source.index('endpointChainCensus(apiA, "relay-a"')
relay_fetch_b = relay_census_source.index('endpointChainCensus(apiB, "relay-b"', relay_fetch_a)
relay_attest = relay_census_source.index("attestEqualEndpointCensus(", relay_fetch_b)
relay_compact = relay_census_source.index("endpoint_b: endpointBAttestation", relay_attest)
assert relay_fetch_a < relay_fetch_b < relay_attest < relay_compact
assert "const relayAggregateChecks = [" in relay_census_source
assert "const relayAggregateMismatches = relayAggregateChecks" in relay_census_source
assert "const forbiddenRelayEventHistogram = new Map();" in relay_census_source
assert "external_action_examples: externalActions.slice(0, 8)" in relay_census_source
assert "forbidden_event_examples: forbiddenEvents.slice(0, 8)" in relay_census_source
assert "candidate_para_id_examples: candidateParaIds.slice(0, 8)" in relay_census_source
assert "event_para_id_examples: eventParaIds.slice(0, 8)" in relay_census_source
assert "allowed_event_counts: eventCounts" in relay_census_source
assert "backed_candidate_count: observation.backed_candidate_count" in relay_census_source
assert "upward_signal_count: observation.upward_signal_count" in relay_census_source
assert 'key === "onDemandAssignmentProvider.SpotPriceSet"' in relay_census_source
assert "const expectedInitialSpotPrice = censusFixture.expected_initial_spot_price" in relay_census_source
assert "expectedInitialSpotPrice === 10_000_000" in relay_census_source
assert "block.number !== 1" in relay_census_source
assert "event.index !== 0" in relay_census_source
assert 'event.phase.kind !== "initialization"' in relay_census_source
assert "event.phase.extrinsic_index !== null" in relay_census_source
assert "event.topics.length !== 0" in relay_census_source
assert "spotPrice !== expectedInitialSpotPrice" in relay_census_source
assert "initialSpotPrice !== null" in relay_census_source
assert 'eventCounts["onDemandAssignmentProvider.SpotPriceSet"]' in relay_census_source
assert "initial_spot_price: initialSpotPrice" in relay_census_source
assert "expected_initial_spot_price: expectedInitialSpotPrice" in relay_census_source
assert 'key === "historical.RootStored"' in relay_census_source
assert 'key === "grandpa.NewAuthorities"' in relay_census_source
assert 'event.phase.kind !== "initialization"' in relay_census_source
assert 'event.phase.kind !== "finalization"' in relay_census_source
assert "canonicalSha256(event.data[0]) !== canonicalSha256(expectedGrandpaAuthorities)" in relay_census_source
assert "historicalRootSessionIndices.length === newSessionIndices.length" in relay_census_source
assert "sessionIndex === newSessionIndices[index] + 1" in relay_census_source
assert "grandpaNewAuthoritiesCount" in relay_census_source
assert "expectedGrandpaNewAuthoritiesCount" in relay_census_source
assert "minimum_session_rotation_count: censusFixture.minimum_session_rotation_count" in relay_census_source
assert "expected_grandpa_authorities: expectedGrandpaAuthorities" in relay_census_source
assert 'if (key === "grandpa.NewAuthorities") {' in driver_source
assert 'if (key === "onDemandAssignmentProvider.SpotPriceSet") {' in driver_source
assert '"onDemandAssignmentProvider.SpotPriceSet data is not one typed price"' in driver_source
assert "authority[0].toHex()" in driver_source
assert "authorityWeight === 1" in driver_source
assert '.slice(0, 16)' in relay_census_source
assert '.slice(0, 4096)' in relay_census_source
assert "relayAggregateMismatches.length === 0" in relay_census_source
assert 'function exactLowerHexByteLength(value, label)' in driver_source
assert '/^0x(?:[0-9a-f]{2})*$/.test(value)' in driver_source
assert 'separatorIndexes.length <= 1' in driver_source
assert 'observation.upward_message_count += separatorIndex' in driver_source
assert 'observation.upward_signal_separator_count += separatorIndexes.length' in driver_source
assert 'observation.upward_signal_count += signalCount' in driver_source
assert '"upward_signal_separators"' in relay_census_source
relay_api_a_declaration = driver_source.index(
    'let relayApiA = networkNodes["relay-a"].apiInstance;', main_start
)
relay_api_b_declaration = driver_source.index(
    'let relayApiB = networkNodes["relay-b"].apiInstance;', relay_api_a_declaration
)
relay_disconnect_a = driver_source.index(
    'withTimeout("pre-pause relay A API disconnect"', relay_api_b_declaration
)
relay_disconnect_a_call = driver_source.index("relayApiA.disconnect()", relay_disconnect_a)
relay_disconnect_b = driver_source.index(
    'withTimeout("pre-pause relay B API disconnect"', relay_disconnect_a_call
)
relay_disconnect_b_call = driver_source.index("relayApiB.disconnect()", relay_disconnect_b)
relay_clear_node_a = driver_source.index(
    'networkNodes["relay-a"].apiInstance = undefined;', relay_disconnect_b_call
)
relay_clear_node_b = driver_source.index(
    'networkNodes["relay-b"].apiInstance = undefined;', relay_clear_node_a
)
relay_clear_local_a = driver_source.index("relayApiA = null;", relay_clear_node_b)
relay_clear_local_b = driver_source.index("relayApiB = null;", relay_clear_local_a)
relay_pause = driver_source.index('networkNodes["relay-b"].pause()', relay_clear_local_b)
relay_resume = driver_source.index('networkNodes["relay-b"].resume()', relay_pause)
relay_reconnect_phase = driver_source.index(
    'terminationPhase.set("relay-resume-api-reconnect")', relay_resume
)
relay_endpoint_a = driver_source.index(
    'const relayEndpointA = "ws://127.0.0.1:9944/";', relay_reconnect_phase
)
relay_endpoint_b = driver_source.index(
    'const relayEndpointB = "ws://127.0.0.1:9945/";', relay_endpoint_a
)
relay_endpoint_gate = driver_source.index(
    'new URL(networkNodes["relay-a"].wsUri).href === relayEndpointA', relay_endpoint_b
)
relay_all_settled = driver_source.index("await Promise.allSettled([", relay_endpoint_gate)
relay_connect_a = driver_source.index("connectPolkadotApi(", relay_all_settled)
relay_assign_a = driver_source.index("relayApiA = api;", relay_connect_a)
relay_publish_a = driver_source.index(
    'networkNodes["relay-a"].apiInstance = api;', relay_assign_a
)
relay_connect_b = driver_source.index("connectPolkadotApi(", relay_publish_a)
relay_assign_b = driver_source.index("relayApiB = api;", relay_connect_b)
relay_publish_b = driver_source.index(
    'networkNodes["relay-b"].apiInstance = api;', relay_assign_b
)
relay_reconnect_failure = driver_source.index(
    "if (relayReconnectFailure) throw relayReconnectFailure.reason;", relay_publish_b
)
relay_genesis_a = driver_source.index("rpcGenesisHash(relayApiA)", relay_reconnect_failure)
relay_genesis_b = driver_source.index("rpcGenesisHash(relayApiB)", relay_genesis_a)
relay_genesis_gate = driver_source.index(
    'relayGenesisA === initialNodes["relay-a"].primary_chain_spec.live_genesis_hash',
    relay_genesis_b,
)
relay_convergence_phase = driver_source.index(
    'terminationPhase.set("relay-resume-convergence")', relay_genesis_gate
)
relay_converge = driver_source.index('"resumed relay B validator catch-up"', relay_convergence_phase)
relay_census = driver_source.index("await finalizedRelayChainCensus(", relay_converge)
relay_disconnect = driver_source.index("relaySideApiA.disconnect()", relay_census)
assert relay_api_a_declaration < relay_api_b_declaration < relay_disconnect_a
assert relay_disconnect_a < relay_disconnect_a_call < relay_disconnect_b < relay_disconnect_b_call
assert relay_disconnect_b_call < relay_clear_node_a < relay_clear_node_b
assert relay_clear_node_b < relay_clear_local_a < relay_clear_local_b < relay_pause < relay_resume
assert relay_resume < relay_reconnect_phase < relay_endpoint_a < relay_endpoint_b < relay_endpoint_gate
assert relay_endpoint_gate < relay_all_settled < relay_connect_a < relay_assign_a < relay_publish_a
assert relay_publish_a < relay_connect_b < relay_assign_b < relay_publish_b < relay_reconnect_failure
assert relay_reconnect_failure < relay_genesis_a < relay_genesis_b < relay_genesis_gate
assert relay_genesis_gate < relay_convergence_phase < relay_converge < relay_census < relay_disconnect
relay_reconnect_source = driver_source[relay_reconnect_phase:relay_convergence_phase]
relay_connect_a_source = driver_source[relay_connect_a:relay_connect_b]
relay_connect_b_source = driver_source[relay_connect_b:relay_reconnect_failure]
assert relay_reconnect_source.count("connectPolkadotApi(") == 2
assert "await Promise.allSettled([" in relay_reconnect_source
assert relay_reconnect_source.count('"ws://127.0.0.1:9944/"') == 1
assert relay_reconnect_source.count('"ws://127.0.0.1:9945/"') == 1
assert relay_reconnect_source.count('"resumed relay A API reconnect"') == 1
assert relay_reconnect_source.count('"resumed relay B API reconnect"') == 1
assert 'networkNodes["relay-a"].apiInstance = api;' in relay_reconnect_source
assert 'networkNodes["relay-b"].apiInstance = api;' in relay_reconnect_source
assert "relayEndpointA," in relay_connect_a_source and "relayEndpointB," not in relay_connect_a_source
assert "relayEndpointB," in relay_connect_b_source and "relayEndpointA," not in relay_connect_b_source
assert "isConnected" not in relay_reconnect_source
assert 'relayGenesisB === initialNodes["relay-b"].primary_chain_spec.live_genesis_hash' in relay_reconnect_source
assert "relayGenesisA === relayGenesisB" in relay_reconnect_source
connect_api_start = driver_source.index("async function connectPolkadotApi(")
connect_api_end = driver_source.index("\nasync function connectRestartPolkadotApi(", connect_api_start)
connect_api_source = driver_source[connect_api_start:connect_api_end]
connect_provider = connect_api_source.index("new WsProvider(endpoint)")
connect_create = connect_api_source.index("ApiPromise.create({ provider })", connect_provider)
connect_ready = connect_api_source.index("await connected.isReady;", connect_create)
connect_transport_gate = connect_api_source.index("provider.endpoint === endpoint", connect_ready)
connect_return = connect_api_source.index("return api;", connect_transport_gate)
assert connect_provider < connect_create < connect_ready < connect_transport_gate < connect_return
assert "provider.isConnected && connected.isConnected" in connect_api_source
assert "if (api) await api.disconnect().catch(() => {});" in connect_api_source
assert "else await provider.disconnect().catch(() => {});" in connect_api_source
assert 'path.join(auditRoot, "action/finalized-relay-chain-census.json")' in driver_source
assert "...relayChainCensus.external_actions" in driver_source
assert "...relayChainCensus.forbidden_events" in driver_source

pvf_start = driver_source.index("async function pvfWorkerProcessInventory(")
pvf_end = driver_source.index("\nasync function pvfHostPathInventory(", pvf_start)
pvf_inventory = driver_source[pvf_start:pvf_end]
zombie_branch = pvf_inventory.index('if (classificationAfter.state === "Z")')
argv_split = pvf_inventory.index("const argv = splitNul(argvBytes);")
worker_shape = pvf_inventory.index("const workerShaped =")
shape_continue = pvf_inventory.index("if (!workerShaped) continue;")
proc_open = pvf_inventory.index('procHandle = await fsp.open(procPath(pid, ""), "r");')
exe_open = pvf_inventory.index('exeHandle = await fsp.open(procPath(pid, "exe"), "r");')
descriptor_after = pvf_inventory.index("const exeDescriptorAfter =")
security_capture = pvf_inventory.index(
    "const security = await pvfProcessSecurityEvidence(pid, workerFixture);"
)
final_stat = pvf_inventory.index("const finalStat = await procStatWithState(pid);")
assert zombie_branch < argv_split < worker_shape < shape_continue < proc_open < exe_open
assert proc_open < security_capture < descriptor_after < final_stat
assert "processPrivilegeEvidence(pid)" not in pvf_inventory
assert pvf_inventory.count("isCompleteProcVector(") == 7
environment_read = pvf_inventory.index(
    'const environmentBytes = await fsp.readFile(procPath(pid, "environ"));'
)
environment_complete = pvf_inventory.index(
    "if (!isCompleteProcVector(environmentBytes))", environment_read
)
environment_split = pvf_inventory.index("const environmentEntries = splitNul(environmentBytes);")
assert environment_read < environment_complete < environment_split

node_start = driver_source.index("async function nodeProcessInventory(")
node_end = driver_source.index("\nasync function pvfWorkerProcessInventory(", node_start)
node_inventory = driver_source[node_start:node_end]
node_complete = node_inventory.index("if (!isCompleteProcVector(argvBytes))")
node_split = node_inventory.index("const argv = splitNul(argvBytes);")
assert node_complete < node_split
assert "await processGenerationExitedWithin(pid, before)" in node_inventory
assert "await processGenerationExited(pid, observedGeneration)" in node_inventory
assert "await processGenerationLive(pid, observedGeneration.start_time_ticks)" not in node_inventory

assert "readonly T1115_OUTPUT_LIMIT=1048576" in loopback_source
assert "readonly T1115_OUTPUT_MEDIATOR_PROGRAM='import os" in loopback_source
assert 'stdout=subprocess.PIPE' in loopback_source
assert 'stderr=subprocess.PIPE' in loopback_source
assert 'close_fds=True' in loopback_source
assert 'start_new_session=True' in loopback_source
assert 'pidfd = os.pidfd_open(child.pid, 0)' in loopback_source
assert 'signal.pidfd_send_signal(pidfd, signum)' in loopback_source
assert 'stdout_payload = stdout_capture.accepted()' in loopback_source
assert 'stderr_payload = stderr_capture.accepted()' in loopback_source
assert loopback_source.index('stdout_payload = stdout_capture.accepted()') < loopback_source.index(
    'write_all(1, stdout_payload)'
)
assert '"$SETPRIV" --pdeathsig KILL --' in loopback_source
assert 'is_t1115_mediated_child child_argv' in loopback_source
assert 'prove_t1115_mediated_descriptors' in loopback_source
assert '[[ ! -e "$fd_path" && ! -L "$fd_path" ]]' in loopback_source
assert 'Globbing /proc/$$/fd can list the transient directory descriptor' in loopback_source
assert 'newinstance,nodev,nosuid,noexec,mode=0620,ptmxmode=0666' in loopback_source
assert 'controlling-terminal=absent' in loopback_source
mediator_prefix = "readonly T1115_OUTPUT_MEDIATOR_PROGRAM='"
mediator_start = loopback_source.index(mediator_prefix) + len(mediator_prefix)
mediator_end = loopback_source.index("'\n\nreadonly SELF=", mediator_start)
mediator_program = loopback_source[mediator_start:mediator_end]
compile(mediator_program, "T1115_OUTPUT_MEDIATOR_PROGRAM", "exec")

def mediate(stdout_limit, stderr_limit, body):
    return subprocess.run(
        [
            sys.executable,
            "-I",
            "-S",
            "-c",
            mediator_program,
            str(stdout_limit),
            str(stderr_limit),
            "--",
            "/usr/bin/bash",
            "--noprofile",
            "--norc",
            "-c",
            body,
        ],
        capture_output=True,
        check=False,
    )

mediated_safe = mediate(1024, 1024, "printf safe-out; printf safe-err >&2")
assert (mediated_safe.returncode, mediated_safe.stdout, mediated_safe.stderr) == (
    0,
    b"safe-out",
    b"safe-err",
)
mediated_failure = mediate(1024, 1024, "printf safe-out; printf safe-err >&2; exit 7")
assert (mediated_failure.returncode, mediated_failure.stdout, mediated_failure.stderr) == (
    7,
    b"safe-out",
    b"safe-err",
)
for unsafe_payload in unsafe_payloads[:-3]:
    unsafe_body = "printf safe-out; printf '" + unsafe_payload.decode("ascii").rstrip("\n") + "' >&2"
    mediated_unsafe = mediate(1024, 1024, unsafe_body)
    assert mediated_unsafe.returncode == 126 and not mediated_unsafe.stdout
    assert mediated_unsafe.stderr == b"loopback-netns: mediated child output rejected\n"
for safe_payload in (
    "password_hash=safe",
    "notpassword=safe",
    "transcript_sha256=safe",
    'zero_count_keys:["secret"]',
):
    mediated_safe_payload = mediate(
        1024,
        1024,
        "printf safe-out; printf '" + safe_payload + "' >&2",
    )
    assert mediated_safe_payload.returncode == 0
    assert mediated_safe_payload.stdout == b"safe-out"
    assert mediated_safe_payload.stderr == safe_payload.encode("ascii")
mediated_overflow = mediate(16, 16, "printf 12345678901234567")
assert mediated_overflow.returncode == 126 and not mediated_overflow.stdout
assert mediated_overflow.stderr == b"loopback-netns: mediated child output rejected\n"
PY
}

test_journey_driver_interface_is_closed() {
    local mutated="$TEST_ROOT/run-four-node-missing-success.sh"
    local log_mutated="$TEST_ROOT/run-four-node-missing-genesis-log-bound.sh"
    local toolchain_mutated="$TEST_ROOT/run-four-node-missing-toolchain-recheck.sh"
    local source_gate_mutated="$TEST_ROOT/driver-missing-source-gate.mjs"
    local identity_gate_mutated="$TEST_ROOT/driver-missing-identity-equality.mjs"
    local relay_reconnect_mutated="$TEST_ROOT/driver-missing-relay-resume-reconnect.mjs"
    local relay_cleanup_mutated="$TEST_ROOT/driver-missing-relay-resume-cleanup.mjs"
    local relay_readiness_mutated="$TEST_ROOT/driver-missing-relay-resume-readiness.mjs"
    local relay_endpoint_mutated="$TEST_ROOT/driver-wrong-relay-resume-endpoint.mjs"
    local relay_genesis_mutated="$TEST_ROOT/driver-aliased-relay-resume-genesis.mjs"
    local relay_diagnostic_mutated="$TEST_ROOT/driver-missing-relay-census-diagnostic-gate.mjs"
    local relay_historical_mutated="$TEST_ROOT/driver-missing-relay-historical-root.mjs"
    local relay_grandpa_mutated="$TEST_ROOT/driver-invalid-relay-grandpa-identity.mjs"
    local relay_spot_block_mutated="$TEST_ROOT/driver-invalid-relay-spot-price-block.mjs"
    local relay_spot_price_mutated="$TEST_ROOT/driver-inverted-relay-spot-price.mjs"
    local raw_spec_capture_mutated="$TEST_ROOT/driver-generic-raw-spec-capture-bound.mjs"
    local raw_spec_audit_mutated="$TEST_ROOT/driver-generic-raw-spec-audit-bound.mjs"
    local pvf_quiesce_mutated="$TEST_ROOT/driver-missing-pre-stop-pvf-quiesce.mjs"
    local pvf_parent_mutated="$TEST_ROOT/driver-open-pvf-reparent-boundary.mjs"
    local pvf_identity_mutated="$TEST_ROOT/driver-inverted-pvf-reparent-identity.mjs"
    local pvf_empty_mutated="$TEST_ROOT/driver-missing-pvf-socket-emptiness.mjs"
    validate_journey_driver_interface "$JOURNEY" ||
        die 'four-node journey interface is not closed'
    /usr/bin/sed "/^readonly SUCCESS_LINE=/d" "$JOURNEY" >"$mutated"
    if validate_journey_driver_interface "$mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted a missing success contract'
    fi
    /usr/bin/sed '/^finish_bootstrap_log_capture "\$GENESIS_LOG" /d' \
        "$JOURNEY" >"$log_mutated"
    if validate_journey_driver_interface "$log_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted a missing genesis-log bound'
    fi
    /usr/bin/sed '/^verify_read_only_toolchain after-driver$/d' \
        "$JOURNEY" >"$toolchain_mutated"
    if validate_journey_driver_interface "$toolchain_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted a missing toolchain post-check'
    fi
    /usr/bin/sed 's/sourceGate\.authorize(/sourceGate.bypass(/' \
        "$DRIVER" >"$source_gate_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$source_gate_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted a source open without authorization'
    fi
    /usr/bin/sed \
        's/canonicalSha256(identityA) === canonicalSha256(identityB)/canonicalSha256(identityA) !== canonicalSha256(identityB)/' \
        "$DRIVER" >"$identity_gate_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$identity_gate_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted a rebuild gate without identity equality'
    fi
    /usr/bin/sed 's/        relayApiB = api;/        relayApiB = null;/' \
        "$DRIVER" >"$relay_reconnect_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$relay_reconnect_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted a discarded fresh relay B API'
    fi
    /usr/bin/sed '/networkNodes\["relay-b"\]\.apiInstance = api;/d' \
        "$DRIVER" >"$relay_cleanup_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$relay_cleanup_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted an unowned fresh relay B API'
    fi
    /usr/bin/sed '/      await connected\.isReady;/d' \
        "$DRIVER" >"$relay_readiness_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$relay_readiness_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted relay reconnection without API readiness'
    fi
    /usr/bin/sed '0,/        relayEndpointB,$/s//        relayEndpointA,/' \
        "$DRIVER" >"$relay_endpoint_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$relay_endpoint_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted relay B reconnection to relay A'
    fi
    /usr/bin/sed '0,/      rpcGenesisHash(relayApiB),/s//      rpcGenesisHash(relayApiA),/' \
        "$DRIVER" >"$relay_genesis_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$relay_genesis_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted an aliased relay B genesis probe'
    fi
    /usr/bin/sed '/^    relayAggregateMismatches\.length === 0,$/d' \
        "$DRIVER" >"$relay_diagnostic_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$relay_diagnostic_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted a missing relay census diagnostic gate'
    fi
    /usr/bin/sed '0,/key === "historical.RootStored"/s//key === "historical.RootIgnored"/' \
        "$DRIVER" >"$relay_historical_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$relay_historical_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted a missing relay historical-root validator'
    fi
    /usr/bin/sed \
        '0,/canonicalSha256(event.data\[0\]) !== canonicalSha256(expectedGrandpaAuthorities)/s/!==/===/' \
        "$DRIVER" >"$relay_grandpa_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$relay_grandpa_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted inverted relay Grandpa authority identity'
    fi
    /usr/bin/sed 's/block\.number !== 1/block.number !== 2/' \
        "$DRIVER" >"$relay_spot_block_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$relay_spot_block_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted a relay spot-price event outside block one'
    fi
    /usr/bin/sed 's/spotPrice !== expectedInitialSpotPrice/spotPrice === expectedInitialSpotPrice/' \
        "$DRIVER" >"$relay_spot_price_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$relay_spot_price_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted an inverted relay initial spot price'
    fi
    /usr/bin/sed \
        's/openedFileEvidence(specPath, MAX_RAW_CHAIN_SPEC_BYTES)/openedFileEvidence(specPath, MAX_AUDIT_ARTIFACT_BYTES)/' \
        "$DRIVER" >"$raw_spec_capture_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$raw_spec_capture_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted the generic cap for live raw chain specs'
    fi
    /usr/bin/sed 's/? MAX_RAW_CHAIN_SPEC_BYTES/? MAX_AUDIT_ARTIFACT_BYTES/' \
        "$DRIVER" >"$raw_spec_audit_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$raw_spec_audit_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted the generic cap for audited raw chain specs'
    fi
    /usr/bin/sed \
        '/^    await withTimeout("pre-stop PVF runtime monitor quiescence"/,+2d' \
        "$DRIVER" >"$pvf_quiesce_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$pvf_quiesce_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted network teardown without PVF monitor quiescence'
    fi
    /usr/bin/sed 's/ancestor\.parent_pid === 1 ||/true ||/' \
        "$DRIVER" >"$pvf_parent_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$pvf_parent_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted an open PVF reparenting boundary'
    fi
    /usr/bin/sed \
        's/canonicalBytes(knownComparable)\.equals(canonicalBytes(currentComparable))/!canonicalBytes(knownComparable).equals(canonicalBytes(currentComparable))/' \
        "$DRIVER" >"$pvf_identity_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$pvf_identity_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted inverted PVF reparented-generation identity'
    fi
    /usr/bin/sed 's/postStop\.sockets\.length === 0/true/g' \
        "$DRIVER" >"$pvf_empty_mutated"
    if validate_journey_driver_interface "$JOURNEY" "$pvf_empty_mutated" >/dev/null 2>&1; then
        die 'journey interface oracle accepted PVF teardown without socket emptiness'
    fi
}

prepare_run_tree() {
    /usr/bin/mkdir -m 0700 -- "$RUN_DIR"
    local name
    for name in alice bob alice-1 bob-1; do
        /usr/bin/mkdir -m 0700 -- "$RUN_DIR/$name" "$RUN_DIR/$name/cfg" \
            "$RUN_DIR/$name/data" "$RUN_DIR/$name/relay-data"
    done
    printf '%s\n' '{"bootNodes":[],"genesis":{"raw":{"top":{}}},"id":"rococo_local_testnet"}' \
        >"$RUN_DIR/alice/cfg/rococo-local.json"
    for name in bob alice-1 bob-1; do
        printf '%s\n' \
            '{"bootNodes":["/ip4/127.0.0.1/tcp/30333/ws/p2p/12D3KooWQCkBm1BYtkHpocxCwMgR8yjitEeHGx8spzcDLGt2gkBm"],"genesis":{"raw":{"top":{}}},"id":"rococo_local_testnet"}' \
            >"$RUN_DIR/$name/cfg/rococo-local.json"
    done
    printf '%s\n' '{"bootNodes":[],"genesis":{"raw":{"top":{}}},"id":"cubikan-local"}' \
        >"$RUN_DIR/alice-1/cfg/cubikan-local_rococo-local-1000.json"
    printf '%s\n' \
        '{"bootNodes":["/ip4/127.0.0.1/tcp/30335/ws/p2p/12D3KooWHhaSXEhWFi3LibWRNgF9PezoqB9Xeae4fS3dxCowJEg3"],"genesis":{"raw":{"top":{}}},"id":"cubikan-local"}' \
        >"$RUN_DIR/bob-1/cfg/cubikan-local_rococo-local-1000.json"
    /usr/bin/chmod 0600 -- \
        "$RUN_DIR"/*/cfg/rococo-local.json \
        "$RUN_DIR"/alice-1/cfg/cubikan-local_rococo-local-1000.json \
        "$RUN_DIR"/bob-1/cfg/cubikan-local_rococo-local-1000.json
}

assert_canonical_spec_bytes_are_identical() {
    local name
    for name in bob alice-1 bob-1; do
        /usr/bin/cmp -s -- \
            "$RUN_DIR/alice/cfg/rococo-local.cubikan-canonical.json" \
            "$RUN_DIR/$name/cfg/rococo-local.cubikan-canonical.json" ||
            die "$name relay canonical spec differs from relay-a"
    done
    /usr/bin/cmp -s -- \
        "$RUN_DIR/alice-1/cfg/cubikan-local_rococo-local-1000.cubikan-canonical.json" \
        "$RUN_DIR/bob-1/cfg/cubikan-local_rococo-local-1000.cubikan-canonical.json" ||
        die 'collator canonical specs are not byte-identical'
    /usr/bin/python3.14 -I -S - \
        "$RUN_DIR/alice/cfg/rococo-local.cubikan-canonical.json" \
        "$RUN_DIR/alice-1/cfg/cubikan-local_rococo-local-1000.cubikan-canonical.json" <<'PY'
import json
import pathlib
import sys

for argument in sys.argv[1:]:
    raw = pathlib.Path(argument).read_bytes()
    value = json.loads(raw)
    assert value["bootNodes"] == []
    expected = json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8") + b"\n"
    assert raw == expected
PY
}

readonly ALICE_KEY=2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90
readonly BOB_KEY=81b637d8fcd2c6da6359e6963113a1170de795e4b725b84d1e0b4cfd9ec58ce9
readonly ALICE_COLLATOR_KEY=a42ac5108869b599bcbac21069f63fb47f07452fcc4b87e89b3c06a945612d0b
readonly BOB_COLLATOR_KEY=a5fc3eac9107fe9b449965916d54b334233b8077a37f782d9b59e6e98e04def8
readonly ALICE_PEER=12D3KooWQCkBm1BYtkHpocxCwMgR8yjitEeHGx8spzcDLGt2gkBm
readonly BOB_PEER=12D3KooWRkZhiRhsqmrQ28rt73K7V3aCBpqKrLGSXmZ99PTcTZby
readonly ALICE_COLLATOR_PEER=12D3KooWHhaSXEhWFi3LibWRNgF9PezoqB9Xeae4fS3dxCowJEg3
readonly BOB_COLLATOR_PEER=12D3KooWDV1yAeEGiye3t2CQpW7MJ5TJV3TTKpTUUxv4xtaggSnA
RAW=()

relay_raw() {
    local role=$1 name key rpc p2p metrics bootnode
    case "$role" in
        relay-a)
            name=alice; key=$ALICE_KEY; rpc=9944; p2p=30333; metrics=9615
            bootnode="/ip4/127.0.0.1/tcp/30334/ws/p2p/$BOB_PEER"
            ;;
        relay-b)
            name=bob; key=$BOB_KEY; rpc=9945; p2p=30334; metrics=9616
            bootnode="/ip4/127.0.0.1/tcp/30333/ws/p2p/$ALICE_PEER"
            ;;
        *) die 'invalid relay fixture role' ;;
    esac
    RAW=(
        --port "$p2p"
        --execute-workers-max-num 1 --prepare-workers-soft-max-num 1
        --prepare-workers-hard-max-num 1
        --chain "$RUN_DIR/$name/cfg/rococo-local.json"
        --name "$name" --rpc-cors all --rpc-methods unsafe
    )
    if [[ "$role" == relay-a ]]; then
        RAW+=(--bootnodes "$bootnode")
    fi
    RAW+=(--workers-path /run/cubikan-exec/pvf-workers)
    RAW+=(
        --no-mdns --node-key "$key" --no-telemetry --prometheus-external
        --validator --insecure-validator-i-know-what-i-do
    )
    if [[ "$role" == relay-b ]]; then
        RAW+=(--bootnodes "$bootnode")
    fi
    RAW+=(
        --prometheus-port "$metrics" --rpc-port "$rpc"
        --listen-addr "/ip4/127.0.0.1/tcp/$p2p/ws"
        --base-path "$RUN_DIR/$name/data"
    )
}

collator_raw() {
    local role=$1 name key rpc p2p metrics relay_rpc relay_p2p relay_metrics primary_bootnode
    case "$role" in
        collator-a)
            name=alice-1; key=$ALICE_COLLATOR_KEY; rpc=9988; p2p=30335; metrics=9617
            relay_rpc=43001; relay_p2p=43002; relay_metrics=43003
            primary_bootnode="/ip4/127.0.0.1/tcp/30336/ws/p2p/$BOB_COLLATOR_PEER"
            ;;
        collator-b)
            name=bob-1; key=$BOB_COLLATOR_KEY; rpc=9989; p2p=30336; metrics=9618
            relay_rpc=43101; relay_p2p=43102; relay_metrics=43103
            primary_bootnode="/ip4/127.0.0.1/tcp/30335/ws/p2p/$ALICE_COLLATOR_PEER"
            ;;
        *) die 'invalid collator fixture role' ;;
    esac
    RAW=(
        --port "$p2p"
        --name "$name" --node-key "$key"
        --chain "$RUN_DIR/$name/cfg/cubikan-local_rococo-local-1000.json"
        --base-path "$RUN_DIR/$name/data"
        --listen-addr "/ip4/127.0.0.1/tcp/$p2p/ws"
        --prometheus-external --rpc-cors all --rpc-methods unsafe
        --prometheus-port "$metrics" --rpc-port "$rpc" --collator
        --blocks-pruning archive --state-pruning archive
        --bootnodes "$primary_bootnode"
        --
        --base-path "$RUN_DIR/$name/relay-data"
        --chain "$RUN_DIR/$name/cfg/rococo-local.json" --execution wasm
        --bootnodes "/ip4/127.0.0.1/tcp/30333/ws/p2p/$ALICE_PEER"
        --workers-path /run/cubikan-exec/pvf-workers
        --execute-workers-max-num 1 --prepare-workers-soft-max-num 1
        --prepare-workers-hard-max-num 1
        --port "$relay_p2p" --rpc-port "$relay_rpc" --prometheus-port "$relay_metrics"
    )
}

expect_valid() {
    local role=$1 output="$TEST_ROOT/$role.normalized" port_count=0 endpoint_count=0
    local forbidden_rpc_globals=0 expected_endpoints=1
    local workers_count=0 execute_count=0 soft_count=0 hard_count=0 hardware_count=0 index
    local -a worker_verification=()
    if [[ -d /run/cubikan-exec/pvf-workers ]]; then
        worker_verification=(--verify-workers)
    fi
    CUBIKAN_ZOMBIENET_RUN_DIR="$RUN_DIR" \
        "$LAUNCHER" --role "$role" --print0 "${worker_verification[@]}" -- \
        "${RAW[@]}" >"$output"
    mapfile -d '' -t normalized <"$output"
    for index in "${!normalized[@]}"; do
        case "${normalized[$index]}" in
            --port) ((port_count += 1)) ;;
            --experimental-rpc-endpoint) ((endpoint_count += 1)) ;;
            --rpc-port|--ws-port|--rpc-cors|--rpc-methods) ((forbidden_rpc_globals += 1)) ;;
            --workers-path) ((workers_count += 1)) ;;
            --execute-workers-max-num) ((execute_count += 1)) ;;
            --prepare-workers-soft-max-num) ((soft_count += 1)) ;;
            --prepare-workers-hard-max-num) ((hard_count += 1)) ;;
            --no-hardware-benchmarks) ((hardware_count += 1)) ;;
        esac
    done
    [[ $port_count -eq 0 ]] || die "$role normalized vector forwarded the incompatible --port flag"
    [[ "$role" == collator-* ]] && expected_endpoints=2
    [[ $endpoint_count -eq $expected_endpoints && $forbidden_rpc_globals -eq 0 ]] ||
        die "$role normalized vector violates the IPv4-only RPC endpoint contract"
    [[ $workers_count -eq 1 && $execute_count -eq 1 &&
        $soft_count -eq 1 && $hard_count -eq 1 ]] ||
        die "$role normalized vector omitted or duplicated its worker contract"
    [[ $hardware_count -eq $expected_endpoints ]] ||
        die "$role normalized vector omitted or duplicated --no-hardware-benchmarks"
}

expect_reject() {
    local label=$1 role=$2
    shift 2
    if CUBIKAN_ZOMBIENET_RUN_DIR="$RUN_DIR" \
        "$LAUNCHER" --role "$role" --print0 -- "$@" >"$TEST_ROOT/reject.out" 2>"$TEST_ROOT/reject.err"; then
        die "$label mutation was accepted"
    fi
}

replace_value_after_flag() {
    local flag=$1 replacement=$2 index
    for index in "${!RAW[@]}"; do
        if [[ "${RAW[$index]}" == "$flag" ]]; then
            RAW[$((index + 1))]=$replacement
            return
        fi
    done
    die "fixture flag $flag is missing"
}

remove_first_flag_pair() {
    local flag=$1 index
    for index in "${!RAW[@]}"; do
        if [[ "${RAW[$index]}" == "$flag" ]]; then
            RAW=("${RAW[@]:0:$index}" "${RAW[@]:$((index + 2))}")
            return
        fi
    done
    die "fixture flag $flag is missing"
}

test_launcher_accepts_only_the_exact_normalized_vectors() {
    local role
    for role in relay-a relay-b; do
        relay_raw "$role"
        expect_valid "$role"
    done
    for role in collator-a collator-b; do
        collator_raw "$role"
        expect_valid "$role"
    done
    assert_canonical_spec_bytes_are_identical

    relay_raw relay-a
    if CUBIKAN_ZOMBIENET_RUN_DIR="$RUN_DIR" \
        "$LAUNCHER" --role relay-a --print0 --verify-workers -- \
        "${RAW[@]}" >/dev/null 2>&1; then
        die 'launcher accepted a missing private PVF worker mount'
    fi

    relay_raw relay-a
    remove_first_flag_pair --port
    expect_reject missing-primary-port relay-a "${RAW[@]}"
    relay_raw relay-a
    RAW+=(--port 30333)
    expect_reject duplicate-primary-port relay-a "${RAW[@]}"
    relay_raw relay-a
    RAW[1]=30334
    expect_reject mismatched-primary-port relay-a "${RAW[@]}"
    relay_raw relay-a
    replace_value_after_flag --listen-addr /ip4/127.0.0.1/tcp/30333
    expect_reject mismatched-primary-listen relay-a "${RAW[@]}"
    relay_raw relay-a
    replace_value_after_flag --name mallory
    expect_reject mismatched-role-name relay-a "${RAW[@]}"
    relay_raw relay-a
    replace_value_after_flag --bootnodes "/ip4/127.0.0.1/tcp/30333/ws/p2p/$ALICE_PEER"
    expect_reject self-bootnode relay-a "${RAW[@]}"
    relay_raw relay-a
    RAW+=(--bootnodes "/ip4/127.0.0.1/tcp/30334/ws/p2p/$BOB_PEER")
    expect_reject duplicate-bootnode relay-a "${RAW[@]}"
    relay_raw relay-a
    remove_first_flag_pair --workers-path
    expect_reject missing-worker-path relay-a "${RAW[@]}"
    relay_raw relay-a
    replace_value_after_flag --workers-path /tmp/pvf-workers
    expect_reject wrong-worker-path relay-a "${RAW[@]}"
    relay_raw relay-a
    remove_first_flag_pair --execute-workers-max-num
    expect_reject missing-execute-worker-cap relay-a "${RAW[@]}"
    relay_raw relay-a
    replace_value_after_flag --prepare-workers-soft-max-num 2
    expect_reject wrong-soft-worker-cap relay-a "${RAW[@]}"
    relay_raw relay-a
    RAW+=(--prepare-workers-hard-max-num 1)
    expect_reject duplicate-hard-worker-cap relay-a "${RAW[@]}"
    relay_raw relay-a
    remove_first_flag_pair --rpc-cors
    expect_reject missing-primary-rpc-cors relay-a "${RAW[@]}"
    relay_raw relay-a
    remove_first_flag_pair --rpc-methods
    expect_reject missing-primary-rpc-methods relay-a "${RAW[@]}"
    relay_raw relay-a
    replace_value_after_flag --rpc-methods safe
    expect_reject wrong-primary-rpc-methods relay-a "${RAW[@]}"
    relay_raw relay-a
    RAW+=(--experimental-rpc-endpoint listen-addr=127.0.0.1:9944,methods=unsafe,cors=all)
    expect_reject generated-structured-rpc-endpoint relay-a "${RAW[@]}"

    collator_raw collator-a
    replace_value_after_flag --blocks-pruning 256
    expect_reject nonarchive-blocks collator-a "${RAW[@]}"
    collator_raw collator-a
    remove_first_flag_pair --state-pruning
    expect_reject missing-state-archive collator-a "${RAW[@]}"
    collator_raw collator-a
    RAW+=(--mystery value)
    expect_reject unknown-generated-flag collator-a "${RAW[@]}"
    collator_raw collator-a
    RAW+=(--)
    expect_reject duplicate-separator collator-a "${RAW[@]}"
    collator_raw collator-a
    remove_first_flag_pair --workers-path
    expect_reject missing-relay-side-workers collator-a "${RAW[@]}"
    collator_raw collator-a
    replace_value_after_flag --prepare-workers-hard-max-num 0
    expect_reject wrong-relay-side-worker-cap collator-a "${RAW[@]}"
    collator_raw collator-a
    RAW=(--workers-path /run/cubikan-exec/pvf-workers "${RAW[@]}")
    expect_reject worker-path-on-collator-primary collator-a "${RAW[@]}"
    collator_raw collator-a
    RAW+=(--rpc-cors all)
    expect_reject primary-rpc-policy-on-relay-side collator-a "${RAW[@]}"

    printf '%s\n' \
        '{"bootNodes":[],"bootNodes":[],"genesis":{"raw":{"top":{}}},"id":"rococo_local_testnet"}' \
        >"$RUN_DIR/alice/cfg/rococo-local.json"
    relay_raw relay-a
    expect_reject duplicate-chain-spec-key relay-a "${RAW[@]}"
    printf '%s\n' '{"bootNodes":[],"id":"rococo_local_testnet"}' \
        >"$RUN_DIR/alice/cfg/rococo-local.json"
    relay_raw relay-a
    expect_reject missing-raw-genesis relay-a "${RAW[@]}"
    printf '%s\n' \
        '{"bootNodes":["/dns/public.example.invalid/tcp/30333/ws/p2p/12D3KooWQCkBm1BYtkHpocxCwMgR8yjitEeHGx8spzcDLGt2gkBm"],"genesis":{"raw":{"top":{}}},"id":"rococo_local_testnet"}' \
        >"$RUN_DIR/bob/cfg/rococo-local.json"
    relay_raw relay-b
    expect_reject public-chain-spec-bootnode relay-b "${RAW[@]}"
    printf '%s\n' '{"bootNodes":[],"genesis":{"raw":{"top":{}}},"id":"tampered"}' \
        >"$RUN_DIR/alice-1/cfg/cubikan-local_rococo-local-1000.cubikan-canonical.json"
    collator_raw collator-a
    expect_reject tampered-canonical-chain-spec collator-a "${RAW[@]}"
}

test_config_is_the_closed_four_node_topology
test_pinned_provider_compatibility_is_explicit
test_materializer_and_candidate_gate_are_closed
test_archive_checker_executes_on_exact_opened_bytes
test_fresh_npm_cache_contract_is_closed
test_journey_driver_interface_is_closed
prepare_run_tree
test_launcher_accepts_only_the_exact_normalized_vectors
printf '%s\n' 'T-1115 Zombienet config, materializer, launcher, and mutation guards passed'
