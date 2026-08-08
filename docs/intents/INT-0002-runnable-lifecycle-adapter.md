# INT-0002 — Runnable lifecycle adapter and E2E boundary

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0002
- **State:** realized
- **Work evidence:** [Sprint 1 build plan](../sprints/s1/sprint-plans/build-plan.md)
- **Completion evidence:** [T-101–T-107 completion ledger](../work/completed-tasks.md#t-101-sprint-1)
- **Code evidence:** [`cubikan-cli` runner](../../crates/cubikan-cli/src/lib.rs) and [process shell](../../crates/cubikan-cli/src/main.rs)
- **Test evidence:** [Sprint 1 test report](../sprints/s1/sprint-tests/test-report.md)
- **Documentation evidence:** [CubiKan CLI guide](../../crates/cubikan-cli/README.md) and [Sprint 1 research](../sprints/s1/sprint-research/research-report.md)

## Intent

Expose the `cubikan-core` lifecycle through a one-shot batch JSON CLI. One
process accepts a caller-defined workflow and ordered lifecycle actions, creates
and mutates an Intent Unit in memory, and emits the externally observable result
through a versioned adapter-owned response.

The CLI is an execution boundary, not a persistence or session abstraction. It
must not add domain policy that belongs in the core or imply a stable format for
the core crate's provisional serialization.

## Acceptance criteria

- Research records why a one-shot batch JSON CLI is the smallest useful,
  reversible runnable boundary for the current product evidence.
- The CLI accepts one versioned scenario on standard input with a caller-defined
  workflow, species, optional fixed Intent Unit ID, and ordered lifecycle
  actions; it emits one versioned success or typed failure response.
- An automated process-level E2E test drives configure → create →
  transition → complete and asserts the externally visible JSON result and
  exit status.
- Adapter behavior delegates lifecycle invariants to `cubikan-core` rather than
  duplicating or weakening them.
- Persistence, networking, deployment, authorization, KPI, naming, blockchain,
  and UI policy remain explicitly outside the implementation boundary.

## Rationale

Sprint 0 proved the library boundary but could not supply a true system E2E
test. A stateless batch CLI closes that evidence gap while keeping unresolved
platform and persistence choices reversible.

## Alternatives

A service/API could be reused by a future Electron client, but it adds an async
runtime, networking, routing, deployment, and service-state decisions. An
Electron boundary adds packaging, IPC, and UI decisions. A blockchain adapter
requires unresolved chain, trust, storage, key, and finality choices. Separate
CLI commands were also rejected because the repository has no persistence and
cross-process mutation would imply a state model that does not exist.

## Consequences

The request and response use adapter-owned DTOs and an explicit protocol version;
compatibility beyond this intent is not promised. A failed lifecycle action is
reported without mutating that action's state, while earlier successful actions
in the same scenario remain visible. Follow-on persistence, UI, service, or
deployment work requires separate intents instead of silently expanding this
one. The local adapter does not limit standard-input size; resource limiting is
explicit future hardening that must precede any production network exposure.

## Transition history

- 2026-08-08: reconstructed during Book v2 migration as `proposed` from Sprint 0's recorded E2E gap.
- 2026-08-08: moved to `planned` when the legacy loop created T-101 as the next-sprint candidate.
- 2026-08-08: moved to `deferred` pending acceptance of the Sprint 0 `dev → main` checkpoint and an explicit next-sprint start.
- 2026-08-08: revised while `deferred` after Sprint 1 research selected a one-shot batch JSON CLI and rejected boundaries that require persistence or platform policy.
- 2026-08-08: moved to `planned` when Sprint 1 decomposed the selected CLI boundary into T-101–T-107 with process-level E2E coverage.
- 2026-08-08: revised while `planned` to make unbounded local input an explicit hardening deferral and lock the adapter-owned version 1 typed-error contract in the Sprint 1 plan.
- 2026-08-08: moved to `active` when Build began T-101 for the planned CLI workspace boundary.
- 2026-08-08: moved to `realized` after T-101–T-107 completion, actual-process E2E verification, all committed-head quality gates, and a clean final Test Critic.
