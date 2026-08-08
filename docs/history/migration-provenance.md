# Legacy Sprint Loops Migration Provenance

<!-- sprint-loop-migration-v2 -->

- **Book schema version:** 2
- **Migrated at:** 2026-08-08T15:32:31Z
- **Authority:** Historical provenance only. The `docs/` Book is the sole writable Sprint Loops authority.

## Path mappings

- `sprints/` -> `docs/sprints/`
- `agent-tasks/agent-tasks.md` -> `docs/work/tasks.md`
- `agent-tasks/completed-tasks.md` -> `docs/work/completed-tasks.md`
- Other `agent-tasks/` content -> the same relative path under `docs/work/`
- `confidence.txt` -> `docs/work/confidence.txt`
- `decisions.md` -> `docs/history/decisions-legacy.md` (non-authoritative history)

## Content inventory

Each indented row is `type<TAB>sha256-or--<TAB>legacy-path<TAB>Book-path`.

    D	-	agent-tasks	docs/work
    D	-	sprints	docs/sprints
    D	-	sprints/s0	docs/sprints/s0
    D	-	sprints/s0/sprint-plans	docs/sprints/s0/sprint-plans
    D	-	sprints/s0/sprint-research	docs/sprints/s0/sprint-research
    D	-	sprints/s0/sprint-tests	docs/sprints/s0/sprint-tests
    D	-	sprints/s1	docs/sprints/s1
    D	-	sprints/s1/sprint-plans	docs/sprints/s1/sprint-plans
    D	-	sprints/s1/sprint-research	docs/sprints/s1/sprint-research
    D	-	sprints/s1/sprint-tests	docs/sprints/s1/sprint-tests
    F	069d3e6facf87861c1f98c9412add4688d92bdc60cae83001511edce48cc33fe	sprints/s0/sprint-tests/integration-tests.md	docs/sprints/s0/sprint-tests/integration-tests.md
    F	2ed2749fe76591f44fd7bedaebc713970754bd0d702de59f95a4e47672866255	sprints/s0/sprint-plans/critique.md	docs/sprints/s0/sprint-plans/critique.md
    F	756384dd05d729bd4d96729eb891a7144e96c4e7d2d401207b60538287f95249	sprints/s0/sprint-plans/build-plan.md	docs/sprints/s0/sprint-plans/build-plan.md
    F	7bb8ee45204cb67ffb6d61f1bce479af271965d02c7a5e1ad8d0aed03e654fc1	sprints/s0/sprint-meta.md	docs/sprints/s0/sprint-meta.md
    F	8655928f91ae828ac4ff5033038a45bfb4266ddf8802039e379137446c3c5130	sprints/s0/sprint-tests/e2e-tests.md	docs/sprints/s0/sprint-tests/e2e-tests.md
    F	8e5f7ffbbce05b245fedcebfdee23f3589d28ca4f517cdc102ab873fba2b7477	sprints/s0/sprint-research/research-report.md	docs/sprints/s0/sprint-research/research-report.md
    F	965e1d75161c8b951952fde3327e6965c54c029d5b22d21575fa123528527658	sprints/s0/sprint-tests/critique.md	docs/sprints/s0/sprint-tests/critique.md
    F	a4e4d69fdd2cd0a28b3c0a9a0c89dfd9bea2aa91d17b15f228643343c400685e	sprints/s1/sprint-meta.md	docs/sprints/s1/sprint-meta.md
    F	b10c76f29aa34eb7bab77eff65ce962035f51d53cac4abb4c5182d16a480a88d	decisions.md	docs/history/decisions-legacy.md
    F	b118c4a239e5fe112a151f66573d3b777e77fb6589122fd0f0eb9d2b7798e3ea	sprints/s0/sprint-plans/test-plan.md	docs/sprints/s0/sprint-plans/test-plan.md
    F	b91408e427f71dd7c0965ef4af0dd2b7cb11b2f1fefd3b0f10b758ef1a55bfb0	agent-tasks/agent-tasks.md	docs/work/tasks.md
    F	c558aff5d11f65e473ecffc851452098ddcb1660c5c343148339d78553e75633	sprints/s0/sprint-tests/test-report.md	docs/sprints/s0/sprint-tests/test-report.md
    F	d8220dc2ece614da25f40189f02215c76ff735c1ee91a3830d2c511097e8c488	sprints/s0/sprint-tests/unit-tests.md	docs/sprints/s0/sprint-tests/unit-tests.md
    F	e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855	sprints/s1/sprint-plans/build-plan.md	docs/sprints/s1/sprint-plans/build-plan.md
    F	e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855	sprints/s1/sprint-plans/test-plan.md	docs/sprints/s1/sprint-plans/test-plan.md
    F	e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855	sprints/s1/sprint-research/research-report.md	docs/sprints/s1/sprint-research/research-report.md
    F	e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855	sprints/s1/sprint-tests/e2e-tests.md	docs/sprints/s1/sprint-tests/e2e-tests.md
    F	e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855	sprints/s1/sprint-tests/integration-tests.md	docs/sprints/s1/sprint-tests/integration-tests.md
    F	e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855	sprints/s1/sprint-tests/test-report.md	docs/sprints/s1/sprint-tests/test-report.md
    F	e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855	sprints/s1/sprint-tests/unit-tests.md	docs/sprints/s1/sprint-tests/unit-tests.md
    F	ea8dbb5d1b8e20b83033c6ee9d98bcb3f96d33c2704fd9d2f57e1974ca203021	agent-tasks/completed-tasks.md	docs/work/completed-tasks.md

## Post-migration normalization

- 2026-08-08: Removed the empty Sprint 1 scaffold created by the legacy Loop implementation. It contained no research, plan, build, or test evidence and was created before Sprint 0's required `dev → main` human-approval checkpoint. The inventory above remains the source-time migration manifest; the canonical router now stops at `ready-for-next-sprint` until that checkpoint is accepted.
- 2026-08-08: Preserved the finalized Sprint 0 build and test plans as locked legacy provenance. Book v2 semantic authority and realization links are supplied by INT-0001, Sprint 0 metadata, the normalized work ledgers, and the intent-verification section in the test report; the locked plans' task scope and EARS clauses were not rewritten after completion.
