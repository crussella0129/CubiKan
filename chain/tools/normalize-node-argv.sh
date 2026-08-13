#!/usr/bin/bash -p

# Bind every continuation to the inode Bash already opened for this script.
# The separately carried path is only a location hint for repository assets;
# it is never reopened as executable authority.
readonly NORMALIZER_BOUND_TOKEN=__cubikan_normalizer_bound_path_v1__
if [[ "${1:-}" == "$NORMALIZER_BOUND_TOKEN" ]]; then
    [[ $# -ge 2 && "$2" == /* && -z "${BASH_SOURCE[0]}" ]] || {
        builtin printf '%s\n' 'normalize-node-argv: invalid bound-path entry' >&2
        builtin exit 126
    }
    normalizer_self_hint=$2
    shift 2
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
    normalizer_initial_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$normalizer_self_hint")" || builtin exit 126
    exec {normalizer_copy_fd}<"$normalizer_self_hint" || builtin exit 126
    normalizer_snapshot="$(/usr/lib/cargo/bin/coreutils/mktemp /tmp/cubikan-normalizer-v1.XXXXXX)" || builtin exit 126
    /usr/lib/cargo/bin/coreutils/dd of="$normalizer_snapshot" status=none <&"$normalizer_copy_fd" || builtin exit 126
    exec {normalizer_copy_fd}<&-
    /usr/lib/cargo/bin/coreutils/chmod 0400 -- "$normalizer_snapshot" || builtin exit 126
    normalizer_snapshot_hash="$(/usr/lib/cargo/bin/coreutils/sha256sum -- "$normalizer_snapshot")" || builtin exit 126
    normalizer_snapshot_hash=${normalizer_snapshot_hash%% *}
    exec {normalizer_compare_fd}<"$normalizer_self_hint" || builtin exit 126
    normalizer_compare_hash="$(/usr/lib/cargo/bin/coreutils/sha256sum - <&"$normalizer_compare_fd")" || builtin exit 126
    normalizer_compare_hash=${normalizer_compare_hash%% *}
    exec {normalizer_compare_fd}<&-
    [[ "$normalizer_snapshot_hash" == "$normalizer_compare_hash" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$normalizer_self_hint")" == "$normalizer_initial_identity" ]] || {
        /usr/bin/gnurm -f -- "$normalizer_snapshot"
        builtin printf '%s\n' 'normalize-node-argv: initial script changed during private snapshot creation' >&2
        builtin exit 126
    }
    normalizer_snapshot_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$normalizer_snapshot")" || builtin exit 126
    exec {normalizer_source_fd}<"$normalizer_snapshot" || builtin exit 126
    /usr/bin/gnurm -f -- "$normalizer_snapshot"
    [[ ! -e "$normalizer_snapshot" && -f "/proc/self/fd/$normalizer_source_fd" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "/proc/self/fd/$normalizer_source_fd")" == "$normalizer_snapshot_identity" ]] || {
        builtin printf '%s\n' 'normalize-node-argv: private script snapshot binding failed' >&2
        builtin exit 126
    }
    builtin unset BASH_ENV ENV CDPATH GLOBIGNORE LD_AUDIT LD_LIBRARY_PATH LD_PRELOAD
    exec /usr/lib/cargo/bin/coreutils/env -i CUBIKAN_NORMALIZER_SANITIZED=1 HOME=/home/charles LC_ALL=C LANG=C TZ=UTC PATH=/usr/bin:/bin \
        /usr/bin/bash --noprofile --norc -p -s -- \
        "$NORMALIZER_BOUND_TOKEN" "$normalizer_self_hint" "$@" <&"$normalizer_source_fd"
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
readonly EXPECTED_GRAMMAR_SHA256="112655c95fcf0b1fe535d0e6b209883374bb650b40ecd9f3383d4378dd1b88b4"

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

usage() {
    printf '%s\n' 'usage: normalize-node-argv.sh --role ROLE [--print0] -- COMMAND ARG...' >&2
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
        --)
            shift
            break
            ;;
        *) usage ;;
    esac
done

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

# Bind verification and private materialization to separate descriptors for
# one inode. DrvFS does not provide stable /proc/self/fd execution semantics,
# so the reviewed bytes are copied into the wrapper's private tmpfs before
# execution instead of executing the workspace procfd pathname.
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
        --listen-addr)
            safe_scalar "$value" || die "$side listen address has unsupported bytes"
            ;;
        --port|--rpc-port|--ws-port|--prometheus-port)
            [[ "$value" =~ ^[0-9]{1,5}$ && "$value" -ge 1 && "$value" -le 65535 ]] || die "$side $flag is not a TCP port"
            ;;
        --rpc-cors)
            [[ "$value" == all ]] || die "$side rpc-cors must be all"
            ;;
        --rpc-methods)
            [[ "$value" == unsafe ]] || die "$side rpc-methods must be unsafe"
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
    [[ "$1" =~ ^/ip4/127\.0\.0\.1/tcp/(30333|30334|30335|30336|30337|30338)/p2p/[1-9A-HJ-NP-Za-km-z]+$ ]] || die "bootnode is not in the locked loopback inventory"
}

PARSED=()
parse_side() {
    local side="$1"
    shift
    local -A seen=()
    local token value
    local saw_chain=0 saw_name=0 saw_key=0 saw_base=0
    local saw_listen=0 saw_port=0 saw_rpc=0 saw_metrics=0
    local saw_validator=0 saw_collator=0 saw_blocks_archive=0 saw_state_archive=0
    local saw_no_mdns=0 saw_no_telemetry=0
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
            --name|--node-key|--chain|--base-path|--listen-addr|--port|--rpc-port|--ws-port|--prometheus-port|--rpc-cors|--rpc-methods|--blocks-pruning|--state-pruning|--execution)
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
                    --chain) saw_chain=1; PARSED+=("$token" "$value") ;;
                    --name) saw_name=1; PARSED+=("$token" "$value") ;;
                    --node-key) saw_key=1; PARSED+=("$token" "$value") ;;
                    --base-path) saw_base=1; PARSED+=("$token" "$value") ;;
                    --blocks-pruning) saw_blocks_archive=1; PARSED+=("$token" "$value") ;;
                    --state-pruning) saw_state_archive=1; PARSED+=("$token" "$value") ;;
                    --listen-addr|--port|--rpc-port|--ws-port|--prometheus-port) ;;
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
        if [[ "$role" == relay-* ]]; then
            [[ $saw_validator -eq 1 && $saw_collator -eq 0 ]] || die "relay role must be validator-only"
        else
            [[ $saw_collator -eq 1 && $saw_validator -eq 0 ]] || die "collator primary must be collator-only"
            [[ $saw_blocks_archive -eq 1 && $saw_state_archive -eq 1 ]] || die "collator primary must preserve both archive flags"
        fi
    else
        [[ $saw_name -eq 0 && $saw_key -eq 0 && $saw_collator -eq 0 ]] || die "relay side contains primary-only identity"
    fi
    [[ $saw_no_mdns -eq 1 ]] || PARSED+=(--no-mdns)
    [[ $saw_no_telemetry -eq 1 ]] || PARSED+=(--no-telemetry)
}

parse_side primary "${primary[@]}"
normalized=("$command_path" "${PARSED[@]}" --listen-addr "/ip4/127.0.0.1/tcp/$primary_p2p/ws" --port "$primary_p2p" --rpc-port "$primary_rpc" --prometheus-port "$primary_metrics")

if [[ "$role" == collator-* ]]; then
    parse_side relay-side "${relay_side[@]}"
    normalized+=(-- "${PARSED[@]}" --listen-addr "/ip4/127.0.0.1/tcp/$relay_p2p/ws" --port "$relay_p2p" --rpc-port "$relay_rpc" --prometheus-port "$relay_metrics")
fi

if [[ $print0 -eq 1 ]]; then
    printf '%s\0' "${normalized[@]}"
else
    [[ "$(/usr/lib/cargo/bin/coreutils/stat -fLc '%T' -- /run/cubikan-exec)" == tmpfs &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%a' -- /run/cubikan-exec)" == 700 ]] || die "node execution requires the wrapper private executable tmpfs"
    execution_dir="$(/usr/lib/cargo/bin/coreutils/mktemp -d "/run/cubikan-exec/cubikan-node-$command_name.XXXXXX")"
    execution_path="$execution_dir/$command_name"
    "$DD" of="$execution_path" status=none <&"$command_copy_fd"
    [[ "$(sha256_file "$execution_path")" == "$(expected_asset_sha256 "$command_name")" ]] || die "private node materialization hash mismatch"
    /usr/lib/cargo/bin/coreutils/chmod 0500 -- "$execution_path"
    exec -a "$command_path" -- "$execution_path" "${normalized[@]:1}" </dev/null
fi
