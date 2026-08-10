mod common;

use std::sync::{Arc, Barrier};

use common::{
    StoredRelationshipDefinitionSnapshot, StoredRelationshipSnapshot, StoredRowSnapshot,
    TestDatabase, fixed_id, linear_workflow, stored_relationship_definitions, stored_relationships,
    stored_rows,
};
use cubikan_backend::{
    BackendError, CreateIntentUnit, CreateRelationship, CreateRelationshipDefinition,
    DeleteRelationship, RelationshipDefinitionId, RelationshipDefinitionKey,
    RelationshipDefinitionVersion, RelationshipDirection, RelationshipEndpoint, RelationshipError,
    RelationshipIdentity, RelationshipPolicy, RelationshipView, SqliteBackend,
};
use cubikan_core::{IntentSpecies, IntentUnitId};
use rusqlite::params;

const A: &str = "10000000-0000-0000-0000-000000000001";
const B: &str = "20000000-0000-0000-0000-000000000002";
const C: &str = "30000000-0000-0000-0000-000000000003";
const NON_CANONICAL_B: &str = "20000000000000000000000000000002";

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

fn definition_command(
    key: RelationshipDefinitionKey,
    source_species: Option<&str>,
    target_species: Option<&str>,
    self_policy: RelationshipPolicy,
    cycle_policy: RelationshipPolicy,
) -> CreateRelationshipDefinition {
    CreateRelationshipDefinition::new(
        key,
        RelationshipDirection::Directed,
        source_species.map(species),
        target_species.map(species),
        self_policy,
        cycle_policy,
    )
}

fn relationship(
    definition: &RelationshipDefinitionKey,
    source: &str,
    target: &str,
) -> RelationshipIdentity {
    RelationshipIdentity::new(definition.clone(), fixed_id(source), fixed_id(target))
}

fn create_unit(backend: &mut SqliteBackend, id: &str, kind: &str) {
    backend
        .create(CreateIntentUnit::new(
            Some(fixed_id(id)),
            species(kind),
            linear_workflow("delivery", "queued", "done"),
        ))
        .expect("fixture unit should create");
}

fn create_standard_units(backend: &mut SqliteBackend) {
    create_unit(backend, A, "feature");
    create_unit(backend, B, "deliverable");
    create_unit(backend, C, "feature");
}

fn create_definition(
    backend: &mut SqliteBackend,
    definition: &RelationshipDefinitionKey,
    source_species: Option<&str>,
    target_species: Option<&str>,
    self_policy: RelationshipPolicy,
    cycle_policy: RelationshipPolicy,
) {
    backend
        .create_relationship_definition(definition_command(
            definition.clone(),
            source_species,
            target_species,
            self_policy,
            cycle_policy,
        ))
        .expect("fixture definition should create");
}

fn create_edge(backend: &mut SqliteBackend, edge: &RelationshipIdentity) -> RelationshipView {
    backend
        .create_relationship(CreateRelationship::new(edge.clone()))
        .expect("fixture relationship should create")
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

fn assert_create_rejected_atomically(
    backend: &mut SqliteBackend,
    database: &TestDatabase,
    edge: RelationshipIdentity,
    expected: RelationshipError,
) {
    let before = snapshot(database);
    assert_eq!(
        backend
            .create_relationship(CreateRelationship::new(edge))
            .expect_err("relationship creation should reject"),
        expected
    );
    assert_eq!(snapshot(database), before);
}

fn assert_delete_rejected_atomically(
    backend: &mut SqliteBackend,
    database: &TestDatabase,
    edge: RelationshipIdentity,
    expected: RelationshipError,
) {
    let before = snapshot(database);
    assert_eq!(
        backend
            .delete_relationship(DeleteRelationship::new(edge))
            .expect_err("relationship deletion should reject"),
        expected
    );
    assert_eq!(snapshot(database), before);
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

fn corrupt_definition_policy(database: &TestDatabase, definition: &RelationshipDefinitionKey) {
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
}

fn delete_raw_endpoint(database: &TestDatabase, id: IntentUnitId) {
    let connection = database.connect();
    connection
        .pragma_update(None, "foreign_keys", false)
        .expect("fixture should disable foreign keys deliberately");
    let changed = connection
        .execute("DELETE FROM intent_units WHERE id=?1", [id.to_string()])
        .expect("fixture endpoint should be removed with foreign keys disabled");
    assert_eq!(changed, 1);
}

fn delete_raw_definition(database: &TestDatabase, definition: &RelationshipDefinitionKey) {
    let connection = database.connect();
    connection
        .pragma_update(None, "foreign_keys", false)
        .expect("fixture should disable foreign keys deliberately");
    let changed = connection
        .execute(
            "DELETE FROM relationship_definitions
             WHERE definition_id=?1 COLLATE BINARY AND definition_version=?2",
            params![
                definition.id().as_str(),
                definition.version().value().to_be_bytes(),
            ],
        )
        .expect("fixture definition should be removed with foreign keys disabled");
    assert_eq!(changed, 1);
}

fn insert_malformed_reachable_edge(
    database: &TestDatabase,
    definition: &RelationshipDefinitionKey,
    source: &str,
) {
    let connection = database.connect();
    let inserted = connection
        .execute(
            "INSERT INTO intent_units (
                id, envelope_version, envelope, workflow_id, species, phase, status, revision
             )
             SELECT ?2, envelope_version, envelope, workflow_id, species, phase,
                    status, revision
             FROM intent_units WHERE id=?1",
            [source, NON_CANONICAL_B],
        )
        .expect("malformed fixture endpoint should insert");
    assert_eq!(inserted, 1);
    let inserted = connection
        .execute(
            "INSERT INTO intent_unit_relationships (
                definition_id, definition_version, source_id, target_id
             ) VALUES (?1, ?2, ?3, ?4)",
            params![
                definition.id().as_str(),
                definition.version().value().to_be_bytes(),
                source,
                NON_CANONICAL_B,
            ],
        )
        .expect("malformed reachable edge should insert");
    assert_eq!(inserted, 1);
}

#[test]
fn test_edge_create_commits_without_mutating_endpoints() {
    let database = TestDatabase::new("edge-create");
    let definition = key("implements", 1);
    let edge = relationship(&definition, A, B);
    let expected = RelationshipView::new(edge.clone());

    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    create_standard_units(&mut backend);
    create_definition(
        &mut backend,
        &definition,
        Some("feature"),
        Some("deliverable"),
        RelationshipPolicy::Reject,
        RelationshipPolicy::Reject,
    );
    let before_units = stored_rows(&database.connect());
    let before_definitions = stored_relationship_definitions(&database.connect());
    assert_eq!(create_edge(&mut backend, &edge), expected);
    assert_eq!(stored_rows(&database.connect()), before_units);
    assert_eq!(
        stored_relationship_definitions(&database.connect()),
        before_definitions
    );
    assert_eq!(
        stored_relationships(&database.connect()),
        [StoredRelationshipSnapshot {
            definition_id: "implements".to_owned(),
            definition_version: 1_u64.to_be_bytes().to_vec(),
            source_id: A.to_owned(),
            target_id: B.to_owned(),
        }]
    );

    drop(backend);
    let mut reopened = SqliteBackend::open(database.path()).expect("database should reopen");
    assert_eq!(
        reopened
            .delete_relationship(DeleteRelationship::new(edge.clone()))
            .expect("reopened backend should observe committed edge"),
        expected
    );
    assert_eq!(create_edge(&mut reopened, &edge), expected);
    assert_eq!(stored_rows(&database.connect()), before_units);
    assert_eq!(
        stored_relationship_definitions(&database.connect()),
        before_definitions
    );
}

#[test]
fn test_edge_policy_rejections_are_atomic() {
    // Missing definition precedes every endpoint lookup.
    {
        let database = TestDatabase::new("edge-missing-definition");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        create_standard_units(&mut backend);
        let definition = key("missing", 1);
        let edge = relationship(&definition, A, B);
        assert_create_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::DefinitionNotFound { definition },
        );
    }

    // A selected corrupt definition outranks endpoint and policy work.
    {
        let database = TestDatabase::new("edge-corrupt-definition");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        create_standard_units(&mut backend);
        let definition = key("corrupt", 1);
        create_definition(
            &mut backend,
            &definition,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Reject,
        );
        corrupt_definition_policy(&database, &definition);
        let edge = relationship(&definition, A, B);
        assert_create_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::CorruptDefinition {
                definition: definition.clone(),
            },
        );
    }

    // Source replay precedes target replay.
    {
        let database = TestDatabase::new("edge-missing-source");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        create_unit(&mut backend, B, "deliverable");
        let definition = key("endpoints", 1);
        create_definition(
            &mut backend,
            &definition,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Allow,
        );
        let edge = relationship(&definition, A, C);
        assert_create_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::EndpointNotFound {
                endpoint: RelationshipEndpoint::Source,
                id: fixed_id(A),
            },
        );
    }
    {
        let database = TestDatabase::new("edge-corrupt-source");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        create_standard_units(&mut backend);
        let definition = key("endpoints", 1);
        create_definition(
            &mut backend,
            &definition,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Allow,
        );
        corrupt_envelope(&database, fixed_id(A));
        let edge = relationship(&definition, A, "40000000-0000-0000-0000-000000000004");
        assert_create_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::EndpointCorrupt {
                endpoint: RelationshipEndpoint::Source,
                id: fixed_id(A),
                source: BackendError::CorruptEnvelope,
            },
        );
    }
    {
        let database = TestDatabase::new("edge-missing-target");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        create_unit(&mut backend, A, "feature");
        let definition = key("endpoints", 1);
        create_definition(
            &mut backend,
            &definition,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Allow,
        );
        let edge = relationship(&definition, A, B);
        assert_create_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::EndpointNotFound {
                endpoint: RelationshipEndpoint::Target,
                id: fixed_id(B),
            },
        );
    }
    {
        let database = TestDatabase::new("edge-corrupt-target");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        create_standard_units(&mut backend);
        let definition = key("endpoints", 1);
        create_definition(
            &mut backend,
            &definition,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Allow,
        );
        corrupt_envelope(&database, fixed_id(B));
        let edge = relationship(&definition, A, B);
        assert_create_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::EndpointCorrupt {
                endpoint: RelationshipEndpoint::Target,
                id: fixed_id(B),
                source: BackendError::CorruptEnvelope,
            },
        );
    }

    // Role-specific species checks precede self, duplicate, and cycle checks.
    for (label, source_kind, target_kind, endpoint, expected, actual) in [
        (
            "edge-source-species",
            "wrong",
            "deliverable",
            RelationshipEndpoint::Source,
            "feature",
            "wrong",
        ),
        (
            "edge-target-species",
            "feature",
            "wrong",
            RelationshipEndpoint::Target,
            "deliverable",
            "wrong",
        ),
    ] {
        let database = TestDatabase::new(label);
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        create_unit(&mut backend, A, source_kind);
        create_unit(&mut backend, B, target_kind);
        let definition = key("species", 1);
        create_definition(
            &mut backend,
            &definition,
            Some("feature"),
            Some("deliverable"),
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        );
        let edge = relationship(&definition, A, B);
        let id = match endpoint {
            RelationshipEndpoint::Source => fixed_id(A),
            RelationshipEndpoint::Target => fixed_id(B),
        };
        assert_create_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::EndpointSpeciesMismatch {
                endpoint,
                id,
                expected: species(expected),
                actual: species(actual),
            },
        );
    }

    // Self rejection precedes duplicate/cycle; duplicate precedes reachability.
    {
        let database = TestDatabase::new("edge-self-duplicate");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        create_standard_units(&mut backend);
        let definition = key("precedes", 1);
        create_definition(
            &mut backend,
            &definition,
            None,
            None,
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        );
        let self_edge = relationship(&definition, A, A);
        assert_create_rejected_atomically(
            &mut backend,
            &database,
            self_edge.clone(),
            RelationshipError::SelfEdgeRejected {
                relationship: self_edge,
            },
        );

        let accepted = relationship(&definition, A, B);
        create_edge(&mut backend, &accepted);
        insert_malformed_reachable_edge(&database, &definition, B);
        assert_create_rejected_atomically(
            &mut backend,
            &database,
            accepted.clone(),
            RelationshipError::DuplicateRelationship {
                relationship: accepted,
            },
        );
        let reaches_corruption = relationship(&definition, C, B);
        assert_create_rejected_atomically(
            &mut backend,
            &database,
            reaches_corruption,
            RelationshipError::CorruptRelationship {
                definition: definition.clone(),
            },
        );
    }

    // Endpoint corruption still outranks a path that would otherwise close a cycle.
    {
        let database = TestDatabase::new("edge-endpoint-before-cycle");
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        create_standard_units(&mut backend);
        let definition = key("cycle", 1);
        create_definition(
            &mut backend,
            &definition,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Reject,
        );
        create_edge(&mut backend, &relationship(&definition, B, C));
        corrupt_envelope(&database, fixed_id(C));
        let proposed = relationship(&definition, C, B);
        assert_create_rejected_atomically(
            &mut backend,
            &database,
            proposed,
            RelationshipError::EndpointCorrupt {
                endpoint: RelationshipEndpoint::Source,
                id: fixed_id(C),
                source: BackendError::CorruptEnvelope,
            },
        );
    }
}

#[test]
fn test_self_and_cycle_policy_matrix_is_version_scoped() {
    let database = TestDatabase::new("edge-policy-matrix");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    create_standard_units(&mut backend);

    for (id, self_policy, cycle_policy, self_allowed, cycle_allowed) in [
        (
            "allow-allow",
            RelationshipPolicy::Allow,
            RelationshipPolicy::Allow,
            true,
            true,
        ),
        (
            "allow-reject",
            RelationshipPolicy::Allow,
            RelationshipPolicy::Reject,
            true,
            false,
        ),
        (
            "reject-allow",
            RelationshipPolicy::Reject,
            RelationshipPolicy::Allow,
            false,
            true,
        ),
        (
            "reject-reject",
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
            false,
            false,
        ),
    ] {
        let definition = key(id, 1);
        create_definition(
            &mut backend,
            &definition,
            None,
            None,
            self_policy,
            cycle_policy,
        );
        let self_edge = relationship(&definition, A, A);
        let self_result = backend.create_relationship(CreateRelationship::new(self_edge.clone()));
        if self_allowed {
            assert_eq!(
                self_result.expect("self edge should be allowed"),
                RelationshipView::new(self_edge)
            );
        } else {
            assert_eq!(
                self_result.expect_err("self edge should reject"),
                RelationshipError::SelfEdgeRejected {
                    relationship: self_edge,
                }
            );
        }

        let forward = relationship(&definition, A, B);
        let reverse = relationship(&definition, B, A);
        create_edge(&mut backend, &forward);
        let reverse_result = backend.create_relationship(CreateRelationship::new(reverse.clone()));
        if cycle_allowed {
            assert_eq!(
                reverse_result.expect("non-self cycle should be allowed"),
                RelationshipView::new(reverse)
            );
        } else {
            assert_eq!(
                reverse_result.expect_err("non-self cycle should reject"),
                RelationshipError::CycleRejected {
                    relationship: reverse,
                }
            );
        }
    }

    let version_one = key("version-scoped", 1);
    let version_two = key("version-scoped", u64::MAX);
    for definition in [&version_one, &version_two] {
        create_definition(
            &mut backend,
            definition,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Reject,
        );
    }
    create_edge(&mut backend, &relationship(&version_one, A, B));
    create_edge(&mut backend, &relationship(&version_two, B, A));
    assert_eq!(
        stored_relationships(&database.connect())
            .iter()
            .filter(|row| row.definition_id == "version-scoped")
            .count(),
        2
    );
}

#[test]
fn test_concurrent_cycle_creators_commit_once_then_reject_cycle() {
    let database = TestDatabase::new("edge-concurrent-cycle");
    let definition = key("serialized-cycle", 1);
    {
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        create_standard_units(&mut backend);
        create_definition(
            &mut backend,
            &definition,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Reject,
        );
    }

    let mut first = SqliteBackend::open(database.path()).expect("first writer should open");
    let mut second = SqliteBackend::open(database.path()).expect("second writer should open");
    let forward = relationship(&definition, A, B);
    let reverse = relationship(&definition, B, A);
    let barrier = Arc::new(Barrier::new(3));
    let first_barrier = Arc::clone(&barrier);
    let first_edge = forward.clone();
    let first_thread = std::thread::spawn(move || {
        first_barrier.wait();
        first.create_relationship(CreateRelationship::new(first_edge))
    });
    let second_barrier = Arc::clone(&barrier);
    let second_edge = reverse.clone();
    let second_thread = std::thread::spawn(move || {
        second_barrier.wait();
        second.create_relationship(CreateRelationship::new(second_edge))
    });
    barrier.wait();
    let results = [
        first_thread.join().expect("first writer should return"),
        second_thread.join().expect("second writer should return"),
    ];

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(RelationshipError::CycleRejected { .. })))
            .count(),
        1
    );
    assert_eq!(stored_relationships(&database.connect()).len(), 1);
    assert!(
        stored_relationships(&database.connect())[0]
            == StoredRelationshipSnapshot {
                definition_id: definition.id().as_str().to_owned(),
                definition_version: definition.version().value().to_be_bytes().to_vec(),
                source_id: A.to_owned(),
                target_id: B.to_owned(),
            }
            || stored_relationships(&database.connect())[0]
                == StoredRelationshipSnapshot {
                    definition_id: definition.id().as_str().to_owned(),
                    definition_version: definition.version().value().to_be_bytes().to_vec(),
                    source_id: B.to_owned(),
                    target_id: A.to_owned(),
                }
    );
}

fn setup_existing_edge(
    label: &str,
) -> (
    TestDatabase,
    SqliteBackend,
    RelationshipDefinitionKey,
    RelationshipIdentity,
) {
    let database = TestDatabase::new(label);
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    create_standard_units(&mut backend);
    let definition = key("deletes", 1);
    create_definition(
        &mut backend,
        &definition,
        None,
        None,
        RelationshipPolicy::Allow,
        RelationshipPolicy::Allow,
    );
    let edge = relationship(&definition, A, B);
    create_edge(&mut backend, &edge);
    (database, backend, definition, edge)
}

#[test]
fn test_edge_delete_is_exact_non_cascading_and_atomic_on_failure() {
    // Exact deletion removes one row only and an explicit corrected edge can be created.
    {
        let (database, mut backend, definition, edge) = setup_existing_edge("edge-delete-exact");
        let retained_one = relationship(&definition, A, C);
        let retained_two = relationship(&definition, C, B);
        create_edge(&mut backend, &retained_one);
        create_edge(&mut backend, &retained_two);
        let before_units = stored_rows(&database.connect());
        let before_definitions = stored_relationship_definitions(&database.connect());
        assert_eq!(
            backend
                .delete_relationship(DeleteRelationship::new(edge.clone()))
                .expect("exact edge should delete"),
            RelationshipView::new(edge.clone())
        );
        assert_eq!(stored_rows(&database.connect()), before_units);
        assert_eq!(
            stored_relationship_definitions(&database.connect()),
            before_definitions
        );
        assert_eq!(
            stored_relationships(&database.connect()),
            [
                StoredRelationshipSnapshot {
                    definition_id: "deletes".to_owned(),
                    definition_version: 1_u64.to_be_bytes().to_vec(),
                    source_id: A.to_owned(),
                    target_id: C.to_owned(),
                },
                StoredRelationshipSnapshot {
                    definition_id: "deletes".to_owned(),
                    definition_version: 1_u64.to_be_bytes().to_vec(),
                    source_id: C.to_owned(),
                    target_id: B.to_owned(),
                },
            ]
        );

        let missing = relationship(&definition, B, A);
        assert_delete_rejected_atomically(
            &mut backend,
            &database,
            missing.clone(),
            RelationshipError::RelationshipNotFound {
                relationship: missing,
            },
        );
        let corrected = relationship(&definition, B, A);
        create_edge(&mut backend, &corrected);
        create_edge(&mut backend, &edge);
        assert_eq!(stored_rows(&database.connect()), before_units);
        assert_eq!(
            stored_relationship_definitions(&database.connect()),
            before_definitions
        );
    }

    // Definition and endpoint failures follow the same strict delete precedence.
    {
        let (database, mut backend, definition, edge) =
            setup_existing_edge("edge-delete-missing-definition");
        delete_raw_definition(&database, &definition);
        assert_delete_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::DefinitionNotFound { definition },
        );
    }
    {
        let (database, mut backend, definition, edge) =
            setup_existing_edge("edge-delete-corrupt-definition");
        corrupt_definition_policy(&database, &definition);
        assert_delete_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::CorruptDefinition {
                definition: definition.clone(),
            },
        );
    }
    for (label, removed, endpoint) in [
        (
            "edge-delete-missing-source",
            fixed_id(A),
            RelationshipEndpoint::Source,
        ),
        (
            "edge-delete-missing-target",
            fixed_id(B),
            RelationshipEndpoint::Target,
        ),
    ] {
        let (database, mut backend, _definition, edge) = setup_existing_edge(label);
        delete_raw_endpoint(&database, removed);
        assert_delete_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::EndpointNotFound {
                endpoint,
                id: removed,
            },
        );
    }
    for (label, corrupt, endpoint) in [
        (
            "edge-delete-corrupt-source",
            fixed_id(A),
            RelationshipEndpoint::Source,
        ),
        (
            "edge-delete-corrupt-target",
            fixed_id(B),
            RelationshipEndpoint::Target,
        ),
    ] {
        let (database, mut backend, _definition, edge) = setup_existing_edge(label);
        corrupt_envelope(&database, corrupt);
        assert_delete_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::EndpointCorrupt {
                endpoint,
                id: corrupt,
                source: BackendError::CorruptEnvelope,
            },
        );
    }
    for (label, column, endpoint, id, expected, actual) in [
        (
            "edge-delete-source-species",
            "source_species",
            RelationshipEndpoint::Source,
            fixed_id(A),
            "other-source",
            "feature",
        ),
        (
            "edge-delete-target-species",
            "target_species",
            RelationshipEndpoint::Target,
            fixed_id(B),
            "other-target",
            "deliverable",
        ),
    ] {
        let (database, mut backend, definition, edge) = setup_existing_edge(label);
        let sql = format!(
            "UPDATE relationship_definitions SET {column}=?1
             WHERE definition_id=?2 COLLATE BINARY AND definition_version=?3"
        );
        let changed = database
            .connect()
            .execute(
                &sql,
                params![
                    expected,
                    definition.id().as_str(),
                    definition.version().value().to_be_bytes(),
                ],
            )
            .expect("fixture species constraint should change");
        assert_eq!(changed, 1);
        assert_delete_rejected_atomically(
            &mut backend,
            &database,
            edge,
            RelationshipError::EndpointSpeciesMismatch {
                endpoint,
                id,
                expected: species(expected),
                actual: species(actual),
            },
        );
    }

    // A real competing writer fails once as busy and never reaches semantic deletion.
    {
        let (database, mut backend, _definition, edge) = setup_existing_edge("edge-delete-busy");
        let before = snapshot(&database);
        let locker = database.connect();
        locker
            .execute_batch("BEGIN IMMEDIATE")
            .expect("fixture writer should acquire lock");
        let error = backend
            .delete_relationship(DeleteRelationship::new(edge))
            .expect_err("competing writer should make deletion busy");
        assert!(matches!(
            error,
            RelationshipError::Backend(BackendError::StorageBusy(_))
        ));
        locker
            .execute_batch("ROLLBACK")
            .expect("fixture writer should release lock");
        assert_eq!(snapshot(&database), before);
    }

    // A SQLite abort after all validation rolls the transaction back without retry.
    {
        let (database, mut backend, _definition, edge) = setup_existing_edge("edge-delete-abort");
        database
            .connect()
            .execute_batch(
                "CREATE TRIGGER reject_relationship_delete
                 BEFORE DELETE ON intent_unit_relationships
                 BEGIN
                    SELECT RAISE(ABORT, 'fixture abort');
                 END",
            )
            .expect("fixture abort trigger should create after backend open");
        let before = snapshot(&database);
        let error = backend
            .delete_relationship(DeleteRelationship::new(edge))
            .expect_err("SQLite abort should fail deletion");
        assert!(matches!(
            error,
            RelationshipError::Backend(BackendError::Storage(_))
        ));
        assert_eq!(snapshot(&database), before);
    }
}
