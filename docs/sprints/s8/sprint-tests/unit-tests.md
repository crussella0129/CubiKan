# Sprint 8 Unit, Documentation, and Repository Verification

- **Primary intent:** [INT-0010](../../../intents/INT-0010-durable-intent-unit-backend.md)
- **Preserved dependency:** [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md)
- **Accepted base:** `91d3260d50af8f6c5ec3a852fad50e4e32df3b59`
- **Build task/ledger head:** `581281cb8e4ab38c0f47f4e12f085ea825b92096`
- **Tested critic-response candidate:** `065b71fa1b63ba6abce6effb23c9d20674171835`
- **Local stable toolchain:** `rustc 1.95.0`; `cargo 1.95.0`
- **Conclusion:** pass; every locked unit, documentation, repository, Book,
  link, and canonical local quality check recorded here passed at the exact
  tested candidate tree

The Test response after the Build ledger head has two commits. `2e5e2b9` adds
the three finalized cross-component tests in
`crates/cubikan-backend/tests/persistence.rs` and
`crates/cubikan-local/tests/protocol.rs`. Subsequent critique identified the
need for a direct exhaustive `BackendError` mapper oracle; `065b71f` responds
by adding only a `#[cfg(test)]` module to
`crates/cubikan-local/src/execution.rs`. Neither response changes non-test
runtime behavior, manifests, dependencies, documentation, workflow
configuration, or Book state. Their real-SQLite results are recorded in
[integration-tests.md](integration-tests.md); actual-process and hosted results
are recorded in [e2e-tests.md](e2e-tests.md).

This artifact covers the plan's unit-level T-801, T-802, and T-807 contracts,
the T-810 documentation checks, exact repository scope, local quality gates,
and preserved-intent regressions. Private envelope DTOs remain private, so their
codec checks correctly run as backend library unit tests. Public command and
protocol checks use only exported crate seams.

## T-801 backend value boundary

| EARS | Named executed evidence | Arrangement | Exact SHALL observation | Result |
|------|-------------------------|-------------|-------------------------|--------|
| T-801-E1 | `test_workspace_adds_isolated_backend_crate`; pinned task-boundary manifest inspection | Execute the public backend model target at the tested candidate, inspect Cargo metadata, and separately read the semantic T-801 Git object `1934c5b` rather than projecting the final dependency set backward in time. | The T-801 object contains a separate Rust 2024 `cubikan-backend` workspace member whose direct dependencies are exactly `cubikan-core`, Serde, and Serde JSON. The final candidate also contains `rusqlite`, added only by T-803, while `cubikan-core` and `cubikan-cli` remain free of backend/SQLite dependencies. | pass |
| T-801-E2 | `test_public_backend_model_exposes_complete_commands_and_results` | Construct create, get, list, transition, and complete commands through public exports; inspect a full view, summary, page, mutation result, and typed missing-ID error. | The public boundary preserves typed IDs, species, owned workflow, phase, status, history, expected/committed revisions, filters, limits, and cursors for all five operations. No stored-envelope or SQL-row DTO is exported. | pass |
| T-801-E3 | `test_public_backend_model_preserves_typed_u64_revisions`; `test_command_models_preserve_typed_u64_revisions` | Table-test revisions `0`, `i64::MAX + 1`, and `u64::MAX` through transition/completion commands, full views, summaries, pages, conflicts, and mutation results. | Every accessor and equality comparison returns the exact `IntentUnitRevision`; no value is narrowed to a signed integer or converted to boundary text inside the public Rust model. | pass |
| T-801-E4 | `test_query_limit_and_cursor_validation` | Construct limits at 0, 1, 100, and 101; parse ordinary and nil canonical UUIDs plus uppercase, compact, whitespace-padded, and malformed text before any store exists. | Limits 1–100 and canonical lowercase hyphenated ordinary/nil IDs are accepted. Out-of-range limits and every malformed/noncanonical cursor are rejected with typed construction errors before storage access. | pass |

The historical T-801 dependency assertion is deliberately not attributed to the
final candidate's compiled test alone. At `1934c5b`, the crate had only the
three locked task-boundary dependencies. At the tested candidate, final
adapter-only SQLite isolation is independently proved by T-803-E4 and the
accepted-base scope check below.

## T-802 strict envelope and revision codecs

| EARS | Named executed evidence | Arrangement | Exact SHALL observation | Result |
|------|-------------------------|-------------|-------------------------|--------|
| T-802-E1 | `test_envelope_v1_round_trips_active_and_completed_units` | Encode and decode active and completed units using a custom workflow with forward, reverse, and self edges and ordered histories; compare the decoded aggregate and compare envelope JSON with direct core Serde. | Decode reconstructs the vocabulary/workflow, replays all records through core behavior, and returns the exact aggregate. `representation_version` and decimal-string revision are present, and the envelope is observably different from direct `IntentUnit` Serde. | pass |
| T-802-E2 | `test_envelope_v1_rejects_malformed_or_unreplayable_lifecycle` | Independently corrupt zero/gapped/duplicate sequence, transition source, undeclared edge, completion eligibility/phase, post-completion record, final phase/status, and revision. | Every malformed or semantically unreplayable case returns typed `CorruptEnvelope` and yields no aggregate; no declaration is normalized into a valid successor. | pass |
| T-802-E3 | `test_envelope_v1_rejects_unknown_missing_invalid_and_unsupported_state` | Remove every required field at each nesting level; add unknown and duplicate keys; corrupt IDs, vocabulary, workflow topology, tags, types, and versions 0/2. | Missing, unknown, duplicate, wrong-typed, invalid-vocabulary, and invalid-topology state is corruption; non-1 representation versions are `UnsupportedEnvelopeVersion` with the exact found version. Nothing is silently dropped, defaulted, or normalized. | pass |
| T-802-E4 | `test_revision_codecs_preserve_full_u64_and_reject_aliases` | Cross both codecs with `0`, `i64::MAX + 1`, and `u64::MAX`; table-test empty/signed/padded/leading-zero/fractional/overflow/Unicode text and blobs of length 0, 7, and 9. | JSON uses exact canonical decimal strings and SQL uses exact eight-byte big-endian values across the full `u64` range. Invalid text is corruption and non-eight-byte projections are projection mismatch. | pass |

These four tests run inside `cubikan-backend` because the adapter-owned stored
DTO and codec are intentionally crate-private. Fixtures use real core
constructors and production JSON codecs; there is no mocked aggregate or
implementation-mirroring decoder.

## T-807 local protocol and executor

| EARS | Named executed evidence | Arrangement | Exact SHALL observation | Result |
|------|-------------------------|-------------|-------------------------|--------|
| T-807-E1 | `test_protocol_v1_decodes_all_locked_operations_strictly`; `test_protocol_v1_rejects_semantically_invalid_values_before_storage` | Table-drive exact create/get/list/transition/complete requests; then vary top-level/operation/nested unknown, missing, wrong-typed, and explicit-null members, versions, canonical IDs, vocabulary, topology, filters, limits, cursors, and revisions while a sentinel database path is absent. | All five exact shapes construct validated backend/core values. Syntax, structure, version, and every semantic field class map to the locked rejection, and the sentinel path remains absent for every invalid request, proving validation precedes storage open. | pass |
| T-807-E2 | `test_protocol_v1_uses_decimal_strings_for_every_revision`; `test_response_revision_codec_preserves_full_u64_as_text`; semantic-invalid table | Exercise expected revisions, full units, summaries, mutation committed/unit revisions, and conflict expected/actual values, including `u64::MAX`; separately submit noncanonical strings and number/null/Boolean values. | Every valid revision crosses JSON as canonical decimal text, never a number. Noncanonical or overflowing strings are `invalid_revision` with `field`; a non-string is structural `invalid_request`. | pass |
| T-807-E3 | `test_protocol_v1_serializes_exact_unit_page_and_mutation_results` | Execute create, list, and transition against real SQLite and compare parsed responses as complete semantic JSON objects. | Unit, page (including always-present `next_cursor`), and mutation responses contain exactly the locked adapter-owned fields, full workflow/history, decimal revisions, and result tags. No core or stored DTO layout leaks into the protocol. | pass |
| T-807-E4 | `execution::tests::test_backend_errors_map_exhaustively_to_protocol_codes`; backend-library and external-protocol tests both named `test_protocol_v1_maps_exact_error_code_taxonomy`; semantic-invalid table | The critic-response oracle constructs all 17 expanded backend cases: every `BackendError`, every transition/completion inner variant, a genuine stale conflict from a real backend, and cloned opaque storage payloads. An exhaustive helper assigns a unique ordinal to every match arm; the earlier tests construct all 28 protocol codes and observe all three public classes. | All 17 backend cases are visited exactly once and map to the exact code, response class, and original display message. `field` is absent for backend errors; expected `"9"`/actual `"0"` appear only for the genuine conflict. Across all 28 codes, `field` appears only for semantic validation and conflict members only for revision conflict. | pass |
| T-807-E5 | `test_executor_preserves_backend_atomicity_on_modeled_failure`; `test_corruption_never_reaches_mutation_or_protocol_success`; `test_revision_conflict_propagates_core_to_local_protocol` | Seed real SQLite, snapshot through a fresh backend, and execute duplicate, missing, current-domain, stale, unavailable-storage, and corrupt-row requests through `execute_request`. | Every modeled rejection has `outcome:"failure"`, never a result/false success. The durable unit remains exactly equal across ordinary rejections; corrupt bytes remain unchanged; stale expected `0`/actual `1` survives core→backend→protocol without overwrite. | pass |

The two protocol tests added by `2e5e2b9` are the corruption and
revision-conflict cross-component checks. Its third added check,
`test_backend_codec_schema_crud_query_and_mutation_compose`, lives in the
backend integration target and is detailed in the integration artifact.
`065b71f` adds only the exhaustive mapper oracle above; this is an honest
critic-response delta, not evidence that the initial critique was clean.

## T-810 documentation contract and nonclaims

### `test_backend_docs_define_versions_storage_pagination_concurrency_and_delivery` — T-810-E1

- **Arrangement:** Inspect the root overview,
  [`cubikan-backend` reference](../../../../crates/cubikan-backend/README.md), and
  [`cubikan-local` reference](../../../../crates/cubikan-local/README.md) together;
  compare their statements with the finalized three version-1 contracts and
  committed schema, query, transaction, protocol, and runner source.
- **SHALL assertion and observation:** Consumers shall receive one present-tense
  account of stored envelope v1, SQLite schema v1, and local JSON protocol v1;
  an explicit literal path; replay plus projection checks; exact schema,
  PRAGMAs, rollback journal, and 5,000-ms lock wait; `BEGIN IMMEDIATE`, guarded
  command, revision-qualified update, one-row requirement, and commit ordering;
  busy-before-stale and stale-before-domain precedence; workflow-ID-only exact
  filters; exclusive lexical keyset and live-page semantics; local-filesystem
  assumptions; exact process exits; fail-closed operator recovery; and
  refresh-after-post-commit-delivery-uncertainty guidance. Every item is stated,
  including open/read busy behavior, lookahead-row validation, and the absence
  of filtered-out corruption detection.
- **Result:** pass.

### `test_backend_docs_state_all_locked_nonclaims` — T-810-E2

- **Arrangement:** Check the same three documents for every locked exclusion and
  reject stronger wording implying network safety, snapshot pages,
  acknowledged delivery, retry safety, cryptographic proof, crash immunity, or
  indefinite compatibility.
- **SHALL assertion and observation:** The docs explicitly exclude network
  filesystems/services, authentication/authorization, tenancy, encryption,
  backup, replication, automatic migration/repair, deletion, unrelated shared
  writable storage, direct core Serde persistence, retries, idempotency and
  exactly-once behavior, cross-unit transactions, indefinite stable
  compatibility, cryptographic audit/tamper proof, metrics/KPI evaluation,
  agent/actor/commit provenance, cross-unit relationships, UI, deployment, and
  blockchain/network policy. They also deny crash-kill/power-loss immunity,
  total-request-timeout meaning, rollback after response failure, and external
  delivery acknowledgement.
- **Result:** pass.

The existing stateless `cubikan` section remains explicitly in-memory and
unchanged in meaning. Current durable guarantees are scoped to the new backend
and local adapter. The Sprint 6 derivative appendix is preserved as historical
INT-0007 evidence; the three T-810 documents above are the current operational
contract surface tested here.

## Repository, Book, and navigation checks

### `test_sprint_eight_scope_preserves_core_cli_ci_and_realized_intents`

- **Arrangement:** Compare accepted base
  `91d3260d50af8f6c5ec3a852fad50e4e32df3b59` with tested candidate
  `065b71fa1b63ba6abce6effb23c9d20674171835`, inspect all 42 changed paths,
  verify protected paths with a quiet Git-object diff, and separately compare
  the Build ledger head with the Test-only candidate.
- **SHALL assertion and observation:** The accepted-base delta contains only
  workspace/lock changes, the new backend/local crates, root README, INT-0010,
  Sprint 8 Book evidence, Summary navigation, and task-completion ledger. All
  `crates/cubikan-core/**`, `crates/cubikan-cli/**`, `.github/workflows/**`, and
  INT-0001 through INT-0009 chapters are byte-identical to the accepted base.
  The complete Test-response delta touches only the two named test files and
  the `#[cfg(test)]` execution module; `065b71f` alone touches only that module.
- **Result:** pass.

The 42 changed paths group as two workspace metadata files, one root README, 16
backend paths, 11 local-adapter paths, and 12 Book/ledger paths. No derivative
repository, service, UI, deployment, or blockchain implementation was created.

### `test_book_v2_and_markdown_navigation_are_valid`

- **Arrangement:** Run the installed Book v2 validator at the exact candidate;
  independently run the read-only path/fragment resolver against Markdown
  blobs pinned to that Git object so later uncommitted Test prose cannot alter
  the oracle.
- **SHALL assertion and observation:** The Book validator reports exactly
  `check-book: valid v2 Book (12 intent chapters)`, with INT-0010 active and
  legally linked to Sprint 8 work evidence. The pinned-object resolver inspects
  115 Markdown files, 770 Markdown links, 707 local links, and 8 fragment
  targets with 0 errors.
- **Result:** pass.

The Book validator proves intent schema/state/evidence rules; the separate
resolver proves path and fragment reachability. Neither substitutes for the
other. Links introduced by these Test artifacts are checked separately at the
working-tree handoff and are not retroactively included in the pinned candidate
counts. With these three uncommitted evidence artifacts included, the resolver
reports 115 Markdown files, 794 links, 729 local links, 8 fragments, and 0
errors.

## Canonical local quality gates

The five commands ran in workflow order against the exact tree committed as
`065b71fa1b63ba6abce6effb23c9d20674171835`. All passed:

| Gate | Exact command | Exact result |
|------|---------------|--------------|
| Formatting | `cargo +stable fmt --all -- --check` | pass |
| Clippy | `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | pass; zero warnings |
| Warnings-denied check | `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | pass; zero warnings |
| All-target tests | `cargo +stable test --workspace --all-targets` | pass; 165 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out |
| Doctests | `cargo +stable test --doc --workspace` | pass; 1 core doctest, 0 failed |

Repository checks also passed: the Book validator, pinned Markdown resolver,
accepted-base and candidate-delta scope inspection, documentation semantic
checks, and `git diff --check`. The exact-hosted repetition is recorded in the
E2E artifact rather than treated as a second local oracle.

## Exact all-target suite breakdown

| Crate / target | Passed | Failed | Ignored / measured / filtered |
|----------------|-------:|-------:|-------------------------------:|
| `cubikan-backend` library unit tests | 6 | 0 | 0 |
| `cubikan-backend` corruption integration target | 1 | 0 | 0 |
| `cubikan-backend` model integration target | 4 | 0 | 0 |
| `cubikan-backend` mutations integration target | 7 | 0 | 0 |
| `cubikan-backend` persistence integration target | 4 | 0 | 0 |
| `cubikan-backend` query integration target | 4 | 0 | 0 |
| `cubikan-backend` schema integration target | 4 | 0 | 0 |
| **`cubikan-backend` subtotal** | **30** | **0** | **0** |
| `cubikan-cli` library unit tests | 32 | 0 | 0 |
| `cubikan` binary unit tests | 0 | 0 | 0 |
| `cubikan-cli` actual-process E2E target | 6 | 0 | 0 |
| `cubikan-cli` public-runner integration target | 13 | 0 | 0 |
| **Stateless CLI subtotal** | **51** | **0** | **0** |
| `cubikan-core` library unit tests | 44 | 0 | 0 |
| `cubikan-core` lifecycle integration target | 16 | 0 | 0 |
| `cubikan-core` serialization integration target | 5 | 0 | 0 |
| **Core subtotal** | **65** | **0** | **0** |
| `cubikan-local` library unit tests | 3 | 0 | 0 |
| `cubikan-local` binary unit tests | 0 | 0 | 0 |
| `cubikan-local` actual-process E2E target | 2 | 0 | 0 |
| `cubikan-local` protocol integration target | 8 | 0 | 0 |
| `cubikan-local` runner integration target | 6 | 0 | 0 |
| **Durable local subtotal** | **19** | **0** | **0** |
| **All-target total** | **165** | **0** | **0** |

Workspace doctests add one passing `cubikan-core` doctest. No test failed, was
ignored, measured, or filtered.

## Preserved INT-0001 through INT-0009 evidence

Every prior intent chapter, core source/test path, stateless CLI source/test
path, and CI workflow is byte-identical to the accepted base. The unchanged
regressions executed inside the 165-test result:

| Preserved intent | Executed evidence and observed boundary | Result |
|------------------|------------------------------------------|--------|
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | All 65 core tests preserve opaque UUID identity, caller-defined workflows, owned snapshots, exact directed/rework/self edges, ordered history, completion, atomic errors, and validated restoration. Backend replay composes with rather than bypasses those rules. | pass |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | All 51 stateless CLI tests, including six real child processes, preserve the one-shot configure/create/operate response contract and exact 0/2/3 behavior. `cubikan-local` is a separate binary/protocol. | pass |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | Retained CLI runner/E2E tests pass the exact 1 MiB boundary, one-byte lookahead, oversize-before-JSON precedence, and bounded consumption without turning the old adapter durable. | pass |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | Retained CLI library/runner tests preserve body→newline→single-flush order, error-source/precedence behavior, best-effort diagnostic, and no acknowledgement claim. | pass |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | `.github/workflows/ci.yml` is byte-identical; all five local gates and the exact-head hosted `Rust CI` job pass without changing human merge authority. | pass |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | Retained protocol, runner, and actual-process tests distinguish absent `intent_unit.id` (generated non-nil UUID v4) from explicit null/wrong types (request rejection) in the old CLI. | pass |
| [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md) | The chapter and advisory derivative appendix are byte-identical; the Book/link oracles remain valid and no derivative repository is created. Current backend operations are documented only in the T-810 contract surfaces. | pass; advisory artifact preserved |
| [INT-0008](../../../intents/INT-0008-traceable-intent-instantiation.md) | The proposed chapter is byte-identical and no provenance/blame/commit association, external-provider authority, or traceability policy is silently added to lifecycle storage. | pass; remains separately owned |
| [INT-0009](../../../intents/INT-0009-revisioned-lifecycle-commands.md) | All 65 unchanged core tests preserve initial/exact-one-step revisions and typed stale-first conflicts. T-806 integration, cross-component propagation, and T-809 process E2E additionally consume the exact guarded API without altering it. | pass; realized contract preserved |

This evidence proves regression and scope at the tested revision. It does not
claim cross-platform support, an MSRV, cryptographic audit, crash/power-loss
immunity, network-filesystem correctness, external response acknowledgement,
or any unselected derivative policy.
