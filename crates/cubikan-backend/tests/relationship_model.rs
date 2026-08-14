use std::{error::Error, str::FromStr};

use cubikan_backend::{
    BackendError, BackendSchemaVersion, CreateRelationship, CreateRelationshipDefinition,
    DeleteRelationship, DirectRelationshipPredicate, IntentUnitSummary, IntentUnitView, ListCursor,
    ListFilters, ListRelationships, MigrationError, PageLimit, ProjectionPage, ProjectionQueryV1,
    RelationshipCursor, RelationshipDefinitionId, RelationshipDefinitionIdError,
    RelationshipDefinitionKey, RelationshipDefinitionVersion, RelationshipDefinitionVersionError,
    RelationshipDefinitionView, RelationshipDirection, RelationshipEndpoint, RelationshipError,
    RelationshipIdentity, RelationshipPage, RelationshipPolicy, RelationshipQueryError,
    RelationshipView,
};
use cubikan_core::{
    ExternalReference, IntentSpecies, IntentUnit, IntentUnitId, IntentUnitStatus, PhaseId,
    ReferenceNamespace, ReferenceText, Workflow, WorkflowEdge, WorkflowId,
};

fn fixed_id(value: &str) -> IntentUnitId {
    value.parse().expect("fixture UUID should be valid")
}

fn origin() -> ExternalReference {
    ExternalReference::new(
        ReferenceNamespace::new("github").expect("fixture namespace should be valid"),
        ReferenceText::new("crussella0129/CubiKan").expect("fixture scope should be valid"),
        ReferenceText::new("issue:1107").expect("fixture value should be valid"),
    )
}

fn definition(value: &str, version: u64) -> RelationshipDefinitionKey {
    RelationshipDefinitionKey::new(
        RelationshipDefinitionId::new(value).expect("fixture definition ID should be valid"),
        RelationshipDefinitionVersion::new(version)
            .expect("fixture definition version should be valid"),
    )
}

fn workflow() -> Workflow {
    let queued = PhaseId::new("queued").expect("fixture phase should be valid");
    let done = PhaseId::new("done").expect("fixture phase should be valid");
    Workflow::new(
        WorkflowId::new("delivery").expect("fixture workflow ID should be valid"),
        vec![queued.clone(), done.clone()],
        queued.clone(),
        vec![WorkflowEdge::new(queued, done.clone())],
        vec![done],
    )
    .expect("fixture workflow should be valid")
}

#[test]
fn test_relationship_model_validates_ids_versions_policies_limits_and_cursors() {
    let maximum = format!("a{}", "z".repeat(63));
    assert_eq!(RelationshipDefinitionId::new("a").unwrap().as_str(), "a");
    assert_eq!(
        RelationshipDefinitionId::new(maximum.clone())
            .unwrap()
            .as_str(),
        maximum
    );
    assert_eq!(
        RelationshipDefinitionId::new("depends.on_v2-3")
            .unwrap()
            .as_str(),
        "depends.on_v2-3"
    );

    assert_eq!(
        RelationshipDefinitionId::new(""),
        Err(RelationshipDefinitionIdError::Empty)
    );
    assert_eq!(
        RelationshipDefinitionId::new("A".repeat(65)),
        Err(RelationshipDefinitionIdError::TooLong { bytes: 65 })
    );
    assert_eq!(
        RelationshipDefinitionId::new("A-valid-after-start"),
        Err(RelationshipDefinitionIdError::InvalidStart)
    );
    assert_eq!(
        RelationshipDefinitionId::new("é"),
        Err(RelationshipDefinitionIdError::InvalidStart)
    );
    assert_eq!(
        RelationshipDefinitionId::new("ab/c"),
        Err(RelationshipDefinitionIdError::InvalidCharacter { index: 2 })
    );
    assert_eq!(
        RelationshipDefinitionId::new("aé"),
        Err(RelationshipDefinitionIdError::InvalidCharacter { index: 1 })
    );

    for value in [1, i64::MAX as u64 + 1, u64::MAX] {
        assert_eq!(
            RelationshipDefinitionVersion::new(value).unwrap().value(),
            value
        );
    }
    assert_eq!(
        RelationshipDefinitionVersion::new(0),
        Err(RelationshipDefinitionVersionError::Zero)
    );
    let policy_key = definition("policy", 1);
    for self_policy in [RelationshipPolicy::Allow, RelationshipPolicy::Reject] {
        for cycle_policy in [RelationshipPolicy::Allow, RelationshipPolicy::Reject] {
            let command = CreateRelationshipDefinition::new(
                policy_key.clone(),
                RelationshipDirection::Directed,
                None,
                None,
                self_policy,
                cycle_policy,
            );
            assert_eq!(command.self_policy(), self_policy);
            assert_eq!(command.cycle_policy(), cycle_policy);
        }
    }
    assert_eq!(PageLimit::new(1).unwrap().value(), 1);
    assert_eq!(PageLimit::new(100).unwrap().value(), 100);
    assert!(PageLimit::new(0).is_err());
    assert!(PageLimit::new(101).is_err());

    let nil = fixed_id("00000000-0000-0000-0000-000000000000");
    let ordinary = fixed_id("67e55044-10b1-426f-9247-bb680e5fe0c8");
    let edge = RelationshipIdentity::new(definition("depends", 1), nil, ordinary);
    assert_eq!(edge.source(), nil);
    assert_eq!(edge.target(), ordinary);

    let cursor = RelationshipCursor::new(edge.clone());
    let matching = ListRelationships::new(
        edge.definition().clone(),
        None,
        None,
        PageLimit::new(10).unwrap(),
        Some(cursor),
    )
    .expect("complete cursor from the same definition should be valid");
    assert_eq!(matching.after().unwrap().relationship(), &edge);

    let expected = definition("depends", 2);
    let actual = edge.definition().clone();
    let mismatch = ListRelationships::new(
        expected.clone(),
        None,
        None,
        PageLimit::new(10).unwrap(),
        Some(RelationshipCursor::new(edge)),
    )
    .expect_err("cross-definition cursor must reject before storage");
    assert_eq!(
        mismatch,
        RelationshipQueryError::CursorDefinitionMismatch { expected, actual }
    );
}

#[test]
fn test_public_relationship_model_exposes_complete_contract() {
    let source = fixed_id("00000000-0000-0000-0000-000000000001");
    let target = fixed_id("00000000-0000-0000-0000-000000000002");
    let definition = definition("blocks", u64::MAX);
    let source_species = IntentSpecies::new("feature").expect("species should be valid");
    let target_species = IntentSpecies::new("deliverable").expect("species should be valid");

    assert_eq!(definition.id().as_str(), "blocks");
    assert_eq!(definition.version().value(), u64::MAX);
    let create_definition = CreateRelationshipDefinition::new(
        definition.clone(),
        RelationshipDirection::Directed,
        Some(source_species.clone()),
        Some(target_species.clone()),
        RelationshipPolicy::Reject,
        RelationshipPolicy::Allow,
    );
    assert_eq!(create_definition.key(), &definition);
    assert_eq!(
        create_definition.direction(),
        RelationshipDirection::Directed
    );
    assert_eq!(create_definition.source_species(), Some(&source_species));
    assert_eq!(create_definition.target_species(), Some(&target_species));
    assert_eq!(create_definition.self_policy(), RelationshipPolicy::Reject);
    assert_eq!(create_definition.cycle_policy(), RelationshipPolicy::Allow);

    let get_definition = definition.clone();
    let definition_view = RelationshipDefinitionView::new(
        get_definition.clone(),
        RelationshipDirection::Directed,
        Some(source_species.clone()),
        Some(target_species.clone()),
        RelationshipPolicy::Reject,
        RelationshipPolicy::Allow,
    );
    assert_eq!(definition_view.key(), &get_definition);
    assert_eq!(definition_view.direction(), RelationshipDirection::Directed);
    assert_eq!(definition_view.source_species(), Some(&source_species));
    assert_eq!(definition_view.target_species(), Some(&target_species));
    assert_eq!(definition_view.self_policy(), RelationshipPolicy::Reject);
    assert_eq!(definition_view.cycle_policy(), RelationshipPolicy::Allow);

    let identity = RelationshipIdentity::new(definition.clone(), source, target);
    let create = CreateRelationship::new(identity.clone());
    let delete = DeleteRelationship::new(identity.clone());
    let view = RelationshipView::new(identity.clone());
    let cursor = RelationshipCursor::new(identity.clone());
    assert_eq!(identity.definition(), &definition);
    assert_eq!(identity.source(), source);
    assert_eq!(identity.target(), target);
    assert_eq!(create.relationship(), &identity);
    assert_eq!(delete.relationship(), &identity);
    assert_eq!(view.relationship(), &identity);
    assert_eq!(cursor.relationship(), &identity);

    let list = ListRelationships::new(
        definition.clone(),
        Some(source),
        Some(target),
        PageLimit::new(2).unwrap(),
        Some(cursor.clone()),
    )
    .unwrap();
    assert_eq!(list.definition(), &definition);
    assert_eq!(list.source(), Some(source));
    assert_eq!(list.target(), Some(target));
    assert_eq!(list.limit().value(), 2);
    assert_eq!(list.after(), Some(&cursor));
    let relationship_page =
        RelationshipPage::new(list.clone(), vec![view.clone()], Some(cursor.clone()));
    assert_eq!(relationship_page.query(), &list);
    assert_eq!(relationship_page.items(), &[view]);
    assert_eq!(relationship_page.next_cursor(), Some(&cursor));

    let outgoing = DirectRelationshipPredicate::Outgoing {
        definition: definition.clone(),
        anchor: source,
    };
    let incoming = DirectRelationshipPredicate::Incoming {
        definition: definition.clone(),
        anchor: target,
    };
    assert_eq!(outgoing.definition(), &definition);
    assert_eq!(outgoing.anchor(), source);
    assert_eq!(incoming.definition(), &definition);
    assert_eq!(incoming.anchor(), target);

    let filters = ListFilters::new(
        Some(WorkflowId::new("delivery").unwrap()),
        Some(source_species.clone()),
        Some(PhaseId::new("queued").unwrap()),
        Some(IntentUnitStatus::Active),
    );
    let unit = IntentUnit::new(source, origin(), source_species, workflow());
    let summary = IntentUnitSummary::from_view(&IntentUnitView::from_intent_unit(&unit));
    let list_cursor = ListCursor::from_str("00000000-0000-0000-0000-000000000001").unwrap();
    let outgoing_query = ProjectionQueryV1::new(
        filters.clone(),
        Some(outgoing.clone()),
        PageLimit::new(3).unwrap(),
        Some(list_cursor),
    );
    let incoming_query = ProjectionQueryV1::new(
        filters.clone(),
        Some(incoming.clone()),
        PageLimit::new(3).unwrap(),
        None,
    );
    let lifecycle_only =
        ProjectionQueryV1::new(filters.clone(), None, PageLimit::new(3).unwrap(), None);

    assert_eq!(ProjectionQueryV1::VERSION, 1);
    assert_eq!(outgoing_query.version(), 1);
    assert_eq!(outgoing_query.filters(), &filters);
    assert_eq!(outgoing_query.predicate(), Some(&outgoing));
    assert_eq!(outgoing_query.limit().value(), 3);
    assert_eq!(outgoing_query.after(), Some(list_cursor));
    assert_eq!(incoming_query.predicate(), Some(&incoming));
    assert_eq!(lifecycle_only.predicate(), None);

    let projection_page = ProjectionPage::new(
        outgoing_query.clone(),
        vec![summary.clone()],
        Some(list_cursor),
    );
    assert_eq!(projection_page.query(), &outgoing_query);
    assert_eq!(projection_page.items(), &[summary]);
    assert_eq!(projection_page.next_cursor(), Some(list_cursor));
    assert_eq!(BackendSchemaVersion::V1.value(), 1);
    assert_eq!(BackendSchemaVersion::V2.value(), 2);
}

#[test]
fn test_relationship_error_taxonomy_is_typed_and_source_preserving() {
    let id = fixed_id("00000000-0000-0000-0000-000000000001");
    let definition_key = definition("depends", 1);
    let relationship = RelationshipIdentity::new(definition_key.clone(), id, id);
    let feature = IntentSpecies::new("feature").unwrap();
    let defect = IntentSpecies::new("defect").unwrap();

    assert_eq!(
        RelationshipDefinitionId::new(""),
        Err(RelationshipDefinitionIdError::Empty)
    );
    assert_eq!(
        RelationshipDefinitionVersion::new(0),
        Err(RelationshipDefinitionVersionError::Zero)
    );
    let other = definition("depends", 2);
    let query_error = ListRelationships::new(
        other.clone(),
        None,
        None,
        PageLimit::new(1).unwrap(),
        Some(RelationshipCursor::new(relationship.clone())),
    )
    .unwrap_err();
    assert_eq!(
        query_error,
        RelationshipQueryError::CursorDefinitionMismatch {
            expected: other,
            actual: definition_key.clone(),
        }
    );

    let relationship_errors = [
        RelationshipError::MigrationRequired {
            found: BackendSchemaVersion::V1,
            required: BackendSchemaVersion::V2,
        },
        RelationshipError::DefinitionAlreadyExists {
            definition: definition_key.clone(),
        },
        RelationshipError::DefinitionNotFound {
            definition: definition_key.clone(),
        },
        RelationshipError::CorruptDefinition {
            definition: definition_key.clone(),
        },
        RelationshipError::EndpointNotFound {
            endpoint: RelationshipEndpoint::Source,
            id,
        },
        RelationshipError::EndpointSpeciesMismatch {
            endpoint: RelationshipEndpoint::Target,
            id,
            expected: feature.clone(),
            actual: defect.clone(),
        },
        RelationshipError::SelfEdgeRejected {
            relationship: relationship.clone(),
        },
        RelationshipError::CycleRejected {
            relationship: relationship.clone(),
        },
        RelationshipError::DuplicateRelationship {
            relationship: relationship.clone(),
        },
        RelationshipError::RelationshipNotFound {
            relationship: relationship.clone(),
        },
        RelationshipError::CorruptRelationship {
            definition: definition_key.clone(),
        },
    ];
    fn relationship_kind(error: &RelationshipError) -> &'static str {
        match error {
            RelationshipError::MigrationRequired { .. } => "migration",
            RelationshipError::DefinitionAlreadyExists { .. } => "definition-duplicate",
            RelationshipError::DefinitionNotFound { .. } => "definition-missing",
            RelationshipError::CorruptDefinition { .. } => "definition-corrupt",
            RelationshipError::EndpointNotFound { .. } => "endpoint-missing",
            RelationshipError::EndpointCorrupt { .. } => "endpoint-corrupt",
            RelationshipError::EndpointSpeciesMismatch { .. } => "endpoint-species",
            RelationshipError::SelfEdgeRejected { .. } => "self",
            RelationshipError::CycleRejected { .. } => "cycle",
            RelationshipError::DuplicateRelationship { .. } => "relationship-duplicate",
            RelationshipError::RelationshipNotFound { .. } => "relationship-missing",
            RelationshipError::CorruptRelationship { .. } => "relationship-corrupt",
            RelationshipError::Backend(_) => "backend",
        }
    }
    assert_eq!(
        relationship_errors
            .each_ref()
            .map(|error| relationship_kind(error)),
        [
            "migration",
            "definition-duplicate",
            "definition-missing",
            "definition-corrupt",
            "endpoint-missing",
            "endpoint-species",
            "self",
            "cycle",
            "relationship-duplicate",
            "relationship-missing",
            "relationship-corrupt",
        ]
    );
    assert!(matches!(
        &relationship_errors[0],
        RelationshipError::MigrationRequired {
            found: BackendSchemaVersion::V1,
            required: BackendSchemaVersion::V2
        }
    ));
    assert!(matches!(
        &relationship_errors[4],
        RelationshipError::EndpointNotFound {
            endpoint: RelationshipEndpoint::Source,
            id: found
        } if *found == id
    ));
    assert!(matches!(
        &relationship_errors[5],
        RelationshipError::EndpointSpeciesMismatch {
            endpoint: RelationshipEndpoint::Target,
            id: found,
            expected,
            actual
        } if *found == id && expected == &feature && actual == &defect
    ));

    let corrupt_endpoint = RelationshipError::EndpointCorrupt {
        endpoint: RelationshipEndpoint::Source,
        id,
        source: BackendError::CorruptEnvelope,
    };
    assert!(matches!(
        corrupt_endpoint,
        RelationshipError::EndpointCorrupt {
            endpoint: RelationshipEndpoint::Source,
            id: found,
            source: BackendError::CorruptEnvelope
        } if found == id
    ));
    assert!(matches!(
        Error::source(&corrupt_endpoint),
        Some(source) if source.downcast_ref::<BackendError>() == Some(&BackendError::CorruptEnvelope)
    ));
    assert_eq!(relationship_kind(&corrupt_endpoint), "endpoint-corrupt");

    let schema = RelationshipError::Backend(BackendError::CorruptSchema);
    let unit = RelationshipError::Backend(BackendError::IntentUnitNotFound { id });
    assert!(matches!(
        Error::source(&schema),
        Some(source) if source.downcast_ref::<BackendError>() == Some(&BackendError::CorruptSchema)
    ));
    assert!(matches!(
        Error::source(&unit),
        Some(source)
            if source.downcast_ref::<BackendError>()
                == Some(&BackendError::IntentUnitNotFound { id })
    ));

    let storage = RelationshipError::from(BackendError::UnsupportedSchemaVersion { found: 2 });
    let backend_source = Error::source(&storage).expect("wrapper should preserve backend source");
    assert!(backend_source.downcast_ref::<BackendError>().is_some());
    assert!(backend_source.source().is_none());

    fn exhaustive_backend_kind(error: &BackendError) -> &'static str {
        match error {
            BackendError::DuplicateIntentUnit { .. } => "duplicate-unit",
            BackendError::IntentUnitNotFound { .. } => "missing-unit",
            BackendError::RevisionConflict(_) => "revision",
            BackendError::TransitionRejected(_) => "transition",
            BackendError::CompletionRejected(_) => "completion",
            BackendError::UnownedDatabase => "unowned",
            BackendError::UnsupportedPlatform => "platform",
            BackendError::InsecureProjectionPath => "projection-path",
            BackendError::UnsupportedSchemaVersion { .. } => "schema-version",
            BackendError::CorruptSchema => "schema",
            BackendError::UnsupportedEnvelopeVersion { .. } => "envelope-version",
            BackendError::CorruptEnvelope => "envelope",
            BackendError::ProjectionMismatch => "projection",
            BackendError::StorageBusy(_) => "busy",
            BackendError::StorageFull(_) => "full",
            BackendError::ConcurrentStorageChange => "concurrent",
            BackendError::Storage(_) => "storage",
        }
    }
    assert_eq!(
        exhaustive_backend_kind(
            Error::source(&storage)
                .unwrap()
                .downcast_ref::<BackendError>()
                .unwrap()
        ),
        "schema-version"
    );

    let source_version = MigrationError::SourceVersionNotOne { found: 2 };
    assert!(matches!(
        source_version,
        MigrationError::SourceVersionNotOne { found: 2 }
    ));
    assert!(Error::source(&source_version).is_none());
    let migration_backend =
        MigrationError::from(BackendError::UnsupportedSchemaVersion { found: 3 });
    assert!(matches!(
        Error::source(&migration_backend),
        Some(source)
            if source.downcast_ref::<BackendError>()
                == Some(&BackendError::UnsupportedSchemaVersion { found: 3 })
    ));
}

#[test]
fn test_relationship_model_does_not_expose_storage_or_execution_authority() {
    fn assert_public_value<T: Clone + std::fmt::Debug + Eq + PartialEq>() {}

    assert_public_value::<BackendSchemaVersion>();
    assert_public_value::<RelationshipDefinitionId>();
    assert_public_value::<RelationshipDefinitionVersion>();
    assert_public_value::<RelationshipDefinitionKey>();
    assert_public_value::<RelationshipDirection>();
    assert_public_value::<RelationshipPolicy>();
    assert_public_value::<RelationshipEndpoint>();
    assert_public_value::<CreateRelationshipDefinition>();
    assert_public_value::<RelationshipDefinitionView>();
    assert_public_value::<RelationshipIdentity>();
    assert_public_value::<CreateRelationship>();
    assert_public_value::<DeleteRelationship>();
    assert_public_value::<RelationshipView>();
    assert_public_value::<RelationshipCursor>();
    assert_public_value::<ListRelationships>();
    assert_public_value::<RelationshipPage>();
    assert_public_value::<DirectRelationshipPredicate>();
    assert_public_value::<ProjectionQueryV1>();
    assert_public_value::<ProjectionPage>();
    assert_public_value::<RelationshipError>();
    assert_public_value::<MigrationError>();

    let relationship_source = include_str!("../src/relationship.rs");
    let projection_source = include_str!("../src/projection.rs");
    let library_source = include_str!("../src/lib.rs");
    for source in [relationship_source, projection_source, library_source] {
        for forbidden in [
            "rusqlite",
            "StoredRow",
            "Board",
            "StoredBoard",
            "ExecutionGraph",
            "Serialize",
            "Deserialize",
            "SystemTime",
            "timestamp:",
            "actor:",
            "Scheduler",
            "Executor",
            "LocalRequest",
            "LocalResponse",
        ] {
            assert!(
                !source.contains(forbidden),
                "public relationship model must not contain `{forbidden}`"
            );
        }
    }
    assert!(library_source.contains("mod relationship;"));
    assert!(library_source.contains("mod projection;"));
    assert!(library_source.contains("pub use relationship::"));
    assert!(library_source.contains("pub use projection::"));
    assert!(!library_source.contains("pub mod relationship;"));
    assert!(!library_source.contains("pub mod projection;"));
    assert!(!relationship_source.contains("impl fmt::Display for RelationshipCursor"));
    assert!(!relationship_source.contains("impl FromStr for RelationshipCursor"));
}
