# INT-0012 — Intent Unit relationships and board projections

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0012
- **State:** proposed
- **Work evidence:** none
- **Completion evidence:** none
- **Code evidence:** none
- **Test evidence:** none
- **Documentation evidence:** none

## Intent

Let a CubiKan backend preserve caller-defined, typed relationships among Intent
Units and project units into multiple board or portfolio views without changing
the lifecycle workflow owned by any unit. Cross-unit dependency, derivation, or
grouping edges remain distinct from `WorkflowEdge`, which continues to connect
phases inside one immutable workflow snapshot.

This intent supplies reusable relationship and query primitives only. It does
not define delegation, scheduling, fan-out/join execution, skill loading, WIP
limits, cascade behavior, board layout, notifications, or user interface policy.

## Acceptance criteria

- A caller can create and query a validated typed relationship between existing
  Intent Units while preserving each unit’s independent identity, workflow,
  phase, status, and history.
- Relationship definitions state direction, endpoint constraints, duplicate,
  self-edge, cycle, correction, and deletion behavior rather than inheriting the
  rules of lifecycle phase edges.
- Rejected unknown-endpoint or policy-invalid relationships are atomic and do
  not mutate either Intent Unit or accepted relationship state.
- A unit can appear in multiple caller-defined board or portfolio projections
  based on explicit query criteria without copying or transferring ownership of
  its lifecycle state.
- Projection results are reproducible from canonical unit and relationship
  state and identify the query or definition version that produced them.
- Documentation keeps multi-board views separate from cross-unit execution
  graphs; a skill-graph or manager application owns readiness, scheduling,
  retries, artifact routing, and executor policy.

## Rationale

Multi-board pipelines and organizational views need relationships beyond the
single-unit phase graph. A generic backend relation/query layer can support those
consumers without teaching the lifecycle core how to run an agent graph or draw
a board.

## Alternatives

Treating relationships as workflow edges was rejected because phase topology
has a different identity and validation scope. Adding parent/child lineage to
the core would prematurely select one relation taxonomy. Copying units between
boards would create competing lifecycle authorities.

## Consequences

This intent has a hard dependency on
[INT-0010](INT-0010-durable-intent-unit-backend.md) for canonical unit
collections and durable relationship state. Relation types and graph constraints
are product policy that must be explicit before implementation. Large
projections need pagination, consistency, and authorization decisions in a
future backend; none is implied by this proposed domain outcome.

## Transition history

- 2026-08-08: created as `proposed` after Sprint 6 research separated multi-board and pipeline relationships from the existing single-unit workflow graph.
- 2026-08-08: revised while `proposed` to make the durable multi-unit backend in INT-0010 an explicit prerequisite.
