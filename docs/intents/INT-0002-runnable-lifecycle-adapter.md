# INT-0002 — Runnable lifecycle adapter and E2E boundary

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0002
- **State:** deferred
- **Work evidence:** [T-101 backlog](../work/tasks.md)
- **Completion evidence:** none
- **Code evidence:** none
- **Test evidence:** none
- **Documentation evidence:** [Sprint 0 E2E deferral](../sprints/s0/sprint-tests/e2e-tests.md)

## Intent

Expose the `cubikan-core` lifecycle through one deliberately selected runnable
boundary so an external input can configure a workflow, create and transition
an Intent Unit, complete it, and observe the resulting state end to end.

Adapter selection must be researched before implementation. This intent does
not itself choose a CLI, service/API, Electron application, blockchain network,
or persistence mechanism, and it must not add domain policy that belongs in the
core or in a separate product intent.

## Acceptance criteria

- Research selects one runnable adapter boundary and records why it is the
  smallest useful, reversible choice for the current product evidence.
- The adapter accepts externally observable caller-defined workflow and
  lifecycle input and exposes success and typed failure outcomes.
- An automated E2E test drives configure → create → transition → complete
  through the runnable boundary and asserts the externally visible result.
- Adapter behavior delegates lifecycle invariants to `cubikan-core` rather than
  duplicating or weakening them.
- Unselected platform, persistence, authorization, KPI, naming, and UI policy
  remain explicitly outside the implementation boundary.

## Rationale

Sprint 0 proved the library boundary but could not supply a true system E2E
test. A runnable adapter is the next evidence gap, while its exact shape remains
a reversible research decision.

## Alternatives

A CLI is likely the smallest local boundary; a service/API may be more directly
reusable by a future Electron client; an Electron or blockchain adapter would
surface product value sooner but commits more unresolved policy. Research must
compare these options rather than treating convention as a requirement.

## Consequences

The intent remains visible without expanding Sprint 0's implementation scope.
Work resumes only after the current human-approved sprint checkpoint and a new
sprint plan.
The selected adapter may create follow-on intents for persistence, UI, or
deployment instead of silently expanding this one.

## Transition history

- 2026-08-08: reconstructed during Book v2 migration as `proposed` from Sprint 0's recorded E2E gap.
- 2026-08-08: moved to `planned` when the legacy loop created T-101 as the next-sprint candidate.
- 2026-08-08: moved to `deferred` pending acceptance of the Sprint 0 `dev → main` checkpoint and an explicit next-sprint start.
