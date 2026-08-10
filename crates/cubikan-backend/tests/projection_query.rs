mod common;

use common::{
    StoredRelationshipDefinitionSnapshot, StoredRelationshipSnapshot, StoredRowSnapshot,
    TestDatabase, initialize_exact_v1, linear_workflow, numbered_id,
    stored_relationship_definitions, stored_relationships, stored_rows,
};
use cubikan_backend::{
    BackendError, BackendSchemaVersion, CreateIntentUnit, CreateRelationship,
    CreateRelationshipDefinition, DeleteRelationship, DirectRelationshipPredicate, ListCursor,
    ListFilters, ListIntentUnits, PageLimit, ProjectionPage, ProjectionQueryV1,
    RelationshipDefinitionId, RelationshipDefinitionKey, RelationshipDefinitionVersion,
    RelationshipDirection, RelationshipEndpoint, RelationshipError, RelationshipIdentity,
    RelationshipPolicy, SqliteBackend, TransitionIntentUnit,
};
use cubikan_core::{
    IntentSpecies, IntentUnitId, IntentUnitRevision, IntentUnitStatus, PhaseId, WorkflowId,
};
use rusqlite::params;

fn species(value: &str) -> IntentSpecies {
    IntentSpecies::new(value).expect("fixture species should be valid")
}

fn workflow_id(value: &str) -> WorkflowId {
    WorkflowId::new(value).expect("fixture workflow ID should be valid")
}

fn phase(value: &str) -> PhaseId {
    PhaseId::new(value).expect("fixture phase should be valid")
}

fn key(id: &str, version: u64) -> RelationshipDefinitionKey {
    RelationshipDefinitionKey::new(
        RelationshipDefinitionId::new(id).expect("fixture definition ID should be valid"),
        RelationshipDefinitionVersion::new(version)
            .expect("fixture definition version should be valid"),
    )
}

fn create_unit(backend: &mut SqliteBackend, id: IntentUnitId, kind: &str, workflow: &str) {
    backend
        .create(CreateIntentUnit::new(
            Some(id),
            species(kind),
            linear_workflow(workflow, "queued", "done"),
        ))
        .expect("fixture unit should create");
}

fn create_definition(
    backend: &mut SqliteBackend,
    definition: &RelationshipDefinitionKey,
    source_species: Option<&str>,
    target_species: Option<&str>,
) {
    backend
        .create_relationship_definition(CreateRelationshipDefinition::new(
            definition.clone(),
            RelationshipDirection::Directed,
            source_species.map(species),
            target_species.map(species),
            RelationshipPolicy::Allow,
            RelationshipPolicy::Allow,
        ))
        .expect("fixture definition should create");
}

fn identity(
    definition: &RelationshipDefinitionKey,
    source: IntentUnitId,
    target: IntentUnitId,
) -> RelationshipIdentity {
    RelationshipIdentity::new(definition.clone(), source, target)
}

fn create_edge(
    backend: &mut SqliteBackend,
    definition: &RelationshipDefinitionKey,
    source: IntentUnitId,
    target: IntentUnitId,
) {
    backend
        .create_relationship(CreateRelationship::new(identity(
            definition, source, target,
        )))
        .expect("fixture relationship should create");
}

fn cursor(id: IntentUnitId) -> ListCursor {
    id.to_string()
        .parse()
        .expect("fixture projection cursor should be canonical")
}

fn projection(
    filters: ListFilters,
    predicate: Option<DirectRelationshipPredicate>,
    limit: usize,
    after: Option<ListCursor>,
) -> ProjectionQueryV1 {
    ProjectionQueryV1::new(
        filters,
        predicate,
        PageLimit::new(limit).expect("fixture projection limit should be valid"),
        after,
    )
}

fn outgoing(
    definition: &RelationshipDefinitionKey,
    anchor: IntentUnitId,
) -> DirectRelationshipPredicate {
    DirectRelationshipPredicate::Outgoing {
        definition: definition.clone(),
        anchor,
    }
}

fn incoming(
    definition: &RelationshipDefinitionKey,
    anchor: IntentUnitId,
) -> DirectRelationshipPredicate {
    DirectRelationshipPredicate::Incoming {
        definition: definition.clone(),
        anchor,
    }
}

fn ids(page: &ProjectionPage) -> Vec<IntentUnitId> {
    page.items().iter().map(|item| item.id()).collect()
}

#[derive(Debug, Eq, PartialEq)]
struct DurableSnapshot {
    units: Vec<StoredRowSnapshot>,
    definitions: Vec<StoredRelationshipDefinitionSnapshot>,
    relationships: Vec<StoredRelationshipSnapshot>,
}

fn snapshot(database: &TestDatabase) -> DurableSnapshot {
    let connection = database.connect();
    DurableSnapshot {
        units: stored_rows(&connection),
        definitions: stored_relationship_definitions(&connection),
        relationships: stored_relationships(&connection),
    }
}

fn corrupt_envelope(database: &TestDatabase, id: IntentUnitId) {
    let changed = database
        .connect()
        .execute(
            "UPDATE intent_units SET envelope='{not-json' WHERE id=?1",
            [id.to_string()],
        )
        .expect("fixture envelope should be corrupted");
    assert_eq!(changed, 1);
}

#[test]
fn test_projection_v1_ands_lifecycle_filters_with_direct_predicate() {
    let database = TestDatabase::new("projection-filter-directions");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    let definition = key("portfolio", u64::MAX);
    let anchor = numbered_id(1_000);
    let matching = numbered_id(10);
    let wrong_species = numbered_id(20);
    let wrong_workflow = numbered_id(30);
    let wrong_phase = numbered_id(40);
    let wrong_status = numbered_id(50);
    let transitive = numbered_id(60);
    let inbound = numbered_id(70);

    for (id, kind, workflow) in [
        (anchor, "anchor", "flow-a"),
        (matching, "feature", "flow-a"),
        (wrong_species, "defect", "flow-a"),
        (wrong_workflow, "feature", "flow-b"),
        (wrong_phase, "feature", "flow-a"),
        (wrong_status, "feature", "flow-a"),
        (transitive, "feature", "flow-a"),
        (inbound, "source", "flow-a"),
    ] {
        create_unit(&mut backend, id, kind, workflow);
    }
    backend
        .transition(TransitionIntentUnit::new(
            wrong_phase,
            phase("done"),
            IntentUnitRevision::INITIAL,
        ))
        .expect("phase fixture should transition");
    let completed_revision = backend
        .transition(TransitionIntentUnit::new(
            wrong_status,
            phase("done"),
            IntentUnitRevision::INITIAL,
        ))
        .expect("completion fixture should transition")
        .committed_revision();
    backend
        .complete(cubikan_backend::CompleteIntentUnit::new(
            wrong_status,
            completed_revision,
        ))
        .expect("completion fixture should complete");

    create_definition(&mut backend, &definition, None, None);
    for target in [
        matching,
        wrong_species,
        wrong_workflow,
        wrong_phase,
        wrong_status,
    ] {
        create_edge(&mut backend, &definition, anchor, target);
    }
    create_edge(&mut backend, &definition, matching, transitive);
    create_edge(&mut backend, &definition, inbound, matching);

    let all_filters = ListFilters::new(
        Some(workflow_id("flow-a")),
        Some(species("feature")),
        Some(phase("queued")),
        Some(IntentUnitStatus::Active),
    );
    let lifecycle_query = projection(all_filters.clone(), None, 100, None);
    let lifecycle_page = backend
        .project(lifecycle_query.clone())
        .expect("predicate-free projection should succeed");
    let list_page = backend
        .list(ListIntentUnits::new(
            all_filters.clone(),
            PageLimit::new(100).unwrap(),
            None,
        ))
        .expect("equivalent lifecycle query should succeed");
    assert_eq!(lifecycle_page.query(), &lifecycle_query);
    assert_eq!(lifecycle_page.items(), list_page.items());
    assert_eq!(lifecycle_page.next_cursor(), list_page.next_cursor());

    let outgoing_query = projection(all_filters, Some(outgoing(&definition, anchor)), 100, None);
    let outgoing_page = backend
        .project(outgoing_query.clone())
        .expect("ANDed outgoing projection should succeed");
    assert_eq!(outgoing_page.query(), &outgoing_query);
    assert_eq!(ids(&outgoing_page), [matching]);
    assert!(!ids(&outgoing_page).contains(&transitive));

    let incoming_page = backend
        .project(projection(
            ListFilters::default(),
            Some(incoming(&definition, matching)),
            100,
            None,
        ))
        .expect("incoming projection should succeed");
    assert_eq!(ids(&incoming_page), [inbound, anchor]);
    assert!(!ids(&incoming_page).contains(&transitive));
}

#[test]
fn test_unit_appears_in_multiple_live_projections_without_copied_state() {
    let database = TestDatabase::new("projection-multiple-live-views");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    let first_definition = key("first-view", 1);
    let second_definition = key("second-view", 9);
    let first_anchor = numbered_id(100);
    let second_anchor = numbered_id(200);
    let target = numbered_id(300);
    for (id, kind) in [
        (first_anchor, "anchor"),
        (second_anchor, "anchor"),
        (target, "feature"),
    ] {
        create_unit(&mut backend, id, kind, "live-flow");
    }
    create_definition(
        &mut backend,
        &first_definition,
        Some("anchor"),
        Some("feature"),
    );
    create_definition(
        &mut backend,
        &second_definition,
        Some("anchor"),
        Some("feature"),
    );
    create_edge(&mut backend, &first_definition, first_anchor, target);
    create_edge(&mut backend, &second_definition, second_anchor, target);

    assert_eq!(
        backend.get(target).unwrap().revision(),
        IntentUnitRevision::INITIAL,
        "relationship creation must not revise the endpoint"
    );
    let before_queries = snapshot(&database);
    let first_query = projection(
        ListFilters::default(),
        Some(outgoing(&first_definition, first_anchor)),
        100,
        None,
    );
    let second_query = projection(
        ListFilters::default(),
        Some(outgoing(&second_definition, second_anchor)),
        100,
        None,
    );
    let first_page = backend.project(first_query.clone()).unwrap();
    let second_page = backend.project(second_query.clone()).unwrap();
    assert_eq!(first_page.items(), second_page.items());
    assert_eq!(ids(&first_page), [target]);
    assert_eq!(first_page.query(), &first_query);
    assert_eq!(second_page.query(), &second_query);
    assert_eq!(snapshot(&database), before_queries);

    let stored_projection_objects: i64 = database
        .connect()
        .query_row(
            "SELECT count(*) FROM sqlite_schema
             WHERE lower(name) LIKE '%projection%' OR lower(name) LIKE '%board%'",
            [],
            |row| row.get(0),
        )
        .expect("schema inventory should be readable");
    assert_eq!(stored_projection_objects, 0);

    backend
        .delete_relationship(DeleteRelationship::new(identity(
            &first_definition,
            first_anchor,
            target,
        )))
        .expect("one view edge should delete");
    assert!(backend.project(first_query).unwrap().items().is_empty());
    assert_eq!(
        ids(&backend.project(second_query.clone()).unwrap()),
        [target]
    );
    assert_eq!(
        backend.get(target).unwrap().revision(),
        IntentUnitRevision::INITIAL,
        "relationship deletion must not revise the endpoint"
    );

    let queued_query = projection(
        ListFilters::new(None, None, Some(phase("queued")), None),
        Some(outgoing(&second_definition, second_anchor)),
        100,
        None,
    );
    assert_eq!(
        ids(&backend.project(queued_query.clone()).unwrap()),
        [target]
    );
    backend
        .transition(TransitionIntentUnit::new(
            target,
            phase("done"),
            IntentUnitRevision::INITIAL,
        ))
        .expect("canonical lifecycle state should change");
    assert!(backend.project(queued_query).unwrap().items().is_empty());
    let done_page = backend
        .project(projection(
            ListFilters::new(None, None, Some(phase("done")), None),
            Some(outgoing(&second_definition, second_anchor)),
            100,
            None,
        ))
        .expect("later projection should observe the lifecycle change");
    assert_eq!(ids(&done_page), [target]);
    assert_eq!(done_page.items()[0].revision().value(), 1);
    assert_eq!(stored_relationships(&database.connect()).len(), 1);
}

#[test]
fn test_projection_v1_reports_query_and_uses_exclusive_live_pages() {
    assert!(PageLimit::new(0).is_err());
    assert!(PageLimit::new(101).is_err());

    let database = TestDatabase::new("projection-page-101");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    let definition = key("page-view", u64::MAX);
    let anchor = numbered_id(1_000);
    create_unit(&mut backend, anchor, "anchor", "page-flow");
    create_definition(&mut backend, &definition, Some("anchor"), Some("feature"));
    for ordinal in 1..=101_u64 {
        let target = numbered_id(ordinal);
        create_unit(&mut backend, target, "feature", "page-flow");
        create_edge(&mut backend, &definition, anchor, target);
    }

    let first_query = projection(
        ListFilters::default(),
        Some(outgoing(&definition, anchor)),
        100,
        None,
    );
    let first = backend
        .project(first_query.clone())
        .expect("first page should succeed");
    let repeated = backend
        .project(first_query.clone())
        .expect("unchanged projection should reproduce");
    assert_eq!(first, repeated);
    assert_eq!(first.query(), &first_query);
    assert_eq!(first.query().version(), ProjectionQueryV1::VERSION);
    assert_eq!(first.items().len(), 100);
    assert_eq!(first.items()[0].id(), numbered_id(1));
    assert_eq!(first.items()[99].id(), numbered_id(100));
    assert_eq!(
        first.next_cursor().map(ListCursor::id),
        Some(numbered_id(100))
    );

    let terminal_query = projection(
        ListFilters::default(),
        Some(outgoing(&definition, anchor)),
        100,
        first.next_cursor(),
    );
    let terminal = backend
        .project(terminal_query.clone())
        .expect("terminal page should succeed");
    assert_eq!(terminal.query(), &terminal_query);
    assert_eq!(ids(&terminal), [numbered_id(101)]);
    assert!(terminal.next_cursor().is_none());
    assert!(
        first
            .items()
            .iter()
            .all(|item| !terminal.items().contains(item))
    );

    let minimum = backend
        .project(projection(
            ListFilters::default(),
            Some(outgoing(&definition, anchor)),
            1,
            None,
        ))
        .expect("minimum page should succeed");
    assert_eq!(minimum.items().len(), 1);
    assert_eq!(
        minimum.next_cursor().map(ListCursor::id),
        Some(numbered_id(1))
    );

    let live_database = TestDatabase::new("projection-live-pages");
    let mut live_backend =
        SqliteBackend::open(live_database.path()).expect("live database should initialize");
    let live_definition = key("live-page", 1);
    let live_anchor = numbered_id(800);
    create_unit(&mut live_backend, live_anchor, "anchor", "live-page-flow");
    for value in [10, 15, 20, 25, 30, 40] {
        create_unit(
            &mut live_backend,
            numbered_id(value),
            "feature",
            "live-page-flow",
        );
    }
    create_definition(
        &mut live_backend,
        &live_definition,
        Some("anchor"),
        Some("feature"),
    );
    for value in [10, 20, 30, 40] {
        create_edge(
            &mut live_backend,
            &live_definition,
            live_anchor,
            numbered_id(value),
        );
    }
    let live_first = live_backend
        .project(projection(
            ListFilters::default(),
            Some(outgoing(&live_definition, live_anchor)),
            2,
            None,
        ))
        .expect("live first page should succeed");
    assert_eq!(ids(&live_first), [numbered_id(10), numbered_id(20)]);
    let live_cursor = live_first.next_cursor().expect("lookahead should exist");
    live_backend
        .delete_relationship(DeleteRelationship::new(identity(
            &live_definition,
            live_anchor,
            numbered_id(30),
        )))
        .expect("post-cursor member should delete");
    create_edge(
        &mut live_backend,
        &live_definition,
        live_anchor,
        numbered_id(25),
    );
    create_edge(
        &mut live_backend,
        &live_definition,
        live_anchor,
        numbered_id(15),
    );
    let live_second = live_backend
        .project(projection(
            ListFilters::default(),
            Some(outgoing(&live_definition, live_anchor)),
            100,
            Some(live_cursor),
        ))
        .expect("later page should use current committed membership");
    assert_eq!(ids(&live_second), [numbered_id(25), numbered_id(40)]);
    assert!(live_second.next_cursor().is_none());

    let absent_cursor_page = live_backend
        .project(projection(
            ListFilters::default(),
            Some(outgoing(&live_definition, live_anchor)),
            100,
            Some(cursor(numbered_id(22))),
        ))
        .expect("cursor ordering state need not name a current unit");
    assert_eq!(ids(&absent_cursor_page), [numbered_id(25), numbered_id(40)]);
}

#[test]
fn test_projection_v1_missing_and_corrupt_inputs_fail_whole_page() {
    // The schema guard applies before both relationship and predicate-free
    // projection paths.
    {
        let database = TestDatabase::new("projection-v1-guard");
        initialize_exact_v1(&database.connect());
        let backend = SqliteBackend::open(database.path()).expect("exact v1 should open");
        assert_eq!(
            backend
                .project(projection(ListFilters::default(), None, 100, None))
                .expect_err("projection should require exact v2"),
            RelationshipError::MigrationRequired {
                found: BackendSchemaVersion::V1,
                required: BackendSchemaVersion::V2,
            }
        );
    }

    // Definition lookup precedes typed anchor replay; anchor failures preserve
    // the direction-specific endpoint role.
    {
        let database = TestDatabase::new("projection-missing-inputs");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let existing = key("existing-view", 1);
        let missing = key("missing-view", 1);
        let present_anchor = numbered_id(1);
        let missing_anchor = numbered_id(2);
        create_unit(&mut backend, present_anchor, "anchor", "missing-flow");
        create_definition(&mut backend, &existing, None, None);
        assert_eq!(
            backend
                .project(projection(
                    ListFilters::default(),
                    Some(outgoing(&missing, present_anchor)),
                    100,
                    None,
                ))
                .expect_err("missing definition should reject first"),
            RelationshipError::DefinitionNotFound {
                definition: missing,
            }
        );
        assert_eq!(
            backend
                .project(projection(
                    ListFilters::default(),
                    Some(outgoing(&existing, missing_anchor)),
                    100,
                    None,
                ))
                .expect_err("missing outgoing anchor should reject"),
            RelationshipError::EndpointNotFound {
                endpoint: RelationshipEndpoint::Source,
                id: missing_anchor,
            }
        );
        assert_eq!(
            backend
                .project(projection(
                    ListFilters::default(),
                    Some(incoming(&existing, missing_anchor)),
                    100,
                    None,
                ))
                .expect_err("missing incoming anchor should reject"),
            RelationshipError::EndpointNotFound {
                endpoint: RelationshipEndpoint::Target,
                id: missing_anchor,
            }
        );
    }

    // A selected definition with an intact key but corrupt value rejects
    // without repair.
    {
        let database = TestDatabase::new("projection-corrupt-definition");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let definition = key("corrupt-view", 1);
        let source = numbered_id(1);
        let target = numbered_id(2);
        create_unit(&mut backend, source, "anchor", "corrupt-flow");
        create_unit(&mut backend, target, "feature", "corrupt-flow");
        create_definition(&mut backend, &definition, None, None);
        create_edge(&mut backend, &definition, source, target);
        let connection = database.connect();
        connection
            .pragma_update(None, "ignore_check_constraints", 1_i64)
            .expect("fixture should bypass definition CHECK");
        let changed = connection
            .execute(
                "UPDATE relationship_definitions SET cycle_policy='sometimes'
                 WHERE definition_id=?1 COLLATE BINARY AND definition_version=?2",
                params![
                    definition.id().as_str(),
                    definition.version().value().to_be_bytes(),
                ],
            )
            .expect("fixture definition should be corrupted");
        assert_eq!(changed, 1);
        drop(connection);
        let before = snapshot(&database);
        assert_eq!(
            backend
                .project(projection(
                    ListFilters::default(),
                    Some(outgoing(&definition, source)),
                    100,
                    None,
                ))
                .expect_err("corrupt selected definition should reject"),
            RelationshipError::CorruptDefinition {
                definition: definition.clone(),
            }
        );
        assert_eq!(snapshot(&database), before);
    }

    // A malformed edge in the one-row lookahead fails the entire page before
    // the valid first item or cursor can escape.
    {
        let database = TestDatabase::new("projection-corrupt-edge-lookahead");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let definition = key("edge-view", 1);
        let source = numbered_id(100);
        let first_target = numbered_id(2);
        let corrupt_target = numbered_id(4);
        for id in [source, first_target, corrupt_target] {
            create_unit(&mut backend, id, "node", "edge-flow");
        }
        create_definition(&mut backend, &definition, None, None);
        create_edge(&mut backend, &definition, source, first_target);
        create_edge(&mut backend, &definition, source, corrupt_target);
        let connection = database.connect();
        connection
            .pragma_update(None, "foreign_keys", 0_i64)
            .expect("fixture should disable FK checks");
        let changed = connection
            .execute(
                "UPDATE intent_unit_relationships SET target_id='not-a-uuid'
                 WHERE definition_id=?1 COLLATE BINARY
                   AND definition_version=?2
                   AND source_id=?3 COLLATE BINARY
                   AND target_id=?4 COLLATE BINARY",
                params![
                    definition.id().as_str(),
                    definition.version().value().to_be_bytes(),
                    source.to_string(),
                    corrupt_target.to_string(),
                ],
            )
            .expect("fixture relationship should be corrupted");
        assert_eq!(changed, 1);
        drop(connection);
        let before = snapshot(&database);
        assert_eq!(
            backend
                .project(projection(
                    ListFilters::default(),
                    Some(outgoing(&definition, source)),
                    1,
                    Some(cursor(numbered_id(1))),
                ))
                .expect_err("corrupt lookahead edge should reject whole page"),
            RelationshipError::CorruptRelationship {
                definition: definition.clone(),
            }
        );
        assert_eq!(snapshot(&database), before);
    }

    // Anchor replay, candidate replay, lifecycle SQL selection, and the
    // predicate-free wrapper retain their distinct typed error boundaries.
    {
        let database = TestDatabase::new("projection-corrupt-endpoints");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let definition = key("endpoint-view", 1);
        let anchor = numbered_id(100);
        let first_target = numbered_id(2);
        let corrupt_target = numbered_id(4);
        for id in [anchor, first_target, corrupt_target] {
            create_unit(&mut backend, id, "node", "endpoint-flow");
        }
        create_definition(&mut backend, &definition, Some("node"), Some("node"));
        create_edge(&mut backend, &definition, anchor, first_target);
        create_edge(&mut backend, &definition, anchor, corrupt_target);
        let changed = database
            .connect()
            .execute(
                "UPDATE intent_units SET envelope='{not-json', phase='done' WHERE id=?1",
                [corrupt_target.to_string()],
            )
            .expect("fixture candidate should be corrupted");
        assert_eq!(changed, 1);
        let before = snapshot(&database);

        let filtered = backend
            .project(projection(
                ListFilters::new(None, None, Some(phase("queued")), None),
                Some(outgoing(&definition, anchor)),
                1,
                None,
            ))
            .expect("SQL-excluded corruption is outside the selected window");
        assert_eq!(ids(&filtered), [first_target]);
        assert!(filtered.next_cursor().is_none());

        assert_eq!(
            backend
                .project(projection(
                    ListFilters::default(),
                    Some(outgoing(&definition, anchor)),
                    1,
                    None,
                ))
                .expect_err("corrupt candidate lookahead should reject whole page"),
            RelationshipError::EndpointCorrupt {
                endpoint: RelationshipEndpoint::Target,
                id: corrupt_target,
                source: BackendError::CorruptEnvelope,
            }
        );
        assert_eq!(
            backend
                .project(projection(ListFilters::default(), None, 100, None))
                .expect_err("predicate-free corruption should retain lifecycle error"),
            RelationshipError::Backend(BackendError::CorruptEnvelope)
        );
        assert_eq!(snapshot(&database), before);

        corrupt_envelope(&database, anchor);
        assert_eq!(
            backend
                .project(projection(
                    ListFilters::default(),
                    Some(outgoing(&definition, anchor)),
                    100,
                    None,
                ))
                .expect_err("corrupt outgoing anchor should reject before candidates"),
            RelationshipError::EndpointCorrupt {
                endpoint: RelationshipEndpoint::Source,
                id: anchor,
                source: BackendError::CorruptEnvelope,
            }
        );
    }
}
