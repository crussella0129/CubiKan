#!/usr/bin/bash -p

# Bind every continuation to the inode Bash already opened for this wrapper.
# SELF remains a repository-location hint only; namespace re-entry always runs
# LOOPBACK_SELF_FD_PATH.
readonly LOOPBACK_BOUND_TOKEN=__cubikan_loopback_bound_path_v1__
if [[ "${1:-}" == "$LOOPBACK_BOUND_TOKEN" ]]; then
    [[ $# -ge 4 && "$2" == /* && -z "${BASH_SOURCE[0]}" &&
        (("$3" == - && "$4" == -) ||
            ("$3" =~ ^[1-9][0-9]*$ && "$4" =~ ^[0-9]+:[0-9]+:[1-9][0-9]*$)) ]] || {
        builtin printf '%s\n' 'loopback-netns: invalid bound-path entry' >&2
        builtin exit 126
    }
    loopback_self_hint=$2
    loopback_reentry_fd=$3
    loopback_reentry_identity=$4
    shift 4
    if [[ "$loopback_reentry_fd" != - ]]; then
        [[ -f "/proc/self/fd/$loopback_reentry_fd" &&
            "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "/proc/self/fd/$loopback_reentry_fd")" == "$loopback_reentry_identity" ]] || {
            builtin printf '%s\n' 'loopback-netns: reentry snapshot identity mismatch' >&2
            builtin exit 126
        }
    fi
else
    if [[ $- != *p* && -n "${BASH_ENV:-}${ENV:-}" ]]; then
        builtin printf '%s\n' 'loopback-netns: non-privileged compatibility entry forbids BASH_ENV and ENV' >&2
        builtin exit 126
    fi
    case "${BASH_SOURCE[0]}" in
        /*) loopback_self_hint="${BASH_SOURCE[0]}" ;;
        *) loopback_self_hint="$(builtin pwd -P)/${BASH_SOURCE[0]}" ;;
    esac
    [[ "$loopback_self_hint" == */chain/tools/loopback-netns.sh &&
        -f "$loopback_self_hint" && ! -L "$loopback_self_hint" ]] || {
        builtin printf '%s\n' 'loopback-netns: initial script path or descriptor is invalid' >&2
        builtin exit 126
    }
    loopback_initial_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$loopback_self_hint")" || builtin exit 126
    exec {loopback_copy_fd}<"$loopback_self_hint" || builtin exit 126
    loopback_snapshot="$(/usr/lib/cargo/bin/coreutils/mktemp /tmp/cubikan-loopback-v1.XXXXXX)" || builtin exit 126
    /usr/lib/cargo/bin/coreutils/dd of="$loopback_snapshot" status=none <&"$loopback_copy_fd" || builtin exit 126
    exec {loopback_copy_fd}<&-
    /usr/lib/cargo/bin/coreutils/chmod 0400 -- "$loopback_snapshot" || builtin exit 126
    loopback_snapshot_hash="$(/usr/lib/cargo/bin/coreutils/sha256sum -- "$loopback_snapshot")" || builtin exit 126
    loopback_snapshot_hash=${loopback_snapshot_hash%% *}
    exec {loopback_compare_fd}<"$loopback_self_hint" || builtin exit 126
    loopback_compare_hash="$(/usr/lib/cargo/bin/coreutils/sha256sum - <&"$loopback_compare_fd")" || builtin exit 126
    loopback_compare_hash=${loopback_compare_hash%% *}
    exec {loopback_compare_fd}<&-
    [[ "$loopback_snapshot_hash" == "$loopback_compare_hash" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$loopback_self_hint")" == "$loopback_initial_identity" ]] || {
        /usr/bin/gnurm -f -- "$loopback_snapshot"
        builtin printf '%s\n' 'loopback-netns: initial script changed during private snapshot creation' >&2
        builtin exit 126
    }
    loopback_snapshot_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "$loopback_snapshot")" || builtin exit 126
    exec {loopback_source_fd}<"$loopback_snapshot" || builtin exit 126
    exec {loopback_reentry_fd}<"$loopback_snapshot" || builtin exit 126
    loopback_reentry_identity="$loopback_snapshot_identity"
    /usr/bin/gnurm -f -- "$loopback_snapshot"
    [[ ! -e "$loopback_snapshot" && -f "/proc/self/fd/$loopback_source_fd" &&
        -f "/proc/self/fd/$loopback_reentry_fd" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "/proc/self/fd/$loopback_source_fd")" == "$loopback_snapshot_identity" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s' -- "/proc/self/fd/$loopback_reentry_fd")" == "$loopback_snapshot_identity" ]] || {
        builtin printf '%s\n' 'loopback-netns: private wrapper snapshot binding failed' >&2
        builtin exit 126
    }
    exec /usr/lib/cargo/bin/coreutils/env -i \
        HOME=/home/charles \
        CARGO_HOME=/home/charles/.cargo \
        RUSTUP_HOME=/home/charles/.rustup \
        LC_ALL=C \
        LANG=C \
        TZ=UTC \
        TMPDIR=/tmp \
        PATH=/home/charles/.cargo/bin:/usr/bin:/bin \
        CUBIKAN_LOOPBACK_SANITIZED=1 \
        /usr/bin/bash --noprofile --norc -p -s -- \
        "$LOOPBACK_BOUND_TOKEN" "$loopback_self_hint" "$loopback_reentry_fd" "$loopback_reentry_identity" \
        __cubikan_loopback_clean_entry_v1__ "$@" <&"$loopback_source_fd"
fi
[[ "$loopback_self_hint" == */chain/tools/loopback-netns.sh &&
    "${CUBIKAN_LOOPBACK_SANITIZED:-}" == 1 &&
    "${1:-}" == __cubikan_loopback_clean_entry_v1__ ]] || {
    builtin printf '%s\n' 'loopback-netns: sanitized bound entry is invalid' >&2
    builtin exit 126
}
shift

set -euo pipefail
shopt -u varredir_close

if [[ -n "$(builtin declare -F)" ]]; then
    builtin printf 'loopback-netns: imported shell functions are forbidden\n' >&2
    builtin exit 1
fi

readonly ENV_BIN=/usr/lib/cargo/bin/coreutils/env
readonly UNSHARE=/usr/bin/unshare
readonly IP=/usr/bin/ip
readonly NC=/usr/bin/nc.openbsd
readonly AWK=/usr/bin/gawk
readonly UNAME=/usr/lib/cargo/bin/coreutils/uname
readonly STAT=/usr/lib/cargo/bin/coreutils/stat
readonly MOUNT=/usr/bin/mount
readonly INTERNAL_NETNS_TOKEN=__cubikan_loopback_netns_child_v1__
readonly ISOLATION_OUTSIDE_STATUS=125

readonly SELF="$loopback_self_hint"
readonly LOOPBACK_REENTRY_FD="$loopback_reentry_fd"
readonly LOOPBACK_REENTRY_IDENTITY="$loopback_reentry_identity"
unset loopback_self_hint
unset loopback_reentry_fd
unset loopback_reentry_identity
unset CUBIKAN_LOOPBACK_SANITIZED
readonly WORKSPACE_ROOT="${SELF%/chain/tools/loopback-netns.sh}"
[[ "$WORKSPACE_ROOT" != "$SELF" && -d "$WORKSPACE_ROOT/chain/tools" ]] || {
    builtin printf '%s\n' 'loopback-netns: workspace path hint is invalid' >&2
    builtin exit 126
}

die() {
    printf 'loopback-netns: %s\n' "$*" >&2
    exit 1
}

snapshot_script_path() {
    local path=$1 fd_variable=$2 identity_variable=$3
    local source_fd compare_fd source_identity snapshot snapshot_hash compare_hash snapshot_identity snapshot_fd
    [[ "$path" == /* && -f "$path" && -x "$path" && ! -L "$path" ]] ||
        die "script snapshot source is not an absolute executable regular file"
    source_identity="$("$STAT" -Lc '%d:%i:%s' -- "$path")"
    exec {source_fd}<"$path" || die "cannot open script snapshot source"
    [[ -f "/proc/self/fd/$source_fd" && "$path" -ef "/proc/self/fd/$source_fd" &&
        "$("$STAT" -Lc '%d:%i:%s' -- "/proc/self/fd/$source_fd")" == "$source_identity" ]] ||
        die "script snapshot source changed while opening"

    snapshot="$(/usr/lib/cargo/bin/coreutils/mktemp /tmp/cubikan-child-script-v1.XXXXXX)" ||
        die "cannot create private child-script snapshot"
    /usr/lib/cargo/bin/coreutils/dd of="$snapshot" status=none <&"$source_fd" ||
        die "cannot copy private child-script snapshot"
    exec {source_fd}<&-
    /usr/lib/cargo/bin/coreutils/chmod 0400 -- "$snapshot" ||
        die "cannot protect private child-script snapshot"
    snapshot_hash="$(/usr/lib/cargo/bin/coreutils/sha256sum -- "$snapshot")" ||
        die "cannot hash private child-script snapshot"
    snapshot_hash=${snapshot_hash%% *}
    exec {compare_fd}<"$path" || die "cannot reopen child-script source for comparison"
    compare_hash="$(/usr/lib/cargo/bin/coreutils/sha256sum - <&"$compare_fd")" ||
        die "cannot compare child-script source"
    compare_hash=${compare_hash%% *}
    exec {compare_fd}<&-
    [[ "$snapshot_hash" == "$compare_hash" && ! -L "$path" &&
        "$("$STAT" -Lc '%d:%i:%s' -- "$path")" == "$source_identity" ]] || {
        /usr/bin/gnurm -f -- "$snapshot"
        die "child-script source changed during private snapshot creation"
    }

    snapshot_identity="$("$STAT" -Lc '%d:%i:%s' -- "$snapshot")"
    exec {snapshot_fd}<"$snapshot" || die "cannot open private child-script snapshot"
    /usr/bin/gnurm -f -- "$snapshot"
    [[ ! -e "$snapshot" && -f "/proc/self/fd/$snapshot_fd" &&
        "$("$STAT" -Lc '%d:%i:%s' -- "/proc/self/fd/$snapshot_fd")" == "$snapshot_identity" ]] ||
        die "private child-script snapshot binding failed"
    printf -v "$fd_variable" '%s' "$snapshot_fd"
    printf -v "$identity_variable" '%s' "$snapshot_identity"
}

snapshot_script_fd() {
    local source_fd=$1 expected_identity=$2 fd_variable=$3 identity_variable=$4
    local snapshot snapshot_identity snapshot_fd
    [[ "$source_fd" =~ ^[1-9][0-9]*$ &&
        "$expected_identity" =~ ^[0-9]+:[0-9]+:[1-9][0-9]*$ &&
        -f "/proc/self/fd/$source_fd" &&
        "$("$STAT" -Lc '%d:%i:%s' -- "/proc/self/fd/$source_fd")" == "$expected_identity" ]] ||
        die "prebound child-script descriptor identity mismatch"
    snapshot="$(/usr/lib/cargo/bin/coreutils/mktemp /tmp/cubikan-child-script-v1.XXXXXX)" ||
        die "cannot create private prebound child-script snapshot"
    /usr/lib/cargo/bin/coreutils/dd of="$snapshot" status=none <&"$source_fd" ||
        die "cannot copy private prebound child-script snapshot"
    exec {source_fd}<&-
    /usr/lib/cargo/bin/coreutils/chmod 0400 -- "$snapshot" ||
        die "cannot protect private prebound child-script snapshot"
    snapshot_identity="$("$STAT" -Lc '%d:%i:%s' -- "$snapshot")"
    exec {snapshot_fd}<"$snapshot" || die "cannot open private prebound child-script snapshot"
    /usr/bin/gnurm -f -- "$snapshot"
    [[ ! -e "$snapshot" && -f "/proc/self/fd/$snapshot_fd" &&
        "$("$STAT" -Lc '%d:%i:%s' -- "/proc/self/fd/$snapshot_fd")" == "$snapshot_identity" ]] ||
        die "private prebound child-script snapshot binding failed"
    printf -v "$fd_variable" '%s' "$snapshot_fd"
    printf -v "$identity_variable" '%s' "$snapshot_identity"
}

require_clean_bootstrap() {
    [[ -z "${BASH_ENV:-}" && -z "${ENV:-}" && -z "${SSH_AUTH_SOCK:-}" ]] || die "forbidden bootstrap environment"
    [[ -z "${http_proxy:-}${https_proxy:-}${HTTP_PROXY:-}${HTTPS_PROXY:-}${ALL_PROXY:-}${NO_PROXY:-}" ]] || die "proxy environment is forbidden"
    [[ "$HOME" == /home/charles && "$CARGO_HOME" == /home/charles/.cargo && "$RUSTUP_HOME" == /home/charles/.rustup ]] || die "bootstrap home environment is not canonical"
    [[ "$LC_ALL" == C && "$LANG" == C && "$TZ" == UTC && "$TMPDIR" == /tmp && "$PATH" == /home/charles/.cargo/bin:/usr/bin:/bin ]] || die "bootstrap environment is not canonical"

    local entry name
    while IFS= read -r entry; do
        name=${entry%%=*}
        case "$name" in
            HOME | CARGO_HOME | RUSTUP_HOME | LC_ALL | LANG | TZ | TMPDIR | PATH | PWD | SHLVL | _) ;;
            *) die "unexpected bootstrap environment entry: $name" ;;
        esac
    done < <("$ENV_BIN")

}

close_fds_except() {
    local fd_path fd keep_fd keep=0
    local -a retained_fds=("$@")
    local -a fd_paths=(/proc/self/fd/*)
    for fd_path in "${fd_paths[@]}"; do
        fd=${fd_path##*/}
        [[ "$fd" =~ ^[0-9]+$ ]] || die "unexpected descriptor name: $fd"
        if ((fd > 2)); then
            keep=0
            for keep_fd in "${retained_fds[@]}"; do
                [[ "$keep_fd" =~ ^[0-9]+$ ]] || die "invalid retained descriptor: $keep_fd"
                if [[ "$fd" == "$keep_fd" ]]; then
                    keep=1
                    break
                fi
            done
            ((keep == 0)) || continue
            # Bash's dynamic-descriptor close form avoids eval.  Closing the
            # script descriptor is safe because this function is parsed before
            # it runs and its caller immediately execs.
            exec {fd}>&-
        fi
    done
}

authorize_verifier_continuation_fd() {
    local -n argv_ref=$1
    local fd verifier_path verifier_argument opened_identity physical_pwd
    PRESERVED_CHILD_FDS=()
    verifier_path="$WORKSPACE_ROOT/chain/tools/verify-pins.sh"
    verifier_argument="${argv_ref[1]:-}"
    physical_pwd="$(builtin pwd -P)"
    if [[ "$verifier_argument" == chain/tools/verify-pins.sh && "$physical_pwd" != "$WORKSPACE_ROOT" ]]; then
        die "relative canonical verifier child requires the workspace root"
    fi
    if [[ "${argv_ref[0]:-}" == /usr/bin/bash &&
        ("$verifier_argument" == "$verifier_path" ||
            ("$verifier_argument" == chain/tools/verify-pins.sh && "$physical_pwd" == "$WORKSPACE_ROOT")) &&
        "${argv_ref[2]:-}" == --locked && "${argv_ref[3]:-}" == --offline &&
        ${#argv_ref[@]} -eq 4 ]]; then
        snapshot_script_path "$verifier_path" fd opened_identity
        argv_ref=(/usr/bin/bash --noprofile --norc -p -s --
            __cubikan_verifier_bound_path_v1__ "$verifier_path" - - \
            --sanitized-entry --locked --offline)
        PRESERVED_CHILD_FDS+=("$fd")
        VERIFIER_CHILD_SCRIPT_FD=$fd
        VERIFIER_CHILD_SCRIPT_IDENTITY=$opened_identity
        return
    fi
    if [[ "${argv_ref[0]:-}" == /usr/bin/bash &&
        "${argv_ref[1]:-}" == --noprofile && "${argv_ref[2]:-}" == --norc &&
        "${argv_ref[3]:-}" == -p && "${argv_ref[4]:-}" == -s &&
        "${argv_ref[5]:-}" == -- &&
        "${argv_ref[6]:-}" == __cubikan_verifier_bound_path_v1__ &&
        "${argv_ref[7]:-}" == "$verifier_path" &&
        "${argv_ref[8]:-}" == __cubikan_prebound_verifier_fd_v1__ &&
        "${argv_ref[9]:-}" =~ ^[1-9][0-9]*$ &&
        "${argv_ref[10]:-}" =~ ^[0-9]+:[0-9]+:[1-9][0-9]*$ &&
        "${argv_ref[11]:-}" == --locked && "${argv_ref[12]:-}" == --offline &&
        ${#argv_ref[@]} -eq 13 ]]; then
        snapshot_script_fd "${argv_ref[9]}" "${argv_ref[10]}" fd opened_identity
        argv_ref=(/usr/bin/bash --noprofile --norc -p -s --
            __cubikan_verifier_bound_path_v1__ "$verifier_path" - - \
            --sanitized-entry --locked --offline)
        PRESERVED_CHILD_FDS+=("$fd")
        VERIFIER_CHILD_SCRIPT_FD=$fd
        VERIFIER_CHILD_SCRIPT_IDENTITY=$opened_identity
        return
    fi
    if [[ "${argv_ref[0]:-}" == /usr/bin/bash &&
        "${argv_ref[1]:-}" == --noprofile && "${argv_ref[2]:-}" == --norc &&
        "${argv_ref[3]:-}" == -p && "${argv_ref[4]:-}" == -s &&
        "${argv_ref[5]:-}" == -- &&
        "${argv_ref[6]:-}" == __cubikan_verifier_bound_path_v1__ &&
        "${argv_ref[7]:-}" == "$verifier_path" && "${argv_ref[8]:-}" == - &&
        "${argv_ref[9]:-}" == - && "${argv_ref[10]:-}" == --sanitized-entry &&
        "${argv_ref[11]:-}" == --locked && "${argv_ref[12]:-}" == --offline &&
        ${#argv_ref[@]} -eq 13 ]]; then
        [[ "${INTERNAL_VERIFIER_FD:-}" =~ ^[1-9][0-9]*$ &&
            "${INTERNAL_VERIFIER_IDENTITY:-}" =~ ^[0-9]+:[0-9]+:[1-9][0-9]*$ ]] ||
            die "rewritten verifier continuation lacks its internal descriptor identity"
        fd=$INTERNAL_VERIFIER_FD
        [[ -f "/proc/self/fd/$fd" &&
            "$("$STAT" -Lc '%d:%i:%s' -- "/proc/self/fd/$fd")" == "$INTERNAL_VERIFIER_IDENTITY" ]] ||
            die "rewritten verifier continuation descriptor identity mismatch"
        PRESERVED_CHILD_FDS+=("$fd")
        VERIFIER_CHILD_SCRIPT_FD=$fd
        VERIFIER_CHILD_SCRIPT_IDENTITY=$INTERNAL_VERIFIER_IDENTITY
        return
    fi
    local argument
    for argument in "${argv_ref[@]}"; do
        [[ "$argument" != /proc/self/fd/* ]] ||
            die "descriptor path is forbidden outside the exact verifier continuation"
    done
}

canonicalize_child_bash() {
    local -n argv_ref=$1
    case "${argv_ref[0]}" in
        bash | /usr/bin/bash)
            [[ -f /usr/bin/bash && -x /usr/bin/bash && ! -L /usr/bin/bash ]] ||
                die "canonical child Bash is unavailable"
            argv_ref[0]=/usr/bin/bash
            ;;
        */bash)
            die "noncanonical child Bash path is forbidden: ${argv_ref[0]}"
            ;;
    esac
}

namespace_identity() {
    local namespace_name=$1 identity
    [[ "$namespace_name" == net || "$namespace_name" == mnt || "$namespace_name" == ipc ]] || die "unsupported namespace identity"
    [[ -e "/proc/self/ns/$namespace_name" ]] || die "$namespace_name namespace identity is unavailable"
    identity="$("$STAT" -Lc '%d:%i' -- "/proc/self/ns/$namespace_name")"
    [[ "$identity" =~ ^[0-9]+:[0-9]+$ ]] || die "$namespace_name namespace identity is invalid"
    printf '%s\n' "$identity"
}

prove_changed_namespace() {
    local namespace_name=$1 launcher_identity=$2 current_identity
    [[ "$launcher_identity" =~ ^[0-9]+:[0-9]+$ ]] || die "launcher $namespace_name namespace identity is invalid"
    current_identity="$(namespace_identity "$namespace_name")"
    [[ "$current_identity" != "$launcher_identity" ]] || die "$namespace_name namespace is not distinct from the launcher"
}

prove_mapped_root_user_namespace() {
    local -a uid_map gid_map
    local uid_inside uid_outside uid_length gid_inside gid_outside gid_length current_gid
    mapfile -t uid_map </proc/self/uid_map
    mapfile -t gid_map </proc/self/gid_map
    [[ ${#uid_map[@]} -eq 1 && ${#gid_map[@]} -eq 1 ]] || die "mapped-root namespace has an unexpected mapping count"
    read -r uid_inside uid_outside uid_length <<<"${uid_map[0]}"
    read -r gid_inside gid_outside gid_length <<<"${gid_map[0]}"
    [[ "$uid_inside" == 0 && "$uid_outside" =~ ^[1-9][0-9]*$ && "$uid_length" == 1 ]] || die "mapped-root uid namespace proof failed"
    [[ "$gid_inside" == 0 && "$gid_outside" =~ ^[1-9][0-9]*$ && "$gid_length" == 1 ]] || die "mapped-root gid namespace proof failed"
}

require_safe_working_directory() {
    local physical_pwd
    physical_pwd="$(builtin pwd -P)"
    case "$physical_pwd/" in
        /tmp/* | /run/*) die "launcher working directory is hidden by the private runtime mounts" ;;
    esac
}

prove_host_init_mount_root_inaccessible() {
    # A process that can traverse the host init mount tree can walk around the
    # private /tmp and /run mounts.
    if "$STAT" -Lc '%d:%i' -- /proc/1/root/tmp >/dev/null 2>&1; then
        die "host init mount root remains traversable"
    fi
}

prove_launcher_parent_mount_root_inaccessible() {
    # This is a launch-time proof.  A later verifier reassertion legitimately
    # has an already-isolated parent, so it must not repeat this PPID check.
    if "$STAT" -Lc '%d:%i' -- "/proc/$PPID/root/tmp" >/dev/null 2>&1; then
        die "launcher-parent mount root remains traversable"
    fi
}

prove_clean_outside_launcher() {
    local -a uid_map gid_map
    local uid_inside uid_outside uid_length gid_inside gid_outside gid_length current_gid
    local tmp_private_mounts run_private_mounts exec_private_mounts namespace_name self_identity parent_identity init_identity

    current_gid="$("$AWK" '/^Gid:/ { print $3; found++ } END { if (found != 1) exit 1 }' /proc/self/status)" ||
        die "outside launcher effective gid is unavailable"
    [[ "$current_gid" =~ ^[1-9][0-9]*$ ]] || die "root or partially mapped launcher cannot create a fresh isolation boundary"
    ((EUID > 0)) || die "root or partially mapped launcher cannot create a fresh isolation boundary"
    mapfile -t uid_map </proc/self/uid_map
    mapfile -t gid_map </proc/self/gid_map
    [[ ${#uid_map[@]} -eq 1 && ${#gid_map[@]} -eq 1 ]] || die "outside launcher has an unexpected identity mapping count"
    read -r uid_inside uid_outside uid_length <<<"${uid_map[0]}"
    read -r gid_inside gid_outside gid_length <<<"${gid_map[0]}"
    [[ "$uid_inside" =~ ^[0-9]+$ && "$uid_outside" =~ ^[0-9]+$ && "$uid_length" =~ ^[1-9][0-9]*$ ]] || die "outside launcher uid mapping is invalid"
    [[ "$gid_inside" =~ ^[0-9]+$ && "$gid_outside" =~ ^[0-9]+$ && "$gid_length" =~ ^[1-9][0-9]*$ ]] || die "outside launcher gid mapping is invalid"
    ((EUID >= uid_inside && EUID < uid_inside + uid_length)) || die "outside launcher uid is not covered by its mapping"
    ((current_gid >= gid_inside && current_gid < gid_inside + gid_length)) || die "outside launcher gid is not covered by its mapping"

    for namespace_name in user net mnt ipc; do
        self_identity="$("$STAT" -Lc '%d:%i' -- "/proc/self/ns/$namespace_name")" ||
            die "outside launcher $namespace_name namespace identity is unavailable"
        parent_identity="$("$STAT" -Lc '%d:%i' -- "/proc/$PPID/ns/$namespace_name")" ||
            die "outside launcher cannot compare its parent $namespace_name namespace"
        [[ "$self_identity" == "$parent_identity" ]] || die "outside launcher is already in a partial $namespace_name namespace"
        if init_identity="$("$STAT" -Lc '%d:%i' -- "/proc/1/ns/$namespace_name" 2>/dev/null)"; then
            [[ "$self_identity" == "$init_identity" ]] || die "outside launcher differs from the visible host-init $namespace_name namespace"
        fi
    done
    "$STAT" -Lc '%d:%i' -- "/proc/$PPID/root/tmp" >/dev/null 2>&1 ||
        die "outside launcher cannot prove access to its parent mount root"

    tmp_private_mounts="$("$AWK" '$5 == "/tmp" { for (i = 6; i <= NF; i++) if ($i == "-" && $(i + 1) == "tmpfs" && $(i + 2) == "cubikan-private-tmp") found++ } END { print found + 0 }' /proc/self/mountinfo)"
    run_private_mounts="$("$AWK" '$5 == "/run" { for (i = 6; i <= NF; i++) if ($i == "-" && $(i + 1) == "tmpfs" && $(i + 2) == "cubikan-private-run") found++ } END { print found + 0 }' /proc/self/mountinfo)"
    exec_private_mounts="$("$AWK" '$5 == "/run/cubikan-exec" { for (i = 6; i <= NF; i++) if ($i == "-" && $(i + 1) == "tmpfs" && $(i + 2) == "cubikan-private-exec") found++ } END { print found + 0 }' /proc/self/mountinfo)"
    [[ "$tmp_private_mounts" == 0 && "$run_private_mounts" == 0 && "$exec_private_mounts" == 0 ]] || die "outside launcher has a partial CubiKan private-mount boundary"
}

mount_private_runtime_paths() {
    [[ -x "$MOUNT" && -x "$STAT" ]] || die "pinned mount proof executables are required"

    local old_tmp_device old_run_device new_tmp_device new_run_device
    old_tmp_device="$("$STAT" -Lc '%d' -- /tmp)"
    old_run_device="$("$STAT" -Lc '%d' -- /run)"

    "$MOUNT" -t tmpfs -o nodev,nosuid,noexec,mode=1777,size=64M \
        cubikan-private-tmp /tmp
    "$MOUNT" -t tmpfs -o nodev,nosuid,noexec,mode=0755,size=16M \
        cubikan-private-run /run
    /usr/lib/cargo/bin/coreutils/mkdir -m 0700 -- /run/cubikan-exec
    "$MOUNT" -t tmpfs -o nodev,nosuid,mode=0700,size=2G \
        cubikan-private-exec /run/cubikan-exec

    new_tmp_device="$("$STAT" -Lc '%d' -- /tmp)"
    new_run_device="$("$STAT" -Lc '%d' -- /run)"
    [[ "$new_tmp_device" != "$old_tmp_device" ]] || die "/tmp did not enter a private mount"
    [[ "$new_run_device" != "$old_run_device" ]] || die "/run did not enter a private mount"
    prove_private_runtime_paths
    [[ -d "$WORKSPACE_ROOT/chain/tools" ]] || die "workspace became inaccessible after private mounts"

    printf '%s\n' 'loopback-netns: private-runtime-mounts=/tmp,/run,/run/cubikan-exec mount-namespace=isolated ipc-namespace=isolated' >&2
}

prove_private_runtime_paths() {
    local tmp_device run_device exec_device tmp_mounts run_mounts exec_mounts
    tmp_device="$("$STAT" -Lc '%d' -- /tmp)"
    run_device="$("$STAT" -Lc '%d' -- /run)"
    exec_device="$("$STAT" -Lc '%d' -- /run/cubikan-exec)"
    [[ "$tmp_device" != "$run_device" && "$exec_device" != "$run_device" ]] || die "private runtime mounts unexpectedly share devices"
    [[ "$("$STAT" -fLc '%T' -- /tmp)" == tmpfs && "$("$STAT" -Lc '%a' -- /tmp)" == 1777 ]] || die "/tmp private tmpfs proof failed"
    [[ "$("$STAT" -fLc '%T' -- /run)" == tmpfs && "$("$STAT" -Lc '%a' -- /run)" == 755 ]] || die "/run private tmpfs proof failed"
    [[ "$("$STAT" -fLc '%T' -- /run/cubikan-exec)" == tmpfs && "$("$STAT" -Lc '%a' -- /run/cubikan-exec)" == 700 ]] || die "private executable tmpfs proof failed"
    tmp_mounts="$("$AWK" '$5 == "/tmp" { for (i = 6; i <= NF; i++) if ($i == "-" && $(i + 1) == "tmpfs" && $(i + 2) == "cubikan-private-tmp") found++ } END { print found + 0 }' /proc/self/mountinfo)"
    run_mounts="$("$AWK" '$5 == "/run" { for (i = 6; i <= NF; i++) if ($i == "-" && $(i + 1) == "tmpfs" && $(i + 2) == "cubikan-private-run") found++ } END { print found + 0 }' /proc/self/mountinfo)"
    exec_mounts="$("$AWK" '$5 == "/run/cubikan-exec" { ok = 0; for (i = 6; i <= NF; i++) if ($i == "-" && $(i + 1) == "tmpfs" && $(i + 2) == "cubikan-private-exec") { if ($6 ~ /(^|,)rw(,|$)/ && $6 ~ /(^|,)nosuid(,|$)/ && $6 ~ /(^|,)nodev(,|$)/ && $6 !~ /(^|,)noexec(,|$)/) ok = 1 } } END { print ok + 0 }' /proc/self/mountinfo)"
    [[ "$tmp_mounts" == 1 && "$run_mounts" == 1 && "$exec_mounts" == 1 ]] || die "private runtime mount identity proof failed"
}

prove_network_boundary() {
    [[ "$($UNAME -s)" == Linux ]] || die "Linux is required"
    [[ $EUID -eq 0 ]] || die "namespace child is not mapped to root"
    [[ -x "$IP" && -x "$NC" && -x "$AWK" && -x "$STAT" ]] || die "pinned network proof executables are required"

    "$IP" link set dev lo up

    local bad_links bad_routes
    bad_links="$("$IP" -o link show | "$AWK" -F': ' '$2 !~ /^lo(@|$)/ { print }')"
    [[ -z "$bad_links" ]] || die "non-loopback interface found: $bad_links"
    bad_routes="$({ "$IP" -4 route show table all; "$IP" -6 route show table all; } | "$AWK" 'NF && $0 !~ /(^|[[:space:]])dev lo([[:space:]]|$)/ { print }')"
    [[ -z "$bad_routes" ]] || die "non-loopback route found: $bad_routes"

    if "$NC" -z -w 1 192.0.2.1 9 >/dev/null 2>&1; then
        die "external-connect probe unexpectedly succeeded"
    fi
    printf '%s\n' 'loopback-netns: external-connect-probe=denied' >&2

    local listener_pid='' loopback_ok=0 attempt
    "$NC" -l 127.0.0.1 39581 </dev/null >/dev/null 2>&1 &
    listener_pid=$!
    trap 'if [[ -n "${listener_pid:-}" ]]; then kill "$listener_pid" 2>/dev/null || true; wait "$listener_pid" 2>/dev/null || true; fi' EXIT
    for attempt in {1..40}; do
        if "$NC" -z -w 1 127.0.0.1 39581 >/dev/null 2>&1; then
            loopback_ok=1
            break
        fi
    done
    [[ $loopback_ok -eq 1 ]] || die "loopback-connect probe failed"
    wait "$listener_pid" 2>/dev/null || true
    listener_pid=''
    trap - EXIT

    printf '%s\n' 'loopback-netns: loopback-connect-probe=succeeded' >&2
    printf '%s\n' 'loopback-netns: non-loopback-interfaces=0 non-loopback-routes=0' >&2
}

prove_network_namespace() {
    local launcher_netns_identity=$1
    prove_changed_namespace net "$launcher_netns_identity"
    prove_network_boundary
}

assert_current_isolated() {
    [[ "$($UNAME -s)" == Linux ]] || die "Linux is required"
    prove_mapped_root_user_namespace
    require_safe_working_directory
    prove_host_init_mount_root_inaccessible
    namespace_identity net >/dev/null
    namespace_identity mnt >/dev/null
    namespace_identity ipc >/dev/null
    prove_private_runtime_paths
    prove_network_boundary
    [[ -d "$WORKSPACE_ROOT/chain/tools" ]] || die "workspace is inaccessible in the isolated environment"
    printf '%s\n' 'loopback-netns: current-process-isolation=verified' >&2
}

classify_current_isolation() {
    if ((EUID != 0)); then
        prove_clean_outside_launcher
        printf '%s\n' 'loopback-netns: current-process-isolation=outside-clean-launcher' >&2
        return "$ISOLATION_OUTSIDE_STATUS"
    fi

    # EUID zero is never classified as an ordinary outside launcher.  It must
    # prove the entire mapped-root/private-mount/loopback boundary or hard-fail.
    assert_current_isolated
}

exec_clean_child() {
    local -a child_argv=("$@")
    canonicalize_child_bash child_argv
    authorize_verifier_continuation_fd child_argv
    close_fds_except "${PRESERVED_CHILD_FDS[@]}"
    if [[ -n "${VERIFIER_CHILD_SCRIPT_FD:-}" ]]; then
        exec "$ENV_BIN" -i CUBIKAN_VERIFIER_SANITIZED=1 HOME=/home/charles \
            CARGO_HOME="$WORKSPACE_ROOT/chain/.cache/cargo-home" \
            RUSTUP_HOME=/home/charles/.rustup LC_ALL=C LANG=C TZ=UTC \
            TMPDIR="$WORKSPACE_ROOT/chain/.cache/tmp" \
            PATH=/home/charles/.cargo/bin:/usr/bin:/bin \
            GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null \
            GIT_NO_REPLACE_OBJECTS=1 GIT_OPTIONAL_LOCKS=0 \
            "${child_argv[@]}" <&"$VERIFIER_CHILD_SCRIPT_FD"
    fi
    exec "$ENV_BIN" -i HOME=/home/charles CARGO_HOME=/home/charles/.cargo \
        RUSTUP_HOME=/home/charles/.rustup LC_ALL=C LANG=C TZ=UTC TMPDIR=/tmp \
        PATH=/home/charles/.cargo/bin:/usr/bin:/bin \
        "${child_argv[@]}" </dev/null
}

run_inside() {
    [[ $# -ge 7 && "$6" == -- ]] || die "internal argv boundary is invalid"
    local launcher_netns_identity=$1 launcher_mountns_identity=$2 launcher_ipcns_identity=$3
    INTERNAL_VERIFIER_FD=$4
    INTERNAL_VERIFIER_IDENTITY=$5
    [[ "$INTERNAL_VERIFIER_FD" == - || "$INTERNAL_VERIFIER_FD" =~ ^[1-9][0-9]*$ ]] ||
        die "internal verifier descriptor token is invalid"
    [[ "$INTERNAL_VERIFIER_IDENTITY" == - || "$INTERNAL_VERIFIER_IDENTITY" =~ ^[0-9]+:[0-9]+:[1-9][0-9]*$ ]] ||
        die "internal verifier identity token is invalid"
    shift 6
    [[ $# -gt 0 ]] || die "missing child argv"

    # The argv token is routing, never authorization: a forged token reaches
    # this same proof and is rejected outside the fresh namespace.
    prove_mapped_root_user_namespace
    require_safe_working_directory
    prove_host_init_mount_root_inaccessible
    prove_launcher_parent_mount_root_inaccessible
    prove_changed_namespace mnt "$launcher_mountns_identity"
    prove_changed_namespace ipc "$launcher_ipcns_identity"
    mount_private_runtime_paths
    prove_network_namespace "$launcher_netns_identity"
    exec_clean_child "$@"
}

exec_namespace_child() {
    local -a child_argv=("$@")
    local launcher_netns_identity launcher_mountns_identity launcher_ipcns_identity
    canonicalize_child_bash child_argv
    launcher_netns_identity="$(namespace_identity net)"
    launcher_mountns_identity="$(namespace_identity mnt)"
    launcher_ipcns_identity="$(namespace_identity ipc)"
    authorize_verifier_continuation_fd child_argv
    close_fds_except "$LOOPBACK_REENTRY_FD" "${PRESERVED_CHILD_FDS[@]}"
    export CUBIKAN_LOOPBACK_SANITIZED=1
    exec "$UNSHARE" --user --map-root-user --net --mount --ipc --propagation private \
        /usr/bin/bash --noprofile --norc -p -s -- \
        "$LOOPBACK_BOUND_TOKEN" "$SELF" - - __cubikan_loopback_clean_entry_v1__ \
        "$INTERNAL_NETNS_TOKEN" \
        "$launcher_netns_identity" "$launcher_mountns_identity" "$launcher_ipcns_identity" \
        "${VERIFIER_CHILD_SCRIPT_FD:--}" "${VERIFIER_CHILD_SCRIPT_IDENTITY:--}" \
        -- "${child_argv[@]}" <&"$LOOPBACK_REENTRY_FD"
}

require_clean_bootstrap

if [[ "${1:-}" == "$INTERNAL_NETNS_TOKEN" ]]; then
    shift
    run_inside "$@"
fi

if [[ "${1:-}" == --assert-current-isolated ]]; then
    [[ $# -eq 1 ]] || die "isolation assertion accepts no child argv"
    isolation_status=0
    classify_current_isolation || isolation_status=$?
    exit "$isolation_status"
fi

[[ "$($UNAME -s)" == Linux ]] || die "Linux is required"
[[ $# -ge 2 && "$1" == -- ]] || die "usage: loopback-netns.sh -- COMMAND [ARG...]"
shift
[[ $# -gt 0 ]] || die "missing child argv"
require_safe_working_directory
[[ -x "$UNSHARE" && -x "$STAT" && -x "$MOUNT" && -f "/proc/self/fd/$LOOPBACK_REENTRY_FD" ]] || die "pinned namespace launcher is unavailable"

exec_namespace_child "$@"
