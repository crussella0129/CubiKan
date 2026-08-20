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
