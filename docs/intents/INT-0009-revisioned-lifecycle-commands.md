# INT-0009 — Revisioned lifecycle commands and atomic conflict rejection

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0009
- **State:** active
- **Work evidence:** [Sprint 7 build plan](../sprints/s7/sprint-plans/build-plan.md)
- **Completion evidence:** none
- **Code evidence:** none
- **Test evidence:** none
- **Documentation evidence:** none

## Intent

Give each Intent Unit an explicit lifecycle revision and allow callers to
condition a transition or completion command on the revision they observed.
Every accepted mutation advances the revision exactly once; a stale expectation
is rejected atomically. This supplies a deterministic concurrency primitive for
future durable and multi-client adapters without selecting locks or a database.

This intent does not define leases, retries, cancellation, distributed
transactions, actor ownership, clocks, idempotency retention, or conflict
resolution beyond reject-and-refresh.

## Acceptance criteria

- A newly created Intent Unit begins at one documented revision and exposes it
  through the public API without deriving it from wall-clock time.
- Each successful declared transition and eligible completion advances the
  revision exactly once alongside its existing single lifecycle record.
- A command carrying the current expected revision preserves all realized
  lifecycle behavior, while a mismatched expectation returns a typed conflict
  and leaves phase, status, history, identity, species, workflow, and revision
  unchanged.
- Revision comparison occurs before lifecycle command evaluation: a stale
  expected revision returns the typed conflict even when the requested command
  would also be invalid, while a current expected revision preserves the
  existing typed terminal, unknown-target, undeclared-edge, and
  completion-ineligible rejections.
- Atomic tests cover both stale-plus-domain-invalid and
  current-plus-domain-invalid combinations and prove that neither mutates the
  aggregate.
- Validated restoration rejects disagreement among the stored revision,
  lifecycle history, phase, and status; semantic round trips preserve the exact
  revision.
- Documentation limits the primitive to optimistic conflict detection and does
  not claim database isolation, cross-unit atomicity, locking, or delivery
  idempotency.

## Rationale

A durable backend, manager/doer coordination, and multiple applications all need
a way to prevent stale writers from silently overwriting newer lifecycle state.
Revision checks are a small domain-level contract that remains useful across
storage and transport choices.

## Alternatives

Database-specific locks would couple the domain to a storage engine. Timestamps
would add clock and ordering semantics that are not required for conflict
detection. Inferring revision only from vector length can work internally but
does not provide a clear public command contract or room for validated
evolution.

## Consequences

Future adapters must propagate revisions and surface conflicts. Command
idempotency may still need a separate identifier and retention policy; a
revision match alone does not make repeated network delivery safe.

## Transition history

- 2026-08-08: created as `proposed` after Sprint 6 research identified optimistic revision checks as the smallest backend-neutral prerequisite for durable multi-client CubiKan use.
- 2026-08-08: revised while `proposed` to define stale-revision precedence before lifecycle command evaluation and require both combined negative paths.
- 2026-08-09: moved to `planned` when Sprint 7 selected the additive core revision contract and mapped every acceptance criterion to T-701–T-704 and named verification.
- 2026-08-09: moved to `active` immediately before T-701 began the explicit lifecycle-revision implementation.
