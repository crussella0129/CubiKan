# Potential Derivative Projects

> **Advisory status:** This appendix is a non-binding architecture map. Project
> Book intent chapters own product meaning and realization status. The current
> CubiKan surfaces described below are governed by their realized intents; no
> recommended derivative repository named in the catalog is asserted to exist,
> be scheduled for implementation, or be authorized for creation. Provider or
> network adapters remain future work unless separately selected and realized.
> Recommended names, boundaries, and sequencing may change when stronger
> evidence appears.

The map distills the sanitized [Sprint 6 retained-theme
inventory](../sprints/s6/sprint-research/research-report.md#retained-theme-inventory).
It is complete against that bounded inventory, not against the omitted portions
or original ordering of the user-provided discussion.

## Current CubiKan boundary

Today CubiKan consists of four deliberately bounded surfaces:

- `cubikan-core` is a chain-agnostic Rust lifecycle kernel. It validates opaque
  Intent Unit identity, caller-defined workflow phases and directed
  transitions, active/completed status, and ordered lifecycle history for one
  aggregate.
- `cubikan` is the experimental stateless, one-shot, in-memory JSON CLI
  adapter. One process configures a workflow, creates one unit, performs its
  requested operations, emits one response, and exits without preserving
  state.
- `cubikan-backend` is a synchronous, embedded SQLite Rust library for multiple
  durable Intent Units at a caller-supplied local filesystem path. It supports
  exact SQLite schema v1 for lifecycle storage and schema v2 for the durable
  relationship extension; relationship contract v1 and projection query v1
  are exposed through this Rust boundary only.
- `cubikan-local` is the separate explicit-path durable JSON process adapter.
  Each invocation executes one local protocol-v1 operation against the selected
  SQLite file. Protocol v1 remains lifecycle-only: create, get, list,
  transition, and complete; it does not expose relationship or projection
  operations.

These versioned durable contracts do not stabilize the provisional
`cubikan-core` Serde form or turn either CLI into an application backend or
resumable service. The current surfaces remain local and supply no network
filesystem or network service, actors, authorization, metrics, UI, deployment,
or blockchain behavior.

## Architectural layers

The recommendations use four distinct layers:

| Layer | Responsibility | Current status |
|-------|----------------|----------------|
| Lifecycle kernel | One-unit identity, immutable workflow, and validated transition/completion rules. | Realized in `cubikan-core`. |
| CubiKan backend capability | Reusable provenance, revision, persistence/query, measurement-evidence, or relationship behavior shared by multiple consumers. | Revision, durable lifecycle storage/query, and relationships/projections are realized under INT-0009, INT-0010, and INT-0012. Provenance and measurement evidence remain proposed under INT-0008 and INT-0011. |
| Adapter | Translation between CubiKan-owned concepts and an external provider or protocol, such as a Project Book parser or Git-host connector. | The stateless `cubikan` adapter and explicit-path `cubikan-local` lifecycle adapter exist; Book, Git-host, and other provider-specific adapters remain future work. |
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

A derivative must preserve the Project Book as product-meaning and historical
realization authority; it may reference or project Book work, but it must not
replace or dual-write that authority. Its acceptable CubiKan integration
direction depends on the required capability:

- For local validation, it may embed the current public `cubikan-core` API at an
  explicitly pinned crate version. That pin does not create a cross-version Rust
  API compatibility promise.
- For durable or multi-process work, it may consume an adapter-owned,
  explicitly versioned boundary that is available today:
  `cubikan-backend` provides the local Rust lifecycle, relationship, and
  projection APIs, while `cubikan-local` protocol v1 provides only create, get,
  list, transition, and complete against an explicit local database path.
  Consumers must pin and honor the selected boundary's independent version
  contract; availability does not imply cross-version compatibility.
- Provider-specific adapters, including Project Book and Git-host connectors,
  remain future work. Any network transport or service likewise requires its
  own selected intent and versioned adapter boundary; neither is implied by the
  available local backend or lifecycle process adapter.

A derivative must not:

- edit a CubiKan database directly or share writable backend storage;
- persist or decode provisional core Serde as if it were a stable disk or wire
  contract;
- treat the stateless `cubikan` CLI or local `cubikan-local` process adapter as
  a session, resumable service, or application backend;
- duplicate lifecycle validation, mint conflicting Intent Unit state, or let a
  projection become a second lifecycle authority; or
- infer authentication, authorization, tenancy, deployment, blockchain, or
  network-service behavior from the local boundaries.

## Data-authority map

Each datum has one canonical authority. Consumers may hold references or
rebuildable projections, but they do not dual-write the source of truth.

| Datum | Canonical authority | Consumer rule |
|-------|---------------------|---------------|
| Product intent, rationale, acceptance criteria, decisions, sprint plans, and current historical realization evidence | The Project Book | CubiKan and derivatives may reference or project it; they do not replace or dual-write it. |
| Current in-process Intent Unit identity, workflow, phase, status, and lifecycle history | The validated `cubikan-core` aggregate | Adapters and derivatives invoke public lifecycle behavior; they do not construct competing state. |
| Durable Intent Unit state, revision, and bounded lifecycle queries | The replay-validated, versioned `cubikan-backend` storage and command/query boundary realized under INT-0009 and INT-0010 | Derivatives use the public boundary and never share or edit its writable storage. |
| Versioned relationship definitions and accepted direct edges; ephemeral board or portfolio projections | The `cubikan-backend` relationship contract version 1 is canonical for accepted definitions and edges; projection query version 1 derives live views without creating another authority | Consumers submit and query explicit definitions, edges, and projections through the public Rust API; they do not copy membership into lifecycle state or infer execution policy. |
| External Git objects, pull requests, and CI records | Their source provider | CubiKan stores namespaced references/evidence associations, not shadow provider objects. |
| Manager/doer identity, decomposition, assignment readiness/priority, allowed-tool/sandbox/budget envelope, delegation retry/cancel policy, and approvals | Agent Ops | Intent Units represent lifecycle work without becoming an agent runtime; Agent Ops authorizes an execution envelope rather than owning node execution. |
| Skill manifests, node readiness/scheduling, executor/tool selection within the approved envelope, attempts/leases, node retry/cancel/recovery, sandbox enforcement, and artifact routing | Skill Graph | These execution records reference canonical units and relations without becoming lifecycle state. |
| Business records, PII, retention, RBAC, notifications, reports, and user experience | The bounded domain application | CubiKan owns only referenced lifecycle state and explicitly selected reusable relations. |
| Business measurement definitions and authorization policy | The authoring process application or caller | A future backend may evaluate only caller-supplied versioned definitions; it does not invent business policy. |
| Raw lifecycle-linked observations and deterministic metric results | A future CubiKan evidence backend under INT-0011 | Analytics consumers interpret results without rewriting the observations or lifecycle. |
| Analytical blame, attribution hypotheses, scores, and recommendations | The governed analytics derivative | They remain derived claims and never certify provenance or mutate agents automatically. |

The Book remains the semantic and historical realization authority; the
realized backend is authoritative only for the durable Intent Unit and
relationship state accepted through its versioned boundaries. Treating backend
state as operational task or completion truth for the Book still requires a
separately selected projection or migration intent with reconciliation and
cutover rules. Book and backend dual-write is prohibited because it would
create split-brain history.

## CubiKan capability status map

The following chapters own distinct reusable outcomes. Their Book states are
authoritative: INT-0009, INT-0010, and INT-0012 are `realized`; INT-0008 and
INT-0011 remain `proposed` with no Work or Completion evidence.

- [INT-0008 — Traceable intent instantiation and artifact
  provenance](../intents/INT-0008-traceable-intent-instantiation.md) is
  **proposed** and owns
  namespaced origin references and provider-neutral evidence associations. A
  read-only origin-reference experiment could proceed independently; full
  revision-scoped and bidirectional provenance can build on the realized
  INT-0009 and INT-0010 primitives but is not itself realized by them.
- [INT-0009 — Revisioned lifecycle commands and atomic conflict
  rejection](../intents/INT-0009-revisioned-lifecycle-commands.md) is
  **realized** and owns the optimistic revision primitive. A stale expected
  revision is checked before command validity; with a current revision,
  existing domain errors remain authoritative.
- [INT-0010 — Durable multi-unit CubiKan
  backend](../intents/INT-0010-durable-intent-unit-backend.md) is **realized**,
  depends on realized INT-0009, and owns explicit-path SQLite persistence,
  validated restoration, guarded lifecycle commands, and bounded, paginated
  collection queries over stable lifecycle fields.
- [INT-0011 — Lifecycle checkpoints and metric
  evidence](../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md)
  is **proposed**. Its INT-0009 and INT-0010 dependencies are realized, but its
  durable observations and deterministic evaluation of caller-supplied
  measurement definitions are not. It does not own business policy or
  transition authorization.
- [INT-0012 — Intent Unit relationships and board
  projections](../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
  is **realized**, depends on realized INT-0010, and owns reusable typed
  cross-unit relations and ephemeral projections through the
  relationship contract version 1 and projection query version 1 Rust backend
  boundary, not
  execution scheduling.

This is a partial order, not one mandatory linear roadmap:

```text
[proposed] read-only INT-0008 origin-reference exploration

[realized] INT-0009 revision contract
    └── [realized] INT-0010 durable multi-unit backend
          ├── [proposed] full INT-0008 provenance index (also uses INT-0009)
          ├── [proposed] INT-0011 measurement evidence (also uses INT-0009)
          └── [realized] INT-0012 relationships and board projections
```

Realized prerequisites satisfy only the recorded technical dependency edges;
they do not resolve, select, or realize the proposed branches.

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
  Book work and Intent Unit IDs; assignment queues; authorized execution
  envelopes; approval records; aggregate attempt references; and assignment-level
  time, token, tool-use, and monetary-cost summaries. Skill Graph or another
  selected executor/provider owns underlying node/tool telemetry; an INT-0011
  backend owns any accepted lifecycle-linked observation. Agent Ops retains
  references or aggregates without copying those records, Book content, or
  CubiKan lifecycle state into a competing authority.
- **Owned policy:** Manager/doer identity, decomposition, assignment readiness
  and priority, delegation-level scheduling/dispatch, permissions and the
  allowed-tool/sandbox/budget envelope, delegation retry/cancel limits,
  approvals, escalation, and aggregate cost controls. Skill Graph owns node-level
  readiness, scheduling, executor/tool selection, sandbox enforcement, and
  retry/recovery within that envelope.
- **Inputs:** Read-only Book intent/task references and acceptance boundaries;
  CubiKan unit IDs and revisions; manager instructions; doer capability and
  tool manifests; human approval decisions; and execution artifacts or
  telemetry.
- **Outputs:** Delegation decisions, ordered assignment queues, authorized
  execution envelopes, dispatch and approval requests, aggregate attempt/cost
  references, artifact references, and versioned create/transition/complete
  commands for CubiKan.
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

### 4. Process Studio — `cubikan-process-studio`

- **Recommended repository:** `cubikan-process-studio`.
- **Problem and outcome:** Give process owners an Electron-first desktop
  workspace for designing, reviewing, versioning, and publishing reusable
  process definitions without turning CubiKan into an editor or embedding one
  industry's funnel in the lifecycle core. The result is a governed definition
  package whose structural subset becomes an immutable, core-validated CubiKan
  workflow and whose business measurements remain explicit and reviewable.
- **Owned data:** Versioned process definitions; domain vocabulary and checkpoint
  metadata; editor layout and draft state; measurement-definition versions;
  business-authorization definitions; validation fixtures; publication records;
  and supersession/migration guidance. Existing units remain pinned to their
  original immutable workflow snapshot; Studio does not own unit state, raw
  operational observations, or durable metric results.
- **Owned policy:** Definition identity/versioning; author, reviewer, and
  publisher permissions; draft, approval, publication, deprecation, and rollback
  rules; business measurement semantics; and business authorization. Definitions
  state units, denominators, windows, aggregation, event-time/source,
  missing-data, duplicate, late-arrival, and correction behavior. Authorization
  may govern who can request work but cannot make an undeclared edge valid or
  bypass core validation.
- **Inputs:** Process-owner stages and transitions; domain vocabulary; checkpoint
  and observation requirements; units, denominators, windows, and source
  semantics; authorization/review rules; existing definition packages; and an
  explicitly selected core or future backend contract version.
- **Outputs:** Immutable versioned process-definition packages; a phase/edge/
  species subset translated through CubiKan validation; caller-supplied versioned
  measurement definitions; business-authorization artifacts; validation
  diagnostics/previews; and references suitable for later adapters. Publication
  does not activate a shared workflow, rewrite units, or become lifecycle evidence.
- **CubiKan interaction:** Local structural validation may embed the current
  public core at an explicitly pinned crate version; that supplies neither
  persistence nor KPI storage. A definition package may be reviewed or
  distributed as a non-operational artifact, but every shared operational or KPI
  activation waits for INT-0009 revisions, INT-0010's versioned durable
  command/query boundary, and INT-0011 observation/evaluation behavior. Studio
  authors and governs the caller-supplied versioned definitions and
  authorization; the future backend stores raw lifecycle-linked observations
  and deterministically evaluates only those definitions; Observatory consumes
  the results for governed analysis. Studio never writes backend storage or
  treats provisional Serde/the one-shot CLI as a durable contract.
- **Prerequisites:** A definition identity/version and compatibility model;
  workflow-version pinning rules; process-owner authorization; complete
  measurement and correction semantics; privacy/retention treatment for
  metadata; and a chosen pinned core version. Shared operational/KPI release also
  requires realized INT-0009, INT-0010, and INT-0011 plus a versioned adapter.
- **Creation trigger:** Create when an authorized team repeatedly needs to author
  and govern multiple process definitions through a reusable Electron-first
  experience, owns the definition/authorization model, and accepts immutable
  workflow versioning. Local definition and validation may begin before
  persistence; shared operations, observations, and KPI results may not.
- **Separation rationale:** Interactive editing, desktop packaging, domain
  vocabulary, formula/version governance, authorization UX, and definition
  releases have different dependencies and trust concerns from a small
  chain-agnostic Rust kernel and a durable backend.
- **Explicit non-goals:** It is not a CubiKan database, unit authority, agent or
  skill executor, organizational system of record, Observatory analytics engine,
  or fixed Kanban/KPI/domain product. It does not put UI, formulas, business
  authorization, clocks, PII, raw observations, or automatic-transition policy
  into the core; rewrite immutable workflows/history; bypass edges; share
  storage; or imply blockchain, transport, deployment, or cross-version API policy.
- **Related intents:** [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md),
  [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md), and
  [INT-0011](../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md).

### 5. Skill Graph — `cubikan-skill-graph`

- **Recommended repository:** `cubikan-skill-graph`.
- **Problem and outcome:** Execute versioned, multi-board skill pipelines without
  turning the lifecycle kernel into an agent runtime. The result is a reviewable
  execution DAG that converts authorized assignments and canonical unit/relation
  state into readiness, dispatch, fan-out/join, retry, and artifact-flow decisions.
- **Owned data:** Versioned skill manifests and immutable digest references;
  declared input/output schemas and runtime requirements; pipeline, node, and
  execution-edge definitions; unit-to-node and board-gate bindings; readiness
  and scheduling decisions; execution attempts/leases; retry, cancellation, and
  recovery state; sandbox-profile references/results; artifact-routing manifests;
  and telemetry. Source artifacts, CubiKan state, and actor identities stay with
  their canonical providers.
- **Owned policy:** Skill admission/version pinning; node readiness; pipeline and
  board routing; node scheduling; fan-out/join; executor selection; retry,
  timeout, cancellation, and partial failure; execution isolation/sandboxing;
  and artifact validation, routing, retention, and promotion. These operate
  inside the identity, tool, permission, and approval envelope from Agent Ops.
- **Inputs:** Authorized assignments and executor references from Agent Ops;
  unit IDs, revisions, and bounded query results from INT-0010; typed unit
  relations and board projections from INT-0012; pinned skills; approved resource
  envelopes; artifact references; cancellation/approval decisions; and optional
  INT-0008 provenance associations.
- **Outputs:** Versioned execution plans; readiness/routing decisions; dispatch
  requests; attempt, retry, cancellation, and sandbox records; fan-out/join
  results; artifact manifests/references; telemetry; and expected-revision
  lifecycle commands that CubiKan may accept or reject.
- **CubiKan interaction:** CubiKan owns unit identity, revisioned lifecycle state,
  and transition/completion validation. INT-0012 owns canonical cross-unit
  relations and projections. Agent Ops owns actor identity, delegation,
  assignment, permission, and approval. Skill Graph owns skill, pipeline, node,
  execution-attempt, readiness, and node-scheduling decisions. Pipeline edges
  connect skill nodes; unit-dependency edges reference explicit INT-0012
  relations; board-routing edges connect projection gates. None is a
  `WorkflowEdge`, transfers lifecycle ownership, or makes an execution result a
  lifecycle transition. Nested loops are explicit pipeline composition plus
  typed relations, never lineage inferred from phase/history.
- **Prerequisites:** Realized INT-0010 and INT-0012 for shared multi-unit work
  (with INT-0009 supplying revisions); defined graph version/cycle behavior,
  fan-out/join and failure semantics, retry/idempotency limits, executor trust,
  capacity/cancellation, skill admission, secret handling, sandbox isolation,
  artifact identity/retention, and the Agent Ops authorization contract. INT-0008
  is needed when durable bidirectional artifact provenance is an outcome.
- **Creation trigger:** Create when an authorized consumer repeatedly runs one
  versioned skill graph across persisted units or board projections, INT-0010
  and INT-0012 exist, and named owners accept the executor, sandbox, retry,
  artifact, and authorization policies. A fixed process-local sequence or board
  visualization alone is insufficient.
- **Separation rationale:** Skill loading, untrusted execution, secrets,
  sandboxing, retries, artifact movement, and executor recovery have a different
  security model, runtime, deployment shape, and release cadence from lifecycle
  validation, relation storage, and delegation.
- **Explicit non-goals:** It is not the Book, actor/delegation authority,
  lifecycle validator/database, board system of record, process/KPI author,
  analytics/scoring layer, or accounting system. It does not reinterpret
  `WorkflowEdge`; infer lineage; copy units between boards; bypass revisions;
  certify artifacts; publish skills/packages by implication; edit shared
  storage; or choose blockchain, database, transport, or deployment policy.
- **Related intents:** [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md),
  [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md),
  [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md),
  and conditionally
  [INT-0008](../intents/INT-0008-traceable-intent-instantiation.md).

### 6. Organizational App Kit — `cubikan-org-app-kit`

- **Recommended repository:** `cubikan-org-app-kit`. This is the one primary
  app-kit recommendation. Per-domain vertical repositories are an unnamed,
  independently authorized future pattern—not extra catalog recommendations or
  repositories claimed to exist.
- **Problem and outcome:** Give bounded organizational applications reusable,
  conformance-tested client and presentation primitives for CubiKan lifecycle
  work without moving domain policy into the kernel or making every application
  invent an incompatible backend integration. The result is a kit, not a hosted
  organizational system.
- **Owned data:** Versioned client-configuration schemas, backend-capability and
  compatibility metadata, lifecycle view-model definitions, reference fixtures,
  and conformance cases. Cached views are rebuildable projections; the kit owns
  no operational unit or business record.
- **Owned policy:** Client compatibility/extension contracts, translation of
  versioned lifecycle responses into neutral view models, projection-cache
  invalidation, secure-default integration guidance, and kit release policy. It
  does not own a vertical's business rules, authorization, retention, or UX.
- **Inputs:** A future versioned CubiKan command/query boundary; unit IDs,
  revisions, lifecycle fields, and bounded paginated results; caller-owned
  authorization context/domain references; and requirements supplied by an
  independently authorized bounded-domain application.
- **Outputs:** Reusable client bindings and command builders, neutral lifecycle
  components/view models, basic list/queue/board projection helpers, integration
  scaffolds, compatibility declarations, and conformance fixtures. Domain
  events, notifications, reports, and applications remain domain outputs.
- **CubiKan interaction:** Basic projections use INT-0010's bounded collection
  query over stable lifecycle fields; advanced multi-board views or typed
  relations wait for INT-0012. The kit sends revision-aware commands through a
  versioned boundary and never edits storage, persists provisional Serde, treats
  the one-shot CLI as a session, or infers relations from `WorkflowEdge`.
- **Prerequisites:** Realized INT-0009 and INT-0010 for operational clients;
  INT-0012 for advanced relations/boards; accepted client versioning and
  compatibility; and bounded-domain decisions for identity, authorization,
  privacy, retention, deployment, and support. A local demo may pin the current
  core, but that is not a shared backend.
- **Creation trigger:** Create only when independently authorized domain work
  demonstrates repeated client, projection, and conformance needs across
  applications and a usable INT-0010 boundary exists. A speculative frontend,
  one bespoke screen, or a desire to preselect vertical repositories is not enough.
- **Separation rationale:** Reusable client/frontend dependencies have a
  different release cadence from the lifecycle kernel, while every vertical has
  its own data classification, security, integrations, deployment, and product
  cadence. The split permits reuse without making CubiKan a UI framework or the
  kit an organizational monolith.
- **Explicit non-goals:** It is not an organizational system of record, durable
  backend, shared database, generic low-code platform, Process Studio, skill
  executor, agent manager, analytics/reporting authority, identity provider,
  notification service, deployment platform, or blockchain adapter. It does not
  own domain records, PII, retention, RBAC, integrations, notifications, reports,
  deployment, or vertical UX; define KPIs, workflows, relation taxonomies, or
  scheduling; authorize a vertical repository; or copy lifecycle state into a
  competing authority.
- **Related intents:** [INT-0009](../intents/INT-0009-revisioned-lifecycle-commands.md),
  [INT-0010](../intents/INT-0010-durable-intent-unit-backend.md), and
  [INT-0012](../intents/INT-0012-intent-unit-relationships-and-board-projections.md).
- **Vertical-repository pattern:** A future vertical requires its own selected
  intent and repository authorization. That bounded domain owns records, PII,
  retention/deletion, RBAC/authorization, integrations, notifications, reports,
  deployment, and product UX; it stores only CubiKan IDs or rebuildable
  projections where needed. No vertical slug is selected here.
- **Frontend sequencing:** When an authorized vertical needs a cross-platform
  desktop client, begin with Electron for functional iteration and consider
  Tauri only after maturity justifies platform optimization. This is advice, not
  evidence that either frontend exists or is scheduled.

## Retained-theme traceability

This table closes the bounded inventory from Sprint 6 Research. “Merged” means
one idea informs multiple authorities; it does not create a hidden seventh repo.

| Theme | Backend/adapter boundary | Derivative recommendation or disposition |
|-------|--------------------------|------------------------------------------|
| `DV-01` | INT-0010 provides the potential common lifecycle backend; the Book retains current semantic/history authority. | Agent Ops coordinates work; Animus Ledger reconciles evidenced work. |
| `DV-02` | Book-to-unit mapping and future provenance remain namespaced. | Agent Ops owns manager/doer execution; Animus reads evidence but does not execute. |
| `DV-03` | INT-0008 owns durable associations; Git/Book/CI connectors remain adapters. | Observatory owns trace views and governed analytical inference. |
| `DV-04` | INT-0011 owns observations and deterministic evaluation of caller definitions. | Process Studio authors/governs definitions; Observatory analyzes results. |
| `DV-05` | The data-authority map keeps the Book canonical until an explicit migration/projection intent. | Animus derives reconciliation without dual-writing Book history. |
| `DV-06` | INT-0012 owns reusable relations/projections, never phase edges. | Skill Graph owns executable DAG policy and multi-board routing. |
| `DV-07` | INT-0010 supplies durable lifecycle commands and bounded queries. | Process Studio and the Organizational App Kit remain separate frontends/policy surfaces. |
| `DV-08` | Explicit INT-0012 relations may represent cross-unit composition; no core lineage is inferred. | Merged across Agent Ops delegation, Skill Graph execution, and Animus reconciliation; exact recursive semantics remain open. |
| `DV-09` | Blockchain remains an unselected adapter concern with unresolved chain/trust/key/finality/data policy. | Deferred; no blockchain derivative repository is recommended. |

## Sequencing and creation gates

The ordering is evidence-driven and deliberately non-calendar-based:

1. **Read-only discovery:** Observatory may prototype approved Book/Git trace
   views after its privacy controls exist. Process Studio may validate local
   definitions against a pinned core after its definition/version model exists.
   Neither prototype claims a backend.
2. **Reusable backend foundation:** Select and realize INT-0009 before INT-0010.
   Storage, transport, schema, recovery, auth, tenancy, and deployment still
   require human decisions. Full INT-0008 reverse provenance follows INT-0009
   and INT-0010.
3. **Shared operational applications:** Agent Ops and basic organizational
   projections may begin only after their identity/security policies and an
   INT-0010 boundary exist. Process Studio may distribute a definition as a
   non-operational artifact earlier, but shared operational or KPI activation
   waits for INT-0009, INT-0010, and INT-0011.
4. **Measurements and graph composition:** Shared metric evidence needs INT-0011.
   Advanced multi-board relations need INT-0012; Skill Graph additionally needs
   an accepted executor, sandbox, retry, artifact, and authorization contract.
5. **Governed reconciliation:** Animus Ledger follows trustworthy provenance and
   an accepted accounting charter. It does not gain authority merely because
   lifecycle data is available.

These gates are necessary, not sufficient. Each repository still needs a
separate selected intent, research/plan, owner, and explicit creation approval.

## Merged, deferred, and rejected alternatives

- A single “CubiKan platform” repository was rejected because lifecycle,
  execution, analytics, desktop authoring, domain applications, and accounting
  have incompatible trust/runtime/release boundaries.
- A repository for every crate, provider connector, or board was rejected.
  Provider adapters can begin inside their owning application until a distinct
  runtime or release boundary is evidenced.
- A standalone Git-only product is merged into Observatory's connector and
  analysis boundary unless provider reuse later justifies separation.
- Discord is retained only as the source medium for the supplied design
  excerpts. No Discord runtime requirement or repository is inferred.
- Per-domain organizational applications remain an unnamed pattern under the
  App Kit boundary. Each needs its own product intent and authorization; none is
  selected by this appendix.
- Blockchain support is deferred. Selecting a chain or claiming immutable audit
  would require explicit trust, key, fee, finality, reorganization, privacy, and
  on-chain/off-chain decisions.
- Replacing the Project Book or dual-writing Book/backend task truth was rejected
  because it would create competing semantic and historical authorities.

## Open questions

- Which storage, transport, deployment, tenancy, authentication, authorization,
  recovery, and schema-compatibility policies should realize INT-0010?
- How should Book intent, Intent Unit, repository, and artifact namespaces be
  identified, corrected, retained, and verified under INT-0008?
- What projection/migration and reconciliation contract would be required before
  operational task/completion truth moves away from the Book?
- What manager/doer identity, permission, approval, cancellation, secret, and
  cost model is acceptable for Agent Ops?
- Which observation clocks, sources, denominators, windows, units, late-arrival,
  correction, and authorization semantics make a measurement trustworthy?
- Which relation types, endpoint/cycle/deletion rules, projection consistency,
  and recursive-loop semantics belong under INT-0012?
- What skill admission, executor trust, sandbox, artifact, retry/idempotency,
  fan-out/join, and partial-failure model is safe enough for Skill Graph?
- What unit of account, valuation, trust, correction, close/reopen, anti-gaming,
  privacy, and approval model makes Animus accounting meaningful?
- Which first Process Studio and organizational-app journeys justify their UI
  surfaces, and what backend compatibility window do those clients require?
- Is any blockchain property actually required after the provider-neutral
  provenance and reconciliation boundaries are tested?

## Appendix-wide non-goals

This appendix does not:

- create, publish, push, release, deploy, fund, or schedule any recommended
  repository, package, service, desktop application, or blockchain adapter;
- select a database, chain, network transport, host, identity provider, tenancy
  model, deployment target, or durable compatibility policy;
- promise cross-version Rust, storage, wire, client, or Book-schema compatibility;
- redefine realized INT-0001–INT-0006 behavior or advance INT-0008–INT-0012 out
  of `proposed`;
- grant a projection, derivative, analytics result, or accounting view authority
  over CubiKan lifecycle state or Project Book semantics;
- treat Git blame, telemetry, scores, metrics, or linked evidence as causal proof,
  certification, audit proof, or permission for automatic agent adaptation; or
- serve as a delivery roadmap. Every future implementation remains subject to
  its own intent lifecycle and the normal human approval checkpoint.
