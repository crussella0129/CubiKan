# Potential Derivative Projects

> **Advisory status:** This appendix is a non-binding architecture map. Project
> Book intent chapters own product meaning. No repository or future backend
> named here is asserted to exist, scheduled for implementation, or authorized
> for creation. Names, boundaries, and sequencing may change when stronger
> evidence appears.

The map distills the sanitized [Sprint 6 retained-theme
inventory](../sprints/s6/sprint-research/research-report.md#retained-theme-inventory).
It is complete against that bounded inventory, not against the omitted portions
or original ordering of the user-provided discussion.

## Current CubiKan boundary

Today CubiKan consists of two deliberately small surfaces:

- `cubikan-core` is a chain-agnostic Rust lifecycle kernel. It validates opaque
  Intent Unit identity, caller-defined workflow phases and directed
  transitions, active/completed status, and ordered lifecycle history for one
  aggregate.
- `cubikan` is an experimental, one-shot, in-memory JSON CLI adapter. One
  process configures a workflow, creates one unit, performs its requested
  operations, emits one response, and exits.

Neither surface currently supplies persistence, a resumable service, actors,
authorization, metrics, cross-unit relationships, multi-board queries, UI,
deployment, or blockchain behavior. The current CLI is an execution boundary,
not an application backend. The core's serialized form is provisional rather
than a durable integration schema.

## Architectural layers

The recommendations use four distinct layers:

| Layer | Responsibility | Current status |
|-------|----------------|----------------|
| Lifecycle kernel | One-unit identity, immutable workflow, and validated transition/completion rules. | Realized in `cubikan-core`. |
| CubiKan backend capability | Reusable provenance, revision, persistence/query, measurement-evidence, or relationship behavior shared by multiple consumers. | Proposed only in INT-0008–INT-0012. |
| Adapter | Translation between CubiKan-owned concepts and an external provider or protocol, such as a Project Book parser or Git-host connector. | Future and provider-specific. |
| Derivative application | User experience, orchestration, business records, analytics, privacy, and domain policy for a bounded problem. | Recommended only; none is created here. |

A separate repository is justified when a surface has a distinct runtime,
deployment model, data/security authority, or release cadence. A crate boundary
or a clever name alone is not enough.

## Graph vocabulary

“Graph” does not denote one universal CubiKan structure:

1. `WorkflowEdge` connects two phases inside one immutable workflow snapshot for
   one Intent Unit.
2. A provenance graph associates namespaced external intent, activity, agent,
   and artifact references.
3. A cross-unit relation connects independent Intent Units for dependency,
   derivation, grouping, or projection.
4. A delegation graph assigns work and expresses responsibility or authority
   without defining how the work executes.
5. An execution graph defines readiness, routing, fan-out/join, retries, and
   executor behavior without transferring lifecycle ownership.

The latter four must not reuse `WorkflowEdge` or imply parent-child lineage in
the current core. Their identities, validation, correction, and authorization
rules belong to separately selected intents or derivative policy.

## Safe CubiKan integration baseline

A derivative has two acceptable integration directions:

- For local validation, it may embed the current public `cubikan-core` API at an
  explicitly pinned crate version. That pin does not create a cross-version Rust
  API compatibility promise.
- For durable or multi-process work, it may consume a future adapter-owned,
  explicitly versioned CubiKan command/query/evidence boundary after the owning
  backend intent is selected and realized.

A derivative must not:

- edit a CubiKan database directly or share writable storage with the backend;
- persist or decode provisional core Serde as if it were a stable disk or wire
  contract;
- treat the current one-shot CLI as a session or resumable service; or
- duplicate lifecycle validation, mint conflicting Intent Unit state, or let a
  projection become a second lifecycle authority.

## Data-authority map

Each datum has one canonical authority. Consumers may hold references or
rebuildable projections, but they do not dual-write the source of truth.

| Datum | Canonical authority | Consumer rule |
|-------|---------------------|---------------|
| Product intent, rationale, acceptance criteria, decisions, sprint plans, and current historical realization evidence | The Project Book | CubiKan and derivatives may reference or project it; they do not replace or dual-write it. |
| Current in-process Intent Unit identity, workflow, phase, status, and lifecycle history | The validated `cubikan-core` aggregate | Adapters and derivatives invoke public lifecycle behavior; they do not construct competing state. |
| Future durable unit state, revision, and bounded lifecycle queries | A future CubiKan backend selected under INT-0009 and INT-0010 | Derivatives use the versioned boundary, never shared writable storage. |
| External Git objects, pull requests, and CI records | Their source provider | CubiKan stores namespaced references/evidence associations, not shadow provider objects. |
| Manager/doer identity, assignment, tools, permissions, scheduling, retries, and approvals | The responsible derivative application | Intent Units represent lifecycle work without becoming an agent runtime. |
| Business records, PII, retention, RBAC, notifications, reports, and user experience | The bounded domain application | CubiKan owns only referenced lifecycle state and explicitly selected reusable relations. |
| Business measurement definitions and authorization policy | The authoring process application or caller | A future backend may evaluate only caller-supplied versioned definitions; it does not invent business policy. |
| Raw lifecycle-linked observations and deterministic metric results | A future CubiKan evidence backend under INT-0011 | Analytics consumers interpret results without rewriting the observations or lifecycle. |
| Analytical blame, attribution hypotheses, scores, and recommendations | The governed analytics derivative | They remain derived claims and never certify provenance or mutate agents automatically. |

The Book is the current semantic and historical authority. Moving operational
task or completion truth to a future backend requires a separately selected
projection or migration intent with reconciliation and cutover rules. Book and
backend dual-write is prohibited because it would create split-brain history.

## Proposed CubiKan capability map

The following chapters preserve reusable outcomes but remain `proposed`, with no
Work or Completion evidence:

- [INT-0008 — Traceable intent instantiation and artifact
  provenance](../intents/INT-0008-traceable-intent-instantiation.md) owns
  namespaced origin references and provider-neutral evidence associations. A
  read-only origin-reference experiment could proceed independently; full
  revision-scoped and bidirectional provenance requires INT-0009 and INT-0010.
- [INT-0009 — Revisioned lifecycle commands and atomic conflict
  rejection](../intents/INT-0009-revisioned-lifecycle-commands.md) owns the
  optimistic revision primitive. A stale expected revision is checked before
  command validity; with a current revision, existing domain errors remain
  authoritative.
- [INT-0010 — Durable multi-unit CubiKan
  backend](../intents/INT-0010-durable-intent-unit-backend.md) depends on
  INT-0009 and owns durable validated restoration plus bounded, paginated
  collection queries over stable lifecycle fields.
- [INT-0011 — Lifecycle checkpoints and metric
  evidence](../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md)
  depends on INT-0009 and INT-0010. It owns durable observations and
  deterministic evaluation of caller-supplied measurement definitions, not
  business policy or transition authorization.
- [INT-0012 — Intent Unit relationships and board
  projections](../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
  depends on INT-0010 and owns reusable typed cross-unit relations and
  projections, not execution scheduling.

This is a partial order, not one mandatory linear roadmap:

```text
read-only INT-0008 exploration

INT-0009 revision contract
    └── INT-0010 durable multi-unit backend
          ├── full INT-0008 provenance index (also needs INT-0009)
          ├── INT-0011 measurement evidence (also needs INT-0009)
          └── INT-0012 relationships and board projections
```

## Decisions required before backend work

The proposed chapters do not choose these policies by implication:

- storage engine, schema evolution, recovery, backup, and migration;
- local versus network transport, deployment, tenancy, authentication, and
  authorization;
- concurrency, idempotency, retry, cancellation, and cross-unit atomicity;
- evidence identity, correction, verification, privacy, retention, and access;
- measurement units, clocks, windows, denominators, correction, and approval;
- relationship taxonomy, cycle/deletion semantics, projection consistency, and
  scheduling authority; or
- blockchain network, key custody, trust, fees, finality, reorganization, and
  on-chain/off-chain data placement.

Each choice needs evidence, a selected intent, and the normal human checkpoint.
This appendix supplies boundaries and creation triggers, not implementation
authority.
