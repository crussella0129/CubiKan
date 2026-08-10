mod common;

use common::{
    StoredRelationshipDefinitionSnapshot, TestDatabase, fixed_id, linear_workflow,
    stored_relationship_definitions, stored_rows,
};
use cubikan_backend::{
    CreateIntentUnit, CreateRelationshipDefinition, RelationshipDefinitionId,
    RelationshipDefinitionKey, RelationshipDefinitionVersion, RelationshipDefinitionView,
    RelationshipDirection, RelationshipError, RelationshipPolicy, SqliteBackend,
};
use cubikan_core::IntentSpecies;
use rusqlite::params;

fn species(value: &str) -> IntentSpecies {
    IntentSpecies::new(value).expect("fixture species should be valid")
}

fn definition_key(id: &str, version: u64) -> RelationshipDefinitionKey {
    RelationshipDefinitionKey::new(
        RelationshipDefinitionId::new(id).expect("fixture definition ID should be valid"),
        RelationshipDefinitionVersion::new(version)
            .expect("fixture definition version should be valid"),
    )
}

fn definition_command(
    id: &str,
    version: u64,
    source_species: Option<&str>,
    target_species: Option<&str>,
    self_policy: RelationshipPolicy,
    cycle_policy: RelationshipPolicy,
) -> CreateRelationshipDefinition {
    CreateRelationshipDefinition::new(
        definition_key(id, version),
        RelationshipDirection::Directed,
        source_species.map(species),
        target_species.map(species),
        self_policy,
        cycle_policy,
    )
}

fn expected_view(command: &CreateRelationshipDefinition) -> RelationshipDefinitionView {
    RelationshipDefinitionView::new(
        command.key().clone(),
        command.direction(),
        command.source_species().cloned(),
        command.target_species().cloned(),
        command.self_policy(),
        command.cycle_policy(),
    )
}

#[test]
fn test_definition_create_commits_exact_view_before_return() {
    let database = TestDatabase::new("definition-create");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    let commands = [
        definition_command(
            "allow.allow",
            1,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Allow,
        ),
        definition_command(
            "allow-reject",
            2,
            Some("feature"),
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Reject,
        ),
        definition_command(
            "reject_allow",
            u64::MAX,
            None,
            Some("milestone"),
            RelationshipPolicy::Reject,
            RelationshipPolicy::Allow,
        ),
        definition_command(
            "reject-reject",
            (i64::MAX as u64) + 1,
            Some("  source α  "),
            Some("target β"),
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        ),
    ];

    for command in commands {
        let expected = expected_view(&command);
        let created = backend
            .create_relationship_definition(command.clone())
            .expect("definition should commit");
        assert_eq!(created, expected);

        let fresh = SqliteBackend::open(database.path())
            .expect("definition should be committed before return");
        assert_eq!(
            fresh
                .get_relationship_definition(command.key().clone())
                .expect("committed definition should load"),
            expected
        );
    }

    let rows = stored_relationship_definitions(&database.connect());
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|row| row.directed == 1));
    assert!(rows.iter().any(|row| {
        row.definition_id == "reject_allow" && row.definition_version == u64::MAX.to_be_bytes()
    }));
}

#[test]
fn test_definition_versions_round_trip_independently_across_reopen() {
    let database = TestDatabase::new("definition-versions");
    let commands = [
        definition_command(
            "depends-on",
            u64::MAX,
            Some("feature"),
            Some("milestone"),
            RelationshipPolicy::Reject,
            RelationshipPolicy::Allow,
        ),
        definition_command(
            "depends-on",
            1,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Reject,
        ),
        definition_command(
            "depends-on",
            (i64::MAX as u64) + 1,
            Some("decision"),
            None,
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        ),
        definition_command(
            "depends-on",
            42,
            None,
            Some("deliverable"),
            RelationshipPolicy::Allow,
            RelationshipPolicy::Allow,
        ),
    ];

    {
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        for command in &commands {
            backend
                .create_relationship_definition(command.clone())
                .expect("independent definition version should create");
        }
    }

    let reopened = SqliteBackend::open(database.path()).expect("database should reopen");
    for command in &commands {
        assert_eq!(
            reopened
                .get_relationship_definition(command.key().clone())
                .expect("exact definition version should load"),
            expected_view(command)
        );
    }

    let encoded_versions = stored_relationship_definitions(&database.connect())
        .into_iter()
        .map(|row| row.definition_version)
        .collect::<Vec<_>>();
    assert_eq!(
        encoded_versions,
        [1, 42, (i64::MAX as u64) + 1, u64::MAX]
            .map(u64::to_be_bytes)
            .map(Vec::from)
    );
}

#[test]
fn test_definition_duplicate_and_missing_are_typed_and_nonmutating() {
    let database = TestDatabase::new("definition-duplicate-missing");
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    backend
        .create(CreateIntentUnit::new(
            Some(fixed_id("30000000-0000-0000-0000-000000000003")),
            species("feature"),
            linear_workflow("delivery", "queued", "done"),
        ))
        .expect("fixture unit should create");
    let accepted = definition_command(
        "implements",
        7,
        Some("feature"),
        Some("deliverable"),
        RelationshipPolicy::Reject,
        RelationshipPolicy::Reject,
    );
    let accepted_view = backend
        .create_relationship_definition(accepted.clone())
        .expect("fixture definition should create");
    let before_definitions = stored_relationship_definitions(&database.connect());
    let before_units = stored_rows(&database.connect());

    let duplicate = definition_command(
        "implements",
        7,
        Some("different-source"),
        None,
        RelationshipPolicy::Allow,
        RelationshipPolicy::Allow,
    );
    assert_eq!(
        backend
            .create_relationship_definition(duplicate)
            .expect_err("exact valid identity collision should reject"),
        RelationshipError::DefinitionAlreadyExists {
            definition: accepted.key().clone(),
        }
    );
    assert_eq!(
        backend
            .get_relationship_definition(definition_key("implements", 99))
            .expect_err("missing exact version should reject"),
        RelationshipError::DefinitionNotFound {
            definition: definition_key("implements", 99),
        }
    );

    assert_eq!(
        backend
            .get_relationship_definition(accepted.key().clone())
            .expect("accepted definition should remain exact"),
        accepted_view
    );
    assert_eq!(
        stored_relationship_definitions(&database.connect()),
        before_definitions
    );
    assert_eq!(stored_rows(&database.connect()), before_units);
}

#[test]
fn test_selected_definition_value_corruption_fails_closed_without_repair() {
    let corruptions = [
        (
            "directed",
            "UPDATE relationship_definitions SET directed=0
             WHERE definition_id=?1 COLLATE BINARY AND definition_version=?2",
        ),
        (
            "source-species",
            "UPDATE relationship_definitions SET source_species='   '
             WHERE definition_id=?1 COLLATE BINARY AND definition_version=?2",
        ),
        (
            "target-species",
            "UPDATE relationship_definitions SET target_species=char(9)
             WHERE definition_id=?1 COLLATE BINARY AND definition_version=?2",
        ),
        (
            "self-policy",
            "UPDATE relationship_definitions SET self_policy='sometimes'
             WHERE definition_id=?1 COLLATE BINARY AND definition_version=?2",
        ),
        (
            "cycle-policy",
            "UPDATE relationship_definitions SET cycle_policy='sometimes'
             WHERE definition_id=?1 COLLATE BINARY AND definition_version=?2",
        ),
    ];

    for (label, tamper_sql) in corruptions {
        let database = TestDatabase::new(label);
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        let command = definition_command(
            "tracks",
            u64::MAX,
            Some("feature"),
            Some("deliverable"),
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        );
        backend
            .create_relationship_definition(command.clone())
            .expect("fixture definition should create");

        let connection = database.connect();
        connection
            .pragma_update(None, "ignore_check_constraints", 1_i64)
            .expect("fixture should bypass CHECK constraints deliberately");
        let changed = connection
            .execute(
                tamper_sql,
                params![
                    command.key().id().as_str(),
                    command.key().version().value().to_be_bytes(),
                ],
            )
            .expect("selected definition value should be tampered");
        assert_eq!(changed, 1);
        drop(connection);

        let before = stored_relationship_definitions(&database.connect());
        assert_eq!(before.len(), 1);
        let expected_error = RelationshipError::CorruptDefinition {
            definition: command.key().clone(),
        };
        assert_eq!(
            backend
                .create_relationship_definition(command.clone())
                .expect_err("corrupt collision must outrank duplicate"),
            expected_error
        );
        assert_eq!(stored_relationship_definitions(&database.connect()), before);
        assert_eq!(
            backend
                .get_relationship_definition(command.key().clone())
                .expect_err("selected corrupt definition must fail closed"),
            RelationshipError::CorruptDefinition {
                definition: command.key().clone(),
            }
        );
        assert_eq!(stored_relationship_definitions(&database.connect()), before);

        let [
            StoredRelationshipDefinitionSnapshot {
                definition_id,
                definition_version,
                ..
            },
        ] = before.as_slice()
        else {
            panic!("fixture should retain exactly one selected row");
        };
        assert_eq!(definition_id, command.key().id().as_str());
        assert_eq!(definition_version, &u64::MAX.to_be_bytes());
    }
}
