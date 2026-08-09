# Sprint 6 Test Report

## Intent Verification

| Intent | Acceptance criterion | EARS / tests | Result | Intent evidence update |
|--------|----------------------|---------------|--------|------------------------|
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | The appendix has the exact advisory title and accurately states the current chain-agnostic core and one-shot CLI boundary. | T-601-E1–E2 / `test_appendix_authority_and_current_boundary`, `test_reader_can_navigate_summary_to_appendix_and_intents` | pass | Link this report as Test evidence and the appendix as Documentation evidence; eligible for realization after Loop attaches completion evidence. |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | Every retained theme is classified without conflating lifecycle, provenance, delegation, cross-unit, or execution edges. | T-601-E2–E4, T-602-E2–E4, T-603-E2–E5 / `test_theme_to_intent_to_repository_traceability`, `test_graph_and_authority_boundaries_are_distinct` | pass | The bounded `DV-01`–`DV-09` inventory is fully traced; no completeness claim is made for omitted chat context. |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | The catalog covers manager/doer operations, provenance and analytics, configurable process/KPI design, skill graphs, organizational applications, and Animus accounting. | T-602-E1–E4, T-603-E1–E4 / `test_derivative_catalog_has_six_complete_unique_entries` | pass | Exactly six primary recommendations cover the six requested derivative families. |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | Every recommendation states outcome, ownership, integration, prerequisites, creation trigger, and non-goals without authorizing a repository. | T-602-E1, T-603-E1, T-603-E5 / `test_agent_accounting_entries_have_required_contract_fields`, `test_process_application_entries_have_required_contract_fields`, `test_appendix_sequence_and_noncommitment_scope` | pass | All six entries have complete fields; slugs remain non-binding recommendations. |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | Reusable backend gaps are separate proposed intents with an explicit dependency map. | T-601-E4 / `test_proposed_backend_intent_map_is_complete_and_unscheduled`, `test_each_derivative_maps_to_declared_cubikan_capabilities` | pass | INT-0008–INT-0012 remain `proposed` with no Work or Completion evidence. |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | Derivatives use an explicitly pinned current core or a future versioned boundary, never shared storage, provisional Serde, or an implied cross-version API. | T-601-E3 / `test_derivative_dependency_direction_is_safe`, `test_each_derivative_maps_to_declared_cubikan_capabilities` | pass | Dependency direction and hard prerequisites are explicit for every recommendation. |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | Each datum has one authority; the Book remains current semantic/history authority and operational truth cannot move without a selected migration/projection intent. | T-601-E6, T-602-E2–E4, T-603-E2–E5 / `test_book_backend_authority_avoids_split_brain`, `test_graph_and_authority_boundaries_are_distinct` | pass | The authority map prohibits Book/backend dual-write and conflicting system-of-record ownership. |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | Navigation and references validate; the sprint stays prose-only, preserves realized contracts and remote configuration, and performs no derivative-repository operation. | T-601-E5, T-603-E5–E6 / `test_book_navigation_and_local_links_resolve`, `test_prose_only_scope_has_no_runtime_or_existing_intent_drift`, `test_remote_scope_matches_recorded_baseline_and_operations`, `test_t603_candidate_passes_five_local_rust_gates`, `test_hosted_sprint_six_quality_run_succeeds` | pass | Nineteen changed paths fit the twenty-path allowlist; protected content is unchanged; the only remote mutation was the authorized push to CubiKan `dev`. |

## Summary

- Unit/repository checks: 18 passed / 0 failed / 18 total.
- Integration checks: 5 passed / 0 failed / 5 total.
- E2E checks: 2 passed / 0 failed / 2 total. The hosted checkpoint is the same real run cited by Integration, not a second execution.
- Runtime regression: 100 passed / 0 failed / 100 all-target tests; 1 passed / 0 failed workspace doctests.
- CI status: green at the exact committed Build head.
- Test critique: [clean](critique.md); no concerns.

Detailed provenance is recorded in the [unit/repository](unit-tests.md),
[integration](integration-tests.md), and [E2E](e2e-tests.md) artifacts.

## CI Confirmation

- **Head SHA:** `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7`
- **CI run:** [Rust CI run 31293927701](https://github.com/crussella0129/CubiKan/actions/runs/31293927701)
- **Conclusion:** success on attempt 1 for event `push`, branch `dev`
- **Confirmations:** sole [Rust quality gate job 93195790436](https://github.com/crussella0129/CubiKan/actions/runs/31293927701/job/93195790436) completed successfully; setup, checkout, stable-Rust installation, formatting, Clippy, warnings-denied workspace check, all-target tests, doctests, post-checkout, and completion all succeeded. Local `HEAD`, `origin/dev`, and the GitHub run API named the same SHA.

The hosted result preserves the existing INT-0005 quality boundary. It is not
proof that a derivative runtime or repository exists, that checks are required
by branch protection, or that future floating runner/toolchain runs cannot
flake.

## Failures

(none)

## Technical Debt Identified

- [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) — immutable origin and artifact-provenance associations remain proposed; full revision-scoped bidirectional behavior waits for INT-0009 and INT-0010.
- [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) — optimistic revision semantics remain proposed and are the first hard prerequisite for a durable backend.
- [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) — storage, transport, schema evolution, concurrency, and operational query policy require a later selected sprint.
- [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md) — measurement definitions and evidence remain caller-governed future work after revisioned durable state.
- [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) — typed cross-unit relations and advanced board projections remain proposed after a durable multi-unit boundary.

These chapters preserve discovered capability boundaries; Sprint 6 neither
schedules nor partially realizes them.

## Coverage Observations

- The prose outcome has credible E2E coverage: a reader traversed Summary →
  appendix index → derivative catalog → owning intents and capability
  prerequisites across 91 Markdown documents, 521 local links, and 7 fragment
  references with no failure.
- Product/runtime derivative E2E is not yet possible because this sprint
  creates no derivative repository or versioned backend. It unlocks only after
  a later sprint realizes the relevant INT-0008–INT-0012 capability and a
  derivative repository is separately authorized and implemented.
- Repository slugs remain code labels and local headings rather than links to
  nonexistent external repositories.
- External CI evidence is one successful, uncached attempt on floating
  `ubuntu-latest` and current stable Rust. It does not establish future
  availability, an MSRV, cross-platform support, coverage/security
  certification, release behavior, or external non-mutation.
- Existing Rust behavior was not modified; the complete 100-test and one-doctest
  suites are preservation evidence, not proof of a derivative runtime.
