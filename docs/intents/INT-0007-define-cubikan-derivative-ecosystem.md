# INT-0007 — Define the CubiKan derivative ecosystem

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0007
- **State:** active
- **Work evidence:** [Sprint 6 build plan](../sprints/s6/sprint-plans/build-plan.md)
- **Completion evidence:** none
- **Code evidence:** none
- **Test evidence:** none
- **Documentation evidence:** none

## Intent

Publish a Project Book appendix that turns the user-provided design discussion
into a bounded map of potential derivative repositories and the CubiKan backend
capabilities they would consume. The map will distinguish the lifecycle kernel,
future backend responsibilities, adapters, and derivative applications so broad
ideas can evolve independently without turning CubiKan into a monolith.

This intent is a documentation outcome. It does not create any proposed
repository, schedule the proposed backend capabilities, or claim those systems
already exist. Stable future CubiKan outcomes belong in distinct proposed intent
chapters; repository names and sequences in the appendix remain recommendations.

## Acceptance criteria

- A Book appendix titled “Potential Derivative Projects” states its advisory
  authority and accurately describes the current CubiKan boundary as a
  chain-agnostic lifecycle library plus an experimental one-shot, in-memory CLI.
- The appendix classifies every retained design theme as a CubiKan backend
  capability, adapter concern, or derivative-project responsibility without
  treating lifecycle phase edges as delegation, provenance, or pipeline edges.
- The catalog covers manager/doer work operations, Git and intent provenance,
  agent-improvement analytics, configurable process and KPI design, multi-board
  skill graphs, organizational applications, and Sprint Loops/Animus accounting.
- Every recommended repository entry names its outcome, owned data and policy,
  CubiKan integration boundary, prerequisites, creation trigger, and explicit
  non-goals; repository slugs are recommendations rather than authorization.
- High-value reusable backend gaps discovered by the exploration are preserved
  as separate `proposed` intent chapters and linked through a dependency map.
- The appendix explains that derivative projects consume a future versioned
  CubiKan command/query/evidence boundary or embed the current public core at an
  explicitly pinned crate version; they do not share a database, depend on
  provisional core serialization, or infer a cross-version API promise.
- The integration map assigns one canonical authority to each datum, preserves
  the Book as the current semantic and historical authority, prohibits Book and
  backend dual-write, and requires a separate projection or migration intent
  before operational task/completion truth moves.
- The Book navigation, intent links, and local references validate; the
  accepted-base diff leaves Rust code, manifests, lockfile, CI, remote
  configuration, and realized intent semantics unchanged; and Sprint execution
  issues no create, push, publish, release, or deployment operation for a
  derivative repository.

## Rationale

The design discussion positions CubiKan as reusable process accounting beneath
agent systems, recursive sprint loops, configurable organizational workflows,
and analytics. Those ideas share lifecycle identity but not their policy,
runtime, privacy, release cadence, or user experience. A durable boundary map
captures the leverage without accepting a single oversized implementation.

## Alternatives

Putting every idea into the CubiKan repository was rejected because agent
execution, Git analysis, KPI policy, UI, and accounting have different trust and
deployment boundaries. Keeping the ideas only in sprint research would make
them hard to discover and would not provide the requested appendix. Creating
repositories now would prematurely choose names, ownership, technology, and
maintenance commitments.

## Consequences

The appendix will make future choices easier to compare, but it is not a
roadmap. Proposed backend intents still require their own research, plan,
implementation, and human checkpoints. Some recommended repositories may later
be merged, renamed, or never created when stronger evidence appears.

## Transition history

- 2026-08-08: created as `proposed` from the user-provided, intentionally incomplete Discord excerpts and the request for a prose-only derivative-project exploration sprint.
- 2026-08-08: moved to `planned` when Sprint 6 limited Build to an advisory appendix with separate backend-intent, repository-boundary, integration, and non-commitment checks.
- 2026-08-08: revised while `planned` after architecture review clarified pinned core compatibility and prohibited Book/backend split-brain authority.
- 2026-08-08: revised while `planned` after evidence review separated locally provable scope from the recorded operational non-creation boundary for derivative repositories.
- 2026-08-08: moved to `active` when Build queued T-601 through T-603 from the finalized clean Sprint 6 plans.
