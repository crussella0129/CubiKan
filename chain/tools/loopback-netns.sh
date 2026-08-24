#!/usr/bin/bash -p

# Freeze every continuation in a Bash string before namespace entry. SELF is a
# repository-location hint only; namespace re-entry consumes reviewed memory.
readonly LOOPBACK_BOUND_TOKEN=__cubikan_loopback_bound_memory_v1__
if [[ "${1:-}" == "$LOOPBACK_BOUND_TOKEN" ]]; then
    [[ $# -ge 4 && "$2" == /* && -z "${BASH_SOURCE[0]}" &&
        "$3" =~ ^[0-9a-f]{64}$ && -n "$4" ]] || {
        builtin printf '%s\n' 'loopback-netns: invalid bound-memory entry' >&2
        builtin exit 126
    }
    loopback_self_hint=$2
    loopback_content_sha256=$3
    loopback_content=$4
    shift 4
    loopback_bound_hash="$(builtin printf '%s' "$loopback_content" | /usr/lib/cargo/bin/coreutils/sha256sum)" || builtin exit 126
    loopback_bound_hash=${loopback_bound_hash%% *}
    [[ "$loopback_bound_hash" == "$loopback_content_sha256" ]] || {
        builtin printf '%s\n' 'loopback-netns: bound-memory identity mismatch' >&2
        builtin exit 126
    }
    builtin unset loopback_bound_hash
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
    loopback_initial_identity="$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s:%Y:%Z' -- "$loopback_self_hint")" || builtin exit 126
    loopback_content=''
    IFS= builtin read -r -d '' loopback_content <"$loopback_self_hint" || [[ -n "$loopback_content" ]] || builtin exit 126
    loopback_content_sha256="$(builtin printf '%s' "$loopback_content" | /usr/lib/cargo/bin/coreutils/sha256sum)" || builtin exit 126
    loopback_content_sha256=${loopback_content_sha256%% *}
    loopback_compare_content=''
    IFS= builtin read -r -d '' loopback_compare_content <"$loopback_self_hint" || [[ -n "$loopback_compare_content" ]] || builtin exit 126
    loopback_compare_hash="$(builtin printf '%s' "$loopback_compare_content" | /usr/lib/cargo/bin/coreutils/sha256sum)" || builtin exit 126
    loopback_compare_hash=${loopback_compare_hash%% *}
    [[ "$loopback_content_sha256" == "$loopback_compare_hash" &&
        "$(/usr/lib/cargo/bin/coreutils/stat -Lc '%d:%i:%s:%Y:%Z' -- "$loopback_self_hint")" == "$loopback_initial_identity" ]] || {
        builtin printf '%s\n' 'loopback-netns: initial script changed during memory capture' >&2
        builtin exit 126
    }
    builtin unset loopback_compare_content loopback_compare_hash loopback_initial_identity
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
        /usr/bin/bash --noprofile --norc -p -c "$loopback_content" "$loopback_self_hint" \
        "$LOOPBACK_BOUND_TOKEN" "$loopback_self_hint" "$loopback_content_sha256" "$loopback_content" \
        __cubikan_loopback_clean_entry_v1__ "$@"
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
readonly PYTHON=/usr/bin/python3.14
readonly SETPRIV=/usr/bin/setpriv
readonly INTERNAL_NETNS_TOKEN=__cubikan_loopback_netns_child_v1__
readonly ISOLATION_OUTSIDE_STATUS=125
readonly T1115_OUTPUT_LIMIT=1048576
readonly T1115_OUTPUT_MEDIATOR_PROGRAM='import os
import re
import signal
import subprocess
import sys
import threading

stdout_limit = int(sys.argv[1])
stderr_limit = int(sys.argv[2])
if sys.argv[3] != "--" or len(sys.argv) < 5:
    raise SystemExit(126)
command = sys.argv[4:]
fixed_failure = b"loopback-netns: mediated child output rejected\n"
forbidden_environment = re.compile(
    rb"(?:^|[^A-Z0-9_])(?:AWS_SECRET_ACCESS_KEY|GIT_ASKPASS|GIT_CONFIG_GLOBAL|GIT_CONFIG_SYSTEM|SSH_AUTH_SOCK|HTTP_PROXY|HTTPS_PROXY|ALL_PROXY|NO_PROXY)=",
    re.IGNORECASE,
)
forbidden_secret = re.compile(
    rb"(?:^|[,{])[ \t\r\n]*\x22(?:credential|credentials|mnemonic|passphrase|password|private_key|private_locator|prompt|provider_secret|secret|seed|source_body|token|transcript)\x22[ \t\r\n]*:|(?:^|[^a-z0-9_])(?:credential|credentials|mnemonic|passphrase|password|private_key|private_locator|prompt|provider_secret|secret|seed|source_body|token|transcript)[ \t]*[:=]",
    re.IGNORECASE,
)
public_url = re.compile(rb"(?:git\+ssh|https?|wss?|ssh|git)://", re.IGNORECASE)

def write_all(fd, payload):
    remaining = memoryview(payload)
    while remaining:
        written = os.write(fd, remaining)
        if written <= 0:
            raise OSError("short output write")
        remaining = remaining[written:]

class Capture:
    def __init__(self, pipe, limit):
        self.pipe = pipe
        self.limit = limit
        self.chunks = []
        self.total = 0
        self.overflowed = False
        self.fault = False

    def drain(self):
        try:
            while True:
                chunk = os.read(self.pipe.fileno(), 65_536)
                if not chunk:
                    return
                self.total += len(chunk)
                if self.total > self.limit:
                    self.overflowed = True
                elif not self.overflowed:
                    self.chunks.append(chunk)
        except BaseException:
            self.fault = True

    def accepted(self):
        if self.fault or self.overflowed:
            return None
        data = b"".join(self.chunks)
        try:
            data.decode("ascii", "strict")
        except UnicodeDecodeError:
            return None
        if any(byte not in (0x09, 0x0A) and not 0x20 <= byte <= 0x7E for byte in data):
            return None
        if forbidden_environment.search(data) or forbidden_secret.search(data) or public_url.search(data):
            return None
        return data

def run():
    child = subprocess.Popen(
        command,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        close_fds=True,
        start_new_session=True,
    )
    pidfd = os.pidfd_open(child.pid, 0)
    interrupted = [0]

    def signal_child(signum):
        try:
            signal.pidfd_send_signal(pidfd, signum)
        except ProcessLookupError:
            pass

    def handle_signal(signum, _frame):
        interrupted[0] = signum
        signal_child(signal.SIGTERM)

    signal.signal(signal.SIGHUP, handle_signal)
    signal.signal(signal.SIGINT, handle_signal)
    signal.signal(signal.SIGTERM, handle_signal)
    stdout_capture = Capture(child.stdout, stdout_limit)
    stderr_capture = Capture(child.stderr, stderr_limit)
    stdout_thread = threading.Thread(target=stdout_capture.drain, daemon=True)
    stderr_thread = threading.Thread(target=stderr_capture.drain, daemon=True)
    stdout_thread.start()
    stderr_thread.start()
    timed_out = False
    try:
        child_status = child.wait(timeout=1900)
    except subprocess.TimeoutExpired:
        timed_out = True
        signal_child(signal.SIGTERM)
        try:
            child_status = child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            signal_child(signal.SIGKILL)
            child_status = child.wait(timeout=5)
    stdout_thread.join(5)
    stderr_thread.join(5)
    retained_writer = stdout_thread.is_alive() or stderr_thread.is_alive()
    stdout_payload = stdout_capture.accepted()
    stderr_payload = stderr_capture.accepted()
    child.stdout.close()
    child.stderr.close()
    os.close(pidfd)
    if (
        interrupted[0] != 0
        or timed_out
        or retained_writer
        or stdout_payload is None
        or stderr_payload is None
    ):
        write_all(2, fixed_failure)
        return 128 + interrupted[0] if interrupted[0] != 0 else 126
    write_all(1, stdout_payload)
    write_all(2, stderr_payload)
    return child_status if child_status >= 0 else 128 - child_status

try:
    raise SystemExit(run())
except SystemExit:
    raise
except BaseException:
    try:
        write_all(2, fixed_failure)
    except BaseException:
        pass
    raise SystemExit(126)'

readonly SELF="$loopback_self_hint"
readonly LOOPBACK_CONTENT_SHA256="$loopback_content_sha256"
readonly LOOPBACK_CONTENT="$loopback_content"
unset loopback_self_hint
unset loopback_content_sha256
unset loopback_content
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

capture_script_path() {
    local path=${1:?} content_variable=${2:?} digest_variable=${3:?}
    local source_identity compare_identity content compare_content actual compare_actual
    [[ "$path" == /* && -f "$path" && -x "$path" && ! -L "$path" ]] ||
        die "script source is not an absolute executable regular file"
    source_identity="$("$STAT" -Lc '%d:%i:%s:%Y:%Z' -- "$path")"
    content=''
    IFS= read -r -d '' content <"$path" || [[ -n "$content" ]] || die "cannot capture child-script bytes"
    actual="$(printf '%s' "$content" | /usr/lib/cargo/bin/coreutils/sha256sum)"
    actual=${actual%% *}
    compare_content=''
    IFS= read -r -d '' compare_content <"$path" || [[ -n "$compare_content" ]] || die "cannot recapture child-script bytes"
    compare_actual="$(printf '%s' "$compare_content" | /usr/lib/cargo/bin/coreutils/sha256sum)"
    compare_actual=${compare_actual%% *}
    compare_identity="$("$STAT" -Lc '%d:%i:%s:%Y:%Z' -- "$path")"
    [[ "$compare_actual" == "$actual" && "$compare_identity" == "$source_identity" ]] ||
        die "child-script source changed during memory capture"
    printf -v "$content_variable" '%s' "$content"
    printf -v "$digest_variable" '%s' "$actual"
}

capture_script_memory() {
    local content=${1:?} expected_digest=${2:?} content_variable=${3:?} digest_variable=${4:?} actual
    [[ "$expected_digest" =~ ^[0-9a-f]{64}$ && -n "$content" ]] ||
        die "prebound child-script memory identity is malformed"
    actual="$(printf '%s' "$content" | /usr/lib/cargo/bin/coreutils/sha256sum)"
    actual=${actual%% *}
    [[ "$actual" == "$expected_digest" ]] || die "prebound child-script memory identity mismatch"
    printf -v "$content_variable" '%s' "$content"
    printf -v "$digest_variable" '%s' "$actual"
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
    local verifier_path verifier_argument captured_content captured_digest physical_pwd
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
        capture_script_path "$verifier_path" captured_content captured_digest
        argv_ref=(/usr/bin/bash --noprofile --norc -p -c "$captured_content" "$verifier_path"
            __cubikan_verifier_bound_memory_v1__ "$verifier_path" "$captured_digest" "$captured_content" \
            --sanitized-entry --locked --offline)
        INTERNAL_VERIFIER_DIGEST=$captured_digest
        INTERNAL_VERIFIER_CONTENT=$captured_content
        return
    fi
    if [[ "${argv_ref[0]:-}" == /usr/bin/bash &&
        "${argv_ref[1]:-}" == --noprofile && "${argv_ref[2]:-}" == --norc &&
        "${argv_ref[3]:-}" == -p && "${argv_ref[4]:-}" == -c &&
        "${argv_ref[6]:-}" == "$verifier_path" &&
        "${argv_ref[7]:-}" == __cubikan_verifier_bound_memory_v1__ &&
        "${argv_ref[8]:-}" == "$verifier_path" &&
        "${argv_ref[9]:-}" == __cubikan_prebound_verifier_memory_v1__ &&
        "${argv_ref[10]:-}" =~ ^[0-9a-f]{64}$ &&
        -n "${argv_ref[11]:-}" &&
        "${argv_ref[12]:-}" == --locked && "${argv_ref[13]:-}" == --offline &&
        ${#argv_ref[@]} -eq 14 ]]; then
        capture_script_memory "${argv_ref[11]}" "${argv_ref[10]}" captured_content captured_digest
        [[ "${argv_ref[5]}" == "$captured_content" ]] || die "prebound verifier command content mismatch"
        argv_ref=(/usr/bin/bash --noprofile --norc -p -c "$captured_content" "$verifier_path"
            __cubikan_verifier_bound_memory_v1__ "$verifier_path" "$captured_digest" "$captured_content" \
            --sanitized-entry --locked --offline)
        INTERNAL_VERIFIER_DIGEST=$captured_digest
        INTERNAL_VERIFIER_CONTENT=$captured_content
        return
    fi
    if [[ "${argv_ref[0]:-}" == /usr/bin/bash &&
        "${argv_ref[1]:-}" == --noprofile && "${argv_ref[2]:-}" == --norc &&
        "${argv_ref[3]:-}" == -p && "${argv_ref[4]:-}" == -c &&
        "${argv_ref[6]:-}" == "$verifier_path" &&
        "${argv_ref[7]:-}" == __cubikan_verifier_bound_memory_v1__ &&
        "${argv_ref[8]:-}" == "$verifier_path" &&
        "${argv_ref[9]:-}" =~ ^[0-9a-f]{64}$ && -n "${argv_ref[10]:-}" &&
        "${argv_ref[11]:-}" == --sanitized-entry &&
        "${argv_ref[12]:-}" == --locked && "${argv_ref[13]:-}" == --offline &&
        ${#argv_ref[@]} -eq 14 ]]; then
        capture_script_memory "${argv_ref[10]}" "${argv_ref[9]}" captured_content captured_digest
        [[ "${argv_ref[5]}" == "$captured_content" ]] || die "rewritten verifier command content mismatch"
        INTERNAL_VERIFIER_DIGEST=$captured_digest
        INTERNAL_VERIFIER_CONTENT=$captured_content
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
    [[ "$namespace_name" == net || "$namespace_name" == mnt ||
        "$namespace_name" == ipc || "$namespace_name" == pid ]] ||
        die "unsupported namespace identity"
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

prove_private_pid_namespace() {
    local self_pidns init_pidns pid1_status proc_mounts
    [[ "$("$STAT" -fLc '%T' -- /proc)" == proc ]] ||
        die "private PID namespace lacks procfs"
    self_pidns="$(namespace_identity pid)"
    init_pidns="$("$STAT" -Lc '%d:%i' -- /proc/1/ns/pid)" ||
        die "PID namespace init identity is unavailable"
    [[ "$self_pidns" == "$init_pidns" ]] ||
        die "current process does not share PID namespace init identity"
    pid1_status="$("$AWK" '
        $1 == "Pid:" {
            if (NF != 2) exit 1
            pid = $2
            pid_count++
        }
        $1 == "PPid:" {
            if (NF != 2) exit 1
            ppid = $2
            ppid_count++
        }
        $1 == "NSpid:" {
            if (NF != 2) exit 1
            nspid = $2
            nspid_count++
        }
        END {
            if (pid_count != 1 || ppid_count != 1 || nspid_count != 1) exit 1
            printf "%s:%s:%s\n", pid, ppid, nspid
        }
    ' /proc/1/status)" || die "PID namespace init status is malformed"
    [[ "$pid1_status" == 1:0:1 ]] ||
        die "private PID namespace does not own exact PID 1"
    proc_mounts="$("$AWK" '
        $5 == "/proc" {
            for (i = 6; i <= NF; i++) {
                if ($i == "-" && $(i + 1) == "proc") found++
            }
        }
        END { print found + 0 }
    ' /proc/self/mountinfo)"
    [[ "$proc_mounts" =~ ^[1-9][0-9]*$ ]] ||
        die "private procfs mount identity failed"
    printf '%s\n' \
        'loopback-netns: pid-namespace=isolated procfs=fresh namespace-init=owned' >&2
}

prove_new_pid_namespace_init() {
    local launcher_pidns_identity=$1
    prove_changed_namespace pid "$launcher_pidns_identity"
    [[ $$ -eq 1 && $BASHPID -eq 1 ]] ||
        die "namespace continuation is not PID 1"
    prove_private_pid_namespace
    printf '%s\n' 'loopback-netns: namespace-launcher-pid=1' >&2
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

    for namespace_name in user net mnt ipc pid; do
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

    local old_tmp_device old_run_device old_pts_device new_tmp_device new_run_device new_pts_device
    old_tmp_device="$("$STAT" -Lc '%d' -- /tmp)"
    old_run_device="$("$STAT" -Lc '%d' -- /run)"
    old_pts_device="$("$STAT" -Lc '%d' -- /dev/pts)"

    "$MOUNT" -t tmpfs -o nodev,nosuid,noexec,mode=1777,size=64M \
        cubikan-private-tmp /tmp
    "$MOUNT" -t tmpfs -o nodev,nosuid,noexec,mode=0755,size=16M \
        cubikan-private-run /run
    /usr/lib/cargo/bin/coreutils/mkdir -m 0700 -- /run/cubikan-exec
    "$MOUNT" -t tmpfs -o nodev,nosuid,mode=0700,size=2G \
        cubikan-private-exec /run/cubikan-exec
    "$MOUNT" -t devpts -o newinstance,nodev,nosuid,noexec,mode=0620,ptmxmode=0666 \
        devpts /dev/pts

    new_tmp_device="$("$STAT" -Lc '%d' -- /tmp)"
    new_run_device="$("$STAT" -Lc '%d' -- /run)"
    new_pts_device="$("$STAT" -Lc '%d' -- /dev/pts)"
    [[ "$new_tmp_device" != "$old_tmp_device" ]] || die "/tmp did not enter a private mount"
    [[ "$new_run_device" != "$old_run_device" ]] || die "/run did not enter a private mount"
    [[ "$new_pts_device" != "$old_pts_device" ]] || die "/dev/pts did not enter a private mount"
    prove_private_runtime_paths
    [[ -d "$WORKSPACE_ROOT/chain/tools" ]] || die "workspace became inaccessible after private mounts"

    printf '%s\n' 'loopback-netns: private-runtime-mounts=/tmp,/run,/run/cubikan-exec,/dev/pts mount-namespace=isolated ipc-namespace=isolated' >&2
}

prove_private_runtime_paths() {
    local tmp_device run_device exec_device pts_device tmp_mounts run_mounts exec_mounts pts_mounts
    tmp_device="$("$STAT" -Lc '%d' -- /tmp)"
    run_device="$("$STAT" -Lc '%d' -- /run)"
    exec_device="$("$STAT" -Lc '%d' -- /run/cubikan-exec)"
    pts_device="$("$STAT" -Lc '%d' -- /dev/pts)"
    [[ "$tmp_device" != "$run_device" && "$exec_device" != "$run_device" &&
        "$pts_device" != "$tmp_device" && "$pts_device" != "$run_device" &&
        "$pts_device" != "$exec_device" ]] || die "private runtime mounts unexpectedly share devices"
    [[ "$("$STAT" -fLc '%T' -- /tmp)" == tmpfs && "$("$STAT" -Lc '%a' -- /tmp)" == 1777 ]] || die "/tmp private tmpfs proof failed"
    [[ "$("$STAT" -fLc '%T' -- /run)" == tmpfs && "$("$STAT" -Lc '%a' -- /run)" == 755 ]] || die "/run private tmpfs proof failed"
    [[ "$("$STAT" -fLc '%T' -- /run/cubikan-exec)" == tmpfs && "$("$STAT" -Lc '%a' -- /run/cubikan-exec)" == 700 ]] || die "private executable tmpfs proof failed"
    tmp_mounts="$("$AWK" '$5 == "/tmp" { for (i = 6; i <= NF; i++) if ($i == "-" && $(i + 1) == "tmpfs" && $(i + 2) == "cubikan-private-tmp") found++ } END { print found + 0 }' /proc/self/mountinfo)"
    run_mounts="$("$AWK" '$5 == "/run" { for (i = 6; i <= NF; i++) if ($i == "-" && $(i + 1) == "tmpfs" && $(i + 2) == "cubikan-private-run") found++ } END { print found + 0 }' /proc/self/mountinfo)"
    exec_mounts="$("$AWK" '$5 == "/run/cubikan-exec" { ok = 0; for (i = 6; i <= NF; i++) if ($i == "-" && $(i + 1) == "tmpfs" && $(i + 2) == "cubikan-private-exec") { if ($6 ~ /(^|,)rw(,|$)/ && $6 ~ /(^|,)nosuid(,|$)/ && $6 ~ /(^|,)nodev(,|$)/ && $6 !~ /(^|,)noexec(,|$)/) ok = 1 } } END { print ok + 0 }' /proc/self/mountinfo)"
    pts_mounts="$("$AWK" '$5 == "/dev/pts" { ok = 0; for (i = 6; i <= NF; i++) if ($i == "-" && $(i + 1) == "devpts") { if ($6 ~ /(^|,)rw(,|$)/ && $6 ~ /(^|,)nosuid(,|$)/ && $6 ~ /(^|,)noexec(,|$)/ && $(i + 3) ~ /(^|,)rw(,|$)/ && $(i + 3) ~ /(^|,)ptmxmode=666(,|$)/ && $(i + 3) ~ /(^|,)mode=620(,|$)/) ok = 1 } } END { print ok + 0 }' /proc/self/mountinfo)"
    [[ "$tmp_mounts" == 1 && "$run_mounts" == 1 && "$exec_mounts" == 1 &&
        "$pts_mounts" == 1 ]] || die "private runtime mount identity proof failed"
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
    prove_private_pid_namespace
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
    if is_t1115_mediated_child child_argv; then
        prove_t1115_mediated_descriptors
    fi
    if [[ "${child_argv[*]}" == *'__cubikan_verifier_bound_memory_v1__'* ]]; then
        exec "$ENV_BIN" -i CUBIKAN_VERIFIER_SANITIZED=1 HOME=/home/charles \
            CARGO_HOME="$WORKSPACE_ROOT/chain/.cache/cargo-home" \
            RUSTUP_HOME=/home/charles/.rustup LC_ALL=C LANG=C TZ=UTC \
            TMPDIR="$WORKSPACE_ROOT/chain/.cache/tmp" \
            PATH=/home/charles/.cargo/bin:/usr/bin:/bin \
            GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null \
            GIT_NO_REPLACE_OBJECTS=1 GIT_OPTIONAL_LOCKS=0 \
            "${child_argv[@]}" </dev/null
    fi
    exec "$ENV_BIN" -i HOME=/home/charles CARGO_HOME=/home/charles/.cargo \
        RUSTUP_HOME=/home/charles/.rustup LC_ALL=C LANG=C TZ=UTC TMPDIR=/tmp \
        PATH=/home/charles/.cargo/bin:/usr/bin:/bin \
        "${child_argv[@]}" </dev/null
}

prove_t1115_mediated_descriptors() {
    local fd_path fd stdin_link stdout_link stderr_link
    close_fds_except
    local -a fd_paths=(/proc/$$/fd/*)
    for fd_path in "${fd_paths[@]}"; do
        fd=${fd_path##*/}
        if [[ ! "$fd" =~ ^[0-2]$ ]]; then
            # Globbing /proc/$$/fd can list the transient directory descriptor
            # used by the glob itself. It is harmless only if it vanished
            # before this liveness recheck.
            [[ ! -e "$fd_path" && ! -L "$fd_path" ]] ||
                die "T-1115 launch child retained a host descriptor"
        fi
    done
    stdin_link="$(/usr/bin/readlink -- "/proc/$$/fd/0")" ||
        die "T-1115 launch child stdin identity is unavailable"
    stdout_link="$(/usr/bin/readlink -- "/proc/$$/fd/1")" ||
        die "T-1115 launch child stdout identity is unavailable"
    stderr_link="$(/usr/bin/readlink -- "/proc/$$/fd/2")" ||
        die "T-1115 launch child stderr identity is unavailable"
    [[ "$stdin_link" == /dev/null && "$stdout_link" =~ ^pipe:\[[1-9][0-9]*\]$ &&
        "$stderr_link" =~ ^pipe:\[[1-9][0-9]*\]$ && "$stdout_link" != "$stderr_link" ]] ||
        die "T-1115 launch child standard descriptors are not isolated pipes"
    "$PYTHON" -I -S -c 'import errno
import os
try:
    fd = os.open("/dev/tty", os.O_WRONLY | os.O_CLOEXEC)
except OSError as error:
    raise SystemExit(0 if error.errno == errno.ENXIO else 1)
else:
    os.close(fd)
    raise SystemExit(2)' || die "T-1115 launch child retained a controlling terminal"
    printf '%s\n' 'loopback-netns: t1115-output-boundary=host-mediated descriptors=pipe-only controlling-terminal=absent' >&2
}

run_inside() {
    [[ $# -ge 8 && "$7" == -- ]] || die "internal argv boundary is invalid"
    local launcher_netns_identity=$1 launcher_mountns_identity=$2 launcher_ipcns_identity=$3
    local launcher_pidns_identity=$4
    INTERNAL_VERIFIER_DIGEST=$5
    INTERNAL_VERIFIER_CONTENT=$6
    [[ "$INTERNAL_VERIFIER_DIGEST" == - || "$INTERNAL_VERIFIER_DIGEST" =~ ^[0-9a-f]{64}$ ]] ||
        die "internal verifier digest token is invalid"
    [[ "$INTERNAL_VERIFIER_CONTENT" == - || -n "$INTERNAL_VERIFIER_CONTENT" ]] ||
        die "internal verifier content token is invalid"
    shift 7
    [[ $# -gt 0 ]] || die "missing child argv"

    # The argv token is routing, never authorization: a forged token reaches
    # this same proof and is rejected outside the fresh namespace.
    prove_mapped_root_user_namespace
    require_safe_working_directory
    prove_new_pid_namespace_init "$launcher_pidns_identity"
    prove_launcher_parent_mount_root_inaccessible
    prove_changed_namespace mnt "$launcher_mountns_identity"
    prove_changed_namespace ipc "$launcher_ipcns_identity"
    mount_private_runtime_paths
    prove_network_namespace "$launcher_netns_identity"
    exec_clean_child "$@"
}

is_t1115_mediated_child() {
    local -n argv_ref=$1
    [[ ${#argv_ref[@]} -eq 9 &&
        "${argv_ref[0]}" == /usr/bin/bash &&
        "${argv_ref[1]}" == chain/tools/run-zombienet-e2e.sh &&
        "${argv_ref[2]}" == --config &&
        "${argv_ref[3]}" == chain/config/zombienet.toml &&
        "${argv_ref[4]}" == --relay-validators && "${argv_ref[5]}" == 2 &&
        "${argv_ref[6]}" == --collators && "${argv_ref[7]}" == 2 &&
        "${argv_ref[8]}" == --loopback-only ]]
}

exec_namespace_child() {
    local -a child_argv=("$@")
    local launcher_netns_identity launcher_mountns_identity launcher_ipcns_identity launcher_pidns_identity
    canonicalize_child_bash child_argv
    launcher_netns_identity="$(namespace_identity net)"
    launcher_mountns_identity="$(namespace_identity mnt)"
    launcher_ipcns_identity="$(namespace_identity ipc)"
    launcher_pidns_identity="$(namespace_identity pid)"
    authorize_verifier_continuation_fd child_argv
    close_fds_except "${PRESERVED_CHILD_FDS[@]}"
    export CUBIKAN_LOOPBACK_SANITIZED=1
    if is_t1115_mediated_child child_argv; then
        ((${#PRESERVED_CHILD_FDS[@]} == 0)) ||
            die "T-1115 output mediation forbids inherited continuation descriptors"
        exec "$PYTHON" -I -S -c "$T1115_OUTPUT_MEDIATOR_PROGRAM" \
            "$T1115_OUTPUT_LIMIT" "$T1115_OUTPUT_LIMIT" -- \
            "$SETPRIV" --pdeathsig KILL -- \
            "$UNSHARE" --user --map-root-user --net --mount --ipc \
            --pid --fork --kill-child=KILL --mount-proc --propagation private \
            /usr/bin/bash --noprofile --norc -p -c "$LOOPBACK_CONTENT" "$SELF" \
            "$LOOPBACK_BOUND_TOKEN" "$SELF" "$LOOPBACK_CONTENT_SHA256" "$LOOPBACK_CONTENT" __cubikan_loopback_clean_entry_v1__ \
            "$INTERNAL_NETNS_TOKEN" \
            "$launcher_netns_identity" "$launcher_mountns_identity" "$launcher_ipcns_identity" \
            "$launcher_pidns_identity" \
            "${INTERNAL_VERIFIER_DIGEST:--}" "${INTERNAL_VERIFIER_CONTENT:--}" \
            -- "${child_argv[@]}"
    fi
    exec "$UNSHARE" --user --map-root-user --net --mount --ipc \
        --pid --fork --kill-child=KILL --mount-proc --propagation private \
        /usr/bin/bash --noprofile --norc -p -c "$LOOPBACK_CONTENT" "$SELF" \
        "$LOOPBACK_BOUND_TOKEN" "$SELF" "$LOOPBACK_CONTENT_SHA256" "$LOOPBACK_CONTENT" __cubikan_loopback_clean_entry_v1__ \
        "$INTERNAL_NETNS_TOKEN" \
        "$launcher_netns_identity" "$launcher_mountns_identity" "$launcher_ipcns_identity" \
        "$launcher_pidns_identity" \
        "${INTERNAL_VERIFIER_DIGEST:--}" "${INTERNAL_VERIFIER_CONTENT:--}" \
        -- "${child_argv[@]}" </dev/null
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
[[ -x "$UNSHARE" && -x "$STAT" && -x "$MOUNT" ]] || die "pinned namespace launcher is unavailable"

exec_namespace_child "$@"
