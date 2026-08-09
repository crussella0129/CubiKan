# INT-0008 — Traceable intent instantiation and artifact provenance

<!-- sprint-loop-intent-v2 -->
- **Intent ID:** INT-0008
- **State:** proposed
- **Work evidence:** none
- **Completion evidence:** none
- **Code evidence:** none
- **Test evidence:** none
- **Documentation evidence:** none

## Intent

Enable CubiKan to preserve an explicit relationship from an Intent Unit to the
external intent that caused its instantiation and to associate lifecycle work
with revision-addressed artifacts such as commits, pull requests, tests, and
documents. The CubiKan-owned model will remain provider-neutral; Book parsing,
Git hosting, blame analysis, and rendered links belong in adapters or derivative
projects.

An Intent Unit ID and a Project Book intent ID are distinct identities. This
intent does not make Git blame authoritative, infer authorship, certify evidence,
or add actor identity, authorization, timestamps, cryptographic proofs, or
blockchain policy.

## Acceptance criteria

- An Intent Unit can preserve a validated, immutable, caller-supplied origin
  reference whose namespace and value remain distinct from `IntentUnitId` and
  survive transitions, completion, serialization, and validated restoration.
- A provider-neutral evidence association can link a unit or exact lifecycle
  revision to one or more external artifact references without embedding a Git,
  GitHub, Project Book, filesystem, or URL client in `cubikan-core`.
- Evidence relationships support the many-to-many reality that one intent may
  produce multiple commits and one commit may serve multiple units, with exact
  duplicate and correction behavior documented and tested.
- Callers can query provenance from unit to artifact and artifact to unit while
  rejected or malformed associations leave accepted lifecycle and evidence
  state unchanged.
- At least one Git-facing adapter demonstration correlates a fixed full commit
  identity through structured metadata and proves that source moves or blame
  output do not rewrite the stored CubiKan relationship.
- Documentation distinguishes recorded association, externally verified
  evidence, human attribution, and causal claims; none is silently promoted to
  another.

## Rationale

The Book already records intent, task, modified-file, and commit relationships
manually. Making the reusable relationship queryable would support direct
intent-to-instantiation traceability while allowing Git and agent analytics to
remain separate consumers. W3C PROV’s separation of entities, activities, and
agents reinforces the value of a provider-neutral core relationship rather than
hard-coding a single source-control model.

## Alternatives

Storing only a commit hash on `IntentUnit` is too narrow for tests, documents,
pull requests, and non-Git work. Treating `git blame` as the source of truth was
rejected because line attribution changes with repository history and does not
prove intent or causality. Embedding full Book or Git objects in the lifecycle
aggregate would couple the core to external schemas and retention policy.

## Consequences

Full realization depends on [INT-0009](INT-0009-revisioned-lifecycle-commands.md)
for revision-scoped evidence and [INT-0010](INT-0010-durable-intent-unit-backend.md)
for a durable many-to-many index and reverse queries. An immutable origin
reference could be explored earlier, but it would not satisfy this whole intent.

The relation vocabulary, correction policy, repository identity, private-source
retention, and evidence-verification boundary must be decided before planning
implementation. Rich provenance can expose sensitive organizational and agent
data, so downstream analytics require explicit data minimization, retention,
redaction, access, and human-approval policy.

## Transition history

- 2026-08-08: created as `proposed` after Sprint 6 research identified original-intent-to-code traceability as the strongest reusable capability in the user-provided design discussion.
- 2026-08-08: revised while `proposed` to make revision-scoped evidence depend on INT-0009 and bidirectional durable indexing depend on INT-0010.
