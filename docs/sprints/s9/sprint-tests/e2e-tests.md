# Sprint 9 End-to-End Test Results

- **Primary intent:** [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Accepted base:** `e4c7aee275b9a95e08fd7f3235addbb41df5855a`
- **Committed Build head:** `90ac02d75f7d756fad3a527487727ea4a27b9f27`
- **First critic-response head:** `0892886f83b40c3230dfe5d492d70dce1f0ecf5d`
- **Final tested critic-response head:** `153aa648847f6b3d48eef2264801807ea5316952`
- **Public-backend real-file journeys:** 2/2 passed
- **Workspace all-target regression:** 191/191 passed
- **Actual-process regressions:** `cubikan-local` 2/2 and `cubikan` 6/6 passed
- **Workspace doctests:** 1/1 passed
- **Exact-head hosted quality result:** pass; run `31362124061`, job `93372894839`, attempt 1
- **Conclusion:** pass; the two planned public Rust API journeys exercise fresh-v2 and explicit v1-to-v2 relationship/projection composition against real SQLite files, while existing process and workspace contracts remain green

The Sprint 9 product E2E boundary is the public `cubikan-backend` Rust API plus
a real test-owned SQLite file. The two named tests use `SqliteBackend` for every
product action. Raw `rusqlite` access is confined to constructing the exact-v1
legacy fixture and reading preservation snapshots; it is not substituted for
the product path after setup. The tests use neither `:memory:` SQLite nor a
mock backend, network service, shared fixed path, or private relationship
execution hook.

Unit and repository evidence is recorded in [unit-tests.md](unit-tests.md), and
the schema, migration, transaction, query, corruption, and concurrency matrix
is recorded in [integration-tests.md](integration-tests.md).

## T-908-E1 fresh-v2 relationship and projection journey

- **Exact named test:** `test_public_backend_relationship_projection_journey_across_reopen`
- **Executed target:** `cargo +stable test -p cubikan-backend --test relationship_e2e`
- **Result:** pass

The test initializes a fresh real file through the public backend and confirms
schema capability v2. It then composes the lifecycle, definition, relationship,
direct-list, and projection APIs across repeated close/reopen boundaries.

| Journey stage | Public observation | Result |
|---------------|--------------------|--------|
| Create lifecycle state | Create one `portfolio` and two `feature` units deliberately out of lexical order; retain their exact typed views. | pass |
| Create immutable definitions | Create `contains` and `depends-on` version 1 with directed identity, endpoint-species constraints, rejecting self policy, and independently selected cycle policy; returned views equal the commands. | pass |
| Create directed edges | Create two portfolio-to-feature containment edges and one feature dependency; each returns its complete relationship identity. | pass |
| Reject invalid work atomically | Attempt the reverse dependency that would close a forbidden cycle; receive typed `CycleRejected` and observe the accepted dependency page unchanged. | pass |
| Reopen and list | Reopen the file, reload both exact definitions, and list canonical direct edges as `(portfolio, feature-a)`, `(portfolio, feature-b)`, then `(feature-a, feature-b)` for the separate definitions. | pass |
| Project into multiple views | Evaluate outgoing containment, outgoing dependency, and incoming dependency predicates with lifecycle filters; the same canonical feature state appears through the applicable ephemeral views without copied membership state. | pass |
| Observe live lifecycle membership | Transition feature B from `queued` to `done`, reopen, and observe it leave queued containment/dependency projections and enter the done dependency projection while incoming membership for feature A remains valid. | pass |
| Delete without cascade | Delete exactly one containment identity; reopen and observe only feature B in the containment edge list/projection, with endpoints and the other definitions/edges retained. | pass |
| Correct by recreate | Recreate the deleted full identity, reopen again, and compare exact final unit views, definition views, canonical edge pages, retained projection queries, and projected summaries. | pass |

This realizes T-908-E1 at the declared boundary: relationship mutations never
rewrite endpoint lifecycle state, projections remain live queries rather than
stored boards, invalid policy work is nonmutating, and committed state survives
new backend connections.

## T-908-E2 exact-v1 migration and post-migration journey

- **Exact named test:** `test_public_backend_migrates_v1_then_relates_projects_and_preserves_units`
- **Executed target:** `cargo +stable test -p cubikan-backend --test relationship_e2e`
- **Result:** pass

Only fixture setup uses raw SQLite to create the accepted exact-v1 schema. All
subsequent lifecycle, migration, definition, relationship, list, and projection
behavior is invoked through the public `SqliteBackend` contract.

| Journey stage | Public observation | Result |
|---------------|--------------------|--------|
| Open exact v1 | Open the fixture as `BackendSchemaVersion::V1` and create two replay-valid feature units through existing lifecycle APIs. | pass |
| Guard every new API | Create/get definition, create/delete/list relationship, and projection each return typed `MigrationRequired { found: V1, required: V2 }`; a complete raw logical snapshot proves no mutation. | pass |
| Migrate explicitly | A separate caller invokes `SqliteBackend::migrate_v1_to_v2(path)`; all pre-existing `intent_units` column values, including stored envelope bytes, compare exactly before and after. | pass |
| Retain cached old-handle behavior | The already-open handle continues to report v1, yet existing create/get/list/transition/complete operations remain usable after the external migration. | pass |
| Keep stale relationship capability closed | The same old handle again rejects all six new APIs as migration-required, with definitions, edges, and units unchanged by those rejections. | pass |
| Reopen at v2 and compose | After dropping the stale handle, a reopened backend reports v2; it retrieves all old/new units, creates and gets a definition, creates and lists an edge, and returns its target through an outgoing projection. | pass |
| Delete/recreate and persist | Exact deletion empties the list and projection, recreation succeeds, and a final reopen returns the exact definition, edge page, projection page, and lifecycle views. | pass |
| Preserve legacy rows through later work | The two rows that existed before migration still equal their complete pre-migration raw snapshots after all later v2 definition, edge, projection, deletion, and recreation activity. | pass |

This realizes T-908-E2 without claiming automatic migration or stale-handle
upgrade. Migration is explicit, old handles retain their cached capability,
existing lifecycle operations remain compatible, and only a reopened v2 handle
can perform relationship/projection work.

## T-908-E3 workspace and actual-process regression

- **Named gate:** `verify_existing_process_and_workspace_regressions`
- **All-target command:** `cargo +stable test --workspace --all-targets`
- **Doctest command:** `cargo +stable test --doc --workspace`
- **Result:** pass

The all-target run executed 191 tests. The crate totals include library tests,
binary targets with zero tests, and integration-test binaries; the process E2Es
below are a subset of these 191 results, not an additional count.

| Crate boundary | Passed | Failed | Ignored | Measured | Filtered |
|----------------|-------:|-------:|--------:|---------:|---------:|
| `cubikan-backend` | 56 | 0 | 0 | 0 | 0 |
| stateless `cubikan-cli` / `cubikan` | 51 | 0 | 0 | 0 | 0 |
| `cubikan-core` | 65 | 0 | 0 | 0 | 0 |
| durable `cubikan-local` | 19 | 0 | 0 | 0 | 0 |
| **Total** | **191** | **0** | **0** | **0** | **0** |

The existing process suites launch the Cargo-built binaries as real child
processes with piped stdin/stdout/stderr. No relationship operation was added
to either JSON protocol.

| Actual-process suite | Exact named test | Preserved boundary | Result |
|----------------------|------------------|--------------------|--------|
| `cubikan-local` | `test_cubikan_local_persists_paginates_and_completes_across_processes` | Independent processes create/get/list/transition/reject stale work/refresh/complete one durable protocol-v1 lifecycle against a shared explicit file. | pass |
| `cubikan-local` | `test_cubikan_local_rejects_unknown_and_malformed_schema_without_mutation` | Protocol v1 still maps unsupported schema v3 and malformed owned schema to exit 4 with exact failure shape and retained logical schema markers. | pass |
| `cubikan` | `test_cli_configure_create_transition_complete` | One-shot protocol-v1 success exits 0 with the exact completed in-memory lifecycle snapshot. | pass |
| `cubikan` | `test_cli_generates_id_when_member_is_omitted` | True omission still generates a parseable non-nil UUID v4 and exits 0. | pass |
| `cubikan` | `test_cli_reports_explicit_null_id_with_exit_2` | Explicit null remains structural `invalid_request` and exits 2. | pass |
| `cubikan` | `test_cli_reports_malformed_request_with_exit_2` | Malformed JSON retains the exact `invalid_json` response and exits 2. | pass |
| `cubikan` | `test_cli_reports_lifecycle_rejection_with_exit_3` | An undeclared transition preserves prior successful in-process state and exits 3. | pass |
| `cubikan` | `test_cli_reports_oversized_request_with_exit_2` | Byte 1,048,577 remains `request_too_large` and exits 2. | pass |

The process subtotal is therefore `cubikan-local` 2/2 and `cubikan` 6/6. The
doctest run separately executed the one example in `cubikan-core/src/lib.rs`;
it passed 1/1, while the other three crates reported zero doctests.

## Exact-head hosted Rust quality run

- **Run:** [31362124061 — Rust CI](https://github.com/crussella0129/CubiKan/actions/runs/31362124061)
- **Job:** [93372894839 — Rust quality gate](https://github.com/crussella0129/CubiKan/actions/runs/31362124061/job/93372894839)
- **Event / branch:** `push` / `dev`
- **Result:** pass

The accepted base is the exact merge base of the tested candidate, and it is an
ancestor of the committed Build and both tested critic-response heads. The two
critic-response commits change five unique backend test sources; the final
commit changes only three of them. Neither commit changes production source,
documentation, a manifest, lockfile, dependency, or workflow.
Three independent checkout observations identify that final tested object:

| Provenance observation | Observed SHA | Result |
|------------------------|--------------|--------|
| Local committed `HEAD` | `153aa648847f6b3d48eef2264801807ea5316952` | match |
| Local remote-tracking `origin/dev` | `153aa648847f6b3d48eef2264801807ea5316952` | match |
| GitHub run/job API plus checkout fetch, checkout, `rev-parse`, and `git log` | `153aa648847f6b3d48eef2264801807ea5316952` | match |

The checkout log fetched
`+153aa648847f6b3d48eef2264801807ea5316952:refs/remotes/origin/dev`
and checked out that exact revision. The tested workflow blob is
`96420136d282ef93bb60b0607dffac1d28427a8d`, identical to the accepted-base
workflow blob; Sprint 9 changed neither the workflow, workspace/crate manifests,
nor `Cargo.lock`.

| Hosted field | Exact observation |
|--------------|-------------------|
| Workflow / run number | `Rust CI` / `27` |
| Display title | `test: strengthen Sprint 9 boundary oracles` |
| Run / job IDs | `31362124061` / `93372894839` |
| Attempt / previous attempt | `1` / none |
| Run status / conclusion | `completed` / `success` |
| Run created / started / updated | `2026-08-10T06:28:38Z` / `2026-08-10T06:28:38Z` / `2026-08-10T06:29:49Z` |
| Job interval | `2026-08-10T06:28:42Z`–`2026-08-10T06:29:49Z` |
| Runner | GitHub-hosted `ubuntu-latest`; `GitHub Actions 1000004629` (runner ID `1000004629`), runner version `2.336.0` |
| OS / image | Ubuntu `24.04.4 LTS`; `ubuntu-24.04` image `20260720.247.2`, provisioner `20260707.563` |
| Installed current stable | `rustc 1.97.1 (8bab26f4f 2026-07-14)` via Rustup minimal profile with `rustfmt` and Clippy |
| Checkout | `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1`; `persist-credentials: false` |
| Token permissions | explicit `Contents: read`; implicit `Metadata: read` |
| Shell / timeout | `/usr/bin/bash -e` for toolchain setup and the five gates; 15-minute job timeout |
| Dependency cache | no dependency or build-cache step declared |

Every hosted step completed successfully on attempt 1:

| Step | UTC interval | Exact command or boundary | Status / conclusion |
|-----:|--------------|---------------------------|---------------------|
| 1 | `06:28:42`–`06:28:43` | Set up GitHub-hosted job, runner, image, token permissions, and shell | `completed` / `success` |
| 2 | `06:28:43`–`06:28:44` | Check out the exact head with the pinned action and no persisted credentials | `completed` / `success` |
| 3 | `06:28:44`–`06:28:44` | `rustup toolchain install stable --profile minimal --component rustfmt,clippy` | `completed` / `success` |
| 4 | `06:28:44`–`06:28:45` | `cargo +stable fmt --all -- --check` | `completed` / `success` |
| 5 | `06:28:45`–`06:28:59` | `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | `completed` / `success` |
| 6 | `06:28:59`–`06:29:11` | `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | `completed` / `success` |
| 7 | `06:29:11`–`06:29:46` | `cargo +stable test --workspace --all-targets`; 191/191 passed, including both T-908 public-backend journeys and all eight actual-process regressions | `completed` / `success` |
| 8 | `06:29:46`–`06:29:47` | `cargo +stable test --doc --workspace`; one core doctest passed | `completed` / `success` |
| 16 | `06:29:47`–`06:29:48` | Post-checkout cleanup | `completed` / `success` |
| 17 | `06:29:48`–`06:29:48` | Complete job | `completed` / `success` |

This was the first and only attempt: no prior-attempt URL, workflow rerun, job
retry, skipped quality step, or repository-authored retry wrapper supplies the
evidence. The absence of a declared cache or retry step does not claim that
GitHub, Rustup, Cargo, the registry, or the runner image performs no internal
caching or retry behavior.

## External, flake, and claim boundary

The two relationship/projection product journeys use fixed typed identities,
real test-owned local files, and the bundled SQLite engine. They make no network
request and have no external service dependency. Temporary-directory creation,
the local filesystem, process scheduling, and the host's ability to run SQLite
remain ordinary test-host dependencies. These E2Es do not inject crash-kill,
power loss, device exhaustion/loss, network-filesystem behavior, or backup and
restore.

The hosted gate crosses the real GitHub Actions service, its floating
`ubuntu-latest` selection, Rustup's moving `stable` channel, and crates.io
index/download availability. The checkout action is immutable-pinned, but the
future runner image, compiler selected by `stable`, registry state, and hosted
availability can change. Attempt-1 success records no observed external flake;
it does not prove those dependencies are permanently available or reproducible.

The results prove the selected current-state, direct-edge, live-projection, and
explicit-migration behavior at the tested Linux revision. They do not establish
an MSRV, Windows/macOS or network-filesystem support, load/performance capacity,
branch protection, merge authorization, coverage or security certification,
release/deployment behavior, backup/recovery, downgrade behavior, crash or
forensic erasure, indefinite schema compatibility, relationship history,
idempotent correction, transitive traversal, stored boards, scheduling, graph
execution, provenance, metrics, blockchain policy, authentication, or tenancy.

## Why there is no relationship process E2E

The two `relationship_e2e` tests are genuine product E2Es at Sprint 9's locked
public Rust-plus-real-file boundary, but they are deliberately not wire/process
E2Es. `cubikan-local` protocol v1 has no relationship-definition, edge, direct
query, projection, or migration operation, and the stateless `cubikan` protocol
also remains unchanged. Relabeling their eight existing protocol, lifecycle,
and storage process regressions as relationship coverage would overstate the
tested surface; extending either protocol only to manufacture such a test would
violate T-907-E3 and INT-0012's scope.

A process-level relationship E2E becomes possible only after a separate intent
authorizes and realizes a new local protocol version (or another explicit
process/network adapter) and defines the relationship surface it exposes. Such
an adapter could then drive a real file through independent processes and
assert its own versioning, framing, error, exit, and compatibility contract.
Sprint 9 neither selects that surface nor makes a protocol-v2 or future-adapter
claim.
