# INT-0006 — Distinguish omitted CLI ID from explicit null

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0006
- **State:** realized
- **Work evidence:** [Sprint 5 build plan](../sprints/s5/sprint-plans/build-plan.md)
- **Completion evidence:** [T-501–T-504 completion ledger](../work/completed-tasks.md#t-501-sprint-5)
- **Code evidence:** [strict ID-presence decoder](../../crates/cubikan-cli/src/protocol.rs), [public-runner boundary tests](../../crates/cubikan-cli/tests/runner.rs), and [actual-process boundary tests](../../crates/cubikan-cli/tests/cli_e2e.rs)
- **Test evidence:** [Sprint 5 test report](../sprints/s5/sprint-tests/test-report.md)
- **Documentation evidence:** [CubiKan CLI guide](../../crates/cubikan-cli/README.md)

## Intent

Make omission the only version 1 JSON representation that asks the `cubikan`
CLI to generate an Intent Unit ID. When `intent_unit.id` is present, it must be
a JSON string and then pass the existing `cubikan-core` UUID parser. Explicit
JSON `null` and every other non-string value are structural `invalid_request`
failures and must never silently create a different identity.

This repairs the experimental adapter's accepted strict-type boundary without
changing the core's ID policy. It does not add a protocol version, response
field, error code, dependency, persistence model, session, or compatibility
promise.

## Acceptance criteria

- The version 1 decoder distinguishes an absent `intent_unit.id` member from a
  present JSON string: absence remains the request for generation, while the
  string value is preserved for core UUID validation.
- Explicit `null`, Boolean, number, array, and object values for
  `intent_unit.id` are rejected as structural `invalid_request` failures; they
  never reach Intent Unit construction and produce no state snapshot.
- An omitted ID still produces a non-nil UUID v4, a valid supplied UUID retains
  its value, and a malformed UUID string retains the existing
  `invalid_intent_unit_id` classification with field `intent_unit.id`.
- The public runner reports explicit `null` as one newline-terminated,
  writer-flush-checked version 1 error response with no `intent_unit`, `field`,
  or `operation_number`, while preserving the existing request-rejection
  status.
- The actual `cubikan` process reports explicit `null` with exit status `2`,
  empty stderr, and exactly one complete JSON error line; wire-level omission
  remains a successful generated-ID path.
- Consumer documentation states that the ID member may be absent, but when
  present must be a JSON string and cannot be `null`; request-size,
  output-flush, lifecycle, and process-exit semantics otherwise remain
  unchanged.
- The implementation adds no dependency and changes no `cubikan-core` API or
  UUID acceptance/generation policy, protocol version, response shape, error
  code, request ceiling, persistence, networking, authorization, UI, or
  blockchain behavior.

## Rationale

The accepted CLI guide says fields with the wrong JSON type are rejected and
that ID generation occurs when the member is omitted. The current derived
`Option<String>` decoder maps both a missing member and explicit `null` to
`None`; execution then generates an ID for either representation. A caller can
therefore send a present invalid identity value and receive success for a new,
unrequested identity. Keeping presence distinct from omission restores the
documented boundary and is fully testable without choosing new product policy.

## Alternatives

Treating `null` as an alias for omission was rejected because it preserves the
observed bug but contradicts the accepted strict-type and “when omitted”
contract. Requiring an ID string on every request would remove the already
realized generated-ID path. A general nullable-field framework or JSON Schema
is unnecessary for this single evidenced mismatch. Tightening the provisional
core serialization format, enforcing a UUID version on caller-supplied IDs, or
adding persistence would each select separate compatibility or product policy.

## Consequences

Clients that currently send explicit `null` must omit the member to request a
generated ID. Structural type failures keep the existing human-oriented Serde
message, so no exact diagnostic prose becomes protocol surface. A present
string continues into core validation, preserving the useful distinction
between JSON shape errors and semantically malformed UUID text. The protocol
remains experimental and unchanged in version and response shape.

## Transition history

- 2026-08-08: created as `proposed` after Sprint 5 research reproduced that explicit JSON `null` silently generated an ID despite the accepted omission-only and strict-type contract.
- 2026-08-08: moved to `planned` when Sprint 5 decomposed the decoder, public runner, actual-process, and documentation boundaries into T-501–T-504 with named unit, integration, E2E, and hosted regression evidence.
- 2026-08-08: moved to `active` immediately before Build began T-501 under the finalized Sprint 5 plans.
- 2026-08-08: moved to `realized` after T-501–T-504 completed, the final Test Critic returned `clean`, 100 local and hosted all-target tests plus one doctest passed, and GitHub Actions push run 31285064082 succeeded at exact Build head `6979ca2217ac1b838c406bf21821e32b3a4f6227`.
