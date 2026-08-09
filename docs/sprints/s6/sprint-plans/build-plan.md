Finalized - DO NOT EDIT

# Sprint 6 Build Plan

## Intents

- [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) — state: `planned`; acceptance criteria covered: advisory appendix authority, current CubiKan boundary, complete derivative catalog, future backend-intent map, integration direction, navigation, and prose-only scope.
- [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md), [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md), and [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) — state: `proposed`; referenced as unscheduled future capability boundaries only. Sprint 6 does not advance or implement their acceptance criteria.

## Schema Tree

- Define the CubiKan derivative ecosystem
  - Appendix foundation and integration contract
    - T-601: Establish authority, current boundary, backend capability map, and navigation
  - Agent, provenance, and recursive-loop derivatives
    - T-602: Document Agent Ops, Observatory, and Animus Ledger
  - Process, graph, and organizational derivatives
    - T-603: Complete the catalog, sequencing, and non-commitment boundary

## Execution Sequence

### T-601: Establish the appendix authority and CubiKan integration baseline

- **Intent:** [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- **Touches:** `docs/appendix/README.md`, `docs/appendix/potential-derivative-projects.md`, `docs/SUMMARY.md`, `docs/intents/INT-0007-define-cubikan-derivative-ecosystem.md` through `docs/intents/INT-0012-intent-unit-relationships-and-board-projections.md`, `docs/sprints/s6/**`, `docs/work/tasks.md`, `docs/work/completed-tasks.md`
- **Depends on:** (none)
- **Acceptance criterion:** The appendix has an advisory authority banner, accurately states the current core/one-shot-CLI boundary, separates backend/adapters/derivatives, links the five proposed backend intents through a dependency map, defines the safe derivative integration direction, and is reachable from the Book.
- **Success criterion (EARS):**
  - **T-601-E1 — WHEN** a reader opens `Potential Derivative Projects`, **THEN** the appendix **SHALL** use that exact title, identify recommendations as non-binding, deny that any listed repository or future backend already exists, and name Project Book intent chapters as semantic authority.
  - **T-601-E2 — WHEN** the current platform section is compared with realized INT-0001 and INT-0002, **THEN** it **SHALL** describe `cubikan-core` as a chain-agnostic validated lifecycle kernel and `cubikan` as an experimental one-shot in-memory adapter, with no persistence, service, actor, metric, cross-unit graph, UI, or blockchain claim.
  - **T-601-E3 — WHEN** the integration baseline is reviewed, **THEN** it **SHALL** assign identity and lifecycle validation to CubiKan, assign domain/UI/orchestration policy to derivatives, permit either the current public core API at an explicitly pinned crate version or a future versioned command/query/evidence boundary, deny a cross-version Rust API promise and shared-database/provisional-Serde coupling, and distinguish one-unit `WorkflowEdge` phase topology from provenance, delegation, cross-unit, and execution edges.
  - **T-601-E4 — WHEN** future CubiKan prerequisites are inspected, **THEN** the appendix **SHALL** link INT-0008 through INT-0012, keep all five `proposed`, show the partial order in which INT-0009 precedes INT-0010, full revision-scoped/bidirectional INT-0008 depends on INT-0009 and INT-0010, INT-0011 depends on INT-0009 and INT-0010, and INT-0012 depends on INT-0010, and list the human-policy tripwires that prevent implementation by implication.
  - **T-601-E5 — WHEN** Book navigation is traversed, **THEN** `docs/SUMMARY.md` **SHALL** link one appendix index and one Potential Derivative Projects page exactly once, and every local link introduced by T-601 **SHALL** resolve.
  - **T-601-E6 — WHEN** the data-authority map is reviewed, **THEN** every datum **SHALL** have one canonical owner, the Book **SHALL** remain the current semantic and historical authority, Book/backend dual-write **SHALL** be prohibited, and any move of operational truth **SHALL** require a separately selected projection or migration intent.
- **Notes:** The appendix directory is a navigation surface, not a second decision store. No derivative repository is created, contacted, or authorized. Because Sprint 6 Research and Plan artifacts are uncommitted phase output, T-601’s semantic commit also records the initialized Sprint 6 Book, research, INT-0007–INT-0012, finalized plans/critique/test placeholders, Summary navigation, intent activation/meta, and queued work ledgers as one coherent first-task boundary.

### T-602: Document agent operations, provenance analytics, and Animus accounting derivatives

- **Intent:** [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- **Touches:** `docs/appendix/potential-derivative-projects.md`
- **Depends on:** T-601
- **Acceptance criterion:** The catalog covers manager/doer work operations, Git/intent provenance plus governed agent analytics, and Sprint Loops/Animus accounting with complete, separate repository contracts.
- **Success criterion (EARS):**
  - **T-602-E1 — WHEN** the three agent/accounting entries are compared, **THEN** each **SHALL** have a unique recommended repo slug, problem/outcome, owned data, owned policy, inputs, outputs, CubiKan interaction, prerequisites, creation trigger, separation rationale, and explicit non-goals.
  - **T-602-E2 — WHEN** `cubikan-agent-ops` is reviewed, **THEN** it **SHALL** keep manager/doer identity, decomposition, assignment, scheduling, tools, permissions, retries, approvals, and cost observations outside CubiKan while representing assigned work through Intent Units.
  - **T-602-E3 — WHEN** `cubikan-observatory` is reviewed, **THEN** it **SHALL** correlate distinct Book intent and Intent Unit identities with full Git revisions, PR/CI/test evidence, and agent traces; classify blame and scores as derived analysis; avoid causal, automatic-improvement, or evidence-certification claims; and make data minimization, retention/redaction, access control, and human approval prerequisites for agent scoring or adaptation.
  - **T-602-E4 — WHEN** `animus-ledger` is reviewed, **THEN** it **SHALL** keep Sprint Loops Book parsing, plan-versus-actual reconciliation, unit-of-account, cost/credit, correction, anti-gaming, and approval authority outside CubiKan, require trustworthy provenance before any accounting claim, and map nested/recursive loop composition to Agent Ops, Skill Graph, and INT-0012 or label it an open question without inferring core parent-child lineage.
- **Notes:** Agent Ops coordinates current work; Animus Ledger reconciles evidenced work. They may integrate but are not collapsed into one repository recommendation.

### T-603: Complete process, skill-graph, and organizational recommendations and sequence the ecosystem

- **Intent:** [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- **Touches:** `docs/appendix/potential-derivative-projects.md`
- **Depends on:** T-602
- **Acceptance criterion:** The catalog covers configurable process/KPI design, multi-board skill graphs, and organizational applications and closes with a creation sequence, triggers, merged/rejected candidates, open questions, and explicit non-goals.
- **Success criterion (EARS):**
  - **T-603-E1 — WHEN** the three process/application entries are compared, **THEN** each **SHALL** have a unique recommended repo slug, problem/outcome, owned data, owned policy, inputs, outputs, CubiKan interaction, prerequisites, creation trigger, separation rationale, explicit non-goals, and links to the relevant proposed CubiKan capability intents.
  - **T-603-E2 — WHEN** `cubikan-process-studio` is reviewed, **THEN** it **SHALL** own process definition/versioning, business measurement definitions and authorization policy, and an Electron-first configuration UX while translating selected immutable definitions into core-validated workflows; a future backend may store raw observations and deterministically evaluate only caller-supplied versioned definitions, Observatory may consume results, local design may use a pinned core before persistence, and shared operational/KPI use **SHALL** wait for INT-0010 and INT-0011.
  - **T-603-E3 — WHEN** `cubikan-skill-graph` is reviewed, **THEN** it **SHALL** distinguish pipeline/unit/board dependency edges from `WorkflowEdge` and own skill manifests, readiness, routing, scheduling, fan-out/join, retries, execution, sandbox, and artifact policy.
  - **T-603-E4 — WHEN** `cubikan-org-app-kit` and its future vertical-repository pattern are reviewed, **THEN** the one primary app-kit entry **SHALL** use CubiKan for lifecycle state and INT-0010’s bounded collection query for basic projections, require INT-0012 for advanced multi-board relations, and keep domain records, PII, retention, RBAC, integrations, notifications, reports, deployment, and UX in separately authorized bounded domains.
  - **T-603-E5 — WHEN** the completed appendix is reviewed as a whole, **THEN** it **SHALL** contain exactly six primary recommendations, map every item in the sanitized retained-theme inventory, order recommendations by prerequisites and evidence, state creation triggers and rejected/merged alternatives, avoid conflicting system-of-record or policy ownership, preserve all current product nonclaims, and neither claim nor authorize derivative/external repository creation or mutation, package publication, release, or deployment.
  - **T-603-E6 — WHEN** the Sprint 6 candidate diff and local quality results are reviewed before the task commit, **THEN** changed paths **SHALL** stay within the declared prose-only allowlist, `.github`, Rust sources, manifests, lockfile, realized INT-0001–INT-0006 semantics, and remote configuration **SHALL** remain unchanged, and the existing five local Rust gates **SHALL** pass.
- **Notes:** Repository slugs are working recommendations. Splitting is justified by data authority, runtime/deployment, security/privacy, or release cadence—not by a crate boundary alone. `cubikan-org-app-kit` is the sixth primary entry; per-domain vertical repositories are an unnamed future pattern, not additional catalog slugs. Test will separately record the pre-Build remote baseline and the fact that execution issued no create/push/release/deploy operation for a derivative repository. After the committed Build head exists, the Test phase—not this pre-commit EARS check—will push the existing CubiKan `dev` branch and observe its exact-head hosted quality run.
