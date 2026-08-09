# CubiKan

CubiKan is an exploration of blockchain-backed workflow coordination. Its core
concept is an **Intent Unit**: a uniquely identified unit of work that occupies a
caller-defined process phase until it reaches terminal completion.

## Current scope

`cubikan-core` is a chain-agnostic Rust library for defining and validating
Intent Unit lifecycles. The `cubikan` executable is an experimental stateless
JSON adapter that executes one complete caller-defined lifecycle scenario per
process. `cubikan-backend` adds a synchronous, embedded SQLite boundary for
multiple durable Intent Units, and the separate `cubikan-local` executable runs
one versioned durable operation per process against an explicit caller-selected
local database path.

The durable layers select adapter-owned stored-envelope, SQLite-schema, and
local-JSON contracts at version 1. They do not turn the provisional core Serde
layout or the stateless `cubikan` protocol into storage authority. No current
layer selects a blockchain, network service, deployment model, or user
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
durable or multi-client adapter must instead receive the caller's previously
observed revision and pass it to the conditioned operation. Reading the revision
inside the adapter immediately before mutation would erase the stale-observation
check. The core comparison alone does not make storage writes atomic.
`cubikan-backend` supplies that boundary with one SQLite `BEGIN IMMEDIATE`
transaction, a revision-qualified compare-and-set update, and commit before
success returns; any future adapter must provide an equivalent guarantee.

## Stateless runnable JSON adapter

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

## Durable local backend and process adapter

`cubikan-backend` owns two exact version 1 persistence contracts: a strict JSON
envelope containing the complete workflow snapshot and lifecycle history, and a
single-table `STRICT` SQLite schema whose query projections are checked against
the envelope after core replay. Every load reconstructs and validates the
aggregate through `cubikan-core`; unsupported versions, corrupt representations,
and projection mismatches fail closed.

`cubikan-local` owns the third version 1 contract: one strict create, get, list,
transition, or complete JSON operation per process. The only invocation form is
an explicit database path:

```sh
cargo run -p cubikan-local --bin cubikan-local -- --database ./cubikan.sqlite3 < request.json
```

The local adapter validates the complete request before opening the path, caps
raw stdin at 1 MiB, and writes one compact JSON response plus newline with one
explicit stdout flush. Its exits are `0` for success, `1` for operational
input/output delivery failure, `2` for usage/request rejection, `3` for
command/domain rejection, and `4` for storage rejection.

SQLite uses rollback-journal `DELETE`, `synchronous=EXTRA`, isolated/default
connections, and a 5,000-millisecond busy timeout. Create and guarded mutations
acquire a writer with `BEGIN IMMEDIATE`; transition and completion replay the
stored unit, preserve the core's stale-before-domain check, update through a
revision-qualified compare-and-set, and commit before success is written.
Writer contention can therefore produce `storage_busy` before a stale revision
is evaluated.

List filters are exact, case-sensitive matches on workflow ID, species, phase,
and status. Workflow-ID equality means the ID only, not equal topology. Limits
range from 1 through 100; pages use ascending lexical canonical-ID order and an
exclusive last-returned-ID cursor. Each request sees a live committed page, not
a snapshot across requests, so mutations can change later membership.

A commit can succeed before stdout body, newline, or flush delivery fails. In
that case the client does not know the committed outcome from process delivery
alone and must retrieve the unit and refresh its revision before deciding what
to do; rollback, retry safety, and idempotency are not implied. See the
[`cubikan-backend` contract](crates/cubikan-backend/README.md) and
[`cubikan-local` protocol](crates/cubikan-local/README.md) for the exact
envelope, schema, recovery, pagination, response, error-code, and exit
contracts.

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

The current durable boundary is a single-tenant, unencrypted, embedded SQLite
file on a caller-controlled local filesystem. It does not support network
filesystems or a network service. Multiple CubiKan backend connections and
`cubikan-local` processes can use the same supported local database under
SQLite's writer serialization; unrelated consumers are not allowed shared
direct write access or row editing.

The current project does not provide:

- authentication or authorization, tenancy, encryption, or privacy policy;
- backup, replication, automatic schema/envelope/protocol migration, repair,
  import, or deletion;
- direct persistence of the provisional core Serde representation;
- automatic retries, idempotency keys, exactly-once execution, or retry-safety
  guarantees;
- cross-Intent-Unit transactions or relationship/parent-child graph policy;
- indefinite stable schema, envelope, protocol, core-serialization, or CLI
  compatibility;
- a blockchain, network, smart contract, cryptographic audit/tamper proof, or
  blockchain policy;
- KPI/metrics evaluation or automatic transition authorization;
- agent, actor, commit, blame, or other provenance tracking;
- a service/API, UI, durable interactive application session, or deployment
  model;
- clocks, timestamps, or global chronological ordering across Intent Units; or
- default phase topology or completed-unit naming syntax.

The existing `cubikan` executable remains stateless and its version 1 protocol
still has no revision fields or revision-conditioned commands. The durable
contracts belong only to `cubikan-backend` and `cubikan-local`. Their local raw
byte ceiling, busy timeout, rollback journal, and explicit flush do not claim a
total request deadline, network controls, crash immunity, acknowledged response
delivery, or production readiness.

These boundaries keep the domain foundation testable and allow later product
policy to be selected through explicit intent rather than embedded assumptions.

## License

CubiKan is available under the [MIT License](LICENSE).
