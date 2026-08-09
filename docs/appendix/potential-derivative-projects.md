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

## Recommended repository catalog

Each primary recommendation below is a boundary proposal, not a repository
creation request. Shared inputs are named explicitly; canonical data and policy
ownership must not conflict. The first group covers agent operations,
provenance analytics, and Sprint Loops accounting.

### 1. Agent Ops — `cubikan-agent-ops`

- **Recommended repository:** `cubikan-agent-ops`.
- **Problem and outcome:** Provide a manager-facing control plane that turns
  selected Project Book work into bounded assignments, routes those assignments
  to doers, and observes execution without replacing Book authority. Each
  executable work item is represented by a CubiKan Intent Unit.
- **Owned data:** Namespaced manager and doer identities; capability and
  availability profiles; decomposition and assignment records that reference
  Book work and Intent Unit IDs; queue state; execution attempts; tool requests
  and results; approval records; retry state; and raw time, token, tool, and
  monetary-cost observations. It does not copy Book content or CubiKan
  lifecycle state into a competing authority.
- **Owned policy:** Manager/doer identity, decomposition, assignment, readiness,
  scheduling, dispatch, tool selection, permissions, sandboxing, budgets,
  retries, cancellation, approvals, escalation, and cost controls.
- **Inputs:** Read-only Book intent/task references and acceptance boundaries;
  CubiKan unit IDs and revisions; manager instructions; doer capability and
  tool manifests; human approval decisions; and execution artifacts or
  telemetry.
- **Outputs:** Delegation decisions, ordered work queues, dispatch and approval
  requests, execution-attempt and cost records, artifact references, and
  versioned create/transition/complete commands for CubiKan.
- **CubiKan interaction:** CubiKan remains authoritative for Intent Unit
  identity and validated lifecycle state. Shared or resumable operation uses
  INT-0009 revisions and waits for INT-0010, whose bounded collection query can
  support basic queues. Advanced decomposition dependencies, nested-loop
  composition, or multi-board projections require INT-0012. Delegation and
  execution edges never become `WorkflowEdge` or implicit core lineage. A local
  experiment may embed an explicitly pinned current core version, but that does
  not supply persistence or an agent service.
- **Prerequisites:** An explicit manager/doer identity and authorization model;
  Book-to-unit reference and reconciliation rules; privacy, retention,
  secret-handling, approval, and cost semantics; INT-0009 plus INT-0010 for
  shared operation; INT-0012 for durable advanced relationships; and INT-0008
  when assignments participate in reusable intent-to-artifact provenance.
- **Creation trigger:** Create only when an authorized project needs
  coordinated, resumable work across multiple managers or doers and has named
  owners for identity, permissions, approvals, and the INT-0010-backed
  operational boundary. A single-process orchestration experiment is not enough.
- **Separation rationale:** Agent orchestration handles credentials, tools,
  expensive execution, sensitive traces, active scheduling, and recovery. Its
  security, runtime, scaling, and release concerns differ materially from both
  the chain-agnostic lifecycle kernel and the Project Book.
- **Explicit non-goals:** It is not a Book replacement, CubiKan database owner,
  lifecycle validator, provenance/blame analytics system, agent-scoring engine,
  Animus accounting ledger, generic skill-execution graph, or blockchain
  adapter. It does not put actors, tools, permissions, retries, approvals,
  scheduling, or cost semantics into CubiKan; access shared storage or
  provisional core Serde; or treat the one-shot CLI as a session.
- **Related intents:** [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md),
  [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md),
  [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md), and
  [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md).

### 2. Observatory — `cubikan-observatory`

- **Recommended repository:** `cubikan-observatory`.
- **Problem and outcome:** Let teams follow product intent into execution
  evidence without mistaking Git metadata or agent telemetry for proof of
  authorship or quality. It provides reviewable traces across Book intent,
  CubiKan work, exact artifacts, and execution evidence while separating
  recorded association, provider verification, human attribution, and
  analytical inference.
- **Owned data:** Versioned analysis-run metadata; rebuildable correlation
  projections; derived blame snapshots; attribution hypotheses; agent-score and
  recommendation records; and human-review dispositions. The Book remains
  authoritative for intent and realization history, CubiKan for Intent Unit
  lifecycle and future provenance associations, and each external provider for
  its source records.
- **Owned policy:** Connector/refresh and cache-invalidation rules; analytical
  definitions; confidence and uncertainty presentation; privacy classification;
  data minimization; retention and deletion; redaction; access control; score
  governance; and human-review gates. It does not decide lifecycle validity,
  work assignment, agent permissions, or intent satisfaction.
- **Inputs:** Distinct namespaced Book intent references and `IntentUnitId`
  values; exact CubiKan lifecycle revisions and future provenance associations;
  repository-qualified full Git object IDs; pull requests with exact head
  revisions; revision-pinned CI runs/jobs/checks; tests and documents with
  source context; and access-controlled, deliberately selected agent-trace
  references. Raw prompts and tool transcripts are not ingested by default.
- **Outputs:** Intent-to-unit, unit-to-artifact, and artifact-to-unit trace views;
  revision-pinned evidence-coverage and missing-link reports; contextual blame
  views; versioned agent-score reports with uncertainty; recommendations and
  human dispositions; and exportable references for review. Outputs never
  directly mutate lifecycle state or an agent.
- **CubiKan interaction:** A bootstrap can read existing Book and Git evidence
  without writing CubiKan. Full revision-scoped, bidirectional provenance uses
  a future versioned evidence boundary only after INT-0008, INT-0009, and
  INT-0010 are selected and realized. Observatory never edits backend storage,
  persists provisional core Serde as a contract, or treats the one-shot CLI as
  a service.
- **Prerequisites:** Read-only bootstrapping needs explicit namespace rules,
  provider connectors that preserve immutable full identities, and access
  controls for every sensitive source. Full bidirectional provenance needs
  INT-0008 plus INT-0009 and INT-0010; no analytical projection substitutes for
  their canonical association.
- **Creation trigger:** Create when repeated cross-project trace questions
  justify a connector-heavy analytics runtime with its own privacy and release
  boundary; a read-only Book/Git prototype may establish that need before a
  durable backend exists. Before publishing agent scores or feeding any
  recommendation into adaptation, an approved governance plan must establish
  data minimization, retention/deletion and redaction, access control, and a
  named human approval gate for score publication and every adaptation decision.
- **Separation rationale:** Git-host integration, CI/test ingestion, sensitive
  agent telemetry, and analytical methods have different trust, privacy,
  deployment, and release concerns from CubiKan's provider-neutral lifecycle
  backend. Separation also prevents analysis from acquiring Agent Ops execution
  or permission authority.
- **Explicit non-goals:** It does not certify intent satisfaction; treat blame,
  authorship, or scores as proof of contribution, causality, quality, or fitness;
  automatically change prompts, models, tools, permissions, routing, or
  lifecycle; replace a source authority; collect unrestricted personal or trace
  data; or promise cryptographic, blockchain, regulatory, or tamper-proof
  evidence.
- **Related intents:** [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md),
  [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md),
  [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md), and future
  observation evidence under
  [INT-0011](../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md).

### 3. Animus Ledger — `animus-ledger`

- **Recommended repository:** `animus-ledger`.
- **Problem and outcome:** Reconcile Sprint Loops plans with evidenced execution
  and produce governed operational-accounting views of work, cost, and credit.
  The outcome is an explainable plan-versus-actual ledger, not another task
  executor or lifecycle authority.
- **Owned data:** Versioned accounting-model identifiers; ledger entries anchored
  to source references; reconciliation runs; cost/credit allocations; correction
  or reversal chains; approval records; anti-gaming findings; and unresolved
  evidence flags. Book, CubiKan, and external providers retain their canonical
  source data.
- **Owned policy:** Book parsing/schema compatibility; plan-versus-actual
  matching; unit-of-account definitions; cost and credit valuation;
  evidence-admission/trust thresholds; correction, reversal, close, and reopen
  behavior; anti-gaming controls; and human approval workflow.
- **Inputs:** Versioned Book intents, sprint plans, tasks, and completion records;
  exact Intent Unit IDs and lifecycle revisions; full Git revisions and
  PR/CI/test evidence; other provider records; and Agent Ops execution/cost
  observations. Inputs carry verification and trust status; missing or disputed
  provenance remains unresolved rather than silently valued.
- **Outputs:** Versioned reconciliation statements; plan-versus-actual variances;
  cost and credit views; correction and approval trails; and explicit missing-
  evidence, trust, or anti-gaming exceptions. Each output identifies the
  accounting-model version and evidence set that produced it.
- **CubiKan interaction:** Consume a future versioned query/evidence boundary
  backed by INT-0008, INT-0009, and INT-0010, referencing exact units and
  revisions without editing CubiKan storage. INT-0011 may supply revision-linked
  observations, but Animus owns their accounting interpretation. CubiKan
  lifecycle history is sequence evidence—not a financial ledger, audit journal,
  valuation record, or proof that work occurred.
- **Prerequisites:** An accepted accounting charter defining the unit of account,
  cost/credit semantics, trustworthy-provenance threshold, corrections,
  anti-gaming treatment, access/retention controls, and human approval roles;
  plus a verified path that distinguishes Book intent IDs, Intent Unit IDs,
  lifecycle revisions, and provider artifacts.
- **Creation trigger:** Create only when a real consumer needs repeatable
  cross-sprint plan-versus-actual reconciliation, the accounting charter is
  accepted, and trustworthy end-to-end provenance is available. Until then,
  Book reports and research are sufficient.
- **Separation rationale:** Accounting models, Book adapters, evidence trust,
  corrections, anti-gaming controls, and approval have different authority,
  privacy, and release cadence from lifecycle validation. Separation prevents
  valuation policy from becoming a CubiKan invariant.
- **Explicit non-goals:** It does not replace or dual-write the Book; orchestrate
  agents or skills; create lifecycle or parent-child relations; certify linked
  provenance; claim financial, tax, regulatory, audit, or double-entry
  compliance; infer causality or agent quality; automate rewards, payments, or
  adaptation; promise blockchain immutability; or use shared storage,
  provisional core serialization, or the one-shot CLI as a durable session.
- **Related intents:** [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md),
  [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md),
  [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md),
  [INT-0011](../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md),
  and [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md).
- **Recursive-loop boundary:** Agent Ops owns manager/doer delegation and loop
  initiation; Skill Graph owns readiness, fan-out/join, retries, and execution
  composition; INT-0012 may later preserve explicit cross-unit grouping or
  dependency relations. Animus can reconcile only explicit loop identities and
  relations. Parent-child meaning, roll-up, and correction propagation remain
  open and are never inferred from `WorkflowEdge`, phase order, or lifecycle
  history.
