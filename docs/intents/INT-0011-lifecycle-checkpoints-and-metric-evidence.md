# INT-0011 — Lifecycle checkpoints and metric evidence

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0011
- **State:** proposed
- **Work evidence:** none
- **Completion evidence:** none
- **Code evidence:** none
- **Test evidence:** none
- **Documentation evidence:** none

## Intent

Allow a CubiKan backend to record caller-defined observations against meaningful
workflow phases, transitions, and completions and derive explicitly defined
process measurements such as counts, conversion ratios, attainment, and cycle
time. Metrics remain projections over lifecycle and observation evidence; they
do not redefine structural workflow validity or automatically authorize a move.
Process applications author and govern business definitions and authorization
policy; the backend stores raw observations and deterministically evaluates only
caller-supplied, versioned definitions; analytics consumers interpret results.

No default KPI names, formulas, thresholds, clocks, units, retention rules,
agent scores, or domain process will be built into `cubikan-core`.

## Acceptance criteria

- A versioned measurement definition identifies the workflow checkpoint or
  lifecycle event it observes and states its value type, unit, denominator,
  aggregation, window, and event-time/source semantics where applicable.
- Caller-supplied observations reference an exact unit and lifecycle revision,
  preserve their source, and have explicit duplicate, late-arrival, and
  correction behavior.
- Counts, conversion ratios, attainment, and time-based projections are
  reproducible from the same accepted lifecycle and observation input without
  mutating Intent Units.
- Arbitrary custom workflows can define meaningful measurements without a
  built-in Kanban topology, mortgage process, organizational taxonomy, or KPI
  formula language in the lifecycle core.
- Metric evaluation cannot make an undeclared transition valid; any future
  automatic transition authorization requires a distinct policy intent and
  retains core validation as the final structural gate.
- Documentation addresses high-cardinality data, missing denominators, clock
  trust, corrections, privacy, and the difference between correlation,
  attribution, and causal agent-quality claims.

## Rationale

The design discussion observes that every real process has different meaningful
checkpoints, while traditional Kanban is one valid configuration. Separating
raw lifecycle events, observations, and derived metrics preserves that
generality and mirrors established telemetry practice where logs, traces, and
metrics are distinct but correlatable signals.

## Alternatives

Embedding fixed KPIs into phase definitions would make the core domain-specific.
Using phase occupancy alone cannot express physical observations, denominators,
or time semantics. Allowing a metric engine to bypass declared workflow edges
would duplicate and weaken the realized lifecycle invariant.

## Consequences

This intent has hard dependencies on
[INT-0009](INT-0009-revisioned-lifecycle-commands.md) for exact lifecycle
revision references and [INT-0010](INT-0010-durable-intent-unit-backend.md) for
durable observations and reproducible projections. Time-based metrics require a
trusted observation clock even though the current domain history intentionally
has none. Agent evaluation introduces privacy, fairness, gaming, and causality
risks and belongs in a derivative analytics project with explicit human
governance.

## Transition history

- 2026-08-08: created as `proposed` after Sprint 6 research separated caller-defined process measurement from lifecycle transition policy.
- 2026-08-08: revised while `proposed` to assign business-definition governance to applications, deterministic caller-defined evaluation to the backend, and analysis to consumers, with hard INT-0009/INT-0010 dependencies.
