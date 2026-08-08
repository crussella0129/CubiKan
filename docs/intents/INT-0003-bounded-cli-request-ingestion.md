# INT-0003 — Bounded CLI request ingestion

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0003
- **State:** active
- **Work evidence:** [Sprint 2 build plan](../sprints/s2/sprint-plans/build-plan.md)
- **Completion evidence:** none
- **Code evidence:** none
- **Test evidence:** none
- **Documentation evidence:** none

## Intent

Make the existing local `cubikan` process's raw request ingestion bounded by
enforcing a 1 MiB (`1_048_576` raw-byte) maximum for its single standard-input
request before JSON decoding. The adapter retains no more than one byte beyond
that ceiling, rejects larger inputs through its existing version 1
typed-response model, and otherwise preserves the realized one-shot lifecycle
behavior.

The ceiling counts the complete raw request, including whitespace. This is a
reversible source-level safety guard for the experimental local adapter, not a
runtime setting or a claim that the process's total memory use is bounded or
that it is ready for network or multi-user exposure.

## Acceptance criteria

- One documented public implementation constant defines the 1 MiB raw request
  ceiling, and research records that changing the source-level value requires
  new evidence rather than runtime configuration or domain policy.
- The runner retains at most the ceiling plus one raw request byte before
  decoding; inputs at or below the ceiling reach the existing strict protocol
  decoder unchanged, while any larger input is rejected before JSON
  classification.
- An oversized request emits exactly one newline-terminated version 1 error
  response with code `request_too_large`, no Intent Unit snapshot, and process
  exit status `2`.
- Genuine input and output failures remain operational failures with exit status
  `1`; existing success, setup-rejection, lifecycle-rejection, and response
  semantics remain unchanged.
- Automated tests cover a valid request below the ceiling, valid JSON padded to
  exactly the ceiling, a one-byte-over-limit input, malformed oversized input,
  and deterministic read/write failures. An actual-process E2E proves the
  oversized response and exit status.
- Consumer documentation explains raw-byte counting, the exact ceiling and
  error code, and that persistence, sessions, networking, deployment,
  authorization, concurrency, UI, blockchain, and network-specific controls
  remain outside this outcome.

## Rationale

INT-0002 explicitly left standard input unbounded and requires resource
limiting before any production network exposure. This is the only unresolved
follow-on requirement already accepted by the Book that does not select a
persistence, service, desktop, or blockchain model. One MiB is intentionally
far above the checked-in lifecycle scenario while providing a simple bound on
retained raw input; it can change in a later intent if real workloads supply
different evidence.

## Alternatives

Leaving input unbounded preserves the smallest implementation but retains a
known allocation risk. A configurable limit adds arguments or environment
policy without an evidenced need. Streaming directly into the JSON decoder
cannot deterministically distinguish oversize input before classification.
A durable SQLite CLI would add more product capability, but it first requires
explicit storage-format, transaction, concurrency, and migration decisions.
HTTP, Electron, and blockchain adapters introduce still broader unresolved
platform policy.

## Consequences

Otherwise valid scenarios larger than 1 MiB are rejected, and raw padding
counts toward the limit. The runner retains a byte-vector payload of at most
the ceiling plus one before decoding instead of deserializing directly from the
reader; allocator capacity and later decoding allocations are not part of this
bound. Expanding the
version 1 error-code set is acceptable for the explicitly experimental adapter
and does not change the request shape. Because parsing moves after raw I/O, the
experimental Rust API's `RunError::Read` payload becomes `std::io::Error`
instead of `serde_json::Error`; modeled JSON failures remain typed responses.
Timeouts, rate limits, concurrent-client quotas, persistence, and stable
protocol compatibility remain separate future outcomes.

## Transition history

- 2026-08-08: created as `proposed` from INT-0002's explicit unbounded-input hardening deferral.
- 2026-08-08: moved to `planned` when Sprint 2 decomposed bounded ingestion into T-201–T-204 with exact-limit and actual-process verification.
- 2026-08-08: moved to `active` when Build began T-201 for the public request-limit contract.
