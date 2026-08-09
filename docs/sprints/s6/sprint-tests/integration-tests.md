# Sprint 6 Integration Test Results

- **Primary intent:** [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- **Proposed capability boundaries:** [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md), [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md), [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md), [INT-0011](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md), and [INT-0012](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md)
- **Accepted base:** `bf5b5f299102d4853fa5312b1091ec0b8fb2dfe1`
- **T-601 commits:** semantic `5cc52aba625acc9e0361014eca8aec0edbe55554`; ledger evidence `cbf2ac44bae15bb3219cd01a6731693ad217a9f5`
- **T-602 commits:** semantic `f1770f774bfafed538316f01c3cd05cd82270855`; ledger evidence `f511dab01ff44d21ce7eb71237d0fd63763d8e5c`
- **T-603 commits:** semantic `f38e974a903f9e4a0cac8a63778c0426877571b5`; ledger evidence and tested Build head `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7`
- **Integration boundary:** the current Project Book composition—sanitized Research inventory, intent chapters, advisory appendix, and navigation—not a derivative runtime or an external derivative repository.
- **Result:** pass

The four locked cross-document checks used the committed Build head and the
current Book as their source oracle. They were one-off semantic inspections;
they did not replace the Book with an implementation-mirroring test fixture.

| Named test | Arrangement | Assertions | Result |
|------------|-------------|------------|--------|
| `test_theme_to_intent_to_repository_traceability` | Join Research items `DV-01` through `DV-09` to the appendix retained-theme table, proposed capability intents, six primary repository entries, and deferred/open dispositions. | Every bounded theme has one retained, merged, retained-analogy, or deferred/open disposition and reaches an explicit backend/adapter boundary plus a derivative, alternative, or open question. No completeness claim is made about omitted chat context. | pass |
| `test_graph_and_authority_boundaries_are_distinct` | Compare current core vocabulary, the authority map, INT-0008 through INT-0012, and all six catalog entries. | Phase topology, provenance, delegation, cross-unit relations, execution DAGs, measurements, accounting, analytics, and Book semantics retain separate canonical authorities; no consumer dual-writes or silently reinterprets another authority's datum. | pass |
| `test_derivative_catalog_has_six_complete_unique_entries` | Parse every primary `Recommended repository` entry and each entry's named ownership/integration fields. | Exactly six unique primary slugs exist; each has complete problem/outcome, owned-data, owned-policy, input, output, CubiKan-interaction, prerequisite, creation-trigger, separation, non-goal, and related-intent fields; unnamed organizational verticals are not extra recommendations. | pass |
| `test_each_derivative_maps_to_declared_cubikan_capabilities` | Compare each catalog entry's `CubiKan interaction`, `Prerequisites`, `Creation trigger`, and `Related intents` against the partial order in the capability map. | Every derivative uses either an explicitly pinned current core for a bounded local experiment or the declared future versioned capability intents, and no shared/operational creation sequence precedes a hard prerequisite. | pass |

## `test_theme_to_intent_to_repository_traceability`

**Coverage:** T-601-E4, T-602-E1 through T-602-E4, and T-603-E1 through
T-603-E5.

The bounded oracle is the enumerated, sanitized inventory in Sprint 6
Research. The following trace was present in the committed appendix:

| Theme | Research disposition | Backend or adapter boundary | Repository recommendation or disposition | Result |
|-------|----------------------|-----------------------------|-------------------------------------------|--------|
| `DV-01` | retained | INT-0010 is the possible common lifecycle backend; the Book remains the current semantic and historical authority. | Agent Ops coordinates work; Animus Ledger reconciles evidenced work. | pass |
| `DV-02` | retained | Book intent and Intent Unit identities remain namespaced; future mapping is provenance rather than identity equivalence. | Agent Ops owns manager/doer execution; Animus consumes evidence without executing work. | pass |
| `DV-03` | retained | INT-0008 owns durable intent/unit/artifact associations; Git, Book, PR, CI, and test records remain provider evidence reached through adapters. | Observatory owns trace views and governed inference; blame is derived analysis, not causality. | pass |
| `DV-04` | retained | INT-0011 owns lifecycle-linked observations and deterministic evaluation of caller-supplied versioned definitions. | Process Studio authors and governs measurement definitions; Observatory analyzes results. | pass |
| `DV-05` | retained analogy | The data-authority map keeps the Book canonical until a separately selected projection or migration intent defines reconciliation and cutover. | Animus derives plan-versus-actual reconciliation and never dual-writes Book history. | pass |
| `DV-06` | retained | INT-0012 owns reusable cross-unit relations and board projections, distinct from one-unit `WorkflowEdge` topology. | Skill Graph owns executable DAG and multi-board routing policy. | pass |
| `DV-07` | retained | INT-0010 supplies a future durable lifecycle command/query boundary and bounded collection queries. | Process Studio and Organizational App Kit remain separate authoring and client/application surfaces. | pass |
| `DV-08` | merged | INT-0012 may later hold explicit grouping or dependency relations; current phase/history never implies parent-child lineage. | Recursive composition is divided among Agent Ops delegation, Skill Graph execution, and Animus reconciliation; exact recursive semantics remain open. | pass |
| `DV-09` | deferred/open | Blockchain remains an unselected adapter concern pending chain, trust, key, cost, finality, and data-placement decisions. | No blockchain derivative repository is recommended or claimed to exist. | pass |

This proves complete traceability for the deliberately bounded `DV-01` through
`DV-09` inventory. It does not claim that the out-of-order excerpts preserved
the omitted Discord conversation or that the catalog is a complete product
roadmap.

## `test_graph_and_authority_boundaries_are_distinct`

**Coverage:** T-601-E2, T-601-E3, T-601-E6, T-602-E2 through T-602-E4,
and T-603-E2 through T-603-E5.

The check arranged the realized core vocabulary, proposed intent ownership,
appendix authority table, and derivative contracts side by side:

| Concern | Canonical authority | Consumer boundary proved at Build head | Result |
|---------|---------------------|----------------------------------------|--------|
| Product semantics and current realization history | Project Book | CubiKan and derivatives reference or project Book identifiers; they do not replace or dual-write the Book. | pass |
| One-unit lifecycle topology and state | Current `cubikan-core` aggregate; `WorkflowEdge` joins phases inside one Intent Unit | No provenance, delegation, relation, board, pipeline, or recursive edge is represented as `WorkflowEdge`. | pass |
| Future revisions and durable lifecycle state | INT-0009 followed by INT-0010 | Derivatives use a future versioned command/query boundary and never shared writable storage, provisional Serde, or the one-shot CLI as a session. | pass |
| Provenance association | INT-0008, revision-scoped and bidirectional only with INT-0009 and INT-0010 | External providers remain authoritative for Git/PR/CI records; Observatory stores rebuildable correlations and derived analysis. | pass |
| Delegation and authorization envelope | Agent Ops | Manager/doer identity, decomposition, assignment, permissions, approvals, retries, and aggregate cost policy remain outside CubiKan and Skill Graph node execution. | pass |
| Cross-unit relations and board projections | INT-0012, after INT-0010 | Agent Ops and Skill Graph consume explicit relations; neither infers lineage from phase order or history. | pass |
| Execution DAG and node runtime | Skill Graph | Pipeline, node, routing, readiness, scheduling, retry, sandbox, and artifact-flow policy do not become lifecycle state. | pass |
| Measurement definitions and business authorization | Process Studio or another authoring caller | INT-0011 may store observations and deterministically evaluate only caller-supplied versioned definitions; it does not invent business policy. | pass |
| Raw observations and deterministic metric results | Future INT-0011 evidence backend | Observatory may interpret results but does not rewrite observations or lifecycle state. | pass |
| Analytical blame, attribution, scoring, and recommendations | Observatory | Derived claims remain governed analysis, never certified causality or automatic agent mutation. | pass |
| Operational accounting | Animus Ledger | Accounting definitions, valuation, reconciliation, corrections, anti-gaming, and approval remain outside lifecycle validation. | pass |
| Bounded-domain business data and policy | Each independently authorized organizational application | Organizational App Kit owns reusable client contracts only; it owns no domain record, PII, RBAC, retention, integration, report, deployment, or vertical UX. | pass |

Every datum therefore has one canonical authority. The Book remains the current
semantic/historical authority, Book/backend dual-write is prohibited, and a
separately selected migration or projection intent is required before any
operational truth changes owner.

## `test_derivative_catalog_has_six_complete_unique_entries`

**Coverage:** T-602-E1 through T-602-E4 and T-603-E1 through T-603-E4.

Parsing primary `Recommended repository` labels produced exactly this set:

1. `cubikan-agent-ops`
2. `cubikan-observatory`
3. `animus-ledger`
4. `cubikan-process-studio`
5. `cubikan-skill-graph`
6. `cubikan-org-app-kit`

For each slug, the check found one non-placeholder value under every required
contract field: `Problem and outcome`, `Owned data`, `Owned policy`, `Inputs`,
`Outputs`, `CubiKan interaction`, `Prerequisites`, `Creation trigger`,
`Separation rationale`, `Explicit non-goals`, and `Related intents`. Each entry
also has exactly one `Recommended repository` field. The ownership comparison
confirmed that shared values are exchanged as IDs, immutable references,
commands, evidence, or rebuildable projections; no two entries claim canonical
ownership of the same source datum or policy.

`cubikan-org-app-kit` is the sixth primary recommendation. Its possible
bounded-domain verticals remain an unnamed, independently authorized pattern,
so they did not create a hidden seventh slug. All six entries and all required
ownership/integration fields passed.

## `test_each_derivative_maps_to_declared_cubikan_capabilities`

**Coverage:** T-601-E3, T-601-E4, T-602-E2 through T-602-E4, and T-603-E2
through T-603-E4.

The locked capability partial order is INT-0009 before INT-0010; full
revision-scoped, bidirectional INT-0008 requires both; INT-0011 requires
INT-0009 and INT-0010; and INT-0012 requires INT-0010. A read-only INT-0008
exploration is the only independent capability experiment described.

| Repository | Permitted current or future CubiKan capability | Non-CubiKan prerequisite and creation gate | Result |
|------------|------------------------------------------------|--------------------------------------------|--------|
| `cubikan-agent-ops` | A local experiment may pin the current public core. Shared/resumable work needs INT-0009 and INT-0010; durable advanced decomposition, nested-loop, or multi-board relations need INT-0012; reusable artifact provenance needs INT-0008. | Requires manager/doer identity and authorization, Book-to-unit reconciliation, privacy/retention/secrets, approvals, and cost semantics; creation waits for an authorized multi-manager/doer need and an owned INT-0010 boundary. | pass |
| `cubikan-observatory` | A read-only Book/Git bootstrap need not write CubiKan. Full revision-scoped bidirectional provenance waits for INT-0008, INT-0009, and INT-0010; INT-0011 may later supply observation evidence. | Requires immutable namespace/provider rules and source access controls; score publication or adaptation additionally requires data minimization, retention/deletion, redaction, access control, and named human approval. | pass |
| `animus-ledger` | Uses a future versioned evidence/query boundary backed by INT-0008, INT-0009, and INT-0010; INT-0011 may supply observations and INT-0012 may supply explicit relations, while Animus retains accounting interpretation. | Requires an accepted accounting charter, trustworthy end-to-end provenance, corrections, anti-gaming, access/retention, and human approvals; creation waits for a real repeatable reconciliation consumer. | pass |
| `cubikan-process-studio` | Local structural validation may pin the current public core. Shared operational/KPI activation waits for INT-0009, INT-0010, and INT-0011; Studio supplies definitions while the future backend evaluates them. | Requires definition/version compatibility, immutable workflow pinning, process-owner authorization, complete measurement/correction semantics, privacy/retention, and an authorized repeated multi-process authoring need. | pass |
| `cubikan-skill-graph` | Shared multi-unit execution needs INT-0010 and INT-0012, with INT-0009 revisions; INT-0008 is conditional when bidirectional artifact provenance is required. | Requires the Agent Ops authorization envelope plus graph/cycle, fan-out/join, failure, idempotency, executor trust, capacity, secrets, sandbox, and artifact policies; creation waits for a repeated authorized persisted-unit graph use. | pass |
| `cubikan-org-app-kit` | Operational clients need INT-0009 and INT-0010; basic projections use INT-0010 bounded queries; advanced boards/typed relations wait for INT-0012. A local demo may pin the current core but is not a shared backend. | Requires client compatibility plus bounded-domain identity, authorization, privacy, retention, deployment, and support decisions; creation waits for repeated cross-application client/projection needs and a usable INT-0010 boundary. | pass |

No derivative is sequenced ahead of an unmet hard capability or governance
prerequisite. References to a pinned current core are bounded local experiments,
not persistence, a service, or a cross-version Rust API promise.

## `test_hosted_sprint_six_quality_run_succeeds`

**Coverage:** Test-phase remote checkpoint and preservation of the existing
[INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) hosted
quality boundary.

GitHub received the exact committed Sprint 6 Build head through the authorized
push to the existing `dev` branch. The GitHub API reported the run and its sole
job at the same exact revision:

| Field | Observed value |
|-------|----------------|
| Remote `dev` / tested Build SHA | `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7` |
| Run | [31293927701 — Rust CI](https://github.com/crussella0129/CubiKan/actions/runs/31293927701) |
| Event / branch | `push` / `dev` |
| Run attempt | `1` |
| Run status / conclusion | `completed` / `success` |
| Run created / started | `2026-08-09T04:08:25Z` / `2026-08-09T04:08:25Z` |
| Run updated | `2026-08-09T04:09:09Z` |
| Sole job | [93195790436 — Rust quality gate](https://github.com/crussella0129/CubiKan/actions/runs/31293927701/job/93195790436) |
| Job status / conclusion | `completed` / `success` |
| Job started / completed | `2026-08-09T04:08:29Z` / `2026-08-09T04:09:08Z` |
| API run head SHA | `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7` |
| API job head SHA | `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7` |

The job's setup, repository checkout, and stable-Rust installation steps all
completed successfully. Each of the five configured quality steps also
completed successfully:

| Hosted quality step | Result |
|---------------------|--------|
| Check formatting — `cargo +stable fmt --all -- --check` | pass |
| Run Clippy — `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | pass |
| Check workspace — `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | pass |
| Run workspace tests — `cargo +stable test --workspace --all-targets` | pass |
| Run workspace doctests — `cargo +stable test --doc --workspace` | pass |

There were no mocks or stubs at this boundary. GitHub Actions and the GitHub
API were the real external system, and the API's run and job objects both named
the exact pushed Build SHA. This hosted oracle proves only that the repository's
existing Rust CI workflow passed at the Sprint 6 Build head. It does not prove
that any derivative runtime or repository exists, that the prose architecture
has runtime behavior, or that no unrelated external mutation occurred.
