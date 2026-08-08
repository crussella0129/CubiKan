# INT-0004 — Explicit CLI response flush before modeled outcome

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0004
- **State:** realized
- **Work evidence:** [Sprint 3 build plan](../sprints/s3/sprint-plans/build-plan.md)
- **Completion evidence:** [T-301–T-303 completion ledger](../work/completed-tasks.md#t-301-sprint-3)
- **Code evidence:** [writer-flush-checked runner](../../crates/cubikan-cli/src/runner.rs) and [process-shell mapping](../../crates/cubikan-cli/src/lib.rs)
- **Test evidence:** [Sprint 3 test report](../sprints/s3/sprint-tests/test-report.md)
- **Documentation evidence:** [CubiKan overview](../../README.md) and [CLI guide](../../crates/cubikan-cli/README.md)

## Intent

Make the existing generic `cubikan-cli` runner return a modeled success or
rejection only after `serde_json::to_writer`, `write_all(b"\n")`, and one call
to the supplied writer's `flush` each return `Ok`. A flush failure is an
operational output failure, preserves its `std::io::Error`, and maps through
the existing process shell to exit status `1` rather than a false protocol
outcome.

This checks only the supplied `Write` implementation's flush contract. It does
not prove OS or kernel delivery, close success, external-reader receipt,
transactional output, durable storage synchronization, or a new protocol/domain
guarantee. Partial bytes cannot be rolled back when a stream reports a late
write or flush error.

## Acceptance criteria

- The centralized response path returns the existing modeled `RunStatus` only
  after JSON serialization, terminating-newline writing, and exactly one call
  to the supplied writer's `flush` each return `Ok`, in that order.
- The public experimental `RunError` distinguishes a flush failure as
  `FlushResponse(std::io::Error)`, preserves the underlying error kind, message,
  and source, and gives it a deterministic human-readable diagnostic.
- Body serialization, newline writing, and flushing have deterministic
  first-error precedence: an earlier failure returns its existing operational
  variant without attempting a later step, while a writer that accepts body and
  newline but fails only on flush cannot produce a modeled outcome.
- Success, malformed-JSON request rejection, setup rejection, lifecycle
  rejection, and oversized-request paths each call the supplied writer's flush
  exactly once after the newline; a realistic buffered-writer test proves a
  failure that drop-time flushing would otherwise hide.
- `run_process` maps a response flush failure to exit status `1`, emits the
  existing best-effort stderr diagnostic, and all existing actual-process exit,
  JSON, and stream behavior remains unchanged when flushing succeeds.
- Consumer documentation explains the writer-flush-checked boundary and
  explicitly avoids claims of stream atomicity, rollback, durable `fsync`,
  persistence, OS delivery, external-reader/network acknowledgement, or stable
  cross-version compatibility.

## Rationale

The realized adapter contract already treats incomplete response output as an
operational failure, and Sprint 2 preserved that outcome for body and newline
errors. The centralized writer currently returns immediately after
`write_all(b"\n")` and never calls `Write::flush`. A buffering writer can
therefore accept every byte, fail while later flushing to its true sink, and be
dropped after the runner has already reported success. Rust documents explicit
flush as the operation that exposes this completion failure, making the gap
locally reproducible without selecting new product policy.

## Alternatives

Relying on writer drop was rejected because `BufWriter` ignores errors from its
drop-time flush. Flushing only in `main` would leave the public generic runner's
result false for other composed writers. Serializing the entire response into a
second byte vector before writing would add allocation without making stream
delivery atomic. File synchronization, retries, acknowledgements, and output
transactions require destination-specific policy and remain outside this
adapter.

## Consequences

Every modeled response now incurs one explicit flush, including unbuffered
writers. Adding a public `RunError` variant is an intentional adjustment to the
experimental Rust API; the JSON protocol shape and exit-code meanings do not
change. A late error can still leave a partial response in the sink, so callers
must treat exit `1` as “complete response not guaranteed,” not as rollback.

## Transition history

- 2026-08-08: created as `proposed` after Sprint 3 research reproduced a missing supplied-writer flush check in the realized generic writer boundary.
- 2026-08-08: moved to `planned` when Sprint 3 decomposed the correction into runner, public/process verification, and documentation tasks with exact output-stage evidence.
- 2026-08-08: moved to `active` when Build began T-301 for the typed supplied-writer flush contract.
- 2026-08-08: moved to `realized` after T-301–T-303 completion, all 91 workspace tests and the core doctest passed at the committed Test head, and the final Test Critic verdict was clean.
