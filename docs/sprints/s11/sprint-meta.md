# Sprint 11 Meta

- **Sprint number:** 11
- **Book schema version:** 2
- **Start timestamp:** 2026-08-11T21:36:54Z
- **End timestamp:** (filled at Loop Phase)
- **Model:** GPT-5
- **Exit status:** in-progress
- **Token count:** (filled at Loop Phase if observable)
- **Summary:** Build a pinned local Polkadot SDK blockchain-canonical CubiKan with mandatory origins, bounded lifecycle/relationship/provenance state, a finalized SQLite v3 projection fully attested against one pinned node-trusted archive RPC stream, adapter-owned protocol v2, and a two-validator/two-collator Zombienet proof.
- **Intents:** [INT-0008](../../intents/INT-0008-traceable-intent-instantiation.md) and [INT-0014](../../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md), carrying forward realized INT-0009 semantics and superseding INT-0010, INT-0012, and INT-0013 where their live authority contracts are replaced.
- **Completion evidence:** (filled at Loop Phase)

## Build Checkpoint Blockages

- **2026-08-24 — T-1115 checkpoint remains incomplete:** The checkpoint branch
  preserves the current four-node Zombienet candidate and its supporting
  chain-client, attestation, submission, pin, fixture, namespace, sealed-node,
  PVF-worker, audit, and mutation-guard work for review. Fast verification is
  green: `cargo fmt --all -- --check`, the focused release chain-E2E contract
  tests (two passed, five intentionally ignored), warnings-denied focused
  Clippy, JavaScript and Bash syntax checks, `git diff --check`, and the full
  `chain/tools/zombienet-t1115.test.sh` static/mutation suite. Exact candidate
  runs have progressed through the relay census and exposed successive raw-spec
  audit-bound and PVF teardown-sampling defects; those discoveries are fixed or
  narrowed in the checkpoint, but no uninterrupted exact run has yet completed.
  Independent review found two remaining teardown-verifier races: post-stop
  inventories must use typed, teardown-only retries for demonstrably exiting,
  reparenting, or disappearing known objects, and success must require at least
  two consecutive error-free empty process/path/socket censuses rather than the
  first empty non-atomic scan. Behavioral transient-versus-persistent guards and
  stronger post-node/listener-zero ordering guards must accompany that repair,
  followed by the exact locked/offline namespace journey. This checkpoint does
  not mark T-1115 complete, move it from `docs/work/tasks.md`, or unblock T-1116.

- **2026-08-13 — T-1101 remains queued:** Foundation source, dependency,
  toolchain, rusqlite, static pin, mutation, Rust, and Wasm checks are green,
  but final E1/E3 evidence is incomplete. The current shell-tool and release-
  asset launch paths still have a same-UID named-snapshot/hash-to-exec race;
  they require a pinned sealed-memory or equivalently immutable execution
  object plus a deterministic post-hash write rejection test. The canonical
  loopback-only namespace/offline rerun also could not start because the local
  elevated-execution service exhausted its weekly allowance. This checkpoint
  does not remove T-1101 from `docs/work/tasks.md`, add completion evidence, or
  authorize T-1102.

- **2026-08-13 — T-1101 blockage resolved:** Reviewed in-process shell bytes
  and a pinned Linux sealed-memfd executor now close the same-UID helper and
  release-asset hash-to-exec races. The gate proves post-seal writes fail with
  `EPERM`, covers DrvFS pathname replacement, and rejects identity drift before
  dependent execution. The exact canonical loopback-only locked/offline gate
  subsequently passed its root checks, warnings-denied chain check, release
  build, and Wasm verification. T-1101 may move to completed and T-1102 is
  unblocked.

- **2026-08-13 — T-1103 benchmark dependency omission resolved:** T-1103 owns
  executable pallet benchmarks and T-1106 must generate weights from them, but
  neither task's locked Touches included the manifests and lockfile needed for
  FRAME v2's direct `frame-benchmarking` dependency. The minimal repair adds
  the optional, default-feature-disabled dependency from the already pinned
  stable2606 SDK revision, updates only the chain manifests/lock and their
  pin-verifier identity, and introduces no root dependency, SDK-source, or
  runtime-semantic expansion. All four lifecycle dispatchables now have
  executable maximum-bound benchmarks; T-1106 remains responsible for running
  the benchmark node and replacing provisional weights with generated output.

- **2026-08-13 — T-1106 runtime and benchmark scope omissions resolved:** The
  locked task requires an operational FRAME/Cumulus runtime, a regenerated
  chain lock, and weights measured at every declared maximum, but its Touches
  omitted `chain/Cargo.toml`, `chain/Cargo.lock`, and the shared maximum-fixture
  source `chain/pallets/cubikan/src/benchmarking.rs`. The minimal repair adds
  only direct dependencies from the already pinned stable2606 SDK revision,
  records their exact lock graph, and seeds the existing benchmark fixtures at
  the maximum global-sequence boundary before generating the retained measured
  evidence and runtime-owned weights. It does not widen the runtime call or
  origin surface.

- **2026-08-14 — T-1107 legacy-migration scope omission resolved:** Required
  origin makes the historical schema-v1-to-v2 migration incapable of producing
  a valid current aggregate without synthetic attribution. The locked Touches
  omitted `crates/cubikan-backend/src/migration.rs`, even though leaving its
  successful migration path compiled would preserve transitional write
  authority. The minimal repair changes only that legacy entry point to return
  the existing typed unsupported-schema error before filesystem access; schema
  v3 remains fresh-only and is introduced by T-1108.

- **2026-08-14 — T-1108 SQLite inspection scope omissions resolved:** Exact
  fail-before-access validation of the linked SQLite compile-option vector and
  registered built-in VFS identities cannot be implemented through rusqlite
  0.40.2's public safe API. The minimal repair adds safe, read-only wrappers for
  the corresponding SQLite C inspection functions inside the already pinned
  vendored rusqlite source, without exposing pointer-valued implementation
  state or adding SQL authority. The repository-owned patch, pin identities,
  and reconstruction verifier are extended over those exact bytes. T-1108's
  locked root-manifest/lock evolution also requires the verifier's pre-Subxt
  phase to accept only the closed six-dependency projection graph before
  T-1110 transitions to the already sealed final Subxt graph. The new backend
  error variants additionally require exhaustive taxonomy and retired-envelope
  expectation updates in the existing backend integration tests
  `relationship_model.rs` and `legacy_generation.rs`; neither repair changes
  projection data authority or the public query surface.

- **2026-08-14 — T-1108 approved-filesystem execution remains queued:** The
  shared classifier corpus, fail-closed DrvFS/tmpfs branches, schema/envelope
  tests, authorizer tests, warnings-denied workspace tests, and Clippy all run
  in the current sandbox. The production create/reopen/page-limit/Busy branch
  deliberately requires `CUBIKAN_TEST_SUPPORTED_ROOT` on an approved ext2/3/4,
  XFS, or Btrfs test-owned directory; the writable workspace is DrvFS and
  `/tmp` is tmpfs, both correctly rejected. An elevated ext4 test-root request
  could not run because the local elevated-execution service has exhausted its
  weekly allowance. No approved-filesystem success is claimed, and T-1108
  remains queued until that exact branch executes or the blockage is resolved
  through a later locked plan.

- **2026-08-14 — T-1112 root-consumer regression scope omission resolved:**
  T-1107's locked root-consumer regression test treated both adapters as
  unsupported-only bridges and therefore banned all in-memory `IntentUnit`
  construction in `cubikan`. T-1112 explicitly supersedes that half of the
  assertion by making `cubikan` a simulation-only core consumer. The minimal
  repair keeps `cubikan-local` under the original complete ban while allowing
  only core simulation in `cubikan` and continuing to reject database, RPC,
  signing, durable-write, and synthetic-origin authority there. No production
  path or protocol surface is added outside T-1112's locked Touches.

- **2026-08-20 — T-1108 approved-filesystem execution blockage resolved:** A
  fresh owner-only test directory on the approved ext4 filesystem was made
  available through `CUBIKAN_TEST_SUPPORTED_ROOT`. The first real execution
  correctly exposed that schema-qualified configuration PRAGMAs produced
  `database_name=Some("main")` while the independent closed authorizer oracle
  requires `None`. Production now emits only the oracle's exact unqualified
  PRAGMAs, retains the deny for every schema-qualified/unlisted tuple, and uses
  non-rowid columns for empty-table probes so SQLite's special empty-column
  callback remains denied. The complete backend all-target/all-feature suite,
  including creation, read-only preflight, sidecar/path rejection, exact schema,
  page-budget rollback, and the 5,000-ms Busy path, passed on that filesystem;
  the ephemeral directory was removed by the test harness.

- **2026-08-20 — T-1109 relationship-query regression scope omission
  resolved:** T-1109's locked Touches require the private verified relationship
  implementation to live in `crates/cubikan-backend/src/relationship.rs`, but a
  T-1108 regression test broadly prohibited the literal `rusqlite` token in
  that file as well as in the unchanged public projection/module boundaries.
  The minimal test-only repair permits SQLite solely inside the private
  `VerifiedReadSnapshot` implementation, retains the prohibition for
  `projection.rs` and `lib.rs`, and explicitly rejects public raw connection or
  open entry points. It adds no path-, connection-, row-, or caller-minted
  capability surface.

- **2026-08-20 — T-1110 capability-mint scope omission resolved:** T-1110
  exclusively owns production minting of an attested `VerifiedReadSnapshot`,
  but its locked Touches omitted `crates/cubikan-backend/src/verified_read.rs`,
  where the opaque fields must remain private. The minimal repair adds only one
  crate-private constructor that accepts an already pinned hardened reader and
  exact checkpoint after full-stream comparison; the existing test issuer is
  routed through it, and the checkpoint value's formerly public constructor is
  narrowed to crate-only so callers can only receive finalized coordinates.
  No public path, connection, row, token, checkpoint-write, or caller-minting
  surface is added.

- **2026-08-20 — T-1108 nonempty projection-delete scope omission resolved:**
  T-1110's first approved ext4 replay populated schema v3 and exposed a DML
  path that T-1108's empty-table authorizer execution could not exercise:
  SQLite's foreign-key enforcement adds parent-key reads while deleting a
  relationship or association. Under `DeleteRelationship`, the observed
  suffix is `intent_units.id` twice, then
  `relationship_definitions.definition_id`,
  `relationship_definitions.definition_version`, and
  `projected_events.global_sequence`; under `DeleteAssociation` it is
  `intent_units.id` then `projected_events.global_sequence`. The minimal repair
  is confined to those exact parent columns in the two existing projection
  statement scopes, with database `main` and no accessor. The immutable
  empty-table authorizer oracle stays unchanged, and unlisted tables, columns,
  accessors, statement scopes, and all public query authority remain denied.

- **2026-08-20 — T-1111 private submission-test placement resolved:** The
  deterministic E2, E4, and E5 behavioral matrices require a scripted chain
  source, but exposing that source to integration tests would add a callable
  raw RPC/finality authority seam. Their exact named tests therefore live in
  `crates/cubikan-chain-client/src/submission.rs` behind `cfg(test)`, where the
  fake remains crate-private. `tests/submission.rs` independently seals the
  public API and consumes the frozen signing oracle, while the real-process E1,
  E3, and E6 filesystem tests remain in `tests/submission_journal.rs`. This
  placement changes no production authority or locked behavior.

- **2026-08-20 — T-1113 typed-error seam omissions resolved:** The locked
  local-v2 response contract requires `unsupported_event_schema_version` and
  an exact `revision_conflict` expected/actual pair, but T-1110 had combined a
  wrong accepted-event schema with malformed sequence evidence and T-1111 had
  intentionally discarded pallet error details after classifying
  `StaleRevision`. The minimal upstream repairs add one typed archive error and
  one constructor-closed submission detail. The latter is reconstructed from
  the signature-verified original call, the inclusion block's parent state,
  and only earlier accepted lifecycle effects in that block, so neither a
  later same-block mutation, the incoming retry, nor SQLite can supply the
  reported revision. Journal bytes and all public raw-RPC, storage, signing,
  row, and capability boundaries remain unchanged.

- **2026-08-20 — T-1113 coordinate-input plan contradiction resolved:** The
  locked operation inventory contains no coordinate-bearing request member;
  ledger coordinates are response-only. Adding one merely to exercise the
  T-1113-E1 phrase “bad coordinate” would violate the same criterion's exact
  fifteen-operation field inventory. `invalid_coordinate` therefore remains a
  closed `ErrorDetail` codec/legality value and unknown caller-supplied
  coordinate members reject as `invalid_request`; the independent request
  corpus records the coordinate-input case as not applicable rather than
  inventing a sixteenth field or operation.

- **2026-08-22 — T-1113 shared-verifier regression scope omission resolved:**
  T-1113 extends the locked shared protocol verifier to validate both adapter-
  owned v2 corpora in one canonical command, but T-1112's existing stateless
  regression asserted that the verifier's complete stdout contained only its
  original line. The minimal test-only repair preserves an exact assertion over
  the stateless evidence prefix while T-1113's local integration test pins the
  complete two-line transcript. No stateless schema, fixture, decoder, response,
  or process behavior changes.

- **2026-08-22 — T-1114 finalized-rebuild ownership contradiction resolved:**
  T-1114's locked Touches can implement and exercise the provider-neutral Git
  adapter but cannot mint a finalized chain event or construct the backend's
  private attested-read capability. Its E3 named test therefore owns the Git
  half of the invariant: record exact `RecordedAssociation` bytes, move/edit the
  source and change blame/committer metadata, then prove that resolving later
  repository state cannot rewrite those bytes or add attribution. T-1115-E2/E4
  owns the literal finalized-association and rebuilt-query proof on the real
  four-node journey and must compare the same immutable reference identity.
  Neither an in-memory vector replay nor caller-constructed read capability may
  stand in for finality or rebuild evidence.

- **2026-08-22 — T-1115 sealed-node evidence scope omission resolved:** The
  pinned argv normalizer executes the reviewed collator bytes from a sealed
  memory file, so `/proc/<pid>/exe` is the deleted named memfd rather than the
  `polkadot-omni-node` pathname that T-1107's archive-process evidence alone
  admitted. That contradiction prevents every real chain-backed T-1115 read.
  The minimal upstream repair in
  `crates/cubikan-chain-client/src/identity.rs` accepts only the exact pinned
  pathname identity or the launcher's exact named sealed memfd, requires the
  pinned executable size and SHA-256 plus the complete seal set for memory
  execution, and holds stable process, proc-directory, and executable
  descriptors while rechecking PID start time and kernel file identity. It
  adds no caller-minted RPC, finality, projection, or read-capability seam and
  does not weaken the sealed launcher or deployment identity.

- **2026-08-22 — T-1115 WebSocket bootnode pin contradiction resolved:** The
  pinned argv normalizer forces every reviewed P2P listener to the exact
  `/ip4/127.0.0.1/tcp/PORT/ws` transport, while its frozen bootnode grammar
  admitted only the transport-incompatible `/tcp/PORT/p2p/PEER` spelling.
  Pinned Zombienet correctly derives `/tcp/PORT/ws/p2p/PEER` from those live
  listeners, so the valid four-node candidate otherwise fails before launch
  or advertises a peer address on which no reviewed listener exists. The
  minimal upstream repair in `chain/tools/node-argv-grammar-v1.txt`,
  `chain/tools/normalize-node-argv.sh` and its test,
  `chain/pins.toml`, and `chain/tools/verify-pins.sh` keeps the same closed
  loopback port and peer-ID inventory but requires the exact `/ws/p2p`
  transport marker, updates the repository-tool pins and bootstrap hash, and
  adds positive/negative transport tests. It admits no non-loopback address,
  new port, external bind, alternate executable, or caller authority.

- **2026-08-22 — T-1115 process-lifecycle containment scope omission
  resolved:** T-1115-E1 requires exact four-node and orchestrator cleanup under
  a hard thirty-minute bound, but process-group cleanup alone cannot contain a
  descendant that creates another session or outlives the Rust harness. The
  minimal repair extends the existing loopback wrapper and its tests with one
  fresh PID namespace and procfs, makes the exact candidate namespace PID 1,
  and pins `unshare --pid --fork --kill-child=KILL --mount-proc`. Exiting PID 1
  therefore makes the kernel kill every remaining namespace process, while
  killing the host-side `unshare` supervisor first kills PID 1 and triggers the
  same teardown. Launch and reassertion prove the PID-namespace/procfs identity,
  and adversarial tests cover an ignored-signal, session-escaped descendant on
  both normal PID-1 exit and supervisor `SIGKILL`. This changes no node argv,
  network allowance, public API, finalized-chain authority, or projection
  capability.

- **2026-08-22 — T-1115 explicit-listener CLI contradiction resolved:** The
  pinned Zombienet provider does not generate a primary `--port`, so the closed
  config supplies that exact role-fixed allocation and the normalizer requires
  it as input evidence. The pinned relay and omni-node CLIs, however, reject a
  final argv containing both `--port` and the normalizer's explicit
  `--listen-addr`; the literal four-node candidate therefore exits before any
  socket binds. The minimal repair keeps requiring and validating every raw
  `--port` allocation but omits it from the sealed final argv, where the exact
  loopback WebSocket `--listen-addr` already fixes the same P2P port. Grammar,
  launcher oracle, tests, and repository-tool hashes update atomically. It
  admits no new input flag, port, address, executable, network route, or caller
  authority.

- **2026-08-22 — T-1115 sealed relay PVF-worker scope omission resolved:** The
  pinned relay executable normally discovers its same-release prepare and
  execute workers beside `/proc/self/exe`, but sealed-memory execution makes
  that location a deleted memfd and prevents both validators from starting.
  The minimal integration repair pins the two official stable2606-1 worker
  assets by URL, size, SHA-256, version, and commit; copies their exact opened
  bytes without following links into the fixed private
  `/run/cubikan-exec/pvf-workers` directory; and exposes that exact two-file
  tree through an executable `ro,nodev,nosuid` tmpfs bind. Every relay
  execution side carries one exact workers path and three pool bounds fixed at
  one; relay-validator bounds live in the immutable command prefix because the
  pinned Zombienet release incorrectly deduplicates repeated scalar argument
  values. The runner installs cleanup before mounting, unmounts and removes the
  tree after the owned process group exits, and the PID namespace remains the
  final lifecycle backstop. PVF workers are pinned subordinate processes—not
  additional blockchain nodes—and remain inside the process, resource, Unix-
  socket, and residue audits. This admits no alternate worker path, writable
  binary store, unbounded pool, fifth chain node, external listener, or caller
  authority.

- **2026-08-22 — T-1115 IPv4-only RPC-listener contradiction resolved:** The
  pinned node CLI's legacy `--rpc-port` path binds both `127.0.0.1` and `::1`,
  while the locked journey permits one exact IPv4 loopback RPC endpoint per
  relay execution side and collator primary. The config retains each
  `rpc_port` only as Zombienet's orchestration allocation; the launcher and
  normalizer validate and consume the provider's exact primary legacy
  `--rpc-port`, `--rpc-cors all`, and `--rpc-methods unsafe` inputs. The pinned
  provider generates only p2p, RPC, and metrics port allocations after the
  collator separator, so the frozen role mapping rejects primary policy globals
  there and supplies the same fixed policy inside one exact
  `--experimental-rpc-endpoint`
  `listen-addr=127.0.0.1:PORT,methods=unsafe,cors=all` on every primary and
  embedded relay side. No final `--rpc-port`, `--ws-port`, `--rpc-cors`, or
  `--rpc-methods` global survives. The chain-client process-identity boundary
  authenticates that same exact primary endpoint and rejects the legacy/global
  forms while admitting at most one exact embedded relay endpoint after the
  omni separator. Pinned relay and omni-node parsing proves the structured
  endpoint spelling, and the closed argv oracles cover all six fixed RPC
  ports. This admits no IPv6 bind, public address, additional endpoint, new
  port, executable, route, or caller authority.

- **2026-08-22 — T-1115 npm Git-dependency and lifecycle-shell scope omission
  resolved:** The pinned Zombienet lock resolves `toml` through one Git commit
  without registry integrity metadata, while the materializer and shared pin
  verifier previously trusted that checkout after `npm ci` and the verifier
  still delegated its build to an npm lifecycle shell. The minimal repair pins
  commit `5e17114f1af5b5b70e4f2ec10cd007623c928988` and the deterministic
  installed-content tree hash, rejects a symbolic root plus every symbolic or
  nonregular entry, and rechecks the tree immediately after each install and
  after each direct build. Both npm boundaries authenticate canonical
  `/usr/bin/git`, start from an empty environment with system/global Git
  configuration, prompts, SSH/proxy helpers, replacement objects, lazy fetch,
  and optional locking disabled, and use distinct empty regular user/global
  npm configuration files under the private build home. Fresh-cache creation
  admits one pinned HTTPS helper only in the explicit online phase, rewrites
  only the exact `toml` repository away from its lockfile SSH spelling, and
  disables credentials; the offline phase exposes no network helper or
  protocol. The live materializer never gives npm the shared cache: it copies
  that cache through no-follow descriptors into its private tmpfs home while
  bounding entries, bytes, depth, and path length, rejects identity drift, and
  verifies the private closure before and after npm can mutate it. The verifier
  now mirrors the materializer's direct pinned Node, TypeScript compiler, and
  Node filesystem build instead of invoking `npm run`; the pinned error-only
  npm log level excludes advisory URLs from the retained E5 transcript while
  preserving nonzero failures. Fresh-cache publication
  remains atomic and a concurrent or malformed cache fails closed. This admits
  no alternate Git revision, installed byte tree, package script, shell,
  executable, network helper, npm configuration source, or caller authority.

- **2026-08-22 — T-1115 derived-toolchain immutability and
  bootstrap-evidence scope omission resolved:** The pinned Zombienet and Node
  archives, lockfile, npm closure, and materializer authenticate derivation
  inputs, but the derived JavaScript/module tree previously remained writable
  under the ext4 work root after materialization; mapped-root consumers also
  retained mount capability, the offline install mutated the shared npm cache,
  and materializer/genesis stderr could be deleted outside the E5 audit. The
  minimal integration repair materializes the actual toolchain in the private
  mount-namespace-only `/run/cubikan-exec` tmpfs, uses a private verified
  snapshot of the shared npm cache, and lets only the runner self-bind/remount
  the completed tree executable `ro,nodev,nosuid`. Canonical pinned
  `/usr/bin/setpriv` launches materialization, genesis export, the orchestrator,
  and all nodes with no-new-privileges and empty inheritable, permitted,
  effective, bounding, and ambient capability sets; PVF entrypoints inherit
  that posture before applying the pinned worker sandbox described below.
  Retained `/proc` evidence and pre/post mount identity, flags, EROFS probes,
  and exact unmount/removal prove the boundary. Bounded materializer and genesis logs
  become required hash-bound E5 artifacts and undergo the same
  public/helper/secret scan. The narrowly necessary `chain/pins.toml` and
  `chain/tools/verify-pins.sh` updates pin the new host executable/cache
  contract outside T-1115's original Touches without changing runtime,
  endpoint, signer, finality, projection, or caller authority.

- **2026-08-22 — T-1115 host curl patch-level pin drift resolved:** The
  unattended host upgrade from Ubuntu's curl `8.18.0-1ubuntu2.3` package to
  `8.18.0-1ubuntu2.4` changed `/usr/bin/curl` after the sprint foundation was
  frozen, so every locked or static pin check failed before reaching the local
  journey. The minimal environment repair refreshes only that executable's
  SHA-256 after confirming the installed package and unchanged closed HTTPS
  fetch invocation. No URL, fetched artifact identity, network phase, command
  argument, or runtime authority changes.

- **2026-08-23 — T-1115 PVF nested-sandbox evidence contradiction resolved:**
  The pinned stable2606-1 prepare and execute workers intentionally call
  `unshare(CLONE_NEWUSER | CLONE_NEWNS)`. Linux then gives that same process a
  full capability mask scoped only to the new user namespace, even though its
  parent entered with an empty bounding set; requiring every observed worker
  capability field to remain zero therefore rejected the real pinned sandbox.
  The minimal evidence repair keeps the zero-capability requirement unchanged
  for the orchestrator, four nodes, and any worker still in their outer
  namespaces, while separately admitting only the exact nested posture:
  no-new-privileges remains set; inheritable and ambient capabilities remain
  zero; permitted, effective, and bounding equal the independently fixed host
  mask; user and mount namespace identities are distinct and stable; and both
  UID and GID maps are stably empty. A sampled worker must demonstrate that
  nested posture. Runtime-monitor maxima remain explicitly observed samples;
  exact pool authority comes from the sealed argv caps and fail-closed sampled
  violations, not a claim that polling observes every transient process. This
  adds no host-namespace, mount, executable, network, signer, finality,
  projection, or caller authority.

- **2026-08-23 — T-1115 failure-output isolation scope omission resolved:**
  A scanner inside the fresh PID namespace could not be the final disclosure
  boundary because a same-namespace descendant could address an ancestor's
  inherited host-output descriptor through `/proc`, and a shared host PTY
  could provide another output path. The loopback wrapper now retains the only
  real host stdout/stderr descriptors in a host-side bounded mediator. The
  exact T-1115 launch child and every inner descendant receive only distinct
  anonymous pipe writers plus `/dev/null` input, start without a controlling
  terminal, and see a private `devpts` instance. Both streams are fully drained
  and classified together: safe bounded bytes are replayed atomically, while
  an unsafe byte, secret/helper/public URL, overflow, scanner failure, or
  retained writer discards both and emits one fixed failure. PID-namespace
  teardown and descriptor proofs remain mandatory. This changes no successful
  transcript, test topology, endpoint, executable, network route, or caller
  authority.
