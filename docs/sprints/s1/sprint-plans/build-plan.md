Finalized - DO NOT EDIT

# Sprint 1 Build Plan

## Intents

- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — state: planned; acceptance criteria covered: one-shot CLI selection, versioned caller-defined input and typed outcomes, process-level lifecycle E2E, core invariant delegation, and explicit platform/policy exclusions

## Schema Tree

- Expose the realized lifecycle through a reversible batch JSON CLI
  - Workspace boundary
    - T-101: Scaffold the `cubikan-cli` workspace package
  - Adapter-owned protocol
    - T-102: Define strict versioned request and response DTOs
  - Core delegation
    - T-103: Construct validated scenarios and map setup failures
    - T-104: Execute ordered lifecycle operations and expose snapshots
  - Runnable boundary
    - T-105: Implement the generic JSON stream runner
    - T-106: Expose and process-test the `cubikan` binary
  - Consumer guidance
    - T-107: Document the CLI contract and Sprint 1 boundaries

## Protocol Version 1 Contract

The strict request fields are `protocol_version`, `workflow`, `intent_unit`,
and `operations`. The workflow contains `id`, `phases`, `initial_phase`,
`edges` (`from` and `to`), and `completion_phases`. The Intent Unit input
contains optional `id` and required `species`. Operations use a `type` tag:
`transition` carries `target`, while `complete` has no additional field.

Every response contains `protocol_version: 1` and an `outcome` tag. A success
contains `intent_unit`. An error contains `error` and contains `intent_unit`
only for a lifecycle rejection. `error` always contains `code` and `message`,
may contain `field` for field-specific setup failures, and contains the
one-based `operation_number` only for lifecycle failures. Messages are
informational; field names, presence rules, and codes are the machine contract
for experimental protocol version 1.

The adapter-owned Intent Unit snapshot fields are `id`, `species`,
`workflow_id`, `phase`, lowercase `status`, and ordered `history`. History uses
a `type` tag: `transition` contains `sequence`, `from`, and `to`; `completion`
contains `sequence` and `phase`.

| Failure source | Exact `code` | Class / exit | Context |
|----------------|--------------|--------------|---------|
| JSON syntax or unexpected EOF | `invalid_json` | request / 2 | no snapshot; no operation number |
| JSON data shape or unknown field | `invalid_request` | request / 2 | no snapshot; no operation number |
| Unsupported `protocol_version` | `unsupported_protocol_version` | request / 2 | no snapshot; no operation number |
| `VocabularyError::Blank` | `blank_value` | request / 2 | exact `field`; no snapshot |
| `ParseIntentUnitIdError` | `invalid_intent_unit_id` | request / 2 | `intent_unit.id`; no snapshot |
| `WorkflowError::EmptyPhases` | `workflow_empty_phases` | request / 2 | no snapshot |
| `WorkflowError::DuplicatePhase` | `workflow_duplicate_phase` | request / 2 | no snapshot |
| `WorkflowError::UnknownInitialPhase` | `workflow_unknown_initial_phase` | request / 2 | no snapshot |
| `WorkflowError::UnknownEdgeSource` | `workflow_unknown_edge_source` | request / 2 | no snapshot |
| `WorkflowError::UnknownEdgeTarget` | `workflow_unknown_edge_target` | request / 2 | no snapshot |
| `WorkflowError::DuplicateEdge` | `workflow_duplicate_edge` | request / 2 | no snapshot |
| `WorkflowError::UnknownCompletionPhase` | `workflow_unknown_completion_phase` | request / 2 | no snapshot |
| `WorkflowError::DuplicateCompletionPhase` | `workflow_duplicate_completion_phase` | request / 2 | no snapshot |
| `TransitionError::AlreadyCompleted` | `transition_already_completed` | lifecycle / 3 | operation number and snapshot |
| `TransitionError::UnknownTarget` | `transition_unknown_target` | lifecycle / 3 | operation number and snapshot |
| `TransitionError::NotAllowed` | `transition_not_allowed` | lifecycle / 3 | operation number and snapshot |
| `CompletionError::AlreadyCompleted` | `completion_already_completed` | lifecycle / 3 | operation number and snapshot |
| `CompletionError::PhaseNotEligible` | `completion_phase_not_eligible` | lifecycle / 3 | operation number and snapshot |

An input/output failure that prevents a complete protocol response is an
operational failure, not an error code: the process exits 1 and writes a
best-effort diagnostic to stderr.

## Execution Sequence

### T-101: Scaffold the `cubikan-cli` workspace package
- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Touches:** `Cargo.toml`, `Cargo.lock`, `crates/cubikan-cli/Cargo.toml`, `crates/cubikan-cli/src/lib.rs`
- **Depends on:** (none)
- **Acceptance criterion:** Research selects a one-shot batch JSON CLI as the smallest reversible runnable boundary, while unselected platforms and policies remain outside it.
- **Success criterion (EARS):**
  - **T-101-E1 — WHEN** Cargo metadata is resolved from the repository root, **THEN** the workspace **SHALL** contain the existing `cubikan-core` library and a new `cubikan-cli` package.
  - **T-101-E2 — WHEN** the adapter package manifest and all targets are checked, **THEN** `cubikan-cli` **SHALL** compile with warnings denied and **SHALL** have only `cubikan-core`, `serde`, and `serde_json` as direct dependencies.
- **Notes:** Do not change `cubikan-core` or add a parser framework, async runtime, HTTP stack, database, UI, or blockchain dependency. The executable target is added only after the library pipeline exists.

### T-102: Define the adapter-owned version 1 JSON contract
- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Touches:** `crates/cubikan-cli/src/lib.rs`, `crates/cubikan-cli/src/protocol.rs`
- **Depends on:** T-101
- **Acceptance criterion:** The CLI accepts one versioned scenario containing caller-defined workflow, species, optional fixed Intent Unit ID, and ordered lifecycle operations, and emits one versioned success or typed failure response.
- **Success criterion (EARS):**
  - **T-102-E1 — WHEN** a structurally valid version 1 request is decoded, **THEN** the protocol layer **SHALL** strictly preserve the caller's workflow ID, phases, initial phase, directed edges, completion phases, species, optional ID, and ordered tagged operations in adapter-owned DTOs while rejecting unknown fields.
  - **T-102-E2 — WHEN** a success or failure response is serialized, **THEN** the protocol layer **SHALL** emit an adapter-owned envelope with `protocol_version: 1`, exactly one snake_case outcome, lowercase status, tagged history, and a machine-readable error code; setup failures **SHALL** include field context only when applicable and no snapshot, while lifecycle failures **SHALL** include a one-based operation number and Intent Unit snapshot.
- **Notes:** Wire DTOs use strings, scalars, vectors, and adapter enums only. Never deserialize a request into `Workflow` or serialize `IntentUnit`, `IntentUnitStatus`, or `LifecycleRecord` directly; the core Serde layout remains provisional.

### T-103: Construct validated core scenarios and map setup failures
- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Touches:** `crates/cubikan-cli/src/lib.rs`, `crates/cubikan-cli/src/protocol.rs`, `crates/cubikan-cli/src/execution.rs`
- **Depends on:** T-102
- **Acceptance criterion:** The adapter accepts caller-defined input and delegates lifecycle invariants to `cubikan-core` rather than duplicating or weakening them.
- **Success criterion (EARS):**
  - **T-103-E1 — WHEN** a valid version 1 scenario supplies a syntactically valid fixed UUID, **THEN** the executor **SHALL** construct all vocabulary, edges, workflow topology, species, and the Intent Unit through public `cubikan-core` constructors while preserving caller text and the supplied ID exactly.
  - **T-103-E2 — WHEN** a valid version 1 scenario omits its Intent Unit ID, **THEN** the executor **SHALL** call `IntentUnitId::generate()` and expose a non-nil UUID v4 ID without adding an ordering contract.
  - **T-103-E3 — WHEN** the protocol version is unsupported or an ID or validated text field is invalid, **THEN** the executor **SHALL** return the matching adapter-owned setup error code and field context with no Intent Unit snapshot.
  - **T-103-E4 — WHEN** `Workflow::new` returns any public `WorkflowError` variant, **THEN** the executor **SHALL** exhaustively map that variant to its matching adapter-owned workflow error code with no Intent Unit snapshot.
- **Notes:** Preserve nonblank text exactly and accept every UUID syntax that the core accepts, including fixed non-v4 values. Do not add policy against empty operation lists, empty completion sets, custom phases, reverse edges, or self edges.

### T-104: Execute ordered lifecycle operations and expose adapter snapshots
- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Touches:** `crates/cubikan-cli/src/protocol.rs`, `crates/cubikan-cli/src/execution.rs`
- **Depends on:** T-103
- **Acceptance criterion:** The adapter exposes typed lifecycle outcomes while delegating transitions and completion to `cubikan-core`.
- **Success criterion (EARS):**
  - **T-104-E1 — WHEN** ordered declared transitions are followed by eligible completion, **THEN** the executor **SHALL** call `transition_to` and `complete` in request order and return a completed adapter-owned snapshot containing stable ID, species, workflow ID, final phase, lowercase status, and exactly sequenced history.
  - **T-104-E2 — WHEN** a valid scenario has no operations, **THEN** the executor **SHALL** return the newly created active initial snapshot with empty history.
  - **T-104-E3 — WHEN** the caller declares a reverse or self edge and requests it, **THEN** the executor **SHALL** honor that topology through the core without imposing forward-only adapter policy.
  - **T-104-E4 — WHEN** a transition or completion is rejected by the core, **THEN** the executor **SHALL** stop before later operations, exhaustively map the typed lifecycle error, report the rejected operation's one-based number, and return a snapshot containing earlier successes but no mutation from the rejected operation.
  - **T-104-E5 — WHEN** an operation is attempted after completion, **THEN** the executor **SHALL** expose the operation-specific already-completed code and preserve the completed snapshot unchanged.
- **Notes:** A scenario is fail-fast, not transactional: successful earlier operations remain visible in the failure snapshot, while the rejected core operation is atomic. Human-readable messages are informational and must not be parsed as machine contracts.

### T-105: Implement the generic JSON stream runner
- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Touches:** `crates/cubikan-cli/src/lib.rs`, `crates/cubikan-cli/src/runner.rs`, `crates/cubikan-cli/tests/runner.rs`
- **Depends on:** T-104
- **Acceptance criterion:** The CLI accepts the versioned scenario on standard input and exposes one externally observable success or typed failure response.
- **Success criterion (EARS):**
  - **T-105-E1 — WHEN** the runner receives a valid scenario through a `Read`, **THEN** it **SHALL** write exactly one compact success JSON document plus one newline to the supplied `Write` and return success classification.
  - **T-105-E2 — WHEN** JSON is syntactically incomplete or malformed, or its data shape is invalid, **THEN** the runner **SHALL** write exactly one `invalid_json` or `invalid_request` response plus one newline, include no Intent Unit snapshot, and return request-rejection classification.
  - **T-105-E3 — WHEN** version, vocabulary, identifier, or workflow setup is rejected, **THEN** the runner **SHALL** write exactly one typed failure response without state and return request-rejection classification.
  - **T-105-E4 — WHEN** lifecycle execution is rejected, **THEN** the runner **SHALL** write exactly one typed failure response with its operation number and partial snapshot and return lifecycle-rejection classification.
  - **T-105-E5 — WHEN** the input stream fails or a response cannot be written completely, **THEN** the runner **SHALL** return the underlying operational error rather than report a false protocol outcome.
- **Notes:** Normal outcomes reserve stdout for the JSON envelope. Operational failures where a response cannot be guaranteed are not modeled domain rejections.

### T-106: Expose and process-test the `cubikan` executable
- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Touches:** `crates/cubikan-cli/Cargo.toml`, `crates/cubikan-cli/src/lib.rs`, `crates/cubikan-cli/src/main.rs`, `crates/cubikan-cli/tests/cli_e2e.rs`, `crates/cubikan-cli/tests/fixtures/lifecycle-success-v1.json`
- **Depends on:** T-105
- **Acceptance criterion:** An automated process-level E2E test drives configure → create → transition → complete and asserts the externally visible JSON result and exit status.
- **Success criterion (EARS):**
  - **T-106-E1 — WHEN** the checked-in version 1 lifecycle fixture is piped to the Cargo-built `cubikan` executable, **THEN** the process **SHALL** exit `0`, emit one newline-terminated success response whose unit has the expected fixed ID, completed state, final phase, and ordered history, and emit nothing to stderr.
  - **T-106-E2 — WHEN** the executable receives a malformed request, **THEN** the process **SHALL** exit `2`, emit one version 1 typed request failure to stdout, and emit nothing to stderr.
  - **T-106-E3 — WHEN** the executable receives a lifecycle rejection after an earlier successful operation, **THEN** the process **SHALL** exit `3`, emit one typed failure with the one-based operation number and unchanged partial snapshot to stdout, and emit nothing to stderr.
  - **T-106-E4 — WHEN** the runner returns an operational input or output error, **THEN** the process shell **SHALL** map it to exit `1`, emit a best-effort diagnostic to stderr, and never classify it as a successful protocol outcome.
- **Notes:** Keep the status/error-to-exit mapping in a small injectable process-shell function called by `main`, so the exit `1` path can be verified without depending on a platform-specific way to break the real process streams. Use `env!("CARGO_BIN_EXE_cubikan")`, `std::process::Command`, and piped standard streams for normal process E2E; add no process-test helper dependency.

### T-107: Document the CLI contract and Sprint 1 boundaries
- **Intent:** [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md)
- **Touches:** `README.md`, `crates/cubikan-cli/README.md`
- **Depends on:** T-106
- **Acceptance criterion:** The one-shot boundary and its exclusions remain explicit, and the adapter's externally visible contract is usable without implying broader product policy.
- **Success criterion (EARS):**
  - **T-107-E1 — WHEN** a consumer reads the CLI documentation, **THEN** it **SHALL** describe the invocation, complete version 1 request and response shapes, exit codes, error-code semantics, fail-fast partial-state behavior, and a configure-create-transition-complete example.
  - **T-107-E2 — WHEN** Sprint 1 evidence and consumer guidance are reviewed, **THEN** they **SHALL** explain why the one-shot CLI is the smallest reversible boundary, mark its protocol experimental, disclose that local standard input is not size-limited, defer resource limiting as future hardening before production exposure, and explicitly exclude persistence, networking, deployment, authorization, KPI, naming, blockchain, and UI policy.
- **Notes:** The docs must say that one process owns one in-memory scenario and that separate durable sessions require a future intent.
