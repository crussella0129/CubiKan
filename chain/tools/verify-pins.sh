#!/usr/bin/bash -p

# Bind every verifier continuation to the inode Bash already opened.  The
# separately carried canonical path is only a repository-location hint; no
# later bootstrap or namespace transition reopens it as executable authority.
readonly VERIFIER_BOUND_TOKEN=__cubikan_verifier_bound_path_v1__
if [[ "${1:-}" == "$VERIFIER_BOUND_TOKEN" ]]; then
    [[ $# -ge 4 && "$2" == /* && -z "${BASH_SOURCE[0]}" &&
        (("$3" == - && "$4" == -) ||
            ("$3" =~ ^[1-9][0-9]*$ && "$4" =~ ^[0-9]+:[0-9]+:[1-9][0-9]*$)) ]] || {
        builtin printf '%s\n' 'verify-pins: invalid bound-path entry' >&2
        builtin exit 126
    }
    verifier_self_hint=$2
    verifier_continuation_fd=$3
    verifier_continuation_identity=$4
    shift 4
    if [[ "$verifier_continuation_fd" != - ]]; then
        [[ -f "/proc/self/fd/$verifier_continuation_fd" &&
            "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "/proc/self/fd/$verifier_continuation_fd")" == "$verifier_continuation_identity" ]] || {
            builtin printf '%s\n' 'verify-pins: continuation snapshot identity mismatch' >&2
            builtin exit 126
        }
    fi
else
    if [[ $- != *p* && -n "${BASH_ENV:-}${ENV:-}" ]]; then
        builtin printf '%s\n' 'verify-pins: non-privileged compatibility entry forbids BASH_ENV and ENV' >&2
        builtin exit 126
    fi
    case "${BASH_SOURCE[0]}" in
        /*) verifier_self_hint="${BASH_SOURCE[0]}" ;;
        *) verifier_self_hint="$(builtin pwd -P)/${BASH_SOURCE[0]}" ;;
    esac
    [[ "$verifier_self_hint" == */chain/tools/verify-pins.sh && -f "$verifier_self_hint" &&
        ! -L "$verifier_self_hint" ]] || {
        builtin printf '%s\n' 'verify-pins: initial script path or descriptor is invalid' >&2
        builtin exit 126
    }
    verifier_initial_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$verifier_self_hint")" || builtin exit 126
    exec {verifier_copy_fd}<"$verifier_self_hint" || builtin exit 126
    verifier_snapshot="$(/usr/lib/cargo/bin/coreutils/mktemp /tmp/cubikan-verifier-v1.XXXXXX)" || builtin exit 126
    /usr/lib/cargo/bin/coreutils/dd of="$verifier_snapshot" status=none <&"$verifier_copy_fd" || builtin exit 126
    exec {verifier_copy_fd}<&-
    /usr/lib/cargo/bin/coreutils/chmod 0400 -- "$verifier_snapshot" || builtin exit 126
    verifier_snapshot_hash="$(/usr/lib/cargo/bin/coreutils/sha256sum -- "$verifier_snapshot")" || builtin exit 126
    verifier_snapshot_hash=${verifier_snapshot_hash%% *}
    exec {verifier_compare_fd}<"$verifier_self_hint" || builtin exit 126
    verifier_compare_hash="$(/usr/lib/cargo/bin/coreutils/sha256sum - <&"$verifier_compare_fd")" || builtin exit 126
    verifier_compare_hash=${verifier_compare_hash%% *}
    exec {verifier_compare_fd}<&-
    [[ "$verifier_snapshot_hash" == "$verifier_compare_hash" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$verifier_self_hint")" == "$verifier_initial_identity" ]] || {
        /usr/bin/gnurm -f -- "$verifier_snapshot"
        builtin printf '%s\n' 'verify-pins: initial script changed during private snapshot creation' >&2
        builtin exit 126
    }
    verifier_snapshot_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$verifier_snapshot")" || builtin exit 126
    exec {verifier_source_fd}<"$verifier_snapshot" || builtin exit 126
    exec {verifier_continuation_fd}<"$verifier_snapshot" || builtin exit 126
    verifier_continuation_identity="$verifier_snapshot_identity"
    /usr/bin/gnurm -f -- "$verifier_snapshot"
    [[ ! -e "$verifier_snapshot" && -f "/proc/self/fd/$verifier_source_fd" &&
        -f "/proc/self/fd/$verifier_continuation_fd" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "/proc/self/fd/$verifier_source_fd")" == "$verifier_snapshot_identity" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "/proc/self/fd/$verifier_continuation_fd")" == "$verifier_snapshot_identity" ]] || {
        builtin printf '%s\n' 'verify-pins: private verifier snapshot binding failed' >&2
        builtin exit 126
    }
    verifier_tool_dir="${verifier_self_hint%/*}"
    verifier_project_root="${verifier_tool_dir%/chain/tools}"
    [[ "$verifier_project_root" != "$verifier_tool_dir" ]] || builtin exit 126
    builtin unset BASH_ENV ENV CDPATH GLOBIGNORE LD_AUDIT LD_LIBRARY_PATH LD_PRELOAD PYTHONPATH
    exec /usr/lib/cargo/bin/coreutils/env -i \
        CUBIKAN_VERIFIER_SANITIZED=1 \
        HOME=/home/charles \
        CARGO_HOME="$verifier_project_root/chain/.cache/cargo-home" \
        RUSTUP_HOME=/home/charles/.rustup \
        PATH=/home/charles/.cargo/bin:/usr/bin:/bin \
        LC_ALL=C LANG=C TZ=UTC TMPDIR="$verifier_project_root/chain/.cache/tmp" \
        GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null \
        GIT_NO_REPLACE_OBJECTS=1 GIT_OPTIONAL_LOCKS=0 \
        /usr/bin/bash --noprofile --norc -p -s -- \
        "$VERIFIER_BOUND_TOKEN" "$verifier_self_hint" "$verifier_continuation_fd" "$verifier_continuation_identity" \
        --sanitized-entry "$@" \
        <&"$verifier_source_fd"
fi

[[ "$verifier_self_hint" == */chain/tools/verify-pins.sh ]] || {
    builtin printf '%s\n' 'verify-pins: path hint is not canonical' >&2
    builtin exit 126
}
set -euo pipefail
shopt -u varredir_close
[[ "${1:-}" == --sanitized-entry ]] || {
    builtin printf '%s\n' 'verify-pins: sanitized entry token is missing' >&2
    builtin exit 126
}
shift
[[ "${CUBIKAN_VERIFIER_SANITIZED:-}" == 1 ]] || {
    builtin printf '%s\n' 'verify-pins: sanitized entry proof is missing' >&2
    builtin exit 126
}
[[ -z "$(builtin compgen -A function)" ]] || {
    builtin printf '%s\n' 'verify-pins: inherited shell function detected' >&2
    builtin exit 126
}
readonly PATH=/home/charles/.cargo/bin:/usr/bin:/bin
readonly HOME=/home/charles
readonly RUSTUP_HOME=/home/charles/.rustup
readonly LC_ALL LANG TZ TMPDIR
unset CUBIKAN_VERIFIER_SANITIZED
umask 077

readonly VERIFIER_SELF="$verifier_self_hint"
readonly VERIFIER_CONTINUATION_FD="$verifier_continuation_fd"
readonly VERIFIER_CONTINUATION_IDENTITY="$verifier_continuation_identity"
unset verifier_self_hint
unset verifier_continuation_fd
unset verifier_continuation_identity
readonly TOOL_DIR="$(cd -- "$(/usr/lib/cargo/bin/coreutils/dirname -- "$VERIFIER_SELF")" && pwd -P)"
readonly PROJECT_ROOT="$(cd -- "$TOOL_DIR/../.." && pwd -P)"
readonly CARGO_HOME="$PROJECT_ROOT/chain/.cache/cargo-home"
readonly TASK_TMP="$PROJECT_ROOT/chain/.cache/tmp"
[[ "$VERIFIER_SELF" == "$TOOL_DIR/verify-pins.sh" ]] || {
    builtin printf '%s\n' 'verify-pins: verifier path hint is invalid' >&2
    builtin exit 126
}
builtin cd -- "$PROJECT_ROOT"
export PATH HOME RUSTUP_HOME CARGO_HOME LC_ALL LANG TZ TMPDIR
[[ "$TMPDIR" == "$TASK_TMP" && ! -L "$PROJECT_ROOT/chain/.cache" && ! -L "$TASK_TMP" ]] || {
    builtin printf '%s\n' 'verify-pins: task temporary path is not canonical' >&2
    builtin exit 126
}
/usr/lib/cargo/bin/coreutils/mkdir -p -- "$TASK_TMP"
readonly DEFAULT_PINS="$PROJECT_ROOT/chain/pins.toml"
readonly CACHE="$PROJECT_ROOT/chain/.cache"
readonly LOOPBACK="$TOOL_DIR/loopback-netns.sh"
LOOPBACK_FD=""
LOOPBACK_FD_PATH=""
LOOPBACK_LAUNCH_FD=""
LOOPBACK_LAUNCH_FD_PATH=""
LOOPBACK_REENTRY_FD=""
LOOPBACK_REENTRY_FD_PATH=""
NORMALIZER_FD=""
NORMALIZER_FD_PATH=""
# Updated only after the complete pin document has passed independent review.
# This closes the bootstrap hole where a modified pin file could bless a
# modified namespace executable before the rest of the verifier runs.
readonly EXPECTED_PINS_SHA256="3105d6bf482feebf8ef89489fb4ae13a9acb66848744162e4a1f8df052d33b56"

die() {
    printf 'verify-pins: %s\n' "$*" >&2
    exit 1
}

usage() {
    printf '%s\n' 'usage: verify-pins.sh --fetch-exact | --locked --offline | --self-test | --test-static [PINS_COPY] | --test-identity CLASS ABSOLUTE_SUBJECT' >&2
    exit 2
}

pin() {
    local section="$1" key="$2"
    /usr/bin/gawk -v section="[$section]" -v key="$key" '
        $0 == section { active = 1; next }
        /^\[/ { active = 0 }
        active && $0 ~ "^" key "[[:space:]]*=" {
            line = $0
            sub("^" key "[[:space:]]*=[[:space:]]*", "", line)
            if (line !~ /^"[^"]*"$/) exit 2
            sub(/^"/, "", line); sub(/"$/, "", line)
            print line
            found = 1
            exit
        }
        END { if (!found) exit 1 }
    ' "$PINS" || die "missing or nonliteral pin $section.$key"
}

sha256_file() {
    [[ -f "$1" && ! -L "$1" ]] || die "missing or symbolic regular file: $1"
    /usr/lib/cargo/bin/coreutils/sha256sum -- "$1" | /usr/bin/gawk '{print $1}'
}

require_hash() {
    local path="$1" expected="$2" actual
    [[ "$expected" =~ ^[0-9a-f]{64}$ && "$expected" != 0000000000000000000000000000000000000000000000000000000000000000 ]] || die "invalid expected SHA-256 for $path"
    actual="$(sha256_file "$path")"
    [[ "$actual" == "$expected" ]] || die "SHA-256 mismatch for $path: expected $expected, got $actual"
}

require_open_fd_hash() {
    local fd="$1" expected="$2" label="$3" fd_path identity actual
    fd_path="/proc/self/fd/$fd"
    [[ "$fd_path" == /proc/self/fd/+([0-9]) && -f "$fd_path" ]] ||
        die "opened $label descriptor is not a regular file"
    [[ "$expected" =~ ^[0-9a-f]{64}$ && "$expected" != 0000000000000000000000000000000000000000000000000000000000000000 ]] ||
        die "invalid expected SHA-256 for $label"
    identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$fd_path")"
    actual="$(/usr/lib/cargo/bin/coreutils/sha256sum - <&"$fd" | /usr/bin/gawk '{print $1}')"
    [[ "$actual" == "$expected" ]] || die "SHA-256 mismatch for opened $label: expected $expected, got $actual"
    [[ "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$fd_path")" == "$identity" ]] ||
        die "opened $label inode changed while hashing"
}

bind_verified_repository_executable() {
    local path="$1" expected="$2" label="$3" fd_variable="$4" fd_path_variable="$5"
    local backup_fd_variable="${6:-}" backup_fd_path_variable="${7:-}"
    local hash_fd copy_fd source_identity path_identity snapshot snapshot_identity
    local opened_fd opened_fd_path backup_fd backup_fd_path
    [[ "$path" == /* && -f "$path" && -x "$path" && ! -L "$path" ]] ||
        die "$label is not an absolute executable regular non-symbolic file"
    exec {hash_fd}<"$path" || die "cannot open $label hash stream"
    exec {copy_fd}<"$path" || die "cannot open $label copy stream"
    [[ -f "/proc/self/fd/$hash_fd" && -f "/proc/self/fd/$copy_fd" &&
        ! -L "$path" && "$path" -ef "/proc/self/fd/$hash_fd" &&
        "$path" -ef "/proc/self/fd/$copy_fd" ]] || die "$label pathname changed while opening"
    source_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "/proc/self/fd/$copy_fd")"
    path_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$path")"
    [[ "$source_identity" == "$path_identity" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "/proc/self/fd/$hash_fd")" == "$source_identity" ]] ||
        die "$label hash stream differs from the reviewed inode"
    require_open_fd_hash "$hash_fd" "$expected" "$label"
    exec {hash_fd}<&-

    snapshot="$(/usr/lib/cargo/bin/coreutils/mktemp /tmp/cubikan-reviewed-script-v1.XXXXXX)"
    /usr/lib/cargo/bin/coreutils/dd of="$snapshot" status=none <&"$copy_fd" || die "cannot materialize reviewed $label bytes"
    exec {copy_fd}<&-
    /usr/lib/cargo/bin/coreutils/chmod 0400 -- "$snapshot"
    require_hash "$snapshot" "$expected"
    snapshot_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$snapshot")"
    exec {opened_fd}<"$snapshot" || die "cannot open reviewed $label snapshot"
    opened_fd_path="/proc/self/fd/$opened_fd"
    if [[ -n "$backup_fd_variable" ]]; then
        [[ -n "$backup_fd_path_variable" ]] || die "missing backup descriptor output for $label"
        exec {backup_fd}<"$snapshot" || die "cannot open backup reviewed $label snapshot"
        backup_fd_path="/proc/self/fd/$backup_fd"
    fi
    /usr/bin/gnurm -f -- "$snapshot"
    [[ ! -e "$snapshot" && -f "$opened_fd_path" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$opened_fd_path")" == "$snapshot_identity" ]] || {
        exec {opened_fd}<&-
        die "$label private snapshot binding failed"
    }
    if [[ -n "$backup_fd_variable" ]]; then
        [[ -f "$backup_fd_path" &&
            "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$backup_fd_path")" == "$snapshot_identity" ]] ||
            die "backup $label snapshot differs from the reviewed snapshot"
    fi
    printf -v "$fd_variable" '%s' "$opened_fd"
    printf -v "$fd_path_variable" '%s' "$opened_fd_path"
    if [[ -n "$backup_fd_variable" ]]; then
        printf -v "$backup_fd_variable" '%s' "$backup_fd"
        printf -v "$backup_fd_path_variable" '%s' "$backup_fd_path"
    fi
}

require_size() {
    local path="$1" expected="$2" actual
    [[ "$expected" =~ ^[1-9][0-9]*$ ]] || die "invalid expected size for $path"
    actual="$(/usr/lib/cargo/bin/coreutils/stat -c '%s' -- "$path")"
    [[ "$actual" == "$expected" ]] || die "size mismatch for $path: expected $expected, got $actual"
}

tree_sha256() {
    local root="$1" manifest
    [[ -d "$root" && ! -L "$root" ]] || die "tree root is missing or symbolic: $root"
    [[ -z "$(/usr/bin/find "$root" -type l -print -quit)" ]] || die "tree contains a symbolic link: $root"
    [[ -z "$(/usr/bin/find "$root" ! -type d ! -type f -print -quit)" ]] || die "tree contains a nonregular entry: $root"
    manifest="$(/usr/lib/cargo/bin/coreutils/mktemp)"
    while IFS= read -r -d '' path; do
        local relative digest
        relative="${path#"$root"/}"
        [[ "$relative" != "$path" && "$relative" != /* && "$relative" != *'/../'* && "$relative" != '../'* && "$relative" != *$'\n'* ]] || die "unsafe tree path"
        printf '%s' "$relative" | /usr/bin/iconv -f UTF-8 -t UTF-8 >/dev/null 2>&1 || die "non-UTF-8 tree path"
        digest="$(sha256_file "$path")"
        printf '%s  %s\n' "$digest" "$relative" >>"$manifest"
    done < <(/usr/bin/find "$root" -type f -print0 | /usr/lib/cargo/bin/coreutils/sort -z)
    sha256_file "$manifest"
    /usr/bin/gnurm -f -- "$manifest"
}

require_no_placeholders() {
    [[ -f "$PINS" && ! -L "$PINS" ]] || die "pins.toml is missing or symbolic"
    if /usr/bin/grep -En '(TO_BE_FILLED|PLACEHOLDER|PENDING|TBD|[0]{64})' "$PINS" >/dev/null; then
        die "pins.toml contains a placeholder identity"
    fi
    [[ "$(sha256_file "$PINS")" == "$EXPECTED_PINS_SHA256" ]] || die "pin document identity mismatch"
}

enter_or_verify_locked_isolation() {
    local isolation_output isolation_status=0 bash_path loopback_reentry_identity
    [[ -n "$LOOPBACK_FD" ]] ||
        die "locked isolation descriptors are unavailable"
    bash_path="$(pin host_tools bash_path)"
    loopback_reentry_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "/proc/self/fd/$LOOPBACK_REENTRY_FD")"
    isolation_output="$(/usr/lib/cargo/bin/coreutils/env -i \
        CUBIKAN_LOOPBACK_SANITIZED=1 HOME=/home/charles \
        CARGO_HOME=/home/charles/.cargo RUSTUP_HOME=/home/charles/.rustup \
        LC_ALL=C LANG=C TZ=UTC TMPDIR=/tmp PATH=/home/charles/.cargo/bin:/usr/bin:/bin \
        "$bash_path" --noprofile --norc -p -s -- \
        __cubikan_loopback_bound_path_v1__ "$LOOPBACK" "$LOOPBACK_REENTRY_FD" "$loopback_reentry_identity" \
        __cubikan_loopback_clean_entry_v1__ --assert-current-isolated \
        <&"$LOOPBACK_FD" 2>&1)" || isolation_status=$?
    case "$isolation_status" in
        0)
            printf '%s\n' "$isolation_output" >&2
            ;;
        125)
            [[ "$isolation_output" == *'loopback-netns: current-process-isolation=outside-clean-launcher'* ]] ||
                die "namespace wrapper returned the outside status without its exact proof marker"
            printf '%s\n' "$isolation_output" >&2
            /usr/bin/gnurm -f -- "$pins_snapshot"
            pins_snapshot=""
            exec /usr/lib/cargo/bin/coreutils/env -i \
                CUBIKAN_LOOPBACK_SANITIZED=1 HOME=/home/charles \
                CARGO_HOME=/home/charles/.cargo RUSTUP_HOME=/home/charles/.rustup \
                LC_ALL=C LANG=C TZ=UTC TMPDIR=/tmp PATH=/home/charles/.cargo/bin:/usr/bin:/bin \
                "$bash_path" --noprofile --norc -p -s -- \
                __cubikan_loopback_bound_path_v1__ "$LOOPBACK" "$LOOPBACK_REENTRY_FD" "$loopback_reentry_identity" \
                __cubikan_loopback_clean_entry_v1__ -- \
                "$bash_path" --noprofile --norc -p -s -- \
                "$VERIFIER_BOUND_TOKEN" "$VERIFIER_SELF" \
                __cubikan_prebound_verifier_fd_v1__ "$VERIFIER_CONTINUATION_FD" "$VERIFIER_CONTINUATION_IDENTITY" \
                --locked --offline \
                <&"$LOOPBACK_LAUNCH_FD"
            ;;
        *)
            [[ -z "$isolation_output" ]] || printf '%s\n' "$isolation_output" >&2
            die "current isolation state is partial or suspicious; refusing nested namespace entry"
            ;;
    esac
}

download_exact() {
    local url="$1" destination="$2" size="$3" digest="$4" temporary
    /usr/lib/cargo/bin/coreutils/mkdir -p -- "$(/usr/lib/cargo/bin/coreutils/dirname -- "$destination")"
    if [[ -f "$destination" && ! -L "$destination" ]] &&
        [[ "$(/usr/lib/cargo/bin/coreutils/stat -c '%s' -- "$destination")" == "$size" ]] &&
        [[ "$(sha256_file "$destination")" == "$digest" ]]; then
        return
    fi
    temporary="$(/usr/lib/cargo/bin/coreutils/mktemp "${destination}.partial.XXXXXX")"
    trap '/usr/bin/gnurm -f -- "${temporary:-}"' RETURN
    /usr/bin/curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error --retry 3 --output "$temporary" -- "$url"
    require_size "$temporary" "$size"
    require_hash "$temporary" "$digest"
    /usr/lib/cargo/bin/coreutils/chmod 0600 "$temporary"
    /usr/bin/gnumv -f -- "$temporary" "$destination"
    trap - RETURN
}

pin_source="$DEFAULT_PINS"
identity_class=""
identity_subject=""
case "${1:-}:${2:-}:${3:-}:${4:-}" in
    --fetch-exact:::) mode=fetch ;;
    --locked:--offline::) mode=locked ;;
    --self-test:::) mode=selftest ;;
    --test-static:::) mode=teststatic ;;
    --test-static:*::) mode=teststatic; pin_source="$2" ;;
    --test-identity:*:*:) mode=testidentity; identity_class="$2"; identity_subject="$3" ;;
    *) usage ;;
esac
[[ "$pin_source" == /* || "$pin_source" == "$DEFAULT_PINS" ]] || die "test pin copy must be an absolute path"
if [[ "$mode" == testidentity ]]; then
    [[ "$identity_subject" == /* ]] || die "identity-test subject must be an absolute path"
fi
pins_snapshot="$(/usr/lib/cargo/bin/coreutils/mktemp "$TASK_TMP/cubikan-pins-v1.XXXXXX")"
trap '/usr/bin/gnurm -f -- "${pins_snapshot:-}"' EXIT
/usr/bin/gnucp -- "$pin_source" "$pins_snapshot"
readonly PINS="$pins_snapshot"
require_no_placeholders

# The exact fetch branch is the sole network-enabled phase. Before locked mode
# enters the namespace, verify every executable that namespace entry will use.
if [[ "$mode" == locked ]]; then
    bind_verified_repository_executable "$LOOPBACK" \
        "$(pin repository_tools loopback_wrapper_sha256)" namespace-wrapper \
        LOOPBACK_FD LOOPBACK_FD_PATH LOOPBACK_LAUNCH_FD LOOPBACK_LAUNCH_FD_PATH
    bind_verified_repository_executable "$LOOPBACK" \
        "$(pin repository_tools loopback_wrapper_sha256)" namespace-wrapper-reentry \
        LOOPBACK_REENTRY_FD LOOPBACK_REENTRY_FD_PATH
    require_hash "$(pin host_tools bash_path)" "$(pin host_tools bash_sha256)"
    require_hash "$(pin host_tools unshare_path)" "$(pin host_tools unshare_sha256)"
    require_hash "$(pin host_tools ip_path)" "$(pin host_tools ip_sha256)"
    require_hash "$(pin host_tools netcat_path)" "$(pin host_tools netcat_sha256)"
    require_hash "$(pin host_tools env_path)" "$(pin host_tools env_sha256)"
    require_hash "$(pin host_tools awk_path)" "$(pin host_tools awk_sha256)"
    require_hash "$(pin host_tools dd_path)" "$(pin host_tools dd_sha256)"
    require_hash "$(pin host_tools stat_path)" "$(pin host_tools stat_sha256)"
    require_hash "$(pin host_tools mount_path)" "$(pin host_tools mount_sha256)"
    require_hash "$(pin host_tools uname_path)" "$(pin host_tools uname_sha256)"
    enter_or_verify_locked_isolation
fi

if [[ "$mode" == fetch ]]; then
    printf '%s\n' 'verify-pins: beginning the one explicit exact fetch phase' >&2
elif [[ "$mode" == locked ]]; then
    export CARGO_NET_OFFLINE=true
    export WASM_BUILD_WORKSPACE_HINT="$PROJECT_ROOT/chain"
fi

# Further fetch and locked checks are deliberately kept below the hard mode and
# namespace boundary so no dependent execution can precede identity validation.

readonly DOWNLOADS="$CACHE/downloads"
readonly SDK_ARCHIVE="$DOWNLOADS/polkadot-sdk-$(pin polkadot_sdk revision).tar.gz"
readonly ZOMBIENET_ARCHIVE="$DOWNLOADS/zombienet-$(pin zombienet revision).tar.gz"
readonly NODE_ARCHIVE="$DOWNLOADS/node-v$(pin node version)-$(pin node platform).tar.xz"
readonly RUSQLITE_ARCHIVE="$DOWNLOADS/rusqlite-$(pin rusqlite version).crate"
readonly ASSET_NAMES=(polkadot polkadot-parachain polkadot-omni-node chain-spec-builder frame-omni-bencher)

require_exact_literal() {
    local section="$1" key="$2" expected="$3" actual
    actual="$(pin "$section" "$key")"
    [[ "$actual" == "$expected" ]] || die "$section.$key mismatch: expected $expected, got $actual"
}

verify_pin_contract() {
    require_exact_literal polkadot_sdk release polkadot-stable2606-1
    require_exact_literal polkadot_sdk node_version 1.24.1
    require_exact_literal polkadot_sdk revision 8ae9775dc43c0d8cdd0f6d87700596e14278b1e1
    require_exact_literal polkadot_sdk archive_symlink_count 26
    require_exact_literal scaffold source_revision "$(pin polkadot_sdk revision)"
    require_exact_literal scaffold workspace_edition 2021
    require_exact_literal scaffold workspace_resolver 2
    require_exact_literal scaffold workspace_rust_version 1.93.0
    require_exact_literal scaffold runtime_package cubikan-runtime
    require_exact_literal scaffold pallet_package pallet-cubikan
    require_exact_literal rust channel 1.93.0
    require_exact_literal rust profile minimal
    require_exact_literal rust components rustfmt,clippy
    require_exact_literal rust target wasm32v1-none
    require_exact_literal subxt version 0.50.2
    require_exact_literal subxt family subxt,subxt-codegen,subxt-lightclient,subxt-macro,subxt-metadata,subxt-rpcs,subxt-signer,subxt-utils-accountid32,subxt-utils-fetchmetadata
    require_exact_literal zombienet revision a7c434271f094320d17cf94f7a2f95fdef417379
    require_exact_literal zombienet cli_version 1.3.138
    require_exact_literal node version 22.23.1
    require_exact_literal node npm_version 10.9.8
    require_exact_literal node platform linux-x64
    require_exact_literal node archive_symlink_count 3
    require_exact_literal repository_tools argv_grammar_version 1
    require_exact_literal foundation snapshot_format cubikan-foundation-snapshot-v1
    require_exact_literal foundation snapshot_file_count 30
    require_exact_literal foundation snapshot_external_tree_count 1

    local key
    for key in \
        frame_support:48.0.0 frame_system:48.0.0 sp_runtime:48.0.0 \
        sp_version:46.0.0 cumulus_primitives_core:0.26.0 \
        substrate_wasm_builder:34.0.0 parity_scale_codec:3.7.5 scale_info:2.11.6; do
        require_exact_literal chain_dependencies "${key%%:*}" "${key##*:}"
    done
    require_exact_literal chain_dependencies sdk_source "git+https://github.com/paritytech/polkadot-sdk.git?rev=$(pin polkadot_sdk revision)#$(pin polkadot_sdk revision)"

    require_exact_literal assets.polkadot role relay-validator
    require_exact_literal assets.polkadot-parachain role same-commit-genesis-export-compatibility
    require_exact_literal assets.polkadot-omni-node role sole-cubikan-collator-host
    require_exact_literal assets.chain-spec-builder role chain-spec
    require_exact_literal assets.frame-omni-bencher role same-commit-benchmark

    local section key_name
    for section in polkadot_sdk scaffold rust subxt root_dependency_contract zombienet node host_tools repository_tools foundation rusqlite; do
        while IFS= read -r key_name; do
            [[ -n "$key_name" ]] || continue
            pin "$section" "$key_name" >/dev/null
        done < <(/usr/bin/gawk -v section="[$section]" '
            $0 == section { active=1; next }
            /^\[/ { active=0 }
            active && /^[a-z0-9_]+[[:space:]]*=/ { key=$0; sub(/[[:space:]]*=.*/, "", key); print key }
        ' "$PINS")
    done
    local asset
    for asset in "${ASSET_NAMES[@]}"; do
        for key_name in role url size sha256 version required_commands; do
            pin "assets.$asset" "$key_name" >/dev/null
        done
    done
}

fetch_all() {
    [[ "$CARGO_HOME" == "$PROJECT_ROOT/chain/.cache/cargo-home" ]] || die "Cargo home is not the task-owned canonical path"
    [[ ! -L "$PROJECT_ROOT/chain/.cache" && ! -L "$CARGO_HOME" ]] || die "Cargo cache path is symbolic"
    /usr/lib/cargo/bin/coreutils/mkdir -p -- "$CARGO_HOME"
    for cargo_config in \
        "$PROJECT_ROOT/.cargo/config" "$PROJECT_ROOT/.cargo/config.toml" \
        "$PROJECT_ROOT/chain/.cargo/config" "$PROJECT_ROOT/chain/.cargo/config.toml" \
        "$CARGO_HOME/config" "$CARGO_HOME/config.toml"; do
        [[ ! -e "$cargo_config" && ! -L "$cargo_config" ]] || die "unapproved Cargo configuration exists: $cargo_config"
    done

    download_exact "$(pin polkadot_sdk archive_url)" "$SDK_ARCHIVE" "$(pin polkadot_sdk archive_size)" "$(pin polkadot_sdk archive_sha256)"
    download_exact "$(pin zombienet archive_url)" "$ZOMBIENET_ARCHIVE" "$(pin zombienet archive_size)" "$(pin zombienet archive_sha256)"
    download_exact "$(pin node archive_url)" "$NODE_ARCHIVE" "$(pin node archive_size)" "$(pin node archive_sha256)"
    download_exact "$(pin rusqlite archive_url)" "$RUSQLITE_ARCHIVE" "$(pin rusqlite archive_size)" "$(pin rusqlite archive_sha256)"
    local asset
    for asset in "${ASSET_NAMES[@]}"; do
        download_exact "$(pin "assets.$asset" url)" "$DOWNLOADS/$asset" "$(pin "assets.$asset" size)" "$(pin "assets.$asset" sha256)"
        # A fetched binary is never made executable until its length and digest
        # have passed. Every later use repeats both checks (including on DrvFs).
        /usr/lib/cargo/bin/coreutils/chmod 0500 -- "$DOWNLOADS/$asset"
    done

    if ! "$(pin host_tools rustup_path)" toolchain list | /usr/bin/gawk '{print $1}' | /usr/bin/grep -Ex '1\.93\.0(-x86_64-unknown-linux-gnu)?' >/dev/null; then
        "$(pin host_tools rustup_path)" toolchain install "$(pin rust channel)" --profile "$(pin rust profile)" --component rustfmt --component clippy --target "$(pin rust target)"
    else
        "$(pin host_tools rustup_path)" component add --toolchain "$(pin rust channel)" rustfmt clippy
        "$(pin host_tools rustup_path)" target add --toolchain "$(pin rust channel)" "$(pin rust target)"
    fi

    verify_toolchain
    "$(pin host_tools rustup_path)" run "$(pin rust channel)" cargo fetch --manifest-path "$PROJECT_ROOT/Cargo.toml" --locked
    "$(pin host_tools rustup_path)" run "$(pin rust channel)" cargo fetch --manifest-path "$PROJECT_ROOT/chain/Cargo.toml" --locked
    local contract_work
    contract_work="$(/usr/lib/cargo/bin/coreutils/mktemp -d "$TASK_TMP/cubikan-root-dependency-fetch.XXXXXX")"
    materialize_root_dependency_contract "$contract_work"
    "$(pin host_tools rustup_path)" run "$(pin rust channel)" cargo fetch --manifest-path "$contract_work/Cargo.toml" --locked
    /usr/bin/gnurm -rf -- "$contract_work"

    verify_registry_archive_cache
    verify_sdk_git_database

    printf '%s\n' 'verify-pins: exact fetch complete; all following work must use --locked --offline' >&2
}

verify_downloads() {
    require_size "$SDK_ARCHIVE" "$(pin polkadot_sdk archive_size)"
    require_hash "$SDK_ARCHIVE" "$(pin polkadot_sdk archive_sha256)"
    require_size "$ZOMBIENET_ARCHIVE" "$(pin zombienet archive_size)"
    require_hash "$ZOMBIENET_ARCHIVE" "$(pin zombienet archive_sha256)"
    require_size "$NODE_ARCHIVE" "$(pin node archive_size)"
    require_hash "$NODE_ARCHIVE" "$(pin node archive_sha256)"
    require_size "$RUSQLITE_ARCHIVE" "$(pin rusqlite archive_size)"
    require_hash "$RUSQLITE_ARCHIVE" "$(pin rusqlite archive_sha256)"
    local asset
    for asset in "${ASSET_NAMES[@]}"; do
        require_size "$DOWNLOADS/$asset" "$(pin "assets.$asset" size)"
        require_hash "$DOWNLOADS/$asset" "$(pin "assets.$asset" sha256)"
        [[ -x "$DOWNLOADS/$asset" ]] || die "verified asset is not executable: $asset"
    done
}

verify_registry_archive_cache() {
    local lock name version checksum
    local -a matches
    for lock in \
        "$PROJECT_ROOT/Cargo.lock" \
        "$PROJECT_ROOT/chain/Cargo.lock" \
        "$PROJECT_ROOT/$(pin foundation snapshot_path)/payload/root/lockfile.lock" \
        "$PROJECT_ROOT/$(pin foundation snapshot_path)/payload/chain/lockfile.lock" \
        "$PROJECT_ROOT/$(pin root_dependency_contract lock_path)"; do
        while IFS=$'\034' read -r name version checksum; do
            [[ -n "$checksum" ]] || continue
            matches=("$CARGO_HOME"/registry/cache/*/"$name-$version.crate")
            [[ ${#matches[@]} -eq 1 && -f "${matches[0]}" ]] || die "registry archive cache is missing or ambiguous for $name $version"
            require_hash "${matches[0]}" "$checksum"
        done < <(/usr/bin/gawk '
            /^\[\[package\]\]/{
                if (name != "" && checksum != "") print name "\034" version "\034" checksum
                name=""; version=""; checksum=""; next
            }
            /^name = /{name=$3; gsub(/"/,"",name)}
            /^version = /{version=$3; gsub(/"/,"",version)}
            /^checksum = /{checksum=$3; gsub(/"/,"",checksum)}
            END{if (name != "" && checksum != "") print name "\034" version "\034" checksum}
        ' "$lock")
    done
}

verify_sdk_git_database() {
    local git_db work archive_source git_source
    git_db="$PROJECT_ROOT/$(pin polkadot_sdk cargo_git_db_path)"
    [[ -d "$git_db" && ! -L "$git_db" ]] || die "canonical SDK Cargo git database is missing"
    "$(pin host_tools git_path)" --git-dir="$git_db" cat-file -e "$(pin polkadot_sdk revision)^{commit}" || die "SDK Cargo git database lacks the pinned commit"
    work="$(/usr/lib/cargo/bin/coreutils/mktemp -d "$TASK_TMP/cubikan-sdk-git-db.XXXXXX")"
    trap '/usr/bin/gnurm -rf -- "${work:-}"' RETURN
    require_size "$SDK_ARCHIVE" "$(pin polkadot_sdk archive_size)"
    require_hash "$SDK_ARCHIVE" "$(pin polkadot_sdk archive_sha256)"
    safe_extract "$SDK_ARCHIVE" "$work/archive" gz
    archive_source="$work/archive/polkadot-sdk-$(pin polkadot_sdk revision)"
    /usr/lib/cargo/bin/coreutils/mkdir -p -- "$work/git"
    "$(pin host_tools git_path)" --git-dir="$git_db" archive --format=tar "$(pin polkadot_sdk revision)" | /usr/bin/tar -xf - -C "$work/git" --no-same-owner --no-same-permissions
    git_source="$work/git"
    /usr/bin/diff -qr -- "$archive_source" "$git_source" >/dev/null || die "Cargo SDK git database tree differs from the verified release archive"
    /usr/bin/gnurm -rf -- "$work"
    trap - RETURN
}

verify_materialized_sdk_checkout() {
    local checkout work archive_source
    checkout="$PROJECT_ROOT/$(pin polkadot_sdk cargo_checkout_path)"
    [[ -d "$checkout" && ! -L "$checkout" ]] || die "Cargo did not materialize the pinned SDK checkout"
    work="$(/usr/lib/cargo/bin/coreutils/mktemp -d "$TASK_TMP/cubikan-sdk-checkout.XXXXXX")"
    trap '/usr/bin/gnurm -rf -- "${work:-}"' RETURN
    require_size "$SDK_ARCHIVE" "$(pin polkadot_sdk archive_size)"
    require_hash "$SDK_ARCHIVE" "$(pin polkadot_sdk archive_sha256)"
    safe_extract "$SDK_ARCHIVE" "$work/archive" gz
    archive_source="$work/archive/polkadot-sdk-$(pin polkadot_sdk revision)"
    /usr/bin/diff -qr --exclude=.git --exclude=.cargo-ok "$archive_source" "$checkout" >/dev/null || die "materialized Cargo SDK checkout differs from the verified release archive"
    /usr/bin/gnurm -rf -- "$work"
    trap - RETURN
}

verify_repository_tool_bytes() {
    require_hash "$PROJECT_ROOT/$(pin repository_tools argv_grammar_path)" "$(pin repository_tools argv_grammar_sha256)"
    if [[ -z "$NORMALIZER_FD_PATH" ]]; then
        bind_verified_repository_executable \
            "$PROJECT_ROOT/$(pin repository_tools argv_normalizer_path)" \
            "$(pin repository_tools argv_normalizer_sha256)" argv-normalizer \
            NORMALIZER_FD NORMALIZER_FD_PATH
    else
        [[ -f "$NORMALIZER_FD_PATH" ]] || die "argv normalizer descriptor is unavailable"
    fi
    if [[ -z "$LOOPBACK_FD_PATH" ]]; then
        bind_verified_repository_executable \
            "$PROJECT_ROOT/$(pin repository_tools loopback_wrapper_path)" \
            "$(pin repository_tools loopback_wrapper_sha256)" namespace-wrapper \
            LOOPBACK_FD LOOPBACK_FD_PATH
    else
        [[ -f "$LOOPBACK_FD_PATH" ]] || die "namespace wrapper descriptor is unavailable"
    fi
}

verify_repository_tool_behavior() {
    [[ -n "$NORMALIZER_FD" ]] || die "argv normalizer descriptor is unavailable"
    CUBIKAN_NORMALIZER_SANITIZED=1 "$(pin host_tools bash_path)" --noprofile --norc -p -s -- \
        __cubikan_normalizer_bound_path_v1__ "$TOOL_DIR/normalize-node-argv.sh" \
        --verify-grammar <&"$NORMALIZER_FD" >/dev/null
}

verify_host_tool_bytes() {
    local path expected_path
    for key in \
        bash unshare ip ss netcat git rustup env awk sha256sum dd mount stat tar \
        patch diff find sort iconv uname dirname readlink sed grep head wc cp rm \
        mkdir mktemp curl chmod mv; do
        path="$(pin host_tools "${key}_path")"
        expected_path="$(/usr/lib/cargo/bin/coreutils/readlink -f -- "$path")"
        [[ "$path" == "$expected_path" ]] || die "pinned host executable path is not canonical: $path"
        [[ -x "$path" ]] || die "pinned host executable is unavailable: $path"
        require_hash "$path" "$(pin host_tools "${key}_sha256")"
    done
}

verify_host_tool_behavior() {
    verify_current_network_namespace
    [[ "$("$(pin host_tools bash_path)" --version | /usr/lib/cargo/bin/coreutils/head -1)" == "$(pin host_tools bash_version)" ]] || die "Bash version mismatch"
    [[ "$("$(pin host_tools unshare_path)" --version | /usr/lib/cargo/bin/coreutils/head -1)" == "unshare from util-linux $(pin host_tools util_linux_version)" ]] || die "util-linux version mismatch"
    [[ "$("$(pin host_tools ip_path)" -V 2>&1)" == *"iproute2-$(pin host_tools iproute2_version)"* ]] || die "iproute2 version mismatch"
    [[ "$("$(pin host_tools ss_path)" -V 2>&1)" == *"iproute2-$(pin host_tools iproute2_version)"* ]] || die "ss version mismatch"
    [[ "$("$(pin host_tools git_path)" --version)" == "git version $(pin host_tools git_version)" ]] || die "Git version mismatch"
    [[ "$("$(pin host_tools netcat_path)" -h 2>&1 | /usr/lib/cargo/bin/coreutils/head -1)" == "OpenBSD netcat (Debian patchlevel $(pin host_tools netcat_version))" ]] || die "netcat version mismatch"
}

verify_current_network_namespace() {
    local ip nc bad_links bad_routes listener_pid="" loopback_ok=0 attempt
    ip="$(pin host_tools ip_path)"
    nc="$(pin host_tools netcat_path)"
    bad_links="$("$ip" -o link show | /usr/bin/gawk -F': ' '$2 !~ /^lo(@|$)/ { print }')"
    [[ -z "$bad_links" ]] || die "current namespace has a non-loopback interface"
    bad_routes="$({ "$ip" -4 route show table all; "$ip" -6 route show table all; } | /usr/bin/gawk 'NF && $0 !~ /(^|[[:space:]])dev lo([[:space:]]|$)/ { print }')"
    [[ -z "$bad_routes" ]] || die "current namespace has a non-loopback route"
    if "$nc" -z -w 1 192.0.2.1 9 >/dev/null 2>&1; then
        die "current namespace external-connect probe unexpectedly succeeded"
    fi
    "$nc" -l 127.0.0.1 39582 </dev/null >/dev/null 2>&1 &
    listener_pid=$!
    trap 'if [[ -n "${listener_pid:-}" ]]; then kill "$listener_pid" 2>/dev/null || true; wait "$listener_pid" 2>/dev/null || true; fi' RETURN
    for attempt in {1..40}; do
        if "$nc" -z -w 1 127.0.0.1 39582 >/dev/null 2>&1; then loopback_ok=1; break; fi
    done
    [[ $loopback_ok -eq 1 ]] || die "current namespace loopback-connect probe failed"
    wait "$listener_pid" 2>/dev/null || true
    listener_pid=""
    trap - RETURN
}

safe_extract() {
    local archive="$1" destination="$2" compression="$3" symlink_policy="${4:-contained}"
    local names modes archive_root root_path link resolved
    /usr/lib/cargo/bin/coreutils/mkdir -p -- "$destination"
    names="$(/usr/lib/cargo/bin/coreutils/mktemp)"
    modes="$(/usr/lib/cargo/bin/coreutils/mktemp)"
    if [[ "$compression" == gz ]]; then
        /usr/bin/tar -tzf "$archive" >"$names"
        /usr/bin/tar -tvzf "$archive" --quoting-style=escape >"$modes"
    else
        /usr/bin/tar -tJf "$archive" >"$names"
        /usr/bin/tar -tvJf "$archive" --quoting-style=escape >"$modes"
    fi
    while IFS= read -r entry; do
        [[ -n "$entry" && "$entry" != /* && "$entry" != ../* && "$entry" != *'/../'* && "$entry" != *$'\n'* ]] || die "unsafe archive member in $archive"
    done <"$names"
    # The first mode byte identifies the only accepted member types. GNU tar
    # prints '-' (regular), 'd' (directory), or 'l' (symbolic link). Hard links,
    # block/char devices, FIFOs, and sockets reject before extraction.
    while IFS= read -r line; do
        case "${line:0:1}" in -|d|l) ;; *) die "archive contains a forbidden member type: $archive" ;; esac
    done <"$modes"
    if [[ "$compression" == gz ]]; then
        /usr/bin/tar -xzf "$archive" -C "$destination" --no-same-owner --no-same-permissions
    else
        /usr/bin/tar -xJf "$archive" -C "$destination" --no-same-owner --no-same-permissions
    fi
    # GNU tar listing above and the exact archive digest lock member bytes. After
    # extraction, every symlink must resolve to an existing path under the one
    # archive root. Devices, FIFOs, sockets, and hard links remain forbidden.
    IFS= read -r first_entry <"$names" || die "archive is empty: $archive"
    archive_root="${first_entry%%/*}"
    [[ -n "$archive_root" && "$archive_root" != . && "$archive_root" != .. ]] || die "archive root is invalid: $archive"
    root_path="$(/usr/lib/cargo/bin/coreutils/readlink -f -- "$destination/$archive_root")"
    [[ -d "$root_path" ]] || die "archive has no exact root directory: $archive_root"
    while IFS= read -r -d '' link; do
        resolved="$(/usr/lib/cargo/bin/coreutils/readlink -f -- "$link")" || die "archive has a dangling symlink: $link"
        [[ "$resolved" == "$root_path"/* && -e "$resolved" ]] || die "archive symlink escapes its root: $link"
    done < <(/usr/bin/find "$destination" -type l -print0)

    if [[ "$symlink_policy" == node ]]; then
        local node_root allowed actual
        node_root="$destination/$(pin node archive_directory)"
        allowed="$(pin node archive_symlink_inventory)"
        allowed="${allowed//,/$'\n'}"
        actual="$(/usr/bin/find "$node_root" -type l -printf '%P -> %l\n' | /usr/lib/cargo/bin/coreutils/sort)"
        [[ "$actual" == "$allowed" ]] || die "Node archive symlink inventory mismatch"
        [[ "$(/usr/bin/find "$node_root" -type l | /usr/lib/cargo/bin/coreutils/wc -l)" == "$(pin node archive_symlink_count)" ]] || die "Node archive symlink count mismatch"
    elif [[ "$archive" == "$SDK_ARCHIVE" ]]; then
        [[ "$(/usr/bin/find "$root_path" -type l | /usr/lib/cargo/bin/coreutils/wc -l)" == "$(pin polkadot_sdk archive_symlink_count)" ]] || die "SDK archive symlink count mismatch"
    elif [[ "$symlink_policy" != contained ]]; then
        die "internal unsupported symlink policy: $symlink_policy"
    fi
    [[ -z "$(/usr/bin/find "$destination" ! -type d ! -type f ! -type l -print -quit)" ]] || die "archive contains a forbidden nonregular member: $archive"
    /usr/bin/gnurm -f -- "$names" "$modes"
}

verify_sdk_and_scaffold() {
    local work source file actual manifest
    require_size "$SDK_ARCHIVE" "$(pin polkadot_sdk archive_size)"
    require_hash "$SDK_ARCHIVE" "$(pin polkadot_sdk archive_sha256)"
    work="$(/usr/lib/cargo/bin/coreutils/mktemp -d)"
    trap '/usr/bin/gnurm -rf -- "${work:-}"' RETURN
    safe_extract "$SDK_ARCHIVE" "$work" gz
    source="$work/polkadot-sdk-$(pin polkadot_sdk revision)"
    [[ -d "$source" ]] || die "SDK archive root identity mismatch"
    require_hash "$source/Cargo.lock" "$(pin polkadot_sdk cargo_lock_sha256)"
    manifest="$(/usr/lib/cargo/bin/coreutils/mktemp)"
    for tuple in \
        runtime_manifest_path:runtime_manifest_sha256 \
        runtime_build_path:runtime_build_sha256 \
        runtime_source_path:runtime_source_sha256 \
        pallet_manifest_path:pallet_manifest_sha256 \
        pallet_source_path:pallet_source_sha256; do
        file="$(pin scaffold "${tuple%%:*}")"
        require_hash "$source/$file" "$(pin scaffold "${tuple##*:}")"
        printf '%s  %s\n' "$(sha256_file "$source/$file")" "$file" >>"$manifest"
    done
    actual="$(sha256_file "$manifest")"
    [[ "$actual" == "$(pin scaffold selected_files_manifest_sha256)" ]] || die "same-commit scaffold manifest mismatch"
    /usr/bin/gnurm -rf -- "$work" "$manifest"
    trap - RETURN
}

verify_node_and_zombienet() {
    local work node_root zombie_root node_bin npm_bin
    require_size "$NODE_ARCHIVE" "$(pin node archive_size)"
    require_hash "$NODE_ARCHIVE" "$(pin node archive_sha256)"
    require_size "$ZOMBIENET_ARCHIVE" "$(pin zombienet archive_size)"
    require_hash "$ZOMBIENET_ARCHIVE" "$(pin zombienet archive_sha256)"
    work="$(/usr/lib/cargo/bin/coreutils/mktemp -d)"
    trap '/usr/bin/gnurm -rf -- "${work:-}"' RETURN
    safe_extract "$NODE_ARCHIVE" "$work/node" xz node
    node_root="$work/node/$(pin node archive_directory)"
    node_bin="$node_root/bin/node"
    npm_bin="$node_root/lib/node_modules/npm/bin/npm-cli.js"
    [[ "$($node_bin --version)" == "v$(pin node version)" ]] || die "Node version mismatch"
    [[ "$($node_bin "$npm_bin" --version)" == "$(pin node npm_version)" ]] || die "npm version mismatch"

    safe_extract "$ZOMBIENET_ARCHIVE" "$work/zombienet" gz
    zombie_root="$work/zombienet/zombienet-$(pin zombienet revision)"
    require_hash "$zombie_root/$(pin zombienet package_lock_path)" "$(pin zombienet package_lock_sha256)"
    /usr/bin/grep -F '"version": "1.3.138"' "$zombie_root/javascript/packages/cli/package.json" >/dev/null || die "Zombienet CLI version mismatch"
    /usr/bin/gnurm -rf -- "$work"
    trap - RETURN
}

verify_asset_capabilities() {
    local asset output expected_version command
    for asset in "${ASSET_NAMES[@]}"; do
        require_size "$DOWNLOADS/$asset" "$(pin "assets.$asset" size)"
        require_hash "$DOWNLOADS/$asset" "$(pin "assets.$asset" sha256)"
        output="$("$DOWNLOADS/$asset" --version 2>&1)"
        # Release artifacts may include a filename-derived argv[0] prefix; the
        # pinned semantic version must still appear exactly.
        [[ "$output" == *"$(pin "assets.$asset" version)"* || "$output" == *"${asset#polkadot-} $(pin polkadot_sdk node_version)-8ae9775dc43"* ]] || die "$asset version mismatch: $output"
        output="$("$DOWNLOADS/$asset" --help 2>&1)"
        IFS=',' read -r -a commands <<<"$(pin "assets.$asset" required_commands)"
        for command in "${commands[@]}"; do
            [[ "$output" == *"$command"* ]] || die "$asset lacks required command $command"
        done
    done
}

verify_root_dependency_contract_bytes() {
    local fixture_root actual_family expected_family
    fixture_root="$PROJECT_ROOT/chain/tools/fixtures/root-dependency-contract-v1"
    require_hash "$PROJECT_ROOT/$(pin root_dependency_contract manifest_path)" "$(pin root_dependency_contract manifest_sha256)"
    require_hash "$PROJECT_ROOT/$(pin root_dependency_contract lock_path)" "$(pin root_dependency_contract lock_sha256)"
    require_hash "$PROJECT_ROOT/$(pin root_dependency_contract source_path)" "$(pin root_dependency_contract source_sha256)"

    actual_family="$(/usr/bin/gawk '
        /^\[\[package\]\]/{name=""}
        /^name = "subxt"$/ || /^name = "subxt-/ {name=$3; gsub(/"/,"",name)}
        name != "" && /^version = /{version=$3; gsub(/"/,"",version); print name, version; name=""}
    ' "$fixture_root/lockfile.lock" | /usr/lib/cargo/bin/coreutils/sort)"
    expected_family=$'subxt 0.50.2\nsubxt-codegen 0.50.2\nsubxt-lightclient 0.50.2\nsubxt-macro 0.50.2\nsubxt-metadata 0.50.2\nsubxt-rpcs 0.50.2\nsubxt-signer 0.50.2\nsubxt-utils-accountid32 0.50.2\nsubxt-utils-fetchmetadata 0.50.2'
    [[ "$actual_family" == "$expected_family" ]] || die "future root Subxt family is not the exact nonempty 0.50.2 set"
}

materialize_root_dependency_contract() {
    local destination="$1"
    /usr/lib/cargo/bin/coreutils/mkdir -p -- "$destination"
    /usr/bin/gnucp -- "$PROJECT_ROOT/$(pin root_dependency_contract manifest_path)" "$destination/Cargo.toml"
    /usr/bin/gnucp -- "$PROJECT_ROOT/$(pin root_dependency_contract lock_path)" "$destination/Cargo.lock"
    /usr/bin/gnucp -- "$PROJECT_ROOT/$(pin root_dependency_contract source_path)" "$destination/lib.rs"
}

verify_root_dependency_contract_resolution() {
    local work actual
    work="$(/usr/lib/cargo/bin/coreutils/mktemp -d "$TASK_TMP/cubikan-root-dependency-contract.XXXXXX")"
    trap '/usr/bin/gnurm -rf -- "${work:-}"' RETURN
    materialize_root_dependency_contract "$work"
    actual="$("$(pin host_tools rustup_path)" run "$(pin rust channel)" cargo --color never tree --manifest-path "$work/Cargo.toml" -e features --locked --offline | /usr/bin/sed -E "s#$PROJECT_ROOT#<project>#g; 1s# \\([^)]*\\)\$# (<fixture>)#" | /usr/lib/cargo/bin/coreutils/sha256sum | /usr/bin/gawk '{print $1}')"
    [[ "$actual" == "$(pin root_dependency_contract feature_tree_sha256)" ]] || die "future root dependency feature closure mismatch"
    /usr/bin/gnurm -rf -- "$work"
    trap - RETURN
}

verify_rusqlite_bytes() {
    require_hash "$PROJECT_ROOT/$(pin rusqlite patch_path)" "$(pin rusqlite patch_sha256)"
    local actual
    actual="$(tree_sha256 "$PROJECT_ROOT/$(pin rusqlite vendor_path)")"
    [[ "$actual" == "$(pin rusqlite patched_tree_sha256)" ]] || die "checked rusqlite vendor tree mismatch"
}

verify_rusqlite_reconstruction() {
    local work pristine patched diff_file actual
    require_size "$RUSQLITE_ARCHIVE" "$(pin rusqlite archive_size)"
    require_hash "$RUSQLITE_ARCHIVE" "$(pin rusqlite archive_sha256)"
    work="$(/usr/lib/cargo/bin/coreutils/mktemp -d)"
    trap '/usr/bin/gnurm -rf -- "${work:-}"' RETURN
    safe_extract "$RUSQLITE_ARCHIVE" "$work/pristine" gz
    pristine="$work/pristine/rusqlite-$(pin rusqlite version)"
    actual="$(tree_sha256 "$pristine")"
    [[ "$actual" == "$(pin rusqlite pristine_tree_sha256)" ]] || die "rusqlite pristine tree mismatch"
    /usr/bin/gnucp -a -- "$pristine" "$work/patched"
    patched="$work/patched"
    /usr/bin/patch --batch --forward --directory "$patched" --strip 1 <"$PROJECT_ROOT/$(pin rusqlite patch_path)" >/dev/null
    actual="$(tree_sha256 "$patched")"
    [[ "$actual" == "$(pin rusqlite patched_tree_sha256)" ]] || die "rusqlite reconstructed tree mismatch"
    /usr/bin/diff -qr -- "$patched" "$PROJECT_ROOT/$(pin rusqlite vendor_path)" >/dev/null || die "reconstructed vendor is not byte-identical"
    diff_file="$work/normalized.diff"
    if /usr/bin/diff -u --label a/src/hooks/mod.rs --label b/src/hooks/mod.rs "$pristine/src/hooks/mod.rs" "$PROJECT_ROOT/$(pin rusqlite vendor_path)/src/hooks/mod.rs" >"$diff_file"; then
        die "rusqlite normalized diff unexpectedly empty"
    else
        [[ $? -eq 1 ]] || die "failed to construct normalized rusqlite diff"
    fi
    require_hash "$diff_file" "$(pin rusqlite normalized_diff_sha256)"
    /usr/bin/gnurm -rf -- "$work"
    trap - RETURN
}

verify_toolchain() {
    local rustup rustc_details cargo_details actual
    rustup="$(pin host_tools rustup_path)"
    require_hash "$rustup" "$(pin host_tools rustup_sha256)"
    actual="$(tree_sha256 "$(pin rust toolchain_path)")"
    [[ "$actual" == "$(pin rust toolchain_tree_sha256)" ]] || die "Rust toolchain tree identity mismatch"
    actual="$(tree_sha256 "$(pin rust wasm_target_path)")"
    [[ "$actual" == "$(pin rust wasm_target_tree_sha256)" ]] || die "wasm32v1-none target tree identity mismatch"

    rustc_details="$("$rustup" run "$(pin rust channel)" rustc -Vv)"
    [[ "$rustc_details" == *"commit-hash: $(pin rust rustc_commit)"* ]] || die "rustc identity mismatch"
    [[ "$rustc_details" == *"host: $(pin rust host)"* ]] || die "rustc host mismatch"
    [[ "$rustc_details" == *"LLVM version: $(pin rust llvm_version)"* ]] || die "LLVM identity mismatch"
    cargo_details="$("$rustup" run "$(pin rust channel)" cargo -Vv)"
    [[ "$cargo_details" == *"commit-hash: $(pin rust cargo_commit)"* ]] || die "Cargo identity mismatch"
    [[ "$("$rustup" run "$(pin rust channel)" rustfmt --version)" == "$(pin rust rustfmt_version)" ]] || die "rustfmt identity mismatch"
    [[ "$("$rustup" run "$(pin rust channel)" cargo clippy -V)" == "$(pin rust clippy_version)" ]] || die "Clippy identity mismatch"
    "$rustup" target list --toolchain "$(pin rust channel)" --installed | /usr/bin/grep -Fx "$(pin rust target)" >/dev/null || die "Wasm target is not installed"
}

prepare_verified_cargo_sources() {
    local cargo_config
    [[ "$CARGO_HOME" == "$PROJECT_ROOT/chain/.cache/cargo-home" ]] || die "Cargo home is not the task-owned canonical path"
    [[ -d "$CARGO_HOME" && ! -L "$CARGO_HOME" ]] || die "canonical Cargo home is missing or symbolic"
    for cargo_config in \
        "$PROJECT_ROOT/.cargo/config" "$PROJECT_ROOT/.cargo/config.toml" \
        "$PROJECT_ROOT/chain/.cargo/config" "$PROJECT_ROOT/chain/.cargo/config.toml" \
        "$CARGO_HOME/config" "$CARGO_HOME/config.toml"; do
        [[ ! -e "$cargo_config" && ! -L "$cargo_config" ]] || die "unapproved Cargo configuration exists: $cargo_config"
    done
    verify_registry_archive_cache
    verify_sdk_git_database
    # These two directories are derived, task-owned materializations. Removing
    # them forces Cargo to reconstruct every source used below from the
    # just-verified registry archives and SDK git object database.
    /usr/bin/gnurm -rf -- "$CARGO_HOME/registry/src" "$CARGO_HOME/git/checkouts"
}

verify_foundation_snapshot_bytes() {
    local snapshot inventory actual
    snapshot="$PROJECT_ROOT/$(pin foundation snapshot_path)"
    inventory="$PROJECT_ROOT/$(pin foundation snapshot_inventory_path)"
    [[ "$inventory" == "$snapshot/inventory.toml" ]] || die "foundation inventory path is outside its snapshot"
    require_hash "$inventory" "$(pin foundation snapshot_inventory_sha256)"
    actual="$(tree_sha256 "$snapshot")"
    [[ "$actual" == "$(pin foundation snapshot_tree_sha256)" ]] || die "foundation snapshot tree identity mismatch"

    /usr/bin/grep -Fx "format = \"$(pin foundation snapshot_format)\"" "$inventory" >/dev/null || die "foundation snapshot format drift"
    /usr/bin/grep -Fx "file_count = $(pin foundation snapshot_file_count)" "$inventory" >/dev/null || die "foundation snapshot file count drift"
    /usr/bin/grep -Fx "external_tree_count = $(pin foundation snapshot_external_tree_count)" "$inventory" >/dev/null || die "foundation snapshot external-tree count drift"
    /usr/bin/grep -Fx "root_feature_tree_sha256 = \"$(pin foundation root_feature_tree_sha256)\"" "$inventory" >/dev/null || die "foundation root proof hash drift"
    /usr/bin/grep -Fx "chain_feature_tree_sha256 = \"$(pin foundation chain_feature_tree_sha256)\"" "$inventory" >/dev/null || die "foundation chain proof hash drift"
    /usr/bin/grep -Fx "runtime_wasm_sha256 = \"$(pin foundation runtime_wasm_sha256)\"" "$inventory" >/dev/null || die "foundation Wasm proof hash drift"

    require_hash "$snapshot/payload/root/workspace-manifest.toml" "$(pin foundation root_manifest_sha256)"
    require_hash "$snapshot/payload/root/lockfile.lock" "$(pin foundation root_lock_sha256)"
    require_hash "$snapshot/payload/root/gitignore" "$(pin foundation gitignore_sha256)"
    require_hash "$snapshot/payload/root/rust-toolchain.toml" "$(pin foundation root_toolchain_sha256)"
    require_hash "$snapshot/payload/root/github/workflows/ci.yml" "$(pin foundation root_ci_sha256)"
    require_hash "$snapshot/payload/chain/workspace-manifest.toml" "$(pin foundation chain_manifest_sha256)"
    require_hash "$snapshot/payload/chain/lockfile.lock" "$(pin foundation chain_lock_sha256)"
    require_hash "$snapshot/payload/chain/rust-toolchain.toml" "$(pin foundation chain_toolchain_sha256)"
    require_hash "$snapshot/payload/chain/pallets/cubikan/manifest.toml" "$(pin foundation chain_pallet_manifest_sha256)"
    require_hash "$snapshot/payload/chain/pallets/cubikan/src/lib.rs" "$(pin foundation chain_pallet_source_sha256)"
    require_hash "$snapshot/payload/chain/runtime/manifest.toml" "$(pin foundation chain_runtime_manifest_sha256)"
    require_hash "$snapshot/payload/chain/runtime/build.rs" "$(pin foundation chain_runtime_build_sha256)"
    require_hash "$snapshot/payload/chain/runtime/src/lib.rs" "$(pin foundation chain_runtime_source_sha256)"
    require_size "$PROJECT_ROOT/$(pin foundation runtime_wasm_reference_path)" "$(pin foundation runtime_wasm_size)"
    require_hash "$PROJECT_ROOT/$(pin foundation runtime_wasm_reference_path)" "$(pin foundation runtime_wasm_sha256)"
}

verify_live_root_dependency_boundary() {
    local actual expected manifest_subxt=0
    expected=$'subxt 0.50.2\nsubxt-codegen 0.50.2\nsubxt-lightclient 0.50.2\nsubxt-macro 0.50.2\nsubxt-metadata 0.50.2\nsubxt-rpcs 0.50.2\nsubxt-signer 0.50.2\nsubxt-utils-accountid32 0.50.2\nsubxt-utils-fetchmetadata 0.50.2'
    if /usr/bin/grep -E '^[[:space:]]*subxt(-signer)?[[:space:]]*=' "$PROJECT_ROOT/Cargo.toml" >/dev/null; then
        manifest_subxt=1
    fi
    actual="$(/usr/bin/gawk '
        /^\[\[package\]\]/{name=""}
        /^name = "subxt"$/ || /^name = "subxt-/ {name=$3; gsub(/"/,"",name)}
        name != "" && /^version = /{version=$3; gsub(/"/,"",version); print name, version; name=""}
    ' "$PROJECT_ROOT/Cargo.lock" | /usr/lib/cargo/bin/coreutils/sort)"
    if [[ -z "$actual" ]]; then
        [[ $manifest_subxt -eq 0 ]] || die "root declares Subxt without its exact locked family"
        return
    fi
    [[ $manifest_subxt -eq 1 ]] || die "root lock contains Subxt without an approved direct declaration"
    /usr/bin/gawk '
        function emit() {
            if (name ~ /^subxt($|-)/ && source != "registry+https://github.com/rust-lang/crates.io-index") bad=1
        }
        /^\[\[package\]\]$/ {emit(); name=""; source=""; next}
        /^name = / {name=$3; gsub(/"/, "", name); next}
        /^source = / {source=$3; gsub(/"/, "", source); next}
        END {emit(); exit bad ? 1 : 0}
    ' "$PROJECT_ROOT/Cargo.lock" || die "root Subxt lock family contains a nonregistry source"
    [[ "$actual" == "$expected" ]] || die "root Subxt lock family is not the closed 0.50.2 set"

    for declaration in \
        'subxt = { version = "=0.50.2", default-features = false, features = ["jsonrpsee", "native"] }' \
        'subxt-signer = { version = "=0.50.2", default-features = false, features = ["sr25519", "subxt"] }' \
        'tokio = { version = "=1.47.1", default-features = false, features = ["rt", "macros", "time", "sync"] }' \
        'futures-util = { version = "=0.3.31", default-features = false, features = ["std"] }' \
        'url = { version = "=2.5.8", default-features = false, features = ["std"] }' \
        'parity-scale-codec = { version = "=3.7.5", default-features = false, features = ["derive", "std"] }' \
        'scale-info = { version = "=2.11.6", default-features = false, features = ["derive", "std"] }' \
        'rustix = { version = "=1.1.4", default-features = false, features = ["fs", "process", "std"] }' \
        'sha2 = { version = "=0.10.9", default-features = false, features = ["std"] }'; do
        /usr/bin/grep -Fx "$declaration" "$PROJECT_ROOT/Cargo.toml" >/dev/null || die "final root dependency declaration drift: $declaration"
    done
    actual="$(/usr/bin/gawk '
        /^\[workspace.dependencies\]$/ {active=1; next}
        /^\[/ {active=0}
        active && /^[A-Za-z0-9_-]+[[:space:]]*=/ {key=$0; sub(/[[:space:]]*=.*/, "", key); print key}
    ' "$PROJECT_ROOT/Cargo.toml" | /usr/lib/cargo/bin/coreutils/sort)"
    expected=$'futures-util\nparity-scale-codec\nrusqlite\nrustix\nscale-info\nserde\nserde_json\nsha2\nsubxt\nsubxt-signer\ntokio\nurl\nuuid'
    [[ "$actual" == "$expected" ]] || die "final root direct dependency set is not closed"

    # T-1110/T-1117 final-candidate seam: project the realized root workspace's
    # external graph (excluding workspace-member topology) and require equality
    # with root_dependency_contract.feature_tree_sha256. The immutable contract
    # is already resolved and checked on every run; those tasks must connect the
    # live projection here rather than replacing or weakening that proof.
}

verify_lock_and_manifest_bytes() {
    local source expected_source git_bin plan_commit actual sdk_source_count=0
    git_bin="$(pin host_tools git_path)"
    plan_commit="$(pin foundation plan_commit)"
    "$git_bin" -C "$PROJECT_ROOT" merge-base --is-ancestor "$plan_commit" HEAD || die "approved Plan commit is not an ancestor"
    actual="$("$git_bin" -C "$PROJECT_ROOT" show "$plan_commit:Cargo.toml" | /usr/lib/cargo/bin/coreutils/sha256sum | /usr/bin/gawk '{print $1}')"
    [[ "$actual" == "$(pin foundation base_root_manifest_sha256)" ]] || die "accepted-base root manifest identity mismatch"
    actual="$("$git_bin" -C "$PROJECT_ROOT" show "$plan_commit:Cargo.lock" | /usr/lib/cargo/bin/coreutils/sha256sum | /usr/bin/gawk '{print $1}')"
    [[ "$actual" == "$(pin foundation base_root_lock_sha256)" ]] || die "accepted-base root lock identity mismatch"
    actual="$("$git_bin" -C "$PROJECT_ROOT" show "$plan_commit:.gitignore" | /usr/lib/cargo/bin/coreutils/sha256sum | /usr/bin/gawk '{print $1}')"
    [[ "$actual" == "$(pin foundation base_gitignore_sha256)" ]] || die "accepted-base gitignore identity mismatch"
    if "$git_bin" -C "$PROJECT_ROOT" cat-file -e "$plan_commit:rust-toolchain.toml" 2>/dev/null; then
        die "accepted base unexpectedly contains a root toolchain file"
    fi

    verify_foundation_snapshot_bytes

    # These candidate surfaces are intentionally immutable for the full Build.
    # Evolvable manifests, locks, and Rust sources are proved by the sealed
    # foundation fixture plus the live semantic boundary below instead.
    require_hash "$PROJECT_ROOT/.gitignore" "$(pin foundation gitignore_sha256)"
    require_hash "$PROJECT_ROOT/rust-toolchain.toml" "$(pin foundation root_toolchain_sha256)"
    require_hash "$PROJECT_ROOT/chain/rust-toolchain.toml" "$(pin foundation chain_toolchain_sha256)"
    require_hash "$PROJECT_ROOT/.github/workflows/ci.yml" "$(pin foundation root_ci_sha256)"

    /usr/bin/grep -F 'exclude = ["chain", "vendor/rusqlite-0.40.2-cubikan"]' "$PROJECT_ROOT/Cargo.toml" >/dev/null || die "root chain/vendor isolation drift"
    /usr/bin/grep -F 'rusqlite = { path = "vendor/rusqlite-0.40.2-cubikan" }' "$PROJECT_ROOT/Cargo.toml" >/dev/null || die "rusqlite patch override drift"
    /usr/bin/gawk '
        /^\[\[package\]\]/{inside=0}
        /^name = "rusqlite"$/{inside=1}
        inside && /^version = / && $0 != "version = \"0.40.2\""{bad=1}
        inside && /^(source|checksum) = /{bad=1}
        END{exit bad ? 1 : 0}
    ' "$PROJECT_ROOT/Cargo.lock" || die "root rusqlite lock entry is not a local path identity"

    expected_source="$(pin chain_dependencies sdk_source)"
    while IFS= read -r source; do
        [[ -n "$source" ]] || continue
        sdk_source_count=$((sdk_source_count + 1))
        [[ "$source" == "$expected_source" ]] || die "chain lock has an unpinned Polkadot SDK source: $source"
    done < <(/usr/bin/gawk -F'"' '/^source = "git\+/{print $2}' "$PROJECT_ROOT/chain/Cargo.lock" | /usr/lib/cargo/bin/coreutils/sort -u)
    [[ $sdk_source_count -eq 1 ]] || die "chain lock does not contain exactly one pinned git source identity"
    [[ -z "$(/usr/bin/grep -En 'polkadot-sdk|paritytech/(substrate|cumulus)' "$PROJECT_ROOT/Cargo.lock" || true)" ]] || die "Polkadot SDK source leaked into root lock"
    verify_live_root_dependency_boundary

    /usr/bin/grep -F 'edition = "2021"' "$PROJECT_ROOT/chain/Cargo.toml" >/dev/null || die "chain edition drift"
    /usr/bin/grep -F 'resolver = "2"' "$PROJECT_ROOT/chain/Cargo.toml" >/dev/null || die "chain resolver drift"
    /usr/bin/grep -F 'rust-version = "1.93.0"' "$PROJECT_ROOT/chain/Cargo.toml" >/dev/null || die "chain Rust version drift"
    /usr/bin/grep -F 'polkadot-sdk-release = "polkadot-stable2606-1"' "$PROJECT_ROOT/chain/Cargo.toml" >/dev/null || die "chain SDK release drift"
    /usr/bin/grep -F "polkadot-sdk-revision = \"$(pin polkadot_sdk revision)\"" "$PROJECT_ROOT/chain/Cargo.toml" >/dev/null || die "chain SDK revision drift"
    /usr/bin/gawk -v revision="$(pin polkadot_sdk revision)" '
        /git[[:space:]]*=/ {
            if ($0 !~ /git = "https:\/\/github.com\/paritytech\/polkadot-sdk.git"/) bad=1
            expected="rev = \"" revision "\""
            if (index($0, expected) == 0) bad=1
        }
        END{exit bad ? 1 : 0}
    ' "$PROJECT_ROOT/chain/Cargo.toml" || die "chain manifest contains an unpinned or non-SDK git dependency"
    /usr/bin/grep -F 'channel = "1.93.0"' "$PROJECT_ROOT/rust-toolchain.toml" >/dev/null || die "root toolchain drift"
    /usr/bin/grep -F 'channel = "1.93.0"' "$PROJECT_ROOT/chain/rust-toolchain.toml" >/dev/null || die "chain toolchain drift"
    /usr/bin/grep -F 'targets = ["wasm32v1-none"]' "$PROJECT_ROOT/chain/rust-toolchain.toml" >/dev/null || die "chain Wasm target drift"
}

materialize_foundation_snapshot() {
    local destination="$1" snapshot inventory source target expected_size expected_hash mode parent copied=0
    local vendor_source vendor_target actual
    declare -A copied_targets=()
    snapshot="$PROJECT_ROOT/$(pin foundation snapshot_path)"
    inventory="$PROJECT_ROOT/$(pin foundation snapshot_inventory_path)"
    [[ -d "$destination" && ! -L "$destination" ]] || die "foundation materialization root is missing or symbolic"
    [[ -z "$(/usr/bin/find "$destination" -mindepth 1 -print -quit)" ]] || die "foundation materialization root is not empty"

    while IFS=$'\034' read -r source target expected_size expected_hash mode; do
        [[ -n "$source" && -n "$target" ]] || die "foundation inventory contains an incomplete materialization entry"
        [[ "$source" != /* && "$source" != ../* && "$source" != */../* && "$source" != *$'\n'* ]] || die "unsafe foundation source path"
        [[ "$target" != /* && "$target" != ../* && "$target" != */../* && "$target" != *$'\n'* ]] || die "unsafe foundation target path"
        [[ -z "${copied_targets[$target]:-}" ]] || die "duplicate foundation materialization target: $target"
        copied_targets["$target"]=1
        [[ "$mode" == 0644 ]] || die "unsupported foundation materialization mode: $mode"
        require_size "$snapshot/$source" "$expected_size"
        require_hash "$snapshot/$source" "$expected_hash"
        parent="$(/usr/lib/cargo/bin/coreutils/dirname -- "$destination/$target")"
        /usr/lib/cargo/bin/coreutils/mkdir -p -- "$parent"
        /usr/bin/gnucp -- "$snapshot/$source" "$destination/$target"
        /usr/lib/cargo/bin/coreutils/chmod "$mode" -- "$destination/$target"
        require_size "$destination/$target" "$expected_size"
        require_hash "$destination/$target" "$expected_hash"
        copied=$((copied + 1))
    done < <(/usr/bin/gawk '
        function unquote(line) {sub(/^[^=]*=[[:space:]]*"/, "", line); sub(/"[[:space:]]*$/, "", line); return line}
        function number(line) {sub(/^[^=]*=[[:space:]]*/, "", line); return line}
        function emit() {
            if (active && target != "") print source "\034" target "\034" bytes "\034" hash "\034" mode
        }
        $0 == "[[files]]" {emit(); active=1; source=""; target=""; bytes=""; hash=""; mode=""; next}
        /^\[\[/ {emit(); active=0; next}
        active && /^source = / {source=unquote($0); next}
        active && /^target = / {target=unquote($0); next}
        active && /^bytes = / {bytes=number($0); next}
        active && /^sha256 = / {hash=unquote($0); next}
        active && /^mode = / {mode=unquote($0); next}
        END{emit()}
    ' "$inventory")
    [[ $copied -eq 28 ]] || die "foundation inventory did not materialize its closed 28-file build set"

    vendor_source="$PROJECT_ROOT/$(pin rusqlite vendor_path)"
    vendor_target="$destination/$(pin rusqlite vendor_path)"
    actual="$(tree_sha256 "$vendor_source")"
    [[ "$actual" == "$(pin rusqlite patched_tree_sha256)" ]] || die "foundation external vendor tree identity mismatch"
    /usr/lib/cargo/bin/coreutils/mkdir -p -- "$(/usr/lib/cargo/bin/coreutils/dirname -- "$vendor_target")"
    /usr/bin/gnucp -a -- "$vendor_source" "$vendor_target"
    actual="$(tree_sha256 "$vendor_target")"
    [[ "$actual" == "$(pin rusqlite patched_tree_sha256)" ]] || die "materialized foundation vendor tree identity mismatch"
}

verify_feature_closures_and_builds() {
    local rustup actual artifact root_target chain_target work reference
    rustup="$(pin host_tools rustup_path)"
    verify_root_dependency_contract_resolution
    verify_materialized_sdk_checkout
    work="$(/usr/lib/cargo/bin/coreutils/mktemp -d "$TASK_TMP/cubikan-foundation-build.XXXXXX")"
    trap '/usr/bin/gnurm -rf -- "${work:-}"' RETURN
    materialize_foundation_snapshot "$work"
    root_target="$work/target"
    chain_target="$work/chain/target"
    artifact="$work/$(pin foundation runtime_wasm_path)"
    reference="$PROJECT_ROOT/$(pin foundation runtime_wasm_reference_path)"
    [[ "$artifact" == "$chain_target"/* ]] || die "foundation runtime artifact is outside its task-owned target directory"

    actual="$(CARGO_NET_OFFLINE=true "$rustup" run "$(pin rust channel)" cargo tree --manifest-path "$work/Cargo.toml" -e features --locked --offline | /usr/bin/sed "s#$work#<PROJECT>#g" | /usr/lib/cargo/bin/coreutils/sha256sum | /usr/bin/gawk '{print $1}')"
    [[ "$actual" == "$(pin foundation root_feature_tree_sha256)" ]] || die "foundation root resolved feature closure mismatch"
    actual="$(CARGO_TARGET_DIR="$chain_target" CARGO_NET_OFFLINE=true WASM_BUILD_WORKSPACE_HINT="$work/chain" "$rustup" run "$(pin rust channel)" cargo tree --manifest-path "$work/chain/Cargo.toml" -e features --locked --offline | /usr/bin/sed "s#$work#<PROJECT>#g" | /usr/lib/cargo/bin/coreutils/sha256sum | /usr/bin/gawk '{print $1}')"
    [[ "$actual" == "$(pin foundation chain_feature_tree_sha256)" ]] || die "foundation chain resolved feature closure mismatch"

    CARGO_TARGET_DIR="$root_target" CARGO_NET_OFFLINE=true RUSTFLAGS=-Dwarnings \
        "$rustup" run "$(pin rust channel)" cargo check --manifest-path "$work/Cargo.toml" --workspace --all-targets --locked --offline
    CARGO_TARGET_DIR="$chain_target" CARGO_NET_OFFLINE=true WASM_BUILD_WORKSPACE_HINT="$work/chain" RUSTFLAGS=-Dwarnings \
        "$rustup" run "$(pin rust channel)" cargo check --manifest-path "$work/chain/Cargo.toml" --workspace --locked --offline
    /usr/bin/gnurm -f -- "$artifact"
    CARGO_TARGET_DIR="$chain_target" CARGO_NET_OFFLINE=true WASM_BUILD_WORKSPACE_HINT="$work/chain" RUSTFLAGS=-Dwarnings \
        "$rustup" run "$(pin rust channel)" cargo build --manifest-path "$work/chain/Cargo.toml" --package "$(pin scaffold runtime_package)" --release --locked --offline
    require_size "$artifact" "$(pin foundation runtime_wasm_size)"
    require_hash "$artifact" "$(pin foundation runtime_wasm_sha256)"
    /usr/bin/diff -q -- "$reference" "$artifact" >/dev/null || die "rebuilt foundation Wasm differs from its immutable reference bytes"
    /usr/bin/gnurm -rf -- "$work"
    trap - RETURN
}

verify_local_static_inputs() {
    verify_pin_contract
    verify_repository_tool_bytes
    verify_host_tool_bytes
    verify_lock_and_manifest_bytes
    verify_root_dependency_contract_bytes
    verify_rusqlite_bytes
}

verify_complete_static_inputs() {
    verify_local_static_inputs
    verify_downloads
}

verify_identity_subject() {
    local class="$1" subject="$2" expected_hash="" expected_size="" actual
    case "$class" in
        sdk_archive) expected_hash="$(pin polkadot_sdk archive_sha256)"; expected_size="$(pin polkadot_sdk archive_size)" ;;
        zombienet_archive) expected_hash="$(pin zombienet archive_sha256)"; expected_size="$(pin zombienet archive_size)" ;;
        node_archive) expected_hash="$(pin node archive_sha256)"; expected_size="$(pin node archive_size)" ;;
        rusqlite_archive) expected_hash="$(pin rusqlite archive_sha256)"; expected_size="$(pin rusqlite archive_size)" ;;
        asset_polkadot) expected_hash="$(pin assets.polkadot sha256)"; expected_size="$(pin assets.polkadot size)" ;;
        asset_polkadot_parachain) expected_hash="$(pin assets.polkadot-parachain sha256)"; expected_size="$(pin assets.polkadot-parachain size)" ;;
        asset_polkadot_omni_node) expected_hash="$(pin assets.polkadot-omni-node sha256)"; expected_size="$(pin assets.polkadot-omni-node size)" ;;
        asset_chain_spec_builder) expected_hash="$(pin assets.chain-spec-builder sha256)"; expected_size="$(pin assets.chain-spec-builder size)" ;;
        asset_frame_omni_bencher) expected_hash="$(pin assets.frame-omni-bencher sha256)"; expected_size="$(pin assets.frame-omni-bencher size)" ;;
        argv_grammar) expected_hash="$(pin repository_tools argv_grammar_sha256)" ;;
        argv_normalizer) expected_hash="$(pin repository_tools argv_normalizer_sha256)" ;;
        loopback_wrapper) expected_hash="$(pin repository_tools loopback_wrapper_sha256)" ;;
        root_manifest) expected_hash="$(pin foundation root_manifest_sha256)" ;;
        root_lock) expected_hash="$(pin foundation root_lock_sha256)" ;;
        gitignore) expected_hash="$(pin foundation gitignore_sha256)" ;;
        root_toolchain) expected_hash="$(pin foundation root_toolchain_sha256)" ;;
        root_ci) expected_hash="$(pin foundation root_ci_sha256)" ;;
        chain_manifest) expected_hash="$(pin foundation chain_manifest_sha256)" ;;
        chain_lock) expected_hash="$(pin foundation chain_lock_sha256)" ;;
        chain_toolchain) expected_hash="$(pin foundation chain_toolchain_sha256)" ;;
        chain_pallet_manifest) expected_hash="$(pin foundation chain_pallet_manifest_sha256)" ;;
        chain_pallet_source) expected_hash="$(pin foundation chain_pallet_source_sha256)" ;;
        chain_runtime_manifest) expected_hash="$(pin foundation chain_runtime_manifest_sha256)" ;;
        chain_runtime_build) expected_hash="$(pin foundation chain_runtime_build_sha256)" ;;
        chain_runtime_source) expected_hash="$(pin foundation chain_runtime_source_sha256)" ;;
        root_dependency_manifest) expected_hash="$(pin root_dependency_contract manifest_sha256)" ;;
        root_dependency_lock) expected_hash="$(pin root_dependency_contract lock_sha256)" ;;
        root_dependency_source) expected_hash="$(pin root_dependency_contract source_sha256)" ;;
        rusqlite_patch) expected_hash="$(pin rusqlite patch_sha256)" ;;
        runtime_wasm) expected_hash="$(pin foundation runtime_wasm_sha256)"; expected_size="$(pin foundation runtime_wasm_size)" ;;
        host_*)
            [[ "${class#host_}" =~ ^(bash|unshare|ip|ss|netcat|git|rustup|env|awk|sha256sum|dd|mount|stat|tar|patch|diff|find|sort|iconv|uname|dirname|readlink|sed|grep|head|wc|cp|rm|mkdir|mktemp|curl|chmod|mv)$ ]] || die "unknown host identity-test class: $class"
            expected_hash="$(pin host_tools "${class#host_}_sha256")"
            ;;
        rusqlite_vendor_tree)
            actual="$(tree_sha256 "$subject")"
            [[ "$actual" == "$(pin rusqlite patched_tree_sha256)" ]] || die "identity-test tree mismatch for $class"
            printf 'verify-pins: identity-checked:%s:dependent-boundary-not-entered\n' "$class"
            return
            ;;
        foundation_snapshot_tree)
            actual="$(tree_sha256 "$subject")"
            [[ "$actual" == "$(pin foundation snapshot_tree_sha256)" ]] || die "identity-test tree mismatch for $class"
            printf 'verify-pins: identity-checked:%s:dependent-boundary-not-entered\n' "$class"
            return
            ;;
        rust_toolchain_tree)
            actual="$(tree_sha256 "$subject")"
            [[ "$actual" == "$(pin rust toolchain_tree_sha256)" ]] || die "identity-test tree mismatch for $class"
            printf 'verify-pins: identity-checked:%s:dependent-boundary-not-entered\n' "$class"
            return
            ;;
        wasm_target_tree)
            actual="$(tree_sha256 "$subject")"
            [[ "$actual" == "$(pin rust wasm_target_tree_sha256)" ]] || die "identity-test tree mismatch for $class"
            printf 'verify-pins: identity-checked:%s:dependent-boundary-not-entered\n' "$class"
            return
            ;;
        *) die "unknown identity-test class: $class" ;;
    esac
    if [[ -n "$expected_size" ]]; then
        require_size "$subject" "$expected_size"
    fi
    require_hash "$subject" "$expected_hash"
    printf 'verify-pins: identity-checked:%s:dependent-boundary-not-entered\n' "$class"
}

if [[ "$mode" == testidentity ]]; then
    verify_identity_subject "$identity_class" "$identity_subject"
    exit 0
fi

if [[ "$mode" == teststatic ]]; then
    verify_complete_static_inputs
    printf '%s\n' 'verify-pins: static-preflight-passed'
    exit 0
fi

if [[ "$mode" == selftest ]]; then
    verify_complete_static_inputs
    printf '%s\n' 'verify-pins: parser and complete static preflight self-test passed'
    exit 0
fi

if [[ "$mode" == fetch ]]; then
    verify_local_static_inputs
    fetch_all
    exit 0
fi

verify_complete_static_inputs
verify_repository_tool_behavior
verify_host_tool_behavior
verify_sdk_and_scaffold
verify_node_and_zombienet
verify_asset_capabilities
verify_rusqlite_reconstruction
verify_toolchain
prepare_verified_cargo_sources
verify_feature_closures_and_builds
printf '%s\n' 'verify-pins: all exact pins, offline closures, native build, and Wasm build verified'
