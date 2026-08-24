#!/usr/bin/bash
set -euo pipefail

readonly TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_ROOT="$(cd -- "$TOOL_DIR/../.." && pwd -P)"
readonly LOOPBACK="$TOOL_DIR/loopback-netns.sh"
readonly EXPECTED_CONFIG="$PROJECT_ROOT/chain/config/zombienet.toml"
readonly TIMEOUT=/usr/lib/cargo/bin/coreutils/timeout
readonly MKTEMP=/usr/lib/cargo/bin/coreutils/mktemp
readonly STAT=/usr/lib/cargo/bin/coreutils/stat
readonly RM=/usr/bin/gnurm
readonly CARGO=/home/charles/.rustup/toolchains/1.93.0-x86_64-unknown-linux-gnu/bin/cargo
readonly RUSTC=/home/charles/.rustup/toolchains/1.93.0-x86_64-unknown-linux-gnu/bin/rustc
readonly RUSTDOC=/home/charles/.rustup/toolchains/1.93.0-x86_64-unknown-linux-gnu/bin/rustdoc

die() {
    printf 'run-zombienet-e2e: %s\n' "$*" >&2
    exit 1
}

usage() {
    die 'usage: run-zombienet-e2e.sh --config chain/config/zombienet.toml --relay-validators 2 --collators 2 --loopback-only'
}

[[ $# -eq 7 && "$1" == --config && "$3" == --relay-validators && "$4" == 2 &&
    "$5" == --collators && "$6" == 2 && "$7" == --loopback-only ]] || usage

case "$2" in
    chain/config/zombienet.toml)
        [[ "$(pwd -P)" == "$PROJECT_ROOT" ]] ||
            die 'the relative canonical config requires the repository root'
        config_path=$EXPECTED_CONFIG
        ;;
    "$EXPECTED_CONFIG") config_path=$EXPECTED_CONFIG ;;
    *) die 'the config must be the exact repository Zombienet config' ;;
esac
readonly CONFIG_PATH=$config_path
unset config_path

[[ -f "$CONFIG_PATH" && ! -L "$CONFIG_PATH" ]] ||
    die 'the exact Zombienet config is missing or symbolic'
[[ -x "$LOOPBACK" && ! -L "$LOOPBACK" ]] ||
    die 'the isolation verifier is missing or symbolic'
[[ -x "$TIMEOUT" && -x "$MKTEMP" && -x "$STAT" && -x "$RM" && -x "$CARGO" &&
    -x "$RUSTC" && -x "$RUSTDOC" ]] ||
    die 'one or more canonical gate executables are unavailable'

export RUSTC RUSTDOC

isolation_stderr=''
if ! isolation_stderr="$($LOOPBACK --assert-current-isolated 2>&1 >/dev/null)"; then
    printf '%s\n' "$isolation_stderr" >&2
    die 'the exact candidate must run inside the verified loopback namespace'
fi
[[ "$isolation_stderr" == *'current-process-isolation=verified'* &&
    "$isolation_stderr" == *'external-connect-probe=denied'* &&
    "$isolation_stderr" == *'non-loopback-interfaces=0 non-loopback-routes=0'* ]] ||
    die 'the loopback namespace proof omitted a required marker'

umask 077
supported_root="$($MKTEMP -d /var/tmp/cubikan-t1115.XXXXXX)" ||
    die 'cannot allocate the supported ext4 test root'
readonly SUPPORTED_ROOT=$supported_root
unset supported_root

cleanup() {
    local status=$?
    if [[ -n "${SUPPORTED_ROOT:-}" && "$SUPPORTED_ROOT" == /var/tmp/cubikan-t1115.?????? &&
        -d "$SUPPORTED_ROOT" && ! -L "$SUPPORTED_ROOT" ]]; then
        "$RM" -rf -- "$SUPPORTED_ROOT"
    fi
    exit "$status"
}
terminate() {
    trap - HUP INT TERM
    exit 143
}
trap cleanup EXIT
trap terminate HUP INT TERM

[[ "$($STAT -Lc '%a:%u:%F' -- "$SUPPORTED_ROOT")" == '700:0:directory' ]] ||
    die 'supported root is not an owner-only directory in the mapped namespace'
[[ "$($STAT -fLc '%t' -- "$SUPPORTED_ROOT")" == ef53 ]] ||
    die 'supported root is not on the required ext4 filesystem'
[[ "$($STAT -fLc '%T' -- /run/cubikan-exec)" == tmpfs &&
    "$($STAT -Lc '%a' -- /run/cubikan-exec)" == 700 ]] ||
    die 'the private executable tmpfs is unavailable'

export CUBIKAN_TEST_SUPPORTED_ROOT="$SUPPORTED_ROOT"
export TMPDIR=/run/cubikan-exec

cd -- "$PROJECT_ROOT"
# Keep compilation outside the live-network watchdog.  The inner launcher gets
# 1,740 seconds plus 20 seconds to reap, leaving ten seconds for the Rust
# harness to consume evidence before this 1,770+30 second hard boundary.
"$CARGO" test -p cubikan-local --test chain_e2e --release --locked --offline --no-run
"$TIMEOUT" --signal=TERM --kill-after=30s 1770s \
    "$CARGO" test -p cubikan-local --test chain_e2e --release --locked --offline -- \
    --ignored --test-threads=1
