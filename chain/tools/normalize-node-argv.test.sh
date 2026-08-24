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
    --port 1 --rpc-port 2 --prometheus-port 3
    --rpc-cors all --rpc-methods unsafe --validator
    --workers-path /run/cubikan-exec/pvf-workers
    --execute-workers-max-num 1 --prepare-workers-soft-max-num 1
    --prepare-workers-hard-max-num 1
    --unsafe-rpc-external --prometheus-external
)

relay+=(--bootnodes /ip4/127.0.0.1/tcp/30334/ws/p2p/12D3KooWLockedPeer)

mapfile -d '' -t normalized < <("$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay[@]}")
joined=" ${normalized[*]} "
[[ "$joined" == *' --experimental-rpc-endpoint listen-addr=127.0.0.1:9944,methods=unsafe,cors=all '* ]]
[[ "$joined" != *' --port '* ]]
[[ "$joined" == *' --prometheus-port 9615 '* ]]
[[ "$joined" == *' /ip4/127.0.0.1/tcp/30333/ws '* ]]
[[ "$joined" == *' --bootnodes /ip4/127.0.0.1/tcp/30334/ws/p2p/12D3KooWLockedPeer '* ]]
[[ "$joined" == *' --workers-path /run/cubikan-exec/pvf-workers '* ]]
[[ "$joined" == *' --execute-workers-max-num 1 --prepare-workers-soft-max-num 1 --prepare-workers-hard-max-num 1 '* ]]
[[ "$joined" != *'external'* ]]
[[ "$joined" != *' --ws-port '* ]]
[[ "$joined" != *' --rpc-port '* ]]
[[ "$joined" != *' --rpc-cors '* ]]
[[ "$joined" != *' --rpc-methods '* ]]
[[ "$(grep -o -- '--experimental-rpc-endpoint' <<<"$joined" | wc -l)" -eq 1 ]]
[[ "$(grep -o -- '--no-hardware-benchmarks' <<<"$joined" | wc -l)" -eq 1 ]]

if "$NORMALIZER" --role relay-a --print0 --verify-workers -- \
    "$POLKADOT" "${relay[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a missing private PVF worker mount' >&2
    exit 1
fi
if "$NORMALIZER" --role relay-a --verify-workers -- \
    "$POLKADOT" "${relay[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted worker verification outside print-only mode' >&2
    exit 1
fi

relay_ws=("${relay[@]}")
relay_ws[12]=--ws-port
mapfile -d '' -t normalized < <("$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay_ws[@]}")
joined=" ${normalized[*]} "
[[ "$joined" == *' --experimental-rpc-endpoint listen-addr=127.0.0.1:9944,methods=unsafe,cors=all '* ]]
[[ "$joined" != *' --ws-port '* ]]

collator=(
    --name collator-a --node-key "$node_key" --chain /tmp/para.json
    --base-path /tmp/collator-a --listen-addr /ip4/0.0.0.0/tcp/1/ws
    --port 1 --rpc-port 2 --prometheus-port 3 --collator
    --blocks-pruning archive --state-pruning archive
    --unsafe-rpc-external --prometheus-external
    --rpc-cors all --rpc-methods unsafe
    --
    --chain /tmp/relay.json --execution wasm --port 4 --rpc-port 5
    --prometheus-port 6 --rpc-external
    --workers-path /run/cubikan-exec/pvf-workers
    --execute-workers-max-num 1 --prepare-workers-soft-max-num 1
    --prepare-workers-hard-max-num 1
)
mapfile -d '' -t normalized < <("$NORMALIZER" --role collator-a --print0 -- "$OMNI" "${collator[@]}")
joined=" ${normalized[*]} "
[[ "$joined" == *' --experimental-rpc-endpoint listen-addr=127.0.0.1:9988,methods=unsafe,cors=all '* ]]
[[ "$joined" == *' --prometheus-port 9617 '* ]]
[[ "$joined" == *' --experimental-rpc-endpoint listen-addr=127.0.0.1:9990,methods=unsafe,cors=all '* ]]
[[ "$joined" == *' --prometheus-port 9619 '* ]]
[[ "$joined" == *' --workers-path /run/cubikan-exec/pvf-workers '* ]]
[[ "$joined" != *' --port '* ]]
[[ "$joined" != *'external'* ]]
[[ "$joined" != *' --rpc-port '* ]]
[[ "$joined" != *' --rpc-cors '* ]]
[[ "$joined" != *' --rpc-methods '* ]]
[[ "$(grep -o -- '--experimental-rpc-endpoint' <<<"$joined" | wc -l)" -eq 2 ]]
[[ "$(grep -o -- '--no-hardware-benchmarks' <<<"$joined" | wc -l)" -eq 2 ]]

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
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay[@]}" \
    --experimental-rpc-endpoint listen-addr=127.0.0.1:9944,methods=unsafe,cors=all \
    >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a caller-supplied structured RPC endpoint' >&2
    exit 1
fi
relay_bad_cors=("${relay[@]}")
for index in "${!relay_bad_cors[@]}"; do
    if [[ "${relay_bad_cors[$index]}" == --rpc-cors ]]; then
        relay_bad_cors[$((index + 1))]=https://example.invalid
        break
    fi
done
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay_bad_cors[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a noncanonical generated RPC CORS policy' >&2
    exit 1
fi
relay_without_cors=("${relay[@]}")
for index in "${!relay_without_cors[@]}"; do
    if [[ "${relay_without_cors[$index]}" == --rpc-cors ]]; then
        unset "relay_without_cors[$index]" "relay_without_cors[$((index + 1))]"
        break
    fi
done
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay_without_cors[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a primary without generated RPC CORS policy' >&2
    exit 1
fi
relay_bad_methods=("${relay[@]}")
for index in "${!relay_bad_methods[@]}"; do
    if [[ "${relay_bad_methods[$index]}" == --rpc-methods ]]; then
        relay_bad_methods[$((index + 1))]=safe
        break
    fi
done
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay_bad_methods[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a noncanonical generated RPC method policy' >&2
    exit 1
fi
relay_without_methods=("${relay[@]}")
for index in "${!relay_without_methods[@]}"; do
    if [[ "${relay_without_methods[$index]}" == --rpc-methods ]]; then
        unset "relay_without_methods[$index]" "relay_without_methods[$((index + 1))]"
        break
    fi
done
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay_without_methods[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a primary without generated RPC method policy' >&2
    exit 1
fi
collator_with_relay_cors=("${collator[@]}" --rpc-cors all)
if "$NORMALIZER" --role collator-a --print0 -- "$OMNI" "${collator_with_relay_cors[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a primary-only RPC policy on the embedded relay side' >&2
    exit 1
fi
relay_without_workers=("${relay[@]}")
for index in "${!relay_without_workers[@]}"; do
    if [[ "${relay_without_workers[$index]}" == --workers-path ]]; then
        unset "relay_without_workers[$index]" "relay_without_workers[$((index + 1))]"
        break
    fi
done
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay_without_workers[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a relay without its pinned PVF workers path' >&2
    exit 1
fi
relay_wrong_worker_path=("${relay[@]}")
for index in "${!relay_wrong_worker_path[@]}"; do
    if [[ "${relay_wrong_worker_path[$index]}" == --workers-path ]]; then
        relay_wrong_worker_path[$((index + 1))]=/tmp/workers
        break
    fi
done
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${relay_wrong_worker_path[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a noncanonical PVF workers path' >&2
    exit 1
fi
nonloopback_relay=("${relay[@]}")
nonloopback_relay[${#nonloopback_relay[@]}-1]=/ip4/203.0.113.1/tcp/30333/ws/p2p/12D3KooWBad
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${nonloopback_relay[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a non-loopback bootnode' >&2
    exit 1
fi
plain_bootnode_relay=("${relay[@]:0:${#relay[@]}-2}" --bootnodes /ip4/127.0.0.1/tcp/30334/p2p/12D3KooWLockedPeer)
if "$NORMALIZER" --role relay-a --print0 -- "$POLKADOT" "${plain_bootnode_relay[@]}" >/dev/null 2>&1; then
    printf '%s\n' 'normalizer accepted a bootnode transport that differs from its WebSocket listener' >&2
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
grep -F 'exec "$PYTHON" -I -S -c "$SEALED_EXEC_CONTENT" exec-fd' "$NORMALIZER" >/dev/null
grep -F 'fcntl.F_ADD_SEALS' "$TOOL_DIR/sealed-exec.py" >/dev/null
if grep -F 'exec -a "$command_path" -- "$command_path"' "$NORMALIZER" >/dev/null; then
    printf '%s\n' 'normalizer executes the mutable canonical pathname' >&2
    exit 1
fi

# DrvFS may resolve an ELF /proc/self/fd path through a replaced pathname.
# Prove the production strategy instead streams the already-open inode into a
# write-sealed memfd and denies a post-seal write before execution.
swap_asset="$WORKSPACE_SWAP_ROOT/node"
cp -- /usr/bin/uptime "$swap_asset"
chmod 0700 -- "$swap_asset"
exec {swap_hash_fd}<"$swap_asset"
exec {swap_copy_fd}<"$swap_asset"
mv -- "$swap_asset" "$swap_asset.opened"
cp -- /usr/bin/pwdx "$swap_asset"
chmod 0700 -- "$swap_asset"
[[ "$(/usr/lib/cargo/bin/coreutils/sha256sum - <&"$swap_hash_fd" | awk '{print $1}')" == "$(sha256sum /usr/bin/uptime | awk '{print $1}')" ]]
sealed_helper_content=''
IFS= read -r -d '' sealed_helper_content <"$TOOL_DIR/sealed-exec.py" || [[ -n "$sealed_helper_content" ]]
sealed_helper_hash="$(printf '%s' "$sealed_helper_content" | /usr/bin/sha256sum | /usr/bin/awk '{print $1}')"
[[ "$sealed_helper_hash" == b3cd068ac20123ca2971aca6dff5f6718778090323e14c3d3156669bc1c1f672 ]]
swap_size="$(/usr/bin/stat -Lc '%s' -- "/proc/self/fd/$swap_copy_fd")"
swap_output="$(/usr/bin/python3.14 -I -S -c "$sealed_helper_content" probe-path /usr/bin/gnutrue \
    "$(/usr/bin/stat -Lc '%s' -- /usr/bin/gnutrue)" \
    "$(/usr/bin/sha256sum /usr/bin/gnutrue | /usr/bin/awk '{print $1}')")"
[[ "$swap_output" == 'sealed-exec: seal-set=write,grow,shrink,seal post-seal-write=denied' ]]
/usr/bin/python3.14 -I -S -c "$sealed_helper_content" exec-fd "$swap_copy_fd" "$swap_size" \
    "$(/usr/bin/sha256sum /usr/bin/uptime | /usr/bin/awk '{print $1}')" -- "$swap_asset" >/dev/null
exec {swap_hash_fd}<&-
exec {swap_copy_fd}<&-

# The normalizer continuation consumes reviewed in-process memory. A
# same-size in-place overwrite of the canonical inode must not change the
# bytes Bash subsequently reads.
self_swap_root="$TEST_ROOT/normalizer-self-swap"
self_swap="$self_swap_root/chain/tools/normalize-node-argv.sh"
self_swap_sentinel="$self_swap_root/alternate-ran"
self_swap_mutant="$self_swap_root/mutant.sh"
mkdir -p -- "$self_swap_root/chain/tools"
cp -- "$NORMALIZER" "$self_swap"
cp -- "$TOOL_DIR/node-argv-grammar-v1.txt" "$self_swap_root/chain/tools/node-argv-grammar-v1.txt"
chmod 0700 -- "$self_swap"
self_swap_content=''
IFS= read -r -d '' self_swap_content <"$self_swap" || [[ -n "$self_swap_content" ]]
self_swap_digest="$(printf '%s' "$self_swap_content" | /usr/bin/sha256sum | /usr/bin/awk '{print $1}')"
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
self_swap_output="$(CUBIKAN_NORMALIZER_SANITIZED=1 /usr/bin/bash --noprofile --norc -p -c "$self_swap_content" "$self_swap" \
    __cubikan_normalizer_bound_memory_v1__ "$self_swap" "$self_swap_digest" "$self_swap_content" --verify-grammar)"
[[ "$self_swap_output" == 64be27a9c5ff19b56adbd009e087c0fae059e667a2cf817cba5c6115fd019498 ]]
[[ ! -e "$self_swap_sentinel" ]] || {
    printf '%s\n' 'alternate normalizer pathname bytes executed' >&2
    exit 1
}

printf '%s\n' 'normalize-node-argv tests passed'
