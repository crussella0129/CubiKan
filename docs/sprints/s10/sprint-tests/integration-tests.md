# Sprint 10 Integration Test Results

- **Intent:** `INT-0013`
- **Accepted base:** `bb257db8c62083ae8be4e8d77ec63762ba2e8fa8`
- **Exact tested head:** `0a7bc3a023364cca9197e735c5acfeab019ce8a1`
- **Primary integration result:** 3 passed / 0 failed / 3 total
- **Structural-suite corroboration:** 13 passed / 0 failed / 13 selected
- **Conclusion:** pass at the exact tested head

The integration boundary is the real checked-in Project Book, appendix, root
project guide, backend guide, local-adapter guide, Git tree, and Sprint Loop
layout. No document, repository, backend, external provider, or filesystem
mock substitutes for those inputs. The checks are read-only apart from ordinary
tool caches; they create no derivative repository and perform no provider
mutation.

The exact tested head is the candidate to which these observations apply. This
result prose is Test-phase provenance written after that head and attributes no
additional product behavior to the evidence working tree.

## Exact structural command and result

The following commands selected the finalized names and executed the structural
suite:

```sh
bash docs/sprints/s10/sprint-tests/documentation-checks.sh list
bash docs/sprints/s10/sprint-tests/documentation-checks.sh structural
```

`documentation-checks.sh structural` selected the nine unit checks, the three
integration checks below, and `verify_repository_hygiene`. It reported exactly
`13 passed, 0 failed, 13 selected` at head
`0a7bc3a023364cca9197e735c5acfeab019ce8a1`. Each named integration check
reported `PASS`.

The runner invokes the authoritative Book-v2 validator and the checked-in
`markdown_resolver.py`. Repository hygiene also ran the resolver's reference
parser self-test and the checked-in `audit_evidence.py` negative/typed-fixture
self-test; both passed. The authority validator reported:

```text
check-book: valid v2 Book (13 intent chapters)
```

The resolver reported the same result on both its navigation invocation and
the repository-hygiene invocation:

```text
markdown_resolver: 136 Markdown files, 981 links, 898 local targets, 13 fragments, 13 Book intents; 0 errors
```

## Finalized integration checks

| Named check | Arrangement | Exact SHALL observation | Result |
|-------------|-------------|-------------------------|--------|
| `test_appendix_matches_current_project_and_backend_guides` | Read the current-boundary and integration statements in `docs/appendix/potential-derivative-projects.md` and cross-check exact contract language in `README.md`, `crates/cubikan-backend/README.md`, and `crates/cubikan-local/README.md`. No generated guide or substituted fixture is used. | All four guides SHALL describe the same four current surfaces: the chain-agnostic core library, stateless `cubikan` adapter, synchronous embedded SQLite backend, and explicit-path `cubikan-local` adapter. They SHALL agree on stored envelope v1, SQLite schemas v1/v2, relationship contract v1, projection query v1, explicit rather than implicit migration, and local protocol v1 remaining lifecycle-only while relationships/projections remain Rust-only. Availability SHALL not become a cross-version compatibility promise. | pass |
| `test_appendix_links_and_book_navigation_resolve` | Run the authoritative Book-v2 validator, then resolve every checked-in or unignored Markdown inline/reference link, repository-local path, local fragment, and Book intent target with `markdown_resolver.py`. Its parser self-test separately covers full, collapsed, and shortcut references plus fail-closed invalid forms. | Book validation SHALL reach exactly 13 intent chapters. All 12 pre-existing intents and new INT-0013 SHALL be present and reachable from `docs/SUMMARY.md`. In this evidence-bearing working tree the resolver inspected 136 Markdown files, 981 links, 898 local targets, and 13 fragments, reaching 13 Book intents with 0 errors. | pass |
| `test_documentation_maintenance_scope_is_non_product` | Resolve the accepted base as a real ancestor, compare `bb257db8c62083ae8be4e8d77ec63762ba2e8fa8..0a7bc3a023364cca9197e735c5acfeab019ce8a1`, inspect every changed path and every pre-existing intent, and run accepted-base `git diff --check`. Inspect both tracked paths and filesystem paths for legacy root authorities. | The accepted-base delta SHALL be limited to INT-0007's legal supersession, new INT-0013, the derivative appendix, `docs/SUMMARY.md` navigation, the two Book ledgers, and Sprint 10 provenance. Rust sources, Cargo manifests and lockfile, `.github`, `docs/work/remote-profile.md`, every other pre-existing intent chapter, and therefore the protected product implementation SHALL remain invariant. No tracked or writable root `sprints`, `agent-tasks`, or `decisions.md` authority SHALL exist. | pass |

## Cross-guide consistency conclusion

The appendix does not infer a relationship/process protocol from schema v2.
The root guide and backend guide expose relationships and ephemeral projections
only through the local Rust backend API, while the local executable continues
to expose exactly five protocol-v1 lifecycle operations. All four sources agree
that `open` does not migrate and that a Rust caller must explicitly migrate a
supported v1 database before reopening it for v2 capabilities. The appendix's
consumer guidance preserves that independent-version and authority boundary.

## Accepted-base scope conclusion

The candidate changes documentation and Book provenance only. Its allowed
scope contains the appendix maintenance itself, legal supersession of the
originating INT-0007 chapter, the active INT-0013 chapter, Book navigation and
work/completion ledgers, and Sprint 10 research, plans, task evidence, scripts,
and result placeholders. The protected source, manifest, CI, remote-profile,
and other-intent surfaces are byte-invariant relative to the accepted base.
Runtime behavior is therefore not redefined by this maintenance delta; the
separate workspace gates supply behavioral regression evidence.

There is one authoritative Book-v2 Sprint Loop layout under `docs/`. Neither
Git tracking nor the working filesystem contains a duplicate writable legacy
root authority. This evidence does not claim a product realization, derivative
authorization, external publication, provider mutation, or merge approval.
