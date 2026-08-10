# Sprint 9 Test Report

## Intent Verification

| Intent | Acceptance criterion | EARS / executed evidence | Result |
|--------|----------------------|--------------------------|--------|
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Callers can create and query immutable, versioned relationship definitions and validated directed edges without changing either endpoint's lifecycle authority. | T-901-E1–E2, T-903-E1–E2, T-904-E1, T-908-E1 / the public-model boundary tests; exact definition create/get and independent-version reopen tests; `test_edge_create_commits_without_mutating_endpoints`; and the fresh-v2 public-backend journey. The edge test uses active revision-1 and completed revision-2 endpoints and proves create, reopen, delete, and recreate preserve every endpoint row. | pass |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Definitions enforce optional endpoint species, explicit self/cycle policy, exact-duplicate rejection, and cycle scope limited to one complete definition version. | T-901-E1–E3, T-903-E1–E2, T-904-E2–E4 / public policy/error construction; the four persisted policy combinations and independent full-`u64` versions; `test_edge_policy_rejections_are_atomic`; `test_self_and_cycle_policy_matrix_is_version_scoped`; and the two-writer opposite-edge race. | pass |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Unknown-endpoint and policy-invalid relationships reject atomically, while concurrent writers serialize through the backend transaction boundary. | T-904-E2, T-904-E4, T-908-E1 / conflicting definition, endpoint, species, self, duplicate, reachability, and cycle failures retain complete durable snapshots; concurrent opposite proposals commit exactly one edge and reject exactly one as `CycleRejected`; the public journey observes policy rejection without changing the accepted page. | pass |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Deletion requires a complete edge identity and valid selected definition/endpoints/species, is physical and non-cascading, and correction is two independently committed delete/recreate operations. | T-901-E2, T-904-E1, T-904-E5, T-908-E1–E2 / the complete public delete command; lifecycle-independent delete/recreate; `test_edge_delete_is_exact_non_cascading_and_atomic_on_failure` over missing/corrupt selected state, species mismatch, busy, and abort fixtures; and both real-file public-backend journeys across reopen. | pass |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Exact definition lookup and direct relationship listing obey bounded canonical ordering, an exclusive complete-edge cursor, live pages, exact filters, and the specified missing/corrupt behavior. | T-903-E2–E4, T-905-E1–E4 / independent exact-version get, typed missing/corrupt definitions, and all four relationship-query primaries. The pagination test proves limits 1/100, lookahead, live membership, absent ordering cursors, and continuation across a composite `(source,target)` boundary where the later source has lower target IDs; corruption tests prove exact source/target roles and species failures without partial pages. | pass |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | A unit can appear in multiple ephemeral projections that AND lifecycle filters with at most one direct relationship predicate without copied membership or lifecycle ownership transfer. | T-901-E2, T-906-E1–E2, T-906-E4, T-908-E1–E2 / the unrepresentable-multiple-predicate model; direct outgoing/incoming filter composition; `test_unit_appears_in_multiple_live_projections_without_copied_state`; hostile anchor/candidate role and species/corruption matrices; and both public-backend projection journeys. | pass |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Projection query v1 returns reproducible canonical-ID pages with required limits, exclusive ID cursors, retained query identity, and documented live rather than snapshot membership. | T-901-E1–E2, T-906-E3, T-907-E1 / model bounds and versioned page shape; `test_projection_v1_reports_query_and_uses_exclusive_live_pages` over 101 units, unchanged-state equality, absent cursors, and between-page mutations; and the guide contract check. | pass |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Fresh stores use exact schema v2, exact v1 remains lifecycle-usable until explicit atomic migration, invalid/busy/interrupted/racing inputs never leave partial state, and callers reopen after success. | T-902-E1–E6, T-908-E2 / fresh/reopen exact-schema and connection checks; cached exact-v1 lifecycle/capability checks; byte-preserving version-last migration; missing/unowned/corrupt/wrong-version input matrices; one-call busy, injected interruption, and racing migration tests; corrupt-v2 open rejection; and the exact-v1-to-v2 public journey. The final migration matrix places corruption first, middle, and last among three canonical rows and preserves each complete pre-call snapshot. | pass |
| [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) | Documentation separates multi-board projection from execution graphs, confines the realization to the Rust backend API, preserves local JSON protocol v1, and states migration, compatibility, history, identity, authorization, and execution nonclaims. | T-901-E3–E4, T-907-E1–E3, T-908-E3 / the typed boundary and public-symbol scan; all three documentation/protected-scope procedural checks; the 191-test workspace and eight-process regression; and the one doctest. The symbol oracle scans the crate root and both model modules for forbidden `Board`, `StoredBoard`, `ExecutionGraph`, storage, actor/time, scheduler, executor, and protocol authority; accepted-base Git objects prove the local protocol production surface is unchanged. | pass |

## Summary

- `cubikan-backend`: 56 passed / 0 failed.
- Existing stateless `cubikan-cli` / `cubikan`: 51 passed / 0 failed,
  including 6/6 actual-process regressions.
- `cubikan-core`: 65 passed / 0 failed.
- Durable `cubikan-local`: 19 passed / 0 failed, including 2/2
  actual-process regressions.
- Full workspace: 191 passed / 0 failed / 191 all-target tests; 1 passed / 0
  failed doctest.
- Formatting, Clippy with warnings denied, warnings-denied workspace check,
  Book-v2 validation, Markdown navigation, dependency/scope inspection, and
  `git diff --check`: pass.
- Final Test Critic: [clean](critique.md) after two test-only strengthening
  commits closed the first four concerns and the residual boundary gaps.

Detailed arrangements and assertions are recorded in the
[unit/repository](unit-tests.md), [integration](integration-tests.md), and
[E2E](e2e-tests.md) artifacts.

## CI Confirmation

- **Final tested head:** `153aa648847f6b3d48eef2264801807ea5316952`
- **Build/correction head:** `90ac02d75f7d756fad3a527487727ea4a27b9f27`
- **First critic-response head:** `0892886f83b40c3230dfe5d492d70dce1f0ecf5d`
- **Final critic-response head:** `153aa648847f6b3d48eef2264801807ea5316952`
- **CI run:** [Rust CI run 31362124061](https://github.com/crussella0129/CubiKan/actions/runs/31362124061)
- **Job:** [Rust quality gate 93372894839](https://github.com/crussella0129/CubiKan/actions/runs/31362124061/job/93372894839)
- **Conclusion:** success on attempt 1 for event `push`, branch `dev`

Local `HEAD`, `origin/dev`, the GitHub run/job API, checkout fetch/revision, and
hosted `git log -1` all identified the exact final tested SHA. Setup,
immutable-pinned checkout, current-stable installation, formatting, Clippy,
warnings-denied check, all-target tests, doctests, checkout cleanup, and job
completion each reported success. The hosted run used runner 2.336.0, Ubuntu
24.04.4 image `ubuntu-24.04` version `20260720.247.2`, and Rust 1.97.1; these
are provenance, not fixed support promises.

The Build head is the completed implementation/evidence point. Both later
critic-response commits modify tests only; no production source, manifest,
lockfile, dependency, workflow, protected core tree, or local protocol-v1
production object changed between the Build and final tested heads.

## Failures

(none)

The first formal Test Critic reported four evidence gaps:

- **C001:** migration corruption rejection did not prove a later corrupt row
  was reached after an earlier replay-valid row.
- **C002:** edge-validation precedence used isolated failures rather than
  conflicting adjacent failures.
- **C003:** direct relationship listing lacked a selected corrupt source
  endpoint oracle.
- **C004:** incoming projection lacked hostile source/target role, corruption,
  and species observations.

First response `0892886` changed four existing integration test files only. It
added a replay-valid row before later migration corruption and retained unowned
sentinel data in rejection snapshots; paired every adjacent edge-validation
precedence boundary with conflicting failures; proved relationship eligibility
across active/done revision-1 and completed/done revision-2 endpoints; covered
both corrupt direct-list endpoint roles; and covered hostile incoming
projection anchor/candidate roles, corruption, and species mismatches.

Final response `153aa648` strengthened three remaining oracles without changing
production code or test counts. Migration corruption is independently first,
middle, and last in canonical three-row scans; the T-901-E4 public API check
scans `lib.rs`, `relationship.rs`, and `projection.rs` and rejects board/graph
authority symbols; and T-905 proves composite-cursor continuation across a new
source with lower target IDs plus selected target-species mismatch. The full
local and hosted suites passed at that head, and the final formal critic
returned [clean](critique.md).

## Technical Debt Identified

- [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md)
  still owns immutable intent-to-artifact/commit provenance and bidirectional
  evidence lookup; relationship rows carry no actor, timestamp, agent, commit,
  blame, or attribution record.
- [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md)
  still owns observation identity, time, correction, privacy, and metric/KPI
  policy; projections do not evaluate or authorize lifecycle transitions.
- A future intent must select a new local protocol version or another explicit
  process/network adapter before relationship process E2E is meaningful.
  Sprint 9 deliberately preserves JSON protocol v1 and tests its eight existing
  lifecycle/storage process journeys only as regressions.
- Definition listing/deletion/latest-version resolution, relationship
  revisions/history/idempotent correction, transitive queries, stored boards,
  and execution-graph behavior remain separate outcomes.

## Coverage Observations and Nonclaims

- Product relationship E2E uses only public `SqliteBackend` operations over
  real test-owned SQLite files across reopen. Raw SQLite is limited to exact-v1
  and hostile fixture setup plus preservation inspection; it does not replace
  the operation under test.
- Independent connections and OS threads exercise writer serialization. Tests
  observe one-call busy and rollback outcomes, but do not instrument SQLite's
  internal attempt behavior or claim crash-kill, power-loss, device-loss, or
  network-filesystem recovery.
- Relationship and projection pages are live committed views, not
  cross-request snapshots. The tests do not claim transitive execution,
  retained correction history, cascade behavior, forensic erasure,
  authorization, tenancy, UI, scheduling, retries, WIP, provenance, metrics,
  or blockchain policy.
- Migration creates no automatic backup and promises no downgrade, reverse
  migration, progress/resume, fixed duration, old-binary compatibility, or
  indefinite schema guarantee. The evidence does not establish performance,
  an MSRV, Windows/macOS support, branch protection, or merge authorization.
- The exact tested commit's Markdown tree contained 125 files, 868 links, 796
  local links, and 13 local fragment references with 0 errors. A separate
  working-tree resolver immediately before this report and INT evidence were
  added inspected 126 Markdown files, 893 links, 817 local links, and 13 local
  fragment references with 0 errors. Those working-tree counts are explicitly
  pre-report and must not be presented as the final handoff count after this
  report's links and the later intent-evidence link are added.
