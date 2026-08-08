# Sprint 1 Test Report

## Intent Verification

| Intent | Acceptance criterion | EARS / tests | Result | Intent evidence update |
|--------|----------------------|---------------|--------|------------------------|
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | Research records why a one-shot batch JSON CLI is the smallest useful, reversible runnable boundary. | T-101-E1, T-107-E2 / `test_cli_workspace_member_resolves`, `test_research_records_one_shot_cli_decision` | pass | Test evidence links this report; eligible for `realized` after completion evidence. |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | One versioned caller-defined scenario produces one versioned success or typed failure response. | T-102-E1–E2, T-103-E1–E4, T-105-E1–E4 / protocol, setup mapping, and runner suites | pass | Strict DTOs, exhaustive setup mapping, exact envelopes, and modeled result classifications are verified. |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | A process-level E2E drives configure → create → transition → complete and asserts JSON output and exit status. | T-106-E1 / `test_cli_configure_create_transition_complete` | pass | The Cargo-built executable is verified through real piped standard streams. |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | Adapter behavior delegates lifecycle invariants to `cubikan-core` without duplication or weakening. | T-103-E1–E4, T-104-E1–E5 / constructor, exhaustive error mapping, topology, atomicity, and terminal-state tests | pass | Core constructors and lifecycle methods remain the execution oracle. |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | Persistence, networking, deployment, authorization, KPI, naming, blockchain, and UI policy remain outside the implementation boundary. | T-101-E2, T-107-E2 / `test_cli_direct_dependency_boundary`, `test_cli_docs_preserve_stateless_boundary_and_exclusions` | pass | Dependency and documentation evidence preserve the stated boundary. |

## Summary

- Unit tests: 18 passed / 0 failed / 18 total
- Integration tests: 4 passed / 0 failed / 4 total
- E2E tests: 3 passed / 0 failed / 3 total
- Workspace regression: 74 passed / 0 failed / 74 total across all all-target binaries
- CI status: not-configured

## CI Confirmation

- **Head SHA:** `4a4d5bf999cd6eddcf76bf92950aeeb224a59811`
- **CI run:** CI not configured — local confirmations only
- **Conclusion:** success
- **Confirmations:** [unit and repository checks](unit-tests.md), [integration tests](integration-tests.md), and [actual-process E2E tests](e2e-tests.md)

The committed head passed Cargo metadata and direct-dependency inspection,
`cargo fmt --all -- --check`, Clippy with warnings denied, warnings-denied
all-target checking, all-target tests, workspace doctests, Book v2 validation,
and the documented CLI fixture command.

## Failures

None.

## Technical Debt Identified

- No follow-up intent was opened. [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) already records that standard-input size is unbounded for this local adapter and that resource limiting must precede production network exposure.

## Coverage Observations

- The adapter protocol is verified for complete success/setup/lifecycle envelopes and unknown-field rejection at every independently decoded DTO boundary.
- Setup failures exhaust every current public `WorkflowError`; lifecycle failures exhaust every current transition and completion error variant.
- Failure snapshots assert exact identity, workflow, status, and ordered prior history, while body-write and trailing-newline failures remain operational rather than modeled successes.
- Actual-process E2E covers success, malformed input, and lifecycle rejection with exit codes and exact observable JSON. Production resource limiting remains intentionally outside this sprint.
