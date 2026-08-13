# CubiKan chain foundation

`chain/` is CubiKan's isolated Polkadot SDK workspace. It pins a minimal
native/Wasm runtime and pallet foundation without introducing SDK, FRAME, or
Cumulus packages into the root workspace. The root and chain workspaces have
separate manifests, lockfiles, toolchains, and target directories.

This is a greenfield development foundation. It has not been deployed to a
public blockchain, does not operate a shared validator set, and makes no
public/shared-security claim. No account, key, funding, ParaId reservation,
coretime purchase, runtime upload, governance action, or release is performed
by these commands.

## Exact fetch and offline verification

Run the network-enabled fetch phase once from the repository root:

```bash
bash chain/tools/verify-pins.sh --fetch-exact
```

This phase fetches only the versions and release artifacts recorded in
`chain/pins.toml`, checks their lengths and SHA-256 digests, installs the exact
Rust components/target when necessary, and fills the Cargo source caches. A
downloaded binary is not made executable until its pinned length and digest
have passed.

Then run the authoritative verification/build phase:

```bash
bash chain/tools/loopback-netns.sh -- bash chain/tools/verify-pins.sh --locked --offline
```

Run that relative spelling from the repository root. The wrapper accepts it
there only, resolves it to the canonical absolute verifier, and binds that
inode before namespace entry.

The locked phase rechecks every identity before dependent execution, enters
the fail-closed `loopback-netns.sh` user/network/mount/IPC namespaces, raises
only loopback, requires an empty non-loopback interface and route inventory,
closes inherited descriptors, and hides conventional host sockets beneath
private `/tmp` and `/run` mounts. A nested private `nodev,nosuid` executable
tmpfs at `/run/cubikan-exec` remains available to the pinned topology tools;
it is bounded to 2 GiB and disappears with the namespace. The broader `/tmp`
and `/run` mounts remain `noexec`. Node and release-asset execution itself uses
a write/grow/shrink/seal-locked Linux memfd, not a workspace or temporary-file
pathname. It then runs Cargo against a task-owned,
verified source cache with locked resolution and offline mode. Failure to
establish those boundaries is a failed gate; `cargo --offline` by itself is
not treated as an egress control.

Repository shell and sealing-helper tools cross each verification boundary as
twice-read, re-hashed in-process byte strings. The verifier retains those
reviewed bytes in readonly Bash variables and every later continuation uses
the pinned `/usr/bin/bash -c` or pinned `/usr/bin/python3.14 -c`; canonical
paths remain repository-location hints only. Path replacement or same-size
in-place mutation after capture therefore cannot select different behavior.
Before any downloaded ELF runs, the reviewed helper copies the already-open
source descriptor into a Linux memfd, checks its exact size/SHA-256, applies
`F_SEAL_WRITE|F_SEAL_GROW|F_SEAL_SHRINK|F_SEAL_SEAL`, proves a post-seal write
returns `EPERM`, re-hashes the sealed object, and executes that descriptor.
The wrapper accepts only the exact canonical verifier child shape, rejects
other `/proc/self/fd` arguments, and gives every non-verifier child
`/dev/null` as standard input.

These finalized literal `bash FILE` commands have a deliberate clean-candidate
shell-startup trust boundary. Bash evaluates `BASH_ENV` before it reads the
named script, so no repository script can retroactively prevent a hostile
startup file from running. The compatibility entry immediately re-enters the
same file with privileged Bash and a clean environment, while hostile inherited
startup-state tests invoke the executable privileged-shebang entry directly.
For an ad hoc invocation outside the finalized clean candidate, use
`/usr/bin/bash -p chain/tools/verify-pins.sh ...`.

The locked verifier classifies namespace state without a caller-provided
marker. An ordinary non-root launcher whose user/network/mount/IPC identities
match its launcher parent (and visible host init) and which has no CubiKan
private mounts receives a distinct status and may enter one fresh namespace. A complete mapped-root,
private-`/tmp`/`/run`, loopback-only state continues in place. Any partial or
suspicious state fails instead of attempting a nested namespace.

This is a local Build gate, not a general-purpose sandbox. A deliberately
placed pathname socket elsewhere in a shared filesystem (including the
workspace or home directory) is outside its claim. The clean environment,
descriptor closure, private conventional runtime paths, and exact source
verification are the reviewed boundary for this sprint.

`chain/.cache/` is ignored, disposable input cache—not authority. The explicit
fetch phase fills its canonical Cargo home; locked mode checks registry
archives against lockfile checksums, compares the SDK git object database and
fresh materialized checkout to the pinned release archive, and rejects ambient
Cargo configuration or target directories. It is safe to remove the cache;
repopulating it requires another explicit `--fetch-exact` phase. The
repository-owned pin artifact, locks, source bytes, and verifier define the
checked identity.

Fast verifier and command-boundary checks are available with:

```bash
/usr/bin/bash -p chain/tools/verify-pins.sh --self-test
/usr/bin/bash chain/tools/verify-pins.test.sh
```

The mutation test first checks every closed identity class against its
authoritative file or tree, then checks a class-specific rejected subject
without crossing the dependent-execution boundary. Large release artifacts
use sparse same-size rejection subjects, and the Rust and vendor trees use
small fake trees, so the test does not duplicate those inputs. Every recorded
pin literal is also independently changed through the non-executing static
preflight. Finally, the suite proves that hostile `PATH`, `BASH_ENV`, and
exported shell functions cannot influence verifier bootstrap, and runs both
the generated-node-argv normalizer and loopback namespace guards. The latter
requires a Linux host on which unprivileged user, network, mount, and IPC
namespaces, mapped-root tmpfs mounts, and loopback netlink setup are available.

## Node roles and ParaId

The pinned `polkadot` artifact is the relay-validator host.
`polkadot-omni-node` is the sole CubiKan collator host. The separately pinned
`polkadot-parachain` binary is retained only as a same-commit
genesis/export-compatibility artifact; it is not an alternative CubiKan
collator. Generated node commands must pass through
`chain/tools/normalize-node-argv.sh`, which rejects unknown or duplicated
fields and rewrites the accepted socket inventory to fixed loopback addresses
and ports.

CubiKan's development ParaId is `1000`. It belongs in the parachain chain
spec's `Extensions`, produced and verified by the pinned chain-spec tooling;
it is not supplied through a collator `--parachain-id` run flag.

Different loopback ports or processes can exercise the same eventual
multi-node model, but ports do not create independent consensus or security
domains. Public/shared operation will require a separately reviewed threat
model, network and genesis identity, validator/key governance, RPC exposure
controls, deployment procedure, and operational evidence. Those are outside
this foundation.

## Foundation versus deployment artifacts

The current compact Wasm proves that the pinned native/Wasm toolchain can
build the isolated foundation. T-1106 owns the final local runtime, chain spec,
metadata, generated weights, genesis identities, and their deployment-artifact
pins. Until that task is complete, this directory must not be described as a
deployable CubiKan network. The four-node failover journey is likewise a later
local, synthetic, loopback-only task.
