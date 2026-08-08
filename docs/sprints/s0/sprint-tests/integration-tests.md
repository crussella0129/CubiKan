# Sprint 0 Integration Test Results

## Summary

- Result: **6 passed / 0 failed / 6 total**.
- Lifecycle suite: **4 passed / 0 failed** via `cargo test --test lifecycle`.
- Serialization suite: **2 passed / 0 failed** via `cargo test --test serialization`.
- All tests use only the public `cubikan-core` API.
- Stubs, mocks, external services, timing assumptions, and shared mutable state: **none**.

## Domain Configuration and Lifecycle

Source: `crates/cubikan-core/tests/lifecycle.rs`

- `test_custom_workflow_configuration_composes_domain_values` — composes arbitrary caller-named vocabulary, topology, reverse edges, and completion policy without exported defaults.
- `test_intent_lifecycle_create_transition_complete` — creates a unit, traverses both declared edges, completes it, verifies immutable identity/species/workflow, exact ordered history, final phase, terminal state, and rejection after completion.
- `test_failed_operations_are_atomic_and_recoverable` — checks unknown target, undeclared edge, and ineligible completion errors against complete before/after snapshots, then proves a valid transition still succeeds.
- `test_explicit_rework_cycle_is_honored` — traverses a configured forward/reverse cycle and then reaches completion, proving there is no built-in forward-only policy.

Result: **4 passed / 0 failed**.

## Validated Serialization Journeys

Source: `crates/cubikan-core/tests/serialization.rs`

- `test_serialize_restore_and_continue_lifecycle` — serializes an active unit after one transition, restores it through validated deserialization, continues to completion, and compares the complete semantic result with an uninterrupted lifecycle.
- `test_tampered_serialized_aggregate_is_rejected` — independently corrupts embedded workflow topology and lifecycle continuity and requires both payloads to fail deserialization.

Result: **2 passed / 0 failed**.

## Combined Confirmation

`cargo test --workspace --all-targets` also ran all 43 module tests and these 6 integration tests together: **49 passed / 0 failed**.
