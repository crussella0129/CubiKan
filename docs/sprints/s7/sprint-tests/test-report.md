# Sprint 7 Test Report

## Intent Verification

| Intent | Acceptance criterion | EARS / tests | Result | Intent evidence update |
|--------|----------------------|---------------|--------|------------------------|
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | A new Intent Unit exposes one documented, clock-independent initial revision. | T-701-E1, T-704-E1–E2 / `test_intent_unit_starts_at_zero_revision`, `test_public_revision_doctest_compiles`, `test_readme_defines_revision_contract_and_nonclaims` | pass | Link this report as Test evidence; initial revision `0` is public, typed, and documented without clock or global-ordering semantics. |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Every accepted declared transition and eligible completion advances revision exactly once beside its one lifecycle record. | T-701-E2, T-702-E1–E2 / `test_unconditioned_mutations_advance_revision_once_per_record`, `test_guarded_transition_returns_exact_successor_revision`, `test_guarded_completion_returns_exact_successor_revision` | pass | Unconditioned and guarded paths cover forward, reverse, self-edge, final transition, and completion behavior with exact revision/record agreement. |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Current expectations preserve realized lifecycle behavior; mismatches return typed expected/actual conflicts and preserve the full aggregate. | T-702-E1–E4 / `test_guarded_transition_returns_exact_successor_revision`, `test_guarded_completion_returns_exact_successor_revision`, `test_revision_conflict_exposes_expected_and_actual`, `test_competing_observers_reject_second_command_and_allow_refresh` | pass | The strengthened guarded-success tests preserve exact records, prior history, identity, species, workflow, phase, and status; stale rejection compares the complete aggregate. |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Revision comparison precedes lifecycle evaluation, while a current expectation retains terminal, target, edge, and completion-eligibility errors. | T-702-E4–E5 / `test_stale_revision_precedes_transition_errors_atomically`, `test_stale_revision_precedes_completion_errors_atomically`, `test_stale_revision_precedes_terminal_errors_atomically`, `test_current_revision_preserves_domain_errors_atomically` | pass | Stale-first precedence is proved for otherwise valid and independently invalid commands; current observations expose the unchanged typed domain errors and sources. |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Both stale-plus-domain-invalid and current-plus-domain-invalid combinations reject atomically. | T-702-E4–E5 / complete transition/completion precedence matrix in `test_stale_revision_precedes_transition_errors_atomically`, `test_stale_revision_precedes_completion_errors_atomically`, `test_stale_revision_precedes_terminal_errors_atomically`, and `test_current_revision_preserves_domain_errors_atomically` | pass | Every negative case snapshots and compares the entire `IntentUnit`, including revision, rather than checking only phase or history length. |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Validated restoration rejects revision/history/phase/status disagreement and semantic round trips preserve the exact revision. | T-703-E1–E3 / `test_revision_round_trip_preserves_active_and_completed_units`, `test_restored_unit_continues_from_exact_revision`, `test_restore_rejects_missing_or_mismatched_revision`, `test_restore_rejects_history_phase_or_status_disagreement` | pass | Active/completed round trips preserve revision; missing, malformed, lower, higher, and lifecycle-inconsistent representations all reject through validated replay. |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | Documentation limits revisions to aggregate-local optimistic conflict detection and excludes stronger persistence, isolation, locking, atomicity, and delivery guarantees. | T-704-E1–E4 / `test_public_revision_doctest_compiles`, `test_readme_defines_revision_contract_and_nonclaims`, `test_sprint_scope_preserves_cli_protocol_and_dependencies`, `test_book_v2_validation` | pass | README and crate docs define reject-refresh-decide usage and explicit nonclaims; manifests, dependencies, CI, and CLI v1 are unchanged. |

## Summary

- Core library unit tests: 44 passed / 0 failed / 44 total.
- Public core integration tests: 21 passed / 0 failed / 21 total (16 lifecycle and 5 validated-serialization tests).
- Product competing-client E2E: N/A for this core-only sprint; [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) is the explicit unlock.
- Existing actual-process CLI regression: 6 passed / 0 failed / 6 total; these are preservation evidence, not revision-conflict E2E proof.
- Full workspace regression: 116 passed / 0 failed / 116 all-target tests; 1 passed / 0 failed workspace doctest.
- CI status: green at exact tested candidate `55cbdea6a492e6b958f92fd9e6286f14bad737cb`.
- Test critique: [clean](critique.md) after the guarded-success assertions were strengthened; no remaining concerns.

Detailed provenance is recorded in the [unit/repository](unit-tests.md),
[integration](integration-tests.md), and [E2E](e2e-tests.md) artifacts.

## CI Confirmation

- **Head SHA:** `55cbdea6a492e6b958f92fd9e6286f14bad737cb`
- **Build ledger head:** `071341d1632ca6cfe363a334b33ba0b77209401e`; the tested head adds only the critic-response lifecycle-test strengthening.
- **CI run:** [Rust CI run 31301197841](https://github.com/crussella0129/CubiKan/actions/runs/31301197841)
- **Conclusion:** success on attempt 1 for event `push`, branch `dev`
- **Confirmations:** sole [Rust quality gate job 93214154471](https://github.com/crussella0129/CubiKan/actions/runs/31301197841/job/93214154471) completed successfully; setup, pinned checkout, stable-Rust installation, formatting, Clippy, warnings-denied workspace check, all-target tests, doctests, post-checkout, and completion all succeeded. Local `HEAD`, `origin/dev`, the run/job API, checkout fetch/revision, and hosted `git log -1` identified the same tested SHA.

The hosted result is a quality and exact-revision delivery oracle. It is not
proof of database compare-and-set, isolation, competing remote clients, branch
protection, merge authorization, or future floating runner/toolchain stability.

## Failures

(none)

The first Test Critic pass identified weak guarded-success assertions. Test-only
commit `55cbdea6a492e6b958f92fd9e6286f14bad737cb` closed that concern by proving
declared forward, reverse, and self-edge records plus immutable aggregate fields
and completion-history preservation. The complete suite and hosted gate then
passed at that exact commit, and the second critic pass returned `clean`.

## Technical Debt Identified

- [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) — a later durable adapter must expose observed revisions and perform conditional mutation against one stable stored aggregate before competing-client product E2E is possible.
- [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) — revision-scoped provenance and artifact associations remain proposed and still depend on a selected durable query/index boundary for bidirectional lookup.
- [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md) — lifecycle-linked measurement evidence remains proposed; Sprint 7 supplies only its revision prerequisite, not observation, time, correction, or metric policy.

The one-shot CLI remains deliberately revision-unaware. Expanding its version 1
wire contract, adding persistence, or selecting retry/idempotency policy was not
required to realize this backend-neutral core primitive.

## Coverage Observations

- The public lifecycle matrix covers guarded current-observer success across declared forward, reverse, and self edges, guarded completion, two-observer reject-refresh behavior, stale-plus-valid commands, every stale-plus-invalid precedence branch, and every current-plus-invalid domain branch.
- Full-aggregate equality on rejection covers identity, species, owned workflow, phase, status, history, and revision. Success assertions independently cover exact records, history prefixes, immutable fields, and successor revisions.
- Restoration coverage includes active and completed round trips, continuation from the restored token, missing revision, negative/fraction/string/out-of-range values, lower/higher replay mismatches, sequence/source/completion-phase/final-phase/final-status corruption, and invalid workflow topology.
- Product-level competing-client E2E is honestly deferred because the current CLI creates a new in-memory unit per process and exposes neither retrieval nor revision-conditioned mutation. INT-0010 names the exact durable boundary needed to unlock it.
- The final Test-phase link resolver inspected 104 Markdown files, 704 Markdown links, 646 local links, and 8 fragment targets with 0 errors. This is reachability evidence separate from Book schema validation.
- The exact-head hosted gate was one uncached attempt on floating `ubuntu-latest` and current stable Rust. It does not establish an MSRV, cross-platform support, ongoing external availability, performance, security certification, or durable concurrency semantics.
