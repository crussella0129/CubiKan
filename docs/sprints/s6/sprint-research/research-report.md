# Sprint 6 Research Report

## Intents Reviewed

- [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) — created and selected; relevance: owns this sprint’s prose-only appendix and ecosystem-boundary outcome; current state: `planned`.
- [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) — created; relevance: preserves original-intent-to-instantiation and artifact traceability as a provider-neutral future CubiKan capability; current state: `proposed`.
- [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) — created; relevance: captures atomic stale-writer rejection as a backend-neutral prerequisite; current state: `proposed`.
- [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) — created; relevance: defines the durable multi-unit boundary required by most derivative applications; current state: `proposed`.
- [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md) — created; relevance: separates configurable process measurements from structural transition policy; current state: `proposed`.
- [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) — created; relevance: separates multi-board and pipeline relationships from a unit’s phase graph; current state: `proposed`.
- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — selected; relevance: its realized lifecycle kernel and explicit exclusions define the boundary that the appendix must preserve; current state: `realized`.
- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — selected; relevance: its one-shot CLI is useful evidence but must not be presented as a durable application backend; current state: `realized`.

## 1. Sprint Goal

Distill the user-provided, out-of-order Discord excerpts into a durable but
non-binding architecture map. Sprint 6 will preserve the strongest reusable
CubiKan backend gaps as proposed intent chapters and, during Build, publish a
Book appendix titled “Potential Derivative Projects.” The appendix will
recommend repository boundaries for agent operations, Git/provenance analytics,
process design, skill graphs, organizational applications, and Animus accounting
and state exactly how each could consume CubiKan. This sprint changes no Rust
code, runtime protocol, dependency, realized product contract, or derivative
repository. It preserves local remote configuration; the normal push of the
existing CubiKan `dev` branch remains the authorized Test-phase quality
checkpoint.

The excerpts are treated as user-authorized ideation input, not as a complete
transcript or proof of exact conversational context. Research retains themes and
architectural inferences without publishing participant identities, channel
details, private locators, or verbatim message bodies.

### Retained theme inventory

This sanitized inventory is the completeness oracle for Sprint 6. It classifies
the themes retained from the incomplete excerpts; it does not claim to reconstruct
the omitted conversation.

| Theme | Classification | Retained meaning | Owning boundary or disposition |
|-------|----------------|------------------|--------------------------------|
| `DV-01` | retained | CubiKan could become a common lifecycle/accounting core beneath work-management systems. | Future backend capability plus `cubikan-agent-ops` and `animus-ledger`; the Book remains current authority. |
| `DV-02` | retained | A manager/keeper can maintain intent and evidence while doers execute bounded work units. | `cubikan-agent-ops` and `animus-ledger`; agent identity, delegation, permissions, and execution policy remain derivative-owned. |
| `DV-03` | retained | Intent Units should be traceable to resulting commits and artifacts, with blame usable as an analytical input for improving agents. | INT-0008 and `cubikan-observatory`; blame remains derived evidence, not causality. |
| `DV-04` | retained | Different processes have different meaningful checkpoints, conversion funnels, attainment measures, and ratios. | INT-0011 and `cubikan-process-studio`; definitions and authorization policy remain caller/application-owned. |
| `DV-05` | retained analogy | CubiKan records rate-of-change facts while the Book integrates semantic and historical evidence. | Integration authority map; operational truth migration remains an open, separately authorized outcome. |
| `DV-06` | retained | Multiple boards and targeted skills can compose into execution pipelines or “graph engineering.” | INT-0012 and `cubikan-skill-graph`; these edges are not `WorkflowEdge`. |
| `DV-07` | retained | The same backend could support management and bounded organizational applications while the core stays separate from frontends. | INT-0010, `cubikan-process-studio`, and `cubikan-org-app-kit`. |
| `DV-08` | merged | Recursive or nested loop composition could connect agent operations, skills, and Sprint Loops accounting. | Merged into Agent Ops, Skill Graph, and Animus Ledger; no parent-child lineage is inferred in the current core. |
| `DV-09` | deferred/open | Blockchain-backed coordination or audit could eventually be an adapter. | No repository recommendation until chain, trust, key, cost, finality, and data-placement policy are selected. |

## 2. Existing Code Survey

| File | Relevance | Finding |
|------|-----------|---------|
| `README.md` | high | CubiKan currently exposes a chain-agnostic core and one-shot in-memory CLI while explicitly excluding persistence, workflow registries, services, UI, authorization, KPI evaluation, and lineage. |
| `Cargo.toml` | medium | The repository is one Rust workspace with core and CLI crates; Sprint 6 requires no package or dependency change. |
| `crates/cubikan-core/src/lib.rs` | high | The public kernel exports IDs, vocabulary, immutable workflow definitions, lifecycle records, and typed transition/completion errors only. |
| `crates/cubikan-core/src/workflow.rs` | high | `WorkflowEdge` connects phases inside one immutable workflow snapshot; it is not a dependency, delegation, or multi-board graph edge. |
| `crates/cubikan-core/src/intent_unit.rs` | high | `IntentUnit` stores ID, species, workflow, phase, status, and sequence-only in-memory history; it has no source intent, revision, actor, timestamp, evidence reference, or cross-unit relationship. |
| `crates/cubikan-cli/README.md` | high | The CLI guide expressly denies persistence, sessions, networking, authorization, stable compatibility, and acknowledged/durable output. |
| `crates/cubikan-cli/src/protocol.rs` | high | A v1 request configures and executes one whole scenario; its response snapshot has workflow ID but not reusable workflow topology or provenance. |
| `crates/cubikan-cli/src/execution.rs` | high | The adapter delegates lifecycle validity to core and projects only current unit state/history; it is not a repository or application service. |
| `docs/intents/INT-0001-chain-agnostic-intent-lifecycle-core.md` | high | The realized core intent explicitly defers durable audit, persistence, services, concurrency, KPI evaluation, lineage, and stable wire formats. |
| `docs/intents/INT-0002-runnable-lifecycle-adapter.md` | high | The realized adapter is an execution boundary, not a persistence/session abstraction, and must not become the derivative integration contract by implication. |
| `docs/intents/INT-0003-bounded-cli-request-ingestion.md` | medium | The 1 MiB request bound is local ingestion hardening, not network-service readiness or total resource control. |
| `docs/intents/INT-0004-explicit-cli-response-flush.md` | medium | Writer-flush checking is not persistence, acknowledgement, transactional output, or cross-process state. |
| `docs/intents/INT-0005-automated-rust-quality-gate.md` | medium | Existing CI can verify this prose-only sprint without granting release, deployment, repository-creation, or merge authority. |
| `docs/intents/INT-0006-distinguish-omitted-cli-id.md` | medium | The latest realized intent reinforces that the CLI protocol is experimental and should not be enlarged into a backend contract in this sprint. |
| `docs/README.md` | medium | The Book is the semantic authority for project intent, executable work, realization evidence, and sprint provenance. |
| `docs/SUMMARY.md` | high | Navigation currently exposes realized intents and sprint history; it has no appendix or derivative-project view. |
| `docs/work/tasks.md` | medium | The persistent backlog is empty after Sprint 5, so proposed intents can remain unscheduled without creating hidden executable work. |
| `docs/work/completed-tasks.md` | high | Completed-task entries already approximate intent-to-commit provenance through intent IDs, modified files, and commit SHAs, but this relationship is manual Book evidence rather than a validated/queryable CubiKan model. |
| `docs/sprints/s5/sprint-tests/test-report.md` | medium | The accepted baseline is 100 all-target tests, one doctest, valid Book navigation, and a clean critic before this documentation-only exploration. |

The survey establishes three graph types that must not be conflated:

1. a `WorkflowEdge` connects two phases of one Intent Unit;
2. a future provenance graph connects intents, activities, agents, and artifacts;
3. a derivative execution graph connects work units, boards, skills, or agents.

It also sharpens the discussion’s “Kanban as rate of change, Book as integral”
analogy: CubiKan can own validated state changes and queryable observations,
while the Book remains durable semantic and historical evidence until an
explicit projection/migration intent says otherwise.

## 3. External Sources

- User-provided Discord excerpts in this session — primary product-design signal; intentionally incomplete/out of order, so only sanitized themes and inferences are retained.
- [Git `interpret-trailers`](https://git-scm.com/docs/git-interpret-trailers) — Git provides machine-parseable structured commit-message trailers, making an intent/unit reference feasible without inventing a commit format inside CubiKan.
- [Git notes](https://git-scm.com/docs/git-notes) — notes can attach supplemental metadata to objects without changing the objects, but live under separate refs and therefore require explicit transport, merge, and authority policy.
- [W3C PROV-DM](https://www.w3.org/TR/prov-dm/) — its provider-neutral distinction among entities, activities, agents, derivation, and association supports separating provenance from lifecycle topology and from later attribution analysis.
- [OpenTelemetry signals](https://opentelemetry.io/docs/concepts/signals/) — the separation of traces, metrics, logs, and contextual baggage supports storing lifecycle facts, observations, and derived process metrics as correlatable but non-interchangeable data.

These sources inform boundaries rather than select an implementation standard.
Git trailers versus notes, PROV compatibility, and OpenTelemetry export remain
future adapter decisions.

Pre-Build remote baseline captured on 2026-08-08: `origin` fetch and push both
use `https://github.com/crussella0129/CubiKan`; its local fetch refspec is
`+refs/heads/*:refs/remotes/origin/*`; `git submodule status` is empty. Test will
compare the final local configuration to this baseline and separately record the
remote operations actually issued during Sprint 6.

## 4. Risks, Unknowns, Dependencies

- **Risk — vocabulary collision:** A Project Book `INT-NNNN` and a CubiKan
  `IntentUnitId` both use “intent” but are not interchangeable identities. Every
  future mapping must retain namespaces and direction.
- **Risk — graph collision:** Phase topology, provenance, delegation, and
  pipeline dependencies have different invariants. Reusing `WorkflowEdge` for
  all of them would corrupt the current domain model.
- **Risk — blame overreach:** Git blame is a useful derived query, not proof of
  original intent, agent contribution, quality, or causality. Repository moves,
  squashes, generated code, review, and shared commits all complicate attribution.
- **Risk — analytics governance:** Agent-improvement scores can expose private
  prompts, tools, cost, mistakes, and human review. Raw observations, derived
  metrics, recommendations, and automatic adaptation need separate access and
  approval boundaries.
- **Risk — false backend claim:** The current v1 CLI cannot resume a unit and
  omits workflow topology from its output snapshot. Calling it the application
  backend would contradict INT-0002.
- **Risk — metric ambiguity:** Conversion, attainment, and cycle time require
  explicit denominator, window, event-time, correction, missing-data, unit, and
  source semantics. A generic phase name does not define a KPI.
- **Risk — repository sprawl:** A repository is justified by distinct runtime,
  deployment, security/data authority, or release cadence—not merely by a Rust
  crate or a clever name.
- **Unknown:** Whether the Book or a future CubiKan event store ultimately owns
  operational task/completion truth. The appendix must preserve the Book’s
  current authority, prohibit Book/backend dual-write, and require a later
  migration/projection intent before any datum changes canonical authority.
- **Unknown:** Git evidence may use trailers, notes, a sidecar index, host APIs,
  or a combination. Reachability, rewriting, private repositories, retention,
  and verification remain unresolved.
- **Unknown:** A durable backend still needs explicit storage, schema migration,
  consistency, idempotency, transport, authentication, tenancy, backup, and
  deployment choices.
- **Unknown:** Checkpoint observations may be informational or may later feed an
  authorization policy. Sprint 6 keeps them informational and derived.
- **Dependency:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
  is a hard prerequisite for [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md):
  stale expected revisions must win before lifecycle-command evaluation, while a
  current revision preserves the existing typed domain rejection.
- **Dependency:** INT-0008 can begin with read-only origin references, but its
  full revision-scoped and bidirectional artifact index depends on INT-0009 and
  INT-0010. INT-0011 depends on INT-0009 and INT-0010; INT-0012 depends on
  INT-0010. These are a partial order, not one universal linear chain.
- **Dependency:** Agent Ops and organizational clients need INT-0010’s bounded
  collection query for basic queues and projections; advanced multi-board views
  additionally need INT-0012. A local Process Studio can embed an explicitly
  pinned core version before persistence, but shared operational/KPI use needs
  INT-0010 and INT-0011.

## 5. Recommended Approach

Keep CubiKan as the reusable lifecycle kernel and evolve a future backend only
through the five proposed capability intents. Their recommended partial order is:

1. explore provider-neutral, read-only origin references under [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) independently;
2. establish revisioned lifecycle commands under [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md);
3. build a durable multi-unit backend under [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md) only after INT-0009;
4. complete INT-0008’s revision-scoped, bidirectional evidence index after INT-0009 and INT-0010; and
5. add lifecycle observations/metric evidence after INT-0009 and INT-0010, and typed relationships/board projections after INT-0010 ([INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md), [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)).

During Build, create one advisory appendix with six recommended derivative
boundaries:

- `cubikan-agent-ops` for manager/doer assignment, execution attempts, queues,
  approvals, and operational accounting;
- `cubikan-observatory` for Book/Git/CI provenance, blame-assisted exploration,
  and governed agent-improvement analytics;
- `cubikan-process-studio` for process/workflow version design, business-owned
  KPI definitions and authorization policy, and an Electron-first configuration
  experience;
- `cubikan-skill-graph` for multi-board skill DAGs, readiness, routing,
  execution, retry, and join/fan-out policy;
- `cubikan-org-app-kit` as one primary reusable-client recommendation, with
  separately authorized bounded-domain repositories as a future pattern;
- `animus-ledger` for Sprint Loops/Animus reconciliation and accounting, after
  “accounting” has an explicit unit, trust, correction, and anti-gaming model.

Every entry will state that CubiKan owns identity and validated lifecycle state;
the derivative owns actors, UI, domain records, scheduling, privacy, and policy.
A derivative may embed the current public `cubikan-core` API at an explicitly
pinned crate version for local validation or use a future versioned CubiKan
command/query/evidence boundary. This is not a cross-version Rust API promise.
It must not edit CubiKan storage, deserialize provisional core state as a durable
contract, or treat the current one-shot CLI as a session service.

Process Studio authors and governs business measurement definitions and
authorization policy. A future backend may store raw observations and
deterministically evaluate caller-supplied, versioned definitions; Observatory
consumes the resulting evidence for analysis rather than becoming the business
policy authority. Each datum has one canonical owner. The Book remains the
current semantic and historical authority, with no Book/backend dual-write and
no migration of operational truth without a separate projection/migration intent.

`cubikan-observatory` can bootstrap read-only from existing Book and Git evidence
before durable CubiKan state exists, but any agent-scoring or adaptation feature
must first establish data minimization, retention/redaction, access control, and
human approval. A local Process Studio can validate definitions against a pinned
core version; shared Agent Ops, operational Process Studio, Skill Graph, and
organizational applications should wait for the relevant durable query/relation
boundaries. Animus accounting should follow trustworthy provenance and a resolved
accounting model; nested-loop composition remains a derivative/open design, not
inferred core lineage.

Alternatives considered: a single “CubiKan platform” repository would collapse
independent trust and runtime boundaries; a repository for every adapter or
crate would create needless maintenance; putting the vision only into intent
chapters would not provide the requested comparative appendix; and creating
repos now would turn exploration into an unapproved external commitment.

## Artifacts

- [INT-0007 — Define the CubiKan derivative ecosystem](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- [INT-0008 — Traceable intent instantiation and artifact provenance](../../../intents/INT-0008-traceable-intent-instantiation.md)
- [INT-0009 — Revisioned lifecycle commands and atomic conflict rejection](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
- [INT-0010 — Durable multi-unit CubiKan backend](../../../intents/INT-0010-durable-intent-unit-backend.md)
- [INT-0011 — Lifecycle checkpoints and metric evidence](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md)
- [INT-0012 — Intent Unit relationships and board projections](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- Planned Build artifact: `docs/appendix/potential-derivative-projects.md` (not created during Research).
