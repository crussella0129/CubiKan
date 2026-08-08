# Sprint 0 Unit Test Results

## Summary

- Result: **52 passed / 0 failed / 52 planned unit and repository checks**.
- Rust module tests: **43 passed / 0 failed** via `cargo test --workspace --lib`.
- Repository, documentation, and executable-example checks: **9 passed / 0 failed**.
- Stubs, mocks, clocks, sleeps, retries, and external services: **none**.

## Command Evidence

| Command or review | Result | Evidence |
|---|---:|---|
| `cargo test --workspace --lib` | Pass | 43 named module tests passed; 0 failed or ignored. |
| `cargo metadata --no-deps --format-version 1` | Pass | One workspace member named `cubikan-core`; target kind is `lib`. |
| `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` | Pass | Workspace and all test targets compiled with warnings denied. |
| Five named decision-log content checks | Pass | Core boundary, snapshot ownership, UUID v4/no-ordering, validated provisional serialization, and product deferrals are each recorded in the log now preserved at `docs/history/decisions-legacy.md`. |
| `README.md` model/boundary content review | Pass | Intent Units, species, workflow topology, transitions, completion, commands, and every Sprint 0 exclusion are documented. |
| `cargo test --doc --workspace` | Pass | The public create-transition-complete example compiled and passed (1 doctest). |

No canonical repository runner or CI configuration exists, so these commands are the local confirmations.

## EARS Traceability

Every EARS clause in the locked build plan maps to at least one named check below. Where one clause contains multiple input classes, the test plan deliberately uses multiple tighter cases.

### T-001 — architecture decisions (5/5)

- `test_decision_log_records_core_boundary` — verifies the chain-agnostic Rust core and deferred blockchain, persistence, API, and UI adapters.
- `test_decision_log_records_workflow_snapshot_ownership` — verifies caller-declared topology and per-unit immutable snapshots.
- `test_decision_log_records_identifier_contract` — verifies opaque UUID v4 generation without an ordering promise.
- `test_decision_log_records_serialization_contract` — verifies checked restoration and provisional wire compatibility.
- `test_decision_log_records_product_deferrals` — verifies default-phase, KPI, naming, lineage, authorization, and concurrency deferrals.

All five passed by direct content review of the then-current `decisions.md`, now preserved losslessly at `docs/history/decisions-legacy.md`.

### T-002 — workspace (2/2)

- `test_workspace_manifest_resolves` — Cargo metadata reports exactly the `cubikan-core` library workspace member.
- `test_core_crate_all_targets_compile` — all targets compile with `RUSTFLAGS=-D warnings`.

### T-003 — identifiers (4/4)

- `test_generated_intent_unit_id_is_non_nil_v4`
- `test_generated_intent_unit_ids_differ`
- `test_intent_unit_id_parse_display_round_trip`
- `test_intent_unit_id_rejects_malformed_text`

The tests assert the UUID variant/non-nil value, inequality of two generated values, exact fixed-value round trip, and the typed parse error respectively.

### T-004 — textual vocabulary (3/3)

- `test_domain_values_preserve_non_blank_text`
- `test_domain_values_reject_empty_text`
- `test_domain_values_reject_whitespace_only_text`

The tests cover all three opaque text types and exact preservation of ASCII, Unicode, surrounding whitespace, and KPI-associated caller text.

### T-005 — workflow topology (9/9)

- `test_workflow_accepts_explicit_topology`
- `test_workflow_rejects_empty_phase_set`
- `test_workflow_rejects_duplicate_phase`
- `test_workflow_rejects_duplicate_edge`
- `test_workflow_rejects_unknown_initial_phase`
- `test_workflow_rejects_unknown_edge_source`
- `test_workflow_rejects_unknown_edge_target`
- `test_workflow_rejects_unknown_completion_phase`
- `test_workflow_allows_only_declared_reverse_and_self_edges`

Assertions compare the exact accepted topology and exact typed error variants; reverse and self edges are checked in both declared and undeclared forms.

### T-006 — Intent Unit construction (3/3)

- `test_intent_unit_starts_active_at_initial_phase`
- `test_intent_unit_owns_workflow_snapshot`
- `test_intent_unit_identity_accessors_are_stable`

Assertions cover active status, initial phase, empty history, the owned workflow value, and repeated immutable identity/species/workflow reads.

### T-007 — guarded transitions (6/6)

- `test_allowed_transition_moves_and_appends_record`
- `test_disallowed_transition_is_atomic`
- `test_unknown_target_transition_is_atomic`
- `test_configured_reverse_transition_succeeds`
- `test_transition_history_preserves_order`
- `test_transition_preserves_identity`

Successful cases assert exact phase and record contents; failure cases compare complete before/after aggregate snapshots for equality.

### T-008 — completion (5/5)

- `test_completion_from_eligible_phase_is_terminal`
- `test_completion_from_ineligible_phase_is_atomic`
- `test_second_completion_is_rejected_without_mutation`
- `test_transition_after_completion_is_rejected_without_mutation`
- `test_completion_preserves_identity_and_species`

The tests assert the exact completion record and terminal status, exact error variants, complete before/after equality on rejection, and stable identity/species/workflow.

### T-009 — scalar and workflow serialization (8/8)

- `test_identifier_and_vocabulary_semantic_round_trip`
- `test_workflow_semantic_round_trip`
- `test_serialization_rejects_malformed_identifier`
- `test_serialization_rejects_blank_vocabulary`
- `test_serialization_rejects_empty_workflow`
- `test_serialization_rejects_duplicate_phase_or_edge`
- `test_serialization_rejects_unknown_initial_phase`
- `test_serialization_rejects_unknown_edge_or_completion_phase`

Valid cases compare semantic values for equality. Invalid JSON is mutated structurally and must fail checked deserialization for each listed invariant.

### T-010 — Intent Unit serialization (5/5)

- `test_active_intent_semantic_round_trip`
- `test_completed_intent_semantic_round_trip`
- `test_serialization_rejects_inconsistent_lifecycle_history`
- `test_serialization_rejects_disallowed_recorded_edge`
- `test_serialization_rejects_invalid_completion_record`

Round trips compare complete semantic aggregates. Tampering covers broken sequences, discontinuity, disallowed edges, state/history disagreement, ineligible completion, and post-completion records.

### T-011 — consumer documentation (1/1)

- `test_readme_documents_vocabulary_and_sprint_zero_exclusions` — direct content review confirmed the complete model, development commands, and all locked deferrals.

### T-012 — public example (1/1)

- `test_documented_lifecycle_example` — `cargo test --doc --workspace` ran the caller-configured create-transition-complete example successfully.
