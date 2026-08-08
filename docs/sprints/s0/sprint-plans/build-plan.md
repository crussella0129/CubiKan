Finalized - DO NOT EDIT

# Sprint 0 Build Plan

## Schema Tree

- Establish CubiKan's chain-agnostic Rust domain foundation
  - Repository architecture and workspace
    - T-001: Record foundational architecture decisions
    - T-002: Scaffold the Cargo workspace and core crate
  - Domain vocabulary
    - T-003: Implement opaque Intent Unit identifiers
    - T-004: Implement validated textual domain values
  - Caller-configured lifecycle
    - T-005: Implement directed workflow definitions
    - T-006: Implement active Intent Unit construction
    - T-007: Implement guarded phase transitions
    - T-008: Implement terminal completion
  - Safe interchange and consumer guidance
    - T-009: Add validated scalar and workflow serialization
    - T-010: Add validated Intent Unit serialization
    - T-011: Document the core model and Sprint 0 boundaries
    - T-012: Add an executable public lifecycle example

## Execution Sequence

### T-001: Record foundational architecture decisions
- **Touches:** `decisions.md`
- **Depends on:** (none)
- **Success criterion (EARS):**
  - **WHEN** Sprint 0 implementation begins, **THEN** `decisions.md` **SHALL** record a chain-agnostic Rust core boundary and defer blockchain, persistence, API, and UI adapters.
  - **WHEN** workflow ownership is decided, **THEN** `decisions.md` **SHALL** record caller-declared topology and an immutable validated workflow snapshot owned by each Intent Unit.
  - **WHEN** identifier generation is decided, **THEN** `decisions.md` **SHALL** record opaque UUID v4 values without an ordering contract.
  - **WHEN** serialization is introduced, **THEN** `decisions.md` **SHALL** record invariant-preserving deserialization and explicitly mark the wire format provisional.
  - **WHEN** unresolved product semantics are reviewed, **THEN** `decisions.md` **SHALL** defer default phases, KPI evaluation, completed-unit naming syntax, parent/child lineage, authorization, and concurrency.
- **Notes:** Keep each decision independently reviewable within the ADR log before dependent implementation begins.

### T-002: Scaffold the Cargo workspace and `cubikan-core` crate
- **Touches:** `Cargo.toml`, `crates/cubikan-core/Cargo.toml`, `crates/cubikan-core/src/lib.rs`, `.gitignore`
- **Depends on:** T-001
- **Success criterion (EARS):**
  - **WHEN** `cargo metadata --no-deps` runs at the repository root, **THEN** Cargo **SHALL** resolve a virtual workspace containing exactly the `cubikan-core` library member.
  - **WHEN** `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` runs, **THEN** the workspace **SHALL** compile successfully with warnings denied.
- **Notes:** Use resolver 3 and Rust edition 2024. Add narrowly scoped workspace dependencies for `uuid` (v4 + Serde), `serde` derive, and dev-only `serde_json`; forbid unsafe code and ignore `/target/`.

### T-003: Implement opaque Intent Unit identifiers
- **Touches:** `crates/cubikan-core/src/id.rs`, `crates/cubikan-core/src/lib.rs`
- **Depends on:** T-002
- **Success criterion (EARS):**
  - **WHEN** an `IntentUnitId` is generated, **THEN** the identifier component **SHALL** produce a non-nil UUID v4 behind an immutable domain type.
  - **WHEN** two `IntentUnitId` values are generated independently, **THEN** the identifier component **SHALL** produce distinct values as a uniqueness smoke check.
  - **WHEN** valid UUID text is parsed and formatted, **THEN** the identifier component **SHALL** preserve the same UUID value.
  - **WHEN** malformed identifier text is parsed, **THEN** the identifier component **SHALL** return a typed parse error.
- **Notes:** Generate UUID v4 values because uniqueness is required but chronological ordering is not. Expose fixed-value construction for deterministic tests without exposing mutable representation.

### T-004: Implement validated textual domain values
- **Touches:** `crates/cubikan-core/src/vocabulary.rs`, `crates/cubikan-core/src/lib.rs`
- **Depends on:** T-003
- **Success criterion (EARS):**
  - **WHEN** non-blank text is supplied for a `WorkflowId`, `PhaseId`, or `IntentSpecies`, **THEN** the vocabulary component **SHALL** construct the requested opaque value while preserving its exact text.
  - **WHEN** empty or whitespace-only text is supplied, **THEN** the vocabulary component **SHALL** reject it with a typed validation error.
- **Notes:** Arbitrary caller text supports predefined, custom, and KPI-associated concepts without inventing a taxonomy or KPI execution engine.

### T-005: Implement caller-declared directed workflow definitions
- **Touches:** `crates/cubikan-core/src/workflow.rs`, `crates/cubikan-core/src/lib.rs`
- **Depends on:** T-004
- **Success criterion (EARS):**
  - **WHEN** a workflow declares unique phases, a known initial phase, valid directed edges, and known completion-eligible phases, **THEN** the workflow component **SHALL** accept it and permit exactly the declared edges and completion points.
  - **WHEN** a workflow has no declared phases, **THEN** the workflow component **SHALL** reject the definition with `WorkflowError::EmptyPhases`.
  - **WHEN** a workflow repeats a phase or directed edge, **THEN** the workflow component **SHALL** reject the duplicated definition with a typed `WorkflowError`.
  - **WHEN** a workflow references an unknown initial phase, **THEN** the workflow component **SHALL** reject the definition with a typed `WorkflowError`.
  - **WHEN** a workflow edge references an unknown endpoint, **THEN** the workflow component **SHALL** reject the definition with a typed `WorkflowError`.
  - **WHEN** a workflow references an unknown completion phase, **THEN** the workflow component **SHALL** reject the definition with a typed `WorkflowError`.
  - **WHEN** a reverse or self edge is queried, **THEN** the workflow component **SHALL** allow it only when that exact directed edge was declared.
- **Notes:** An empty completion set is valid for non-terminating workflows. Example Kanban names belong only in tests and documentation, never as exported defaults.

### T-006: Implement active Intent Unit construction
- **Touches:** `crates/cubikan-core/src/intent_unit.rs`, `crates/cubikan-core/src/lib.rs`
- **Depends on:** T-005
- **Success criterion (EARS):**
  - **WHEN** an Intent Unit is created from an ID, species, and validated workflow, **THEN** it **SHALL** own an immutable workflow snapshot, begin active at that snapshot's initial phase, expose immutable identity/workflow/species values, and have an empty lifecycle history.
- **Notes:** Snapshot ownership makes later operations deterministic without a workflow registry or same-ID topology ambiguity. The immutable species is the minimal provenance needed by a future completed-unit naming policy; do not add parent-unit lineage, actors, timestamps, ownership, or persistence concerns.

### T-007: Implement guarded phase transitions and append-only records
- **Touches:** `crates/cubikan-core/src/intent_unit.rs`
- **Depends on:** T-006
- **Success criterion (EARS):**
  - **WHEN** an active unit requests an edge declared by its workflow snapshot, **THEN** it **SHALL** move to the target phase and append exactly one monotonically sequenced transition record containing the previous and target phases.
  - **WHEN** an undeclared edge or unknown target is requested, **THEN** the unit **SHALL** return a typed transition error without changing its state or history.
  - **WHEN** a transition succeeds, **THEN** the unit **SHALL** preserve its ID, species, and workflow identity.
- **Notes:** Records are deterministic in-memory domain history, not a durable or cryptographic audit log; omit time and actor fields.

### T-008: Implement terminal completion
- **Touches:** `crates/cubikan-core/src/intent_unit.rs`
- **Depends on:** T-007
- **Success criterion (EARS):**
  - **WHEN** an active unit in a phase marked completion-eligible by its workflow snapshot is completed, **THEN** it **SHALL** enter a terminal completed state and append exactly one sequenced completion record containing its final phase.
  - **WHEN** completion is requested from an ineligible phase, **THEN** the unit **SHALL** return a typed error without changing its state or history.
  - **WHEN** transition or completion is requested after completion, **THEN** the unit **SHALL** return `AlreadyCompleted` without changing state or history.
  - **WHEN** a unit completes, **THEN** its ID, species, and workflow identity **SHALL** remain available unchanged.
- **Notes:** Preserve species explicitly but defer generated completed-unit names until their grammar is defined.

### T-009: Add validated format-neutral serialization for scalars and workflows
- **Touches:** `crates/cubikan-core/src/id.rs`, `crates/cubikan-core/src/vocabulary.rs`, `crates/cubikan-core/src/workflow.rs`
- **Depends on:** T-008
- **Success criterion (EARS):**
  - **WHEN** a valid identifier, vocabulary value, or workflow is serialized and deserialized, **THEN** the reconstructed semantic value **SHALL** equal the original.
  - **WHEN** serialized identifier input contains a malformed UUID, **THEN** deserialization **SHALL** fail instead of constructing an invalid identifier.
  - **WHEN** serialized vocabulary input is blank, **THEN** deserialization **SHALL** fail instead of constructing an invalid value.
  - **WHEN** serialized workflow input has no phases, **THEN** deserialization **SHALL** fail through normal topology validation.
  - **WHEN** serialized workflow input repeats a phase or edge, **THEN** deserialization **SHALL** fail through normal topology validation.
  - **WHEN** serialized workflow input references an unknown initial, edge, or completion phase, **THEN** deserialization **SHALL** fail through normal topology validation.
- **Notes:** Route deserialization through validating constructors or equivalent checked representations. JSON is a test vehicle only; Sprint 0 does not promise a stable JSON field layout.

### T-010: Add validated format-neutral serialization for Intent Units
- **Touches:** `crates/cubikan-core/src/intent_unit.rs`
- **Depends on:** T-009
- **Success criterion (EARS):**
  - **WHEN** a valid active or completed Intent Unit is serialized and deserialized, **THEN** the reconstructed aggregate, including its owned workflow snapshot and ordered history, **SHALL** equal the original.
  - **WHEN** serialized lifecycle input has broken sequence numbers or disallowed/discontinuous transitions, **THEN** deserialization **SHALL** fail instead of constructing an inconsistent aggregate.
  - **WHEN** serialized lifecycle input has invalid completion eligibility or records after completion, **THEN** deserialization **SHALL** fail instead of constructing an inconsistent aggregate.
  - **WHEN** serialized lifecycle state disagrees with its final phase or history, **THEN** deserialization **SHALL** fail instead of constructing an inconsistent aggregate.
- **Notes:** Validate restoration against the embedded workflow snapshot. This history is a domain record, not proof of durable storage or cryptographic integrity.

### T-011: Document the core vocabulary and Sprint 0 boundaries
- **Touches:** `README.md`
- **Depends on:** T-010
- **Success criterion (EARS):**
  - **WHEN** a consumer reads the README, **THEN** it **SHALL** explain Intent Units, species provenance, workflows, directed transitions, completion, development commands, and Sprint 0 exclusions.
- **Notes:** Explicitly defer blockchain/network selection, persistence, CLI/API/Electron UI, authorization, concurrency, KPI enforcement, default workflows, parent/child lineage, completed-name grammar, and stable serialization compatibility.

### T-012: Add an executable public lifecycle example
- **Touches:** `crates/cubikan-core/src/lib.rs`
- **Depends on:** T-011
- **Success criterion (EARS):**
  - **WHEN** `cargo test --doc --workspace` runs, **THEN** the documented create-transition-complete example **SHALL** compile and pass.
- **Notes:** Keep the example caller-configured and library-only; do not imply a default workflow or executable application boundary.
