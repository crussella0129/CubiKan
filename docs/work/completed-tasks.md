# Completed Tasks Log (Append-Only)

## T-001 (sprint 0)
- **Description:** Record foundational architecture decisions
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:27:24Z
- **Files modified:** `decisions.md`, `.gitignore`, `agent-tasks/agent-tasks.md`, `agent-tasks/completed-tasks.md`
- **Commit:** `b8fe6811352fcae801c72d09e08d2022e43b27dc`

## T-002 (sprint 0)
- **Description:** Scaffold the Cargo workspace and cubikan-core crate
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:30:45Z
- **Files modified:** `.gitignore`, `Cargo.toml`, `Cargo.lock`, `crates/cubikan-core/Cargo.toml`, `crates/cubikan-core/src/lib.rs`
- **Commit:** `4d77343f45ea4cbc6b906097127b8d884551fca8`

## T-003 (sprint 0)
- **Description:** Implement opaque Intent Unit identifiers
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:34:00Z
- **Files modified:** `crates/cubikan-core/src/id.rs`, `crates/cubikan-core/src/lib.rs`
- **Commit:** `047b571b4459b981b6b604a094296bf131dc4753`

## T-004 (sprint 0)
- **Description:** Implement validated textual domain values
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:35:14Z
- **Files modified:** `crates/cubikan-core/src/vocabulary.rs`, `crates/cubikan-core/src/lib.rs`
- **Commit:** `d96d5e6afae6be9a073e05b146cd14a4971e5741`

## T-005 (sprint 0)
- **Description:** Implement caller-declared directed workflow definitions
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:37:26Z
- **Files modified:** `crates/cubikan-core/src/workflow.rs`, `crates/cubikan-core/src/lib.rs`
- **Commit:** `389b9e8863748fb2ed258fcb13604d42f0136940`

## T-006 (sprint 0)
- **Description:** Implement active Intent Unit construction
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:39:02Z
- **Files modified:** `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/src/lib.rs`
- **Commit:** `51d7af0c6f9156f156265170efe91d961c8815cb`

## T-007 (sprint 0)
- **Description:** Implement guarded phase transitions and append-only records
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:40:48Z
- **Files modified:** `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/src/lib.rs`
- **Commit:** `74883da8f7b149a2c7963d543eb55256cc92bc7a`

## T-008 (sprint 0)
- **Description:** Implement terminal completion
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:42:14Z
- **Files modified:** `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/src/lib.rs`
- **Commit:** `f94c4689e125bf64e190e73f0a4743f21f7c324c`

## T-009 (sprint 0)
- **Description:** Add validated format-neutral serialization for scalars and workflows
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:44:28Z
- **Files modified:** `crates/cubikan-core/src/id.rs`, `crates/cubikan-core/src/vocabulary.rs`, `crates/cubikan-core/src/workflow.rs`
- **Commit:** `308761f1a8058828a8f532007fe20de5acae61e0`

## T-010 (sprint 0)
- **Description:** Add validated format-neutral serialization for Intent Units
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:46:35Z
- **Files modified:** `crates/cubikan-core/src/intent_unit.rs`
- **Commit:** `61cfa193d5196ef612da75e2134c39ccba48f8f2`

## T-011 (sprint 0)
- **Description:** Document the core vocabulary and Sprint 0 boundaries
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:48:05Z
- **Files modified:** `README.md`
- **Commit:** `e918ab07b987a0e358ce66e3a5e298280ba5c539`

## T-012 (sprint 0)
- **Description:** Add an executable public lifecycle example
- **Intent:** [INT-0001](../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- **Completed:** 2026-08-08T13:49:36Z
- **Files modified:** `crates/cubikan-core/src/lib.rs`
- **Commit:** `3f79615c3c8cc1d740f4642455f7fffb2112755e`

## T-101 (sprint 1)
- **Description:** Scaffold the cubikan-cli workspace package
- **Intent:** [INT-0002](../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Completed:** 2026-08-08T16:23:49Z
- **Files modified:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-cli/Cargo.toml`, `crates/cubikan-cli/src/lib.rs`, `docs/SUMMARY.md`, `docs/intents/INT-0002-runnable-lifecycle-adapter.md`, `docs/sprints/s1/sprint-meta.md`, `docs/sprints/s1/sprint-research/research-report.md`, `docs/sprints/s1/sprint-plans/build-plan.md`, `docs/sprints/s1/sprint-plans/test-plan.md`, `docs/sprints/s1/sprint-plans/critique.md`, `docs/sprints/s1/sprint-tests/unit-tests.md`, `docs/sprints/s1/sprint-tests/integration-tests.md`, `docs/sprints/s1/sprint-tests/e2e-tests.md`, `docs/sprints/s1/sprint-tests/test-report.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `2a050101b29b03f8aca5d22acc2b45058270ce4b`

## T-102 (sprint 1)
- **Description:** Define the adapter-owned version 1 JSON contract
- **Intent:** [INT-0002](../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Completed:** 2026-08-08T16:33:36Z
- **Files modified:** `crates/cubikan-cli/src/lib.rs`, `crates/cubikan-cli/src/protocol.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `d6237e4a2dc64d4ac47fab248b6a0f0f13f1a735`

## T-103 (sprint 1)
- **Description:** Construct validated core scenarios and map setup failures
- **Intent:** [INT-0002](../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Completed:** 2026-08-08T16:38:13Z
- **Files modified:** `crates/cubikan-cli/src/lib.rs`, `crates/cubikan-cli/src/execution.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `8122bf48bd50f70472a5f91c7502b05568fee6c9`

## T-104 (sprint 1)
- **Description:** Execute ordered lifecycle operations and expose adapter snapshots
- **Intent:** [INT-0002](../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Completed:** 2026-08-08T16:42:00Z
- **Files modified:** `crates/cubikan-cli/src/execution.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `bd599c05328c664f96112971a38e7077a9b0a44f`

## T-105 (sprint 1)
- **Description:** Implement the generic JSON stream runner
- **Intent:** [INT-0002](../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Completed:** 2026-08-08T16:44:47Z
- **Files modified:** `crates/cubikan-cli/src/lib.rs`, `crates/cubikan-cli/src/protocol.rs`, `crates/cubikan-cli/src/execution.rs`, `crates/cubikan-cli/src/runner.rs`, `crates/cubikan-cli/tests/runner.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `5819ef135d00f06dbe812220cd666a43f985619e`

## T-106 (sprint 1)
- **Description:** Expose and process-test the cubikan executable
- **Intent:** [INT-0002](../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Completed:** 2026-08-08T16:46:38Z
- **Files modified:** `crates/cubikan-cli/Cargo.toml`, `crates/cubikan-cli/src/lib.rs`, `crates/cubikan-cli/src/main.rs`, `crates/cubikan-cli/tests/cli_e2e.rs`, `crates/cubikan-cli/tests/fixtures/lifecycle-success-v1.json`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `37896a330c95548e6c2a1bf163e709e7cd467584`

## T-107 (sprint 1)
- **Description:** Document the CLI contract and Sprint 1 boundaries
- **Intent:** [INT-0002](../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Completed:** 2026-08-08T16:48:35Z
- **Files modified:** `README.md`, `crates/cubikan-cli/README.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `ae876734fd9bbd43b0fc3278187057c91bca65c2`

## T-201 (sprint 2)
- **Description:** Define the request ceiling and typed oversize response contract
- **Intent:** [INT-0003](../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Completed:** 2026-08-08T18:47:33Z
- **Files modified:** `crates/cubikan-cli/src/lib.rs`, `crates/cubikan-cli/src/protocol.rs`, `docs/SUMMARY.md`, `docs/intents/INT-0002-runnable-lifecycle-adapter.md`, `docs/intents/INT-0003-bounded-cli-request-ingestion.md`, `docs/sprints/s2/sprint-meta.md`, `docs/sprints/s2/sprint-research/research-report.md`, `docs/sprints/s2/sprint-plans/build-plan.md`, `docs/sprints/s2/sprint-plans/test-plan.md`, `docs/sprints/s2/sprint-plans/critique.md`, `docs/sprints/s2/sprint-tests/unit-tests.md`, `docs/sprints/s2/sprint-tests/integration-tests.md`, `docs/sprints/s2/sprint-tests/e2e-tests.md`, `docs/sprints/s2/sprint-tests/test-report.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `3494d8d8173095ef61504bafe1d5847af159faba`

## T-202 (sprint 2)
- **Description:** Implement ceiling-plus-one request ingestion before JSON decoding
- **Intent:** [INT-0003](../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Completed:** 2026-08-08T18:50:15Z
- **Files modified:** `crates/cubikan-cli/src/protocol.rs`, `crates/cubikan-cli/src/runner.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `4ff64088f17f2dfbcb35ce2175a8a44cc3893e5f`

## T-203 (sprint 2)
- **Description:** Add public-seam and actual-process boundary coverage
- **Intent:** [INT-0003](../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Completed:** 2026-08-08T18:52:25Z
- **Files modified:** `crates/cubikan-cli/tests/runner.rs`, `crates/cubikan-cli/tests/cli_e2e.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** PENDING
