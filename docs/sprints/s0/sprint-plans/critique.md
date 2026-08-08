# Plan Critique — Sprint 0

## Concerns

### C-001: Identifier tests were not fully traceable to EARS criteria
- **Where:** `build-plan.md` T-003 / `test-plan.md` T-003
- **Quote:** "an `IntentUnitId` is generated, formatted, and parsed"
- **Failure mode:** plan-test-mismatch
- **Why it matters:** UUID version, non-nil generation, and distinct-generation checks appeared in notes/tests rather than atomic criteria.
- **Suggested response:** fix-in-plan
- **Primary response:** `fix-in-plan` — T-003 now has separate EARS clauses for non-nil v4 generation, two-value distinctness, valid parse/format round trips, and malformed parsing.

### C-002: Serialization task was not elementary
- **Where:** `build-plan.md` former T-009
- **Quote:** "Add invariant-preserving format-neutral serialization"
- **Failure mode:** granularity
- **Why it matters:** One task crossed every scalar, workflow, active/completed aggregate, and lifecycle-history corruption concern.
- **Suggested response:** fix-in-plan
- **Primary response:** `fix-in-plan` — serialization is split into T-009 for scalars/workflows and T-010 for Intent Unit restoration/history validation.

### C-003: Documentation and executable examples were combined
- **Where:** `build-plan.md` former T-010
- **Quote:** "Document the core vocabulary, example, and exclusions"
- **Failure mode:** granularity
- **Why it matters:** README content and a compiling doctest are distinct deliverables and success boundaries.
- **Suggested response:** fix-in-plan
- **Primary response:** `fix-in-plan` — README documentation is T-011 and the executable public example is T-012.

### C-004: Workflow ownership was undefined
- **Where:** `build-plan.md` T-006 through T-010
- **Quote:** "under its own workflow"
- **Failure mode:** hidden-dep
- **Why it matters:** Identity alone cannot disambiguate workflows with the same ID and different topology during transitions or restoration.
- **Suggested response:** fix-in-plan
- **Primary response:** `fix-in-plan` — each Intent Unit now owns an immutable validated workflow snapshot, and serialization/history validation uses that snapshot without an external registry.

### C-005: Several EARS criteria bundled independent behaviors
- **Where:** `build-plan.md` T-001, T-005, T-009, and T-010
- **Quote:** "workflow is empty, repeats a phase or edge, or references an unknown..."
- **Failure mode:** EARS-vague
- **Why it matters:** Compound triggers can hide partial implementation and weaken one-clause-to-one-test traceability.
- **Suggested response:** fix-in-plan
- **Primary response:** `fix-in-plan` — decisions, workflow validation, scalar/topology deserialization, and lifecycle restoration now use separate measurable EARS clauses aligned with named tests.

### C-006: Deserialization did not cover every constructor invariant
- **Where:** `test-plan.md` former T-009
- **Quote:** "unknown edge/completion endpoints fail"
- **Failure mode:** plan-test-mismatch
- **Why it matters:** Constructor tests do not prove that deserialization rejects empty/duplicate topology or unknown initial phases.
- **Suggested response:** fix-in-plan
- **Primary response:** `fix-in-plan` — T-009 tests now cover empty workflows, duplicate phases/edges, unknown initial phases, and unknown edge/completion endpoints; T-010 covers lifecycle corruption classes.

### C-007: Test artifact locations were absent from the plan
- **Where:** `build-plan.md` touches versus `test-plan.md` integration checks
- **Quote:** "Integration Tests"
- **Failure mode:** plan-test-mismatch
- **Why it matters:** Without locations, the later Test phase could implement an inconsistent or ad hoc harness.
- **Suggested response:** fix-in-plan
- **Primary response:** `reject` — the Sprint Loop protocol intentionally creates tests in the separate Test phase, not as Build tasks. The test plan now nevertheless names module, integration, serialization, and command-check locations to remove ambiguity.

### C-008: Verification did not enforce repository policy
- **Where:** `build-plan.md` / `test-plan.md` T-002
- **Quote:** "cargo check --workspace --all-targets"
- **Failure mode:** hidden-dep
- **Why it matters:** Ordinary Cargo checks do not deny warnings, and the repository requires formatting and Clippy verification.
- **Suggested response:** fix-in-plan
- **Primary response:** `fix-in-plan` — compile checks deny warnings and the test plan adds mandatory `cargo fmt`, `cargo clippy -D warnings`, workspace test, and doctest gates.

### C-009: Foundational decisions were bundled into one acceptance criterion
- **Where:** `build-plan.md` T-001
- **Quote:** "record accepted decisions for a chain-agnostic Rust core..."
- **Failure mode:** ignored-ADR
- **Why it matters:** Independently reversible choices should remain independently reviewable before dependent implementation begins.
- **Suggested response:** fix-in-plan
- **Primary response:** `fix-in-plan` — T-001 now has separate clauses and checks for core boundary, workflow ownership, identifier contract, serialization contract, and product deferrals, while remaining one coherent ADR-log commit.

### C-010: Lineage was weakened to species provenance without explicit acceptance
- **Where:** `research-report.md` and `build-plan.md` T-006/T-008
- **Quote:** "The immutable species is the minimal provenance"
- **Failure mode:** missing-risk
- **Why it matters:** Parent/child lineage and completed-name semantics are not defined, so silently treating species as the entire model could harden a guess.
- **Suggested response:** defer-with-rationale
- **Primary response:** `defer-with-rationale` — `sprint-meta.md` records the unresolved product question, T-001 records the deferral, and T-011 must document that Sprint 0 preserves species only and does not define parent/child lineage or naming grammar.

### C-011: E2E deferral lacked a tracked handoff
- **Where:** `test-plan.md` End-to-End Tests
- **Quote:** "the first post-Sprint 0 adapter sprint"
- **Failure mode:** e2e-drift
- **Why it matters:** The deferral is valid, but an executable boundary must remain visible to later planning.
- **Suggested response:** defer-with-rationale
- **Primary response:** `defer-with-rationale` — `sprint-meta.md` records a Sprint 1 adapter candidate, the E2E section names Sprint 1 conditionally, and the public-API lifecycle integration test remains the interim top-level journey.

## Confidence

proceed-with-caveats

All blocking plan defects were fixed. The remaining caveats are explicitly deferred product choices: completed-unit naming/lineage beyond species and selection of a runnable adapter boundary.
