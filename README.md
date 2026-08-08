# CubiKan

CubiKan is an exploration of blockchain-backed workflow coordination. Its core
concept is an **Intent Unit**: a uniquely identified unit of work that occupies a
caller-defined process phase until it reaches terminal completion.

## Current scope

Sprint 0 provides `cubikan-core`, a chain-agnostic Rust library for defining and
validating Intent Unit lifecycles. It deliberately establishes the domain rules
before selecting a blockchain, persistence layer, service boundary, or user
interface.

## Core model

- An `IntentUnitId` is an opaque UUID-backed identity. Locally generated IDs use
  UUID v4 and carry no chronological ordering contract.
- An `IntentSpecies` is caller-defined text preserved on the Intent Unit for
  future completion and naming policy.
- A `Workflow` contains caller-declared phases, an initial phase, exact directed
  transition edges, and zero or more completion-eligible phases.
- Each Intent Unit owns an immutable validated workflow snapshot, so its
  lifecycle does not depend on a mutable external registry.
- A successful transition changes the current phase and appends a deterministic,
  one-based record. Undeclared edges and unknown phases leave the unit unchanged.
- Completion is allowed only at configured phases, appends one final record, and
  makes the unit terminal.
- Serialized workflows and Intent Units are restored through the same validation
  rules as live operations. JSON is currently a test format, not a stable wire
  contract.

The library does not export default Kanban phase names. A caller can declare a
simple forward workflow, explicit rework/backward edges, self edges, or arbitrary
custom phase labels.

## Species provenance and future naming

Sprint 0 preserves an Intent Unit's immutable species through every transition
and after completion. It does **not** define a completed-unit naming grammar or
parent/child lineage model. Those are product decisions that must be specified
before the core exposes them as stable behavior.

## Development

The workspace requires a current Rust toolchain with Rust 2024 edition support.
Run the quality gates from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo test --doc --workspace
```

## Explicit Sprint 0 exclusions

The current core does not choose or implement:

- a blockchain, network, smart contract, or cryptographic audit proof;
- database persistence, workflow registries, or workflow migration/versioning;
- a CLI, service/API, Electron UI, or other runnable application boundary;
- ownership, authorization, privacy, concurrency, or multi-user conflict rules;
- KPI evaluation or automatic transition authorization;
- default phase topology, completed-unit naming syntax, or parent/child lineage;
- stable JSON field names or cross-version wire-schema compatibility.

These boundaries keep the domain foundation testable and allow later adapters to
be selected through evidence rather than embedded assumptions.

## License

CubiKan is available under the [MIT License](LICENSE).
