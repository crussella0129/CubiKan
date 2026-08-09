# CubiKan

CubiKan is an exploration of blockchain-backed workflow coordination. Its core
concept is an **Intent Unit**: a uniquely identified unit of work that occupies a
caller-defined process phase until it reaches terminal completion.

## Current scope

Sprint 0 provides `cubikan-core`, a chain-agnostic Rust library for defining and
validating Intent Unit lifecycles. Sprint 1 adds `cubikan`, an experimental,
stateless JSON command-line adapter that executes one complete caller-defined
lifecycle scenario per process. Neither layer selects a blockchain, persistence
model, service boundary, or user interface.

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
- Every new Intent Unit starts at aggregate-local revision
  `IntentUnitRevision::INITIAL` (numeric `0`). Each accepted conditioned or
  unconditioned transition or completion advances that revision exactly once;
  every rejected operation leaves it unchanged.
- Serialized workflows and Intent Units are restored through the same validation
  rules as live operations, including replay validation of the serialized
  revision. JSON is currently a test format, not a stable wire contract; a
  snapshot from an older schema that omits the required revision fails to
  restore.

The library does not export default Kanban phase names. A caller can declare a
simple forward workflow, explicit rework/backward edges, self edges, or arbitrary
custom phase labels.

## Revision-conditioned lifecycle mutations

`transition_to_if_revision` and `complete_if_revision` provide aggregate-local
optimistic conflict detection using the revision that the caller previously
observed. They compare that value before terminal-state, phase, edge, or
completion-eligibility validation. A stale value therefore returns the
`Conflict(RevisionConflict)` variant of `RevisionedTransitionError` or
`RevisionedCompletionError` even when the requested lifecycle operation would
also be invalid. Matching that variant exposes both `expected()` (the caller's
observation) and `actual()` (the aggregate's current revision), allowing the
caller to reject the command, refresh its view, and decide whether to retry. If
the supplied revision is current, normal domain validation runs and retains its
typed transition or completion error; a successful operation returns the newly
committed revision.

A revision is an opaque version of one Intent Unit, not a clock or a global
sequence. Lifecycle record sequence numbers are one-based positions in that
unit's history; revision values start at zero. Although both currently advance
with each accepted mutation, they are distinct contracts, and callers must not
derive one from the other.

The unconditioned `transition_to` and `complete` methods remain convenient for a
single owner that already has exclusive access to the in-memory aggregate. A
future durable or multi-client adapter must instead receive the caller's
previously observed revision and pass it to the conditioned operation. Reading
the revision inside the adapter immediately before mutation would erase the
stale-observation check. The core comparison alone does not make storage writes
atomic: such an adapter must provide its own durable compare-and-set or
transaction/isolation boundary.

## Runnable JSON adapter

Run the checked-in configure → create → transition → complete example
from the repository root:

```sh
cargo run -p cubikan-cli --bin cubikan < crates/cubikan-cli/tests/fixtures/lifecycle-success-v1.json
```

The process reads one strict version 1 request from standard input and writes one
newline-terminated JSON success or typed error response to standard output. The
runner returns a modeled outcome only after the supplied output writer accepts
the JSON, the newline, and one explicit `flush()`; a failure at any of those
steps is an operational error (`1`) with a best-effort stderr diagnostic. This
checks only the supplied writer's flush contract, not durable or acknowledged
delivery. Other exit codes distinguish request or setup rejection (`2`) and
lifecycle rejection (`3`). See the
[`cubikan` protocol reference](crates/cubikan-cli/README.md) for complete request,
response, error-code, and partial-state semantics.

Raw request ingestion is capped at 1 MiB (`1_048_576` bytes), including JSON
whitespace. An input whose required next byte would exceed that ceiling receives
one `request_too_large` response and exit `2` before JSON classification. The
ceiling is a compile-time source constant, not a runtime setting. It bounds the
retained raw request payload only; it is not a total-memory guarantee or a claim
that the adapter is production-ready.

This is intentionally one-shot and in-memory. It does not preserve state between
invocations, and its experimental adapter-owned protocol is not a cross-version
compatibility promise.

## Species provenance and future naming

Sprint 0 preserves an Intent Unit's immutable species through every transition
and after completion. It does **not** define a completed-unit naming grammar or
parent/child lineage model. Those are product decisions that must be specified
before the core exposes them as stable behavior.

## Development

The workspace requires current-stable Rust with Rust 2024 edition support. The
[Rust CI](.github/workflows/ci.yml) workflow runs on GitHub-hosted Ubuntu for
pull requests targeting `dev` or `main` and pushes to `dev` or `main`.
Reproduce its five quality gates from the repository root in this order:

```sh
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --all-features -- -D warnings
RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets
cargo +stable test --workspace --all-targets
cargo +stable test --doc --workspace
```

The workflow produces a GitHub status for review. It does not configure
required branch protection or authorize a merge; retained human merge approval
remains required. This CI boundary intentionally adds no caches, artifacts,
coverage/security scanners, secrets, releases/deployment, automatic merge
(auto-merge), MSRV, or OS/toolchain matrices. These CI nonclaims do not change
the product boundaries below.

## Explicit current exclusions

The current core does not choose or implement:

- a blockchain, network, smart contract, or cryptographic audit proof;
- database persistence, durable compare-and-set, transaction/isolation, workflow
  registries, or workflow migration/versioning;
- a service/API, Electron UI, or durable interactive application session;
- ownership, authorization, or privacy policy;
- locking, synchronization, cross-Intent-Unit atomicity, or durable multi-client
  coordination;
- idempotency keys, automatic retry behavior, or retry-safety guarantees;
- clocks, timestamps, or global ordering across different Intent Units;
- KPI evaluation or automatic transition authorization;
- default phase topology, completed-unit naming syntax, or parent/child lineage;
- stable core serialization or cross-version CLI wire-schema compatibility;
- revision fields or revision-conditioned commands in the experimental CLI v1
  protocol;
- network-specific controls such as timeouts, rate limits, or concurrent-client
  quotas; the local raw-byte ceiling does not make the CLI a network service.

These boundaries keep the domain foundation testable and allow later adapters to
be selected through evidence rather than embedded assumptions.

## License

CubiKan is available under the [MIT License](LICENSE).
