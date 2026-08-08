# Sprint 1 Research Report

## Intents Reviewed

- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — revised; relevance: selects the runnable lifecycle and E2E boundary for this sprint; current state: planned
- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — selected; relevance: supplies the realized lifecycle invariants that the adapter must delegate to; current state: realized

## 1. Sprint Goal

Expose the realized `cubikan-core` lifecycle through the smallest honest runnable boundary: a one-shot, versioned JSON CLI that accepts a complete caller-defined in-memory scenario, delegates all workflow and lifecycle rules to the core, and makes both success and typed failure observable through an actual process. The sprint does not introduce persistence, networking, UI, authorization, blockchain, KPI, naming, or deployment policy.

## 2. Existing Code Survey

| File | Relevance | Notes |
|------|-----------|-------|
| `Cargo.toml` | high | The virtual workspace currently contains only `cubikan-core`; Sprint 1 can add one adapter member without changing the core boundary. |
| `crates/cubikan-core/Cargo.toml` | medium | The core already uses `serde` and UUID support; the adapter only needs the core, `serde`, and `serde_json`. |
| `crates/cubikan-core/src/lib.rs` | high | Exports the constructors, accessors, records, and typed errors needed by an external adapter. |
| `crates/cubikan-core/src/id.rs` | high | Supports caller-supplied parsed IDs for deterministic scenarios and generated UUID v4 IDs when omitted. |
| `crates/cubikan-core/src/vocabulary.rs` | high | Validates caller-defined workflow IDs, phase IDs, and species without imposing a taxonomy. |
| `crates/cubikan-core/src/workflow.rs` | high | Builds validated arbitrary directed workflows and exposes typed topology failures. |
| `crates/cubikan-core/src/intent_unit.rs` | high | Owns atomic transitions, completion, terminal state, and ordered lifecycle history; it intentionally has no persistence abstraction. |
| `crates/cubikan-core/tests/lifecycle.rs` | high | Demonstrates public-API lifecycle journeys and failure atomicity that adapter tests must exercise through the process boundary. |
| `crates/cubikan-core/tests/serialization.rs` | medium | Confirms validated core round trips, while the README keeps that representation provisional rather than an adapter contract. |
| `README.md` | high | Defines the current product boundary and explicitly defers persistence, UI, naming, KPI, and blockchain choices. |
| `docs/intents/INT-0001-chain-agnostic-intent-lifecycle-core.md` | high | Requires the adapter to preserve the realized chain-agnostic core rather than reimplement its policy. |
| `docs/intents/INT-0002-runnable-lifecycle-adapter.md` | high | Carries the runnable-boundary outcome and acceptance criteria advanced by Sprint 1. |
| `docs/work/tasks.md` | high | T-101 is the sole backlog task and identifies boundary selection plus lifecycle E2E coverage. |
| `docs/sprints/s0/sprint-tests/e2e-tests.md` | high | Records why Sprint 0 library integration tests were not true E2E and names a runnable adapter as the unlocking boundary. |

## 3. External Sources

- [Cargo integration tests](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#integration-tests) — Cargo automatically builds binary targets for integration tests and exposes each executable through `CARGO_BIN_EXE_<name>`, enabling a process-level E2E without a helper crate.
- [`std::process::Command`](https://doc.rust-lang.org/std/process/struct.Command.html) — the standard library can launch the built CLI and explicitly pipe its standard input, output, and error streams.
- [`serde_json::from_reader`](https://docs.rs/serde_json/latest/serde_json/fn.from_reader.html) — supports decoding one request directly from standard input; the test must close the input stream so the reader observes EOF.
- [`serde_json::to_writer`](https://docs.rs/serde_json/latest/serde_json/fn.to_writer.html) — supports writing a structured response directly to standard output without an intermediate string contract.
- [Axum crate documentation](https://docs.rs/axum/latest/axum/) — even a minimal HTTP alternative introduces a router, Tokio listener, and server runtime, confirming that it is a broader boundary than the current intent requires.

## 4. Risks, Unknowns, Dependencies

- **Risk:** Reusing serialized `Workflow` or `IntentUnit` values as the public CLI protocol would accidentally elevate the core crate's provisional representation into an adapter compatibility promise. Adapter-owned request and response DTOs must convert through public constructors and accessors.
- **Risk:** Human-readable domain error text is not a stable machine contract. The adapter must exhaustively map public typed error variants to adapter-local codes and keep display messages informational.
- **Risk:** Batch failure behavior could be ambiguous. The runner should fail fast, identify the one-based operation number, and expose the state after earlier successful operations while relying on the core's per-operation atomicity for the rejected operation.
- **Risk:** A local CLI can currently read unbounded standard input. Resource limits are future hardening work and the Sprint 1 artifact must not be described as a production network service.
- **Unknown:** No product evidence chooses durable sessions, files, a database, a service, Electron IPC, or a blockchain. The one-shot request therefore owns the complete in-memory lifecycle and makes none of those choices.
- **Dependency:** Add one workspace crate depending only on `cubikan-core`, `serde`, and `serde_json`; no CLI parser, async runtime, HTTP framework, database, or test-process helper is required.
- **Dependency:** A deterministic E2E fixture needs an optional caller-supplied Intent Unit ID; omitted IDs continue to use the core's UUID v4 generator without promising ordering.

## 5. Recommended Approach

Primary: add a `cubikan-cli` workspace package with a `cubikan` binary. The binary reads one protocol-versioned scenario from standard input: adapter-owned workflow fields, species, an optional fixed Intent Unit ID, and ordered tagged `transition` or `complete` actions. A deterministic library runner converts text through the core's validated constructors, executes actions in order, and builds an adapter-owned success or error envelope. The binary emits exactly one JSON document plus a newline and exits `0` on success or a documented nonzero code on parse, validation, or lifecycle rejection. Process tests use Cargo's supplied executable path and the standard library only.

Alternative considered: separate `create`, `transition`, and `complete` CLI invocations. Rejected because the repository has no persistence or session model; carrying aggregate snapshots between processes would either expose provisional core serialization or invent a durable state boundary.

Alternative considered: an HTTP service. Rejected for this sprint because it adds runtime, routing, network, deployment, and service-state decisions without improving the lifecycle evidence required by INT-0002.

Alternative considered: Electron or blockchain adapters. Rejected because they require unresolved UI, packaging, IPC, chain, trust, storage, key, and finality policy and would make the sprint materially less reversible.

Rationale: a one-shot JSON CLI is independently runnable and externally observable, supports a genuine configure → create → transition → complete process test, and remains a thin adapter over the existing domain. Its lack of cross-process state is explicit and accurate for the current repository.

## Artifacts

- None.
