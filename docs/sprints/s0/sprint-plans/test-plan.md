Finalized - DO NOT EDIT

# Sprint 0 Test Plan

## Unit Tests

### T-001 decision-record checks
- `test_decision_log_records_core_boundary`: require the chain-agnostic Rust core decision and adapter deferrals.
- `test_decision_log_records_workflow_snapshot_ownership`: require caller-configured topology and immutable per-unit snapshots.
- `test_decision_log_records_identifier_contract`: require opaque UUID v4 identity without ordering guarantees.
- `test_decision_log_records_serialization_contract`: require validated restoration and a provisional wire format.
- `test_decision_log_records_product_deferrals`: require explicit naming/lineage/KPI/auth/concurrency deferrals.
- Stubs: none.

### T-002 workspace checks
- `test_workspace_manifest_resolves`: `cargo metadata --no-deps` reports one workspace member named `cubikan-core`.
- `test_core_crate_all_targets_compile`: `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` exits successfully.
- Stubs: none; these are deterministic command checks.

### T-003 identifier unit tests
- `test_generated_intent_unit_id_is_non_nil_v4`: generated ID has UUID v4 and is not nil.
- `test_generated_intent_unit_ids_differ`: two generated IDs differ as a smoke check, without claiming a mathematical uniqueness proof.
- `test_intent_unit_id_parse_display_round_trip`: fixed UUID text parses and formats to the same value.
- `test_intent_unit_id_rejects_malformed_text`: malformed input returns the typed parse error.
- Stubs: fixed UUID literals; no services.

### T-004 vocabulary unit tests
- `test_domain_values_preserve_non_blank_text`: `WorkflowId`, `PhaseId`, and `IntentSpecies` preserve representative ASCII, Unicode, and custom/KPI-associated strings.
- `test_domain_values_reject_empty_text`: each type rejects an empty string.
- `test_domain_values_reject_whitespace_only_text`: each type rejects whitespace-only input while making no additional normalization promise.
- Stubs: table-driven text values.

### T-005 workflow unit tests
- `test_workflow_accepts_explicit_topology`: valid initial phase, directed edges, and completion phases are preserved and queried correctly.
- `test_workflow_rejects_empty_phase_set`: no phases returns `WorkflowError::EmptyPhases`.
- `test_workflow_rejects_duplicate_phase`: duplicate phase IDs return a typed error.
- `test_workflow_rejects_duplicate_edge`: duplicate directed edges return a typed error.
- `test_workflow_rejects_unknown_initial_phase`: undeclared initial phase returns a typed error.
- `test_workflow_rejects_unknown_edge_source`: undeclared source endpoint returns a typed error.
- `test_workflow_rejects_unknown_edge_target`: undeclared target endpoint returns a typed error.
- `test_workflow_rejects_unknown_completion_phase`: undeclared completion endpoint returns a typed error.
- `test_workflow_allows_only_declared_reverse_and_self_edges`: configured reverse/self edges are allowed; their undeclared counterparts are denied.
- Stubs: small in-memory phase and edge fixtures.

### T-006 construction unit tests
- `test_intent_unit_starts_active_at_initial_phase`: a new unit owns the supplied workflow snapshot, exposes ID/species/workflow, starts at the configured initial phase, and has empty history.
- `test_intent_unit_owns_workflow_snapshot`: the unit's immutable topology equals the validated workflow used at construction and requires no external registry.
- `test_intent_unit_identity_accessors_are_stable`: repeated reads preserve immutable identity, workflow, and species values.
- Stubs: fixed IDs and a valid workflow fixture.

### T-007 transition unit tests
- `test_allowed_transition_moves_and_appends_record`: valid movement changes phase and appends one record with sequence/from/to.
- `test_disallowed_transition_is_atomic`: undeclared edge returns a typed error and a before/after snapshot remains equal.
- `test_unknown_target_transition_is_atomic`: unknown target returns a typed error without mutation.
- `test_configured_reverse_transition_succeeds`: an explicitly declared reverse edge is honored.
- `test_transition_history_preserves_order`: multiple valid moves produce contiguous sequence numbers in request order.
- `test_transition_preserves_identity`: ID, workflow, and species remain unchanged across valid moves.
- Stubs: workflow fixtures with forward, reverse, and self edges; fixed IDs.

### T-008 completion unit tests
- `test_completion_from_eligible_phase_is_terminal`: completion changes status and appends one correctly sequenced final-phase record.
- `test_completion_from_ineligible_phase_is_atomic`: typed rejection leaves status, phase, and history unchanged.
- `test_second_completion_is_rejected_without_mutation`: repeated completion returns `AlreadyCompleted`.
- `test_transition_after_completion_is_rejected_without_mutation`: terminal units cannot move.
- `test_completion_preserves_identity_and_species`: ID, workflow, and species remain readable and unchanged.
- Stubs: workflows with explicit eligible and ineligible phases.

### T-009 scalar and workflow serialization unit tests
- `test_identifier_and_vocabulary_semantic_round_trip`: JSON round trips preserve all opaque scalar values.
- `test_workflow_semantic_round_trip`: JSON round trip preserves validated topology and completion points.
- `test_serialization_rejects_malformed_identifier`: malformed UUID payload fails.
- `test_serialization_rejects_blank_vocabulary`: blank validated text payload fails.
- `test_serialization_rejects_empty_workflow`: an empty phase list fails normal topology validation.
- `test_serialization_rejects_duplicate_phase_or_edge`: duplicated topology input fails normal validation.
- `test_serialization_rejects_unknown_initial_phase`: an undeclared initial phase fails.
- `test_serialization_rejects_unknown_edge_or_completion_phase`: unknown endpoints fail.
- Stubs: fixed semantic fixtures and deliberately tampered JSON; no external store.

### T-010 Intent Unit serialization unit tests
- `test_active_intent_semantic_round_trip`: JSON round trip preserves active state, owned workflow, and ordered transition history.
- `test_completed_intent_semantic_round_trip`: JSON round trip preserves terminal state and completion record.
- `test_serialization_rejects_inconsistent_lifecycle_history`: broken sequences, discontinuous transitions, or state/history disagreement fail.
- `test_serialization_rejects_disallowed_recorded_edge`: a history edge absent from the owned workflow fails.
- `test_serialization_rejects_invalid_completion_record`: ineligible completion or records after completion fail.
- Stubs: fixed semantic fixtures and deliberately tampered JSON; no external store.

### T-011 documentation checks
- `test_readme_documents_vocabulary_and_sprint_zero_exclusions`: inspect README headings/content for the promised model and deferrals.
- Stubs: none.

### T-012 public example checks
- `test_documented_lifecycle_example`: `cargo test --doc --workspace` compiles and runs the public example.
- Stubs: none.

## Integration Tests

### Domain configuration integration
- `test_custom_workflow_configuration_composes_domain_values`: public vocabulary and workflow APIs construct an arbitrary caller-named topology without exported default phases.

### Lifecycle integration
- `test_intent_lifecycle_create_transition_complete`: create a unit, traverse two declared edges, complete it, and assert stable ID/species/workflow, ordered records, final phase, and terminal behavior using only the public API.
- `test_failed_operations_are_atomic_and_recoverable`: attempt unknown, undeclared, and ineligible-completion operations; verify the aggregate is unchanged after each, then successfully continue the valid lifecycle.
- `test_explicit_rework_cycle_is_honored`: move forward, follow a configured reverse edge, then finish without any built-in forward-only policy.

### Serialization integration
- `test_serialize_restore_and_continue_lifecycle`: serialize after one transition, restore through validated deserialization, continue to completion, and compare the final semantic aggregate.
- `test_tampered_serialized_aggregate_is_rejected`: corrupt topology and lifecycle invariants independently and require deserialization failure.

## End-to-End Tests

- **Status:** not-yet-possible
- **Unlocked by:** Sprint 1 if its research selects a runnable CLI or service/API adapter; otherwise the first later adapter sprint. That sprint can exercise mock-real workflow input through create/move/complete operations to observable output. Blockchain and UI E2E remain deferred until those platforms are explicitly selected.

## Test Artifact Locations

- Module unit tests: `crates/cubikan-core/src/id.rs`, `vocabulary.rs`, `workflow.rs`, and `intent_unit.rs` under `#[cfg(test)]` modules.
- Public-API integration tests: `crates/cubikan-core/tests/lifecycle.rs`.
- Serialization integration tests: `crates/cubikan-core/tests/serialization.rs`.
- Repository and documentation checks: commands recorded in `sprints/s0/sprint-tests/` and the Test Report; no persistent custom harness is required.

## Final Quality Gates

- `cargo fmt --all -- --check` must pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass.
- `cargo test --workspace --all-targets` must pass.
- `cargo test --doc --workspace` must pass.
