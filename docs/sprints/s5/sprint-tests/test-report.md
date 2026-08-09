# Sprint 5 Test Report

## Intent Verification

| Intent | Acceptance criterion | EARS / tests | Result | Intent evidence update |
|--------|----------------------|---------------|--------|------------------------|
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | Absence requests generation while a present string is preserved for core UUID validation. | T-501-E1 / `test_protocol_distinguishes_absent_string_and_null_id`; T-502-E1, T-502-E3 / `test_run_generates_id_when_member_is_omitted`, `test_run_preserves_id_string_validation_taxonomy` | pass | This report records decoder and public-boundary proof; eligible for `realized` after Loop completion evidence is linked. |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | Present `null`, Boolean, number, array, and object values are structural `invalid_request` failures without state. | T-501-E2, T-502-E2 / `test_protocol_distinguishes_absent_string_and_null_id`, `test_run_rejects_present_non_string_ids_without_creating_state` | pass | Decoder and public-runner tables cover every prohibited JSON type and prove no snapshot is emitted. |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | Omission generates a non-nil UUID v4; valid and malformed supplied strings preserve their existing semantics. | T-502-E1, T-502-E3, T-503-E1 / `test_run_generates_id_when_member_is_omitted`, `test_run_preserves_id_string_validation_taxonomy`, `test_cli_generates_id_when_member_is_omitted`, plus existing execution regressions | pass | Generated IDs are property-checked rather than value-pinned; fixed and malformed strings retain exact taxonomy. |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | Public explicit-null rejection is one newline-terminated, writer-flush-checked v1 error with request-rejection status and no optional state fields. | T-502-E2 / `test_run_rejects_present_non_string_ids_without_creating_state`; preserved flush and precedence regressions | pass | Recording-writer evidence proves one newline, exactly one final flush, `invalid_request`, and absent `intent_unit`, `field`, and `operation_number`. |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | The actual process distinguishes omission success from explicit-null exit `2`. | T-503-E1–E2 / `test_cli_generates_id_when_member_is_omitted`, `test_cli_reports_explicit_null_id_with_exit_2` | pass | Cargo-built process evidence proves exact exit/stdout/stderr behavior at the consumer boundary. |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | Consumer documentation defines omission, present-string, null, and structural-versus-semantic failure rules without changing neighboring contracts. | T-504-E1–E2 / `test_cli_guide_documents_id_presence_contract` | pass | The CLI guide is linked as documentation evidence during Loop close. |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | No dependency, core API, UUID policy, protocol/version/response/error, request-ceiling, output, or unrelated product-policy drift occurs. | T-501-E3–E4, T-502-E4, T-503-E3, T-504-E3 / strictness, oversize-precedence, process, scope, dependency-tree, Book, and hosted regressions | pass | Accepted-base-to-Build-head scope is bounded; the exact committed Build head passed local and hosted gates. |
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | Chain-agnostic workflow, lifecycle atomicity, terminal behavior, and validated serialization remain realized. | T-504-E3 / 43 core unit, 4 lifecycle integration, 2 serialization integration tests, and 1 doctest | pass | This report is appended as regression Test evidence; no core source changed. |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | The versioned runnable lifecycle adapter and typed process outcomes remain realized. | T-501-E4, T-502-E1–E3, T-503-E1–E3 / CLI unit, public-runner, and six actual-process tests | pass | This report is appended as regression Test evidence. |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | The 1 MiB ceiling, size-before-decode precedence, bounded read, and exit behavior remain realized. | T-502-E4, T-503-E3, T-504-E3 / exact-limit, one-over, bounded-read, oversize-null, and process oversize regressions | pass | This report is appended as regression Test evidence. |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Exactly-once supplied-writer flush and first-output-error precedence remain realized. | T-501-E4, T-502-E2, T-504-E3 / five-response flush, precedence, real `BufWriter`, and process-shell regressions | pass | This report is appended as regression Test evidence. |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | The existing hosted current-stable Rust quality workflow remains operational at the tested Build head. | T-504-E3 / `test_hosted_sprint_five_quality_run_succeeds` | pass | This report is appended as regression Test evidence; the workflow itself is unchanged. |

## Summary

- Unit tests: 75 passed / 0 failed / 75 total (32 CLI and 43 core).
- Integration tests: 19 passed / 0 failed / 19 total (13 public runner, 4 core lifecycle, and 2 core serialization).
- E2E tests: 6 passed / 0 failed / 6 total (Cargo-built `cubikan` process).
- Workspace regression: 100 passed / 0 failed / 100 total; 0 ignored, measured, or filtered out.
- Doctests: 1 passed / 0 failed / 1 total.
- Repository/Book confirmations: metadata, full normal-edge dependency tree, scoped drift review, Markdown links, Book v2 validation, and diff checks passed.
- CI status: green.

## CI Confirmation

- **Head SHA:** `6979ca2217ac1b838c406bf21821e32b3a4f6227`
- **CI run:** [31285064082 — Rust CI](https://github.com/crussella0129/CubiKan/actions/runs/31285064082)
- **Job:** [93172288024 — Rust quality gate](https://github.com/crussella0129/CubiKan/actions/runs/31285064082/job/93172288024)
- **Conclusion:** `success` on attempt 1 for event `push`, branch `dev`, at the exact committed Build head.
- **Confirmations:** the sole job passed formatting, Clippy with warnings denied, warnings-denied compilation, all-target tests, and workspace doctests. The remote ref, checkout log, run SHA, job SHA, and local committed Build SHA all matched. Detailed local, hosted-integration, and process evidence is recorded in [unit results](unit-tests.md), [integration results](integration-tests.md), and [E2E results](e2e-tests.md).

## Failures

None. The final read-only Test Critic found no unresolved concerns and returned `clean`.

## Technical Debt Identified

- No new follow-up intent was opened by Sprint 5. Lockfile-strict CI remains a separate automation-policy candidate rather than part of this protocol repair.
- Tightening the provisional core restore format and adding persistence, a service, Electron UI, or blockchain behavior remain separate product/compatibility decisions that require explicit intent and, where noted in research, human policy choices.

## Coverage Observations

- Decoder and public-runner tables cover all five prohibited present JSON types; actual-process coverage intentionally uses explicit `null` as the representative consumer defect and separately proves true omission.
- Human-readable Serde diagnostics remain deliberately unpinned; tests assert stable response taxonomy and shape instead.
- Generated UUIDs are checked for parseability, non-nil value, and version 4 without asserting a random concrete value.
- Local stable Rust 1.95.0 and hosted stable Rust 1.97.1 produced the same 100-test and one-doctest results. Hosted Ubuntu and Rust versions are run provenance, not fixed support or MSRV promises.
- The later draft `dev` to `main` pull-request run is a remote handoff confirmation, not the realization oracle recorded here; successful CI status does not authorize merge or configure branch protection.
