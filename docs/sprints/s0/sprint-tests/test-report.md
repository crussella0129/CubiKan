# Sprint 0 Test Report

## Intent Verification

| Intent | Acceptance criterion | EARS / tests | Result | Intent evidence update |
|--------|----------------------|---------------|--------|------------------------|
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | Durable evidence records the core boundary and deferrals. | T-001 / five decision-record checks | pass | Completion ledger and this report support `realized`. |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | The Rust workspace exposes one warning-free core library. | T-002 / metadata, warning-denied check, formatting, and Clippy | pass | This report is linked as Test evidence. |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | Identifiers and vocabulary validate and preserve semantic values. | T-003–T-004 / identifier and vocabulary unit tests | pass | This report is linked as Test evidence. |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | Caller workflows enforce exact topology and completion policy. | T-005 / workflow unit and configuration integration tests | pass | This report is linked as Test evidence. |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | Intent Units enforce atomic transitions, ordered history, and terminal completion. | T-006–T-008 / lifecycle unit and integration tests | pass | This report is linked as Test evidence. |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | Serialization restores valid state and rejects inconsistent state. | T-009–T-010 / serialization unit and integration tests | pass | This report is linked as Test evidence. |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | Documentation and an executable example explain the model and boundaries. | T-011–T-012 / README review and doctest | pass | This report is linked as Test and Documentation evidence. |

## Summary
- Unit tests/checks: 52 passed / 0 failed / 52 total (43 Rust module tests and 9 repository/documentation checks)
- Integration tests: 6 passed / 0 failed / 6 total
- E2E tests: N/A
- CI status: not-configured

## CI Confirmation
- **Head SHA:** `6f32a1228d08ced32eda391370f41c2b45cf0056`
- **CI run:** N/A
- **Conclusion:** success (local)
- **Confirmations:** [unit tests](unit-tests.md), [integration tests](integration-tests.md), [E2E status](e2e-tests.md), and [test critique](critique.md); final local gates were `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets`, `cargo test --workspace --all-targets`, and `cargo test --doc --workspace`.

CI not configured — local confirmations only. The repository defines no canonical suite runner, so the Test Phase recorded each authoritative local command in the artifacts above.

## Failures

(none)

The initial test critic blocked on three weak assertions. The affected workflow-policy, stable-accessor, and lifecycle-history tests were tightened, rerun successfully, and accepted by a follow-up critic with `clean` confidence.

## Technical Debt Identified

- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — E2E remains unavailable because Sprint 0 is a library with no runnable system boundary; the follow-on intent preserves the adapter and E2E outcome without prematurely selecting the boundary.
- The locked two-generated-UUID uniqueness smoke test is inherently probabilistic. Its UUID v4 collision risk is negligible, while replacing generation with a deterministic fixture would no longer exercise the production generator.
- No quantitative line or branch coverage tool is configured. Sprint 0 relies on EARS traceability, exhaustive topology assertions for the representative workflow, negative-path tests, integration journeys, and adversarial serialization tampering.

## Coverage Observations

- The final Rust suite contains 50 executable tests: 43 module tests, 6 public-API integration tests, and 1 doctest. All passed on the recorded head commit.
- All 40 EARS clauses across T-001 through T-012 have at least one named verification case; compound clauses are split into tighter cases where needed.
- Error-path coverage asserts exact typed errors and full aggregate equality before and after rejected operations.
- Workflow coverage includes declared and undeclared forward, reverse, and self edges plus exhaustive phase-pair policy checks on the representative topology.
- Serialization coverage routes restoration through domain validation and rejects malformed scalar values, invalid topology, broken lifecycle sequences, disallowed/discontinuous transitions, inconsistent state, invalid completion, and post-completion records.
- Integration coverage exercises arbitrary caller vocabulary, lifecycle success and recovery, explicit rework cycles, restore-and-continue behavior, and tamper rejection using only the public API.
- No external services, mocks, clocks, sleeps, retries, or shared mutable fixtures are used.
