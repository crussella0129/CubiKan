mod common;

use common::{
    StoredRelationshipDefinitionSnapshot, StoredRelationshipSnapshot, StoredRowSnapshot,
    TestDatabase, initialize_exact_v1, linear_workflow, numbered_id, phase,
    stored_relationship_definitions, stored_relationships, stored_rows,
};
use cubikan_backend::{
    BackendSchemaVersion, CompleteIntentUnit, CreateIntentUnit, CreateRelationship,
    CreateRelationshipDefinition, DeleteRelationship, DirectRelationshipPredicate,
    IntentUnitSummary, IntentUnitView, ListFilters, ListIntentUnits, ListRelationships, PageLimit,
    ProjectionPage, ProjectionQueryV1, RelationshipDefinitionId, RelationshipDefinitionKey,
    RelationshipDefinitionVersion, RelationshipDefinitionView, RelationshipDirection,
    RelationshipError, RelationshipIdentity, RelationshipPage, RelationshipPolicy,
    RelationshipView, SqliteBackend, TransitionIntentUnit,
};
use cubikan_core::{IntentSpecies, IntentUnitId, IntentUnitRevision, IntentUnitStatus};

fn species(value: &str) -> IntentSpecies {
    IntentSpecies::new(value).expect("fixture species should be valid")
}

fn definition_key(id: &str) -> RelationshipDefinitionKey {
    RelationshipDefinitionKey::new(
        RelationshipDefinitionId::new(id).expect("fixture definition ID should be valid"),
        RelationshipDefinitionVersion::new(1).expect("fixture definition version should be valid"),
    )
}

fn definition_command(
    key: &RelationshipDefinitionKey,
    source_species: &str,
    target_species: &str,
    cycle_policy: RelationshipPolicy,
) -> CreateRelationshipDefinition {
    CreateRelationshipDefinition::new(
        key.clone(),
        RelationshipDirection::Directed,
        Some(species(source_species)),
        Some(species(target_species)),
        RelationshipPolicy::Reject,
        cycle_policy,
    )
}

fn definition_view(command: &CreateRelationshipDefinition) -> RelationshipDefinitionView {
    RelationshipDefinitionView::new(
        command.key().clone(),
        command.direction(),
        command.source_species().cloned(),
        command.target_species().cloned(),
        command.self_policy(),
        command.cycle_policy(),
    )
}

fn identity(
    definition: &RelationshipDefinitionKey,
    source: IntentUnitId,
    target: IntentUnitId,
) -> RelationshipIdentity {
    RelationshipIdentity::new(definition.clone(), source, target)
}

fn relationship_view(
    definition: &RelationshipDefinitionKey,
    source: IntentUnitId,
    target: IntentUnitId,
) -> RelationshipView {
    RelationshipView::new(identity(definition, source, target))
}

fn relationship_query(definition: &RelationshipDefinitionKey) -> ListRelationships {
    ListRelationships::new(
        definition.clone(),
        None,
        None,
        PageLimit::new(100).expect("fixture relationship limit should be valid"),
        None,
    )
    .expect("fixture relationship query should be valid")
}

fn relationship_pairs(page: &RelationshipPage) -> Vec<(IntentUnitId, IntentUnitId)> {
    page.items()
        .iter()
        .map(|view| (view.relationship().source(), view.relationship().target()))
        .collect()
}

fn projection_query(
    filters: ListFilters,
    definition: &RelationshipDefinitionKey,
    anchor: IntentUnitId,
    incoming: bool,
) -> ProjectionQueryV1 {
    let predicate = if incoming {
        DirectRelationshipPredicate::Incoming {
            definition: definition.clone(),
            anchor,
        }
    } else {
        DirectRelationshipPredicate::Outgoing {
            definition: definition.clone(),
            anchor,
        }
    };
    ProjectionQueryV1::new(
        filters,
        Some(predicate),
        PageLimit::new(100).expect("fixture projection limit should be valid"),
        None,
    )
}

fn projection_ids(page: &ProjectionPage) -> Vec<IntentUnitId> {
    page.items().iter().map(|item| item.id()).collect()
}

fn lifecycle_filters(
    exemplar: &IntentUnitView,
    phase_name: &str,
    status: IntentUnitStatus,
) -> ListFilters {
    ListFilters::new(
        Some(exemplar.workflow_id().clone()),
        Some(exemplar.species().clone()),
        Some(phase(phase_name)),
        Some(status),
    )
}

fn create_unit(
    backend: &mut SqliteBackend,
    id: IntentUnitId,
    unit_species: &str,
    workflow_id: &str,
) -> IntentUnitView {
    backend
        .create(CreateIntentUnit::new(
            Some(id),
            species(unit_species),
            linear_workflow(workflow_id, "queued", "done"),
        ))
        .expect("fixture Intent Unit should create")
}

fn assert_all_relationship_apis_require_migration(
    backend: &mut SqliteBackend,
    definition: &CreateRelationshipDefinition,
    relationship: &RelationshipIdentity,
) {
    let expected = RelationshipError::MigrationRequired {
        found: BackendSchemaVersion::V1,
        required: BackendSchemaVersion::V2,
    };
    assert_eq!(
        backend.create_relationship_definition(definition.clone()),
        Err(expected.clone())
    );
    assert_eq!(
        backend.get_relationship_definition(definition.key().clone()),
        Err(expected.clone())
    );
    assert_eq!(
        backend.create_relationship(CreateRelationship::new(relationship.clone())),
        Err(expected.clone())
    );
    assert_eq!(
        backend.delete_relationship(DeleteRelationship::new(relationship.clone())),
        Err(expected.clone())
    );
    assert_eq!(
        backend.list_relationships(relationship_query(definition.key())),
        Err(expected.clone())
    );
    assert_eq!(
        backend.project(projection_query(
            ListFilters::default(),
            definition.key(),
            relationship.source(),
            false,
        )),
        Err(expected)
    );
}

fn preserved_rows(rows: Vec<StoredRowSnapshot>, ids: &[IntentUnitId]) -> Vec<StoredRowSnapshot> {
    rows.into_iter()
        .filter(|row| ids.iter().any(|id| row.id == id.to_string()))
        .collect()
}

#[derive(Debug, Eq, PartialEq)]
struct RawDatabaseSnapshot {
    user_version: i64,
    objects: Vec<(String, String, String, Option<String>)>,
    units: Vec<StoredRowSnapshot>,
    definitions: Vec<StoredRelationshipDefinitionSnapshot>,
    relationships: Vec<StoredRelationshipSnapshot>,
}

fn raw_database_snapshot(database: &TestDatabase) -> RawDatabaseSnapshot {
    let connection = database.connect();
    let user_version = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("fixture schema version should be readable");
    let objects = connection
        .prepare("SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY name")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .expect("fixture schema inventory should be readable");
    let has_relationship_definitions = objects
        .iter()
        .any(|object| object.0 == "table" && object.1 == "relationship_definitions");
    let has_relationships = objects
        .iter()
        .any(|object| object.0 == "table" && object.1 == "intent_unit_relationships");

    RawDatabaseSnapshot {
        user_version,
        objects,
        units: stored_rows(&connection),
        definitions: if has_relationship_definitions {
            stored_relationship_definitions(&connection)
        } else {
            Vec::new()
        },
        relationships: if has_relationships {
            stored_relationships(&connection)
        } else {
            Vec::new()
        },
    }
}

#[test]
fn test_public_backend_relationship_projection_journey_across_reopen() {
    let database = TestDatabase::new("public-relationship-projection-journey");
    let portfolio = numbered_id(300);
    let feature_a = numbered_id(100);
    let feature_b = numbered_id(200);
    let contains = definition_key("contains");
    let depends = definition_key("depends-on");
    let contains_command =
        definition_command(&contains, "portfolio", "feature", RelationshipPolicy::Allow);
    let depends_command =
        definition_command(&depends, "feature", "feature", RelationshipPolicy::Reject);

    let (portfolio_view, feature_a_view, feature_b_view, contains_view, depends_view) = {
        let mut backend =
            SqliteBackend::open(database.path()).expect("fresh database should initialize");
        assert_eq!(backend.schema_version(), BackendSchemaVersion::V2);

        // Deliberately create out of canonical ID order, as well as the
        // containment edges, so the public queries prove durable ordering.
        let portfolio_view = create_unit(&mut backend, portfolio, "portfolio", "delivery");
        let feature_b_view = create_unit(&mut backend, feature_b, "feature", "delivery");
        let feature_a_view = create_unit(&mut backend, feature_a, "feature", "delivery");
        let contains_view = backend
            .create_relationship_definition(contains_command.clone())
            .expect("containment definition should create");
        let depends_view = backend
            .create_relationship_definition(depends_command.clone())
            .expect("dependency definition should create");
        assert_eq!(contains_view, definition_view(&contains_command));
        assert_eq!(depends_view, definition_view(&depends_command));

        for (definition, source, target) in [
            (&contains, portfolio, feature_b),
            (&contains, portfolio, feature_a),
            (&depends, feature_a, feature_b),
        ] {
            assert_eq!(
                backend
                    .create_relationship(CreateRelationship::new(identity(
                        definition, source, target,
                    )))
                    .expect("valid relationship should create"),
                relationship_view(definition, source, target)
            );
        }

        let before_cycle = backend
            .list_relationships(relationship_query(&depends))
            .expect("dependency relationships should list");
        let rejected_cycle = identity(&depends, feature_b, feature_a);
        assert_eq!(
            backend.create_relationship(CreateRelationship::new(rejected_cycle.clone())),
            Err(RelationshipError::CycleRejected {
                relationship: rejected_cycle,
            })
        );
        assert_eq!(
            backend
                .list_relationships(relationship_query(&depends))
                .expect("rejected cycle should leave relationships queryable"),
            before_cycle,
            "a rejected cycle must not mutate the accepted edge set"
        );

        (
            portfolio_view,
            feature_a_view,
            feature_b_view,
            contains_view,
            depends_view,
        )
    };

    let feature_b_transitioned = {
        let mut backend = SqliteBackend::open(database.path()).expect("database should reopen");
        assert_eq!(
            backend
                .get_relationship_definition(contains.clone())
                .expect("containment definition should reopen"),
            contains_view
        );
        assert_eq!(
            backend
                .get_relationship_definition(depends.clone())
                .expect("dependency definition should reopen"),
            depends_view
        );
        assert_eq!(
            relationship_pairs(
                &backend
                    .list_relationships(relationship_query(&contains))
                    .expect("containment relationships should list")
            ),
            [(portfolio, feature_a), (portfolio, feature_b)]
        );
        assert_eq!(
            relationship_pairs(
                &backend
                    .list_relationships(relationship_query(&depends))
                    .expect("dependency relationships should list")
            ),
            [(feature_a, feature_b)]
        );

        let queued = lifecycle_filters(&feature_a_view, "queued", IntentUnitStatus::Active);
        assert_eq!(
            projection_ids(
                &backend
                    .project(projection_query(
                        queued.clone(),
                        &contains,
                        portfolio,
                        false,
                    ))
                    .expect("outgoing containment projection should evaluate")
            ),
            [feature_a, feature_b]
        );
        assert_eq!(
            projection_ids(
                &backend
                    .project(projection_query(queued.clone(), &depends, feature_a, false,))
                    .expect("outgoing dependency projection should evaluate")
            ),
            [feature_b]
        );
        assert_eq!(
            projection_ids(
                &backend
                    .project(projection_query(queued, &depends, feature_b, true))
                    .expect("incoming dependency projection should evaluate")
            ),
            [feature_a]
        );

        backend
            .transition(TransitionIntentUnit::new(
                feature_b,
                phase("done"),
                IntentUnitRevision::INITIAL,
            ))
            .expect("feature B should transition")
            .intent_unit()
            .clone()
    };

    {
        let mut backend = SqliteBackend::open(database.path())
            .expect("database should reopen after lifecycle mutation");
        let queued = lifecycle_filters(&feature_a_view, "queued", IntentUnitStatus::Active);
        let done = lifecycle_filters(&feature_b_transitioned, "done", IntentUnitStatus::Active);
        assert_eq!(
            projection_ids(
                &backend
                    .project(projection_query(
                        queued.clone(),
                        &contains,
                        portfolio,
                        false,
                    ))
                    .expect("live queued projection should reevaluate")
            ),
            [feature_a]
        );
        assert!(
            backend
                .project(projection_query(queued.clone(), &depends, feature_a, false,))
                .expect("live dependency projection should reevaluate")
                .items()
                .is_empty()
        );
        assert_eq!(
            projection_ids(
                &backend
                    .project(projection_query(done, &depends, feature_a, false))
                    .expect("done dependency projection should evaluate")
            ),
            [feature_b]
        );
        assert_eq!(
            projection_ids(
                &backend
                    .project(projection_query(queued, &depends, feature_b, true))
                    .expect("incoming dependency projection should remain live")
            ),
            [feature_a]
        );

        assert_eq!(
            backend
                .delete_relationship(DeleteRelationship::new(identity(
                    &contains, portfolio, feature_a,
                )))
                .expect("containment edge should delete"),
            relationship_view(&contains, portfolio, feature_a)
        );
    }

    {
        let mut backend =
            SqliteBackend::open(database.path()).expect("deleted edge state should reopen");
        assert_eq!(
            relationship_pairs(
                &backend
                    .list_relationships(relationship_query(&contains))
                    .expect("remaining containment relationship should list")
            ),
            [(portfolio, feature_b)]
        );
        assert_eq!(
            projection_ids(
                &backend
                    .project(projection_query(
                        ListFilters::default(),
                        &contains,
                        portfolio,
                        false,
                    ))
                    .expect("projection should observe the deleted edge")
            ),
            [feature_b]
        );
        backend
            .create_relationship(CreateRelationship::new(identity(
                &contains, portfolio, feature_a,
            )))
            .expect("deleted containment identity should be recreatable");
    }

    let backend = SqliteBackend::open(database.path()).expect("final state should reopen");
    assert_eq!(backend.get(portfolio).unwrap(), portfolio_view);
    assert_eq!(backend.get(feature_a).unwrap(), feature_a_view);
    assert_eq!(backend.get(feature_b).unwrap(), feature_b_transitioned);
    assert_eq!(
        backend
            .get_relationship_definition(contains.clone())
            .unwrap(),
        contains_view
    );
    assert_eq!(
        backend
            .get_relationship_definition(depends.clone())
            .unwrap(),
        depends_view
    );
    let contains_list = relationship_query(&contains);
    assert_eq!(
        backend.list_relationships(contains_list.clone()).unwrap(),
        RelationshipPage::new(
            contains_list,
            vec![
                relationship_view(&contains, portfolio, feature_a),
                relationship_view(&contains, portfolio, feature_b),
            ],
            None,
        )
    );
    let depends_list = relationship_query(&depends);
    assert_eq!(
        backend.list_relationships(depends_list.clone()).unwrap(),
        RelationshipPage::new(
            depends_list,
            vec![relationship_view(&depends, feature_a, feature_b)],
            None,
        )
    );
    let queued_contains = projection_query(
        lifecycle_filters(&feature_a_view, "queued", IntentUnitStatus::Active),
        &contains,
        portfolio,
        false,
    );
    assert_eq!(
        backend.project(queued_contains.clone()).unwrap(),
        ProjectionPage::new(
            queued_contains,
            vec![IntentUnitSummary::from_view(&feature_a_view)],
            None,
        )
    );
    let done_dependency = projection_query(
        lifecycle_filters(&feature_b_transitioned, "done", IntentUnitStatus::Active),
        &depends,
        feature_a,
        false,
    );
    assert_eq!(
        backend.project(done_dependency.clone()).unwrap(),
        ProjectionPage::new(
            done_dependency,
            vec![IntentUnitSummary::from_view(&feature_b_transitioned)],
            None,
        )
    );
    let incoming_dependency = projection_query(
        lifecycle_filters(&feature_a_view, "queued", IntentUnitStatus::Active),
        &depends,
        feature_b,
        true,
    );
    assert_eq!(
        backend.project(incoming_dependency.clone()).unwrap(),
        ProjectionPage::new(
            incoming_dependency,
            vec![IntentUnitSummary::from_view(&feature_a_view)],
            None,
        )
    );
    assert_eq!(
        feature_b_view.revision(),
        IntentUnitRevision::INITIAL,
        "the retained creation view distinguishes the later live mutation"
    );
}

#[test]
fn test_public_backend_migrates_v1_then_relates_projects_and_preserves_units() {
    let database = TestDatabase::new("public-v1-migration-relationship-journey");
    {
        let connection = database.connect();
        initialize_exact_v1(&connection);
    }

    let unit_a = numbered_id(10);
    let unit_b = numbered_id(20);
    let unit_c = numbered_id(30);
    let definition = definition_key("links");
    let definition_command = definition_command(
        &definition,
        "feature",
        "feature",
        RelationshipPolicy::Reject,
    );
    let relationship = identity(&definition, unit_a, unit_b);

    let mut cached_v1 = SqliteBackend::open(database.path()).expect("exact v1 should open");
    assert_eq!(cached_v1.schema_version(), BackendSchemaVersion::V1);
    let unit_a_view = create_unit(&mut cached_v1, unit_a, "feature", "migration-flow");
    let unit_b_view = create_unit(&mut cached_v1, unit_b, "feature", "migration-flow");
    let pre_migration_rows = stored_rows(&database.connect());
    assert_eq!(pre_migration_rows.len(), 2);

    let before_v1_guards = raw_database_snapshot(&database);
    assert_all_relationship_apis_require_migration(
        &mut cached_v1,
        &definition_command,
        &relationship,
    );
    assert_eq!(raw_database_snapshot(&database), before_v1_guards);

    SqliteBackend::migrate_v1_to_v2(database.path())
        .expect("explicit exact-v1 migration should commit");
    assert_eq!(
        stored_rows(&database.connect()),
        pre_migration_rows,
        "migration must preserve every stored unit column and envelope byte"
    );
    assert_eq!(
        cached_v1.schema_version(),
        BackendSchemaVersion::V1,
        "an already-open handle retains its cached capability"
    );

    let unit_c_created = create_unit(&mut cached_v1, unit_c, "feature", "migration-flow");
    assert_eq!(cached_v1.get(unit_c).unwrap(), unit_c_created);
    assert_eq!(
        cached_v1
            .list(ListIntentUnits::new(
                ListFilters::default(),
                PageLimit::new(100).unwrap(),
                None,
            ))
            .expect("cached v1 handle should retain list support")
            .items()
            .iter()
            .map(|item| item.id())
            .collect::<Vec<_>>(),
        [unit_a, unit_b, unit_c]
    );
    let unit_c_transitioned = cached_v1
        .transition(TransitionIntentUnit::new(
            unit_c,
            phase("done"),
            IntentUnitRevision::INITIAL,
        ))
        .expect("cached v1 handle should retain transition support");
    let unit_c_completed = cached_v1
        .complete(CompleteIntentUnit::new(
            unit_c,
            unit_c_transitioned.committed_revision(),
        ))
        .expect("cached v1 handle should retain completion support")
        .intent_unit()
        .clone();

    let before_stale_guards = raw_database_snapshot(&database);
    assert_all_relationship_apis_require_migration(
        &mut cached_v1,
        &definition_command,
        &relationship,
    );
    assert_eq!(raw_database_snapshot(&database), before_stale_guards);
    drop(cached_v1);

    let migrated_definition_view = {
        let mut backend = SqliteBackend::open(database.path()).expect("migrated v2 should reopen");
        assert_eq!(backend.schema_version(), BackendSchemaVersion::V2);
        assert_eq!(backend.get(unit_a).unwrap(), unit_a_view);
        assert_eq!(backend.get(unit_b).unwrap(), unit_b_view);
        assert_eq!(backend.get(unit_c).unwrap(), unit_c_completed);

        let created_definition_view = backend
            .create_relationship_definition(definition_command.clone())
            .expect("definition should create after reopen");
        assert_eq!(
            created_definition_view,
            definition_view(&definition_command)
        );
        assert_eq!(
            backend
                .get_relationship_definition(definition.clone())
                .expect("created definition should load"),
            created_definition_view
        );
        assert_eq!(
            backend
                .create_relationship(CreateRelationship::new(relationship.clone()))
                .expect("relationship should create after migration"),
            RelationshipView::new(relationship.clone())
        );
        assert_eq!(
            relationship_pairs(
                &backend
                    .list_relationships(relationship_query(&definition))
                    .expect("created relationship should list")
            ),
            [(unit_a, unit_b)]
        );
        let projection = projection_query(ListFilters::default(), &definition, unit_a, false);
        assert_eq!(
            projection_ids(
                &backend
                    .project(projection.clone())
                    .expect("created relationship should project")
            ),
            [unit_b]
        );
        assert_eq!(
            backend
                .delete_relationship(DeleteRelationship::new(relationship.clone()))
                .expect("created relationship should delete"),
            RelationshipView::new(relationship.clone())
        );
        assert!(
            backend
                .list_relationships(relationship_query(&definition))
                .unwrap()
                .items()
                .is_empty()
        );
        assert!(backend.project(projection).unwrap().items().is_empty());
        backend
            .create_relationship(CreateRelationship::new(relationship.clone()))
            .expect("deleted relationship should be recreatable");
        created_definition_view
    };

    let backend = SqliteBackend::open(database.path()).expect("final migrated state should reopen");
    assert_eq!(backend.schema_version(), BackendSchemaVersion::V2);
    assert_eq!(backend.get(unit_a).unwrap(), unit_a_view);
    assert_eq!(backend.get(unit_b).unwrap(), unit_b_view);
    assert_eq!(backend.get(unit_c).unwrap(), unit_c_completed);
    assert_eq!(
        backend
            .get_relationship_definition(definition.clone())
            .unwrap(),
        migrated_definition_view
    );
    let final_relationship_query = relationship_query(&definition);
    assert_eq!(
        backend
            .list_relationships(final_relationship_query.clone())
            .unwrap(),
        RelationshipPage::new(
            final_relationship_query,
            vec![RelationshipView::new(relationship.clone())],
            None,
        )
    );
    let final_projection_query =
        projection_query(ListFilters::default(), &definition, unit_a, false);
    assert_eq!(
        backend.project(final_projection_query.clone()).unwrap(),
        ProjectionPage::new(
            final_projection_query,
            vec![IntentUnitSummary::from_view(&unit_b_view)],
            None,
        )
    );
    assert_eq!(
        preserved_rows(stored_rows(&database.connect()), &[unit_a, unit_b]),
        pre_migration_rows,
        "the original v1 rows must remain byte-exact through migration and later v2 work"
    );
}
