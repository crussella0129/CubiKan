# Sprint 6 Unit and Repository Verification

- **Primary intent:** [INT-0007](../../../intents/INT-0007-define-cubikan-derivative-ecosystem.md)
- **Preserved intents:** [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md), [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md), [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md), [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md), [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md), and [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md)
- **Accepted base:** `bf5b5f299102d4853fa5312b1091ec0b8fb2dfe1`
- **Tested Build/evidence head:** `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7`
- **Local stable toolchain:** `rustc 1.95.0`; `cargo 1.95.0`
- **Conclusion:** pass; every locked unit/repository check below passed at the exact Build head

The checks were one-off source, repository, and command inspections. They added
no product test harness, dependency, fixture, mock, or stub. The hosted
exact-head checkpoint is recorded in the integration and E2E artifacts; this
file does not substitute local inspection for that external oracle.

## Locked EARS checks

### T-601 appendix foundation

#### `test_appendix_authority_and_current_boundary` — T-601-E1–E2

- **Arrangement:** Inspect the exact page heading, advisory block, current
  boundary, and architectural-layer sections of the appendix at
  `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7`; compare the present-tense claims
  with realized INT-0001 and INT-0002.
- **SHALL assertion and observation:** The page is titled exactly “Potential
  Derivative Projects,” calls the catalog non-binding, identifies Project Book
  intents as semantic authority, and denies that a listed repository or future
  backend exists or is authorized. It describes `cubikan-core` only as a
  chain-agnostic, one-aggregate lifecycle kernel and `cubikan` only as an
  experimental one-shot, in-memory adapter. It explicitly denies current
  persistence, resumable service, actor/authorization, metric, cross-unit
  graph, UI, deployment, and blockchain behavior.
- **Result:** pass.

#### `test_derivative_dependency_direction_is_safe` — T-601-E3

- **Arrangement:** Inspect the layer table, graph vocabulary, safe-integration
  rules, authority table, and all six entries' CubiKan interaction fields.
- **SHALL assertion and observation:** CubiKan owns Intent Unit identity and
  lifecycle validation; derivatives own domain, UI, orchestration, analytics,
  security, and business policy. The only permitted paths are embedding the
  current public core at an explicitly pinned crate version or consuming a
  future explicitly versioned command/query/evidence boundary. The text denies
  a cross-version Rust API promise, shared/direct database access, provisional
  Serde as a disk/wire contract, and the one-shot CLI as a session. It names
  separate meanings and owners for one-unit `WorkflowEdge` topology,
  provenance, delegation, cross-unit relations, and execution edges.
- **Result:** pass.

#### `test_proposed_backend_intent_map_is_complete_and_unscheduled` — T-601-E4

- **Arrangement:** Resolve the appendix's capability-map links and inspect the
  metadata and acceptance boundaries of INT-0008 through INT-0012.
- **SHALL assertion and observation:** Each proposed chapter is linked exactly
  as a future capability, remains `proposed`, and has `none` for Work and
  Completion evidence. The map records the locked order: INT-0009 precedes and
  is a hard prerequisite of INT-0010; full revision-scoped, bidirectional
  INT-0008 requires INT-0009 and INT-0010; INT-0011 requires INT-0009 and
  INT-0010; and INT-0012 requires INT-0010. INT-0009 checks a stale expected
  revision before command-domain validity and preserves the existing domain
  error for a current revision. The appendix leaves storage/transport/auth,
  concurrency, evidence/privacy, measurement, relationship, and blockchain
  policies as explicit human tripwires rather than implied implementation
  choices.
- **Result:** pass.

#### `test_book_navigation_and_local_links_resolve` — T-601-E5

- **Arrangement:** Count the Sprint 6 links in `docs/SUMMARY.md`, then run the
  separate repository-wide Markdown path-and-fragment resolver against the
  exact Build tree.
- **SHALL assertion and observation:** Summary contains exactly one link to
  `appendix/README.md`, one to
  `appendix/potential-derivative-projects.md`, and one to each INT-0007 through
  INT-0012 chapter. The resolver checked 91 Markdown files, 521 local links,
  and 7 fragment targets; every path and fragment resolved. This resolver—not
  `check-book.sh`—is the navigation and link oracle.
- **Result:** pass.

#### `test_book_backend_authority_avoids_split_brain` — T-601-E6

- **Arrangement:** Inspect every row of the appendix data-authority map and the
  migration paragraph that follows it.
- **SHALL assertion and observation:** Every named datum has one canonical
  owner. The Book remains the current semantic and historical authority;
  projections may reference it but may not dual-write it. Operational task or
  completion truth cannot move to a backend until a separately selected
  projection or migration intent defines reconciliation and cutover.
- **Result:** pass.

#### `test_book_v2_intent_schema_and_state` — T-601-E4

- **Arrangement:** Run the installed Sprint Loop `check-book.sh` from the
  repository root at the exact Build head.
- **SHALL assertion and observation:** The validator returned exactly
  `check-book: valid v2 Book (12 intent chapters)`. This proves Book v2 intent
  metadata, evidence shape, uniqueness, and lifecycle-state requirements only;
  it is not evidence for Summary reachability or local-link resolution.
- **Result:** pass.

### T-602 agent and accounting catalog

#### `test_agent_accounting_entries_have_required_contract_fields` — T-602-E1

- **Arrangement:** Parse the first three numbered catalog entries—
  `cubikan-agent-ops`, `cubikan-observatory`, and `animus-ledger`—by their
  explicit field labels.
- **SHALL assertion and observation:** Each has one unique recommended slug and
  non-placeholder values for problem/outcome, owned data, owned policy, inputs,
  outputs, CubiKan interaction, prerequisites, creation trigger, separation
  rationale, explicit non-goals, and related intents. These are the first three
  of the document-wide six `Recommended repository` fields and 66 other
  required fields.
- **Result:** pass.

#### `test_agent_operations_boundary_is_explicit` — T-602-E2

- **Arrangement:** Inspect the Agent Ops ownership, input/output, interaction,
  prerequisites, and non-goal fields together.
- **SHALL assertion and observation:** Agent Ops represents executable assigned
  work with CubiKan Intent Units but keeps manager/doer identity,
  decomposition, assignment, delegation scheduling/dispatch, tools,
  permissions, sandbox/budget envelopes, retries/cancellation, approvals,
  escalation, and cost policy outside CubiKan. It neither becomes the Book nor
  delegates lifecycle validity to an agent runtime.
- **Result:** pass.

#### `test_observatory_separates_recorded_provenance_from_inference` — T-602-E3

- **Arrangement:** Inspect Observatory's owned records and policies, all input
  identity requirements, its outputs, creation gate, and non-goals.
- **SHALL assertion and observation:** Book intents and Intent Units use
  distinct namespaces; Git objects are repository-qualified full revisions;
  PR, CI, test, document, lifecycle-revision, and deliberately selected trace
  evidence remain revision-pinned. Blame snapshots, attribution hypotheses,
  scores, and recommendations are labeled derived analysis rather than
  causality, authorship proof, fitness, or evidence certification. Before score
  publication or adaptation, the creation gate requires data minimization,
  retention/deletion and redaction, access control, and a named human approval
  gate; automatic agent mutation is a non-goal.
- **Result:** pass.

#### `test_animus_ledger_requires_an_accounting_model` — T-602-E4

- **Arrangement:** Inspect Animus Ledger's data/policy ownership, prerequisites,
  interaction boundary, recursive-loop paragraph, and non-goals.
- **SHALL assertion and observation:** Book parsing, plan-versus-actual
  reconciliation, the unit of account, cost/credit allocation, corrections and
  reversals, trust thresholds, anti-gaming rules, and approvals remain
  ledger-owned. Trustworthy provenance is a prerequisite, and current CubiKan
  lifecycle history is expressly not treated as a journal. Nested/recursive
  composition is mapped to Agent Ops, Skill Graph, and INT-0012, while
  unresolved loop identity and correction semantics remain open; no current
  core parent-child lineage is inferred.
- **Result:** pass.

### T-603 process, graph, organization, and scope

#### `test_process_application_entries_have_required_contract_fields` — T-603-E1

- **Arrangement:** Parse the final three numbered catalog entries—
  `cubikan-process-studio`, `cubikan-skill-graph`, and
  `cubikan-org-app-kit`—by their explicit field labels.
- **SHALL assertion and observation:** Each has one unique recommended slug and
  non-placeholder problem/outcome, owned-data, owned-policy, input, output,
  CubiKan-interaction, prerequisite, creation-trigger, separation-rationale,
  explicit-non-goal, and related-intent fields. Together with T-602, the final
  catalog contains exactly six `Recommended repository` fields plus 66 other
  required fields—11 complete fields per entry—and no placeholder value.
- **Result:** pass.

#### `test_process_studio_keeps_kpi_policy_outside_core` — T-603-E2

- **Arrangement:** Inspect Process Studio's ownership, integration,
  prerequisites, creation trigger, and non-goals, then compare them with
  INT-0010 and INT-0011.
- **SHALL assertion and observation:** Studio authors and governs versioned
  process/measurement definitions and authorization policy, translating only
  selected immutable topology into core validation. A future backend may store
  raw observations and deterministically evaluate only caller-supplied,
  versioned definitions; Observatory may consume results but does not own those
  definitions. Local design may embed a pinned current core, while shared
  operational/KPI use waits for INT-0010 and INT-0011. Electron-first is an
  advisory UI recommendation, not an implemented application.
- **Result:** pass.

#### `test_skill_graph_keeps_execution_edges_outside_workflow` — T-603-E3

- **Arrangement:** Compare Skill Graph's owned graph identities and execution
  policies with the appendix graph vocabulary and core `WorkflowEdge` boundary.
- **SHALL assertion and observation:** Pipeline, unit, and board-dependency
  edges have distinct IDs and semantics from one-unit phase edges. Skill Graph
  owns manifests, readiness, routing, node scheduling, fan-out/join, retries,
  execution, sandbox enforcement, recovery, and artifact routing; none becomes
  lifecycle topology or transfers lifecycle authority.
- **Result:** pass.

#### `test_org_apps_keep_domain_data_and_policy_bounded` — T-603-E4

- **Arrangement:** Inspect the one Organizational App Kit entry and the
  separately authorized future vertical-repository pattern.
- **SHALL assertion and observation:** `cubikan-org-app-kit` is the sole primary
  entry; verticals are an unnamed pattern rather than extra recommendations.
  Basic projections use INT-0010's bounded collection query, while advanced
  multi-board relations wait for INT-0012. Each vertical keeps domain records,
  PII, retention, RBAC, integrations, notifications, reports, deployment, and
  UX inside its separately authorized bounded domain.
- **Result:** pass.

#### `test_appendix_sequence_and_noncommitment_scope` — T-603-E5

- **Arrangement:** Parse all primary headings, the dependency-ordered sequence,
  creation triggers, alternatives, open questions, and final non-goals.
- **SHALL assertion and observation:** The six and only six primary slugs are
  `cubikan-agent-ops`, `cubikan-observatory`, `animus-ledger`,
  `cubikan-process-studio`, `cubikan-skill-graph`, and
  `cubikan-org-app-kit`. Their sequencing follows hard prerequisites and
  evidence, every entry has a creation trigger, and merged/rejected candidates
  and unresolved questions are explicit. No datum or policy has conflicting
  authority. The appendix neither claims nor authorizes an external repository
  operation, package publication, release, deployment, chain/database/transport
  choice, or roadmap commitment.
- **Result:** pass.

#### `test_prose_only_scope_has_no_runtime_or_existing_intent_drift` — T-601-E2–E4, T-603-E5–E6

- **Arrangement:** Compare accepted base
  `bf5b5f299102d4853fa5312b1091ec0b8fb2dfe1` with committed Build head
  `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7`; enumerate every changed path,
  compare INT-0001–INT-0006 and protected paths byte-for-byte, inspect workspace
  metadata and the normal dependency tree, and run `git diff --check`.
- **SHALL assertion and observation:** `git diff --check` passed. The exact
  Build-head set is 19 changed paths, all inside the locked 20-path allowlist;
  the twentieth permitted path, `docs/work/tasks.md`, is byte-identical and
  therefore absent from the changed set. INT-0001–INT-0006 and every manifest,
  lockfile, Rust source, workflow/CI, submodule, workspace-member, dependency,
  package/release, root README, license, history, and remote-profile path are
  unchanged. Metadata still reports exactly two Rust 2024 crates, and the
  complete normal dependency tree is unchanged.
- **Result:** pass.

#### `test_t603_candidate_passes_five_local_rust_gates` — T-603-E6

- **Arrangement:** After the complete T-603 prose candidate and scope check
  existed, but before semantic task commit
  `f38e974a903f9e4a0cac8a63778c0426877571b5` and evidence commit
  `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7`, run the five locked commands in
  order. Repeat the same commands at the exact committed Build head.
- **SHALL assertion and observation:** Both timing-aware candidate and
  exact-head runs passed formatting, Clippy with `-D warnings`, warnings-denied
  all-target checking, all-target tests, and doctests. The committed-head run
  had 100 passing tests and one passing core doctest, with no failure, ignored,
  measured, or filtered case. The repeat confirms the final head but does not
  replace the required pre-commit candidate run.
- **Result:** pass.

#### `test_remote_scope_matches_recorded_baseline_and_operations` — T-603-E5–E6

- **Arrangement:** Compare the pre-Build `git remote -v`, local `remote.*`
  configuration, and submodule baseline with Test state; review the Sprint 6
  remote-operation record separately from the Git content diff.
- **SHALL assertion and observation:** Both baseline and Test state contain one
  existing remote, `origin`, with fetch and push URL
  `https://github.com/crussella0129/CubiKan` and fetch refspec
  `+refs/heads/*:refs/remotes/origin/*`; `git submodule status` is empty. The
  only Sprint 6 remote mutation was the authorized push of the existing CubiKan
  `dev` branch. No create, push, publish, release, or deployment operation
  targeted any derivative slug. This is an operational record with the stated
  Sprint scope, not a claim that a repository diff can observe unrelated
  external activity.
- **Result:** pass.

## Exact changed-path evidence

The accepted-base-to-Build-head changed set was:

```text
docs/SUMMARY.md
docs/appendix/README.md
docs/appendix/potential-derivative-projects.md
docs/intents/INT-0007-define-cubikan-derivative-ecosystem.md
docs/intents/INT-0008-traceable-intent-instantiation.md
docs/intents/INT-0009-revisioned-lifecycle-commands.md
docs/intents/INT-0010-durable-intent-unit-backend.md
docs/intents/INT-0011-lifecycle-checkpoints-and-metric-evidence.md
docs/intents/INT-0012-intent-unit-relationships-and-board-projections.md
docs/sprints/s6/sprint-meta.md
docs/sprints/s6/sprint-plans/build-plan.md
docs/sprints/s6/sprint-plans/critique.md
docs/sprints/s6/sprint-plans/test-plan.md
docs/sprints/s6/sprint-research/research-report.md
docs/sprints/s6/sprint-tests/e2e-tests.md
docs/sprints/s6/sprint-tests/integration-tests.md
docs/sprints/s6/sprint-tests/test-report.md
docs/sprints/s6/sprint-tests/unit-tests.md
docs/work/completed-tasks.md
```

The exact allowlist additionally permits `docs/work/tasks.md`; its final
Build-head bytes equal the accepted base. No path outside those 20 explicit
possibilities changed.

## Local quality gates and suite totals

The five commands ran in locked order once on the complete pre-commit T-603
candidate and again at the exact Build head:

| Gate | Command | Exact-head result |
|------|---------|-------------------|
| Formatting | `cargo +stable fmt --all -- --check` | pass |
| Clippy | `cargo +stable clippy --workspace --all-targets --all-features -- -D warnings` | pass; zero warnings |
| Warnings-denied check | `RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets` | pass; zero warnings |
| All-target tests | `cargo +stable test --workspace --all-targets` | pass; 100 passed, 0 failed |
| Doctests | `cargo +stable test --doc --workspace` | pass; 1 core doctest passed, CLI has 0 doctests |

| Suite | Passed | Failed | Ignored / measured / filtered |
|-------|-------:|-------:|-------------------------------:|
| `cubikan-cli` library unit tests | 32 | 0 | 0 |
| `cubikan-cli` actual-process E2E tests | 6 | 0 | 0 |
| `cubikan-cli` public-runner integration tests | 13 | 0 | 0 |
| `cubikan-core` library unit tests | 43 | 0 | 0 |
| `cubikan-core` lifecycle integration tests | 4 | 0 | 0 |
| `cubikan-core` serialization integration tests | 2 | 0 | 0 |
| **All-target total** | **100** | **0** | **0** |

The `cubikan` binary target contains 0 unit tests. Workspace doctests contain 1
passing core test and 0 CLI tests. `cargo metadata --no-deps --format-version 1`
reported exactly the `cubikan-core` and `cubikan-cli` Rust 2024 packages; the
normal dependency tree is unchanged from the accepted base.

## Preserved-intent named regressions

| Intent | Named retained evidence | Result |
|--------|-------------------------|--------|
| [INT-0001](../../../intents/INT-0001-chain-agnostic-intent-lifecycle-core.md) | `test_generated_intent_unit_id_is_non_nil_v4`, `test_workflow_accepts_explicit_topology`, `test_intent_lifecycle_create_transition_complete`, `test_failed_operations_are_atomic_and_recoverable`, and `test_tampered_serialized_aggregate_is_rejected` preserve opaque identity, arbitrary validated topology, atomic lifecycle semantics, terminal behavior, ordered history, and invariant-preserving restore. | pass |
| [INT-0002](../../../intents/INT-0002-runnable-lifecycle-adapter.md) | `test_cli_configure_create_transition_complete`, `test_cli_reports_malformed_request_with_exit_2`, `test_cli_reports_lifecycle_rejection_with_exit_3`, and `test_runner_preserves_prior_successes_on_lifecycle_failure` preserve the versioned one-shot adapter and core-delegated lifecycle boundary. | pass |
| [INT-0003](../../../intents/INT-0003-bounded-cli-request-ingestion.md) | `test_run_accepts_valid_json_at_exact_limit`, `test_run_rejects_oversize_before_json_classification`, `test_run_consumes_at_most_limit_plus_one`, `test_run_preserves_boundary_io_precedence`, and `test_cli_reports_oversized_request_with_exit_2` preserve the exact 1 MiB ceiling and classification/operational precedence. | pass |
| [INT-0004](../../../intents/INT-0004-explicit-cli-response-flush.md) | `test_run_flushes_each_modeled_response_once_after_newline`, `test_run_preserves_response_output_error_precedence`, `test_runner_surfaces_buffered_sink_failure_on_explicit_flush`, `test_process_shell_maps_flush_failure_to_exit_1`, and `test_process_shell_keeps_exit_1_when_flush_diagnostic_fails` preserve body → newline → one-flush ordering and exit behavior. | pass |
| [INT-0005](../../../intents/INT-0005-automated-rust-quality-gate.md) | The byte-identical `Rust CI` workflow retains the previously named `test_ci_workflow_runs_five_canonical_gates_in_order` structure, and those exact five current-stable commands passed locally at the Build head. Hosted exact-head job evidence is recorded in the integration/E2E artifacts. | pass at the local/repository boundary |
| [INT-0006](../../../intents/INT-0006-distinguish-omitted-cli-id.md) | `test_protocol_distinguishes_absent_string_and_null_id`, `test_run_generates_id_when_member_is_omitted`, `test_run_rejects_present_non_string_ids_without_creating_state`, `test_cli_generates_id_when_member_is_omitted`, and `test_cli_reports_explicit_null_id_with_exit_2` preserve omission-only generation and strict present-value typing. | pass |

Because all Rust, manifest, lockfile, and CI paths are byte-identical to the
accepted base, the 100-test run is regression evidence rather than a claim that
Sprint 6 implemented any proposed backend or derivative runtime.

## Book, research, and repository confirmations

- Installed `research-budget.sh` returned `files=19 sources=4` and exited 0.
- Installed `check-book.sh` returned
  `check-book: valid v2 Book (12 intent chapters)` and exited 0.
- The independent whole-Book resolver passed all 91 Markdown files, 521 local
  links, and 7 fragment references at the exact Build head.
- The structural catalog inspection found exactly six recommended repository
  labels and 66 complete companion fields, with no duplicate primary slug or
  placeholder.
- `git diff --check` passed for the whole accepted-base-to-Build-head diff.

`check-book.sh` proves intent schema/state. The independent resolver proves
Summary reachability, local paths, and fragments; neither result is attributed
to the other tool.

## Build and task provenance

| Task | Semantic task commit | Completion-ledger evidence commit |
|------|----------------------|-----------------------------------|
| T-601 | `5cc52aba625acc9e0361014eca8aec0edbe55554` | `cbf2ac44bae15bb3219cd01a6731693ad217a9f5` |
| T-602 | `f1770f774bfafed538316f01c3cd05cd82270855` | `f511dab01ff44d21ce7eb71237d0fd63763d8e5c` |
| T-603 | `f38e974a903f9e4a0cac8a63778c0426877571b5` | `b6daf73cf4c12e496466ebdcb393b3204e7ffeb7` |

The verified ancestry is:

```text
bf5b5f299102d4853fa5312b1091ec0b8fb2dfe1
  -> 5cc52aba625acc9e0361014eca8aec0edbe55554
  -> cbf2ac44bae15bb3219cd01a6731693ad217a9f5
  -> f1770f774bfafed538316f01c3cd05cd82270855
  -> f511dab01ff44d21ce7eb71237d0fd63763d8e5c
  -> f38e974a903f9e4a0cac8a63778c0426877571b5
  -> b6daf73cf4c12e496466ebdcb393b3204e7ffeb7
```

The final T-603 evidence commit is the exact tested Build head. The complete
candidate's five local gates preceded both T-603 commits and were then repeated
against that head.

## Reproduction commands

```sh
git rev-parse HEAD
git merge-base --is-ancestor bf5b5f299102d4853fa5312b1091ec0b8fb2dfe1 b6daf73cf4c12e496466ebdcb393b3204e7ffeb7
git diff --name-only bf5b5f299102d4853fa5312b1091ec0b8fb2dfe1...b6daf73cf4c12e496466ebdcb393b3204e7ffeb7
git diff --check bf5b5f299102d4853fa5312b1091ec0b8fb2dfe1...b6daf73cf4c12e496466ebdcb393b3204e7ffeb7
git diff --quiet bf5b5f299102d4853fa5312b1091ec0b8fb2dfe1...b6daf73cf4c12e496466ebdcb393b3204e7ffeb7 -- Cargo.toml Cargo.lock .github README.md LICENSE crates docs/intents/INT-0001-chain-agnostic-intent-lifecycle-core.md docs/intents/INT-0002-runnable-lifecycle-adapter.md docs/intents/INT-0003-bounded-cli-request-ingestion.md docs/intents/INT-0004-explicit-cli-response-flush.md docs/intents/INT-0005-automated-rust-quality-gate.md docs/intents/INT-0006-distinguish-omitted-cli-id.md
git remote -v
git config --local --get-regexp '^remote\\.'
git submodule status
cargo +stable metadata --no-deps --format-version 1
cargo +stable tree --workspace --edges normal
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --all-features -- -D warnings
RUSTFLAGS="-D warnings" cargo +stable check --workspace --all-targets
cargo +stable test --workspace --all-targets
cargo +stable test --doc --workspace
bash /mnt/c/Users/charl/.codex/plugins/cache/sprint-loops/sprint-loop/local/skills/sprint-loop/scripts/research-budget.sh
bash /mnt/c/Users/charl/.codex/plugins/cache/sprint-loops/sprint-loop/local/skills/sprint-loop/scripts/check-book.sh
```

Every listed command passed at the tested Build head. This file is subsequent
Test evidence and does not redefine Build-head content or runtime behavior.
