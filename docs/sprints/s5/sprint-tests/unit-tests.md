# Sprint 5 Unit and Repository Verification

- **Primary intent:** [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md)
- **Preserved intents:** [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md), and [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md)
- **Accepted base:** `eadf6f53615267e0948205cc78a3db1b9d4ab950`
- **Tested Build head:** `6979ca2217ac1b838c406bf21821e32b3a4f6227`
- **Initial worktree:** clean; `HEAD` and `origin/dev` both identified the tested Build head
- **Local stable toolchain:** `rustc 1.95.0`; `cargo 1.95.0`
- **Local conclusion:** pass; hosted run [31285064082](https://github.com/crussella0129/CubiKan/actions/runs/31285064082) is referenced here only as the external exact-head checkpoint, with job and step evidence recorded separately in [integration](integration-tests.md) and [E2E](e2e-tests.md) results

## Locked unit and repository EARS checks

| Named check | EARS / acceptance boundary | WHEN arrangement | SHALL assertions and observed result | Result |
|-------------|----------------------------|------------------|--------------------------------------|--------|
| `test_protocol_distinguishes_absent_string_and_null_id` | T-501-E1–E2; INT-0006 decoder/type criteria | Starting from one otherwise valid version 1 `serde_json::Value`, the test removed `intent_unit.id`, supplied exact string `"  Preserve-ME  "`, and then table-tested `null`, Boolean, number, array, and object values. | True omission decoded to `None`; the string decoded unchanged to `Some("  Preserve-ME  ")`; every present non-string value failed with Serde category `Data` rather than becoming omission. | pass |
| `test_protocol_preserves_required_and_unknown_field_strictness` | T-501-E3; INT-0006 and preserved INT-0002 strict-request criteria | The test separately omitted only `intent_unit.id`, removed required `intent_unit.species`, and injected unknown members at root, workflow, edge, Intent Unit, transition, and completion DTO boundaries. | The sole optional ID member decoded successfully; missing species and all six unknown-member arrangements remained rejected. | pass |
| `test_protocol_decodes_complete_v1_scenario_strictly` | T-501-E1/E3 regression | The accepted fixed-ID request included caller-defined text, declared edges, a transition, and completion. | The decoder preserved all values and the exact fixed ID while retaining tagged-operation decoding. | pass |
| `test_process_shell_fixture_uses_true_omission` | T-501-E4; INT-0006 omission and preserved INT-0002 process fixture | `VALID_REQUEST` was decoded through the production `ProtocolRequest` DTO after its ID member was removed. | The fixture decoded successfully with `intent_unit.id == None`, proving true member omission rather than JSON `null`. | pass |
| `test_process_shell_maps_operational_failure_to_exit_1` | T-501-E4; preserved INT-0003/INT-0004 operational behavior | The process shell received a failing reader and, separately, the corrected valid fixture with a trailing-newline-failing writer. | Both remained exit `1`; the read case produced no stdout and both best-effort diagnostics remained newline-terminated with the established failure class. | pass |
| `test_process_shell_maps_flush_failure_to_exit_1` | T-501-E4; preserved INT-0004 flush behavior | A writer accepted the modeled success body and newline, then failed its sole explicit flush. | The shell retained exit `1`, one flush attempt, complete newline-terminated success bytes, and the exact flush-failure stderr diagnostic. | pass |
| `test_process_shell_keeps_exit_1_when_flush_diagnostic_fails` | T-501-E4; preserved INT-0004 diagnostic precedence | The response flush failed and the best-effort diagnostic writer also rejected output. | Exit remained `1`, the response writer attempted exactly one flush after the newline, and diagnostic delivery was attempted without replacing the operational outcome. | pass |
| `test_cli_guide_documents_id_presence_contract` | T-504-E1–E2; INT-0006 documentation criterion | The version 1 request, typed-failure, exit, and boundary sections of `crates/cubikan-cli/README.md` were inspected at the Build head. | The guide identifies `intent_unit.id` as the sole optional member; defines absence as non-nil UUID v4 generation; requires every present value to be a JSON string; classifies `null`, Boolean, number, array, and object as `invalid_request`; retains malformed-string `invalid_intent_unit_id` with field `intent_unit.id`; and says human message text is not stable protocol. | pass |
| `test_sprint_scope_has_no_dependency_core_or_output_contract_drift` | T-504-E3; INT-0006 scope criterion; preserved INT-0001–INT-0005 | The accepted-base-to-Build-head path and content diffs, workspace metadata, full normal-edge dependency tree, local gates, and established regression suites were inspected. | No manifest, lockfile, workflow, `cubikan-core`, CLI manifest, execution, runner, main, or checked-in fixture path changed. The production delta is limited to presence-sensitive ID decoding; `lib.rs` changes are under `#[cfg(test)]`; protocol version, response DTOs, error codes, request ceiling, output precedence, UUID policy, and product exclusions are unchanged. | pass |
| `test_book_v2_validation` | T-504-E3; INT-0006 durable-evidence criterion | The installed validator ran at the Build head; a repository-wide relative-target scan covered every Markdown file after the three Test evidence artifacts were populated. | The validator reported exactly `check-book: valid v2 Book (6 intent chapters)`; INT-0006 and Sprint 5 are reachable from `SUMMARY.md`; all 395 local links across 75 Markdown files resolve, with 0 missing targets. | pass |

No domain, service, clock, network, or process mock was used for these unit
checks. The process-shell writers are deterministic `Read`/`Write` test doubles
for established I/O seams; they do not reproduce protocol, UUID, or lifecycle
logic.

## Five canonical local gates

The following commands ran in the locked order while the worktree was clean at
the tested Build head:

| Gate | Command | Result |
|------|---------|--------|
| Formatting | `cargo +stable fmt --all -- --check` | pass |
| Clippy | `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | pass; zero warnings |
| Warnings-denied check | `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | pass; zero warnings |
| All-target tests | `cargo +stable test --workspace --all-targets` | pass; 100 passed, 0 failed |
| Doctests | `cargo +stable test --doc --workspace` | pass; 1 core doctest passed, 0 failed; CLI has 0 doctests |

## Exact suite breakdown

| Suite | Passed | Failed | Ignored / measured / filtered |
|-------|-------:|-------:|-------------------------------:|
| `cubikan-cli` library unit tests | 32 | 0 | 0 |
| `cubikan-cli` actual-process E2E tests | 6 | 0 | 0 |
| `cubikan-cli` public-runner integration tests | 13 | 0 | 0 |
| `cubikan-core` library unit tests | 43 | 0 | 0 |
| `cubikan-core` lifecycle integration tests | 4 | 0 | 0 |
| `cubikan-core` serialization integration tests | 2 | 0 | 0 |
| **All-target total** | **100** | **0** | **0** |

The `cubikan` binary target contains 0 unit tests. Workspace doctests were 1
core pass and 0 CLI tests. Detailed public-runner arrangements and actual
process observations belong to [integration-tests.md](integration-tests.md)
and [e2e-tests.md](e2e-tests.md); their six and thirteen tests are retained in
the authoritative all-target total here.

## Metadata, dependency, and scope confirmation

`cargo metadata --no-deps --format-version 1` resolved exactly the two
Rust 2024 workspace packages, `cubikan-core` and `cubikan-cli`. The CLI's direct
normal dependencies remain exactly `cubikan-core`, `serde`, and `serde_json`,
with no build dependency or direct UUID dependency; generated-ID verification
uses the existing `cubikan-core::IntentUnitId` API. The full
`cargo tree --workspace --edges normal` output remains the accepted graph, and
neither `Cargo.toml` nor `Cargo.lock` changed.

The scoped quiet diff passed for:

```sh
git diff --quiet eadf6f53615267e0948205cc78a3db1b9d4ab950...6979ca2217ac1b838c406bf21821e32b3a4f6227 -- \
  Cargo.toml Cargo.lock .github crates/cubikan-core \
  crates/cubikan-cli/Cargo.toml crates/cubikan-cli/src/execution.rs \
  crates/cubikan-cli/src/runner.rs crates/cubikan-cli/src/main.rs \
  crates/cubikan-cli/tests/fixtures
```

Review of the two intended source-file diffs confirmed that
`crates/cubikan-cli/src/protocol.rs` changes only the ID field decoder and its
tests, while `crates/cubikan-cli/src/lib.rs` changes only the `#[cfg(test)]`
omission fixture and assertion. The workflow, protocol response definitions,
error-code enum, request constant, shell status mapping, core API, and UUID
implementation are byte-for-byte inherited from the accepted base.

## Preserved-intent regression assertions

| Intent | Executed evidence | Result |
|--------|-------------------|--------|
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | 43 core unit tests, 4 lifecycle tests, 2 serialization tests, and the core doctest retained generated non-nil UUID v4, caller-supplied UUID parsing, arbitrary validated topology, atomic lifecycle behavior, immutable identity, ordered history, terminal completion, and invariant-preserving restore. No core path changed. | pass |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | The 32 CLI unit, 13 public-runner, and 6 process tests retained a strict versioned one-shot adapter, core-delegated setup/lifecycle behavior, fixed/generated identities, one typed response, and established exit meanings. The stateless/persistence/network/UI exclusions remain documented. | pass |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | `test_run_accepts_valid_json_at_exact_limit`, `test_run_rejects_oversize_before_json_classification`, `test_run_consumes_at_most_limit_plus_one`, `test_run_preserves_boundary_io_precedence`, both public limit tests, the explicit-null oversize case, and the oversized-process E2E retained the exact 1 MiB ceiling, ceiling-plus-one retention, classification precedence, and exit behavior. | pass |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | `test_run_flushes_each_modeled_response_once_after_newline`, staged output-precedence/error-source tests, the public `BufWriter` drain failure, the new non-string recording-writer table, and all process-shell regressions retained body → newline → one flush ordering and operational exit `1` on output failure. | pass |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | The unchanged workflow's exact five current-stable local commands passed at the clean Build SHA. Successful hosted run 31285064082 is the authoritative external checkpoint; its event, checkout, job, and step facts are reserved for the integration/E2E artifacts. | pass |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | T-501's decoder and process-fixture checks above passed; all T-502 public-runner and T-503 actual-process tests also contribute to the 100-test total; T-504's documentation, scope, and Book checks passed. | pass at the unit/repository boundary |

## Build and task provenance

| Task | Implementation commit recorded by completion ledger | Ledger/evidence follow-up |
|------|----------------------------------------------------|---------------------------|
| T-501 | `1a689edf525b02e05f44eb5027d6ff42d698fb0d` | `530dbddda8bf92477bc9763447d7653645736cba`; zero-byte test-artifact provenance clarified by `71c0162bd44409309a18a2465ce91aa52b584868` |
| T-502 | `e3cd97727752c05cf2c02702ff25bb8da3dbae9a` | `707c60ee7c37b6c1a644180fc6303786a2d5b706` |
| T-503 | `3eadc3ac44d73c4aa6b67582dbbea6b6f33b629d` | `5c053467ae4ee1e0a5455a4bad3221fccfe48f4b` |
| T-504 | `4ce4ff88a8dfb05135ae2b088e900a5e49201a88` | `6979ca2217ac1b838c406bf21821e32b3a4f6227` |

All listed commits descend from the accepted base, and the final T-504 ledger
follow-up is the exact tested Build head. The hosted checkpoint does not replace
the local semantic oracle, and this unit artifact does not duplicate or infer
its job-level evidence.

## Reproduction commands

```sh
git rev-parse HEAD
git status --porcelain=v1
git merge-base --is-ancestor eadf6f53615267e0948205cc78a3db1b9d4ab950 6979ca2217ac1b838c406bf21821e32b3a4f6227
git diff --check eadf6f53615267e0948205cc78a3db1b9d4ab950...6979ca2217ac1b838c406bf21821e32b3a4f6227
cargo metadata --no-deps --format-version 1
cargo tree --workspace --edges normal
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --all-features -- -D warnings
RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets
cargo +stable test --workspace --all-targets
cargo +stable test --doc --workspace
bash /mnt/c/Users/charl/.codex/plugins/cache/sprint-loops/sprint-loop/local/skills/sprint-loop/scripts/check-book.sh
```

Every command passed at the clean tested Build head. The subsequent change to
this file is Test evidence only and is not represented as Build-head runtime
behavior.
