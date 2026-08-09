mod common;

use common::{TestDatabase, fixed_id, linear_workflow, phase, stored_rows};
use cubikan_backend::{
    BackendError, CompleteIntentUnit, CreateIntentUnit, IntentUnitSummary, IntentUnitView,
    ListFilters, ListIntentUnits, PageLimit, SqliteBackend, TransitionIntentUnit,
};
use cubikan_core::{
    IntentSpecies, IntentUnit, IntentUnitRevision, IntentUnitStatus, LifecycleRecord,
};

fn species(value: &str) -> IntentSpecies {
    IntentSpecies::new(value).expect("fixture species should be valid")
}

#[test]
fn test_create_commits_complete_revision_zero_unit() {
    let database = TestDatabase::new("create");
    let ordinary = fixed_id("00000000-0000-0000-0000-000000000001");
    let nil = fixed_id("00000000-0000-0000-0000-000000000000");
    let workflow = linear_workflow("delivery", "queued", "done");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");

    let ordinary_view = backend
        .create(CreateIntentUnit::new(
            Some(ordinary),
            species("feature"),
            workflow.clone(),
        ))
        .expect("supplied ordinary ID should create");
    {
        let fresh =
            SqliteBackend::open(database.path()).expect("ordinary create should be committed");
        assert_eq!(fresh.get(ordinary).unwrap(), ordinary_view);
    }
    let nil_view = backend
        .create(CreateIntentUnit::new(
            Some(nil),
            species("nil-feature"),
            workflow.clone(),
        ))
        .expect("supplied nil ID should create");
    {
        let fresh = SqliteBackend::open(database.path()).expect("nil create should be committed");
        assert_eq!(fresh.get(nil).unwrap(), nil_view);
    }
    let generated_view = backend
        .create(CreateIntentUnit::new(
            None,
            species("generated-feature"),
            workflow,
        ))
        .expect("omitted ID should generate");
    {
        let fresh =
            SqliteBackend::open(database.path()).expect("generated create should be committed");
        assert_eq!(fresh.get(generated_view.id()).unwrap(), generated_view);
    }

    assert_eq!(ordinary_view.id(), ordinary);
    assert_eq!(nil_view.id(), nil);
    assert!(!generated_view.id().as_uuid().is_nil());
    assert_eq!(generated_view.id().as_uuid().get_version_num(), 4);
    for view in [&ordinary_view, &nil_view, &generated_view] {
        assert_eq!(view.status(), IntentUnitStatus::Active);
        assert_eq!(view.revision(), IntentUnitRevision::INITIAL);
        assert!(view.history().is_empty());
    }

    let fresh = SqliteBackend::open(database.path()).expect("fresh connection should reopen");
    assert_eq!(fresh.get(ordinary).unwrap(), ordinary_view);
    assert_eq!(fresh.get(nil).unwrap(), nil_view);
    assert_eq!(fresh.get(generated_view.id()).unwrap(), generated_view);
    let rows = stored_rows(&database.connect());
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|row| row.envelope_version == 1));
    assert!(rows.iter().all(|row| row.revision == [0_u8; 8]));
}

#[test]
fn test_create_get_round_trip_multiple_units_across_reopen() {
    let database = TestDatabase::new("round-trip");
    let first_id = fixed_id("10000000-0000-0000-0000-000000000001");
    let second_id = fixed_id("20000000-0000-0000-0000-000000000002");
    let first_workflow = linear_workflow("  delivery α  ", "待機", "完了");
    let second_workflow = linear_workflow("support", "intake", "resolved");
    let (first_created, second_created) = {
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let first = backend
            .create(CreateIntentUnit::new(
                Some(first_id),
                species("  feature α  "),
                first_workflow.clone(),
            ))
            .expect("first unit should create");
        let second = backend
            .create(CreateIntentUnit::new(
                Some(second_id),
                species("support-case"),
                second_workflow.clone(),
            ))
            .expect("second unit should create");
        (first, second)
    };

    let reopened = SqliteBackend::open(database.path()).expect("database should reopen");
    let first = reopened.get(first_id).expect("first unit should load");
    let second = reopened.get(second_id).expect("second unit should load");

    assert_eq!(first, first_created);
    assert_eq!(second, second_created);
    assert_eq!(first.workflow(), &first_workflow);
    assert_eq!(second.workflow(), &second_workflow);
    assert_ne!(first.id(), second.id());
    assert_ne!(first.species(), second.species());
    assert!(first.history().is_empty());
    assert!(second.history().is_empty());
}

#[test]
fn test_duplicate_create_and_missing_get_are_typed_and_nonmutating() {
    let database = TestDatabase::new("duplicate-missing");
    let accepted_id = fixed_id("30000000-0000-0000-0000-000000000003");
    let missing_id = fixed_id("40000000-0000-0000-0000-000000000004");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    backend
        .create(CreateIntentUnit::new(
            Some(accepted_id),
            species("accepted"),
            linear_workflow("accepted-flow", "queued", "done"),
        ))
        .expect("fixture unit should create");
    let before = stored_rows(&database.connect());

    let duplicate = backend
        .create(CreateIntentUnit::new(
            Some(accepted_id),
            species("different"),
            linear_workflow("different-flow", "new", "closed"),
        ))
        .expect_err("duplicate ID should fail");
    assert_eq!(
        duplicate,
        BackendError::DuplicateIntentUnit { id: accepted_id }
    );
    assert_eq!(stored_rows(&database.connect()), before);

    let missing = backend
        .get(missing_id)
        .expect_err("unknown ID should be typed");
    assert_eq!(missing, BackendError::IntentUnitNotFound { id: missing_id });
    assert_eq!(stored_rows(&database.connect()), before);
}

#[test]
fn test_backend_codec_schema_crud_query_and_mutation_compose() {
    let database = TestDatabase::new("cross-component-compose");
    let id = fixed_id("50000000-0000-0000-0000-000000000005");
    let unit_species = species("cross-component-feature");
    let workflow = linear_workflow("custom-compose-flow", "draft", "shipped");

    let created = {
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        backend
            .create(CreateIntentUnit::new(
                Some(id),
                unit_species.clone(),
                workflow.clone(),
            ))
            .expect("custom-workflow unit should create")
    };
    assert_eq!(created.revision(), IntentUnitRevision::INITIAL);
    assert_eq!(created.workflow(), &workflow);

    let transitioned = {
        let mut backend = SqliteBackend::open(database.path()).expect("database should reopen");
        assert_eq!(
            backend.get(id).expect("created unit should replay"),
            created
        );

        let page = backend
            .list(ListIntentUnits::new(
                ListFilters::new(
                    Some(workflow.id().clone()),
                    Some(unit_species.clone()),
                    Some(phase("draft")),
                    Some(IntentUnitStatus::Active),
                ),
                PageLimit::new(1).expect("fixture limit should be valid"),
                None,
            ))
            .expect("created unit should be queryable after reopen");
        assert_eq!(page.items(), [IntentUnitSummary::from_view(&created)]);
        assert_eq!(page.next_cursor(), None);

        backend
            .transition(TransitionIntentUnit::new(
                id,
                phase("shipped"),
                IntentUnitRevision::INITIAL,
            ))
            .expect("guarded transition should commit")
    };
    assert_eq!(
        transitioned.committed_revision(),
        IntentUnitRevision::new(1)
    );
    assert_eq!(transitioned.intent_unit().workflow(), &workflow);

    let completed = {
        let mut backend =
            SqliteBackend::open(database.path()).expect("database should reopen for completion");
        assert_eq!(
            backend
                .get(id)
                .expect("transitioned unit should replay before completion"),
            *transitioned.intent_unit()
        );
        backend
            .complete(CompleteIntentUnit::new(
                id,
                transitioned.committed_revision(),
            ))
            .expect("guarded completion should commit")
    };

    let final_backend =
        SqliteBackend::open(database.path()).expect("database should reopen after completion");
    let final_view = final_backend
        .get(id)
        .expect("completed unit should replay after reopen");
    assert_eq!(final_view, *completed.intent_unit());
    assert_eq!(final_view.revision(), IntentUnitRevision::new(2));
    assert_eq!(final_view.status(), IntentUnitStatus::Completed);
    assert_eq!(final_view.workflow(), &workflow);

    let mut expected = IntentUnit::new(id, unit_species.clone(), workflow.clone());
    expected
        .transition_to(&phase("shipped"))
        .expect("expected transition should be valid");
    expected
        .complete()
        .expect("expected completion should be valid");
    assert_eq!(final_view, IntentUnitView::from_intent_unit(&expected));
    assert!(matches!(
        final_view.history(),
        [
            LifecycleRecord::Transition(_),
            LifecycleRecord::Completion(_)
        ]
    ));

    let completed_page = final_backend
        .list(ListIntentUnits::new(
            ListFilters::new(
                Some(workflow.id().clone()),
                Some(unit_species),
                Some(phase("shipped")),
                Some(IntentUnitStatus::Completed),
            ),
            PageLimit::new(1).expect("fixture limit should be valid"),
            None,
        ))
        .expect("completed unit should remain queryable");
    assert_eq!(
        completed_page.items(),
        [IntentUnitSummary::from_view(&final_view)]
    );
    assert_eq!(completed_page.next_cursor(), None);
}
