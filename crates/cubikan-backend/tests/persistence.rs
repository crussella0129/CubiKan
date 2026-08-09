mod common;

use common::{TestDatabase, fixed_id, linear_workflow, stored_rows};
use cubikan_backend::{BackendError, CreateIntentUnit, SqliteBackend};
use cubikan_core::{IntentSpecies, IntentUnitRevision, IntentUnitStatus};

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
