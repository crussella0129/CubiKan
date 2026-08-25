#!/usr/bin/bash -p

# Freeze every continuation in a Bash string before later execution. The path
# is only a repository-location hint; the reviewed behavior is in memory.
readonly NORMALIZER_BOUND_TOKEN=__cubikan_normalizer_bound_memory_v1__
if [[ "${1:-}" == "$NORMALIZER_BOUND_TOKEN" ]]; then
    [[ $# -ge 4 && "$2" == /* && -z "${BASH_SOURCE[0]}" &&
        "$3" =~ ^[0-9a-f]{64}$ && -n "$4" ]] || {
        builtin printf '%s\n' 'normalize-node-argv: invalid bound-memory entry' >&2
        builtin exit 126
    }
    normalizer_self_hint=$2
    normalizer_content_sha256=$3
    normalizer_content=$4
    shift 4
    normalizer_bound_hash="$(builtin printf '%s' "$normalizer_content" | /usr/lib/cargo/bin/coreutils/sha256sum)" || builtin exit 126
    normalizer_bound_hash=${normalizer_bound_hash%% *}
    [[ "$normalizer_bound_hash" == "$normalizer_content_sha256" ]] || {
        builtin printf '%s\n' 'normalize-node-argv: bound-memory identity mismatch' >&2
        builtin exit 126
    }
    builtin unset normalizer_bound_hash normalizer_content_sha256 normalizer_content
else
    case "${BASH_SOURCE[0]}" in
        /*) normalizer_self_hint="${BASH_SOURCE[0]}" ;;
        *) normalizer_self_hint="$(builtin pwd -P)/${BASH_SOURCE[0]}" ;;
    esac
    [[ "$normalizer_self_hint" == */chain/tools/normalize-node-argv.sh &&
        -f "$normalizer_self_hint" && ! -L "$normalizer_self_hint" ]] || {
        builtin printf '%s\n' 'normalize-node-argv: initial script path or descriptor is invalid' >&2
        builtin exit 126
    }
    normalizer_initial_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s:%Y:%Z' -- "$normalizer_self_hint")" || builtin exit 126
    normalizer_content=''
    IFS= builtin read -r -d '' normalizer_content <"$normalizer_self_hint" || [[ -n "$normalizer_content" ]] || builtin exit 126
    normalizer_content_sha256="$(builtin printf '%s' "$normalizer_content" | /usr/lib/cargo/bin/coreutils/sha256sum)" || builtin exit 126
    normalizer_content_sha256=${normalizer_content_sha256%% *}
    normalizer_compare_content=''
    IFS= builtin read -r -d '' normalizer_compare_content <"$normalizer_self_hint" || [[ -n "$normalizer_compare_content" ]] || builtin exit 126
    normalizer_compare_hash="$(builtin printf '%s' "$normalizer_compare_content" | /usr/lib/cargo/bin/coreutils/sha256sum)" || builtin exit 126
    normalizer_compare_hash=${normalizer_compare_hash%% *}
    [[ "$normalizer_content_sha256" == "$normalizer_compare_hash" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s:%Y:%Z' -- "$normalizer_self_hint")" == "$normalizer_initial_identity" ]] || {
        builtin printf '%s\n' 'normalize-node-argv: initial script changed during memory capture' >&2
        builtin exit 126
    }
    builtin unset normalizer_compare_content normalizer_compare_hash normalizer_initial_identity
    builtin unset BASH_ENV ENV CDPATH GLOBIGNORE LD_AUDIT LD_LIBRARY_PATH LD_PRELOAD
    exec /usr/lib/cargo/bin/coreutils/env -i CUBIKAN_NORMALIZER_SANITIZED=1 HOME=/home/charles LC_ALL=C LANG=C TZ=UTC PATH=/usr/bin:/bin \
        /usr/bin/bash --noprofile --norc -p -c "$normalizer_content" "$normalizer_self_hint" \
        "$NORMALIZER_BOUND_TOKEN" "$normalizer_self_hint" "$normalizer_content_sha256" "$normalizer_content" "$@"
fi
[[ "$normalizer_self_hint" == */chain/tools/normalize-node-argv.sh ]] || {
    builtin printf '%s\n' 'normalize-node-argv: path hint is not canonical' >&2
    builtin exit 126
}
[[ "${CUBIKAN_NORMALIZER_SANITIZED:-}" == 1 ]] || {
    builtin printf '%s\n' 'normalize-node-argv: sanitized entry proof is missing' >&2
    builtin exit 126
}
unset CUBIKAN_NORMALIZER_SANITIZED
set -euo pipefail
[[ -z "$(builtin compgen -A function)" ]] || {
    builtin printf '%s\n' 'normalize-node-argv: inherited shell function detected' >&2
    builtin exit 126
}
readonly PATH=/usr/bin:/bin
readonly DD=/usr/lib/cargo/bin/coreutils/dd

readonly NORMALIZER_SELF="$normalizer_self_hint"
unset normalizer_self_hint
readonly SELF_DIR="$(cd -- "$(/usr/lib/cargo/bin/coreutils/dirname -- "$NORMALIZER_SELF")" && pwd -P)"
readonly PROJECT_ROOT="$(cd -- "$SELF_DIR/../.." && pwd -P)"
readonly GRAMMAR_FILE="$SELF_DIR/node-argv-grammar-v1.txt"
readonly SEALED_EXEC="$SELF_DIR/sealed-exec.py"
readonly PYTHON=/usr/bin/python3.14
readonly PVF_WORKERS_DIR=/run/cubikan-exec/pvf-workers
readonly EXPECTED_GRAMMAR_SHA256="64be27a9c5ff19b56adbd009e087c0fae059e667a2cf817cba5c6115fd019498"
readonly EXPECTED_SEALED_EXEC_SHA256="b3cd068ac20123ca2971aca6dff5f6718778090323e14c3d3156669bc1c1f672"
readonly EXPECTED_PYTHON_SHA256="b8d8288faefdd300201f43fcf00f6f539a27218eeed3a3dff5ab10b9c4c99700"
SEALED_EXEC_CONTENT=""

die() {
    printf 'normalize-node-argv: %s\n' "$*" >&2
    exit 1
}

sha256_file() {
    /usr/lib/cargo/bin/coreutils/sha256sum -- "$1" | /usr/bin/gawk '{print $1}'
}

expected_asset_sha256() {
    case "$1" in
        polkadot) printf '%s\n' '53c9f450f619d680578dbeed6685de102a9632db7b134631650450b84ea83567' ;;
        polkadot-omni-node) printf '%s\n' 'ff8e5253e8a3e30b421c83d938a3245bdc5de222d807aaf3648575ae029faece' ;;
        *) die "internal unsupported node asset $1" ;;
    esac
}

verify_grammar() {
    [[ -f "$GRAMMAR_FILE" && ! -L "$GRAMMAR_FILE" ]] || die "grammar file is missing or symbolic"
    local actual
    actual="$(sha256_file "$GRAMMAR_FILE")"
    [[ "$actual" == "$EXPECTED_GRAMMAR_SHA256" ]] || die "grammar hash mismatch"
}

verify_sealed_exec_boundary() {
    local source_identity compare_identity content compare_content actual compare_actual
    [[ -f "$SEALED_EXEC" && ! -L "$SEALED_EXEC" ]] || die "sealed execution helper is missing or symbolic"
    source_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s:%Y:%Z' -- "$SEALED_EXEC")"
    content=''
    IFS= read -r -d '' content <"$SEALED_EXEC" || [[ -n "$content" ]] || die "cannot capture sealed execution helper"
    actual="$(printf '%s' "$content" | /usr/lib/cargo/bin/coreutils/sha256sum | /usr/bin/gawk '{print $1}')"
    [[ "$actual" == "$EXPECTED_SEALED_EXEC_SHA256" ]] || die "sealed execution helper hash mismatch"
    compare_content=''
    IFS= read -r -d '' compare_content <"$SEALED_EXEC" || [[ -n "$compare_content" ]] || die "cannot recapture sealed execution helper"
    compare_actual="$(printf '%s' "$compare_content" | /usr/lib/cargo/bin/coreutils/sha256sum | /usr/bin/gawk '{print $1}')"
    compare_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s:%Y:%Z' -- "$SEALED_EXEC")"
    [[ "$compare_actual" == "$EXPECTED_SEALED_EXEC_SHA256" && "$compare_actual" == "$actual" &&
        "$compare_identity" == "$source_identity" ]] || die "sealed execution helper changed during memory capture"
    SEALED_EXEC_CONTENT="$content"
    [[ -x "$PYTHON" && ! -L "$PYTHON" && "$(sha256_file "$PYTHON")" == "$EXPECTED_PYTHON_SHA256" ]] || die "sealed execution interpreter identity mismatch"
}

verify_pvf_worker_file() {
    local path=$1 expected_size=$2 expected_hash=$3
    local prefix_fd hash_fd worker_opened_identity worker_path_identity prefix digest
    [[ -f "$path" && -x "$path" && ! -L "$path" ]] || die 'PVF worker is missing, symbolic, or nonexecutable'
    [[ "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%u:%a:%h:%s:%F' -- "$path")" == \
        "$EUID:500:1:$expected_size:regular file" ]] || die 'PVF worker metadata mismatch'
    exec {prefix_fd}<"$path"
    exec {hash_fd}<"$path"
    worker_opened_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s:%Y:%Z' -- "/proc/self/fd/$hash_fd")"
    worker_path_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s:%Y:%Z' -- "$path")"
    [[ "$worker_opened_identity" == "$worker_path_identity" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s:%Y:%Z' -- "/proc/self/fd/$prefix_fd")" == "$worker_opened_identity" ]] ||
        die 'PVF worker changed while opening'
    IFS= read -r -N 4 prefix <&"$prefix_fd" || die 'PVF worker lacks a complete ELF header'
    [[ "$prefix" == $'\x7fELF' ]] || die 'PVF worker is not ELF'
    digest="$(/usr/lib/cargo/bin/coreutils/sha256sum - <&"$hash_fd")"
    [[ "${digest%% *}" == "$expected_hash" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s:%Y:%Z' -- "$path")" == "$worker_opened_identity" ]] ||
        die 'PVF worker bytes or identity mismatch'
    exec {prefix_fd}<&-
    exec {hash_fd}<&-
}

verify_pvf_worker_tree() {
    local mount_proof probe_status=0
    local -a entries
    [[ -d "$PVF_WORKERS_DIR" && ! -L "$PVF_WORKERS_DIR" &&
        "$(/usr/lib/cargo/bin/coreutils/realpath -e -- "$PVF_WORKERS_DIR")" == "$PVF_WORKERS_DIR" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -fLc '%T' -- "$PVF_WORKERS_DIR")" == tmpfs &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%u:%a:%F' -- "$PVF_WORKERS_DIR")" == "$EUID:500:directory" ]] ||
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
        "$PVF_WORKERS_DIR/.normalizer-write-probe" || probe_status=$?
    [[ $probe_status -eq 0 && ! -e "$PVF_WORKERS_DIR/.normalizer-write-probe" ]] ||
        die 'PVF worker root did not reject creation with EROFS'
}

usage() {
    printf '%s\n' 'usage: normalize-node-argv.sh --role ROLE [--print0 [--verify-workers]] -- COMMAND ARG...' >&2
    exit 2
}

[[ $# -gt 0 ]] || usage
if [[ "$1" == "--verify-grammar" ]]; then
    [[ $# -eq 1 ]] || usage
    verify_grammar
    printf '%s\n' "$EXPECTED_GRAMMAR_SHA256"
    exit 0
fi

role=""
print0=0
verify_workers=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --role)
            [[ -z "$role" && $# -ge 2 ]] || usage
            role="$2"
            shift 2
            ;;
        --print0)
            [[ $print0 -eq 0 ]] || usage
            print0=1
            shift
            ;;
        --verify-workers)
            [[ $verify_workers -eq 0 ]] || usage
            verify_workers=1
            shift
            ;;
        --)
            shift
            break
            ;;
        *) usage ;;
    esac
done

[[ $verify_workers -eq 0 || $print0 -eq 1 ]] || usage

case "$role" in
    relay-a)
        primary_rpc=9944; primary_p2p=30333; primary_metrics=9615
        relay_rpc=""; relay_p2p=""; relay_metrics=""
        ;;
    relay-b)
        primary_rpc=9945; primary_p2p=30334; primary_metrics=9616
        relay_rpc=""; relay_p2p=""; relay_metrics=""
        ;;
    collator-a)
        primary_rpc=9988; primary_p2p=30335; primary_metrics=9617
        relay_rpc=9990; relay_p2p=30337; relay_metrics=9619
        ;;
    collator-b)
        primary_rpc=9989; primary_p2p=30336; primary_metrics=9618
        relay_rpc=9991; relay_p2p=30338; relay_metrics=9620
        ;;
    *) usage ;;
esac

[[ $# -gt 0 ]] || die "missing node command"
verify_grammar

command_path="$1"
shift
[[ "$command_path" == /* && -f "$command_path" && -x "$command_path" && ! -L "$command_path" ]] || die "command must be an absolute executable regular file"
command_name="${command_path##*/}"
case "$role:$command_name" in
    relay-a:polkadot|relay-b:polkadot|collator-a:polkadot-omni-node|collator-b:polkadot-omni-node) ;;
    *) die "command basename is not allowed for role $role" ;;
esac
readonly expected_path="$PROJECT_ROOT/chain/.cache/downloads/$command_name"
[[ "$command_path" == "$expected_path" && ! -L "$command_path" ]] || die "command must be the canonical verified cache asset"

# Bind verification and sealed materialization to separate descriptors for
# one inode. DrvFS does not provide stable /proc/self/fd execution semantics,
# so the reviewed bytes are copied into a write-sealed memfd before execution
# instead of executing the workspace procfd pathname.
exec {command_prefix_fd}<"$command_path"
exec {command_hash_fd}<"$command_path"
exec {command_copy_fd}<"$command_path"
readonly command_prefix_fd command_hash_fd command_copy_fd
readonly command_prefix_fd_path="/proc/self/fd/$command_prefix_fd"
readonly command_hash_fd_path="/proc/self/fd/$command_hash_fd"
readonly command_copy_fd_path="/proc/self/fd/$command_copy_fd"
[[ -f "$command_prefix_fd_path" && -f "$command_hash_fd_path" && -f "$command_copy_fd_path" ]] || die "opened node asset is not an executable regular file"
opened_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$command_hash_fd_path")"
path_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$command_path")"
[[ "$opened_identity" == "$path_identity" &&
    "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$command_prefix_fd_path")" == "$opened_identity" &&
    "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$command_copy_fd_path")" == "$opened_identity" ]] || die "canonical node asset changed while opening"
IFS= read -r -N 4 asset_prefix <&"$command_prefix_fd" || die "canonical node asset has no complete ELF header"
[[ "$asset_prefix" == $'\x7fELF' ]] || die "canonical node asset is not ELF"
[[ "$(/usr/lib/cargo/bin/coreutils/sha256sum - <&"$command_hash_fd" | /usr/bin/gawk '{print $1}')" == "$(expected_asset_sha256 "$command_name")" ]] || die "canonical node asset hash mismatch"
[[ "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$command_copy_fd_path")" == "$opened_identity" ]] || die "opened node asset changed while hashing"
[[ "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$command_path")" == "$opened_identity" ]] || die "canonical node asset changed while hashing"
readonly opened_identity path_identity

generated=("$@")
separator=-1
for index in "${!generated[@]}"; do
    if [[ "${generated[$index]}" == "--" ]]; then
        [[ $separator -eq -1 ]] || die "duplicate collator separator"
        separator=$index
    fi
done

if [[ "$role" == relay-* ]]; then
    [[ $separator -eq -1 ]] || die "relay validator cannot contain a collator separator"
    primary=("${generated[@]}")
    relay_side=()
else
    [[ $separator -ge 0 ]] || die "collator command is missing relay-side separator"
    primary=("${generated[@]:0:$separator}")
    relay_side=("${generated[@]:$((separator + 1))}")
    [[ ${#relay_side[@]} -gt 0 ]] || die "collator relay side is empty"
fi

safe_scalar() {
    [[ "$1" =~ ^[A-Za-z0-9_./:@,+%=-]+$ ]]
}

validate_value() {
    local side="$1" flag="$2" value="$3"
    [[ -n "$value" && "$value" != --* ]] || die "$side $flag has no value"
    case "$flag" in
        --name)
            [[ "$value" =~ ^[A-Za-z0-9_-]{1,64}$ ]] || die "$side name is unsafe"
            ;;
        --node-key)
            [[ "$value" =~ ^[0-9a-f]{64}$ ]] || die "$side node key is not 32-byte lowercase hex"
            ;;
        --chain)
            [[ "$value" == /* && "$value" == *.json && "$value" != *'/../'* && "$value" != *'/./'* ]] || die "$side chain path is unsafe"
            safe_scalar "$value" || die "$side chain path has unsupported bytes"
            ;;
        --base-path)
            [[ "$value" == /* && "$value" != *'/../'* && "$value" != *'/./'* ]] || die "$side base path is unsafe"
            safe_scalar "$value" || die "$side base path has unsupported bytes"
            ;;
        --workers-path)
            [[ "$value" == /run/cubikan-exec/pvf-workers ]] ||
                die "$side workers path is not the locked private mount"
            if [[ "$side" == primary ]]; then
                [[ "$role" == relay-* ]] || die 'collator primary cannot select relay PVF workers'
            else
                [[ "$role" == collator-* ]] || die 'relay validator cannot contain a second worker side'
            fi
            ;;
        --execute-workers-max-num|--prepare-workers-soft-max-num|--prepare-workers-hard-max-num)
            [[ "$value" == 1 ]] || die "$side $flag must be exactly 1"
            if [[ "$side" == primary ]]; then
                [[ "$role" == relay-* ]] || die "collator primary cannot supply $flag"
            else
                [[ "$role" == collator-* ]] || die "relay validator cannot contain a second worker side"
            fi
            ;;
        --listen-addr)
            safe_scalar "$value" || die "$side listen address has unsupported bytes"
            ;;
        --port|--rpc-port|--ws-port|--prometheus-port)
            [[ "$value" =~ ^[0-9]{1,5}$ && "$value" -ge 1 && "$value" -le 65535 ]] || die "$side $flag is not a TCP port"
            ;;
        --rpc-cors)
            [[ "$side" == primary && "$value" == all ]] ||
                die "$side rpc-cors must be primary-only and all"
            ;;
        --rpc-methods)
            [[ "$side" == primary && "$value" == unsafe ]] ||
                die "$side rpc-methods must be primary-only and unsafe"
            ;;
        --blocks-pruning|--state-pruning)
            [[ "$side" == primary && "$value" == archive ]] || die "$side $flag must be archive"
            ;;
        --execution)
            [[ "$side" == relay-side && "$value" == wasm ]] || die "$side execution must be wasm"
            ;;
        *) die "internal unsupported value flag $flag" ;;
    esac
}

validate_bootnode() {
    [[ "$1" =~ ^/ip4/127\.0\.0\.1/tcp/(30333|30334|30335|30336|30337|30338)/ws/p2p/[1-9A-HJ-NP-Za-km-z]+$ ]] || die "bootnode is not in the locked loopback WebSocket inventory"
}

PARSED=()
parse_side() {
    local side="$1"
    shift
    local -A seen=()
    local token value
    local saw_chain=0 saw_name=0 saw_key=0 saw_base=0
    local saw_listen=0 saw_port=0 saw_rpc=0 saw_metrics=0 saw_workers=0
    local saw_execute_workers=0 saw_prepare_soft=0 saw_prepare_hard=0
    local saw_rpc_cors=0 saw_rpc_methods=0
    local saw_validator=0 saw_collator=0 saw_blocks_archive=0 saw_state_archive=0
    local saw_no_mdns=0 saw_no_telemetry=0 saw_no_hardware_benchmarks=0
    PARSED=()

    while [[ $# -gt 0 ]]; do
        token="$1"
        shift
        [[ "$token" == --* && "$token" != *=* ]] || die "$side contains an unexpected positional or joined flag"
        case "$token" in
            --rpc-external|--unsafe-rpc-external|--ws-external|--unsafe-ws-external|--prometheus-external)
                [[ -z "${seen[$token]:-}" ]] || die "$side duplicates $token"
                seen[$token]=1
                ;;
            --validator|--collator|--force-authoring|--insecure-validator-i-know-what-i-do|--no-mdns|--no-telemetry|--no-hardware-benchmarks)
                [[ -z "${seen[$token]:-}" ]] || die "$side duplicates $token"
                seen[$token]=1
                PARSED+=("$token")
                [[ "$token" == --validator ]] && saw_validator=1
                [[ "$token" == --collator ]] && saw_collator=1
                [[ "$token" == --no-mdns ]] && saw_no_mdns=1
                [[ "$token" == --no-telemetry ]] && saw_no_telemetry=1
                [[ "$token" == --no-hardware-benchmarks ]] && saw_no_hardware_benchmarks=1
                ;;
            --bootnodes)
                [[ -z "${seen[$token]:-}" ]] || die "$side duplicates $token"
                seen[$token]=1
                [[ $# -gt 0 && "$1" != --* ]] || die "$side bootnodes has no value"
                PARSED+=("$token")
                while [[ $# -gt 0 && "$1" != --* ]]; do
                    validate_bootnode "$1"
                    PARSED+=("$1")
                    shift
                done
                ;;
            --name|--node-key|--chain|--base-path|--listen-addr|--port|--rpc-port|--ws-port|--prometheus-port|--workers-path|--execute-workers-max-num|--prepare-workers-soft-max-num|--prepare-workers-hard-max-num|--rpc-cors|--rpc-methods|--blocks-pruning|--state-pruning|--execution)
                [[ -z "${seen[$token]:-}" ]] || die "$side duplicates $token"
                seen[$token]=1
                [[ $# -gt 0 ]] || die "$side $token has no value"
                value="$1"
                shift
                validate_value "$side" "$token" "$value"
                case "$token" in
                    --listen-addr) saw_listen=1 ;;
                    --port) saw_port=1 ;;
                    --rpc-port|--ws-port)
                        [[ $saw_rpc -eq 0 ]] || die "$side contains both --rpc-port and --ws-port"
                        saw_rpc=1
                        ;;
                    --prometheus-port) saw_metrics=1 ;;
                    --rpc-cors) saw_rpc_cors=1 ;;
                    --rpc-methods) saw_rpc_methods=1 ;;
                    --workers-path) saw_workers=1; PARSED+=("$token" "$value") ;;
                    --execute-workers-max-num) saw_execute_workers=1; PARSED+=("$token" "$value") ;;
                    --prepare-workers-soft-max-num) saw_prepare_soft=1; PARSED+=("$token" "$value") ;;
                    --prepare-workers-hard-max-num) saw_prepare_hard=1; PARSED+=("$token" "$value") ;;
                    --chain) saw_chain=1; PARSED+=("$token" "$value") ;;
                    --name) saw_name=1; PARSED+=("$token" "$value") ;;
                    --node-key) saw_key=1; PARSED+=("$token" "$value") ;;
                    --base-path) saw_base=1; PARSED+=("$token" "$value") ;;
                    --blocks-pruning) saw_blocks_archive=1; PARSED+=("$token" "$value") ;;
                    --state-pruning) saw_state_archive=1; PARSED+=("$token" "$value") ;;
                    --listen-addr|--port|--rpc-port|--ws-port|--prometheus-port|--rpc-cors|--rpc-methods) ;;
                    *) PARSED+=("$token" "$value") ;;
                esac
                ;;
            *) die "$side contains unknown flag $token" ;;
        esac
    done

    [[ $saw_chain -eq 1 ]] || die "$side is missing --chain"
    [[ $saw_port -eq 1 && $saw_rpc -eq 1 && $saw_metrics -eq 1 ]] || die "$side is missing generated port fields"
    if [[ "$side" == primary ]]; then
        [[ $saw_name -eq 1 && $saw_key -eq 1 && $saw_base -eq 1 && $saw_listen -eq 1 ]] || die "primary side is missing generated identity/path/listener fields"
        [[ $saw_rpc_cors -eq 1 && $saw_rpc_methods -eq 1 ]] ||
            die 'primary side is missing its generated RPC policy fields'
        if [[ "$role" == relay-* ]]; then
            [[ $saw_validator -eq 1 && $saw_collator -eq 0 ]] || die "relay role must be validator-only"
            [[ $saw_workers -eq 1 && $saw_execute_workers -eq 1 &&
                $saw_prepare_soft -eq 1 && $saw_prepare_hard -eq 1 ]] ||
                die 'relay role is missing its pinned PVF worker contract'
        else
            [[ $saw_collator -eq 1 && $saw_validator -eq 0 ]] || die "collator primary must be collator-only"
            [[ $saw_blocks_archive -eq 1 && $saw_state_archive -eq 1 ]] || die "collator primary must preserve both archive flags"
            [[ $saw_workers -eq 0 && $saw_execute_workers -eq 0 &&
                $saw_prepare_soft -eq 0 && $saw_prepare_hard -eq 0 ]] ||
                die 'collator primary unexpectedly configures relay PVF workers'
        fi
    else
        [[ $saw_name -eq 0 && $saw_key -eq 0 && $saw_collator -eq 0 ]] || die "relay side contains primary-only identity"
        [[ $saw_rpc_cors -eq 0 && $saw_rpc_methods -eq 0 ]] ||
            die 'relay side unexpectedly contains primary-only RPC policy fields'
        [[ $saw_workers -eq 1 && $saw_execute_workers -eq 1 &&
            $saw_prepare_soft -eq 1 && $saw_prepare_hard -eq 1 ]] ||
            die 'collator relay side is missing its pinned PVF worker contract'
    fi
    [[ $saw_no_mdns -eq 1 ]] || PARSED+=(--no-mdns)
    [[ $saw_no_telemetry -eq 1 ]] || PARSED+=(--no-telemetry)
    [[ $saw_no_hardware_benchmarks -eq 1 ]] || PARSED+=(--no-hardware-benchmarks)
}

parse_side primary "${primary[@]}"
normalized=(
    "$command_path" "${PARSED[@]}"
    --listen-addr "/ip4/127.0.0.1/tcp/$primary_p2p/ws"
    --experimental-rpc-endpoint "listen-addr=127.0.0.1:$primary_rpc,methods=unsafe,cors=all"
    --prometheus-port "$primary_metrics"
)

if [[ "$role" == collator-* ]]; then
    parse_side relay-side "${relay_side[@]}"
    normalized+=(
        -- "${PARSED[@]}"
        --listen-addr "/ip4/127.0.0.1/tcp/$relay_p2p/ws"
        --experimental-rpc-endpoint "listen-addr=127.0.0.1:$relay_rpc,methods=unsafe,cors=all"
        --prometheus-port "$relay_metrics"
    )
fi

if [[ $print0 -eq 1 ]]; then
    if [[ $verify_workers -eq 1 ]]; then
        verify_pvf_worker_tree
    fi
    printf '%s\0' "${normalized[@]}"
else
    verify_sealed_exec_boundary
    verify_pvf_worker_tree
    readonly command_size="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%s' -- "$command_copy_fd_path")"
    exec "$PYTHON" -I -S -c "$SEALED_EXEC_CONTENT" exec-fd "$command_copy_fd" "$command_size" \
        "$(expected_asset_sha256 "$command_name")" -- "$command_path" "${normalized[@]:1}" </dev/null
fi
