mod common;

use std::str::FromStr;

use common::{TestDatabase, fixed_id, phase, replace_stored_unit, stored_rows};
use cubikan_backend::{
    BackendError, CreateIntentUnit, IntentUnitPage, ListCursor, ListFilters, ListIntentUnits,
    PageLimit, SqliteBackend,
};
use cubikan_core::{
    IntentSpecies, IntentUnit, IntentUnitId, IntentUnitStatus, Workflow, WorkflowEdge, WorkflowId,
};

fn species(value: &str) -> IntentSpecies {
    IntentSpecies::new(value).expect("fixture species should be valid")
}

fn workflow(id: &str, initial: &str, terminal: &str) -> Workflow {
    let initial = phase(initial);
    let terminal = phase(terminal);
    Workflow::new(
        WorkflowId::new(id).expect("fixture workflow ID should be valid"),
        vec![terminal.clone(), initial.clone()],
        initial.clone(),
        vec![WorkflowEdge::new(initial, terminal.clone())],
        vec![terminal],
    )
    .expect("fixture workflow should be valid")
}

fn three_phase_workflow(id: &str) -> Workflow {
    let queued = phase("queued");
    let doing = phase("doing");
    let done = phase("done");
    Workflow::new(
        WorkflowId::new(id).expect("fixture workflow ID should be valid"),
        vec![queued.clone(), doing.clone(), done.clone()],
        queued.clone(),
        vec![
            WorkflowEdge::new(queued, doing.clone()),
            WorkflowEdge::new(doing, done.clone()),
        ],
        vec![done],
    )
    .expect("fixture workflow should be valid")
}

fn create_unit(
    backend: &mut SqliteBackend,
    id: IntentUnitId,
    unit_species: IntentSpecies,
    unit_workflow: Workflow,
) -> IntentUnit {
    backend
        .create(CreateIntentUnit::new(
            Some(id),
            unit_species.clone(),
            unit_workflow.clone(),
        ))
        .expect("fixture unit should create");
    IntentUnit::new(id, unit_species, unit_workflow)
}

fn page(
    backend: &SqliteBackend,
    filters: ListFilters,
    limit: usize,
    after: Option<ListCursor>,
) -> IntentUnitPage {
    backend
        .list(ListIntentUnits::new(
            filters,
            PageLimit::new(limit).expect("fixture limit should be valid"),
            after,
        ))
        .expect("fixture page should succeed")
}

fn ids(page: &IntentUnitPage) -> Vec<IntentUnitId> {
    page.items().iter().map(|summary| summary.id()).collect()
}

fn cursor(id: IntentUnitId) -> ListCursor {
    ListCursor::from_str(&id.to_string()).expect("fixture cursor should be canonical")
}

fn ordinal_id(ordinal: u64) -> IntentUnitId {
    fixed_id(&format!("00000000-0000-0000-0000-{ordinal:012x}"))
}

#[test]
fn test_list_filters_exact_fields_and_orders_ids_lexically() {
    let database = TestDatabase::new("query-filters");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    let shared_a = workflow("shared-flow", "queued", "done");
    let shared_b = workflow("shared-flow", "inbox", "closed");
    let other = workflow("other-flow", "queued", "done");
    let completed = workflow("completed-flow", "queued", "done");

    let cases = [
        (ordinal_id(8), species("feature"), completed.clone()),
        (ordinal_id(2), species("feature"), shared_b.clone()),
        (ordinal_id(7), species("' OR 1=1 --"), other.clone()),
        (ordinal_id(1), species("feature"), shared_a.clone()),
        (ordinal_id(6), species(" feature "), other.clone()),
        (ordinal_id(4), species("féature"), other.clone()),
        (ordinal_id(3), species("Feature"), other.clone()),
        (ordinal_id(5), species("féature"), other.clone()),
    ];
    let mut completed_unit = None;
    for (id, unit_species, unit_workflow) in cases {
        let unit = create_unit(&mut backend, id, unit_species, unit_workflow);
        if id == ordinal_id(8) {
            completed_unit = Some(unit);
        }
    }
    let mut completed_unit = completed_unit.expect("completed fixture should exist");
    completed_unit
        .transition_to(&phase("done"))
        .expect("fixture transition should succeed");
    completed_unit
        .complete()
        .expect("fixture completion should succeed");
    replace_stored_unit(&database.connect(), &completed_unit);

    let all = page(&backend, ListFilters::default(), 100, None);
    assert_eq!(
        ids(&all),
        (1..=8).map(ordinal_id).collect::<Vec<_>>(),
        "insertion order must not affect canonical ID ordering"
    );

    let shared = page(
        &backend,
        ListFilters::new(Some(shared_a.id().clone()), None, None, None),
        100,
        None,
    );
    assert_eq!(ids(&shared), [ordinal_id(1), ordinal_id(2)]);
    assert_ne!(shared_a.phases(), shared_b.phases());

    let exact_feature = page(
        &backend,
        ListFilters::new(None, Some(species("feature")), None, None),
        100,
        None,
    );
    assert_eq!(
        ids(&exact_feature),
        [ordinal_id(1), ordinal_id(2), ordinal_id(8)]
    );
    let exact_phase = page(
        &backend,
        ListFilters::new(None, None, Some(phase("queued")), None),
        100,
        None,
    );
    assert_eq!(
        ids(&exact_phase),
        [
            ordinal_id(1),
            ordinal_id(3),
            ordinal_id(4),
            ordinal_id(5),
            ordinal_id(6),
            ordinal_id(7),
        ]
    );
    let completed_only = page(
        &backend,
        ListFilters::new(None, None, None, Some(IntentUnitStatus::Completed)),
        100,
        None,
    );
    assert_eq!(ids(&completed_only), [ordinal_id(8)]);
    let intersection = page(
        &backend,
        ListFilters::new(
            Some(shared_a.id().clone()),
            Some(species("feature")),
            Some(phase("queued")),
            Some(IntentUnitStatus::Active),
        ),
        100,
        None,
    );
    assert_eq!(ids(&intersection), [ordinal_id(1)]);

    for (value, expected) in [
        ("Feature", ordinal_id(3)),
        ("féature", ordinal_id(4)),
        ("féature", ordinal_id(5)),
        (" feature ", ordinal_id(6)),
        ("' OR 1=1 --", ordinal_id(7)),
    ] {
        let exact = page(
            &backend,
            ListFilters::new(None, Some(species(value)), None, None),
            100,
            None,
        );
        assert_eq!(
            ids(&exact),
            [expected],
            "filter must bind `{value}` exactly"
        );
    }
}

#[test]
fn test_list_enforces_bounds_and_exclusive_keyset_cursor() {
    let database = TestDatabase::new("query-pagination");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    let unit_workflow = workflow("pagination", "queued", "done");
    for ordinal in (0..=100).rev() {
        create_unit(
            &mut backend,
            ordinal_id(ordinal),
            species("feature"),
            unit_workflow.clone(),
        );
    }

    let first = page(&backend, ListFilters::default(), 100, None);
    assert_eq!(first.items().len(), 100);
    assert_eq!(first.items().first().unwrap().id(), ordinal_id(0));
    assert_eq!(first.items().last().unwrap().id(), ordinal_id(99));
    assert_eq!(first.next_cursor(), Some(cursor(ordinal_id(99))));
    let second = page(&backend, ListFilters::default(), 100, first.next_cursor());
    assert_eq!(ids(&second), [ordinal_id(100)]);
    assert_eq!(second.next_cursor(), None);

    let mut observed = Vec::new();
    let mut after = None;
    loop {
        let current = page(&backend, ListFilters::default(), 1, after);
        observed.extend(ids(&current));
        match current.next_cursor() {
            Some(next) => after = Some(next),
            None => break,
        }
    }
    assert_eq!(observed, (0..=100).map(ordinal_id).collect::<Vec<_>>());

    let after_nil = page(
        &backend,
        ListFilters::default(),
        1,
        Some(cursor(ordinal_id(0))),
    );
    assert_eq!(ids(&after_nil), [ordinal_id(1)]);
    let after_absent = page(
        &backend,
        ListFilters::default(),
        100,
        Some(cursor(ordinal_id(101))),
    );
    assert!(after_absent.items().is_empty());
    assert_eq!(after_absent.next_cursor(), None);

    assert!(PageLimit::new(0).is_err());
    assert!(PageLimit::new(101).is_err());
}

#[test]
fn test_list_pages_are_live_committed_views_with_mutable_membership() {
    let database = TestDatabase::new("query-live");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    let unit_workflow = three_phase_workflow("live-flow");

    let mut id_02 = create_unit(
        &mut backend,
        ordinal_id(2),
        species("feature"),
        unit_workflow.clone(),
    );
    id_02
        .transition_to(&phase("doing"))
        .expect("fixture transition should succeed");
    replace_stored_unit(&database.connect(), &id_02);
    let mut id_04 = create_unit(
        &mut backend,
        ordinal_id(4),
        species("feature"),
        unit_workflow.clone(),
    );
    id_04
        .transition_to(&phase("doing"))
        .expect("fixture transition should succeed");
    replace_stored_unit(&database.connect(), &id_04);

    let filters = ListFilters::new(None, None, Some(phase("doing")), None);
    let first = page(&backend, filters.clone(), 1, None);
    assert_eq!(ids(&first), [ordinal_id(2)]);
    assert_eq!(first.next_cursor(), Some(cursor(ordinal_id(2))));

    let mut id_01 = create_unit(
        &mut backend,
        ordinal_id(1),
        species("feature"),
        unit_workflow.clone(),
    );
    id_01
        .transition_to(&phase("doing"))
        .expect("below-cursor fixture should enter the filter");
    replace_stored_unit(&database.connect(), &id_01);
    let mut id_03 = create_unit(
        &mut backend,
        ordinal_id(3),
        species("feature"),
        unit_workflow,
    );
    id_03
        .transition_to(&phase("doing"))
        .expect("above-cursor fixture should enter the filter");
    replace_stored_unit(&database.connect(), &id_03);
    id_04
        .transition_to(&phase("done"))
        .expect("above-cursor fixture should leave the filter");
    replace_stored_unit(&database.connect(), &id_04);

    let second = page(&backend, filters, 10, first.next_cursor());
    assert_eq!(ids(&second), [ordinal_id(3)]);
    assert_eq!(second.next_cursor(), None);
}

fn seed_corruption_page(database: &TestDatabase) -> SqliteBackend {
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    let unit_workflow = workflow("corruption-flow", "queued", "done");
    for ordinal in 1..=3 {
        create_unit(
            &mut backend,
            ordinal_id(ordinal),
            species(if ordinal == 3 { "other" } else { "target" }),
            unit_workflow.clone(),
        );
    }
    backend
}

#[test]
fn test_list_fails_whole_page_for_any_selected_invalid_row() {
    let malformed = TestDatabase::new("query-corrupt-envelope");
    let backend = seed_corruption_page(&malformed);
    malformed
        .connect()
        .execute(
            "UPDATE intent_units SET envelope='{' WHERE id=?1",
            [ordinal_id(2).to_string()],
        )
        .expect("fixture envelope should corrupt");
    let before = stored_rows(&malformed.connect());
    let error = backend
        .list(ListIntentUnits::new(
            ListFilters::new(None, Some(species("target")), None, None),
            PageLimit::new(2).unwrap(),
            None,
        ))
        .expect_err("selected corrupt row must fail the page");
    assert!(matches!(error, BackendError::CorruptEnvelope));
    assert_eq!(stored_rows(&malformed.connect()), before);

    let projection = TestDatabase::new("query-projection-mismatch");
    let backend = seed_corruption_page(&projection);
    projection
        .connect()
        .execute(
            "UPDATE intent_units SET phase='wrong' WHERE id=?1",
            [ordinal_id(2).to_string()],
        )
        .expect("fixture projection should change");
    let error = backend
        .list(ListIntentUnits::new(
            ListFilters::default(),
            PageLimit::new(3).unwrap(),
            None,
        ))
        .expect_err("selected projection mismatch must fail the page");
    assert!(matches!(error, BackendError::ProjectionMismatch));

    let lookahead = TestDatabase::new("query-corrupt-lookahead");
    let backend = seed_corruption_page(&lookahead);
    lookahead
        .connect()
        .execute(
            "UPDATE intent_units SET envelope='{' WHERE id=?1",
            [ordinal_id(3).to_string()],
        )
        .expect("lookahead envelope should corrupt");
    let error = backend
        .list(ListIntentUnits::new(
            ListFilters::default(),
            PageLimit::new(2).unwrap(),
            None,
        ))
        .expect_err("corrupt limit+1 candidate must fail the page");
    assert!(matches!(error, BackendError::CorruptEnvelope));

    let excluded = TestDatabase::new("query-filtered-corruption");
    let backend = seed_corruption_page(&excluded);
    excluded
        .connect()
        .execute(
            "UPDATE intent_units SET envelope='{' WHERE id=?1",
            [ordinal_id(3).to_string()],
        )
        .expect("filtered-out envelope should corrupt");
    let accepted = page(
        &backend,
        ListFilters::new(None, Some(species("target")), None, None),
        10,
        None,
    );
    assert_eq!(ids(&accepted), [ordinal_id(1), ordinal_id(2)]);
}
