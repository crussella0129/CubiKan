#!/usr/bin/bash -p
set -euo pipefail

readonly TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_ROOT="$(cd -- "$TOOL_DIR/../.." && pwd -P)"
readonly FIXTURE="$PROJECT_ROOT/tests/chain-e2e/journey-v1.json"
readonly DRIVER="$PROJECT_ROOT/tests/chain-e2e/driver.mjs"
readonly CONFIG="$PROJECT_ROOT/chain/config/zombienet.toml"
readonly MATERIALIZER="$TOOL_DIR/materialize-zombienet.sh"
readonly NODE_LAUNCHER="$TOOL_DIR/zombienet-node-launcher.sh"
readonly PARACHAIN_SPEC="$PROJECT_ROOT/chain/config/cubikan-local.json"
readonly POLKADOT="$PROJECT_ROOT/chain/.cache/downloads/polkadot"
readonly POLKADOT_PREPARE_WORKER="$PROJECT_ROOT/chain/.cache/downloads/polkadot-prepare-worker"
readonly POLKADOT_EXECUTE_WORKER="$PROJECT_ROOT/chain/.cache/downloads/polkadot-execute-worker"
readonly POLKADOT_PARACHAIN="$PROJECT_ROOT/chain/.cache/downloads/polkadot-parachain"
readonly POLKADOT_OMNI_NODE="$PROJECT_ROOT/chain/.cache/downloads/polkadot-omni-node"
readonly TOOLCHAIN_ROOT=/run/cubikan-exec/toolchain
readonly PVF_WORKERS_DIR=/run/cubikan-exec/pvf-workers
readonly STAT=/usr/lib/cargo/bin/coreutils/stat
readonly ENV=/usr/lib/cargo/bin/coreutils/env
readonly DD=/usr/lib/cargo/bin/coreutils/dd
readonly SHA256SUM=/usr/lib/cargo/bin/coreutils/sha256sum
readonly PYTHON=/usr/bin/python3.14
readonly REALPATH=/usr/lib/cargo/bin/coreutils/realpath
readonly SETSID=/usr/bin/setsid
readonly SETPRIV=/usr/bin/setpriv
readonly MKFIFO=/usr/lib/cargo/bin/coreutils/mkfifo
readonly MOUNT=/usr/bin/mount
readonly UMOUNT=/usr/bin/umount
readonly SLEEP=/usr/lib/cargo/bin/coreutils/sleep
readonly SUCCESS_LINE='verified cubikan four-node journey v1'
readonly DRIVER_OUTPUT_RETAIN_LIMIT=65536
readonly DRIVER_CAPTURE_PROGRAM='import os
import re
import sys

stream = sys.argv[1]
output_limit = int(sys.argv[2])
safe_prefix = b"run-four-node-journey: bounded driver stderr follows\n"
safe_suffix = b"run-four-node-journey: end bounded driver stderr\n"
shell_trailer_budget = 256
payload_limit = output_limit - len(safe_prefix) - len(safe_suffix) - shell_trailer_budget - 1
chunks = []
total = 0
overflowed = False

def write_all(fd, payload):
    remaining = memoryview(payload)
    while remaining:
        try:
            written = os.write(fd, remaining)
        except OSError:
            raise SystemExit(2)
        if written <= 0:
            raise SystemExit(2)
        remaining = remaining[written:]

while True:
    chunk = os.read(0, 65_536)
    if not chunk:
        break
    total += len(chunk)
    if total > payload_limit:
        overflowed = True
    elif not overflowed:
        chunks.append(chunk)
if overflowed:
    write_all(2, b"run-four-node-journey: driver output exceeded its bound\n")
    raise SystemExit(2)
data = b"".join(chunks)
if stream == "stdout":
    raise SystemExit(0 if not data else 3)
if stream != "stderr":
    write_all(2, b"run-four-node-journey: invalid driver capture stream\n")
    raise SystemExit(2)
if not data:
    raise SystemExit(0)
try:
    data.decode("ascii", "strict")
except UnicodeDecodeError:
    write_all(2, b"run-four-node-journey: bounded driver stderr rejected by scan\n")
    raise SystemExit(2)
if any(byte not in (0x09, 0x0A) and not 0x20 <= byte <= 0x7E for byte in data):
    write_all(2, b"run-four-node-journey: bounded driver stderr rejected by scan\n")
    raise SystemExit(2)
forbidden_environment = re.compile(
    rb"(?:^|[^A-Z0-9_])(?:AWS_SECRET_ACCESS_KEY|GIT_ASKPASS|GIT_CONFIG_GLOBAL|GIT_CONFIG_SYSTEM|SSH_AUTH_SOCK|HTTP_PROXY|HTTPS_PROXY|ALL_PROXY|NO_PROXY)=",
    re.IGNORECASE,
)
forbidden_secret = re.compile(
    rb"(?:^|[,{])[ \t\r\n]*\x22(?:credential|credentials|mnemonic|passphrase|password|private_key|private_locator|prompt|provider_secret|secret|seed|source_body|token|transcript)\x22[ \t\r\n]*:|(?:^|[^a-z0-9_])(?:credential|credentials|mnemonic|passphrase|password|private_key|private_locator|prompt|provider_secret|secret|seed|source_body|token|transcript)[ \t]*[:=]",
    re.IGNORECASE,
)
public_url = re.compile(rb"(?:git\+ssh|https?|wss?|ssh|git)://", re.IGNORECASE)
if forbidden_environment.search(data) or forbidden_secret.search(data) or public_url.search(data):
    write_all(2, b"run-four-node-journey: bounded driver stderr rejected by scan\n")
    raise SystemExit(2)
write_all(2, safe_prefix)
write_all(2, data)
if not data.endswith(b"\n"):
    write_all(2, b"\n")
write_all(2, safe_suffix)
raise SystemExit(3)'
readonly BOOTSTRAP_LOG_MAX_BYTES=1048576
readonly BOOTSTRAP_LOG_RETAIN_LIMIT=1048577

die() {
    printf 'run-four-node-journey: %s\n' "$*" >&2
    exit 1
}

usage() {
    die 'usage: run-four-node-journey.sh --fixture ABS --evidence ABS --work-root ABS'
}

canonical_existing_directory() {
    local path=$1 canonical
    [[ "$path" == /* && -d "$path" && ! -L "$path" ]] || return 1
    canonical="$($REALPATH -e -- "$path")" || return 1
    [[ "$canonical" == "$path" ]] || return 1
    printf '%s\n' "$canonical"
}

require_private_directory() {
    local path=$1 expected_mode=$2
    [[ "$($STAT -Lc '%u:%a:%F' -- "$path")" == "$EUID:$expected_mode:directory" ]] ||
        die "${path##*/} must be an owner-only directory with mode $expected_mode"
}

require_regular_file() {
    local path=$1 label=$2
    [[ -f "$path" && ! -L "$path" ]] || die "$label is missing or symbolic"
}

require_bounded_bootstrap_log() {
    local path=$1 label=$2 size
    [[ -f "$path" && ! -L "$path" &&
        "$($STAT -Lc '%u:%a:%h:%F' -- "$path")" == "$EUID:600:1:regular file" ]] ||
        die "$label is not an owner-only regular file"
    size="$($STAT -Lc '%s' -- "$path")"
    [[ "$size" =~ ^[0-9]+$ && $size -le $BOOTSTRAP_LOG_MAX_BYTES ]] ||
        die "$label exceeds its exact byte bound"
}

finish_bootstrap_log_capture() {
    local log=$1 pipe=$2 label=$3 reader_status=0
    wait "$bootstrap_reader_pid" || reader_status=$?
    bootstrap_reader_pid=''
    [[ $reader_status -eq 0 ]] || die "$label bounded reader failed"
    require_bounded_bootstrap_log "$log" "$label"
    [[ -p "$pipe" && ! -L "$pipe" ]] || die "$label capture pipe identity drifted"
    /usr/bin/gnurm -f -- "$pipe"
}

process_is_live() {
    local pid=$1 state
    [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 1
    state="$(/usr/bin/ps -o stat= -p "$pid" 2>/dev/null)" || return 1
    [[ -n "$state" && "${state:0:1}" != Z ]]
}

process_group_is_empty() {
    local group=$1 inventory count
    [[ "$group" =~ ^[1-9][0-9]*$ ]] || return 1
    inventory="$(/usr/bin/ps -e -o pgid=)" || return 1
    count="$(/usr/bin/gawk -v group="$group" \
        '$1 == group { count++ } END { print count + 0 }' <<<"$inventory")" || return 1
    [[ "$count" == 0 ]]
}

quiesce_driver_group() {
    local group=$1
    [[ "$group" =~ ^[1-9][0-9]*$ ]] || return 1
    /usr/bin/kill -TERM -- "-$group" 2>/dev/null || true
    for _ in {1..100}; do
        if process_group_is_empty "$group"; then
            return 0
        fi
        "$SLEEP" 0.05
    done
    /usr/bin/kill -KILL -- "-$group" 2>/dev/null || true
    for _ in {1..100}; do
        if process_group_is_empty "$group"; then
            return 0
        fi
        "$SLEEP" 0.05
    done
    return 1
}

run_unprivileged() {
    "$SETPRIV" --no-new-privs --inh-caps=-all --ambient-caps=-all \
        --bounding-set=-all -- "$@"
}

[[ $# -eq 6 && "$1" == --fixture && "$3" == --evidence && "$5" == --work-root ]] || usage
fixture_argument=$2
evidence_argument=$4
work_argument=$6
[[ "$fixture_argument" == "$FIXTURE" ]] || die 'fixture must be the exact repository journey fixture'
[[ "$evidence_argument" == /* && "$evidence_argument" != / &&
    "$evidence_argument" != */../* && "$evidence_argument" != */./* &&
    "$evidence_argument" != */.. && "$evidence_argument" != */. ]] ||
    die 'evidence path must be a normalized absolute path'
[[ "$work_argument" == /* && "$work_argument" != / &&
    "$work_argument" != */../* && "$work_argument" != */./* &&
    "$work_argument" != */.. && "$work_argument" != */. ]] ||
    die 'work root must be a normalized absolute path'
readonly EVIDENCE_PATH=$evidence_argument
readonly WORK_ROOT=$work_argument
unset fixture_argument evidence_argument work_argument

[[ "$(pwd -P)" == "$PROJECT_ROOT" ]] || die 'journey must start at the repository root'
require_regular_file "$FIXTURE" 'journey fixture'
require_regular_file "$DRIVER" 'journey driver'
require_regular_file "$CONFIG" 'Zombienet config'
require_regular_file "$MATERIALIZER" 'Zombienet materializer'
require_regular_file "$NODE_LAUNCHER" 'node launcher'
require_regular_file "$PARACHAIN_SPEC" 'parachain chain spec'
require_regular_file "$POLKADOT" 'relay executable'
require_regular_file "$POLKADOT_PREPARE_WORKER" 'relay prepare worker'
require_regular_file "$POLKADOT_EXECUTE_WORKER" 'relay execute worker'
require_regular_file "$POLKADOT_PARACHAIN" 'parachain utility executable'
require_regular_file "$POLKADOT_OMNI_NODE" 'collator executable'
require_regular_file "$SETPRIV" 'privilege-drop executable'
[[ -x "$MATERIALIZER" && -x "$NODE_LAUNCHER" && -x "$POLKADOT" &&
    -x "$POLKADOT_PREPARE_WORKER" && -x "$POLKADOT_EXECUTE_WORKER" &&
    -x "$POLKADOT_PARACHAIN" && -x "$POLKADOT_OMNI_NODE" && -x "$SETPRIV" ]] ||
    die 'one or more pinned journey executables is not executable'

supported_root=${CUBIKAN_TEST_SUPPORTED_ROOT:-}
[[ -n "$supported_root" ]] || die 'CUBIKAN_TEST_SUPPORTED_ROOT is required'
supported_root="$(canonical_existing_directory "$supported_root")" ||
    die 'supported root must be a canonical nonsymbolic directory'
readonly SUPPORTED_ROOT=$supported_root
unset supported_root
require_private_directory "$SUPPORTED_ROOT" 700
[[ "$($STAT -fLc '%t' -- "$SUPPORTED_ROOT")" == ef53 ]] ||
    die 'supported root must be on the declared ext4 filesystem'

local_binary=${CUBIKAN_LOCAL_TEST_BINARY:-}
[[ "$local_binary" == /* && -f "$local_binary" && -x "$local_binary" && ! -L "$local_binary" ]] ||
    die 'CUBIKAN_LOCAL_TEST_BINARY must be an absolute executable regular file'
local_binary="$($REALPATH -e -- "$local_binary")" || die 'cannot canonicalize local test binary'
case "$local_binary" in
    "$PROJECT_ROOT"/target/*/cubikan-local | "$PROJECT_ROOT"/target/*/deps/cubikan-local-*) ;;
    *) die 'local test binary is outside the repository target tree' ;;
esac
readonly LOCAL_BINARY=$local_binary
unset local_binary

mapfile -t inherited_cubikan_names < <(
    "$PYTHON" -I -S -c \
        'import os; print("\n".join(sorted(k for k in os.environ if k.startswith("CUBIKAN_"))))'
)
[[ ${#inherited_cubikan_names[@]} -eq 2 &&
    "${inherited_cubikan_names[0]}" == CUBIKAN_LOCAL_TEST_BINARY &&
    "${inherited_cubikan_names[1]}" == CUBIKAN_TEST_SUPPORTED_ROOT ]] ||
    die 'launcher inherited an unexpected CUBIKAN_* environment variable'
unset inherited_cubikan_names

evidence_parent=${EVIDENCE_PATH%/*}
work_parent=${WORK_ROOT%/*}
[[ -n "$evidence_parent" && -n "$work_parent" ]] || die 'journey paths require explicit parents'
evidence_parent="$(canonical_existing_directory "$evidence_parent")" ||
    die 'evidence parent must be a canonical nonsymbolic directory'
work_parent="$(canonical_existing_directory "$work_parent")" ||
    die 'work parent must be a canonical nonsymbolic directory'
[[ "$evidence_parent" == "$work_parent" && "$evidence_parent" == "$SUPPORTED_ROOT"/* ]] ||
    die 'evidence and work roots must share one session parent below the supported root'
require_private_directory "$evidence_parent" 700
session_name=${evidence_parent#"$SUPPORTED_ROOT"/}
[[ "$session_name" != */* && "$session_name" =~ ^cubikan-t1115-[1-9][0-9]*-[0-9]+-[0-9]+$ ]] ||
    die 'session parent does not use the closed T-1115 name grammar'
readonly SESSION_ROOT=$evidence_parent
unset evidence_parent work_parent session_name
readonly DRIVER_PHASE_PATH="$SESSION_ROOT/.driver.phase"
readonly DRIVER_PHASE_TEMP="$SESSION_ROOT/.driver.phase.tmp"
[[ "$EVIDENCE_PATH" == "$SESSION_ROOT/evidence-v1.json" &&
    "$WORK_ROOT" == "$SESSION_ROOT/work" ]] ||
    die 'session paths must use the exact evidence-v1.json and work names'
[[ ! -e "$EVIDENCE_PATH" && ! -L "$EVIDENCE_PATH" ]] ||
    die 'evidence path must not exist before launch'
[[ ! -e "$WORK_ROOT" && ! -L "$WORK_ROOT" ]] ||
    die 'work root must not exist before launch'
[[ ! -e "$DRIVER_PHASE_PATH" && ! -L "$DRIVER_PHASE_PATH" &&
    ! -e "$DRIVER_PHASE_TEMP" && ! -L "$DRIVER_PHASE_TEMP" ]] ||
    die 'driver phase marker paths must not exist before launch'

umask 077
/usr/lib/cargo/bin/coreutils/mkdir -m 0700 -- "$WORK_ROOT"
readonly BOOTSTRAP_ROOT="$WORK_ROOT/bootstrap"
/usr/lib/cargo/bin/coreutils/mkdir -m 0700 -- "$BOOTSTRAP_ROOT"
readonly MATERIALIZER_LOG="$BOOTSTRAP_ROOT/materializer.log"
readonly MATERIALIZER_LOG_PIPE="$BOOTSTRAP_ROOT/materializer.log.pipe"
readonly GENESIS_HEAD="$BOOTSTRAP_ROOT/parachain-genesis-head"
readonly GENESIS_WASM="$BOOTSTRAP_ROOT/parachain-genesis-wasm"
readonly GENESIS_LOG="$BOOTSTRAP_ROOT/genesis-export.log"
readonly GENESIS_LOG_PIPE="$BOOTSTRAP_ROOT/genesis-export.log.pipe"

toolchain_created=0
toolchain_mounted=0
toolchain_root_identity=''
pvf_workers_created=0
pvf_workers_mounted=0

verify_read_only_toolchain() {
    local phase=$1 actual_identity mount_proof probe_status
    [[ "$phase" == before-driver || "$phase" == after-driver ]] ||
        die 'internal unsupported toolchain verification phase'
    [[ -d "$TOOLCHAIN_ROOT" && ! -L "$TOOLCHAIN_ROOT" &&
        "$($REALPATH -e -- "$TOOLCHAIN_ROOT")" == "$TOOLCHAIN_ROOT" ]] ||
        die "materialized toolchain root is invalid $phase"
    actual_identity="$($STAT -Lc '%d:%i:%u:%a:%F' -- "$TOOLCHAIN_ROOT")"
    [[ -n "$toolchain_root_identity" && "$actual_identity" == "$toolchain_root_identity" &&
        "${actual_identity#*:*:}" == "$EUID:700:directory" ]] ||
        die "materialized toolchain root identity drifted $phase"
    [[ "$($STAT -fLc '%t' -- "$TOOLCHAIN_ROOT")" == 1021994 ]] ||
        die "materialized toolchain is not on the private tmpfs $phase"
    mount_proof="$(/usr/bin/gawk -v target="$TOOLCHAIN_ROOT" '
        $5 == target {
            count++
            split($6, options, ",")
            delete seen
            for (option in options) seen[options[option]] = 1
            dash = 0
            for (i = 7; i <= NF; i++) if ($i == "-") { dash = i; break }
            if ($4 != "/" && seen["ro"] && seen["nodev"] && seen["nosuid"] &&
                !seen["noexec"] && dash > 0 && $(dash + 1) == "tmpfs") ok++
        }
        END { printf "%d:%d\n", count + 0, ok + 0 }
    ' /proc/self/mountinfo)"
    [[ "$mount_proof" == 1:1 ]] ||
        die "materialized toolchain mount is not one exact executable read-only tmpfs bind $phase"
    probe_status=0
    "$PYTHON" -I -S -c \
        'import errno, os, sys
p = sys.argv[1]
try:
    fd = os.open(p, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
except OSError as error:
    raise SystemExit(0 if error.errno == errno.EROFS else 1)
else:
    os.close(fd)
    os.unlink(p)
    raise SystemExit(2)' \
        "$TOOLCHAIN_ROOT/.write-probe" || probe_status=$?
    [[ $probe_status -eq 0 && ! -e "$TOOLCHAIN_ROOT/.write-probe" ]] ||
        die "materialized toolchain did not reject creation with EROFS $phase"
}

seal_read_only_toolchain() {
    [[ "$($STAT -fLc '%T' -- /run/cubikan-exec)" == tmpfs &&
        "$($STAT -Lc '%a' -- /run/cubikan-exec)" == 700 &&
        -d "$TOOLCHAIN_ROOT" && ! -L "$TOOLCHAIN_ROOT" &&
        "$($REALPATH -e -- "$TOOLCHAIN_ROOT")" == "$TOOLCHAIN_ROOT" ]] ||
        die 'private executable tmpfs is unavailable for the materialized toolchain'
    toolchain_root_identity="$($STAT -Lc '%d:%i:%u:%a:%F' -- "$TOOLCHAIN_ROOT")"
    [[ "${toolchain_root_identity#*:*:}" == "$EUID:700:directory" ]] ||
        die 'materialized toolchain root metadata is invalid before sealing'
    toolchain_mounted=2
    "$MOUNT" --bind "$TOOLCHAIN_ROOT" "$TOOLCHAIN_ROOT"
    toolchain_mounted=1
    "$MOUNT" -o remount,bind,ro,nodev,nosuid "$TOOLCHAIN_ROOT"
    verify_read_only_toolchain before-driver
}

copy_verified_pvf_worker() {
    local source=$1 destination=$2 expected_size=$3 expected_hash=$4
    [[ ! -e "$destination" && ! -L "$destination" ]] ||
        die 'PVF worker destination already exists'
    if ! "$PYTHON" -I -S - "$source" "$destination" "$expected_size" "$expected_hash" <<'PY'
import hashlib
import os
import stat
import sys

source, destination, expected_size_text, expected_hash = sys.argv[1:]
expected_size = int(expected_size_text)
identity_fields = (
    "st_dev", "st_ino", "st_mode", "st_nlink", "st_uid", "st_gid",
    "st_size", "st_mtime_ns", "st_ctime_ns",
)

def identity(value):
    return tuple(getattr(value, field) for field in identity_fields)

source_flags = os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK
destination_flags = (
    os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW
)
source_fd = os.open(source, source_flags)
destination_fd = -1
try:
    opened = os.fstat(source_fd)
    path_before = os.lstat(source)
    if (
        identity(opened) != identity(path_before)
        or not stat.S_ISREG(opened.st_mode)
        or opened.st_nlink != 1
        or opened.st_uid != os.geteuid()
        or stat.S_IMODE(opened.st_mode) & 0o022 != 0
        or stat.S_IMODE(opened.st_mode) & 0o111 != 0o111
        or opened.st_size != expected_size
    ):
        raise RuntimeError("source identity")
    destination_fd = os.open(destination, destination_flags, 0o500)
    digest = hashlib.sha256()
    total = 0
    while True:
        chunk = os.read(source_fd, min(1024 * 1024, expected_size + 1 - total))
        if not chunk:
            break
        total += len(chunk)
        if total > expected_size:
            raise RuntimeError("source bound")
        digest.update(chunk)
        view = memoryview(chunk)
        while view:
            written = os.write(destination_fd, view)
            if written <= 0:
                raise RuntimeError("destination write")
            view = view[written:]
    if total != expected_size or digest.hexdigest() != expected_hash:
        raise RuntimeError("source content")
    os.fchmod(destination_fd, 0o500)
    os.fsync(destination_fd)
    copied = os.fstat(destination_fd)
    source_after = os.fstat(source_fd)
    path_after = os.lstat(source)
    if (
        identity(source_after) != identity(opened)
        or identity(path_after) != identity(opened)
        or not stat.S_ISREG(copied.st_mode)
        or copied.st_nlink != 1
        or copied.st_uid != os.geteuid()
        or stat.S_IMODE(copied.st_mode) != 0o500
        or copied.st_size != expected_size
    ):
        raise RuntimeError("post-copy identity")
finally:
    if destination_fd >= 0:
        os.close(destination_fd)
    os.close(source_fd)
PY
    then
        die 'same-descriptor PVF worker copy failed'
    fi
}

materialize_read_only_pvf_workers() {
    local mount_proof probe_status
    local -a worker_entries
    [[ "$($STAT -fLc '%T' -- /run/cubikan-exec)" == tmpfs &&
        "$($STAT -Lc '%a' -- /run/cubikan-exec)" == 700 &&
        ! -e "$PVF_WORKERS_DIR" && ! -L "$PVF_WORKERS_DIR" ]] ||
        die 'private executable tmpfs is unavailable for PVF workers'
    pvf_workers_created=2
    /usr/lib/cargo/bin/coreutils/mkdir -m 0700 -- "$PVF_WORKERS_DIR"
    pvf_workers_created=1
    copy_verified_pvf_worker "$POLKADOT_PREPARE_WORKER" \
        "$PVF_WORKERS_DIR/polkadot-prepare-worker" \
        21387400 5e67a05516e24d5e9b9616bacb3a2d58235beb3392de14dfbe51ff6914244267
    copy_verified_pvf_worker "$POLKADOT_EXECUTE_WORKER" \
        "$PVF_WORKERS_DIR/polkadot-execute-worker" \
        19463336 cc642041ef2582d972071cd4f7122e9803703bc7775e8d432b2d7626f5011b21
    shopt -s dotglob nullglob
    worker_entries=("$PVF_WORKERS_DIR"/*)
    shopt -u dotglob nullglob
    [[ ${#worker_entries[@]} -eq 2 &&
        "${worker_entries[0]}" == "$PVF_WORKERS_DIR/polkadot-execute-worker" &&
        "${worker_entries[1]}" == "$PVF_WORKERS_DIR/polkadot-prepare-worker" ]] ||
        die 'private PVF worker directory has a missing or extra entry'
    /usr/lib/cargo/bin/coreutils/chmod 0500 -- "$PVF_WORKERS_DIR"
    pvf_workers_mounted=2
    "$MOUNT" --bind "$PVF_WORKERS_DIR" "$PVF_WORKERS_DIR"
    pvf_workers_mounted=1
    "$MOUNT" -o remount,bind,ro,nodev,nosuid "$PVF_WORKERS_DIR"
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
    [[ "$mount_proof" == 1:1 ]] || die 'PVF worker mount is not one exact executable read-only tmpfs bind'
    probe_status=0
    "$PYTHON" -I -S -c \
        'import errno, os, sys
p = sys.argv[1]
try:
    fd = os.open(p, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
except OSError as error:
    raise SystemExit(0 if error.errno == errno.EROFS else 1)
else:
    os.close(fd)
    os.unlink(p)
    raise SystemExit(2)' \
        "$PVF_WORKERS_DIR/.write-probe" || probe_status=$?
    [[ $probe_status -eq 0 && ! -e "$PVF_WORKERS_DIR/.write-probe" ]] ||
        die 'PVF worker mount did not reject creation with EROFS'
    [[ "$($STAT -Lc '%u:%a:%F' -- "$PVF_WORKERS_DIR")" == "$EUID:500:directory" ]] ||
        die 'PVF worker mount root metadata drifted'
    [[ "$($SHA256SUM -- "$PVF_WORKERS_DIR/polkadot-prepare-worker")" == \
        '5e67a05516e24d5e9b9616bacb3a2d58235beb3392de14dfbe51ff6914244267  /run/cubikan-exec/pvf-workers/polkadot-prepare-worker' &&
        "$($SHA256SUM -- "$PVF_WORKERS_DIR/polkadot-execute-worker")" == \
        'cc642041ef2582d972071cd4f7122e9803703bc7775e8d432b2d7626f5011b21  /run/cubikan-exec/pvf-workers/polkadot-execute-worker' ]] ||
        die 'PVF worker bytes drifted after read-only remount'
}

driver_pid=''
driver_group=''
bootstrap_reader_pid=''
stdout_reader_pid=''
stderr_reader_pid=''
journey_succeeded=0
launcher_phase=preflight
cleanup() {
    local status=$? toolchain_mount_count worker_mount_count driver_phase phase_size
    trap - EXIT HUP INT TERM
    if ((journey_succeeded == 0)); then
        case "${launcher_phase:-}" in
            preflight|toolchain-materialization|genesis-export|driver|post-driver)
                printf 'run-four-node-journey: launcher-phase=%s\n' "$launcher_phase" >&2
                ;;
            *) printf 'run-four-node-journey: launcher-phase=unavailable\n' >&2 ;;
        esac
    fi
    if [[ -n "${driver_group:-}" && "$driver_group" =~ ^[1-9][0-9]*$ ]]; then
        /usr/bin/kill -TERM -- "-$driver_group" 2>/dev/null || true
        for _ in {1..100}; do
            if process_group_is_empty "$driver_group"; then
                break
            fi
            "$SLEEP" 0.05
        done
        if ! process_group_is_empty "$driver_group"; then
            /usr/bin/kill -KILL -- "-$driver_group" 2>/dev/null || true
            for _ in {1..100}; do
                if process_group_is_empty "$driver_group"; then
                    break
                fi
                "$SLEEP" 0.05
            done
        fi
        if ! process_group_is_empty "$driver_group"; then
            printf 'run-four-node-journey: driver process-group cleanup census failed\n' >&2
            status=1
        fi
    fi
    if [[ -n "${driver_pid:-}" && "$driver_pid" =~ ^[1-9][0-9]*$ ]]; then
        wait "$driver_pid" 2>/dev/null || true
    fi
    if ((journey_succeeded == 0)) &&
        [[ "${launcher_phase:-}" == driver || "${launcher_phase:-}" == post-driver ]]; then
        driver_phase=''
        if [[ -f "$DRIVER_PHASE_PATH" && ! -L "$DRIVER_PHASE_PATH" &&
            "$($STAT -Lc '%u:%a:%h:%F' -- "$DRIVER_PHASE_PATH")" == \
                "$EUID:600:1:regular file" ]]; then
            phase_size="$($STAT -Lc '%s' -- "$DRIVER_PHASE_PATH")"
            if [[ "$phase_size" =~ ^[1-9][0-9]*$ && $phase_size -le 64 ]]; then
                driver_phase="$(<"$DRIVER_PHASE_PATH")"
            fi
        fi
        case "$driver_phase" in
            module-imports|zombienet-start|api-connect|node-evidence|normalizer-matrix|pre-mutation-readiness|initial-submissions|initial-submission-m0[1-7]|collator-stop|survivor-submissions|survivor-submission-m0[8-9]|survivor-submission-m1[0-9]|survivor-submission-m2[0-1]|finality-stability|collator-restart-spawn|collator-restart-native-readiness|collator-restart-primary-listener|collator-restart-api-reconnect|collator-restart-relay-api|collator-restart-convergence|collator-restart-evidence|archive-and-observations|parachain-census|projection-snapshot-uninterrupted|projection-snapshot-fresh-pair|projection-snapshot-rebuild-a|projection-snapshot-rebuild-b|relay-resume|relay-census|network-stop|post-stop-verification|audit-materialization|evidence-publication|complete)
                printf 'run-four-node-journey: driver-phase=%s\n' "$driver_phase" >&2
                ;;
            *) printf 'run-four-node-journey: driver-phase=unavailable\n' >&2 ;;
        esac
    fi
    for _ in {1..100}; do
        if ! process_is_live "${bootstrap_reader_pid:-}" &&
            ! process_is_live "${stdout_reader_pid:-}" &&
            ! process_is_live "${stderr_reader_pid:-}"; then
            break
        fi
        "$SLEEP" 0.01
    done
    for reader_pid in "${bootstrap_reader_pid:-}" \
        "${stdout_reader_pid:-}" "${stderr_reader_pid:-}"; do
        if [[ "$reader_pid" =~ ^[1-9][0-9]*$ ]]; then
            if process_is_live "$reader_pid"; then
                /usr/bin/kill -KILL -- "$reader_pid" 2>/dev/null || true
            fi
            wait "$reader_pid" 2>/dev/null || true
        fi
    done
    if ((toolchain_mounted != 0)); then
        toolchain_mount_count="$(/usr/bin/gawk -v target="$TOOLCHAIN_ROOT" \
            '$5 == target { count++ } END { print count + 0 }' /proc/self/mountinfo)"
        if [[ "$toolchain_mount_count" == 1 ]]; then
            if "$UMOUNT" -- "$TOOLCHAIN_ROOT"; then
                toolchain_mounted=0
            else
                printf 'run-four-node-journey: could not unmount the materialized toolchain\n' >&2
                status=1
            fi
        elif [[ "$toolchain_mount_count" == 0 && $toolchain_mounted -eq 2 ]]; then
            toolchain_mounted=0
        else
            printf 'run-four-node-journey: toolchain mount inventory drifted during cleanup\n' >&2
            status=1
        fi
    fi
    if ((toolchain_created != 0 && toolchain_mounted == 0)); then
        if [[ "$TOOLCHAIN_ROOT" == /run/cubikan-exec/toolchain &&
            -d "$TOOLCHAIN_ROOT" && ! -L "$TOOLCHAIN_ROOT" &&
            "$($REALPATH -e -- "$TOOLCHAIN_ROOT")" == "$TOOLCHAIN_ROOT" ]]; then
            /usr/bin/gnurm -rf -- "$TOOLCHAIN_ROOT" || status=1
            [[ ! -e "$TOOLCHAIN_ROOT" && ! -L "$TOOLCHAIN_ROOT" ]] || status=1
            toolchain_created=0
        elif [[ ! -e "$TOOLCHAIN_ROOT" && ! -L "$TOOLCHAIN_ROOT" &&
            $toolchain_created -eq 2 ]]; then
            toolchain_created=0
        else
            printf 'run-four-node-journey: refusing unsafe toolchain cleanup\n' >&2
            status=1
        fi
    fi
    if ((pvf_workers_mounted != 0)); then
        worker_mount_count="$(/usr/bin/gawk -v target="$PVF_WORKERS_DIR" \
            '$5 == target { count++ } END { print count + 0 }' /proc/self/mountinfo)"
        if [[ "$worker_mount_count" == 1 ]]; then
            if "$UMOUNT" -- "$PVF_WORKERS_DIR"; then
                pvf_workers_mounted=0
            else
                printf 'run-four-node-journey: could not unmount the PVF worker tree\n' >&2
                status=1
            fi
        elif [[ "$worker_mount_count" == 0 && $pvf_workers_mounted -eq 2 ]]; then
            pvf_workers_mounted=0
        else
            printf 'run-four-node-journey: PVF worker mount inventory drifted during cleanup\n' >&2
            status=1
        fi
    fi
    if ((pvf_workers_created != 0 && pvf_workers_mounted == 0)); then
        if [[ -d "$PVF_WORKERS_DIR" && ! -L "$PVF_WORKERS_DIR" &&
            "$(/usr/lib/cargo/bin/coreutils/realpath -e -- "$PVF_WORKERS_DIR")" == "$PVF_WORKERS_DIR" ]]; then
            /usr/lib/cargo/bin/coreutils/chmod 0700 -- "$PVF_WORKERS_DIR" || status=1
            /usr/bin/gnurm -rf -- "$PVF_WORKERS_DIR" || status=1
            [[ ! -e "$PVF_WORKERS_DIR" && ! -L "$PVF_WORKERS_DIR" ]] || status=1
            pvf_workers_created=0
        elif [[ ! -e "$PVF_WORKERS_DIR" && ! -L "$PVF_WORKERS_DIR" &&
            $pvf_workers_created -eq 2 ]]; then
            pvf_workers_created=0
        else
            printf 'run-four-node-journey: refusing unsafe PVF worker cleanup\n' >&2
            status=1
        fi
    fi
    if [[ -e "${WORK_ROOT:-}" || -L "${WORK_ROOT:-}" ]]; then
        if [[ "$WORK_ROOT" == "$SESSION_ROOT/work" &&
            -d "$WORK_ROOT" && ! -L "$WORK_ROOT" ]]; then
            /usr/bin/gnurm -rf -- "$WORK_ROOT"
        else
            printf 'run-four-node-journey: refusing unsafe work-root cleanup\n' >&2
            status=1
        fi
    fi
    if ((journey_succeeded == 0)) && [[ -e "${EVIDENCE_PATH:-}" || -L "${EVIDENCE_PATH:-}" ]]; then
        if [[ "$EVIDENCE_PATH" == "$SESSION_ROOT/evidence-v1.json" &&
            -f "$EVIDENCE_PATH" && ! -L "$EVIDENCE_PATH" ]]; then
            /usr/bin/gnurm -f -- "$EVIDENCE_PATH"
        else
            printf 'run-four-node-journey: refusing unsafe evidence cleanup\n' >&2
            status=1
        fi
    fi
    for phase_capture in "$DRIVER_PHASE_PATH" "$DRIVER_PHASE_TEMP"; do
        if [[ -f "$phase_capture" && ! -L "$phase_capture" &&
            "$($STAT -Lc '%u:%a:%h:%F' -- "$phase_capture")" == \
                "$EUID:600:1:regular file" ]]; then
            /usr/bin/gnurm -f -- "$phase_capture"
        elif [[ -e "$phase_capture" || -L "$phase_capture" ]]; then
            printf 'run-four-node-journey: refusing unsafe driver-phase cleanup\n' >&2
            status=1
        fi
    done
    for capture in \
        "$SESSION_ROOT/.driver.stdout.pipe" "$SESSION_ROOT/.driver.stderr.pipe"; do
        if [[ -p "$capture" && ! -L "$capture" ]]; then
            /usr/bin/gnurm -f -- "$capture"
        elif [[ -e "$capture" || -L "$capture" ]]; then
            printf 'run-four-node-journey: refusing unsafe driver-capture cleanup\n' >&2
            status=1
        fi
    done
    exit "$status"
}
terminate() {
    trap - HUP INT TERM
    exit 143
}
trap cleanup EXIT
trap terminate HUP INT TERM

[[ "$($STAT -fLc '%T' -- /run/cubikan-exec)" == tmpfs &&
    "$($STAT -Lc '%a' -- /run/cubikan-exec)" == 700 &&
    ! -e "$TOOLCHAIN_ROOT" && ! -L "$TOOLCHAIN_ROOT" ]] ||
    die 'private executable tmpfs is unavailable for a fresh materialized toolchain'
launcher_phase=toolchain-materialization
toolchain_created=2
/usr/lib/cargo/bin/coreutils/mkdir -m 0700 -- "$TOOLCHAIN_ROOT"
toolchain_created=1
"$MKFIFO" -m 0600 -- "$MATERIALIZER_LOG_PIPE"
"$DD" if="$MATERIALIZER_LOG_PIPE" of="$MATERIALIZER_LOG" \
    bs="$BOOTSTRAP_LOG_RETAIN_LIMIT" count=1 iflag=fullblock status=none &
bootstrap_reader_pid=$!
materializer_status=0
materialized_cli="$({
    printf 'exec: %s --output-dir %s\n' "$MATERIALIZER" "$TOOLCHAIN_ROOT" >&2
    run_unprivileged "$MATERIALIZER" --output-dir "$TOOLCHAIN_ROOT"
} 2>"$MATERIALIZER_LOG_PIPE")" || materializer_status=$?
finish_bootstrap_log_capture \
    "$MATERIALIZER_LOG" "$MATERIALIZER_LOG_PIPE" 'materializer log'
[[ $materializer_status -eq 0 ]] || die 'offline Zombienet materialization failed'
unset materializer_status
readonly ZOMBIENET_ROOT="$TOOLCHAIN_ROOT/zombienet"
readonly PINNED_NODE="$TOOLCHAIN_ROOT/node/bin/node"
readonly EXPECTED_CLI="$ZOMBIENET_ROOT/javascript/packages/cli/dist/cli.js"
[[ "$materialized_cli" == "$EXPECTED_CLI" && -f "$EXPECTED_CLI" && ! -L "$EXPECTED_CLI" &&
    -x "$PINNED_NODE" && -f "$PINNED_NODE" && ! -L "$PINNED_NODE" ]] ||
    die 'materializer returned an unexpected pinned tool layout'
unset materialized_cli
seal_read_only_toolchain

materialize_read_only_pvf_workers

launcher_phase=genesis-export
"$MKFIFO" -m 0600 -- "$GENESIS_LOG_PIPE"
"$DD" if="$GENESIS_LOG_PIPE" of="$GENESIS_LOG" \
    bs="$BOOTSTRAP_LOG_RETAIN_LIMIT" count=1 iflag=fullblock status=none &
bootstrap_reader_pid=$!
genesis_status=0
{
    printf 'exec: %s export-genesis-head --chain %s %s\n' \
        "$POLKADOT_PARACHAIN" "$PARACHAIN_SPEC" "$GENESIS_HEAD" &&
        run_unprivileged "$ENV" -i HOME=/home/charles LC_ALL=C LANG=C TZ=UTC \
            PATH=/usr/bin:/bin "$POLKADOT_PARACHAIN" export-genesis-head \
            --chain "$PARACHAIN_SPEC" "$GENESIS_HEAD" &&
        printf 'exec: %s export-genesis-wasm --chain %s %s\n' \
            "$POLKADOT_PARACHAIN" "$PARACHAIN_SPEC" "$GENESIS_WASM" &&
        run_unprivileged "$ENV" -i HOME=/home/charles LC_ALL=C LANG=C TZ=UTC \
            PATH=/usr/bin:/bin "$POLKADOT_PARACHAIN" export-genesis-wasm \
            --chain "$PARACHAIN_SPEC" "$GENESIS_WASM"
} >"$GENESIS_LOG_PIPE" 2>&1 || genesis_status=$?
finish_bootstrap_log_capture "$GENESIS_LOG" "$GENESIS_LOG_PIPE" 'genesis export log'
[[ $genesis_status -eq 0 ]] || die 'parachain genesis export failed'
unset genesis_status
[[ -s "$GENESIS_HEAD" && -f "$GENESIS_HEAD" && ! -L "$GENESIS_HEAD" &&
    -s "$GENESIS_WASM" && -f "$GENESIS_WASM" && ! -L "$GENESIS_WASM" ]] ||
    die 'parachain genesis exports are missing or invalid'

readonly NETWORK_ROOT="$WORK_ROOT/network"
readonly DRIVER_STDOUT_PIPE="$SESSION_ROOT/.driver.stdout.pipe"
readonly DRIVER_STDERR_PIPE="$SESSION_ROOT/.driver.stderr.pipe"
"$MKFIFO" -m 0600 -- "$DRIVER_STDOUT_PIPE" "$DRIVER_STDERR_PIPE"
"$SETPRIV" --no-new-privs --inh-caps=-all --ambient-caps=-all \
    --bounding-set=-all -- "$PYTHON" -I -S -c "$DRIVER_CAPTURE_PROGRAM" \
    stdout "$DRIVER_OUTPUT_RETAIN_LIMIT" <"$DRIVER_STDOUT_PIPE" &
stdout_reader_pid=$!
"$SETPRIV" --no-new-privs --inh-caps=-all --ambient-caps=-all \
    --bounding-set=-all -- "$PYTHON" -I -S -c "$DRIVER_CAPTURE_PROGRAM" \
    stderr "$DRIVER_OUTPUT_RETAIN_LIMIT" <"$DRIVER_STDERR_PIPE" &
stderr_reader_pid=$!

launcher_phase=driver
"$SETSID" "$SETPRIV" --no-new-privs --inh-caps=-all --ambient-caps=-all \
    --bounding-set=-all -- \
    "$ENV" -i HOME=/home/charles LANG=C LC_ALL=C PATH=/usr/bin:/bin \
    PWD="$PROJECT_ROOT" SHLVL=0 TZ=UTC \
    CUBIKAN_LOCAL_TEST_BINARY="$LOCAL_BINARY" \
    CUBIKAN_TEST_SUPPORTED_ROOT="$SUPPORTED_ROOT" \
    CUBIKAN_ZOMBIENET_RUN_DIR="$NETWORK_ROOT" \
    CUBIKAN_ZOMBIENET_NODE_LAUNCHER="$NODE_LAUNCHER" \
    CUBIKAN_POLKADOT="$POLKADOT" \
    CUBIKAN_POLKADOT_WORKERS_DIR="$PVF_WORKERS_DIR" \
    CUBIKAN_POLKADOT_OMNI_NODE="$POLKADOT_OMNI_NODE" \
    CUBIKAN_POLKADOT_PARACHAIN="$POLKADOT_PARACHAIN" \
    CUBIKAN_PARACHAIN_CHAIN_SPEC="$PARACHAIN_SPEC" \
    CUBIKAN_PARACHAIN_GENESIS_HEAD="$GENESIS_HEAD" \
    CUBIKAN_PARACHAIN_GENESIS_WASM="$GENESIS_WASM" \
    "$PINNED_NODE" "$DRIVER" \
    --fixture "$FIXTURE" \
    --evidence "$EVIDENCE_PATH" \
    --work-root "$WORK_ROOT" \
    --config "$CONFIG" \
    --zombienet-root "$ZOMBIENET_ROOT" \
    >"$DRIVER_STDOUT_PIPE" 2>"$DRIVER_STDERR_PIPE" &
driver_pid=$!
for _ in {1..100}; do
    if [[ ! -e "/proc/$driver_pid/stat" ]]; then
        break
    fi
    read -r observed_group observed_session < <(
        /usr/bin/ps -o pgid=,sid= -p "$driver_pid"
    ) || true
    if [[ "${observed_group:-}" == "$driver_pid" && "${observed_session:-}" == "$driver_pid" ]]; then
        driver_group=$driver_pid
        break
    fi
    "$SLEEP" 0.01
done
[[ "$driver_group" == "$driver_pid" ]] || die 'setsid did not create the driver-owned session'

driver_status=0
wait "$driver_pid" || driver_status=$?
driver_pid=''
for _ in {1..100}; do
    if ! process_is_live "$stdout_reader_pid" &&
        ! process_is_live "$stderr_reader_pid"; then
        break
    fi
    "$SLEEP" 0.05
done
if process_is_live "$stdout_reader_pid" || process_is_live "$stderr_reader_pid"; then
    die 'journey descendant retained a bounded output pipe after driver exit'
fi
stdout_reader_status=0
stderr_reader_status=0
wait "$stdout_reader_pid" || stdout_reader_status=$?
wait "$stderr_reader_pid" || stderr_reader_status=$?
stdout_reader_pid=''
stderr_reader_pid=''
if [[ $driver_status -ne 0 ]]; then
    quiesce_driver_group "$driver_group" ||
        die 'journey driver failure group did not quiesce'
    driver_group=''
    [[ $stdout_reader_status -eq 0 ]] ||
        die 'journey driver wrote unexpected stdout or its scanner failed'
    [[ $stderr_reader_status -eq 0 || $stderr_reader_status -eq 2 ||
        $stderr_reader_status -eq 3 ]] ||
        die 'bounded journey stderr scanner failed'
    die 'four-node journey driver failed'
fi
if ! process_group_is_empty "$driver_group"; then
    die 'journey driver exited with process-group members or an invalid census'
fi
driver_group=''
[[ $stdout_reader_status -eq 0 && $stderr_reader_status -eq 0 ]] ||
    die 'journey driver wrote unexpected process output or its scanner failed'
launcher_phase=post-driver
verify_read_only_toolchain after-driver

[[ -p "$DRIVER_STDOUT_PIPE" && ! -L "$DRIVER_STDOUT_PIPE" &&
    -p "$DRIVER_STDERR_PIPE" && ! -L "$DRIVER_STDERR_PIPE" ]] ||
    die 'journey driver capture pipe identity drifted'
/usr/bin/gnurm -f -- "$DRIVER_STDOUT_PIPE" "$DRIVER_STDERR_PIPE"
[[ ! -e "$WORK_ROOT" && ! -L "$WORK_ROOT" ]] ||
    die 'journey driver did not remove its complete work root'
[[ -f "$EVIDENCE_PATH" && ! -L "$EVIDENCE_PATH" &&
    "$($STAT -Lc '%u:%a:%F' -- "$EVIDENCE_PATH")" == "$EUID:600:regular file" ]] ||
    die 'journey evidence was not atomically published as an owner-only regular file'
[[ -f "$DRIVER_PHASE_PATH" && ! -L "$DRIVER_PHASE_PATH" &&
    "$($STAT -Lc '%u:%a:%h:%F' -- "$DRIVER_PHASE_PATH")" == \
        "$EUID:600:1:regular file" &&
    ! -e "$DRIVER_PHASE_TEMP" && ! -L "$DRIVER_PHASE_TEMP" &&
    "$(<"$DRIVER_PHASE_PATH")" == complete ]] ||
    die 'driver did not publish the exact complete phase marker'
/usr/bin/gnurm -f -- "$DRIVER_PHASE_PATH"
[[ ! -e "$DRIVER_PHASE_PATH" && ! -L "$DRIVER_PHASE_PATH" ]] ||
    die 'driver phase marker cleanup failed'

journey_succeeded=1
launcher_phase=complete
trap - HUP INT TERM
printf '%s\n' "$SUCCESS_LINE"
