Finalized - DO NOT EDIT

# Sprint 7 Test Plan

## Intent Traceability

| Intent | Acceptance criterion | Build task / EARS clause | Verification |
|--------|----------------------|--------------------------|--------------|
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | A new Intent Unit exposes one documented, clock-independent initial revision. | T-701-E1, T-704-E1–E2 | `test_intent_unit_starts_at_zero_revision`, `test_public_revision_doctest_compiles`, `test_readme_defines_revision_contract_and_nonclaims` |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Every successful declared transition, reverse/self-edge, and eligible completion advances revision exactly once with one record. | T-701-E2, T-702-E1–E2 | `test_unconditioned_mutations_advance_revision_once_per_record`, `test_guarded_transition_returns_exact_successor_revision`, `test_guarded_completion_returns_exact_successor_revision` |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Current expectations preserve lifecycle behavior; mismatches return typed conflicts and preserve the full aggregate. | T-702-E1–E4 | `test_competing_observers_reject_second_command_and_allow_refresh`, `test_stale_revision_rejects_otherwise_valid_command_atomically`, `test_revision_conflict_exposes_expected_and_actual` |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Revision comparison precedes command evaluation; current expectations retain terminal, unknown-target, undeclared-edge, and completion-ineligible errors. | T-702-E4–E5 | `test_stale_revision_precedes_transition_errors_atomically`, `test_stale_revision_precedes_completion_errors_atomically`, `test_stale_revision_precedes_terminal_errors_atomically`, `test_current_revision_preserves_domain_errors_atomically` |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Both stale-plus-invalid and current-plus-invalid paths are atomic. | T-702-E4–E5 | the complete transition/completion precedence matrix above; every case snapshots and compares the entire `IntentUnit` |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Restoration rejects missing, malformed, or inconsistent revision/history/phase/status, while valid round trips preserve revision exactly. | T-703-E1–E3 | `test_revision_round_trip_preserves_active_and_completed_units`, `test_restored_unit_continues_from_exact_revision`, `test_restore_rejects_missing_or_mismatched_revision`, `test_restore_rejects_history_phase_or_status_disagreement` |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Documentation limits revisions to optimistic conflict detection and excludes stronger guarantees. | T-704-E1–E4 | `test_public_revision_doctest_compiles`, `test_readme_defines_revision_contract_and_nonclaims`, `test_sprint_scope_preserves_cli_protocol_and_dependencies`, `test_book_v2_validation` |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | Existing lifecycle error taxonomy, atomic mutation, history, topology, terminal behavior, and validated restoration remain realized. | T-701-E2–E3, T-702-E5, T-703-E3 | existing core unit/integration tests plus strengthened full-aggregate assertions |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | The one-shot CLI remains behaviorally unchanged and does not claim revision-aware coordination. | T-704-E3 | full CLI runner and actual-process regression suites; scoped diff inspection |

## Unit Tests

### T-701 revision state and unconditioned mutation

- **Intent:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- `test_intent_unit_starts_at_zero_revision` [T-701-E1]: construct a unit and assert its public typed revision is `IntentUnitRevision::INITIAL`, numeric value `0`, with active status, initial phase, and empty history.
- `test_unconditioned_mutations_advance_revision_once_per_record` [T-701-E2]: execute forward, reverse, declared self-edge, final transition, and completion operations; after each success assert `revision == prior + 1`, history grew by exactly one, and the latest one-based record sequence equals the numeric revision.
- `test_failed_unconditioned_commands_preserve_revision_and_aggregate` [T-701-E3]: exercise unknown target, undeclared edge, ineligible completion, post-completion transition, and repeated completion; assert the existing exact error and full `IntentUnit` equality against a pre-command clone.
- `test_revision_checked_next_rejects_maximum_without_wrap` [T-701-E4]: exercise the private checked-successor seam at `u64::MAX`; assert it returns no successor and never wraps to zero.
- Stubs/mocks: none.

### T-702 guarded command and precedence matrix

- **Intent:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- `test_guarded_transition_returns_exact_successor_revision` [T-702-E1]: use the current revision for a valid transition and assert the returned revision, accessor, history delta, record sequence, and phase all agree on one successor.
- `test_guarded_completion_returns_exact_successor_revision` [T-702-E2]: reach an eligible phase, complete using its current revision, and assert one successor revision, one completion record, unchanged phase, and terminal status.
- `test_revision_conflict_exposes_expected_and_actual` [T-702-E3]: retain two copies of one observed revision, let the first command succeed, and match the second command's public conflict including exact expected and actual accessors, `Display`, and error source.
- `test_competing_observers_reject_second_command_and_allow_refresh` [T-702-E3]: after the first observer succeeds, assert the second observer's stale command leaves the full aggregate unchanged; refresh to the actual revision and assert the next valid command succeeds.
- `test_stale_revision_rejects_otherwise_valid_command_atomically` [T-702-E4]: issue an operation that is valid against the aggregate's current phase but carries an older revision; assert conflict precedence and full equality.
- `test_stale_revision_precedes_transition_errors_atomically` [T-702-E4]: table-test stale expectations with an unknown target and a known target on an undeclared edge; require conflict, never wrapped `UnknownTarget` or `NotAllowed`, and full equality.
- `test_stale_revision_precedes_completion_errors_atomically` [T-702-E4]: attempt ineligible completion with a stale expectation; require conflict and full equality.
- `test_stale_revision_precedes_terminal_errors_atomically` [T-702-E4]: on a completed unit, use a stale revision for transition and completion—including transition to an unknown target—and require conflict before terminal or target evaluation.
- `test_current_revision_preserves_domain_errors_atomically` [T-702-E5]: with the current revision, separately assert wrapped `UnknownTarget`, `NotAllowed`, `PhaseNotEligible`, transition `AlreadyCompleted`, and completion `AlreadyCompleted`, including exact payloads, error sources, and full aggregate equality.
- Stubs/mocks: none. Every rejection compares the entire derived-`Eq` aggregate, covering identity, species, workflow, phase, status, history, and revision.

### T-704 documentation and repository checks

- **Intents:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) and [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- `test_readme_defines_revision_contract_and_nonclaims` [T-704-E2]: inspect README and crate docs for initial revision `0`, one increment per accepted lifecycle command, compare-before-evaluate behavior, typed reject-and-refresh guidance, revision-versus-record-sequence distinction, and explicit exclusions for database isolation, cross-unit atomicity, locking, and delivery idempotency.
- `test_sprint_scope_preserves_cli_protocol_and_dependencies` [T-704-E3]: compare accepted base to Build head and require no manifest, lockfile, dependency, workflow, CLI protocol, response, error-code, fixture, persistence, transport, clock, actor, or unrelated product-policy change.
- `test_book_v2_validation` [T-704-E4]: run the installed Book validator, assert INT-0009 has legal work/state evidence, and independently resolve every new local Markdown link because schema validation does not prove link reachability.
- These are Test-phase repository inspections recorded as evidence; they do not require an implementation-mirroring test harness.

## Integration Tests

### Public core lifecycle contract

- **Intent:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- Place public-consumer checks in `crates/cubikan-core/tests/lifecycle.rs`, importing every new token and error solely through the crate root.
- `test_public_unconditioned_lifecycle_advances_revision` [T-701-E1–E3]: use the existing public mutators through forward, reverse/self, completion, and rejection paths; assert revision/record agreement and preserved error/atomic behavior.
- `test_public_revisioned_lifecycle_accepts_current_observer` [T-702-E1–E2]: perform guarded transitions and completion through exported APIs; assert exact returned revisions and one-record advancement.
- `test_public_revisioned_lifecycle_rejects_competing_observer_atomically` [T-702-E3–E4]: model two copied observations, prove first-writer success, second-writer typed conflict, full aggregate preservation, and success after refresh.
- `test_public_current_revision_preserves_error_taxonomy` [T-702-E5]: match every wrapped existing domain error through the public API and prove its source value remains the original typed error.
- Existing lifecycle journeys continue to cover custom topology, rework, self-edges, ordered history, immutable provenance, recovery after rejection, and terminal behavior.

### Validated serialization contract

- **Intent:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), preserving [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md)
- `test_revision_round_trip_preserves_active_and_completed_units` [T-703-E1]: serialize and restore active and completed units; explicitly assert exact revision and full semantic equality.
- `test_restored_unit_continues_from_exact_revision` [T-703-E1]: restore an active unit, submit the restored current revision to a valid guarded command, and assert the exact next revision and record.
- `test_restore_rejects_missing_or_mismatched_revision` [T-703-E2]: independently remove revision, replace it with a negative, fractional, or out-of-range value, and replace a valid stored revision with lower and higher values while leaving other state valid; require deserialization failure in every case.
- `test_restore_rejects_history_phase_or_status_disagreement` [T-703-E3]: retain a plausible correct revision while independently corrupting lifecycle sequence, transition source, completion-record phase, final aggregate phase, and final status; require validated restoration to reject every variant.
- Existing topology, disallowed-recorded-edge, invalid-completion-record, record-after-completion, and semantic-round-trip tests remain regression oracles.

### Public documentation and hosted integration

- `test_public_revision_doctest_compiles` [T-704-E1]: the crate doctest imports the new public revision/error surface, observes revision `0`, performs a guarded command, and asserts the returned/accessor revision.
- After the complete Build head is pushed to `dev`, require the existing hosted `Rust CI` run and sole quality job to succeed for that exact SHA. Record run/job URLs and individual step conclusions. This is a Test-phase strengthening gate for the unchanged automation, not a Build EARS prerequisite or product E2E claim.

## End-to-End Tests

- **Status:** not-yet-possible for INT-0009's competing-client behavior.
- **Unlocked by:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md), once a durable adapter exposes revision observation and conditional mutation across separate client requests.
- The current CLI creates one in-memory aggregate per process and exposes no resumable revision token. Expanding its v1 protocol merely to manufacture an E2E case would violate Sprint 7's boundary.
- Existing Cargo-built CLI actual-process tests still run as regression coverage under T-704-E3, but are not claimed as revision-conflict E2E evidence.
- Hosted CI is a delivery/quality integration oracle, not product E2E proof.

## Test Artifact Locations

- Unit, documentation, scope, local quality, and Book evidence: `docs/sprints/s7/sprint-tests/unit-tests.md`.
- Public core, serialization, and hosted job integration evidence: `docs/sprints/s7/sprint-tests/integration-tests.md`.
- Runtime-E2E deferral and existing actual-process regression evidence: `docs/sprints/s7/sprint-tests/e2e-tests.md`.
- Reviewed intent verification and exact committed-head provenance: `docs/sprints/s7/sprint-tests/test-report.md`.

## Final Quality Gates

- `cargo +stable test -p cubikan-core --lib`
- `cargo +stable test -p cubikan-core --test lifecycle`
- `cargo +stable test -p cubikan-core --test serialization`
- `cargo +stable metadata --no-deps --format-version 1`
- `cargo +stable tree --workspace --edges normal`
- `cargo +stable fmt --all -- --check`
- `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings`
- `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets`
- `cargo +stable test --workspace --all-targets`
- `cargo +stable test --doc --workspace`
- `git diff --check`
- Installed Book validation and explicit Markdown-link inspection.
- Successful hosted `dev` push run for the exact committed Build head.
