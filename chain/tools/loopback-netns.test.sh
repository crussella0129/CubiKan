#!/usr/bin/bash
set -euo pipefail

readonly TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly WRAPPER="$TOOL_DIR/loopback-netns.sh"
readonly INTERNAL_NETNS_TOKEN=__cubikan_loopback_netns_child_v1__

fail() {
    printf 'loopback-netns test: %s\n' "$*" >&2
    exit 1
}

extract_wrapper_function() {
    local function_name="$1"
    /usr/bin/awk -v signature="${function_name}() {" '
        $0 == signature { active = 1 }
        active { print }
        active && /^}$/ { exit }
    ' "$WRAPPER"
}

readonly SHARED_TMP_ROOT="$TOOL_DIR/../.cache"
/usr/bin/mkdir -p -- "$SHARED_TMP_ROOT"
tmpdir="$(/usr/bin/mktemp -d "$SHARED_TMP_ROOT/loopback-netns.test.XXXXXX")"
host_socket_pids=()
host_socket_paths=()

cleanup() {
    local pid socket_path
    for pid in "${host_socket_pids[@]}"; do
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
    done
    for socket_path in "${host_socket_paths[@]}"; do
        /usr/bin/rm -f -- "$socket_path"
    done
    /usr/bin/rm -rf -- "$tmpdir"
}
trap cleanup EXIT

outside_status=0
"$WRAPPER" --assert-current-isolated >"$tmpdir/outside.out" 2>"$tmpdir/outside.err" || outside_status=$?
[[ $outside_status -eq 125 ]] || fail "ordinary outside launcher did not return the distinct outside status"
/usr/bin/grep -Fq 'current-process-isolation=outside-clean-launcher' "$tmpdir/outside.err" ||
    fail "ordinary outside launcher omitted its exact classification marker"

if "$WRAPPER" /usr/bin/true >/dev/null 2>&1; then
    fail "namespace wrapper accepted a missing argv boundary"
fi
if "$WRAPPER" -- >/dev/null 2>&1; then
    fail "namespace wrapper accepted an empty child argv"
fi
if "$WRAPPER" "$INTERNAL_NETNS_TOKEN" 0:0 0:0 0:0 /usr/bin/true >/dev/null 2>&1; then
    fail "internal namespace entry accepted a missing argv boundary"
fi
if (builtin cd /tmp && "$WRAPPER" -- /usr/bin/true) >/dev/null 2>&1; then
    fail "namespace wrapper accepted a working directory hidden by its private mounts"
fi

# Child Bash is an exact pinned pathname, never a PATH lookup or a user-owned
# shadow.  This rejection happens before namespace creation so it remains a
# deterministic fast guard even on hosts without unprivileged namespaces.
if "$WRAPPER" -- /home/charles/.cargo/bin/bash -c true \
    >"$tmpdir/shadow.out" 2>"$tmpdir/shadow.err"; then
    fail "namespace wrapper accepted a user-owned Bash shadow"
fi
/usr/bin/grep -Fq 'noncanonical child Bash path is forbidden' "$tmpdir/shadow.err" ||
    fail "user-owned Bash shadow rejection omitted its exact reason"

shadow_bin="$tmpdir/shadow-bin"
shadow_sentinel="$tmpdir/path-shadow-ran"
/usr/bin/mkdir -p -- "$shadow_bin"
printf '#!/usr/bin/bash\n/usr/bin/touch -- %q\nexit 99\n' "$shadow_sentinel" >"$shadow_bin/bash"
/usr/bin/chmod 0700 -- "$shadow_bin/bash"
PATH="$shadow_bin:/usr/bin:/bin" "$WRAPPER" -- bash -c true \
    >"$tmpdir/path-shadow.out" 2>"$tmpdir/path-shadow.err" || true
[[ ! -e "$shadow_sentinel" ]] || fail "caller PATH shadow replaced the canonical child Bash"

# Merely spelling an inherited descriptor in arbitrary child argv cannot
# authorize retaining it across the namespace boundary.
exec 9</dev/null
if "$WRAPPER" -- /usr/bin/true /proc/self/fd/9 \
    >"$tmpdir/named-fd.out" 2>"$tmpdir/named-fd.err"; then
    fail "namespace wrapper accepted an arbitrary descriptor-bearing argv"
fi
exec 9<&-
/usr/bin/grep -Fq 'descriptor path is forbidden outside the exact verifier continuation' \
    "$tmpdir/named-fd.err" || fail "arbitrary descriptor argv rejection omitted its exact reason"

# Both wrapper child-script snapshot helpers must detach reviewed bytes from a
# same-size in-place overwrite of the original inode.
helper_root="$tmpdir/wrapper-snapshot-helpers"
helper_harness="$helper_root/harness.sh"
helper_output="$helper_root/executed"
/usr/bin/mkdir -p -- "$helper_root"
printf '%s\n' '#!/usr/bin/bash' 'printf "%s\n" path-original >>"${OUTPUT:?}"' >"$helper_root/path-source.sh"
printf '%s\n' '#!/usr/bin/bash' 'printf "%s\n" path-mutated! >>"${OUTPUT:?}"' >"$helper_root/path-mutant.sh"
printf '%s\n' '#!/usr/bin/bash' 'printf "%s\n" fd-original >>"${OUTPUT:?}"' >"$helper_root/fd-source.sh"
printf '%s\n' '#!/usr/bin/bash' 'printf "%s\n" fd-mutated! >>"${OUTPUT:?}"' >"$helper_root/fd-mutant.sh"
[[ "$(/usr/bin/stat -c '%s' -- "$helper_root/path-source.sh")" == "$(/usr/bin/stat -c '%s' -- "$helper_root/path-mutant.sh")" &&
    "$(/usr/bin/stat -c '%s' -- "$helper_root/fd-source.sh")" == "$(/usr/bin/stat -c '%s' -- "$helper_root/fd-mutant.sh")" ]] ||
    fail "wrapper helper fixture scripts are not the same size"
/usr/bin/chmod 0700 -- "$helper_root"/*.sh
{
    printf '%s\n' '#!/usr/bin/bash -p' 'set -euo pipefail' 'shopt -u varredir_close' \
        'readonly STAT=/usr/lib/cargo/bin/coreutils/stat' \
        'die() { printf "fixture: %s\n" "$*" >&2; exit 1; }'
    extract_wrapper_function snapshot_script_path
    extract_wrapper_function snapshot_script_fd
    printf '%s\n' \
        'path_identity="$($STAT -Lc "%d:%i" -- "${PATH_SOURCE:?}")"' \
        'path_size="$($STAT -Lc "%s" -- "$PATH_SOURCE")"' \
        'snapshot_script_path "$PATH_SOURCE" PATH_FD PATH_IDENTITY' \
        '/usr/lib/cargo/bin/coreutils/dd if="${PATH_MUTANT:?}" of="$PATH_SOURCE" conv=notrunc status=none' \
        '[[ "$($STAT -Lc "%d:%i" -- "$PATH_SOURCE")" == "$path_identity" && "$($STAT -Lc "%s" -- "$PATH_SOURCE")" == "$path_size" ]]' \
        'OUTPUT="${OUTPUT_FILE:?}" /usr/bin/bash -s <&"$PATH_FD"' \
        'exec {PREBOUND_SOURCE_FD}<"${FD_SOURCE:?}"' \
        'prebound_identity="$($STAT -Lc "%d:%i:%s" -- "/proc/self/fd/$PREBOUND_SOURCE_FD")"' \
        'fd_inode="$($STAT -Lc "%d:%i" -- "$FD_SOURCE")"' \
        'fd_size="$($STAT -Lc "%s" -- "$FD_SOURCE")"' \
        'snapshot_script_fd "$PREBOUND_SOURCE_FD" "$prebound_identity" FD_SNAPSHOT_FD FD_SNAPSHOT_IDENTITY' \
        '/usr/lib/cargo/bin/coreutils/dd if="${FD_MUTANT:?}" of="$FD_SOURCE" conv=notrunc status=none' \
        '[[ "$($STAT -Lc "%d:%i" -- "$FD_SOURCE")" == "$fd_inode" && "$($STAT -Lc "%s" -- "$FD_SOURCE")" == "$fd_size" ]]' \
        'OUTPUT="$OUTPUT_FILE" /usr/bin/bash -s <&"$FD_SNAPSHOT_FD"' \
        '[[ "$(/usr/bin/readlink -- "/proc/self/fd/$PATH_FD")" == *" (deleted)" ]]' \
        '[[ "$(/usr/bin/readlink -- "/proc/self/fd/$FD_SNAPSHOT_FD")" == *" (deleted)" ]]'
} >"$helper_harness"
/usr/bin/chmod 0700 -- "$helper_harness"
PATH_SOURCE="$helper_root/path-source.sh" PATH_MUTANT="$helper_root/path-mutant.sh" \
    FD_SOURCE="$helper_root/fd-source.sh" FD_MUTANT="$helper_root/fd-mutant.sh" \
    OUTPUT_FILE="$helper_output" /usr/bin/bash -p "$helper_harness"
[[ "$(<"$helper_output")" == $'path-original\nfd-original' ]] ||
    fail "wrapper snapshot helper executed bytes from an overwritten workspace inode"

# The bound wrapper continuation must likewise consume an unlinked private
# snapshot after its canonical pathname is overwritten in place.
swap_root="$tmpdir/wrapper-swap"
swap_wrapper="$swap_root/chain/tools/loopback-netns.sh"
swap_sentinel="$swap_root/alternate-ran"
swap_snapshot="$(/usr/bin/mktemp /tmp/cubikan-loopback-test-v1.XXXXXX)"
swap_mutant="$swap_root/mutant.sh"
/usr/bin/mkdir -p -- "$swap_root/chain/tools"
/usr/bin/cp -- "$WRAPPER" "$swap_wrapper"
/usr/bin/chmod 0700 -- "$swap_wrapper"
/usr/bin/cp -- "$swap_wrapper" "$swap_snapshot"
/usr/bin/chmod 0400 -- "$swap_snapshot"
swap_snapshot_identity="$(/usr/bin/stat -Lc '%d:%i:%s' -- "$swap_snapshot")"
exec {swap_fd}<"$swap_snapshot"
exec {swap_reentry_fd}<"$swap_snapshot"
/usr/bin/rm -f -- "$swap_snapshot"
swap_inode="$(/usr/bin/stat -Lc '%d:%i' -- "$swap_wrapper")"
swap_size="$(/usr/bin/stat -Lc '%s' -- "$swap_wrapper")"
printf '#!/usr/bin/bash\n/usr/bin/touch -- %q\nexit 0\n#' "$swap_sentinel" >"$swap_mutant"
swap_padding=$((swap_size - $(/usr/bin/stat -Lc '%s' -- "$swap_mutant")))
((swap_padding >= 0)) || fail "wrapper mutant exceeds reviewed script size"
/usr/bin/head -c "$swap_padding" /dev/zero | /usr/bin/tr '\0' '#' >>"$swap_mutant"
[[ "$(/usr/bin/stat -Lc '%s' -- "$swap_mutant")" == "$swap_size" ]] || fail "wrapper mutant size drifted"
/usr/lib/cargo/bin/coreutils/dd if="$swap_mutant" of="$swap_wrapper" conv=notrunc status=none
[[ "$(/usr/bin/stat -Lc '%d:%i' -- "$swap_wrapper")" == "$swap_inode" &&
    "$(/usr/bin/stat -Lc '%s' -- "$swap_wrapper")" == "$swap_size" ]] ||
    fail "wrapper overwrite did not preserve inode and size"
swap_status=0
/usr/lib/cargo/bin/coreutils/env -i CUBIKAN_LOOPBACK_SANITIZED=1 HOME=/home/charles \
    CARGO_HOME=/home/charles/.cargo RUSTUP_HOME=/home/charles/.rustup \
    LC_ALL=C LANG=C TZ=UTC TMPDIR=/tmp PATH=/home/charles/.cargo/bin:/usr/bin:/bin \
    /usr/bin/bash --noprofile --norc -p -s -- \
    __cubikan_loopback_bound_path_v1__ "$swap_wrapper" "$swap_reentry_fd" "$swap_snapshot_identity" \
    __cubikan_loopback_clean_entry_v1__ --assert-current-isolated \
    <&"$swap_fd" >"$tmpdir/swap.out" 2>"$tmpdir/swap.err" || swap_status=$?
exec {swap_fd}<&-
exec {swap_reentry_fd}<&-
[[ $swap_status -eq 125 ]] || fail "private wrapper snapshot did not retain ordinary outside classification"
[[ ! -e "$swap_sentinel" ]] || fail "alternate wrapper pathname bytes executed"

# The internal token is intentionally forgeable as data.  It cannot authorize
# execution: outside the new namespace, the independent identity proof fails.
forged_child_sentinel="$tmpdir/forged-child-ran"
if "$WRAPPER" "$INTERNAL_NETNS_TOKEN" 0:0 0:0 0:0 - - -- /usr/bin/bash --noprofile --norc -c \
    'printf ran >"$1"' _ "$forged_child_sentinel" >/dev/null 2>&1; then
    fail "a forged internal token bypassed the namespace proof"
fi
[[ ! -e "$forged_child_sentinel" ]] || fail "child ran after a failed namespace proof"

# If this host disallows unprivileged user/network namespaces, the remaining
# integration assertions cannot run.  The forged-token assertion above still
# proves that proof failure cannot dispatch the requested child.
capability_sentinel="$tmpdir/capability-child-ran"
if ! /usr/bin/unshare --user --map-root-user --net --mount --ipc --propagation private \
    /usr/bin/bash --noprofile --norc -c '
        /usr/bin/mount -t tmpfs -o nodev,nosuid,noexec,mode=1777,size=64M cubikan-private-tmp /tmp
        /usr/bin/mount -t tmpfs -o nodev,nosuid,noexec,mode=0755,size=16M cubikan-private-run /run
        /usr/bin/mkdir -m 0700 /run/cubikan-exec
        /usr/bin/mount -t tmpfs -o nodev,nosuid,mode=0700,size=2G cubikan-private-exec /run/cubikan-exec
        /usr/bin/ip link set dev lo up
    ' >/dev/null 2>"$tmpdir/capability-preflight.err"; then
    if "$WRAPPER" -- /usr/bin/bash --noprofile --norc -c \
        'printf ran >"$1"' _ "$capability_sentinel" >/dev/null 2>"$tmpdir/capability.err"; then
        fail "wrapper dispatched a child although namespace creation was unavailable"
    fi
    [[ ! -e "$capability_sentinel" ]] || fail "child ran after namespace creation or proof failed"
    printf '%s\n' 'loopback-netns integration tests skipped: user/network namespaces unavailable' >&2
    printf '%s\n' 'loopback-netns fail-closed tests passed'
    exit 0
fi
if ! "$WRAPPER" -- /usr/bin/bash --noprofile --norc -c \
    'printf ran >"$1"' _ "$capability_sentinel" >/dev/null 2>"$tmpdir/capability.err"; then
    sed -n '1,80p' "$tmpdir/capability.err" >&2
    fail "wrapper failed despite an available user/network namespace capability"
fi
[[ -e "$capability_sentinel" ]] || fail "capability probe reported success without running its child"

# Exercise the finalized literal child shape through both namespace hops. The
# wrapper must bind the canonical verifier file, carry that FD through
# unshare, and execute its body with the closed bound-entry grammar.
literal_root="$tmpdir/literal-root"
literal_wrapper="$literal_root/chain/tools/loopback-netns.sh"
literal_verifier="$literal_root/chain/tools/verify-pins.sh"
literal_sentinel="$literal_root/verifier-body-ran"
/usr/bin/mkdir -p -- "$literal_root/chain/tools"
/usr/bin/cp -- "$WRAPPER" "$literal_wrapper"
printf '#!/usr/bin/bash\n[[ "$1" == __cubikan_verifier_bound_path_v1__ && "$2" == %q && "$3" == - && "$4" == - && "$5" == --sanitized-entry && "$6" == --locked && "$7" == --offline ]]\nprintf ran >%q\n' \
    "$literal_verifier" "$literal_sentinel" >"$literal_verifier"
/usr/bin/chmod 0700 -- "$literal_wrapper" "$literal_verifier"
if ! (builtin cd -- "$literal_root" && \
    /usr/bin/bash chain/tools/loopback-netns.sh -- \
        bash chain/tools/verify-pins.sh --locked --offline) \
    >"$tmpdir/literal.out" 2>"$tmpdir/literal.err"; then
    /usr/bin/sed -n '1,80p' "$tmpdir/literal.err" >&2
    fail "literal Bash verifier child did not traverse the FD-bound namespace handoff"
fi
[[ "$(<"$literal_sentinel")" == ran ]] || fail "FD-bound verifier body did not execute inside"

if (builtin cd -- "$literal_root/chain" && \
    /usr/bin/bash ../chain/tools/loopback-netns.sh -- \
        bash chain/tools/verify-pins.sh --locked --offline) \
    >"$tmpdir/wrong-root.out" 2>"$tmpdir/wrong-root.err"; then
    fail "relative canonical verifier child was accepted outside the workspace root"
fi
/usr/bin/grep -Fq 'relative canonical verifier child requires the workspace root' \
    "$tmpdir/wrong-root.err" || fail "wrong-root verifier rejection omitted its exact reason"

# A mapped-root namespace without the complete private-mount/network boundary
# is suspicious, not an ordinary outside launcher that may wrap again.
partial_status=0
/usr/bin/unshare --user --map-root-user \
    "$WRAPPER" --assert-current-isolated >"$tmpdir/partial.out" 2>"$tmpdir/partial.err" || partial_status=$?
[[ $partial_status -ne 0 && $partial_status -ne 125 ]] ||
    fail "partial mapped-root state was accepted or classified as a clean outside launcher"

# The finalized clean-candidate spelling uses plain Bash for the outer wrapper.
# Its child can reassert the already-established boundary even though PPID now
# names a legitimate process in the same isolated mount namespace.
if ! /usr/bin/bash "$WRAPPER" -- "$WRAPPER" --assert-current-isolated \
    >"$tmpdir/reassert.out" 2>"$tmpdir/reassert.err"; then
    /usr/bin/sed -n '1,80p' "$tmpdir/reassert.err" >&2
    fail "outer wrapper child could not reassert the complete isolation boundary"
fi
/usr/bin/grep -Fq 'current-process-isolation=verified' "$tmpdir/reassert.err" ||
    fail "inside reassertion omitted its exact verified marker"

# Preserve every byte and boundary in the requested argv, including an empty
# argument and a literal newline.
argv_output="$tmpdir/argv.out"
"$WRAPPER" -- /usr/bin/bash --noprofile --norc -c '
    [[ $# -eq 6 ]]
    [[ "$1" == alpha ]]
    [[ "$2" == "two words" ]]
    [[ -z "$3" ]]
    [[ "$4" == -- ]]
    [[ "$5" == $'"'"'line\nbreak'"'"' ]]
    [[ "$6" == "*?[" ]]
    printf "%s" argv-preserved
' _ alpha 'two words' '' -- $'line\nbreak' '*?[' >"$argv_output" 2>"$tmpdir/argv.err"
[[ "$(<"$argv_output")" == argv-preserved ]] || fail "child argv was not preserved exactly"
/usr/bin/grep -q 'external-connect-probe=denied' "$tmpdir/argv.err" || fail "external-connect denial was not proved"
/usr/bin/grep -q 'loopback-connect-probe=succeeded' "$tmpdir/argv.err" || fail "loopback connectivity was not proved"
/usr/bin/grep -q 'non-loopback-interfaces=0 non-loopback-routes=0' "$tmpdir/argv.err" || fail "network inventory was not proved"
/usr/bin/grep -q 'private-runtime-mounts=/tmp,/run,/run/cubikan-exec mount-namespace=isolated ipc-namespace=isolated' "$tmpdir/argv.err" || fail "private runtime mounts were not proved"

# Conventional host pathname sockets are hidden by the private /tmp and /run
# mounts.  The shared workspace remains visible, so place its sentinels there.
host_tmp_socket="/tmp/cubikan-loopback-host-$$-$RANDOM.sock"
host_run_socket="/run/user/$(/usr/bin/id -u)/cubikan-loopback-host-$$-$RANDOM.sock"
for socket_path in "$host_tmp_socket" "$host_run_socket"; do
    /usr/bin/nc.openbsd -lU "$socket_path" </dev/null >/dev/null 2>&1 &
    host_socket_pids+=("$!")
    host_socket_paths+=("$socket_path")
    for _ in {1..40}; do
        [[ -S "$socket_path" ]] && break
        /usr/bin/sleep 0.05
    done
    [[ -S "$socket_path" ]] || fail "could not create host AF_UNIX sentinel: $socket_path"
done
"$WRAPPER" -- /usr/bin/bash --noprofile --norc -c '
    [[ ! -e "$1" && ! -e "$2" ]]
    [[ "$(/usr/bin/stat -fLc %T -- /tmp)" == tmpfs ]]
    [[ "$(/usr/bin/stat -fLc %T -- /run)" == tmpfs ]]
    [[ "$(/usr/bin/stat -fLc %T -- /run/cubikan-exec)" == tmpfs ]]
    [[ "$(/usr/bin/stat -Lc %a -- /run/cubikan-exec)" == 700 ]]
    /usr/bin/gawk '\''
        $5 == "/run/cubikan-exec" {
            for (i = 6; i <= NF; i++) {
                if ($i == "-" && $(i + 1) == "tmpfs" && $(i + 2) == "cubikan-private-exec" &&
                    $6 ~ /(^|,)rw(,|$)/ && $6 ~ /(^|,)nosuid(,|$)/ &&
                    $6 ~ /(^|,)nodev(,|$)/ && $6 !~ /(^|,)noexec(,|$)/) ok++
            }
        }
        END { exit ok == 1 ? 0 : 1 }
    '\'' /proc/self/mountinfo
    /usr/bin/gnucp -- /usr/bin/true /run/cubikan-exec/test-true
    /usr/bin/chmod 0500 /run/cubikan-exec/test-true
    /run/cubikan-exec/test-true
    printf child-visible >"$3"
' _ "$host_tmp_socket" "$host_run_socket" "$tmpdir/mount-workspace-visible" \
    >"$tmpdir/socket.out" 2>"$tmpdir/socket.err"
[[ -S "$host_tmp_socket" && -S "$host_run_socket" ]] || fail "host AF_UNIX sentinel did not survive outside the child"
[[ ! -e /run/cubikan-exec ]] || fail "private executable tmpfs leaked into the host mount namespace"
[[ "$(<"$tmpdir/mount-workspace-visible")" == child-visible ]] || fail "workspace was not shared into the mount namespace"

# The requested command receives only the eight fixed environment entries.
"$WRAPPER" -- /usr/bin/env >"$tmpdir/environment.out" 2>"$tmpdir/environment.err"
expected_environment=$'HOME=/home/charles\nCARGO_HOME=/home/charles/.cargo\nRUSTUP_HOME=/home/charles/.rustup\nLC_ALL=C\nLANG=C\nTZ=UTC\nTMPDIR=/tmp\nPATH=/home/charles/.cargo/bin:/usr/bin:/bin'
[[ "$(<"$tmpdir/environment.out")" == "$expected_environment" ]] || fail "child environment was not exact"

# Hostile Bash startup state, proxy variables, PATH, and exported functions are
# discarded by the privileged pinned-Bash -> /usr/bin/env -i bootstrap.
bash_env_sentinel="$tmpdir/bash-env-ran"
function ip() { printf ran >"$bash_env_sentinel"; }
function unshare() { printf ran >"$bash_env_sentinel"; }
export -f ip unshare
printf 'printf ran >%q\n' "$bash_env_sentinel" >"$tmpdir/hostile-bash-env"
if ! BASH_ENV="$tmpdir/hostile-bash-env" \
    PATH="$tmpdir" \
    http_proxy=http://127.0.0.1:1 \
    HTTPS_PROXY=http://127.0.0.1:1 \
    SSH_AUTH_SOCK="$tmpdir/agent.sock" \
    CUBIKAN_LOOPBACK_NETNS=1 \
    CUBIKAN_PIN_VERIFY_INSIDE=1 \
    "$WRAPPER" -- /usr/bin/bash --noprofile --norc -c '
        [[ -z "${BASH_ENV:-}${http_proxy:-}${HTTPS_PROXY:-}${SSH_AUTH_SOCK:-}" ]]
        [[ -z "${CUBIKAN_LOOPBACK_NETNS:-}${CUBIKAN_PIN_VERIFY_INSIDE:-}" ]]
        [[ -z "$(declare -F ip)$(declare -F unshare)" ]]
    ' >"$tmpdir/hostile.out" 2>"$tmpdir/hostile.err"; then
    fail "hostile caller environment affected wrapper execution"
fi
unset -f ip unshare
[[ ! -e "$bash_env_sentinel" ]] || fail "hostile BASH_ENV or exported function ran"

# A descriptor inherited by the wrapper must not reach the namespace child.
exec 9</dev/null
"$WRAPPER" -- /usr/bin/bash --noprofile --norc -c \
    '[[ ! -e /proc/self/fd/9 ]]' >"$tmpdir/fd.out" 2>"$tmpdir/fd.err"
exec 9<&-

printf '%s\n' 'loopback-netns tests passed'
