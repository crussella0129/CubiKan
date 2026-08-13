#!/usr/bin/bash
set -euo pipefail

readonly TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly NORMALIZER="$TOOL_DIR/normalize-node-argv.sh"
readonly PROJECT_ROOT="$(cd -- "$TOOL_DIR/../.." && pwd -P)"
readonly POLKADOT="$PROJECT_ROOT/chain/.cache/downloads/polkadot"
readonly OMNI="$PROJECT_ROOT/chain/.cache/downloads/polkadot-omni-node"
readonly TEST_ROOT="$(mktemp -d)"
readonly WORKSPACE_SWAP_ROOT="$(mktemp -d "$PROJECT_ROOT/chain/.cache/node-fd-swap.XXXXXX")"
trap 'rm -rf -- "$TEST_ROOT" "$WORKSPACE_SWAP_ROOT"' EXIT
[[ -x "$POLKADOT" && -x "$OMNI" ]]
node_key=1111111111111111111111111111111111111111111111111111111111111111

relay=(
    --name relay-a --node-key "$node_key" --chain /tmp/relay.json
    --base-path /tmp/relay-a --listen-addr /ip4/0.0.0.0/tcp/1/ws
    --port 1 --rpc-port 2 --prometheus-port 3 --validator
    --unsafe-rpc-external --prometheus-external
)

mapfile -d '' -t normalized < <("$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay[@]}")
joined=" ${normalized[*]} "
[[ "$joined" == *' --rpc-port 9944 '* ]]
[[ "$joined" == *' --port 30333 '* ]]
[[ "$joined" == *' --prometheus-port 9615 '* ]]
[[ "$joined" == *' /ip4/127.0.0.1/tcp/30333/ws '* ]]
[[ "$joined" != *'external'* ]]
[[ "$joined" != *' --ws-port '* ]]
[[ "$(grep -o -- '--rpc-port' <<<"$joined" | wc -l)" -eq 1 ]]

relay_ws=("${relay[@]}")
relay_ws[12]=--ws-port
mapfile -d '' -t normalized < <("$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay_ws[@]}")
joined=" ${normalized[*]} "
[[ "$joined" == *' --rpc-port 9944 '* ]]
[[ "$joined" != *' --ws-port '* ]]

collator=(
    --name collator-a --node-key "$node_key" --chain /tmp/para.json
    --base-path /tmp/collator-a --listen-addr /ip4/0.0.0.0/tcp/1/ws
    --port 1 --rpc-port 2 --prometheus-port 3 --collator
    --blocks-pruning archive --state-pruning archive
    --unsafe-rpc-external --prometheus-external
    --
    --chain /tmp/relay.json --execution wasm --port 4 --rpc-port 5
    --prometheus-port 6 --rpc-external
)
mapfile -d '' -t normalized < <("$NORMALIZER" --role collator-a --print0 -- "$OMNI" "${collator[@]}")
joined=" ${normalized[*]} "
[[ "$joined" == *' --rpc-port 9988 '* ]]
[[ "$joined" == *' --port 30335 '* ]]
[[ "$joined" == *' --prometheus-port 9617 '* ]]
[[ "$joined" == *' --rpc-port 9990 '* ]]
[[ "$joined" == *' --port 30337 '* ]]
[[ "$joined" == *' --prometheus-port 9619 '* ]]
[[ "$joined" != *'external'* ]]

if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay[@]}" --mystery >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted an unknown flag' >&2
    exit 1
fi
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay[@]}" --rpc-port 9 >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a duplicate flag' >&2
    exit 1
fi
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay[@]}" --ws-port 9 >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted both RPC flag spellings' >&2
    exit 1
fi
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay[@]}" --bootnodes /ip4/203.0.113.1/tcp/30333/p2p/12D3KooWBad >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a non-loopback bootnode' >&2
    exit 1
fi

ln -s -- "$POLKADOT" "$TEST_ROOT/polkadot"
if "$NORMALIZER" --role relay-a --print0 -- "$TEST_ROOT/polkadot" "${relay[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a substituted node path' >&2
    exit 1
fi

# The execution boundary must stay tied to the verified open inode.  A FIFO
# can replay the pinned bytes during hashing and then leave a different path
# target for execution, so it must be rejected before any read.  The source
# assertion prevents a later pathname-exec regression from silently weakening
# the open-FD boundary exercised by the production launcher.
replica_root="$TEST_ROOT/replica"
mkdir -p -- "$replica_root/chain/tools" "$replica_root/chain/.cache/downloads"
cp -- "$NORMALIZER" "$replica_root/chain/tools/normalize-node-argv.sh"
cp -- "$TOOL_DIR/node-argv-grammar-v1.txt" "$replica_root/chain/tools/node-argv-grammar-v1.txt"
mkfifo -- "$replica_root/chain/.cache/downloads/polkadot"
chmod 0700 -- "$replica_root/chain/.cache/downloads/polkadot"
if "$replica_root/chain/tools/normalize-node-argv.sh" --role relay-a --print0 -- \
    "$replica_root/chain/.cache/downloads/polkadot" "${relay[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a non-regular canonical asset' >&2
    exit 1
fi
rm -f -- "$replica_root/chain/.cache/downloads/polkadot"
printf '#!/usr/bin/bash\nexit 0\n' >"$replica_root/chain/.cache/downloads/polkadot"
chmod 0700 -- "$replica_root/chain/.cache/downloads/polkadot"
if "$replica_root/chain/tools/normalize-node-argv.sh" --role relay-a --print0 -- \
    "$replica_root/chain/.cache/downloads/polkadot" "${relay[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a script node asset instead of ELF' >&2
    exit 1
fi
grep -F '"$DD" of="$execution_path" status=none <&"$command_copy_fd"' "$NORMALIZER" >/dev/null
grep -F 'exec -a "$command_path" -- "$execution_path"' "$NORMALIZER" >/dev/null
if grep -F 'exec -a "$command_path" -- "$command_path"' "$NORMALIZER" >/dev/null; then
    printf '%s\n' 'normalizer executes the mutable canonical pathname' >&2
    exit 1
fi

# DrvFS may resolve an ELF /proc/self/fd path through a replaced pathname.
# Prove the production strategy instead streams the already-open inode into a
# private execution file, so a same-size ELF replacement is never selected.
swap_asset="$WORKSPACE_SWAP_ROOT/node"
cp -- /usr/bin/uptime "$swap_asset"
chmod 0700 -- "$swap_asset"
exec {swap_hash_fd}<"$swap_asset"
exec {swap_copy_fd}<"$swap_asset"
mv -- "$swap_asset" "$swap_asset.opened"
cp -- /usr/bin/pwdx "$swap_asset"
chmod 0700 -- "$swap_asset"
[[ "$(/usr/lib/cargo/bin/coreutils/sha256sum - <&"$swap_hash_fd" | awk '{print $1}')" == "$(sha256sum /usr/bin/uptime | awk '{print $1}')" ]]
materialized="$TEST_ROOT/materialized-uptime"
/usr/lib/cargo/bin/coreutils/dd of="$materialized" status=none <&"$swap_copy_fd"
chmod 0500 -- "$materialized"
"$materialized" >/dev/null
exec {swap_hash_fd}<&-
exec {swap_copy_fd}<&-

# The normalizer continuation consumes an unlinked private snapshot.  A
# same-size in-place overwrite of the canonical inode must not change the
# bytes Bash subsequently reads.
self_swap_root="$TEST_ROOT/normalizer-self-swap"
self_swap="$self_swap_root/chain/tools/normalize-node-argv.sh"
self_swap_sentinel="$self_swap_root/alternate-ran"
self_swap_snapshot="$(mktemp /tmp/cubikan-normalizer-test-v1.XXXXXX)"
self_swap_mutant="$self_swap_root/mutant.sh"
mkdir -p -- "$self_swap_root/chain/tools"
cp -- "$NORMALIZER" "$self_swap"
cp -- "$TOOL_DIR/node-argv-grammar-v1.txt" "$self_swap_root/chain/tools/node-argv-grammar-v1.txt"
chmod 0700 -- "$self_swap"
cp -- "$self_swap" "$self_swap_snapshot"
chmod 0400 -- "$self_swap_snapshot"
exec {self_swap_fd}<"$self_swap_snapshot"
rm -f -- "$self_swap_snapshot"
self_swap_inode="$(/usr/bin/stat -Lc '%d:%i' -- "$self_swap")"
self_swap_size="$(/usr/bin/stat -Lc '%s' -- "$self_swap")"
printf '#!/usr/bin/bash\n/usr/bin/touch -- %q\nexit 0\n#' "$self_swap_sentinel" >"$self_swap_mutant"
self_swap_padding=$((self_swap_size - $(/usr/bin/stat -Lc '%s' -- "$self_swap_mutant")))
((self_swap_padding >= 0)) || {
    printf '%s\n' 'normalizer mutant exceeds reviewed script size' >&2
    exit 1
}
/usr/bin/head -c "$self_swap_padding" /dev/zero | /usr/bin/tr '\0' '#' >>"$self_swap_mutant"
[[ "$(/usr/bin/stat -Lc '%s' -- "$self_swap_mutant")" == "$self_swap_size" ]]
/usr/lib/cargo/bin/coreutils/dd if="$self_swap_mutant" of="$self_swap" conv=notrunc status=none
[[ "$(/usr/bin/stat -Lc '%d:%i' -- "$self_swap")" == "$self_swap_inode" &&
    "$(/usr/bin/stat -Lc '%s' -- "$self_swap")" == "$self_swap_size" ]]
self_swap_output="$(CUBIKAN_NORMALIZER_SANITIZED=1 /usr/bin/bash --noprofile --norc -p -s -- \
    __cubikan_normalizer_bound_path_v1__ "$self_swap" --verify-grammar <&"$self_swap_fd")"
exec {self_swap_fd}<&-
[[ "$self_swap_output" == 112655c95fcf0b1fe535d0e6b209883374bb650b40ecd9f3383d4378dd1b88b4 ]]
[[ ! -e "$self_swap_sentinel" ]] || {
    printf '%s\n' 'alternate normalizer pathname bytes executed' >&2
    exit 1
}

printf '%s\n' 'normalize-node-argv tests passed'
