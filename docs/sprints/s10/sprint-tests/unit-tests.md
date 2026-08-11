# Sprint 10 Unit and Repository Verification

- **Primary intent:** [INT-0013](../../../intents/INT-0013-maintain-derivative-ecosystem-current-state.md)
- **Accepted base:** `bb257db8c62083ae8be4e8d77ec63762ba2e8fa8`
- **Final tested candidate:** `0a7bc3a023364cca9197e735c5acfeab019ce8a1`
- **Executable oracle:** [`documentation-checks.sh`](documentation-checks.sh),
  supported by the read-only [`markdown_resolver.py`](markdown_resolver.py) and
  [`audit_evidence.py`](audit_evidence.py) self-tests
- **Conclusion:** pass; all 9 finalized unit/repository checks passed, and the
  complete structural selection passed 13/13 with the Book reporting all 13
  intent chapters

These checks inspect the checked-in Book, appendix, ledgers, and Git objects
directly. They are documentation/repository tests, not compiled Rust tests. No
stub, mock, database, network call, provider mutation, or derivative repository
is involved. Exact literals establish locked contract statements; bounded
section extraction and multiline-field review establish their local semantic
context; Git plumbing establishes historical identity and ancestry. The
separate E2E audit owns current provider observations and action-ledger claims.

The harness runs with `set -uo pipefail`, executes every selected check through
an output-capturing `run_check`, and returns a nonzero status when any check
fails. Its catalog oracle checks every relevant multiline field rather than a
single matching line. Its ledger oracle checks exact objects, subjects,
parentage, order, and path scope rather than accepting SHA-shaped prose. The
resolver and bounded action-audit helpers also have negative self-tests, so the
structural result is not based on an untested parser or a permissive evidence
shape.

## T-1001 current boundary and version matrix

### `test_current_boundary_and_version_matrix_are_current`

- **Arrangement:** Extract `Current CubiKan boundary` through `Architectural
  layers` from the live
  [appendix](../../../appendix/potential-derivative-projects.md), then require
  the exact present-tense descriptions and version statements for each
  supported surface.
- **SHALL assertion and observation:** The appendix shall distinguish exactly
  four bounded surfaces: chain-agnostic `cubikan-core`, stateless one-shot
  in-memory `cubikan`, embedded SQLite Rust library `cubikan-backend`, and the
  explicit-path durable JSON process adapter `cubikan-local`. It does. It names
  lifecycle schema v1, relationship-extension schema v2, relationship contract
  v1, and projection query v1; it says relationships/projections are exposed
  only through the Rust boundary; and it says local protocol v1 is
  lifecycle-only and exposes neither relationship nor projection operations.
- **Result:** pass — `verified four surfaces, schema v1/v2, Rust
  relationship/projection v1, and lifecycle-only local protocol v1`.

## T-1002 capability states, dependencies, and authorities

### `test_capability_map_statuses_match_book`

- **Arrangement:** Resolve exactly one Book chapter for each of INT-0008
  through INT-0012, read its `State` field, compare those values with the
  appendix capability map, reject the former blanket-proposed wording, and
  inspect the canonical-authority table and realized dependency statements.
- **SHALL assertion and observation:** The appendix shall match the Book:
  INT-0008 and INT-0011 are `proposed`; INT-0009, INT-0010, and INT-0012 are
  `realized`. It does, including the realized INT-0009 dependency of INT-0010
  and realized INT-0010 dependency of INT-0012. It retains one canonical
  authority for Book meaning/history, external-provider facts, bounded-domain
  business records and policy, and caller-owned measurement definitions and
  authorization policy. Neither hyphenated nor en-dash variants of the stale
  “INT-0008 through INT-0012 are proposed” claim remain.
- **Result:** pass — `matched five Book states, realized dependencies, and
  canonical authorities`.

## T-1003 consumption, authority transfer, and protocol guardrails

### `test_supported_consumption_paths_and_authority_transfer_are_explicit`

- **Arrangement:** Inspect the complete `Safe CubiKan integration baseline`
  and `Data-authority map` sections for the available local Rust boundary,
  explicitly pinned core path, separately governed provider/network adapters,
  and transfer-of-authority rules.
- **SHALL assertion and observation:** A consumer shall be able to select an
  explicitly pinned `cubikan-core` version or the available versioned
  `cubikan-backend` Rust boundary; local protocol v1 remains limited to its five
  lifecycle operations. Provider adapters and any network service require
  their own governance. The Book remains semantic and historical realization
  authority, while the backend is authoritative only for accepted durable
  Intent Unit and relationship state. Moving operational Book truth requires a
  **separately authorized** projection or migration intent with reconciliation
  and cutover rules, and Book/backend dual-write is prohibited. Every statement
  is present.
- **Result:** pass — `verified available backend/pinned-core paths, separately
  governed adapters, and one-way authority transfer`.

### `test_advisory_and_storage_protocol_boundaries_remain_intact`

- **Arrangement:** Inspect the advisory banner and safe-integration section for
  explicit noncommitment language, storage and Serde prohibitions,
  compatibility limits, and local-boundary nonclaims.
- **SHALL assertion and observation:** The appendix shall remain advisory and
  shall not assert that a recommended derivative repository exists. Consumers
  shall not edit a CubiKan database directly, share writable backend storage,
  treat provisional core Serde as a disk/wire contract, infer cross-version
  compatibility, or infer authentication, authorization, tenancy, deployment,
  blockchain, or network-service behavior. All prohibitions and nonclaims are
  explicit.
- **Result:** pass — `verified advisory status, storage/Serde prohibitions,
  compatibility limit, and explicit nonclaims`.

## T-1004 complete catalog and current prerequisites

### `test_catalog_remains_complete_and_preserves_edge_meaning`

- **Arrangement:** Count the six numbered entries and six exact recommended
  slugs. For every entry, require exactly one `Problem and outcome`, `Owned
  data`, `Owned policy`, `CubiKan interaction`, `Prerequisites`, `Creation
  trigger`, and `Explicit non-goals` field. Then inspect the backend/adapter/
  derivative classification, seven retained theme families, phase-edge rule,
  and the exact DV-01 through DV-09 boundary/disposition rows.
- **SHALL assertion and observation:** The catalog shall keep all six complete
  recommendations, all seven families (manager/doer operations,
  intent/artifact traceability, agent scoring, process/KPI definitions,
  multi-board skill pipelines, bounded organizational apps, and
  plan-versus-actual accounting), and all nine responsibility mappings. It
  does. The four non-lifecycle edge families do not reuse `WorkflowEdge`, so a
  lifecycle phase edge is not silently turned into delegation, provenance,
  relationship, or pipeline meaning.
- **Result:** pass — `verified six complete entries, seven theme families, nine
  exact responsibility mappings, and phase-edge non-conflation`.

### `test_catalog_prerequisites_use_realized_capabilities`

- **Arrangement:** Parse every multiline catalog field in all six entries.
  Where INT-0009, INT-0010, or INT-0012 occurs, require nearby current-state
  language showing that the realized primitive is available, and reject stale
  patterns such as “waits for,” “requires realization of,” or “remains
  proposed.” Separately preserve future wording for INT-0008 and INT-0011.
- **SHALL assertion and observation:** Realized revision, durability/query, and
  relationship/projection prerequisites shall be described as available,
  while provenance/evidence capabilities remain future until their own intents
  are realized. They are. The Observatory creation trigger now correctly says
  that a read-only Book/Git prototype may establish need **without depending on
  or writing to the available durable backend**; the obsolete “before a durable
  backend exists” statement is explicitly rejected.
- **Result:** pass — `reviewed all catalog references to realized and
  still-proposed capabilities`.

## T-1005 conditional creation governance

### `test_derivative_creation_remains_unauthorized`

- **Arrangement:** Inspect the advisory/noncommitment text and flatten the
  complete catalog to reject added global or named-repository claims of
  existence, authorization, or scheduling. Extract every entry's one creation
  trigger and require `Create only when` or `Create when` conditional form.
- **SHALL assertion and observation:** Every recommended name shall remain a
  boundary proposal rather than a creation request. Creation still requires a
  separately selected intent, explicit authorization, named owners, accepted
  data/policy/security boundaries, a compatible versioned integration
  contract, and the entry-specific trigger. The appendix contains six and only
  six conditional triggers and no affirmative existence, schedule, or
  authorization claim.
- **Result:** pass — `verified appendix noncommitment and six conditional
  creation triggers`.

This is a documentation-governance oracle. It does not claim an account-wide
provider inventory or erased external history; `verify_no_derivative_repository_operations`
owns the separately bounded durable-provider and sprint-action evidence in
[`e2e-tests.md`](e2e-tests.md).

## T-1006 sequencing and open questions

### `test_sequence_and_open_questions_exclude_completed_foundations`

- **Arrangement:** Extract `Sequencing and creation gates`, `Open questions`,
  and `Appendix-wide non-goals`. Reject questions asking whether INT-0009,
  INT-0010, or INT-0012 will be realized; require the closed-foundation
  statements, unresolved policy families, representative exact questions, and
  the corrected intent-state non-goal.
- **SHALL assertion and observation:** Revision, durable backend, and
  relationship/projection foundations shall be treated as completed. They are.
  Compatibility, authorization, provenance, privacy, deployment, blockchain,
  security, evidence, UI, and derivative-owned policy questions remain open,
  with explicit identity, measurement, relationship, skill-execution, and
  accounting-policy questions.
  The non-goal protects realized INT-0001–INT-0006, INT-0009, INT-0010, and
  INT-0012; distinguishes superseded INT-0007; and does not advance proposed
  INT-0008 or INT-0011.
- **Result:** pass — `verified realized foundations are closed, eight policy
  themes remain open, and superseded INT-0007 is not redefined`.

## T-1007 backlog and commit attribution

### `test_backlog_moves_once_to_completion_ledger`

- **Arrangement:** Read the exact maintenance backlog line from the accepted
  base with `git show`; compare it with the current
  [task ledger](../../../work/tasks.md) and
  [completion ledger](../../../work/completed-tasks.md); then inspect every
  normative task commit, its pending ledger tree, its direct evidence child,
  its locked integrated implementation object, ancestry, exact subject, and
  allowed path set with Git plumbing.
- **SHALL assertion and observation:** The original INT-0007 maintenance line
  shall be absent from the live backlog. `MAINT-001` and each of T-1001 through
  T-1007 shall occur once, in dependency order. They do. `MAINT-001` attributes
  the originating realized backlog to INT-0007, names INT-0013 as the
  superseding reconciliation authority, and records integrated T-1007 object
  `a7ed48992897c8463ba6cc729e944398c8ae8779`. Each T-100x normative `Commit`
  is the helper-created Book reconciliation object, not the integrated legacy
  object or the authority-repair object; its pending tree contains `PENDING`,
  and exactly one direct child records the commit evidence.
- **Result:** pass — `verified exact backlog closure, singular MAINT-001, seven
  ordered Book commits, and seven locked legacy mappings`.

The exact ledger mapping observed is:

| Task | Normative Book commit | Evidence child | Locked integrated implementation |
|------|-----------------------|----------------|----------------------------------|
| T-1001 | `b6ba646e88093e8f88eedd31b04405b44a031a82` | `41955e0083fc5a0b634d0379f82a2f48a3028631` | `d725411e0bf4c97437544e28c604e48f0c1badbf` |
| T-1002 | `1a7c1b210f24170f4f07c4dbd700e8bf58c320d5` | `bdfebb8f83df1295a9928b4c8ffa9ffab21da35f` | `a4c14cfcaccc23afeebafe28490b63b0683d17e8` |
| T-1003 | `c923c0b8dae3ac56a40a7d738a192d5359429ea4` | `55c49bbb1d2ff2db29a53001c0f5c531fcf2a996` | `a3e6aec3afe739091d03103744a82d89ad1c467b` |
| T-1004 | `b797cc832363639ba6343ee274cec321b65240e2` | `df6761bd2af1e64e3ce5adb472069d58f86a5d0e` | `336b4e48e791f9a7d0a25e5de84c9404c3e266d2` |
| T-1005 | `168f9b1843d031ee1430bbbf689a4ecd48bf1db5` | `6fc2d8392912aa6a9989af04b475f1ce52ac788f` | `99864da63fc9a51b24ead1d5792c4d6b7f706207` |
| T-1006 | `aa98b41c8d7ce96ad94e281bb9dbc323ec834868` | `306182a33d0a3dc7db5a3003412c3f960281dd5f` | `9517dc17797f25e7a2d8f924abf1b5d51fb62e5a` |
| T-1007 | `ec2dac4bda1a1e615bdb0bc0d99b54f6dbbcaacb` | `4040493ce4cb3ff060d10721211e3ec1135de6d5` | `a7ed48992897c8463ba6cc729e944398c8ae8779` |

Authority restoration is intentionally separate:
`b170e107d08ac1855d6b1be82fbf1ebe25a22f3a` has the exact subject `Restore
Book-v2 Sprint Loop authority`, is not attributed to MAINT-001 or any T-100x
product task, and precedes the normative Sprint 10 reconciliation history.

## Correction and hardened-oracle provenance

Candidate commit `cdede78dae2d3838328b63abd6171e54d4a557f4`
introduced the executable exact-head harness and corrected two appendix
statements exposed by its semantic checks:

1. Operational Book truth now requires a separately **authorized** projection
   or migration intent, not merely a selected one.
2. A read-only Book/Git prototype is now described as independent of and
   non-writing to the already available durable backend, rather than occurring
   “before a durable backend exists.”

The first correction is enforced by the T-1003 authority-transfer oracle; the
second is enforced by the T-1004 multiline prerequisite oracle and an explicit
stale-literal rejection. Candidate commit
`1c26ee010b8c201a8e8092255df4b949011ed21a` further narrowed the audit success
message to what its evidence proves: no targeted derivative repository was
found and no derivative mutation was recorded. It does not elevate a bounded
targeted audit into an account-wide nonexistence claim. That hardening changes
no unit result, but it preserves the oracle boundary shared with E2E evidence.

Final tested candidate commit
`0a7bc3a023364cca9197e735c5acfeab019ce8a1` corrected the malformed first-push
SHA exposed by the formal critic and made the offline validator require full
resolvable push commits, ordered candidate ancestry, final-push equality,
observation-window timestamps, and scoped repository findings. The durable
audit fixture is bound to this candidate while remaining replayable from later
evidence-only checkouts where this candidate is an ancestor.

## Exact execution

The complete structural command was:

```text
bash docs/sprints/s10/sprint-tests/documentation-checks.sh structural
```

At final tested candidate `0a7bc3a023364cca9197e735c5acfeab019ce8a1`,
the runner reported each of the nine checks above as `PASS`. The full selection
reported `13 passed, 0 failed, 13 selected`: nine unit/repository checks, three
integration checks, and repository hygiene. Its independent structure oracles
reported:

```text
check-book: valid v2 Book (13 intent chapters)
markdown_resolver: 136 Markdown files, 967 links, 886 local targets,
13 fragments, 13 Book intents; 0 errors
layout-router: test
```

The resolver self-test accepted full, collapsed, and shortcut references and
rejected invalid forms. The offline audit helper self-test rejected five
unsafe/unknown actions, malformed and unresolved push heads, and four invalid
evidence shapes and accepted its typed offline fixture. These helper results
establish parser/evidence-shape behavior;
they do not substitute for the nine unit observations above or for provider
evidence in the E2E artifact.

After all three result artifacts were populated, repository-wide Markdown
resolution and `git diff --check` were rerun and passed with 0 errors. Those
working-tree checks validate the evidence prose without replacing the exact
committed-candidate counts above.
