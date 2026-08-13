//! Exact relationship-definition and edge acceptance tests.

use frame_support::{dispatch::DispatchResult, sp_runtime::DispatchError};
use parity_scale_codec::Encode;

use crate::{
    mock::{
        new_test_ext, CubiKan, RuntimeEvent, RuntimeOrigin, System, ALICE, CHARLIE, DEPLOYMENT_ID,
    },
    pallet::{
        Error, Event, GlobalSequence, IntentUnits, RelationshipDefinitions, RelationshipEdges,
        RelationshipEdgesOf,
    },
    types::{
        DefinitionKey, DefinitionVersion, DomainPayload, ExternalReference, IntentSpecies,
        IntentUnitId, IntentUnitState, Namespace, PhaseId, ReferenceScope, ReferenceValue,
        RelationshipDefinition, RelationshipKey, RelationshipPolicy, Workflow, WorkflowId,
    },
};

const COMMAND_SCHEMA_VERSION: u16 = 1;
const EVENT_SCHEMA_VERSION: u16 = 1;

fn unit_id(value: u8) -> IntentUnitId {
    IntentUnitId::from_bytes([value; 16])
}

fn species(value: &str) -> IntentSpecies {
    IntentSpecies::try_from(value).expect("test species must be valid")
}

fn definition_key(id: &str, version: u64) -> DefinitionKey {
    DefinitionKey::new(
        Namespace::try_from(id).expect("test definition id must be valid"),
        DefinitionVersion::try_new(version).expect("test definition version must be positive"),
    )
}

fn definition(
    id: &str,
    version: u64,
    source_species: Option<&str>,
    target_species: Option<&str>,
    self_policy: RelationshipPolicy,
    cycle_policy: RelationshipPolicy,
) -> RelationshipDefinition {
    RelationshipDefinition::new(
        definition_key(id, version),
        source_species.map(species),
        target_species.map(species),
        self_policy,
        cycle_policy,
    )
}

fn endpoint_state(id: IntentUnitId, selected_species: &str) -> IntentUnitState {
    let phase = PhaseId::try_from("ready").unwrap();
    IntentUnitState::new(
        id,
        ExternalReference::new(
            Namespace::try_from("git").unwrap(),
            ReferenceScope::try_from("repository").unwrap(),
            ReferenceValue::try_from(format!("unit-{id:?}").as_str()).unwrap(),
        ),
        species(selected_species),
        Workflow::try_new(
            WorkflowId::try_from("relationship-test").unwrap(),
            core::slice::from_ref(&phase),
            phase.clone(),
            &[],
            core::slice::from_ref(&phase),
        )
        .unwrap(),
    )
}

fn insert_endpoint(id: IntentUnitId, selected_species: &str) {
    IntentUnits::<crate::mock::Test>::insert(id, endpoint_state(id, selected_species));
}

fn insert_definition(selected: &RelationshipDefinition) {
    RelationshipDefinitions::<crate::mock::Test>::insert(selected.key(), selected);
}

fn set_edges(key: &DefinitionKey, edges: Vec<RelationshipKey>) {
    let bounded: RelationshipEdgesOf = edges
        .try_into()
        .expect("test relationship set must respect the 128-edge bound");
    RelationshipEdges::<crate::mock::Test>::insert(key, bounded);
}

fn pallet_events() -> Vec<Event<crate::mock::Test>> {
    System::events()
        .into_iter()
        .filter_map(|record| match record.event {
            RuntimeEvent::CubiKan(event) => Some(event),
            _ => None,
        })
        .collect()
}

fn accepted_events() -> Vec<Event<crate::mock::Test>> {
    pallet_events()
        .into_iter()
        .filter(|event| matches!(event, Event::Accepted { .. }))
        .collect()
}

fn domain_snapshot(keys: &[DefinitionKey], units: &[IntentUnitId]) -> Vec<u8> {
    let definitions: Vec<_> = keys
        .iter()
        .map(|key| {
            (
                key.clone(),
                RelationshipDefinitions::<crate::mock::Test>::get(key),
                RelationshipEdges::<crate::mock::Test>::get(key),
            )
        })
        .collect();
    let endpoints: Vec<_> = units
        .iter()
        .map(|id| (*id, IntentUnits::<crate::mock::Test>::get(id)))
        .collect();
    (
        definitions,
        endpoints,
        GlobalSequence::<crate::mock::Test>::get(),
        System::events(),
    )
        .encode()
}

fn assert_atomic_failure(
    keys: &[DefinitionKey],
    units: &[IntentUnitId],
    expected: DispatchError,
    call: impl FnOnce() -> DispatchResult,
) {
    let before = domain_snapshot(keys, units);
    assert_eq!(call(), Err(expected));
    assert_eq!(domain_snapshot(keys, units), before);
}

fn edge(key: &DefinitionKey, source: u8, target: u8) -> RelationshipKey {
    RelationshipKey::new(key.clone(), unit_id(source), unit_id(target))
}

fn graph_has_cycle(edges: &[RelationshipKey]) -> bool {
    edges.iter().any(|edge| {
        let mut reachable = vec![edge.target_id()];
        for _ in 0..=edges.len() {
            if reachable.contains(&edge.source_id()) {
                return true;
            }
            let before = reachable.len();
            for candidate in edges {
                if reachable.contains(&candidate.source_id())
                    && !reachable.contains(&candidate.target_id())
                {
                    reachable.push(candidate.target_id());
                }
            }
            if reachable.len() == before {
                return false;
            }
        }
        reachable.contains(&edge.source_id())
    })
}

#[test]
fn test_definition_and_edge_creation_preserve_endpoint_lifecycle() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let source = unit_id(1);
        let target = unit_id(2);
        insert_endpoint(source, "source-kind");
        insert_endpoint(target, "target-kind");
        let endpoints_before =
            (CubiKan::intent_units(source), CubiKan::intent_units(target)).encode();

        let version_seven = definition(
            "depends-on",
            7,
            Some("source-kind"),
            Some("target-kind"),
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        );
        let version_two = definition(
            "depends-on",
            2,
            Some("source-kind"),
            Some("target-kind"),
            RelationshipPolicy::Reject,
            RelationshipPolicy::Allow,
        );
        let edge_seven = RelationshipKey::new(version_seven.key().clone(), source, target);
        let edge_two = RelationshipKey::new(version_two.key().clone(), source, target);

        frame_support::assert_ok!(CubiKan::create_relationship_definition(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            version_seven.clone(),
        ));
        frame_support::assert_ok!(CubiKan::create_relationship_definition(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            version_two.clone(),
        ));
        frame_support::assert_ok!(CubiKan::create_relationship(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            edge_seven.clone(),
        ));
        frame_support::assert_ok!(CubiKan::create_relationship(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            edge_two.clone(),
        ));

        assert_eq!(
            CubiKan::relationship_definitions(version_seven.key()),
            Some(version_seven.clone())
        );
        assert_eq!(
            CubiKan::relationship_definitions(version_two.key()),
            Some(version_two.clone())
        );
        assert_eq!(
            CubiKan::relationship_edges(version_seven.key()).into_inner(),
            vec![edge_seven.clone()]
        );
        assert_eq!(
            CubiKan::relationship_edges(version_two.key()).into_inner(),
            vec![edge_two.clone()]
        );
        assert_eq!(
            (CubiKan::intent_units(source), CubiKan::intent_units(target)).encode(),
            endpoints_before
        );
        assert_eq!(CubiKan::global_sequence(), Some(4));
        assert_eq!(
            accepted_events(),
            vec![
                Event::Accepted {
                    deployment_id: DEPLOYMENT_ID,
                    event_schema_version: EVENT_SCHEMA_VERSION,
                    global_sequence: 1,
                    signer: ALICE,
                    payload: DomainPayload::RelationshipDefinitionCreated(version_seven),
                },
                Event::Accepted {
                    deployment_id: DEPLOYMENT_ID,
                    event_schema_version: EVENT_SCHEMA_VERSION,
                    global_sequence: 2,
                    signer: ALICE,
                    payload: DomainPayload::RelationshipDefinitionCreated(version_two),
                },
                Event::Accepted {
                    deployment_id: DEPLOYMENT_ID,
                    event_schema_version: EVENT_SCHEMA_VERSION,
                    global_sequence: 3,
                    signer: ALICE,
                    payload: DomainPayload::RelationshipCreated(edge_seven),
                },
                Event::Accepted {
                    deployment_id: DEPLOYMENT_ID,
                    event_schema_version: EVENT_SCHEMA_VERSION,
                    global_sequence: 4,
                    signer: ALICE,
                    payload: DomainPayload::RelationshipCreated(edge_two),
                },
            ]
        );
    });
}

#[test]
fn test_definition_creation_precedence_is_typed_and_atomic() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let duplicate = definition(
            "blocks",
            3,
            None,
            None,
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        );
        let novel = definition(
            "blocks",
            9,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Allow,
        );
        insert_definition(&duplicate);
        let keys = [duplicate.key().clone(), novel.key().clone()];

        assert_atomic_failure(
            &keys,
            &[],
            Error::<crate::mock::Test>::UnsupportedCommandSchemaVersion.into(),
            || CubiKan::create_relationship_definition(RuntimeOrigin::root(), 2, duplicate.clone()),
        );
        assert_atomic_failure(&keys, &[], DispatchError::BadOrigin, || {
            CubiKan::create_relationship_definition(
                RuntimeOrigin::root(),
                COMMAND_SCHEMA_VERSION,
                duplicate.clone(),
            )
        });
        assert_atomic_failure(
            &keys,
            &[],
            Error::<crate::mock::Test>::UnauthorizedSubmitter.into(),
            || {
                CubiKan::create_relationship_definition(
                    RuntimeOrigin::signed(CHARLIE),
                    COMMAND_SCHEMA_VERSION,
                    duplicate.clone(),
                )
            },
        );
        GlobalSequence::<crate::mock::Test>::put(u64::MAX);
        assert_atomic_failure(
            &keys,
            &[],
            Error::<crate::mock::Test>::RelationshipDefinitionAlreadyExists.into(),
            || {
                CubiKan::create_relationship_definition(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    duplicate.clone(),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &[],
            Error::<crate::mock::Test>::GlobalSequenceExhausted.into(),
            || {
                CubiKan::create_relationship_definition(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    novel,
                )
            },
        );
    });
}

#[test]
fn test_edge_creation_precedence_bounds_and_cycles_are_exact() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let strict = definition(
            "strict",
            1,
            Some("source-kind"),
            Some("target-kind"),
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        );
        let open = definition(
            "open",
            1,
            None,
            None,
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        );
        let cycle = definition(
            "cycle",
            1,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Reject,
        );
        let global = definition(
            "global",
            1,
            None,
            None,
            RelationshipPolicy::Allow,
            RelationshipPolicy::Allow,
        );
        for selected in [&strict, &open, &cycle, &global] {
            insert_definition(selected);
        }
        insert_endpoint(unit_id(1), "source-kind");
        insert_endpoint(unit_id(2), "target-kind");
        insert_endpoint(unit_id(3), "wrong-kind");
        for id in 4..=132 {
            insert_endpoint(unit_id(id), "open-kind");
        }
        let keys = [
            strict.key().clone(),
            open.key().clone(),
            cycle.key().clone(),
            global.key().clone(),
            definition_key("missing", 1),
        ];
        let units: Vec<_> = (1..=132).map(unit_id).collect();

        let missing_definition = edge(&definition_key("missing", 1), 200, 201);
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::UnsupportedCommandSchemaVersion.into(),
            || CubiKan::create_relationship(RuntimeOrigin::root(), 2, missing_definition.clone()),
        );
        assert_atomic_failure(&keys, &units, DispatchError::BadOrigin, || {
            CubiKan::create_relationship(
                RuntimeOrigin::root(),
                COMMAND_SCHEMA_VERSION,
                missing_definition.clone(),
            )
        });
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::UnauthorizedSubmitter.into(),
            || {
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(CHARLIE),
                    COMMAND_SCHEMA_VERSION,
                    missing_definition.clone(),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipDefinitionNotFound.into(),
            || {
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    missing_definition.clone(),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipSourceNotFound.into(),
            || {
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(strict.key(), 200, 201),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipTargetNotFound.into(),
            || {
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(strict.key(), 1, 201),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipSourceSpeciesMismatch.into(),
            || {
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(strict.key(), 3, 3),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipTargetSpeciesMismatch.into(),
            || {
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(strict.key(), 1, 3),
                )
            },
        );
        let rejected_self = edge(open.key(), 4, 4);
        set_edges(open.key(), vec![rejected_self.clone()]);
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipSelfEdgeRejected.into(),
            || {
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    rejected_self,
                )
            },
        );

        let mut capacity_edges: Vec<_> = (4..=130)
            .map(|source| edge(open.key(), source, source + 1))
            .collect();
        assert_eq!(capacity_edges.len(), 127);
        set_edges(open.key(), capacity_edges.clone());
        frame_support::assert_ok!(CubiKan::create_relationship(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            edge(open.key(), 131, 132),
        ));
        capacity_edges.push(edge(open.key(), 131, 132));
        assert_eq!(CubiKan::relationship_edges(open.key()).len(), 128);
        assert!(!graph_has_cycle(&capacity_edges));

        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipAlreadyExists.into(),
            || {
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(open.key(), 4, 5),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipCapacityExceeded.into(),
            || {
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(open.key(), 132, 4),
                )
            },
        );

        set_edges(cycle.key(), vec![edge(cycle.key(), 4, 5)]);
        GlobalSequence::<crate::mock::Test>::put(u64::MAX);
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipCycleRejected.into(),
            || {
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(cycle.key(), 5, 4),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::GlobalSequenceExhausted.into(),
            || {
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(global.key(), 4, 5),
                )
            },
        );
    });
}

#[test]
fn test_opposite_cycle_closures_accept_at_most_one() {
    for candidates in [[(2, 3), (3, 1)], [(3, 1), (2, 3)]] {
        new_test_ext(vec![ALICE]).execute_with(|| {
            let definition = definition(
                "acyclic",
                1,
                None,
                None,
                RelationshipPolicy::Reject,
                RelationshipPolicy::Reject,
            );
            insert_definition(&definition);
            for id in 1..=3 {
                insert_endpoint(unit_id(id), "node");
            }
            set_edges(definition.key(), vec![edge(definition.key(), 1, 2)]);
            let endpoints_before = (1..=3)
                .map(|id| CubiKan::intent_units(unit_id(id)))
                .collect::<Vec<_>>()
                .encode();

            frame_support::assert_ok!(CubiKan::create_relationship(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                edge(definition.key(), candidates[0].0, candidates[0].1),
            ));
            frame_support::assert_noop!(
                CubiKan::create_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(definition.key(), candidates[1].0, candidates[1].1),
                ),
                Error::<crate::mock::Test>::RelationshipCycleRejected
            );

            let stored = CubiKan::relationship_edges(definition.key()).into_inner();
            assert_eq!(stored.len(), 2);
            assert!(!graph_has_cycle(&stored));
            assert_eq!(CubiKan::global_sequence(), Some(1));
            assert_eq!(accepted_events().len(), 1);
            assert_eq!(
                (1..=3)
                    .map(|id| CubiKan::intent_units(unit_id(id)))
                    .collect::<Vec<_>>()
                    .encode(),
                endpoints_before
            );
        });
    }
}

#[test]
fn test_relationship_delete_is_exact_noncascading_and_ordered() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let strict = definition(
            "deletion",
            8,
            Some("source-kind"),
            Some("target-kind"),
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        );
        insert_definition(&strict);
        insert_endpoint(unit_id(1), "source-kind");
        insert_endpoint(unit_id(2), "target-kind");
        insert_endpoint(unit_id(3), "wrong-kind");
        insert_endpoint(unit_id(4), "target-kind");
        insert_endpoint(unit_id(5), "source-kind");
        let named = edge(strict.key(), 1, 2);
        let neighbor_out = edge(strict.key(), 1, 4);
        let neighbor_in = edge(strict.key(), 5, 2);
        set_edges(
            strict.key(),
            vec![named.clone(), neighbor_out.clone(), neighbor_in.clone()],
        );
        let missing_key = definition_key("missing-delete", 1);
        let keys = [strict.key().clone(), missing_key.clone()];
        let units: Vec<_> = (1..=5).map(unit_id).collect();

        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::UnsupportedCommandSchemaVersion.into(),
            || CubiKan::delete_relationship(RuntimeOrigin::root(), 2, named.clone()),
        );
        assert_atomic_failure(&keys, &units, DispatchError::BadOrigin, || {
            CubiKan::delete_relationship(
                RuntimeOrigin::root(),
                COMMAND_SCHEMA_VERSION,
                named.clone(),
            )
        });
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::UnauthorizedSubmitter.into(),
            || {
                CubiKan::delete_relationship(
                    RuntimeOrigin::signed(CHARLIE),
                    COMMAND_SCHEMA_VERSION,
                    named.clone(),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipDefinitionNotFound.into(),
            || {
                CubiKan::delete_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(&missing_key, 200, 201),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipSourceNotFound.into(),
            || {
                CubiKan::delete_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(strict.key(), 200, 201),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipTargetNotFound.into(),
            || {
                CubiKan::delete_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(strict.key(), 1, 201),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipSourceSpeciesMismatch.into(),
            || {
                CubiKan::delete_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(strict.key(), 3, 2),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipTargetSpeciesMismatch.into(),
            || {
                CubiKan::delete_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(strict.key(), 1, 3),
                )
            },
        );
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::RelationshipNotFound.into(),
            || {
                CubiKan::delete_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    edge(strict.key(), 5, 4),
                )
            },
        );
        GlobalSequence::<crate::mock::Test>::put(u64::MAX);
        assert_atomic_failure(
            &keys,
            &units,
            Error::<crate::mock::Test>::GlobalSequenceExhausted.into(),
            || {
                CubiKan::delete_relationship(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    named.clone(),
                )
            },
        );

        GlobalSequence::<crate::mock::Test>::put(10);
        let endpoints_before = units
            .iter()
            .map(|id| CubiKan::intent_units(*id))
            .collect::<Vec<_>>()
            .encode();
        let definition_before = CubiKan::relationship_definitions(strict.key()).encode();
        let event_count = accepted_events().len();
        frame_support::assert_ok!(CubiKan::delete_relationship(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            named.clone(),
        ));
        assert_eq!(
            CubiKan::relationship_edges(strict.key()).into_inner(),
            vec![neighbor_out.clone(), neighbor_in.clone()]
        );
        assert_eq!(
            CubiKan::relationship_definitions(strict.key()).encode(),
            definition_before
        );
        assert_eq!(
            units
                .iter()
                .map(|id| CubiKan::intent_units(*id))
                .collect::<Vec<_>>()
                .encode(),
            endpoints_before
        );
        assert_eq!(CubiKan::global_sequence(), Some(11));
        assert_eq!(
            accepted_events()[event_count],
            Event::Accepted {
                deployment_id: DEPLOYMENT_ID,
                event_schema_version: EVENT_SCHEMA_VERSION,
                global_sequence: 11,
                signer: ALICE,
                payload: DomainPayload::RelationshipDeleted(named.clone()),
            }
        );

        frame_support::assert_ok!(CubiKan::create_relationship(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            named.clone(),
        ));
        assert_eq!(
            CubiKan::relationship_edges(strict.key()).into_inner(),
            vec![neighbor_out, neighbor_in, named.clone()]
        );
        assert_eq!(CubiKan::global_sequence(), Some(12));
        assert_eq!(
            accepted_events()[event_count + 1],
            Event::Accepted {
                deployment_id: DEPLOYMENT_ID,
                event_schema_version: EVENT_SCHEMA_VERSION,
                global_sequence: 12,
                signer: ALICE,
                payload: DomainPayload::RelationshipCreated(named),
            }
        );
    });
}
