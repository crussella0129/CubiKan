# Sprint 8 Test Report

## Intent Verification

| Intent | Acceptance criterion | EARS / executed evidence | Result |
|--------|----------------------|--------------------------|--------|
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Multiple units survive restart with stable identity, immutable workflows, and complete ordered history. | T-802-E1, T-804-E1–E2, T-809-E1 / envelope round trips, public create/get reopen tests, backend composition, and `test_cubikan_local_persists_paginates_and_completes_across_processes` | pass |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Bounded exact-filter keyset pagination uses canonical lexical IDs, limits 1–100, exclusive cursors, and documented live-page membership. | T-801-E4, T-805-E1–E4, T-809-E1, T-810-E1 / query model, 101-row pagination, mutable-membership, corruption, process, and documentation checks | pass |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Create/get/list/transition/complete use adapter-owned versioned boundaries and decimal revision strings rather than core Serde or direct storage edits. | T-801-E1–E3, T-802-E1/E4, T-807-E1–E4 / public model, strict envelope, revision codecs, protocol shapes, exact results, and exhaustive taxonomy tests | pass |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Every load reconstructs through core behavior; unsupported, corrupt, or projection-inconsistent state fails closed without mutation. | T-802-E1–E3, T-803-E2–E3, T-804-E4, T-805-E4, T-806-E3, T-809-E2 / replay matrices, schema rejection, projection corruption, cross-component corruption, and process fixtures | pass |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | One complete guarded update commits before success; duplicate, missing, stale, domain, busy, CAS-zero, abort, and delivery failures retain the documented atomicity boundary. | T-804-E3, T-806-E1–E5, T-807-E5, T-808-E4–E6 / real transactions, independent writers, forced rollback, protocol propagation, and post-commit output failure tests | pass |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Actual processes prove restart, multiple units, pagination, stale conflict, refreshed continuation, completion, and final retrieval. | T-809-E1–E2 / two Cargo-built `cubikan-local` E2E tests using independent processes and real explicit-path SQLite files | pass |
| [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) | Storage, schema, concurrency, recovery, pagination, delivery guarantees, and nonclaims are precise. | T-803-E1–E4, T-806-E4–E5, T-808-E1–E6, T-810-E1–E2 / exact schema/PRAGMAs, contention, runner, documentation, dependency, and scope checks | pass |

## Summary

- `cubikan-backend`: 30 passed / 0 failed.
- Existing stateless `cubikan-cli`: 51 passed / 0 failed, including 6/6 actual-process regressions.
- `cubikan-core`: 65 passed / 0 failed.
- Durable `cubikan-local`: 19 passed / 0 failed, including 2/2 actual-process E2E tests.
- Full workspace: 165 passed / 0 failed / 165 all-target tests; 1 passed / 0 failed doctest.
- Formatting, Clippy with warnings denied, warnings-denied workspace check, Book-v2 validation, Markdown navigation, dependency/scope inspection, and `git diff --check`: pass.
- Final Test Critic: [clean](critique.md) after the backend-error mapper oracle closed C-001.

Detailed arrangements and assertions are recorded in the
[unit/repository](unit-tests.md), [integration](integration-tests.md), and
[E2E](e2e-tests.md) artifacts.

## CI Confirmation

- **Tested head:** `065b71fa1b63ba6abce6effb23c9d20674171835`
- **Build task/ledger head:** `581281cb8e4ab38c0f47f4e12f085ea825b92096`
- **First Test-only cross-component commit:** `2e5e2b935bb26ea2b33b27165980839a3537ede8`
- **Final critic-response commit:** `065b71fa1b63ba6abce6effb23c9d20674171835`
- **CI run:** [Rust CI run 31344560356](https://github.com/crussella0129/CubiKan/actions/runs/31344560356)
- **Job:** [Rust quality gate 93323978596](https://github.com/crussella0129/CubiKan/actions/runs/31344560356/job/93323978596)
- **Conclusion:** success on attempt 1 for event `push`, branch `dev`

Local `HEAD`, `origin/dev`, the GitHub run API, checkout fetch/revision, and
hosted `git log -1` all identified the exact tested SHA. Setup, immutable-pinned
checkout, current-stable installation, formatting, Clippy, warnings-denied
check, all-target tests, doctests, checkout cleanup, and job completion each
reported success. The hosted run used runner 2.336.0, Ubuntu 24.04.4 image
`ubuntu-24.04` version `20260720.247.2`, and Rust 1.97.1; these are provenance,
not fixed support promises.

The hosted job is exact-revision quality evidence. It does not prove branch
protection, merge authorization, future runner/toolchain stability, network
service safety, backup/recovery, or storage behavior beyond the executed tests.

## Failures

(none)

The first Test Critic pass found that protocol code construction and three
representative executor classes did not exhaustively prove every
`BackendError` mapping arm. Test-only commit `065b71f` added
`test_backend_errors_map_exhaustively_to_protocol_codes`, which executes every
top-level backend variant and each nested transition/completion variant exactly
once. It uses a real stale conflict and an opaque failure obtained from a real
storage-open error, then checks the exact protocol code, response class,
message, and optional members. The full local and hosted suites passed at that
commit, and the second Test Critic returned `clean`.

## Technical Debt Identified

- [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) still owns immutable intent-to-artifact/commit provenance and bidirectional evidence lookup; Sprint 8 stores no provenance records.
- [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md) still owns observation identity, time, correction, privacy, and metric-definition policy; no KPI or metric engine was added.
- [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) still owns typed cross-unit relationships and advanced board projections; the realized list query is intentionally limited to unit lifecycle projections.
- Network service/API, authentication/authorization, tenancy, encryption, deletion, migration, backup, replication, automatic retry, idempotency, cross-unit transactions, and deployment remain separate outcomes.

## Coverage Observations

- Storage integration uses real on-disk SQLite files; independent-writer tests use separate connections and threads. Raw SQLite is limited to explicit corrupt/unsupported/abort fixtures and never substitutes for the production operation under test.
- The local protocol mapper now has a direct 17-case oracle in addition to the 28-code serialization taxonomy and real executor/process observations.
- Product E2E uses separate Cargo-built processes for each request and proves exact persistence, paging, stale-first rejection, refreshed continuation, completion, and fail-closed schema handling.
- The 5,000-ms busy test intentionally uses a 4.5–9-second observation window; extreme host stalls could false-fail. No retry hides that result.
- Pinned-candidate Book and Markdown checks are separate oracles: Book v2 validates 12 intent chapters, while the final Test-phase working-tree resolver inspected 116 Markdown files, 811 links, 744 local links, and 8 fragment targets with 0 errors.
- Passing tests do not claim crash-kill, power-loss, device-loss, network-filesystem, load/performance, cryptographic audit, acknowledged delivery, or indefinite compatibility guarantees.
