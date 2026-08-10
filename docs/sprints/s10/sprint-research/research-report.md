# Sprint 10 Research Report

## Intents Reviewed

- [INT-0013 — Maintain derivative ecosystem current-state accuracy](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md) — created and selected as the follow-on maintenance authority; moved from proposed to planned for Sprint 10.
- [INT-0007 — Define the CubiKan derivative ecosystem](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) — reviewed, preserved as immutable Sprint 6 history, and legally moved `realized → superseded` by INT-0013 to eliminate competing live current-state semantics.
- [INT-0008 — Traceable Intent Instantiation](../../../intents/INT-0008-traceable-intent-instantiation.md) — reviewed, still proposed, and deliberately not selected for partial realization.
- [INT-0009 — Revisioned lifecycle commands](../../../intents/INT-0009-revisioned-lifecycle-commands.md) — reviewed as realized current-state evidence for guarded lifecycle revisions.
- [INT-0010 — Durable Intent Unit Backend](../../../intents/INT-0010-durable-intent-unit-backend.md) — reviewed as realized current-state evidence.
- [INT-0011 — Lifecycle Checkpoints and Metric Evidence](../../../intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md) — reviewed, still proposed, and deliberately not presented as realized.
- [INT-0012 — Intent Unit Relationships and Board Projections](../../../intents/INT-0012-intent-unit-relationships-and-board-projections.md) — reviewed as realized current-state evidence.

## 1. Sprint Goal

Refresh the non-authoritative `Potential Derivative Projects` appendix so its
current-state, capability-status, prerequisite, sequencing, and open-question
language recognizes the already realized revisioned lifecycle commands,
durable local SQLite backend, and relationship/projection API. The correction
must remain documentation-only: preserve INT-0007's historical advisory
outcome, supersede its stale current-state authority legally, leave runtime
product contracts and every other pre-existing intent state unchanged, and never imply that a
recommended derivative repository has been created or authorized.

## 2. Existing Code Survey

| File | Relevance | Finding |
|------|-----------|---------|
| `docs/work/tasks.md` | high | Carries the explicit INT-0007 maintenance item requesting this appendix refresh. |
| `docs/appendix/potential-derivative-projects.md` | high | Target document; Sprint-6-era current-state and prerequisite language still describes realized INT-0009, INT-0010, and INT-0012 capabilities as future or proposed. |
| `docs/intents/INT-0013-maintain-derivative-ecosystem-current-state.md` | high | Follow-on authority for factual current-state maintenance without rewriting terminal INT-0007. |
| `docs/intents/INT-0007-define-cubikan-derivative-ecosystem.md` | high | Superseded Sprint 6 authority preserved as history for the original catalog outcome and advisory/non-creation scope. |
| `docs/intents/INT-0008-traceable-intent-instantiation.md` | medium | Remains proposed and must not be presented as a realized provenance capability. |
| `docs/intents/INT-0009-revisioned-lifecycle-commands.md` | high | Authoritative realized revision contract for stale-first guarded lifecycle commands. |
| `docs/intents/INT-0010-durable-intent-unit-backend.md` | high | Authoritative realized contract for the embedded SQLite backend and `cubikan-local` lifecycle protocol. |
| `docs/intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md` | medium | Remains proposed and retains unresolved observation, clock, correction, privacy, and metric-policy work. |
| `docs/intents/INT-0012-intent-unit-relationships-and-board-projections.md` | high | Authoritative realized Rust-only relationship, migration, direct-query, and projection boundary. |
| `README.md` | high | Current version matrix distinguishes core, stateless CLI, durable backend, local adapter, schema-v2 relationships, and Rust-only projections. |
| `crates/cubikan-backend/README.md` | high | Precise backend API, schema-v1/v2, migration, relationship, query, and projection contract. |
| `crates/cubikan-local/README.md` | medium | Confirms durable lifecycle protocol v1 remains five-operation only and exposes no relationship/projection commands. |

## 3. External Sources

No external source is needed. This is a repository-internal factual correction
whose authorities are the realized intent chapters and checked-in operator
documentation above.

## 4. Risks, Unknowns, Dependencies

- **Risk:** A broad replacement of “future” could falsely realize INT-0008
  provenance or INT-0011 measurement evidence. Corrections must be
  capability-specific.
- **Risk:** Updating the current boundary could imply that `cubikan-local`
  exposes relationship or projection commands. Those remain a Rust-only
  `cubikan-backend` surface under local protocol v1.
- **Risk:** Rewording catalog prerequisites could sound like authorization to
  create derivative repositories. The advisory banner, creation triggers, and
  explicit non-goals must remain intact.
- **Risk:** Rewriting terminal INT-0007 would violate Book lifecycle rules,
  while leaving it realized would create competing live current-state
  semantics. Preserve its historical outcome and evidence, use the legal
  `realized → superseded` transition, and place maintenance in INT-0013.
- **Unknown:** The only editorial judgment is how much historical Sprint 6
  framing to retain. Historical provenance should stay; present-tense claims
  must describe the current tree.
- **Dependency:** Accurate wording depends on INT-0009, INT-0010, and INT-0012
  remaining realized and on the current README/backend/local version matrices.

## 5. Recommended Approach

Make a focused editorial pass over the appendix. Update the current surface
and authority tables; split realized capabilities (revision,
persistence/query, relationships/projections) from still-proposed capabilities
(provenance and measurement evidence); revise derivative prerequisites and the
sequencing/open-question sections only where they still wait for realized
work; preserve superseded INT-0007 as Sprint 6 history and use INT-0013 as maintenance authority;
and retain every advisory, separation, compatibility, security, and non-creation
boundary. Close the exact maintenance backlog item once, then
validate links, Book state, documentation-only scope, and the unchanged Rust,
manifest, workflow, remote-profile, and intent surfaces.

Alternative considered: begin realization of INT-0008 or INT-0011 instead.
That would ignore the explicit maintenance item and require unresolved product
choices for provenance correction/provider identity or observation identity,
clocks, windows, corrections, privacy, and measurement governance. It is a
materially different sprint, not a substitute for correcting known stale
documentation.

Rationale: this is the smallest honest vertical. It restores the appendix's
usefulness without expanding a runtime API, changing intent ownership, or
authorizing a derivative project.

## Artifacts

No separate research artifacts were created.
