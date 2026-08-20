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
- **Commit:** `809b4b9e828523875febd511ae048373daaa1262`

## T-204 (sprint 2)
- **Description:** Document the bounded local ingestion contract
- **Intent:** [INT-0003](../intents/INT-0003-bounded-cli-request-ingestion.md)
- **Completed:** 2026-08-08T18:53:46Z
- **Files modified:** `README.md`, `crates/cubikan-cli/README.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `5e4f43db0675be7be6aa33cbf540031ccabe363d`

## T-301 (sprint 3)
- **Description:** Implement the typed supplied-writer flush contract
- **Intent:** [INT-0004](../intents/INT-0004-explicit-cli-response-flush.md)
- **Completed:** 2026-08-08T20:08:20Z
- **Files modified:** `crates/cubikan-cli/src/runner.rs`, `docs/SUMMARY.md`, `docs/intents/INT-0004-explicit-cli-response-flush.md`, `docs/sprints/s3/sprint-meta.md`, `docs/sprints/s3/sprint-research/research-report.md`, `docs/sprints/s3/sprint-plans/build-plan.md`, `docs/sprints/s3/sprint-plans/test-plan.md`, `docs/sprints/s3/sprint-plans/critique.md`, `docs/sprints/s3/sprint-tests/unit-tests.md`, `docs/sprints/s3/sprint-tests/integration-tests.md`, `docs/sprints/s3/sprint-tests/e2e-tests.md`, `docs/sprints/s3/sprint-tests/test-report.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `17f57cad08447f3931ae3e6e196bc3da428dc90c`

## T-302 (sprint 3)
- **Description:** Prove the public buffered-writer and process-shell boundaries
- **Intent:** [INT-0004](../intents/INT-0004-explicit-cli-response-flush.md)
- **Completed:** 2026-08-08T20:14:17Z
- **Files modified:** `crates/cubikan-cli/tests/runner.rs`, `crates/cubikan-cli/tests/cli_e2e.rs`, `crates/cubikan-cli/src/lib.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `064b18bb623db9e3843f954202a7cebd69aabe6b`

## T-303 (sprint 3)
- **Description:** Document the writer-flush-checked response boundary
- **Intent:** [INT-0004](../intents/INT-0004-explicit-cli-response-flush.md)
- **Completed:** 2026-08-08T20:17:13Z
- **Files modified:** `README.md`, `crates/cubikan-cli/README.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `e62d29c3b4db4e6eb9617f5b0d6d0fec80704a78`

## T-401 (sprint 4)
- **Description:** Establish the bounded Rust CI workflow shell
- **Intent:** [INT-0005](../intents/INT-0005-automated-rust-quality-gate.md)
- **Completed:** 2026-08-08T22:07:45Z
- **Files modified:** `.github/workflows/ci.yml`, `docs/SUMMARY.md`, `docs/intents/INT-0005-automated-rust-quality-gate.md`, `docs/sprints/s4/sprint-meta.md`, `docs/sprints/s4/sprint-research/research-report.md`, `docs/sprints/s4/sprint-plans/build-plan.md`, `docs/sprints/s4/sprint-plans/test-plan.md`, `docs/sprints/s4/sprint-plans/critique.md`, `docs/sprints/s4/sprint-tests/unit-tests.md`, `docs/sprints/s4/sprint-tests/integration-tests.md`, `docs/sprints/s4/sprint-tests/e2e-tests.md`, `docs/sprints/s4/sprint-tests/test-report.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `567e3d5f496cb9bd27830052c4fecbd56d06d36f`

## T-402 (sprint 4)
- **Description:** Add the canonical Rust quality gates and hosted proof boundary
- **Intent:** [INT-0005](../intents/INT-0005-automated-rust-quality-gate.md)
- **Completed:** 2026-08-08T22:11:25Z
- **Files modified:** `.github/workflows/ci.yml`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `f70ee3d34f633023a633aad6e7377108cebf571d`

## T-403 (sprint 4)
- **Description:** Document the automated quality boundary
- **Intent:** [INT-0005](../intents/INT-0005-automated-rust-quality-gate.md)
- **Completed:** 2026-08-08T22:14:16Z
- **Files modified:** `README.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `c4489cd35bfdce36600925918d73c215b0b2a891`

## T-501 (sprint 5)
- **Description:** Distinguish absent and present ID values in the version 1 decoder
- **Intent:** [INT-0006](../intents/INT-0006-distinguish-omitted-cli-id.md)
- **Completed:** 2026-08-08T23:42:50Z
- **Files modified:** `crates/cubikan-cli/src/protocol.rs`, `crates/cubikan-cli/src/lib.rs`, `docs/SUMMARY.md`, `docs/intents/INT-0006-distinguish-omitted-cli-id.md`, `docs/sprints/s5/sprint-meta.md`, `docs/sprints/s5/sprint-research/research-report.md`, `docs/sprints/s5/sprint-plans/build-plan.md`, `docs/sprints/s5/sprint-plans/test-plan.md`, `docs/sprints/s5/sprint-plans/critique.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `1a689edf525b02e05f44eb5027d6ff42d698fb0d`
- **Evidence clarification:** The task commit also created the initialized zero-byte placeholders `docs/sprints/s5/sprint-tests/unit-tests.md`, `docs/sprints/s5/sprint-tests/integration-tests.md`, `docs/sprints/s5/sprint-tests/e2e-tests.md`, and `docs/sprints/s5/sprint-tests/test-report.md`.

## T-502 (sprint 5)
- **Description:** Prove the public runner identity boundary
- **Intent:** [INT-0006](../intents/INT-0006-distinguish-omitted-cli-id.md)
- **Completed:** 2026-08-08T23:48:26Z
- **Files modified:** `crates/cubikan-cli/tests/runner.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `e3cd97727752c05cf2c02702ff25bb8da3dbae9a`

## T-503 (sprint 5)
- **Description:** Prove the actual-process identity boundary
- **Intent:** [INT-0006](../intents/INT-0006-distinguish-omitted-cli-id.md)
- **Completed:** 2026-08-08T23:51:26Z
- **Files modified:** `crates/cubikan-cli/tests/cli_e2e.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `3eadc3ac44d73c4aa6b67582dbbea6b6f33b629d`

## T-504 (sprint 5)
- **Description:** Document the ID-presence contract and preserve scope
- **Intent:** [INT-0006](../intents/INT-0006-distinguish-omitted-cli-id.md)
- **Completed:** 2026-08-08T23:53:36Z
- **Files modified:** `crates/cubikan-cli/README.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `4ce4ff88a8dfb05135ae2b088e900a5e49201a88`

## T-601 (sprint 6)
- **Description:** Establish appendix authority and CubiKan integration baseline
- **Intent:** [INT-0007](../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- **Completed:** 2026-08-09T03:39:13Z
- **Files modified:** `docs/appendix/README.md`, `docs/appendix/potential-derivative-projects.md`, `docs/SUMMARY.md`, `docs/intents/INT-0007-define-cubikan-derivative-ecosystem.md`, `docs/intents/INT-0008-traceable-intent-instantiation.md`, `docs/intents/INT-0009-revisioned-lifecycle-commands.md`, `docs/intents/INT-0010-durable-intent-unit-backend.md`, `docs/intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md`, `docs/intents/INT-0012-intent-unit-relationships-and-board-projections.md`, `docs/sprints/s6/sprint-meta.md`, `docs/sprints/s6/sprint-research/research-report.md`, `docs/sprints/s6/sprint-plans/build-plan.md`, `docs/sprints/s6/sprint-plans/test-plan.md`, `docs/sprints/s6/sprint-plans/critique.md`, `docs/sprints/s6/sprint-tests/unit-tests.md`, `docs/sprints/s6/sprint-tests/integration-tests.md`, `docs/sprints/s6/sprint-tests/e2e-tests.md`, `docs/sprints/s6/sprint-tests/test-report.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `5cc52aba625acc9e0361014eca8aec0edbe55554`

## T-602 (sprint 6)
- **Description:** Document Agent Ops, Observatory, and Animus Ledger
- **Intent:** [INT-0007](../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- **Completed:** 2026-08-09T03:49:20Z
- **Files modified:** `docs/appendix/potential-derivative-projects.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `f1770f774bfafed538316f01c3cd05cd82270855`

## T-603 (sprint 6)
- **Description:** Complete the process, skill-graph, and organizational catalog
- **Intent:** [INT-0007](../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- **Completed:** 2026-08-09T04:05:24Z
- **Files modified:** `docs/appendix/potential-derivative-projects.md`, `docs/appendix/README.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `f38e974a903f9e4a0cac8a63778c0426877571b5`
- **Evidence clarification:** `docs/appendix/README.md` changed only to remove a terminal blank line found by the locked whole-sprint `git diff --check` gate; its appendix-navigation content is unchanged.

## T-701 (sprint 7)
- **Description:** Add explicit lifecycle revision state and advance existing mutations
- **Intent:** [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Completed:** 2026-08-09T06:40:40Z
- **Files modified:** `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/src/lib.rs`, `crates/cubikan-core/tests/lifecycle.rs`, `docs/SUMMARY.md`, `docs/intents/INT-0009-revisioned-lifecycle-commands.md`, `docs/sprints/s7/sprint-meta.md`, `docs/sprints/s7/sprint-research/research-report.md`, `docs/sprints/s7/sprint-plans/build-plan.md`, `docs/sprints/s7/sprint-plans/test-plan.md`, `docs/sprints/s7/sprint-plans/critique.md`, `docs/sprints/s7/sprint-tests/unit-tests.md`, `docs/sprints/s7/sprint-tests/integration-tests.md`, `docs/sprints/s7/sprint-tests/e2e-tests.md`, `docs/sprints/s7/sprint-tests/test-report.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `380e2285ee1a25b37b12a09612dce32784a30319`

## T-702 (sprint 7)
- **Description:** Add stale-first revision-conditioned lifecycle commands
- **Intent:** [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Completed:** 2026-08-09T06:49:09Z
- **Files modified:** `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/src/lib.rs`, `crates/cubikan-core/tests/lifecycle.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `536b83ee3a2c58781a937c7e876e85a5c315a0a4`

## T-703 (sprint 7)
- **Description:** Persist and validate lifecycle revisions during restoration
- **Intent:** [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Completed:** 2026-08-09T06:55:38Z
- **Files modified:** `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/tests/serialization.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `6f894980523856ec5b06d2aa5577b6e74733cef5`

## T-704 (sprint 7)
- **Description:** Document optimistic lifecycle revisions and scope boundaries
- **Intent:** [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Completed:** 2026-08-09T07:04:13Z
- **Files modified:** `crates/cubikan-core/src/lib.rs`, `README.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `8124ffa76286b7df7ab30af3a1c0d924c9e32c64`

## T-801 (sprint 8)
- **Description:** Scaffold cubikan-backend and define its public value contract
- **Intent:** [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md)
- **Completed:** 2026-08-09T21:01:13Z
- **Files modified:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-backend/Cargo.toml`, `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/model.rs`, `crates/cubikan-backend/src/error.rs`, `crates/cubikan-backend/tests/model.rs`, `docs/SUMMARY.md`, `docs/intents/INT-0010-durable-intent-unit-backend.md`, `docs/sprints/s8/sprint-meta.md`, `docs/sprints/s8/sprint-research/research-report.md`, `docs/sprints/s8/sprint-plans/build-plan.md`, `docs/sprints/s8/sprint-plans/test-plan.md`, `docs/sprints/s8/sprint-plans/critique.md`, `docs/sprints/s8/sprint-tests/unit-tests.md`, `docs/sprints/s8/sprint-tests/integration-tests.md`, `docs/sprints/s8/sprint-tests/e2e-tests.md`, `docs/sprints/s8/sprint-tests/test-report.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `1934c5b6040e540cf40a82d0a3be8281283e2cf0`

## T-802 (sprint 8)
- **Description:** Implement the strict replay-validated storage envelope
- **Intent:** [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md)
- **Completed:** 2026-08-09T21:11:29Z
- **Files modified:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/stored.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `1550ccd2d9919b1e8b0d256bd62852bdbfc4fefe`

## T-803 (sprint 8)
- **Description:** Own, initialize, and validate SQLite schema v1
- **Intent:** [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md)
- **Completed:** 2026-08-09T21:41:15Z
- **Files modified:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-backend/Cargo.toml`, `crates/cubikan-backend/src/error.rs`, `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/schema.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/model.rs`, `crates/cubikan-backend/tests/schema.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `00ab50eed285aea5795de6a7a59b48444262481c`

## T-804 (sprint 8)
- **Description:** Add transactional durable create and replay-validated get
- **Intent:** [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md)
- **Completed:** 2026-08-09T21:55:32Z
- **Files modified:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/model.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/corruption.rs`, `crates/cubikan-backend/tests/persistence.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `38924cad7849d63fcfac4d8170d5b114da3ab5ec`

## T-805 (sprint 8)
- **Description:** Add bounded exact-filter live keyset pagination
- **Intent:** [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md)
- **Completed:** 2026-08-09T22:14:51Z
- **Files modified:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/model.rs`, `crates/cubikan-backend/src/query.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/query.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `ecb434b0dd2ec26dc3d877d9608a8ae49fe1857b`

## T-806 (sprint 8)
- **Description:** Add revision-guarded transition and completion transactions
- **Intent:** [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md), preserving [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Completed:** 2026-08-09T22:27:01Z
- **Files modified:** `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/tests/mutations.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `e4b7510659f502050598ec9b0b2b9aa5a92e673a`

## T-807 (sprint 8)
- **Description:** Define local protocol v1 and execute every backend command
- **Intent:** [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md)
- **Completed:** 2026-08-09T22:54:19Z
- **Files modified:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-local/Cargo.toml`, `crates/cubikan-local/src/lib.rs`, `crates/cubikan-local/src/protocol.rs`, `crates/cubikan-local/src/execution.rs`, `crates/cubikan-local/tests/protocol.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `7c56ce645303b554033d47ad9e23dd53ba7bbbe6`

## T-808 (sprint 8)
- **Description:** Add the bounded runner and cubikan-local executable
- **Intent:** [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md)
- **Completed:** 2026-08-09T23:12:26Z
- **Files modified:** `crates/cubikan-local/src/lib.rs`, `crates/cubikan-local/src/main.rs`, `crates/cubikan-local/src/runner.rs`, `crates/cubikan-local/tests/runner.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `9f8050f6347364c03a6b0c2c1bfa70b904a11e6e`

## T-809 (sprint 8)
- **Description:** Prove cross-process continuity and fail-closed process behavior
- **Intent:** [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md), preserving [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Completed:** 2026-08-09T23:20:44Z
- **Files modified:** `crates/cubikan-local/tests/cli_e2e.rs`, `crates/cubikan-local/tests/fixtures/durable-lifecycle-v1.json`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `6e19edd75820fd8d62338f667217cf19f929f787`

## T-810 (sprint 8)
- **Description:** Document the first backend boundary and nonclaims
- **Intent:** [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md)
- **Completed:** 2026-08-09T23:33:51Z
- **Files modified:** `crates/cubikan-backend/README.md`, `crates/cubikan-local/README.md`, `README.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `315fb711c6f84b06cdd2b363b682a7f1b394bbaf`

## T-901 (sprint 9)
- **Description:** Add the public relationship and projection value contract
- **Intent:** [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Completed:** 2026-08-10T02:58:02Z
- **Files modified:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/relationship.rs`, `crates/cubikan-backend/src/projection.rs`, `crates/cubikan-backend/tests/relationship_model.rs`, `docs/SUMMARY.md`, `docs/intents/INT-0012-intent-unit-relationships-and-board-projections.md`, `docs/sprints/s9/sprint-meta.md`, `docs/sprints/s9/sprint-research/research-report.md`, `docs/sprints/s9/sprint-plans/build-plan.md`, `docs/sprints/s9/sprint-plans/test-plan.md`, `docs/sprints/s9/sprint-plans/critique.md`, `docs/sprints/s9/sprint-tests/unit-tests.md`, `docs/sprints/s9/sprint-tests/integration-tests.md`, `docs/sprints/s9/sprint-tests/e2e-tests.md`, `docs/sprints/s9/sprint-tests/test-report.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `3a7872465f43044389df617dce6b47244689dcc0`

## T-902 (sprint 9)
- **Description:** Introduce exact schema v2 and explicit atomic v1-to-v2 migration
- **Intent:** [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Completed:** 2026-08-10T03:32:34Z
- **Files modified:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/error.rs`, `crates/cubikan-backend/src/schema.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/migration.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/schema.rs`, `crates/cubikan-backend/tests/migration.rs`, `crates/cubikan-local/tests/cli_e2e.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `d7a4d44d662691b713ed9f9e107a876d14e1af2f`

## T-903 (sprint 9)
- **Description:** Persist immutable relationship definitions
- **Intent:** [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Completed:** 2026-08-10T03:42:06Z
- **Files modified:** `crates/cubikan-backend/src/lib.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/relationship_store.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/relationship_definitions.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `259431d3ecf87c12d22eca33b9c7fd31620f6f51`

## T-904 (sprint 9)
- **Description:** Create and delete validated directed relationships atomically
- **Intent:** [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Completed:** 2026-08-10T03:56:51Z
- **Files modified:** `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/relationship_store.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/relationship_mutations.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `7494f447e020d3cb59b803ac3a8a555d2624c1cf`

## T-905 (sprint 9)
- **Description:** Add bounded direct relationship queries
- **Intent:** [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Completed:** 2026-08-10T04:09:04Z
- **Files modified:** `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/relationship_store.rs`, `crates/cubikan-backend/tests/common/mod.rs`, `crates/cubikan-backend/tests/relationship_query.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `a08c269e534aade7c876f9adc018a563ca9179db`

## T-906 (sprint 9)
- **Description:** Add ephemeral board-projection query version 1
- **Intent:** [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Completed:** 2026-08-10T04:25:46Z
- **Files modified:** `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/query.rs`, `crates/cubikan-backend/src/relationship_store.rs`, `crates/cubikan-backend/tests/projection_query.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `83fb93e8f26cb0a560ea96cd5ac46d2e7d57b80a`

## T-907 (sprint 9)
- **Description:** Document relationship, migration, projection, and nonclaim boundaries
- **Intent:** [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Completed:** 2026-08-10T04:43:25Z
- **Files modified:** `README.md`, `crates/cubikan-backend/README.md`, `crates/cubikan-local/README.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `c8b3a1af3bca2a8a67293e40fc7943f9dafa13f5`

## T-908 (sprint 9)
- **Description:** Prove the public-backend relationship and projection vertical across reopen
- **Intent:** [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Completed:** 2026-08-10T05:05:06Z
- **Files modified:** `crates/cubikan-backend/tests/relationship_e2e.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `751aee2af457d17d01bfd07c343ea37a6b42d2f9`

## MAINT-001 (post-Sprint 9)
- **Description:** Refresh the advisory derivative-project appendix for the realized local backend and relationship/projection boundary
- **Intent:** [INT-0007](../intents/INT-0007-define-cubikan-derivative-ecosystem.md) (originating realized backlog authority); Book reconciliation is governed by superseding [INT-0013](../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Completed:** 2026-08-10T13:51:14Z
- **Files modified:** `docs/appendix/potential-derivative-projects.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Scope:** Originating maintenance corrected current-state and prerequisite language without changing runtime behavior or derivative-repository authorization; Sprint 10 reconciliation records INT-0013 as its superseding current-state authority.
- **Commit:** `a7ed48992897c8463ba6cc729e944398c8ae8779`

## T-1001 (sprint 10)
- **Description:** Correct the appendix's current CubiKan surfaces and version boundary
- **Intent:** [INT-0013](../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Completed:** 2026-08-10T14:50:00Z
- **Files modified:** `docs/intents/INT-0013-maintain-derivative-ecosystem-current-state.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Integrated implementation commit:** `d725411e0bf4c97437544e28c604e48f0c1badbf`
- **Commit:** `b6ba646e88093e8f88eedd31b04405b44a031a82`

## T-1002 (sprint 10)
- **Description:** Correct the appendix's capability status, dependency, and authority maps
- **Intent:** [INT-0013](../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Completed:** 2026-08-11T19:08:11Z
- **Files modified:** `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Integrated implementation commit:** `a4c14cfcaccc23afeebafe28490b63b0683d17e8`
- **Commit:** `1a7c1b210f24170f4f07c4dbd700e8bf58c320d5`

## T-1003 (sprint 10)
- **Description:** Correct the safe integration boundary while preserving current exclusions
- **Intent:** [INT-0013](../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Completed:** 2026-08-11T19:09:13Z
- **Files modified:** `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Integrated implementation commit:** `a3e6aec3afe739091d03103744a82d89ad1c467b`
- **Commit:** `c923c0b8dae3ac56a40a7d738a192d5359429ea4`

## T-1004 (sprint 10)
- **Description:** Preserve the complete catalog while replacing stale waits on realized primitives
- **Intent:** [INT-0013](../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Completed:** 2026-08-11T19:10:36Z
- **Files modified:** `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Integrated implementation commit:** `336b4e48e791f9a7d0a25e5de84c9404c3e266d2`
- **Commit:** `b797cc832363639ba6343ee274cec321b65240e2`

## T-1005 (sprint 10)
- **Description:** Preserve conditional creation governance for every recommended repository
- **Intent:** [INT-0013](../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Completed:** 2026-08-11T19:11:54Z
- **Files modified:** `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Integrated implementation commit:** `99864da63fc9a51b24ead1d5792c4d6b7f706207`
- **Commit:** `168f9b1843d031ee1430bbbf689a4ecd48bf1db5`

## T-1006 (sprint 10)
- **Description:** Correct global sequencing, open questions, and status non-goals
- **Intent:** [INT-0013](../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Completed:** 2026-08-11T19:13:20Z
- **Files modified:** `docs/appendix/potential-derivative-projects.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Integrated implementation commit:** `9517dc17797f25e7a2d8f924abf1b5d51fb62e5a`
- **Commit:** `aa98b41c8d7ce96ad94e281bb9dbc323ec834868`

## T-1007 (sprint 10)
- **Description:** Close the derivative-appendix refresh backlog item
- **Intent:** [INT-0013](../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Completed:** 2026-08-11T19:14:13Z
- **Files modified:** `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Integrated implementation commit:** `a7ed48992897c8463ba6cc729e944398c8ae8779`
- **Commit:** `ec2dac4bda1a1e615bdb0bc0d99b54f6dbbcaacb`

## T-1101 (sprint 11)
- **Description:** Pin and isolate the Polkadot SDK development toolchain
- **Intent:** [INT-0014](../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Completed:** 2026-08-13T06:38:49Z
- **Files modified:** `.gitignore`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `patches/rusqlite-0.40.2-commit-authorizer.patch`, `vendor/rusqlite-0.40.2-cubikan/**`, `chain/Cargo.toml`, `chain/Cargo.lock`, `chain/rust-toolchain.toml`, `chain/README.md`, `chain/pins.toml`, `chain/pallets/cubikan/**`, `chain/runtime/**`, `chain/tools/**`, `docs/intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md`, `docs/sprints/s11/sprint-meta.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `700ca5377b7447f312d1c6938c9972796fd6c19c`

## T-1102 (sprint 11)
- **Description:** Define bounded SCALE values and independent conformance fixtures
- **Intent:** [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md), [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md), [INT-0014](../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Completed:** 2026-08-13T14:11:28Z
- **Files modified:** `chain/pallets/cubikan/src/conformance.rs`, `chain/pallets/cubikan/src/tests/model.rs`, `chain/pallets/cubikan/src/types.rs`, `crates/cubikan-core/src/external_reference.rs`, `crates/cubikan-core/src/lib.rs`, `crates/cubikan-core/src/vocabulary.rs`, `crates/cubikan-core/src/workflow.rs`, `crates/cubikan-core/tests/bounded_conformance.rs`, `tests/fixtures/chain-conformance-v1.json`, `docs/intents/INT-0008-traceable-intent-instantiation.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Commit:** `94bbbd54f15a6b95b9ff331cee8fdcff9098f2d3`

## T-1103 (sprint 11)
- **Description:** Implement canonical lifecycle events and pallet mutations
- **Intent:** [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md), [INT-0014](../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Completed:** 2026-08-13T16:35:19Z
- **Files modified:** `chain/Cargo.lock`, `chain/Cargo.toml`, `chain/pallets/cubikan/Cargo.toml`, `chain/pallets/cubikan/src/lib.rs`, `chain/pallets/cubikan/src/event.rs`, `chain/pallets/cubikan/src/error.rs`, `chain/pallets/cubikan/src/benchmarking.rs`, `chain/pallets/cubikan/src/weights.rs`, `chain/pallets/cubikan/src/mock.rs`, `chain/pallets/cubikan/src/tests/lifecycle.rs`, `chain/pins.toml`, `chain/tools/verify-pins.sh`, `docs/sprints/s11/sprint-meta.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Scope repair:** The locked task assigned executable FRAME benchmarks but omitted the direct optional `frame-benchmarking` manifest/lock/pin paths; Sprint 11 meta records the minimal same-revision dependency repair.
- **Commit:** `33322344bb0f587ee94a2b05002ee0da78302198`

## T-1104 (sprint 11)
- **Description:** Port bounded relationship definitions and edges to the pallet
- **Intent:** [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md), [INT-0014](../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Completed:** 2026-08-13T20:14:47Z
- **Files modified:** `chain/pallets/cubikan/src/relationship.rs`, `chain/pallets/cubikan/src/lib.rs`, `chain/pallets/cubikan/src/event.rs`, `chain/pallets/cubikan/src/error.rs`, `chain/pallets/cubikan/src/benchmarking.rs`, `chain/pallets/cubikan/src/weights.rs`, `chain/pallets/cubikan/src/tests/relationships.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Fixture note:** The independently authored T-1102 conformance corpus already carries the bounded relationship value and 128/129 capacity cases; T-1104 added no implementation-derived fixture bytes.
- **Commit:** `96fb0749e6b01fcc3ca80a157f3ef653b797f884`

## T-1105 (sprint 11)
- **Description:** Implement canonical provenance record and revocation
- **Intent:** [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md), [INT-0014](../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Completed:** 2026-08-13T21:14:16Z
- **Files modified:** `chain/pallets/cubikan/src/provenance.rs`, `chain/pallets/cubikan/src/lib.rs`, `chain/pallets/cubikan/src/event.rs`, `chain/pallets/cubikan/src/error.rs`, `chain/pallets/cubikan/src/benchmarking.rs`, `chain/pallets/cubikan/src/weights.rs`, `chain/pallets/cubikan/src/tests/provenance.rs`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Fixture note:** The independently authored T-1102 conformance corpus already covers association identity, subjects, capacity, authorization, and structural SCALE/reference rejection; T-1105 added no implementation-derived fixture bytes.
- **Commit:** `9ce2fbeee4839f624d0d9c3f1ce2ea2b9f0a6167`

## T-1106 (sprint 11)
- **Description:** Integrate the fixed local parachain runtime and artifact contract
- **Intent:** [INT-0014](../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Completed:** 2026-08-14T05:01:30Z
- **Files modified:** `chain/Cargo.toml`, `chain/Cargo.lock`, `chain/pallets/cubikan/src/benchmarking.rs`, `chain/runtime/Cargo.toml`, `chain/runtime/src/lib.rs`, `chain/runtime/src/apis.rs`, `chain/runtime/src/benchmarks.rs`, `chain/runtime/src/configs.rs`, `chain/runtime/src/genesis_config_presets.rs`, `chain/runtime/src/weights/mod.rs`, `chain/runtime/src/weights/pallet_cubikan.rs`, `chain/config/cubikan-local.json`, `chain/metadata/cubikan-runtime-v1.scale`, `chain/artifacts/local-deployment-anchor-v1.json`, `chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm`, `chain/artifacts/benchmarks/cubikan-runtime-v1.runtime-benchmarks.compact.compressed.wasm`, `chain/artifacts/benchmarks/cubikan-pallet-v1.json`, `chain/pins.toml`, `chain/tools/verify-pins.sh`, `chain/tools/verify-runtime-artifacts.sh`, `chain/tools/verify-runtime-artifacts.py`, `chain/tools/verify-weights.sh`, `chain/tools/verify-weights.py`, `chain/tests/runtime.rs`, `chain/tests/weights.rs`, `docs/sprints/s11/sprint-meta.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Scope repair:** The locked Touches omitted `chain/Cargo.toml`, `chain/Cargo.lock`, and the maximum-fixture source `chain/pallets/cubikan/src/benchmarking.rs`; Sprint 11 meta records the minimal same-revision dependency, lock, and benchmark-source repair.
- **Artifact evidence:** deployable Wasm SHA-256 `640cc616674fe7393fc93928904f0fd92d77571209c8200f08b8da6290c6a275`; chain-spec SHA-256 `dc7945fbeed5b18d21c1839f8f4f5ab13a1660ca956a3513b8a9946bab6334c7`; metadata SHA-256 `171a323b1e6bf0122e549eecd5f5932e672a3e0835f32edf0b8808cfefd97302`; resolved anchor SHA-256 `38f795fb3bbb666f571b3bd1e4fa3ad1666476f3fff20dee9d93feb9c925dee7`; generated weights SHA-256 `5300fec791e7d352be42abdfbf8a7168beafa736bc7f665883ab03e1eac3e1f8`; relay genesis `0xeb2ada687ce553d3b9d695afd5d9d0a9c44a0b82e1f6eb823ac87e81638200f0`; parachain genesis `0x627f53b3abc01130ec273ef85759f90779e8497614a428a66d862a624ee01a17`.
- **Commit:** `5b3cf5548ce6670b70ee9289283abc369912f338`

## T-1107 (sprint 11)
- **Description:** Rebaseline the chain-neutral core around required origin
- **Intent:** [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md), [INT-0014](../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Completed:** 2026-08-14T05:25:40Z
- **Files modified:** `crates/cubikan-core/src/lib.rs`, `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/tests/**`, `crates/cubikan-backend/src/model.rs`, `crates/cubikan-backend/src/stored.rs`, `crates/cubikan-backend/src/sqlite.rs`, `crates/cubikan-backend/src/migration.rs`, `crates/cubikan-backend/tests/**`, `crates/cubikan-cli/src/**`, `crates/cubikan-cli/tests/**`, `crates/cubikan-local/src/**`, `crates/cubikan-local/tests/**`, `docs/sprints/s11/sprint-meta.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Scope repair:** The locked Touches omitted `crates/cubikan-backend/src/migration.rs`; Sprint 11 meta records the minimal fail-closed removal of the otherwise-successful originless v1-to-v2 migration.
- **Verification:** Root workspace all-target tests and doctests, warnings-denied Clippy, rustfmt, the Book-v2 validator, immutable fixture hashes, and an independent E2 authority audit passed.
- **Commit:** `80c7a1f984534eba62d38b69e57abdc1ac373c75`

## T-1112 (sprint 11)
- **Description:** Replace `cubikan` with strict stateless protocol v2
- **Intent:** [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Completed:** 2026-08-14T08:06:25Z
- **Files modified:** `crates/cubikan-cli/**`, `protocol/v2/cubikan.schema.json`, `protocol/v2/verify-fixtures.sh`, `tests/fixtures/protocol-v2/cubikan/**`, `crates/cubikan-local/tests/protocol.rs`, `docs/sprints/s11/sprint-meta.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Scope repair:** T-1107's cross-consumer regression test prohibited the in-memory core construction T-1112 explicitly assigns to `cubikan`; Sprint 11 meta records the minimal test-only exemption while preserving every database, RPC, signing, durable-write, synthetic-origin, and `cubikan-local` prohibition.
- **Oracle evidence:** Independently authored schema SHA-256 `309697fe6e718c78ef8802861d60a660500a985c05b5a94aaba35a28fb2cb4a3`, 96-case manifest SHA-256 `46eab998ec22d8c806c7f8ac347aa89efb4f69578c7a34f6ee4737fc24e97c75`, and four-case I/O oracle SHA-256 `32ed09ae7ec55005229e1a7fa1b5edc02ae1867c55bbbd6279b88048b0dd14f4` passed the isolated locked verifier before and after implementation.
- **Verification:** Exact E1–E3 named tests, all CLI targets, the full warnings-denied workspace test suite, warnings-denied Clippy, doctests, rustfmt, diff checks, the Book-v2 validator, and an independent implementation acceptance audit passed.
- **Commit:** `01de61203c04a04046df660865db735bae7bd59e`

## T-1108 (sprint 11)
- **Description:** Build the exact hardened SQLite v3/envelope v2 projection store
- **Intent:** [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md), [INT-0014](../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Completed:** 2026-08-20T04:07:02Z
- **Files modified:** `Cargo.toml`, `Cargo.lock`, `chain/pins.toml`, `chain/tools/verify-pins.sh`, `patches/rusqlite-0.40.2-commit-authorizer.patch`, `vendor/rusqlite-0.40.2-cubikan/src/version.rs`, `crates/cubikan-backend/Cargo.toml`, `crates/cubikan-backend/src/{error.rs,lib.rs,projection_store.rs,schema.rs,sqlite.rs,stored.rs}`, `crates/cubikan-backend/src/{projection_store,schema}/tests.rs`, `crates/cubikan-backend/tests/{legacy_generation.rs,relationship_model.rs,security.rs}`, `tests/fixtures/filesystem-boundary-v1.json`, `tests/fixtures/sqlite-authorizer-v1.json`, `tests/fixtures/envelope-v2/**`, `docs/sprints/s11/sprint-meta.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Scope repair:** Exact safe SQLite compile-option/VFS inspection required the pinned vendored rusqlite wrapper and reconstruction-pin updates; exhaustive error handling required minimal existing backend regression-test updates. Sprint 11 meta records both repairs.
- **Integrated implementation checkpoint:** `3239085b006884cb6c5c5452cc38b26e974635fb`
- **Verification:** All four real owner-only approved-ext4 branches passed under `CUBIKAN_TEST_SUPPORTED_ROOT`; exact E1–E6 tests, all backend targets, full warnings-denied workspace tests, warnings-denied Clippy, doctests, rustfmt, diff checks, the Book-v2 validator, and independent authorizer/acceptance audits passed.
- **Commit:** `da903ecb10b9e6d6169613821c88606159e17676`

## T-1109 (sprint 11)
- **Description:** Implement capability-gated v3 queries
- **Intent:** [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md), [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md), [INT-0014](../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Completed:** 2026-08-20T04:55:25Z
- **Files modified:** `crates/cubikan-backend/src/{lib.rs,model.rs,query.rs,relationship.rs,provenance.rs,verified_read.rs}`, `crates/cubikan-backend/tests/{read_boundary.rs,relationship_model.rs}`, `docs/sprints/s11/sprint-meta.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Scope repair:** T-1108's broad storage-authority regression test prohibited the private `rusqlite` implementation T-1109 assigns to `relationship.rs`; Sprint 11 meta records the narrow test-only exemption while retaining every public raw-connection, path, open, and unchanged-module prohibition.
- **Verification:** Exact E1–E4 named tests and the complete private query matrix passed on a fresh owner-only approved-ext4 root, including pinned-C refresh, the 5,000-ms Busy path, full-key cursors, decoded lookahead, and corruption rejection. Full warnings-denied workspace tests, warnings-denied Clippy, doctests, rustfmt, diff checks, the Book-v2 validator, and an independent final acceptance audit also passed.
- **Commit:** `4a9d7264d85ebc9172e604d1246ca087eae9d102`

## T-1110 (sprint 11)
- **Description:** Project and attest the complete finalized archive-RPC event stream
- **Intent:** [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md), [INT-0014](../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Completed:** 2026-08-20T06:23:48Z
- **Files modified:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-chain-client/**`, `crates/cubikan-backend/Cargo.toml`, `crates/cubikan-backend/src/{lib.rs,projector.rs,attestation.rs,sqlite.rs,verified_read.rs}`, `crates/cubikan-backend/src/{projector,attestation}/tests.rs`, `crates/cubikan-backend/tests/read_boundary.rs`, `tests/fixtures/finalized-events-v1/**`, `docs/sprints/s11/sprint-meta.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Scope repair:** Production-only attested snapshot minting required one crate-private constructor in `verified_read.rs`; literal no-caller-checkpoint closure required its integration-test update; and the first populated relationship/association deletes exposed exact SQLite foreign-key parent reads omitted by T-1108's empty-table authorizer run. Sprint 11 meta records both narrow repairs; the immutable authorizer-v1 fixture remains byte-identical.
- **Oracle evidence:** The independent finalized-event corpus verifies 6 blocks, 11 accepted events, all eight payload variants, 75 fault cases, and exact projection replay. Manifest SHA-256 `90d969339a2b08d4872b7a9e4fa65d010a3c61bae6129c12a31462890bb03b71`; inventory SHA-256 `feea5b6a39c0204dfb2d4be9c7cd12dc73060c6e1ca3db78809663d140885a63`; stream SHA-256 `0a414d49f6dce4bbfa6d5584fda112c3875bbc229cc195503675055999634dce`; expected-projection SHA-256 `0ea41a2a4dd9477446d6491a00504b6ad7cfc6fa93e57b3b391af4519489aeef`; faults SHA-256 `fd11e2a6b55ea9aa948e62a74ba6871bae0d5fc6e980486643b3d1e390b01e29`.
- **Verification:** Exact E1–E6 tests, chain-client hostile RPC/SCALE tests, the complete backend suite on a fresh owner-only approved-ext4 root, full warnings-denied workspace tests, warnings-denied Clippy, doctests, rustfmt, fixture and pin verifiers, diff checks, and an independent final acceptance audit all passed.
- **Commit:** `b7d1591ec6b8485c0139722eb8f9b22f919882be`

## T-1111 (sprint 11)
- **Description:** Submit finalized Subxt mutations through a crash-recoverable signer lane
- **Intent:** [INT-0014](../intents/INT-0014-canonical-blockchain-lifecycle-and-verified-sqlite-projection.md)
- **Completed:** 2026-08-20T07:55:50Z
- **Files modified:** `Cargo.lock`, `crates/cubikan-chain-client/Cargo.toml`, `crates/cubikan-chain-client/src/{lib.rs,submission.rs,submission_journal.rs}`, `crates/cubikan-chain-client/tests/{submission.rs,submission_journal.rs}`, `tests/fixtures/submission-journal-v1/**`, `docs/sprints/s11/sprint-meta.md`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Test-placement note:** Exact deterministic E2/E4/E5 behavior uses a crate-private scripted finalized-chain source inside `submission.rs`; public integration tests independently seal the API/oracle boundary, and E1/E3/E6 retain their real-process integration coverage. Sprint 11 meta records why this preserves the no-raw-authority contract.
- **Oracle evidence:** Independent manifest SHA-256 `ef3241ee5cb7d1cda3f12c628aae1f0533fd3fb673ffeecae0dc0626c15f942c`; inventory SHA-256 `590d2e4b414f11bfa2c4a6355a60bfefb4a339256fff25469a86fe19f6671e21`; signed-extrinsic oracle SHA-256 `ed971f3032334f8d99de5d0a41000191f8985aea28c3c5b8277d1e0d195385b1`; closed-tree SHA-256 `5b6044769ee2f1e3d1cb1f90b2f0fa6fda76e7483f738ce712e7bac08381bcfb`. The verifier consumes 5 journal states, 39 rejections, 36 transition cells, 23 crash points, and 38 reconciliation cases; Alice is synthetic oracle-only while Charlie and Dave are the only production signer choices.
- **Verification:** Exact E1–E6 tests, the complete real-process crash/lock matrix on a fresh owner-only approved-ext4 root, raw finalized System-event and all-eight-call SCALE tests, persisted-expiry era rescans, full warnings-denied workspace tests, warnings-denied workspace Clippy, rustfmt, frozen fixture and pin verifiers, diff checks, the Book-v2 validator, and an independent final acceptance/security audit all passed.
- **Integrated implementation commit:** `da00c7e8d729b250ccae1441ae327f7ce73eb5e1`
