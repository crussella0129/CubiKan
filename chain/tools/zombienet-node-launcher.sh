#!/usr/bin/bash -p
set -euo pipefail

readonly TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_ROOT="$(cd -- "$TOOL_DIR/../.." && pwd -P)"
readonly NORMALIZER="$TOOL_DIR/normalize-node-argv.sh"
readonly EXPECTED_NORMALIZER_SHA256=41662d4f05c8f1a875d1b2901bda52c82478046403451e261c33de658e20a1d2
readonly POLKADOT="$PROJECT_ROOT/chain/.cache/downloads/polkadot"
readonly OMNI="$PROJECT_ROOT/chain/.cache/downloads/polkadot-omni-node"
readonly PVF_WORKERS_DIR=/run/cubikan-exec/pvf-workers
readonly ENV=/usr/lib/cargo/bin/coreutils/env
readonly STAT=/usr/lib/cargo/bin/coreutils/stat
readonly SHA256SUM=/usr/lib/cargo/bin/coreutils/sha256sum
readonly PYTHON=/usr/bin/python3.14

readonly ALICE_KEY=2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90
readonly BOB_KEY=81b637d8fcd2c6da6359e6963113a1170de795e4b725b84d1e0b4cfd9ec58ce9
readonly ALICE_COLLATOR_KEY=a42ac5108869b599bcbac21069f63fb47f07452fcc4b87e89b3c06a945612d0b
readonly BOB_COLLATOR_KEY=a5fc3eac9107fe9b449965916d54b334233b8077a37f782d9b59e6e98e04def8
readonly ALICE_PEER=12D3KooWQCkBm1BYtkHpocxCwMgR8yjitEeHGx8spzcDLGt2gkBm
readonly BOB_PEER=12D3KooWRkZhiRhsqmrQ28rt73K7V3aCBpqKrLGSXmZ99PTcTZby
readonly ALICE_COLLATOR_PEER=12D3KooWHhaSXEhWFi3LibWRNgF9PezoqB9Xeae4fS3dxCowJEg3
readonly BOB_COLLATOR_PEER=12D3KooWDV1yAeEGiye3t2CQpW7MJ5TJV3TTKpTUUxv4xtaggSnA

die() {
    printf 'zombienet-node-launcher: %s\n' "$*" >&2
    exit 1
}

usage() {
    printf '%s\n' \
        'usage: zombienet-node-launcher.sh --role ROLE [--print0 [--verify-workers]] -- GENERATED_ARG...' >&2
    exit 2
}

[[ $# -ge 4 && "$1" == --role ]] || usage
role=$2
shift 2
print0=0
verify_workers=0
if [[ "${1:-}" == --print0 ]]; then
    print0=1
    shift
    if [[ "${1:-}" == --verify-workers ]]; then
        verify_workers=1
        shift
    fi
fi
[[ "${1:-}" == -- ]] || usage
shift
[[ $# -gt 0 ]] || usage
readonly ROLE=$role
readonly PRINT0=$print0
readonly VERIFY_WORKERS=$verify_workers
unset role print0 verify_workers

case "$ROLE" in
    relay-a)
        node_name=alice; node_key=$ALICE_KEY; node_asset=$POLKADOT
        primary_rpc=9944; primary_p2p=30333; primary_metrics=9615
        primary_bootnode="/ip4/127.0.0.1/tcp/30334/ws/p2p/$BOB_PEER"
        relay_rpc=''; relay_p2p=''; relay_metrics=''; relay_bootnode=''
        ;;
    relay-b)
        node_name=bob; node_key=$BOB_KEY; node_asset=$POLKADOT
        primary_rpc=9945; primary_p2p=30334; primary_metrics=9616
        primary_bootnode="/ip4/127.0.0.1/tcp/30333/ws/p2p/$ALICE_PEER"
        relay_rpc=''; relay_p2p=''; relay_metrics=''; relay_bootnode=''
        ;;
    collator-a)
        node_name=alice-1; node_key=$ALICE_COLLATOR_KEY; node_asset=$OMNI
        primary_rpc=9988; primary_p2p=30335; primary_metrics=9617
        primary_bootnode="/ip4/127.0.0.1/tcp/30336/ws/p2p/$BOB_COLLATOR_PEER"
        relay_rpc=9990; relay_p2p=30337; relay_metrics=9619
        relay_bootnode="/ip4/127.0.0.1/tcp/30333/ws/p2p/$ALICE_PEER"
        ;;
    collator-b)
        node_name=bob-1; node_key=$BOB_COLLATOR_KEY; node_asset=$OMNI
        primary_rpc=9989; primary_p2p=30336; primary_metrics=9618
        primary_bootnode="/ip4/127.0.0.1/tcp/30335/ws/p2p/$ALICE_COLLATOR_PEER"
        relay_rpc=9991; relay_p2p=30338; relay_metrics=9620
        # Bob is paused to freeze F; both collators must retain live Alice.
        relay_bootnode="/ip4/127.0.0.1/tcp/30333/ws/p2p/$ALICE_PEER"
        ;;
    *) usage ;;
esac
readonly NODE_NAME=$node_name NODE_KEY=$node_key NODE_ASSET=$node_asset
readonly PRIMARY_RPC=$primary_rpc PRIMARY_P2P=$primary_p2p PRIMARY_METRICS=$primary_metrics
readonly PRIMARY_BOOTNODE=$primary_bootnode
readonly RELAY_RPC=$relay_rpc RELAY_P2P=$relay_p2p RELAY_METRICS=$relay_metrics
readonly RELAY_BOOTNODE=$relay_bootnode
unset node_name node_key node_asset primary_rpc primary_p2p primary_metrics primary_bootnode
unset relay_rpc relay_p2p relay_metrics relay_bootnode

run_dir=${CUBIKAN_ZOMBIENET_RUN_DIR:-}
[[ "$run_dir" == /* && "$run_dir" != / && -d "$run_dir" && ! -L "$run_dir" ]] ||
    die 'CUBIKAN_ZOMBIENET_RUN_DIR is not an absolute real directory'
canonical_run_dir="$(/usr/lib/cargo/bin/coreutils/realpath -e -- "$run_dir")" || die 'cannot canonicalize run directory'
[[ "$canonical_run_dir" == "$run_dir" ]] || die 'run directory is not canonical'
readonly RUN_DIR=$canonical_run_dir
unset run_dir canonical_run_dir
[[ "$($STAT -Lc '%u:%a:%F' -- "$RUN_DIR")" == "$EUID:700:directory" ]] ||
    die 'run directory must be owned by this user with mode 0700'

verify_pvf_worker_file() {
    local path=$1 expected_size=$2 expected_hash=$3
    local prefix_fd hash_fd opened_identity path_identity prefix digest
    [[ -f "$path" && -x "$path" && ! -L "$path" ]] ||
        die 'PVF worker is missing, symbolic, or nonexecutable'
    [[ "$($STAT -Lc '%u:%a:%h:%s:%F' -- "$path")" == \
        "$EUID:500:1:$expected_size:regular file" ]] || die 'PVF worker metadata mismatch'
    exec {prefix_fd}<"$path"
    exec {hash_fd}<"$path"
    opened_identity="$($STAT -Lc '%d:%i:%s:%Y:%Z' -- "/proc/self/fd/$hash_fd")"
    path_identity="$($STAT -Lc '%d:%i:%s:%Y:%Z' -- "$path")"
    [[ "$opened_identity" == "$path_identity" &&
        "$($STAT -Lc '%d:%i:%s:%Y:%Z' -- "/proc/self/fd/$prefix_fd")" == "$opened_identity" ]] ||
        die 'PVF worker changed while opening'
    IFS= read -r -N 4 prefix <&"$prefix_fd" || die 'PVF worker lacks a complete ELF header'
    [[ "$prefix" == $'\x7fELF' ]] || die 'PVF worker is not ELF'
    digest="$($SHA256SUM - <&"$hash_fd")"
    [[ "${digest%% *}" == "$expected_hash" &&
        "$($STAT -Lc '%d:%i:%s:%Y:%Z' -- "$path")" == "$opened_identity" ]] ||
        die 'PVF worker bytes or identity mismatch'
    exec {prefix_fd}<&-
    exec {hash_fd}<&-
}

verify_pvf_worker_tree() {
    local mount_proof
    local -a entries
    [[ -d "$PVF_WORKERS_DIR" && ! -L "$PVF_WORKERS_DIR" &&
        "$(/usr/lib/cargo/bin/coreutils/realpath -e -- "$PVF_WORKERS_DIR")" == "$PVF_WORKERS_DIR" &&
        "$($STAT -fLc '%T' -- "$PVF_WORKERS_DIR")" == tmpfs &&
        "$($STAT -Lc '%u:%a:%F' -- "$PVF_WORKERS_DIR")" == "$EUID:500:directory" ]] ||
        die 'PVF worker root is not the exact private tmpfs directory'
    shopt -s dotglob nullglob
    entries=("$PVF_WORKERS_DIR"/*)
    shopt -u dotglob nullglob
    [[ ${#entries[@]} -eq 2 &&
        "${entries[0]}" == "$PVF_WORKERS_DIR/polkadot-execute-worker" &&
        "${entries[1]}" == "$PVF_WORKERS_DIR/polkadot-prepare-worker" ]] ||
        die 'PVF worker root has a missing or extra entry'
    mount_proof="$(/usr/bin/gawk -v target="$PVF_WORKERS_DIR" '
        $5 == target {
            count++
            split($6, options, ",")
            delete seen
            for (option in options) seen[options[option]] = 1
            dash = 0
            for (i = 7; i <= NF; i++) if ($i == "-") { dash = i; break }
            if (seen["ro"] && seen["nodev"] && seen["nosuid"] && !seen["noexec"] &&
                dash > 0 && $(dash + 1) == "tmpfs") ok++
        }
        END { printf "%d:%d\n", count + 0, ok + 0 }
    ' /proc/self/mountinfo)"
    [[ "$mount_proof" == 1:1 ]] || die 'PVF worker root is not one executable read-only bind mount'
    verify_pvf_worker_file "$PVF_WORKERS_DIR/polkadot-prepare-worker" \
        21387400 5e67a05516e24d5e9b9616bacb3a2d58235beb3392de14dfbe51ff6914244267
    verify_pvf_worker_file "$PVF_WORKERS_DIR/polkadot-execute-worker" \
        19463336 cc642041ef2582d972071cd4f7122e9803703bc7775e8d432b2d7626f5011b21
}

if [[ $PRINT0 -eq 0 || $VERIFY_WORKERS -eq 1 ]]; then
    verify_pvf_worker_tree
fi

readonly NODE_ROOT="$RUN_DIR/$NODE_NAME"
readonly PRIMARY_BASE="$NODE_ROOT/data"
readonly RELAY_BASE="$NODE_ROOT/relay-data"
readonly GENERATED_RELAY_CHAIN="$NODE_ROOT/cfg/rococo-local.json"
readonly RELAY_CHAIN="$NODE_ROOT/cfg/rococo-local.cubikan-canonical.json"
if [[ "$ROLE" == relay-* ]]; then
    generated_primary_chain=$GENERATED_RELAY_CHAIN
    primary_chain=$RELAY_CHAIN
else
    generated_primary_chain="$NODE_ROOT/cfg/cubikan-local_rococo-local-1000.json"
    primary_chain="$NODE_ROOT/cfg/cubikan-local_rococo-local-1000.cubikan-canonical.json"
fi
readonly GENERATED_PRIMARY_CHAIN=$generated_primary_chain
readonly PRIMARY_CHAIN=$primary_chain
unset generated_primary_chain primary_chain

verify_private_directory() {
    local path=$1 canonical mode mode_value
    [[ -d "$path" && ! -L "$path" ]] || die 'generated node directory is missing or symbolic'
    canonical="$(/usr/lib/cargo/bin/coreutils/realpath -e -- "$path")" || die 'cannot canonicalize generated directory'
    [[ "$canonical" == "$path" && "$path" == "$NODE_ROOT"/* ]] ||
        die 'generated node directory escapes its exact root'
    [[ "$($STAT -Lc '%u:%F' -- "$path")" == "$EUID:directory" ]] ||
        die 'generated node directory owner or type mismatch'
    mode="$($STAT -Lc '%a' -- "$path")"
    mode_value=$((8#$mode))
    (( (mode_value & 077) == 0 )) || die 'generated node directory is not owner-only'
}

materialize_canonical_chain_spec() {
    local source=$1 destination=$2 expected_id=$3 expected_bootnode=$4
    "$ENV" -i HOME=/home/charles LC_ALL=C LANG=C TZ=UTC PATH=/usr/bin:/bin \
        "$PYTHON" -I -S - "$source" "$destination" "$expected_id" "$expected_bootnode" <<'PY'
import json
import hashlib
import os
import stat
import sys

source, destination, expected_id, expected_bootnode = sys.argv[1:]
maximum_bytes = 8_388_608

def fail(message):
    raise SystemExit(f"zombienet-node-launcher: {message}")

def identity(value):
    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_nlink,
        value.st_uid,
        value.st_gid,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )

def read_private_regular(path, label):
    flags = os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        fail(f"cannot open {label}: {error.errno}")
    try:
        opened_before = os.fstat(descriptor)
        path_before = os.lstat(path)
        if identity(opened_before) != identity(path_before):
            fail(f"{label} descriptor/path identity mismatch")
        if (
            not stat.S_ISREG(opened_before.st_mode)
            or opened_before.st_uid != os.geteuid()
            or opened_before.st_nlink != 1
            or stat.S_IMODE(opened_before.st_mode) & 0o077
            or opened_before.st_size <= 0
            or opened_before.st_size > maximum_bytes
        ):
            fail(f"{label} metadata is outside the closed contract")
        chunks = []
        remaining = opened_before.st_size
        while remaining:
            chunk = os.read(descriptor, min(65_536, remaining))
            if not chunk:
                fail(f"{label} ended before its pinned size")
            chunks.append(chunk)
            remaining -= len(chunk)
        if os.read(descriptor, 1):
            fail(f"{label} grew during capture")
        opened_after = os.fstat(descriptor)
        path_after = os.lstat(path)
        if identity(opened_before) != identity(opened_after) or identity(opened_after) != identity(path_after):
            fail(f"{label} identity changed during capture")
        return b"".join(chunks)
    finally:
        os.close(descriptor)

def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            fail("generated chain spec contains a duplicate JSON key")
        result[key] = value
    return result

def reject_constant(value):
    fail(f"generated chain spec contains non-finite JSON number {value}")

raw = read_private_regular(source, "generated chain spec")
try:
    text = raw.decode("utf-8", "strict")
    document = json.loads(
        text,
        object_pairs_hook=unique_object,
        parse_constant=reject_constant,
    )
except (UnicodeDecodeError, json.JSONDecodeError) as error:
    fail(f"generated chain spec is not exact UTF-8 JSON: {error}")
if not isinstance(document, dict) or document.get("id") != expected_id:
    fail("generated chain spec family id drifted")
genesis = document.get("genesis")
raw_genesis = genesis.get("raw") if isinstance(genesis, dict) else None
raw_top = raw_genesis.get("top") if isinstance(raw_genesis, dict) else None
if not isinstance(raw_top, dict):
    fail("generated chain spec lacks raw genesis top storage")
if "bootNodes" not in document or not isinstance(document["bootNodes"], list):
    fail("generated chain spec lacks one top-level bootNodes array")
allowed_bootnodes = [[]]
if expected_bootnode:
    allowed_bootnodes.append([expected_bootnode])
if document["bootNodes"] not in allowed_bootnodes:
    fail("generated chain spec bootNodes preimage is outside the exact role contract")
document["bootNodes"] = []
try:
    canonical = (
        json.dumps(
            document,
            ensure_ascii=False,
            allow_nan=False,
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
        + b"\n"
    )
except (TypeError, ValueError) as error:
    fail(f"generated chain spec cannot be serialized canonically: {error}")
if not canonical or len(canonical) > maximum_bytes:
    fail("canonical chain spec size is outside the closed contract")

if source == destination:
    if raw != canonical:
        fail("replayed canonical chain spec bytes drifted")
else:
    destination_exists = False
    try:
        existing = read_private_regular(destination, "canonical chain spec")
        destination_exists = True
    except SystemExit:
        if os.path.lexists(destination):
            raise
    if destination_exists:
        if existing != canonical:
            fail("existing canonical chain spec bytes drifted")
    else:
        parent = os.path.dirname(destination)
        temporary = f"{destination}.tmp-{os.getpid()}"
        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW
        descriptor = -1
        try:
            descriptor = os.open(temporary, flags, 0o600)
            remaining = memoryview(canonical)
            while remaining:
                written = os.write(descriptor, remaining)
                if written <= 0:
                    fail("canonical chain spec write made no progress")
                remaining = remaining[written:]
            os.fsync(descriptor)
            os.close(descriptor)
            descriptor = -1
            if read_private_regular(source, "generated chain spec") != raw:
                fail("generated chain spec changed before canonical publication")
            os.link(temporary, destination, follow_symlinks=False)
            os.unlink(temporary)
            parent_descriptor = os.open(
                parent,
                os.O_RDONLY | os.O_CLOEXEC | os.O_DIRECTORY | os.O_NOFOLLOW,
            )
            try:
                os.fsync(parent_descriptor)
            finally:
                os.close(parent_descriptor)
        finally:
            if descriptor >= 0:
                os.close(descriptor)
            try:
                os.unlink(temporary)
            except FileNotFoundError:
                pass

final_source = read_private_regular(source, "generated chain spec")
final_destination = read_private_regular(destination, "canonical chain spec")
if final_destination != canonical:
    fail("published canonical chain spec bytes drifted")
if final_source != raw:
    fail("generated chain spec changed across canonical publication")

def handoff_identity(path):
    observed = os.lstat(path)
    return ":".join(
        str(value)
        for value in (
            observed.st_dev,
            observed.st_ino,
            observed.st_size,
            observed.st_mtime_ns // 1_000_000_000,
            observed.st_ctime_ns // 1_000_000_000,
        )
    )

print(
    "\t".join(
        (
            handoff_identity(source),
            hashlib.sha256(final_source).hexdigest(),
            handoff_identity(destination),
            hashlib.sha256(final_destination).hexdigest(),
        )
    )
)
PY
}

PINNED_PATHS=()
PINNED_IDENTITIES=()
PINNED_HASHES=()
pin_generated_file() {
    local path=$1 expected_identity=$2 expected_hash=$3 canonical mode mode_value digest observed_identity
    [[ -f "$path" && ! -L "$path" ]] || die 'generated chain spec is missing or symbolic'
    canonical="$(/usr/lib/cargo/bin/coreutils/realpath -e -- "$path")" || die 'cannot canonicalize generated chain spec'
    [[ "$canonical" == "$path" && "$path" == "$NODE_ROOT"/* ]] ||
        die 'generated chain spec escapes its exact root'
    [[ "$($STAT -Lc '%u:%F' -- "$path")" == "$EUID:regular file" ]] ||
        die 'generated chain spec owner or type mismatch'
    mode="$($STAT -Lc '%a' -- "$path")"
    mode_value=$((8#$mode))
    (( (mode_value & 077) == 0 )) || die 'generated chain spec is not owner-only'
    observed_identity="$($STAT -Lc '%d:%i:%s:%Y:%Z' -- "$path")"
    digest="$($SHA256SUM -- "$path")"
    [[ "$observed_identity" == "$expected_identity" && "${digest%% *}" == "$expected_hash" ]] ||
        die 'generated chain spec changed across the canonicalizer handoff'
    PINNED_PATHS+=("$path")
    PINNED_IDENTITIES+=("$observed_identity")
    PINNED_HASHES+=("${digest%% *}")
}

recheck_generated_files() {
    local index path digest
    for index in "${!PINNED_PATHS[@]}"; do
        path=${PINNED_PATHS[$index]}
        [[ -f "$path" && ! -L "$path" &&
            "$($STAT -Lc '%d:%i:%s:%Y:%Z' -- "$path")" == "${PINNED_IDENTITIES[$index]}" ]] ||
            die 'generated chain spec identity changed before execution'
        digest="$($SHA256SUM -- "$path")"
        [[ "${digest%% *}" == "${PINNED_HASHES[$index]}" ]] ||
            die 'generated chain spec bytes changed before execution'
    done
}

verify_private_directory "$PRIMARY_BASE"
verify_private_directory "$NODE_ROOT/cfg"
if [[ "$ROLE" == collator-* ]]; then
    verify_private_directory "$RELAY_BASE"
fi
case "$ROLE" in
    relay-a)
        primary_spec_id=rococo_local_testnet
        primary_spec_bootnode=''
        ;;
    relay-b)
        primary_spec_id=rococo_local_testnet
        primary_spec_bootnode="/ip4/127.0.0.1/tcp/30333/ws/p2p/$ALICE_PEER"
        ;;
    collator-a)
        primary_spec_id=cubikan-local
        primary_spec_bootnode=''
        ;;
    collator-b)
        primary_spec_id=cubikan-local
        primary_spec_bootnode="/ip4/127.0.0.1/tcp/30335/ws/p2p/$ALICE_COLLATOR_PEER"
        ;;
esac
primary_handoff="$(materialize_canonical_chain_spec \
    "$GENERATED_PRIMARY_CHAIN" "$PRIMARY_CHAIN" "$primary_spec_id" "$primary_spec_bootnode")"
unset primary_spec_id primary_spec_bootnode
IFS=$'\t' read -r primary_source_identity primary_source_hash \
    primary_canonical_identity primary_canonical_hash <<<"$primary_handoff"
[[ -n "$primary_source_identity" && -n "$primary_source_hash" &&
    -n "$primary_canonical_identity" && -n "$primary_canonical_hash" ]] ||
    die 'primary chain-spec canonicalizer handoff is malformed'
pin_generated_file "$GENERATED_PRIMARY_CHAIN" "$primary_source_identity" "$primary_source_hash"
pin_generated_file "$PRIMARY_CHAIN" "$primary_canonical_identity" "$primary_canonical_hash"
unset primary_handoff primary_source_identity primary_source_hash
unset primary_canonical_identity primary_canonical_hash
if [[ "$ROLE" == collator-* ]]; then
    relay_handoff="$(materialize_canonical_chain_spec \
        "$GENERATED_RELAY_CHAIN" "$RELAY_CHAIN" rococo_local_testnet \
        "/ip4/127.0.0.1/tcp/30333/ws/p2p/$ALICE_PEER")"
    IFS=$'\t' read -r relay_source_identity relay_source_hash \
        relay_canonical_identity relay_canonical_hash <<<"$relay_handoff"
    [[ -n "$relay_source_identity" && -n "$relay_source_hash" &&
        -n "$relay_canonical_identity" && -n "$relay_canonical_hash" ]] ||
        die 'relay chain-spec canonicalizer handoff is malformed'
    pin_generated_file "$GENERATED_RELAY_CHAIN" "$relay_source_identity" "$relay_source_hash"
    pin_generated_file "$RELAY_CHAIN" "$relay_canonical_identity" "$relay_canonical_hash"
    unset relay_handoff relay_source_identity relay_source_hash
    unset relay_canonical_identity relay_canonical_hash
fi

RAW=("$@")
separator=-1
for index in "${!RAW[@]}"; do
    if [[ "${RAW[$index]}" == -- ]]; then
        [[ $separator -eq -1 ]] || die 'generated argv contains duplicate separators'
        separator=$index
    fi
done
if [[ "$ROLE" == relay-* ]]; then
    [[ $separator -eq -1 ]] || die 'relay generated argv contains a separator'
    RAW_PRIMARY=("${RAW[@]}")
    RAW_RELAY=()
else
    [[ $separator -ge 0 ]] || die 'collator generated argv is missing its separator'
    RAW_PRIMARY=("${RAW[@]:0:$separator}")
    RAW_RELAY=("${RAW[@]:$((separator + 1))}")
fi

require_exact_raw_value() {
    local array_name=$1 flag=$2 expected=$3 count=0 found='' index
    local -n values=$array_name
    for index in "${!values[@]}"; do
        if [[ "${values[$index]}" == "$flag" ]]; then
            ((count += 1))
            (( index + 1 < ${#values[@]} )) || die "generated $flag has no value"
            found=${values[$((index + 1))]}
        fi
    done
    [[ $count -eq 1 && "$found" == "$expected" ]] ||
        die "generated argv must contain one exact $flag"
}

require_exact_raw_chain() {
    local array_name=$1 expected_generated=$2 expected_canonical=$3
    local count=0 found='' index
    local -n values=$array_name
    for index in "${!values[@]}"; do
        if [[ "${values[$index]}" == --chain ]]; then
            ((count += 1))
            (( index + 1 < ${#values[@]} )) || die 'generated --chain has no value'
            found=${values[$((index + 1))]}
        fi
    done
    [[ $count -eq 1 && ( "$found" == "$expected_generated" || "$found" == "$expected_canonical" ) ]] ||
        die 'generated argv must contain one exact role-local --chain path'
}

replace_raw_chain() {
    local array_name=$1 replacement=$2 count=0 index
    local -n values=$array_name
    for index in "${!values[@]}"; do
        if [[ "${values[$index]}" == --chain ]]; then
            ((count += 1))
            (( index + 1 < ${#values[@]} )) || die 'generated --chain has no value'
            values[$((index + 1))]=$replacement
        fi
    done
    [[ $count -eq 1 ]] || die 'generated argv must contain one exact --chain path'
}

require_one_raw_port() {
    local array_name=$1 flag=$2 count=0 value='' index
    local -n values=$array_name
    for index in "${!values[@]}"; do
        if [[ "${values[$index]}" == "$flag" ]]; then
            ((count += 1))
            (( index + 1 < ${#values[@]} )) || die "generated relay-side $flag has no value"
            value=${values[$((index + 1))]}
        fi
    done
    [[ $count -eq 1 && "$value" =~ ^[0-9]{1,5}$ && "$value" -ge 1 && "$value" -le 65535 ]] ||
        die "generated relay side must contain one valid $flag"
}

require_exact_raw_value RAW_PRIMARY --name "$NODE_NAME"
require_exact_raw_value RAW_PRIMARY --node-key "$NODE_KEY"
require_exact_raw_chain RAW_PRIMARY "$GENERATED_PRIMARY_CHAIN" "$PRIMARY_CHAIN"
require_exact_raw_value RAW_PRIMARY --base-path "$PRIMARY_BASE"
# The pinned native provider allocates primary p2p ports and generates the
# listen address, but its cmdGenerator omits the primary --port flag. The
# reviewed config command prefix supplies this one exact compatibility field.
require_exact_raw_value RAW_PRIMARY --port "$PRIMARY_P2P"
require_exact_raw_value RAW_PRIMARY --rpc-port "$PRIMARY_RPC"
require_exact_raw_value RAW_PRIMARY --rpc-cors all
require_exact_raw_value RAW_PRIMARY --rpc-methods unsafe
require_exact_raw_value RAW_PRIMARY --prometheus-port "$PRIMARY_METRICS"
require_exact_raw_value RAW_PRIMARY --listen-addr "/ip4/127.0.0.1/tcp/$PRIMARY_P2P/ws"
if [[ "$ROLE" == relay-* ]]; then
    require_exact_raw_value RAW_PRIMARY --workers-path "$PVF_WORKERS_DIR"
    require_exact_raw_value RAW_PRIMARY --execute-workers-max-num 1
    require_exact_raw_value RAW_PRIMARY --prepare-workers-soft-max-num 1
    require_exact_raw_value RAW_PRIMARY --prepare-workers-hard-max-num 1
else
    require_exact_raw_value RAW_PRIMARY --blocks-pruning archive
    require_exact_raw_value RAW_PRIMARY --state-pruning archive
    require_exact_raw_chain RAW_RELAY "$GENERATED_RELAY_CHAIN" "$RELAY_CHAIN"
    require_exact_raw_value RAW_RELAY --base-path "$RELAY_BASE"
    require_one_raw_port RAW_RELAY --port
    require_one_raw_port RAW_RELAY --rpc-port
    require_one_raw_port RAW_RELAY --prometheus-port
    require_exact_raw_value RAW_RELAY --workers-path "$PVF_WORKERS_DIR"
    require_exact_raw_value RAW_RELAY --execute-workers-max-num 1
    require_exact_raw_value RAW_RELAY --prepare-workers-soft-max-num 1
    require_exact_raw_value RAW_RELAY --prepare-workers-hard-max-num 1
fi

replace_raw_chain RAW_PRIMARY "$PRIMARY_CHAIN"
if [[ "$ROLE" == relay-* ]]; then
    RAW=("${RAW_PRIMARY[@]}")
else
    replace_raw_chain RAW_RELAY "$RELAY_CHAIN"
    RAW=("${RAW_PRIMARY[@]}" -- "${RAW_RELAY[@]}")
fi

[[ -f "$NORMALIZER" && ! -L "$NORMALIZER" ]] || die 'normalizer is missing or symbolic'
normalizer_identity="$($STAT -Lc '%d:%i:%s:%Y:%Z' -- "$NORMALIZER")"
normalizer_content=''
IFS= read -r -d '' normalizer_content <"$NORMALIZER" || [[ -n "$normalizer_content" ]] ||
    die 'cannot capture normalizer bytes'
normalizer_digest="$(printf '%s' "$normalizer_content" | "$SHA256SUM")"
normalizer_digest=${normalizer_digest%% *}
[[ "$normalizer_digest" == "$EXPECTED_NORMALIZER_SHA256" &&
    "$($STAT -Lc '%d:%i:%s:%Y:%Z' -- "$NORMALIZER")" == "$normalizer_identity" ]] ||
    die 'normalizer identity mismatch during memory capture'
readonly NORMALIZER_CONTENT=$normalizer_content
readonly NORMALIZER_DIGEST=$normalizer_digest
unset normalizer_content normalizer_digest normalizer_identity

run_bound_normalizer() {
    "$ENV" -i CUBIKAN_NORMALIZER_SANITIZED=1 HOME=/home/charles \
        LC_ALL=C LANG=C TZ=UTC PATH=/usr/bin:/bin \
        /usr/bin/bash --noprofile --norc -p -c "$NORMALIZER_CONTENT" "$NORMALIZER" \
        __cubikan_normalizer_bound_memory_v1__ "$NORMALIZER" "$NORMALIZER_DIGEST" \
        "$NORMALIZER_CONTENT" "$@"
}

first_vector="$(/usr/lib/cargo/bin/coreutils/mktemp "$RUN_DIR/.cubikan-$ROLE-normalized.XXXXXX")"
second_vector="$(/usr/lib/cargo/bin/coreutils/mktemp "$RUN_DIR/.cubikan-$ROLE-normalized.XXXXXX")"
cleanup_vectors() {
    [[ -n "${first_vector:-}" && "$first_vector" == "$RUN_DIR"/.cubikan-"$ROLE"-normalized.?????? ]] &&
        /usr/bin/gnurm -f -- "$first_vector"
    [[ -n "${second_vector:-}" && "$second_vector" == "$RUN_DIR"/.cubikan-"$ROLE"-normalized.?????? ]] &&
        /usr/bin/gnurm -f -- "$second_vector"
    return 0
}
terminate() {
    local status=$1
    trap - EXIT HUP INT TERM
    cleanup_vectors
    exit "$status"
}
trap cleanup_vectors EXIT
trap 'terminate 129' HUP
trap 'terminate 130' INT
trap 'terminate 143' TERM

normalizer_print_args=(--role "$ROLE" --print0)
if [[ $VERIFY_WORKERS -eq 1 ]]; then
    normalizer_print_args+=(--verify-workers)
fi
run_bound_normalizer "${normalizer_print_args[@]}" -- "$NODE_ASSET" "${RAW[@]}" >"$first_vector" ||
    die 'normalizer rejected generated argv'
run_bound_normalizer "${normalizer_print_args[@]}" -- "$NODE_ASSET" "${RAW[@]}" >"$second_vector" ||
    die 'normalizer rejected generated argv on identity replay'
first_digest="$($SHA256SUM -- "$first_vector")"
second_digest="$($SHA256SUM -- "$second_vector")"
[[ "${first_digest%% *}" == "${second_digest%% *}" ]] ||
    die 'normalized argv bytes changed across the bound replay'
/usr/bin/cmp -s -- "$first_vector" "$second_vector" ||
    die 'normalized argv vector changed across the bound replay'

mapfile -d '' -t NORMALIZED <"$first_vector"
expected=()
if [[ "$ROLE" == relay-a ]]; then
    expected=(
        "$NODE_ASSET"
        --execute-workers-max-num 1 --prepare-workers-soft-max-num 1
        --prepare-workers-hard-max-num 1
        --chain "$PRIMARY_CHAIN" --name "$NODE_NAME"
        --bootnodes "$PRIMARY_BOOTNODE"
        --workers-path "$PVF_WORKERS_DIR"
        --no-mdns --node-key "$NODE_KEY" --no-telemetry --validator
        --insecure-validator-i-know-what-i-do --base-path "$PRIMARY_BASE"
        --no-hardware-benchmarks
        --listen-addr "/ip4/127.0.0.1/tcp/$PRIMARY_P2P/ws"
        --experimental-rpc-endpoint "listen-addr=127.0.0.1:$PRIMARY_RPC,methods=unsafe,cors=all"
        --prometheus-port "$PRIMARY_METRICS"
    )
elif [[ "$ROLE" == relay-b ]]; then
    expected=(
        "$NODE_ASSET"
        --execute-workers-max-num 1 --prepare-workers-soft-max-num 1
        --prepare-workers-hard-max-num 1
        --chain "$PRIMARY_CHAIN" --name "$NODE_NAME"
        --workers-path "$PVF_WORKERS_DIR"
        --no-mdns --node-key "$NODE_KEY" --no-telemetry --validator
        --insecure-validator-i-know-what-i-do --bootnodes "$PRIMARY_BOOTNODE"
        --base-path "$PRIMARY_BASE"
        --no-hardware-benchmarks
        --listen-addr "/ip4/127.0.0.1/tcp/$PRIMARY_P2P/ws"
        --experimental-rpc-endpoint "listen-addr=127.0.0.1:$PRIMARY_RPC,methods=unsafe,cors=all"
        --prometheus-port "$PRIMARY_METRICS"
    )
else
    expected=(
        "$NODE_ASSET"
        --name "$NODE_NAME" --node-key "$NODE_KEY" --chain "$PRIMARY_CHAIN"
        --base-path "$PRIMARY_BASE" --collator
        --blocks-pruning archive --state-pruning archive --bootnodes "$PRIMARY_BOOTNODE"
        --no-mdns --no-telemetry --no-hardware-benchmarks
        --listen-addr "/ip4/127.0.0.1/tcp/$PRIMARY_P2P/ws"
        --experimental-rpc-endpoint "listen-addr=127.0.0.1:$PRIMARY_RPC,methods=unsafe,cors=all"
        --prometheus-port "$PRIMARY_METRICS"
        --
        --base-path "$RELAY_BASE" --chain "$RELAY_CHAIN" --execution wasm
        --bootnodes "$RELAY_BOOTNODE"
        --workers-path "$PVF_WORKERS_DIR"
        --execute-workers-max-num 1 --prepare-workers-soft-max-num 1
        --prepare-workers-hard-max-num 1
        --no-mdns --no-telemetry --no-hardware-benchmarks
        --listen-addr "/ip4/127.0.0.1/tcp/$RELAY_P2P/ws"
        --experimental-rpc-endpoint "listen-addr=127.0.0.1:$RELAY_RPC,methods=unsafe,cors=all"
        --prometheus-port "$RELAY_METRICS"
    )
fi

[[ ${#NORMALIZED[@]} -eq ${#expected[@]} ]] || die 'normalized argv length drifted'
for index in "${!expected[@]}"; do
    [[ "${NORMALIZED[$index]}" == "${expected[$index]}" ]] ||
        die "normalized argv drifted at field $index"
done
recheck_generated_files

cleanup_vectors
trap - EXIT HUP INT TERM
if [[ $PRINT0 -eq 1 ]]; then
    printf '%s\0' "${NORMALIZED[@]}"
    exit 0
fi

exec "$ENV" -i CUBIKAN_NORMALIZER_SANITIZED=1 HOME=/home/charles \
    LC_ALL=C LANG=C TZ=UTC PATH=/usr/bin:/bin \
    /usr/bin/bash --noprofile --norc -p -c "$NORMALIZER_CONTENT" "$NORMALIZER" \
    __cubikan_normalizer_bound_memory_v1__ "$NORMALIZER" "$NORMALIZER_DIGEST" \
    "$NORMALIZER_CONTENT" --role "$ROLE" -- "$NODE_ASSET" "${RAW[@]}"
