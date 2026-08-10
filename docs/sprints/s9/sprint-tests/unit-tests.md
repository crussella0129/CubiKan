# Sprint 9 Unit and Repository Verification

- **Primary intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Affected prior intents:** [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) through [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md), traced individually below
- **Accepted base:** `e4c7aee275b9a95e08fd7f3235addbb41df5855a`
- **Build/correction head:** `90ac02d75f7d756fad3a527487727ea4a27b9f27`
- **First critic-response head:** `0892886f83b40c3230dfe5d492d70dce1f0ecf5d`
- **Final tested critic-response head:** `153aa648847f6b3d48eef2264801807ea5316952`
- **Local stable toolchain:** `rustc 1.95.0`; `cargo 1.95.0`
- **Conclusion:** pass; all seven locked unit/library primary tests, all three T-907 procedural checks, repository oracles, local quality gates, and exact-head hosted quality run passed at the final tested critic-response head

The four public-model tests execute through the crate-root API in
`crates/cubikan-backend/tests/relationship_model.rs`. The three migration and
schema-capability tests execute against real test-owned SQLite files from the
backend library test modules. No mock database, `:memory:` substitute, network,
clock, or implementation-mirroring DTO was introduced. The named T-907 checks
are read-only documentation, Git-object, metadata, and semantic-scope
inspections; they are not additional compiled Rust tests.

Exact-head hosted corroboration is summarized below and recorded in full in
[e2e-tests.md](e2e-tests.md). It remains external workflow evidence rather than
an additional unit test or merge authorization.

## T-901 public relationship and projection model

### `test_relationship_model_validates_ids_versions_policies_limits_and_cursors` — T-901-E1

- **Arrangement:** Construct definition IDs at one and 64 bytes, a mixed
  canonical ID, versions `1`, `i64::MAX + 1`, and `u64::MAX`, every self/cycle
  policy pair, limits 1 and 100, and a complete edge/cursor containing nil and
  ordinary typed endpoint IDs. Separately submit empty, 65-byte, uppercase,
  non-ASCII, and bad-character IDs, version zero, limits 0/101, and a cursor
  whose definition version differs from the query.
- **SHALL assertion and observation:** The model accepts exactly the canonical
  1–64-byte ASCII grammar, preserves the full positive `u64` range, all four
  policy combinations, complete typed edge identity, and the 1–100 page bound.
  Rejections follow empty → byte length → first byte → remaining byte
  precedence, report the exact zero-based invalid-byte index, reject version
  zero and out-of-range limits, and return typed
  `CursorDefinitionMismatch { expected, actual }` before storage access.
- **Result:** pass.

### `test_public_relationship_model_exposes_complete_contract` — T-901-E2

- **Arrangement:** Construct and inspect definition keys, create commands and
  views with both species constraints and both policies; complete edge
  create/delete/view/cursor values; a filtered relationship query/page; outgoing
  and incoming direct predicates; projection-v1 queries with and without a
  relationship predicate; retained pages and next cursors; and both supported
  backend-schema values.
- **SHALL assertion and observation:** Every locked definition, edge, query,
  result, filter, direction, endpoint, item, and cursor field is available
  through the public crate-root values without narrowing core IDs. The
  relationship cursor retains the complete edge identity, projection pages
  retain the complete version-1 query, and `ProjectionQueryV1::VERSION`,
  `version()`, `BackendSchemaVersion::V1.value()`, and `V2.value()` return
  exactly 1, 1, 1, and 2. The enum/option shape permits at most one direct
  projection predicate, and `DeleteRelationship` requires one complete
  identity rather than a partial deletion key.
- **Result:** pass.

### `test_relationship_error_taxonomy_is_typed_and_source_preserving` — T-901-E3

- **Arrangement:** Construct definition-ID/version/query errors and every
  relationship classification; exercise source/target roles and species
  fields; wrap corrupt-envelope, corrupt-schema, missing-unit, real storage,
  and real five-second writer-busy backend failures; exhaustively match the
  existing `BackendError` variants; and construct both migration-error forms.
- **SHALL assertion and observation:** Migration-required, definition
  duplicate/missing/corrupt, endpoint missing/corrupt/species mismatch,
  self/cycle rejection, edge duplicate/missing/corrupt, and backend failures
  remain distinct typed variants with exact fields. `EndpointCorrupt`,
  `RelationshipError::Backend`, and `MigrationError::Backend` retain their
  nested `Error::source` chains, including SQLite's underlying diagnostic,
  without message parsing. `SourceVersionNotOne` remains source-free, and the
  pre-existing backend/local error taxonomy remains exhaustive and unchanged.
- **Result:** pass.

### `test_relationship_model_does_not_expose_storage_or_execution_authority` — T-901-E4

- **Arrangement:** Compile-check the complete public value inventory, inspect
  `relationship.rs`, `projection.rs`, and crate root `lib.rs`; then reject
  SQL/storage-row, `Board`, `StoredBoard`, `ExecutionGraph`, Serde cursor,
  time/actor, local-protocol, scheduler, and executor symbols. Inspect the
  relationship cursor for textual display/parsing implementations.
- **SHALL assertion and observation:** Public relationship/projection types are
  ordinary cloneable, comparable values re-exported from private modules. The
  complete public surface contains no `rusqlite` or stored-row DTO, direct
  Serde exposure, `Board`/`StoredBoard`/`ExecutionGraph`, timestamp/actor,
  local request/response, scheduler, or executor authority; the cursor has no
  provisional `Display`, `FromStr`, or Serde contract. No stored-board or
  execution-graph API can hide in crate-root re-exports.
- **Result:** pass.

The exact targeted command was:

```text
cargo +stable test -p cubikan-backend --test relationship_model
```

It returned 4 passed, 0 failed, 0 ignored, 0 measured, and 0 filtered out.

## T-902 schema capability and migration library tests

### `test_exact_v1_retains_unit_operations_and_caches_relationship_migration_guard` — T-902-E2

- **Arrangement:** Create a literal exact-v1 file, open it through the public
  backend, create and retrieve one unit, list it, transition it, complete it,
  snapshot the stored row, inspect schema ownership and connection settings,
  and call the private relationship-capability guard.
- **SHALL assertion and observation:** The handle reports cached schema version
  1 and successfully retains create/get/list/transition/complete. The guard
  returns exact typed `MigrationRequired { found: V1, required: V2 }` before
  relationship SQL. The complete stored row and exact v1 ownership remain
  unchanged across that rejection, and the realized DELETE/EXTRA,
  foreign-key/trusted-schema/read-uncommitted/locking/busy connection contract
  still validates.
- **Result:** pass.

### `test_explicit_migration_orders_version_last_and_preserves_all_unit_columns` — T-902-E3

- **Arrangement:** Seed exact v1 with one public-created unit; snapshot all eight
  `intent_units` columns; execute the production migration with a private stage
  observer; compare the post-migration row; then exercise an already-open v1
  handle and a newly reopened handle.
- **SHALL assertion and observation:** The observer sees version 1 at
  `BeforeVersion` and version 2 only at `AfterVersion`; migration commits exact
  v2 while preserving every typed/text/blob column value exactly. The old
  handle still reports v1, successfully gets the existing unit, and rejects the
  private relationship guard. Only the reopened v2 handle reports v2 and passes
  that guard.
- **Result:** pass.

### `test_busy_interrupted_and_racing_migrations_leave_one_exact_state` — T-902-E5

- **Arrangement:** Use three independent exact-v1 files. Inject a typed failure
  after the in-transaction version step on the first; hold a real immediate
  writer lock while migrating the second; and barrier-start two migration
  threads against the third.
- **SHALL assertion and observation:** The injected pre-commit failure returns
  `ConcurrentStorageChange` and rolls back to exact v1. The locked attempt
  returns one source-retaining `StorageBusy` without retry and leaves exact v1.
  The race produces exactly one success and one
  `SourceVersionNotOne { found: 2 }`, with one final exact-v2 store and no
  partial state.
- **Result:** pass.

The exact targeted command was:

```text
cargo +stable test -p cubikan-backend --lib
```

It returned 9 passed, 0 failed, 0 ignored, 0 measured, and 0 filtered out. The
three named T-902 tests above were its locked Sprint 9 primaries; the other six
tests were retained backend-model, envelope, codec, and fresh-schema
regressions.

## Critic-response ownership and history

First critic-response commit `0892886f83b40c3230dfe5d492d70dce1f0ecf5d`
changes exactly four existing real-file integration test files and no
production or protected object. It adds assertions and fixtures without adding
or renaming a test function.

The detailed Arrangement/SHALL evidence for these critic responses belongs in
[integration-tests.md](integration-tests.md), not in the unit-primary count:

- **C-001:** `test_migration_rejects_unowned_corrupt_and_wrong_version_sources`
  now places replay-valid canonical unit `...0001` before corrupt canonical unit
  `...0002`, requiring migration to reach the later row and reject atomically.
  Its complete rejection snapshots also retain the unowned table and
  `foreign_data('keep')` row rather than checking object shape alone.
- **C-002:** `test_edge_policy_rejections_are_atomic` now pairs adjacent
  conflicting failures across definition, endpoint replay, endpoint species,
  self-edge, duplicate, and reachability checks, and also proves an
  otherwise-valid cycle rejection leaves the full durable snapshot unchanged.
  The adjacent lifecycle-independence strengthening uses transitioned and
  completed endpoints for create, delete, and recreate.
- **C-003 and C-004:** direct-list source-endpoint validation and hostile
  incoming-projection role/species/corruption coverage likewise belong to the
  relationship-query and projection integration results. They are not
  relabeled as unit tests.

Second critic-response commit
`153aa648847f6b3d48eef2264801807ea5316952` changes exactly three existing
test files relative to `0892886` and still changes no production or protected
object:

- The migration rejection primary now tables corrupt-first, corrupt-middle,
  and corrupt-last canonical replay order. Each fixture contains IDs `...0001`,
  `...0002`, and `...0003`, with the corrupt row independently occupying
  ordinal 1, 2, or 3; every position returns `CorruptEnvelope` and preserves
  the full pre-call snapshot.
- T-901-E4 strengthens the public-symbol oracle itself: the same named test now
  scans `lib.rs` in addition to both private model modules and explicitly
  rejects `Board`, `StoredBoard`, and `ExecutionGraph` symbols. This is one of
  the seven unit/library primaries, not a new test.
- T-905's real-file query primaries now prove continuation across a composite
  `(source,target)` cursor boundary where the next source has lower target IDs,
  and prove selected target-species mismatch with the exact `Target` role.
  Both observations remain integration-owned and are detailed in
  `integration-tests.md`.

Neither response adds or renames a test function. The final inventory therefore
remains seven unit/library primaries, 191 all-target tests, and one doctest.

## T-907 documentation and protected-scope checks

These are the three named procedural checks from the finalized Test plan. They
were executed against the root overview, backend guide, local-protocol guide,
accepted-base Git objects, exact tested tree, and production source.

### `test_backend_docs_define_schema_v2_relationship_migration_and_projection_contract` — T-907-E1

- **Arrangement:** Checklist `README.md`, `crates/cubikan-backend/README.md`, and
  `crates/cubikan-local/README.md` together against the exact schema,
  migration, relationship-store, query, projection, and backend implementation
  at final tested head `153aa648`.
- **SHALL assertion and observation:** The guides publish the independent
  envelope-v1/schema-v1-and-v2/relationship-v1/projection-v1/protocol-v1 matrix;
  exact v2 object, DDL, constraint, foreign-key, PRAGMA, and structural-open
  behavior; explicit non-retrying v1-to-v2 migration, version-last ordering,
  byte-preserved unit columns, race/interruption behavior, recovery copy, and
  reopen/cached-handle rules. They state complete immutable definition/edge
  identity, policy and validation precedence, non-cascading delete/recreate,
  typed errors, direct bounded canonical cursors, projection directions,
  retained versioned queries, reproducibility from unchanged canonical state,
  selected/lookahead corruption, busy precedence, and live pages.
- **Result:** pass.

### `test_docs_separate_projection_from_execution_graph_and_list_nonclaims` — T-907-E2

- **Arrangement:** Inspect all three guides' boundary and exclusion sections,
  then compare them with INT-0012's negative boundary and the finalized plan.
- **SHALL assertion and observation:** Projection remains an ephemeral read
  model, never stored membership, authorization, scheduling state, a workflow
  edge, or an execution graph. The guides explicitly exclude automatic
  migration/repair/backup, downgrade/reverse migration, progress/resume/
  cancellation/fixed duration, old-binary or indefinite compatibility,
  definition list/delete/latest/history, relationship revision/history/actor/
  timestamp, idempotent or atomic correction, cascade and forensic erasure,
  stored boards/results and snapshots, transitive/Boolean graph traversal,
  delegation/readiness/scheduling/retries/fan-out/WIP/skills/artifact routing/
  executor policy, protocol-v1 relationship expansion, auth/tenancy/network/UI/
  deployment, provenance, metrics/KPI behavior, and blockchain policy.
- **Result:** pass.

### `test_sprint_nine_scope_preserves_core_envelope_and_local_protocol_v1` — T-907-E3

- **Arrangement:** Compare accepted base `e4c7aee` with final tested head
  `153aa648` by
  changed-path inventory and quiet Git-object diffs; inspect metadata and the
  complete normal dependency tree; compare local production request/result/
  error sources and fixtures; run the existing process regressions and
  `git diff --check`.
- **SHALL assertion and observation:** The 35-path accepted-base delta is one
  root guide, 19 backend paths, two local-adapter documentation/test paths, and
  13 Book/ledger paths. `crates/cubikan-core/**`, `crates/cubikan-cli/**`,
  `crates/cubikan-backend/src/stored.rs`, every `crates/cubikan-local/src/**`
  production file, local protocol/runner tests and checked-in fixture,
  `Cargo.toml`, `Cargo.lock`, and `.github/workflows/**` are byte-identical to
  the accepted base. INT-0001 through INT-0011 chapters are also byte-identical.
  The only local process-test change advances the unsupported-schema fixture
  from now-supported v2 to unsupported v3 while retaining
  `unsupported_schema_version`, response shape, exit 4, empty stderr, and
  logical nonmutation checks. No operation, field, result, or error code enters
  local protocol v1. Relative to Build/correction head `90ac02d`, the final
  tested head modifies only `crates/cubikan-backend/tests/migration.rs`,
  `crates/cubikan-backend/tests/projection.rs`,
  `crates/cubikan-backend/tests/relationship_model.rs`,
  `crates/cubikan-backend/tests/relationship_mutations.rs`, and
  `crates/cubikan-backend/tests/relationship_query.rs`; the accepted-base path
  count remains 35 and every production/protected Git object remains unchanged.
  `git diff --check` passes.
- **Result:** pass.

The T-906 semantic task originally committed its four projection tests as
`projection_query.rs`. Commit
`90ac02d75f7d756fad3a527487727ea4a27b9f27` is the trace-preserving correction
that renames that 99%-identical target to the finalized-plan path
`crates/cubikan-backend/tests/projection.rs` and adds only a module-level
coverage comment. Test names, EARS mappings, and behavior are unchanged; all
four pass at that correction boundary. First response `0892886` then
strengthens the same `projection.rs` integration target without changing its
test name or count; final head `153aa648` retains that coverage unchanged. The
added evidence is owned by [integration-tests.md](integration-tests.md).

## Book, navigation, metadata, and dependency evidence

The authoritative Book-v2 validator command was:

```text
bash /mnt/c/Users/charl/Animus_Sprint_Loops/codex-cli/skills/sprint-loops/scripts/check-book.sh
```

It returned exactly `check-book: valid v2 Book (12 intent chapters)`.
INT-0012 is active, is reachable from `docs/SUMMARY.md`, and legally links its
Sprint 9 Work evidence. The validator proves Book schema, intent uniqueness,
state, and evidence shape; it is not the Markdown reachability oracle.

A separate one-off, read-only Python 3 regular-expression/path resolver read
Markdown blobs pinned to final tested head
`153aa648847f6b3d48eef2264801807ea5316952` rather than the later Test prose
working tree. Its Markdown tree is byte-identical to both first response
`0892886` and Build/correction head `90ac02d`. It inspected 125 Markdown files,
868 Markdown links, 796 local links, and 13 local fragment references, with 0
missing path or fragment errors. The pinned count therefore remains honest
after this artifact itself adds links.

Immediately before this final report revision, the same resolver inspected the
populated working tree, including the link-free critique file: 126 Markdown
files, 893 Markdown links, 817 local links, and 13 local fragment references,
with 0 missing path or fragment errors. Those are explicitly pre-report-change
counts; the hosted links added by this revision are included only in the final
handoff resolver. This working-tree checkpoint validates the evidence documents
without replacing the exact-commit oracle above.

The exact workspace commands were:

```text
cargo +stable metadata --no-deps --format-version 1
cargo +stable tree --workspace --edges normal
```

Metadata reports exactly four Rust 2024 packages. The complete normal tree is
unchanged from the accepted base: `cubikan-core` depends on Serde and UUID;
`cubikan-cli` depends on core, Serde, and Serde JSON; `cubikan-backend` depends
on core, Serde, Serde JSON, and bundled `rusqlite = 0.40.2`; and
`cubikan-local` depends on backend, core, Serde, and Serde JSON. No manifest,
lockfile, dependency, workspace member, package, or CI/workflow change occurred.

## Canonical local quality gates

Every command below passed against final tested head
`153aa648847f6b3d48eef2264801807ea5316952`:

| Gate | Exact command | Exact result |
|------|---------------|--------------|
| Formatting | `cargo +stable fmt --all -- --check` | pass; no diff |
| Clippy | `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | pass; zero warnings |
| Warnings-denied check | `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | pass; zero warnings |
| All-target tests | `cargo +stable test --workspace --all-targets` | pass; 191 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out |
| Doctests | `cargo +stable test --doc --workspace` | pass; 1 core doctest, 0 failed |
| Accepted-base whitespace | `git diff --check e4c7aee275b9a95e08fd7f3235addbb41df5855a..153aa648847f6b3d48eef2264801807ea5316952` | pass; no output |

## Exact-head hosted Rust quality corroboration

- **Run:** [31362124061 — Rust CI](https://github.com/crussella0129/CubiKan/actions/runs/31362124061)
- **Job:** [93372894839 — Rust quality gate](https://github.com/crussella0129/CubiKan/actions/runs/31362124061/job/93372894839)
- **Checked-out head:** `153aa648847f6b3d48eef2264801807ea5316952`
- **Result:** pass

| Hosted field | Exact observation |
|--------------|-------------------|
| Workflow / run number | `Rust CI` / `27` |
| Display title | `test: strengthen Sprint 9 boundary oracles` |
| Event / branch / attempt | `push` / `dev` / `1`; no prior attempt |
| Run / job IDs | `31362124061` / `93372894839` |
| Run status / conclusion | `completed` / `success` |
| Run created / started / updated | `2026-08-10T06:28:38Z` / `2026-08-10T06:28:38Z` / `2026-08-10T06:29:49Z` |
| Job interval / conclusion | `2026-08-10T06:28:42Z`–`2026-08-10T06:29:49Z`; `success` |
| Runner | GitHub-hosted `ubuntu-latest`; `GitHub Actions 1000004629` (runner ID `1000004629`), runner version `2.336.0` |
| OS / image | Ubuntu `24.04.4 LTS`; `ubuntu-24.04` image `20260720.247.2`, provisioner `20260707.563` |
| Installed current stable | `rustc 1.97.1 (8bab26f4f 2026-07-14)` via Rustup minimal profile with `rustfmt` and Clippy |
| Checkout | `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1`; `persist-credentials: false` |

The hosted checkout, `rev-parse`, and log observations all identify the exact
final tested head above. Workflow blob
`96420136d282ef93bb60b0607dffac1d28427a8d` is unchanged from the accepted
base. The same pinned checkout action, runner image, provisioner, and moving
stable toolchain used by the prior hosted checkpoint executed all five canonical
commands successfully: formatting, Clippy, warnings-denied all-target checking,
191/191 all-target tests, and one core doctest. This attempt-1 success is
external Linux corroboration, not an MSRV, cross-platform promise, branch
protection, or merge authorization.

## Exact all-target suite breakdown

| Crate / target | Passed | Failed | Ignored / measured / filtered |
|----------------|-------:|-------:|-------------------------------:|
| `cubikan-backend` library unit tests | 9 | 0 | 0 |
| `cubikan-backend` corruption integration target | 1 | 0 | 0 |
| `cubikan-backend` migration integration target | 1 | 0 | 0 |
| `cubikan-backend` model integration target | 4 | 0 | 0 |
| `cubikan-backend` mutations integration target | 7 | 0 | 0 |
| `cubikan-backend` persistence integration target | 4 | 0 | 0 |
| `cubikan-backend` projection integration target | 4 | 0 | 0 |
| `cubikan-backend` query integration target | 4 | 0 | 0 |
| `cubikan-backend` relationship-definition integration target | 4 | 0 | 0 |
| `cubikan-backend` relationship E2E target | 2 | 0 | 0 |
| `cubikan-backend` relationship-model integration target | 4 | 0 | 0 |
| `cubikan-backend` relationship-mutation integration target | 5 | 0 | 0 |
| `cubikan-backend` relationship-query integration target | 4 | 0 | 0 |
| `cubikan-backend` schema integration target | 3 | 0 | 0 |
| **`cubikan-backend` subtotal** | **56** | **0** | **0** |
| `cubikan-cli` library unit tests | 32 | 0 | 0 |
| `cubikan` binary unit tests | 0 | 0 | 0 |
| `cubikan-cli` actual-process E2E target | 6 | 0 | 0 |
| `cubikan-cli` public-runner integration target | 13 | 0 | 0 |
| **Stateless CLI subtotal** | **51** | **0** | **0** |
| `cubikan-core` library unit tests | 44 | 0 | 0 |
| `cubikan-core` lifecycle integration target | 16 | 0 | 0 |
| `cubikan-core` serialization integration target | 5 | 0 | 0 |
| **Core subtotal** | **65** | **0** | **0** |
| `cubikan-local` library unit tests | 3 | 0 | 0 |
| `cubikan-local` binary unit tests | 0 | 0 | 0 |
| `cubikan-local` actual-process E2E target | 2 | 0 | 0 |
| `cubikan-local` protocol integration target | 8 | 0 | 0 |
| `cubikan-local` runner integration target | 6 | 0 | 0 |
| **Durable local subtotal** | **19** | **0** | **0** |
| **All-target total** | **191** | **0** | **0** |

Workspace doctests add one passing `cubikan-core` doctest. No test failed, was
ignored, measured, or filtered.

## Affected prior-intent regression evidence

Every prior intent chapter is byte-identical to the accepted base. The table
records why each prior outcome is affected by, or protected from, the schema-v2
relationship work and which passing regressions preserve its authority.

| Prior intent | Executed evidence and observed boundary | Result |
|--------------|------------------------------------------|--------|
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | All 65 unchanged core tests preserve typed identity, caller-owned workflow topology, exact transition/completion behavior, revision/history, and validated restoration. Backend endpoint replay composes with those rules; an accepted edge does not mutate either aggregate. | pass |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | All 51 unchanged stateless CLI tests, including six real child processes, preserve the one-shot in-memory configure/create/operate contract. Durable relationships remain a separate Rust backend API. | pass |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Unchanged runner/process regressions retain the exact 1 MiB boundary, one-byte lookahead, and oversize-before-JSON precedence. No relationship request enters either protocol-v1 ingestion shape. | pass |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Unchanged stateless and durable runner tests preserve body → newline → one flush ordering, output-error precedence, and the no-acknowledgement boundary. Schema v2 does not change response delivery. | pass |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | `.github/workflows/ci.yml` is byte-identical and all five local Rust gates pass. Exact-head hosted Rust CI run 27 also passes all five gates at `153aa648`; it remains external corroboration and does not alter human merge authority. | pass |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | Unchanged protocol, runner, and actual-process tests still distinguish omitted IDs from explicit null/wrong types and generate a non-nil UUID v4 only for true omission. | pass |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | The advisory derivative appendix is unchanged. T-907 documentation keeps projections reusable but separate from manager, skill-graph, scheduling, UI, and organizational policy; no derivative repository is created. | pass |
| [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) | The proposed provenance intent is unchanged. Relationship definitions/edges add no actor, timestamp, agent, commit, blame, attribution, or traceability authority. | pass; remains proposed |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | All core revision tests and backend mutation/process regressions pass. Exact-v1 and v2 lifecycle operations retain typed stale-before-domain behavior, while relationship mutations leave endpoint revisions and histories unchanged. | pass |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | All 56 backend and 19 durable-local tests preserve replay-valid envelope v1, lifecycle CRUD/query/mutation, transaction/CAS, error, pagination, and process behavior. Schema v1 remains usable and changes only through the separately selected explicit, atomic, byte-preserving migration to exact v2. | pass |
| [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md) | The proposed measurement intent is unchanged. Relationship/projection state carries no KPI, observation, attainment, metric evaluation, or automatic transition-authorization semantics, as both public API inventory and T-907 nonclaims confirm. | pass; remains proposed |

This evidence proves the locked unit/library promises and protected repository
scope at the tested revision. It does not claim relationship wire/process E2E,
snapshot pagination, transitive execution, secure deletion, migration backup or
downgrade, network-filesystem behavior, crash/power-loss immunity, an MSRV, or
indefinite compatibility.
