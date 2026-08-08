# INT-0001 — Chain-agnostic Intent Unit lifecycle core

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0001
- **State:** realized
- **Work evidence:** [Sprint 0 build plan](../sprints/s0/sprint-plans/build-plan.md)
- **Completion evidence:** [T-001–T-012 completion ledger](../work/completed-tasks.md#t-001-sprint-0)
- **Code evidence:** [`cubikan-core` public API](../../crates/cubikan-core/src/lib.rs)
- **Test evidence:** [Sprint 0 test report](../sprints/s0/sprint-tests/test-report.md)
- **Documentation evidence:** [CubiKan README](../../README.md)

## Intent

Provide a chain-agnostic Rust domain core in which callers can define arbitrary
directed workflows and move uniquely identified Intent Units through validated
transitions to terminal completion. `IntentUnitId` is an opaque UUID v4 value
without an ordering contract, and every Intent Unit owns the immutable validated
workflow snapshot used to create it. The core preserves immutable species and
workflow identity, records ordered in-memory lifecycle history, and rejects
invalid restored state.

This intent deliberately excludes blockchain/network selection, persistence,
service or desktop adapters, authorization, concurrency, KPI evaluation,
default workflows, parent/child lineage, completed-unit naming grammar, and a
stable wire-format commitment.

## Acceptance criteria

- Durable project evidence records the chain-agnostic core boundary and keeps
  unresolved infrastructure and product-policy choices explicitly deferred.
- A Rust 2024 virtual workspace exposes one warning-free `cubikan-core` library.
- Opaque identifiers and textual vocabulary validate their invariants while
  preserving caller-supplied semantic values; generated Intent Unit identifiers
  are non-nil UUID v4 values with no ordering guarantee.
- A caller-declared workflow accepts only known phases, exact directed edges,
  and explicit completion-eligible phases, including declared reverse or self
  edges without an implicit default topology.
- An Intent Unit starts active at its workflow's initial phase, performs only
  declared transitions atomically, owns its immutable validated workflow
  snapshot, preserves immutable identity/species/workflow values, records
  ordered lifecycle events, and becomes terminal only through eligible
  completion.
- Serialization round trips valid scalars, workflows, and active/completed
  aggregates while rejecting malformed or internally inconsistent state.
- Consumer documentation and an executable example explain the model,
  development commands, and explicit Sprint 0 boundaries.

## Rationale

The repository initially defined only the minimal Intent Unit concept. A small,
pure domain library made those stable invariants executable without coupling
them to unresolved platform or product-policy choices.

## Alternatives

An Ethereum contract or Electron application could have demonstrated a visible
vertical sooner, but either would have selected network, persistence, cost,
deployment, and UX semantics that the repository did not define. UUID v7,
default Kanban phases, mutable workflow registries, and unchecked Serde derives
were also rejected because they introduced unrequired contracts or bypassed
domain validation.

## Consequences

Later adapters can reuse one deterministic lifecycle model. Infrastructure,
product-policy, and durable-audit semantics require follow-on intents. The
serialized representation remains provisional even though restoration is
invariant-preserving.

## Transition history

- 2026-08-08: reconstructed during Book v2 migration as originally `proposed` by Sprint 0 research.
- 2026-08-08: moved to `planned` when the Sprint 0 build and test plans were locked.
- 2026-08-08: moved to `active` when T-001 implementation began.
- 2026-08-08: moved to `realized` after T-001–T-012 completion, all quality gates, and a clean follow-up test critique.
