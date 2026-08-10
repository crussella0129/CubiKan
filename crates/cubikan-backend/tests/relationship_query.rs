mod common;

use common::{
    StoredRelationshipDefinitionSnapshot, StoredRelationshipSnapshot, StoredRowSnapshot,
    TestDatabase, linear_workflow, numbered_id, stored_relationship_definitions,
    stored_relationships, stored_rows,
};
use cubikan_backend::{
    BackendError, CreateIntentUnit, CreateRelationship, CreateRelationshipDefinition,
    DeleteRelationship, ListRelationships, PageLimit, RelationshipCursor, RelationshipDefinitionId,
    RelationshipDefinitionKey, RelationshipDefinitionVersion, RelationshipDirection,
    RelationshipEndpoint, RelationshipError, RelationshipIdentity, RelationshipPage,
    RelationshipPolicy, RelationshipQueryError, SqliteBackend,
};
use cubikan_core::{IntentSpecies, IntentUnitId};
use rusqlite::params;

fn species(value: &str) -> IntentSpecies {
    IntentSpecies::new(value).expect("fixture species should be valid")
}

fn key(id: &str, version: u64) -> RelationshipDefinitionKey {
    RelationshipDefinitionKey::new(
        RelationshipDefinitionId::new(id).expect("fixture definition ID should be valid"),
        RelationshipDefinitionVersion::new(version)
            .expect("fixture definition version should be valid"),
    )
}

fn create_unit(backend: &mut SqliteBackend, id: IntentUnitId, kind: &str) {
    backend
        .create(CreateIntentUnit::new(
            Some(id),
            species(kind),
            linear_workflow("query-flow", "queued", "done"),
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

fn list_query(
    definition: &RelationshipDefinitionKey,
    source: Option<IntentUnitId>,
    target: Option<IntentUnitId>,
    limit: usize,
    after: Option<RelationshipCursor>,
) -> ListRelationships {
    ListRelationships::new(
        definition.clone(),
        source,
        target,
        PageLimit::new(limit).expect("fixture page limit should be valid"),
        after,
    )
    .expect("fixture relationship query should be valid")
}

fn pairs(page: &RelationshipPage) -> Vec<(IntentUnitId, IntentUnitId)> {
    page.items()
        .iter()
        .map(|view| (view.relationship().source(), view.relationship().target()))
        .collect()
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
        .expect("fixture endpoint envelope should be corrupted");
    assert_eq!(changed, 1);
}

#[test]
fn test_relationship_query_ands_exact_filters_and_orders_direct_edges() {
    let database = TestDatabase::new("relationship-query-filters");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    let ids = (1..=5).map(numbered_id).collect::<Vec<_>>();
    for id in &ids {
        create_unit(&mut backend, *id, "node");
    }

    let definition = key("depends-on", u64::MAX);
    let other_version = key("depends-on", 1);
    create_definition(&mut backend, &definition, None, None);
    create_definition(&mut backend, &other_version, None, None);

    // Deliberately insert out of key order and include paths longer than one
    // hop; list remains an exact-version direct-edge query.
    for (source, target) in [(3, 4), (1, 3), (2, 4), (1, 2), (2, 3)] {
        create_edge(
            &mut backend,
            &definition,
            numbered_id(source),
            numbered_id(target),
        );
    }
    create_edge(&mut backend, &other_version, ids[0], ids[4]);

    let unfiltered = backend
        .list_relationships(list_query(&definition, None, None, 100, None))
        .expect("unfiltered query should succeed");
    assert_eq!(
        pairs(&unfiltered),
        [
            (ids[0], ids[1]),
            (ids[0], ids[2]),
            (ids[1], ids[2]),
            (ids[1], ids[3]),
            (ids[2], ids[3])
        ]
    );
    assert!(unfiltered.next_cursor().is_none());
    assert!(
        unfiltered
            .items()
            .iter()
            .all(|view| { view.relationship().definition() == &definition })
    );

    let source_only = backend
        .list_relationships(list_query(&definition, Some(ids[0]), None, 100, None))
        .expect("source filter should succeed");
    assert_eq!(pairs(&source_only), [(ids[0], ids[1]), (ids[0], ids[2])]);
    assert!(!pairs(&source_only).contains(&(ids[0], ids[3])));

    let target_only = backend
        .list_relationships(list_query(&definition, None, Some(ids[3]), 100, None))
        .expect("target filter should succeed");
    assert_eq!(pairs(&target_only), [(ids[1], ids[3]), (ids[2], ids[3])]);

    let both = backend
        .list_relationships(list_query(
            &definition,
            Some(ids[1]),
            Some(ids[3]),
            100,
            None,
        ))
        .expect("ANDed filters should succeed");
    assert_eq!(pairs(&both), [(ids[1], ids[3])]);

    let near_miss = backend
        .list_relationships(list_query(&definition, Some(ids[4]), None, 100, None))
        .expect("near-miss filter should be an empty success");
    assert!(near_miss.items().is_empty());
}

#[test]
fn test_relationship_query_enforces_bounds_complete_cursor_and_live_pages() {
    assert!(PageLimit::new(0).is_err());
    assert!(PageLimit::new(101).is_err());

    // One source and 101 targets force a real limit-100 lookahead while using
    // a definition version above SQLite's signed INTEGER range.
    let database = TestDatabase::new("relationship-query-101");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    let definition = key("page", u64::MAX);
    let source = numbered_id(1_000);
    create_unit(&mut backend, source, "node");
    create_definition(&mut backend, &definition, None, None);
    for ordinal in 1..=101_u64 {
        let target = numbered_id(ordinal * 2);
        create_unit(&mut backend, target, "node");
        create_edge(&mut backend, &definition, source, target);
    }

    let first_query = list_query(&definition, Some(source), None, 100, None);
    let first = backend
        .list_relationships(first_query.clone())
        .expect("first page should succeed");
    assert_eq!(first.query(), &first_query);
    assert_eq!(first.items().len(), 100);
    assert_eq!(first.items()[0].relationship().target(), numbered_id(2));
    assert_eq!(first.items()[99].relationship().target(), numbered_id(200));
    let first_cursor = first
        .next_cursor()
        .expect("validated lookahead should expose a cursor")
        .clone();
    assert_eq!(first_cursor.relationship().definition(), &definition);
    assert_eq!(first_cursor.relationship().source(), source);
    assert_eq!(first_cursor.relationship().target(), numbered_id(200));

    let second = backend
        .list_relationships(list_query(
            &definition,
            Some(source),
            None,
            100,
            Some(first_cursor.clone()),
        ))
        .expect("terminal page should succeed");
    assert_eq!(second.items().len(), 1);
    assert_eq!(second.items()[0].relationship().target(), numbered_id(202));
    assert!(second.next_cursor().is_none());
    assert!(
        first
            .items()
            .iter()
            .all(|item| !second.items().contains(item))
    );

    let limit_one = backend
        .list_relationships(list_query(&definition, Some(source), None, 1, None))
        .expect("minimum limit should succeed");
    assert_eq!(limit_one.items().len(), 1);
    assert!(limit_one.next_cursor().is_some());

    // A cursor is ordering state, not a requirement that its edge still exist.
    let absent_cursor = RelationshipCursor::new(identity(&definition, source, numbered_id(101)));
    let after_absent = backend
        .list_relationships(list_query(
            &definition,
            Some(source),
            None,
            100,
            Some(absent_cursor.clone()),
        ))
        .expect("same-definition absent cursor should succeed");
    assert_eq!(after_absent.items().len(), 51);
    assert_eq!(
        after_absent.items()[0].relationship().target(),
        numbered_id(102)
    );
    assert!(after_absent.next_cursor().is_none());

    let other_definition = key("other-page", u64::MAX);
    assert_eq!(
        ListRelationships::new(
            other_definition.clone(),
            Some(source),
            None,
            PageLimit::new(1).unwrap(),
            Some(absent_cursor),
        )
        .expect_err("cross-definition cursor should reject before storage"),
        RelationshipQueryError::CursorDefinitionMismatch {
            expected: other_definition,
            actual: definition.clone(),
        }
    );

    // Lookahead is fully endpoint-replay validated before a cursor is emitted.
    let lookahead_database = TestDatabase::new("relationship-query-lookahead");
    let mut lookahead_backend = SqliteBackend::open(lookahead_database.path())
        .expect("lookahead database should initialize");
    let lookahead_definition = key("lookahead", 1);
    let lookahead_source = numbered_id(500);
    let first_target = numbered_id(2);
    let corrupt_target = numbered_id(4);
    for id in [lookahead_source, first_target, corrupt_target] {
        create_unit(&mut lookahead_backend, id, "node");
    }
    create_definition(&mut lookahead_backend, &lookahead_definition, None, None);
    create_edge(
        &mut lookahead_backend,
        &lookahead_definition,
        lookahead_source,
        first_target,
    );
    create_edge(
        &mut lookahead_backend,
        &lookahead_definition,
        lookahead_source,
        corrupt_target,
    );
    corrupt_envelope(&lookahead_database, corrupt_target);
    assert_eq!(
        lookahead_backend
            .list_relationships(list_query(
                &lookahead_definition,
                Some(lookahead_source),
                None,
                1,
                None,
            ))
            .expect_err("corrupt lookahead should fail the whole page"),
        RelationshipError::EndpointCorrupt {
            endpoint: RelationshipEndpoint::Target,
            id: corrupt_target,
            source: BackendError::CorruptEnvelope,
        }
    );

    // Later requests are live: membership after the exclusive cursor can be
    // removed and inserted between pages, while insertion before it stays out.
    let live_database = TestDatabase::new("relationship-query-live");
    let mut live_backend =
        SqliteBackend::open(live_database.path()).expect("live database should initialize");
    let live_definition = key("live", 1);
    let live_source = numbered_id(800);
    create_unit(&mut live_backend, live_source, "node");
    for value in [10, 15, 20, 25, 30, 40] {
        create_unit(&mut live_backend, numbered_id(value), "node");
    }
    create_definition(&mut live_backend, &live_definition, None, None);
    for value in [10, 20, 30, 40] {
        create_edge(
            &mut live_backend,
            &live_definition,
            live_source,
            numbered_id(value),
        );
    }
    let live_first = live_backend
        .list_relationships(list_query(
            &live_definition,
            Some(live_source),
            None,
            2,
            None,
        ))
        .expect("live first page should succeed");
    assert_eq!(
        pairs(&live_first),
        [
            (live_source, numbered_id(10)),
            (live_source, numbered_id(20))
        ]
    );
    let live_cursor = live_first.next_cursor().unwrap().clone();
    live_backend
        .delete_relationship(DeleteRelationship::new(identity(
            &live_definition,
            live_source,
            numbered_id(30),
        )))
        .expect("post-cursor edge should delete");
    create_edge(
        &mut live_backend,
        &live_definition,
        live_source,
        numbered_id(25),
    );
    create_edge(
        &mut live_backend,
        &live_definition,
        live_source,
        numbered_id(15),
    );
    let live_second = live_backend
        .list_relationships(list_query(
            &live_definition,
            Some(live_source),
            None,
            100,
            Some(live_cursor),
        ))
        .expect("live terminal page should succeed");
    assert_eq!(
        pairs(&live_second),
        [
            (live_source, numbered_id(25)),
            (live_source, numbered_id(40))
        ]
    );
    assert!(live_second.next_cursor().is_none());
}

#[test]
fn test_relationship_query_missing_definition_and_absent_filter_are_distinct() {
    let database = TestDatabase::new("relationship-query-missing-inputs");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    let existing = key("existing", 1);
    let missing = key("missing", 1);
    create_definition(&mut backend, &existing, None, None);

    assert_eq!(
        backend
            .list_relationships(list_query(&missing, None, None, 100, None))
            .expect_err("missing required definition should reject"),
        RelationshipError::DefinitionNotFound {
            definition: missing,
        }
    );

    let absent_source = numbered_id(900);
    let absent_target = numbered_id(901);
    for query in [
        list_query(&existing, Some(absent_source), None, 100, None),
        list_query(&existing, None, Some(absent_target), 100, None),
        list_query(
            &existing,
            Some(absent_source),
            Some(absent_target),
            100,
            None,
        ),
    ] {
        let page = backend
            .list_relationships(query.clone())
            .expect("absent optional endpoint filter should be empty, not an error");
        assert_eq!(page.query(), &query);
        assert!(page.items().is_empty());
        assert!(page.next_cursor().is_none());
    }
}

#[test]
fn test_relationship_query_rejects_selected_corruption_without_partial_results() {
    // The exact definition is selected and decoded even if edge selection
    // could otherwise succeed.
    {
        let database = TestDatabase::new("relationship-query-corrupt-definition");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let definition = key("corrupt-definition", 1);
        let source = numbered_id(1);
        let target = numbered_id(2);
        for id in [source, target] {
            create_unit(&mut backend, id, "node");
        }
        create_definition(&mut backend, &definition, None, None);
        create_edge(&mut backend, &definition, source, target);
        let connection = database.connect();
        connection
            .pragma_update(None, "ignore_check_constraints", 1_i64)
            .expect("fixture should bypass definition CHECK deliberately");
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
                .list_relationships(list_query(&definition, None, None, 100, None))
                .expect_err("corrupt selected definition should reject"),
            RelationshipError::CorruptDefinition {
                definition: definition.clone(),
            }
        );
        assert_eq!(snapshot(&database), before);
    }

    // A malformed edge in the limit-plus-one position fails the complete page
    // before returning the valid first candidate or a cursor.
    {
        let database = TestDatabase::new("relationship-query-corrupt-edge-lookahead");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let definition = key("corrupt-edge", 1);
        let source = numbered_id(1);
        let first_target = numbered_id(2);
        let second_target = numbered_id(4);
        for id in [source, first_target, second_target] {
            create_unit(&mut backend, id, "node");
        }
        create_definition(&mut backend, &definition, None, None);
        create_edge(&mut backend, &definition, source, first_target);
        create_edge(&mut backend, &definition, source, second_target);
        let connection = database.connect();
        connection
            .pragma_update(None, "foreign_keys", 0_i64)
            .expect("fixture should disable FK checks on its raw connection");
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
                    second_target.to_string(),
                ],
            )
            .expect("fixture relationship should be corrupted");
        assert_eq!(changed, 1);
        drop(connection);
        let before = snapshot(&database);
        assert_eq!(
            backend
                .list_relationships(list_query(&definition, Some(source), None, 1, None,))
                .expect_err("malformed edge lookahead should reject whole page"),
            RelationshipError::CorruptRelationship {
                definition: definition.clone(),
            }
        );
        assert_eq!(snapshot(&database), before);
    }

    // Replay and endpoint-species validation apply to both roles on every
    // selected edge.
    {
        let database = TestDatabase::new("relationship-query-corrupt-source");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let definition = key("corrupt-source", 1);
        let source = numbered_id(1);
        let target = numbered_id(2);
        for id in [source, target] {
            create_unit(&mut backend, id, "node");
        }
        create_definition(&mut backend, &definition, Some("node"), Some("node"));
        create_edge(&mut backend, &definition, source, target);
        corrupt_envelope(&database, source);
        let before = snapshot(&database);
        assert_eq!(
            backend
                .list_relationships(list_query(&definition, None, None, 100, None))
                .expect_err("corrupt selected source should reject"),
            RelationshipError::EndpointCorrupt {
                endpoint: RelationshipEndpoint::Source,
                id: source,
                source: BackendError::CorruptEnvelope,
            }
        );
        assert_eq!(snapshot(&database), before);
    }
    {
        let database = TestDatabase::new("relationship-query-corrupt-target");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let definition = key("corrupt-target", 1);
        let source = numbered_id(1);
        let target = numbered_id(2);
        for id in [source, target] {
            create_unit(&mut backend, id, "node");
        }
        create_definition(&mut backend, &definition, Some("node"), Some("node"));
        create_edge(&mut backend, &definition, source, target);
        corrupt_envelope(&database, target);
        let before = snapshot(&database);
        assert_eq!(
            backend
                .list_relationships(list_query(&definition, None, None, 100, None))
                .expect_err("corrupt selected target should reject"),
            RelationshipError::EndpointCorrupt {
                endpoint: RelationshipEndpoint::Target,
                id: target,
                source: BackendError::CorruptEnvelope,
            }
        );
        assert_eq!(snapshot(&database), before);
    }
    {
        let database = TestDatabase::new("relationship-query-species");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let definition = key("species", 1);
        let source = numbered_id(1);
        let target = numbered_id(2);
        for id in [source, target] {
            create_unit(&mut backend, id, "node");
        }
        create_definition(&mut backend, &definition, Some("node"), Some("node"));
        create_edge(&mut backend, &definition, source, target);
        let changed = database
            .connect()
            .execute(
                "UPDATE relationship_definitions SET source_species='other'
                 WHERE definition_id=?1 COLLATE BINARY AND definition_version=?2",
                params![
                    definition.id().as_str(),
                    definition.version().value().to_be_bytes(),
                ],
            )
            .expect("fixture definition species should change");
        assert_eq!(changed, 1);
        let before = snapshot(&database);
        assert_eq!(
            backend
                .list_relationships(list_query(&definition, None, None, 100, None))
                .expect_err("selected endpoint species mismatch should reject"),
            RelationshipError::EndpointSpeciesMismatch {
                endpoint: RelationshipEndpoint::Source,
                id: source,
                expected: species("other"),
                actual: species("node"),
            }
        );
        assert_eq!(snapshot(&database), before);
    }

    // Corruption behind a SQL filter is outside this query's selection; the
    // same state fails once a later query actually selects it.
    {
        let database = TestDatabase::new("relationship-query-filtered-corruption");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let definition = key("filtered", 1);
        let source = numbered_id(1);
        let target = numbered_id(2);
        let corrupt_source = numbered_id(3);
        let corrupt_target = numbered_id(4);
        for id in [source, target, corrupt_source, corrupt_target] {
            create_unit(&mut backend, id, "node");
        }
        create_definition(&mut backend, &definition, None, None);
        create_edge(&mut backend, &definition, source, target);
        create_edge(&mut backend, &definition, corrupt_source, corrupt_target);
        corrupt_envelope(&database, corrupt_target);
        let before = snapshot(&database);

        let selected = backend
            .list_relationships(list_query(&definition, Some(source), None, 100, None))
            .expect("filtered-out corruption should not be globally scanned");
        assert_eq!(pairs(&selected), [(source, target)]);
        assert_eq!(snapshot(&database), before);

        assert_eq!(
            backend
                .list_relationships(list_query(
                    &definition,
                    Some(corrupt_source),
                    None,
                    100,
                    None,
                ))
                .expect_err("selected corruption should reject"),
            RelationshipError::EndpointCorrupt {
                endpoint: RelationshipEndpoint::Target,
                id: corrupt_target,
                source: BackendError::CorruptEnvelope,
            }
        );
        assert_eq!(snapshot(&database), before);
    }
}
