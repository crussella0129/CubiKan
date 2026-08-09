# Sprint 5 Research Report

## Intents Reviewed

- [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) — created; relevance: owns the selected repair that makes omission the only request for generated CLI identity and rejects present non-string values; current state: `proposed`.
- [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) — selected; relevance: owns the realized strict version 1 adapter boundary whose missing-versus-wrong-type behavior must be restored; current state: `realized`.
- [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) — selected; relevance: oversized-input precedence and request-rejection classification must remain unchanged; current state: `realized`.
- [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) — selected; relevance: explicit-null rejection must use the realized newline and supplied-writer flush contract; current state: `realized`.
- [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) — selected; relevance: core UUID parsing, generation, and lifecycle semantics remain authoritative and unchanged; current state: `realized`.
- [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) — selected; relevance: its realized local and hosted quality gates will verify the repair without changing CI policy; current state: `realized`.

## 1. Sprint Goal

Repair the experimental version 1 CLI identity boundary so that only omission
of `intent_unit.id` requests a generated UUID. A present JSON string will keep
flowing through the existing core UUID parser, while explicit `null` and every
other non-string JSON type will be rejected structurally as `invalid_request`
without constructing state. Sprint 5 will prove the distinction at decoder,
public-runner, and actual-process boundaries and clarify it in consumer
documentation. It will not change core ID semantics, dependencies, protocol
version or response shape, request limits, flush behavior, persistence,
networking, authorization, UI, or blockchain policy.

## 2. Existing Code Survey

| File | Relevance | Finding |
|------|-----------|---------|
| `crates/cubikan-cli/src/protocol.rs` | high | `IntentUnitInput.id` is a derived `Option<String>`; Serde therefore maps both an absent member and explicit `null` to `None`, despite `deny_unknown_fields` elsewhere enforcing a strict request shape. |
| `crates/cubikan-cli/src/execution.rs` | high | Setup generates `IntentUnitId::generate()` for every decoded `None`, so the decoder ambiguity becomes silent identity substitution rather than a request failure. |
| `crates/cubikan-cli/src/lib.rs` | high | The process-shell success fixture currently supplies `"id": null`, which normalizes the unintended behavior and masks it in output-error tests. |
| `crates/cubikan-cli/src/runner.rs` | high | JSON data/shape failures already map to version 1 `invalid_request`, one newline, exactly one successful supplied-writer flush, and `RunStatus::RequestRejected`; the repair can reuse this path. |
| `crates/cubikan-cli/tests/runner.rs` | high | Public integration coverage proves generated and fixed IDs through constructed inputs but has no wire-level missing-versus-null assertion. |
| `crates/cubikan-cli/tests/cli_e2e.rs` | high | Actual-process tests cover success, malformed JSON, lifecycle rejection, and oversize rejection, but not explicit-null identity rejection or wire-level omitted-ID generation. |
| `crates/cubikan-cli/README.md` | high | The accepted guide says wrong JSON types are `invalid_request` and generation occurs “When omitted”; it does not authorize `null` as an omission alias. |
| `crates/cubikan-core/src/id.rs` | medium | Core accepts parseable supplied UUID text and generates non-nil UUID v4 values; neither policy needs to change. |
| `docs/intents/INT-0002-runnable-lifecycle-adapter.md` | medium | The realized adapter accepts an optional fixed ID through a strict versioned request, while compatibility beyond the experimental intent is not promised. |
| `docs/intents/INT-0003-bounded-cli-request-ingestion.md` | medium | Oversized input must still be rejected before JSON classification and real I/O failures must retain operational exit `1`. |
| `docs/intents/INT-0004-explicit-cli-response-flush.md` | medium | Every modeled response must retain body → newline → exactly-one-flush ordering and deterministic output-error precedence. |
| `.github/workflows/ci.yml` | medium | The realized Rust gate supplies hosted regression coverage; this sprint requires no workflow or CI-policy change. |
| `docs/sprints/s4/sprint-tests/test-report.md` | medium | Records the accepted baseline of 91 all-target tests, one doctest, and a successful hosted quality run before this behavior repair. |

The accepted executable was also exercised directly with otherwise equivalent
requests. An absent ID succeeded and generated a non-nil UUID v4; explicit
`null` also incorrectly succeeded and generated a UUID v4; integer `42` was
correctly rejected as `invalid_request` with process exit `2`. This isolates the
gap to Serde's missing/null treatment for `Option<String>` rather than the
existing non-string or execution error taxonomy.

## 3. External Sources

None. The accepted repository documentation, implementation, and directly
reproduced executable behavior fully establish the semantic mismatch; Sprint 5
does not depend on an unstable external fact or a new library contract.

## 4. Risks, Unknowns, Dependencies

- **Risk:** A custom field decoder can reject `null` but accidentally make a
  missing member required. Decoder tests must separately prove absent, string,
  `null`, Boolean, number, array, and object representations.
- **Risk:** Exact Serde diagnostic prose is intended for humans and can vary.
  Tests should lock the stable `invalid_request` code and response shape while
  requiring only a nonempty message.
- **Risk:** A malformed UUID string is structurally a string and must continue
  to produce `invalid_intent_unit_id` with field `intent_unit.id`; it must not be
  collapsed into the structural failure class.
- **Risk:** The current process-shell fixture uses `null` for success. It must
  omit the member so output-error and flush tests keep proving their intended
  paths rather than retaining an invalid fixture.
- **Risk:** Generated IDs are nondeterministic. Tests should assert parseable,
  non-nil UUID v4 semantics rather than an exact value.
- **Unknown:** Existing external consumers may send `null`, but the adapter is
  explicitly experimental and the accepted guide already classifies present
  wrong-type values as invalid. Those consumers must omit the field instead.
- **Dependency:** The repair uses the existing Serde field-deserialization seam
  and current core constructors. It requires no new crate or platform service.

## 5. Recommended Approach

Give `IntentUnitInput.id` a field-specific deserializer with a missing-field
default. When the member is absent, the default remains `None`; when it is
present, deserialize a `String` and wrap it in `Some`, causing `null` and all
other non-string values to take the runner's existing `invalid_request` path.
Keep execution's `Some` UUID parsing and `None` UUID generation unchanged.

Add table-driven protocol coverage for absent, string, `null`, Boolean, number,
array, and object values. Add public-runner tests proving omitted-ID success and
explicit-null rejection without state, field, or operation number. Add
actual-process coverage for the same two paths, including exit `2`, empty
stderr, one complete response line, and a generated non-nil UUID v4 on
omission. Preserve malformed-string classification, oversize-before-decode
precedence, exact response flushing, normal lifecycle behavior, and all
existing exits. Replace the internal success fixture's `null` member with true
omission and document the explicit absent-versus-present rule.

Alternatives considered: treating `null` as omission contradicts the accepted
contract; making ID mandatory removes a realized feature; strict unknown-field
restoration for provisional core Serde selects a broader compatibility policy;
lockfile-strict CI is a useful but lower-priority engineering increment; and
persistence, service, Electron, or blockchain boundaries require unresolved
product and platform decisions.

## Artifacts

- [INT-0006 — Distinguish omitted CLI ID from explicit null](../../../intents/INT-0006-distinguish-omitted-cli-id.md)
- This report records the direct executable reproduction; no separate binary or
  data artifact was saved.
