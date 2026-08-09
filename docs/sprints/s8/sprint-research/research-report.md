# Sprint 8 Research Report

## Intents Reviewed

- [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) — selected and revised; relevance: it is the next reusable CubiKan platform boundary after its INT-0009 prerequisite was realized; current state: `planned`.
- [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) — reviewed; relevance: its realized aggregate-local revision and stale-first guarded commands are the concurrency primitive the durable backend must preserve; current state: `realized`.
- [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) — reviewed but not selected; relevance: immutable origin vocabulary could be split into a narrower prerequisite, but full bidirectional many-to-many evidence queries require INT-0010; current state: `proposed`.
- [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md) — reviewed but not selected; relevance: durable observations and reproducible metric projections depend on INT-0010 and still need clock, correction, denominator, and governance policy; current state: `proposed`.
- [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) — reviewed but not selected; relevance: canonical existing endpoints, durable relationships, and multi-board projections depend on INT-0010; current state: `proposed`.

## 1. Sprint Goal

Realize INT-0010 with the smallest honest durable vertical: add a reusable
Rust backend over an explicit local SQLite database and a separate versioned
`cubikan-local` process adapter that can create, retrieve, query, transition,
and complete multiple replay-validated Intent Units across independent process
invocations. Preserve `cubikan-core` as the chain-agnostic lifecycle authority
and leave the existing one-shot `cubikan` protocol unchanged.

## 2. Existing Code Survey

| File | Relevance | Notes |
|------|-----------|-------|
| `Cargo.toml` | high | The Rust 2024 workspace currently contains only `cubikan-core` and `cubikan-cli`; a durable vertical needs new backend/process crates and one storage dependency. |
| `crates/cubikan-core/Cargo.toml` | medium | Core remains dependency-light and must not acquire SQLite or adapter dependencies. |
| `crates/cubikan-core/src/lib.rs` | high | Exposes workflow, lifecycle, revision, guarded command, and error contracts required by an adapter. |
| `crates/cubikan-core/src/intent_unit.rs` | high | Owns immutable identity/species/workflow, phase/status/history/revision, stale-first guarded mutation, and validated replay; its Serde representation remains provisional. |
| `crates/cubikan-core/src/workflow.rs` | high | Public getters allow an adapter-owned workflow snapshot to be encoded and reconstructed through `Workflow::new`. |
| `crates/cubikan-core/tests/lifecycle.rs` | medium | Existing public journeys define the lifecycle and conflict behavior the durable layer must retain. |
| `crates/cubikan-core/tests/serialization.rs` | medium | Tamper tests demonstrate replay validation but do not authorize core JSON as a storage format. |
| `crates/cubikan-cli/Cargo.toml` | medium | The existing process adapter has only core/Serde dependencies and should not acquire persistence. |
| `crates/cubikan-cli/src/protocol.rs` | high | Protocol v1 configures one whole scenario, omits revision/workflow topology in responses, and cannot resume durable state. |
| `crates/cubikan-cli/src/execution.rs` | high | Delegates lifecycle validity to core but invokes unconditioned operations on a freshly created in-memory aggregate. |
| `crates/cubikan-cli/src/runner.rs` | medium | Its bounded one-request/one-response and flush semantics are useful precedent, not a durable contract to extend in place. |
| `crates/cubikan-cli/README.md` | high | Explicitly denies persistence, sessions, networking, authorization, and stable compatibility; Sprint 8 must preserve those claims for `cubikan`. |
| `README.md` | high | Documents the future-adapter requirement to propagate observed revisions and provide its own transaction/CAS boundary. |
| `docs/intents/INT-0008-traceable-intent-instantiation.md` | medium | Full evidence indexing waits for durable many-to-many storage and reverse queries. |
| `docs/intents/INT-0009-revisioned-lifecycle-commands.md` | high | Realized prerequisite defines expected/actual conflict values and stale-before-domain-validation precedence. |
| `docs/intents/INT-0010-durable-intent-unit-backend.md` | high | Selected semantic authority; revised in Research to choose the bounded local SQLite realization. |
| `docs/intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md` | medium | Remains downstream because observations and reproducible projections need durable state plus unresolved metric policy. |
| `docs/intents/INT-0012-intent-unit-relationships-and-board-projections.md` | medium | Remains downstream because relationships require a canonical durable unit collection. |
| `docs/appendix/potential-derivative-projects.md` | high | Requires a versioned backend boundary, prohibits direct database/core-Serde coupling, and sequences INT-0010 after INT-0009. |

## 3. External Sources

- [rusqlite 0.40.2 documentation](https://docs.rs/crate/rusqlite/0.40.2) — documents the current Rust wrapper and recommends the `bundled` feature for applications that control their own SQLite database, avoiding a system SQLite dependency.
- [SQLite transaction documentation](https://www.sqlite.org/lang_transaction.html) — defines `BEGIN IMMEDIATE`, single active writer behavior, busy failures, and explicit commit/rollback semantics.
- [SQLite isolation documentation](https://www.sqlite.org/isolation.html) — confirms isolated connections see only committed transactions and SQLite serializes writers unless shared-cache/read-uncommitted behavior is deliberately enabled.
- [SQLite atomic-commit documentation](https://www.sqlite.org/atomiccommit.html) — describes rollback-journal commit and crash recovery, while documenting filesystem/hardware assumptions and warning against network filesystems.
- [SQLite PRAGMA documentation](https://www.sqlite.org/pragma.html) — defines rollback `DELETE` mode, `synchronous=EXTRA`, `trusted_schema=OFF`, and application-owned `user_version` used by the selected schema contract.

## 4. Risks, Unknowns, Dependencies

- **Risk — accidental core-schema contract:** Serializing `IntentUnit` directly would contradict INT-0001/INT-0010. The backend must own a strict envelope, reconstruct vocabulary/workflow explicitly, replay history through core behavior, and compare the resulting aggregate with every stored projection.
- **Risk — stale writer despite core checks:** Reading a fresh revision and substituting it into a mutation would erase the caller's observation. A write transaction must load the row, pass the request's expected revision to `transition_to_if_revision` or `complete_if_revision`, update only the same stored revision, and commit before success.
- **Risk — dual stored authority:** SQL columns needed for filtering can drift from the complete envelope. Every loaded or listed row must compare ID, workflow, species, phase, status, and fixed-width revision projections with the replayed aggregate and fail closed on disagreement.
- **Risk — false durability claim:** SQLite atomicity depends on its VFS, local filesystem, device, and synchronous behavior. Version 1 selects rollback-journal `DELETE` plus `EXTRA`, excludes network filesystems, backup/replication, and cryptographic audit, and does not promise protection from hostile database editing.
- **Risk — response after commit:** Once SQLite commit succeeds, a later stdout write or flush failure cannot roll the mutation back. Documentation must label that result delivery-unknown and require refresh; revision checks do not provide idempotent delivery.
- **Risk — bundled dependency:** `rusqlite` with bundled SQLite adds a compiled C dependency and increases build time. Keeping it in `cubikan-backend` preserves the dependency-free domain boundary and avoids system-library variance.
- **Unknown — future schema migration:** This sprint initializes only a newly empty schema v1 and accepts only storage/envelope v1. Automatic migration, import/export, backup, deletion, and compatibility windows are deferred and unsupported versions fail closed.
- **Unknown — cross-page consistency:** UUID keyset pagination gives deterministic, non-repeating ID order, but separate process requests are not one database snapshot. Units whose mutable filter membership changes between pages can be omitted or newly included; this is documented rather than hidden behind a false snapshot claim.
- **Unknown — lock contention:** A finite busy timeout bounds local lock waiting. Timeout is an operational busy failure with no automatic retry and no mutation; it is not an INT-0009 revision conflict.
- **Dependency:** INT-0009 is realized and supplies the exact aggregate token and stale-first error precedence required by INT-0010.
- **Dependency:** INT-0008 full provenance, INT-0011 observations, and INT-0012 relationships remain downstream and do not enlarge the first backend schema.

## 5. Recommended Approach

Primary: add `cubikan-backend`, exposing a concrete synchronous
`SqliteBackend` over a caller-supplied local path, and `cubikan-local`, exposing
one separate experimental JSON protocol v1 request per process. The backend
uses a strict schema-v1 `intent_units` table containing one complete
adapter-owned envelope plus checked query projections. Revisions use an
eight-byte big-endian SQL projection and decimal strings at the JSON boundary,
so the core's `u64` contract is not narrowed and JavaScript number precision is
not assumed.

Create/get/list operations validate every returned aggregate. List supports
exact workflow/species/phase/status filters, limit `1..=100`, canonical UUID
ascending order, and an exclusive last-ID cursor. Transition and completion use
`BEGIN IMMEDIATE`, replay validation, the externally supplied expected revision,
the core guarded method, a revision-qualified row update, and commit-before-
response. Schema/envelope mismatch, corruption, duplicate/missing identity,
stale revision, domain rejection, busy storage, and I/O remain distinguishable.

The process adapter accepts an explicit `--database PATH`; no default operating-
system directory is selected. Actual-process E2E creates multiple units, exits,
queries them, mutates one across later invocations, rejects a stale competing
command without durable change, completes the unit, and retrieves its exact
workflow/history/revision after another restart. Existing `cubikan` v1 tests
remain unchanged.

Alternative considered: a per-unit JSON directory plus atomic rename is
dependency-light but requires bespoke cross-process locking, durable CAS,
multi-field indexes, pagination, and recovery. `redb` is pure Rust and ACID but
would also require custom secondary indexes and schema/query machinery. A
PostgreSQL or HTTP service could be a later multi-host adapter, but would add
authentication, tenancy, timeouts, rate limits, deployment, encryption, and
delivery-idempotency policy before there is evidence for them. Extending the
existing one-shot CLI or persisting core Serde was rejected because both
contradict realized boundaries.

Rationale: local embedded SQLite is the smallest reversible boundary that
satisfies every INT-0010 acceptance criterion and unlocks the downstream
provenance, relationship, metric, and derivative-application intents. Its
technical choices are explicit in the stable intent and remain subject to the
normal human-approved Sprint 8 `dev -> main` checkpoint.

## Artifacts

- [Revised INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) — stable authority for the selected first backend boundary and its acceptance semantics.
- [Potential Derivative Projects](../../../appendix/potential-derivative-projects.md) — existing ecosystem dependency and data-authority map used to prevent direct storage coupling.
