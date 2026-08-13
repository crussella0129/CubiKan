//! Exact canonical provenance-association acceptance tests.

use frame_support::{dispatch::DispatchResult, sp_runtime::DispatchError};
use parity_scale_codec::{Decode, Encode};
use scale_info::{meta_type, PortableRegistry, Registry, TypeDef, TypeInfo};

use crate::{
    mock::{
        new_test_ext, CubiKan, RuntimeCall, RuntimeEvent, RuntimeOrigin, System, ALICE, CHARLIE,
        DEPLOYMENT_ID,
    },
    pallet::{ActiveAssociations, ActiveAssociationsOf, Error, Event, GlobalSequence, IntentUnits},
    types::{
        AssociationKey, AssociationSubject, DomainPayload, ExternalReference, IntentSpecies,
        IntentUnitId, IntentUnitState, Namespace, PhaseId, ReferenceScope, ReferenceValue,
        Workflow, WorkflowEdge, WorkflowId, MAX_ACTIVE_ASSOCIATIONS,
    },
};

const COMMAND_SCHEMA_VERSION: u16 = 1;
const EVENT_SCHEMA_VERSION: u16 = 1;

fn unit_id(value: u8) -> IntentUnitId {
    IntentUnitId::from_bytes([value; 16])
}

fn reference(value: &str) -> ExternalReference {
    ExternalReference::new(
        Namespace::try_from("git").unwrap(),
        ReferenceScope::try_from("commit").unwrap(),
        ReferenceValue::try_from(value).unwrap(),
    )
}

fn association(
    unit_id: IntentUnitId,
    subject: AssociationSubject,
    value: impl core::fmt::Display,
) -> AssociationKey {
    AssociationKey::new(unit_id, subject, reference(format!("ref-{value}").as_str()))
}

fn revision_three_state(id: IntentUnitId) -> IntentUnitState {
    let phases: Vec<_> = ["zero", "one", "two", "three"]
        .into_iter()
        .map(|value| PhaseId::try_from(value).unwrap())
        .collect();
    let edges = vec![
        WorkflowEdge::new(phases[0].clone(), phases[1].clone()),
        WorkflowEdge::new(phases[1].clone(), phases[2].clone()),
        WorkflowEdge::new(phases[2].clone(), phases[3].clone()),
    ];
    let workflow = Workflow::try_new(
        WorkflowId::try_from("provenance-test").unwrap(),
        &phases,
        phases[0].clone(),
        &edges,
        core::slice::from_ref(&phases[3]),
    )
    .unwrap();
    let mut state = IntentUnitState::new(
        id,
        reference("unit-origin"),
        IntentSpecies::try_from("test-unit").unwrap(),
        workflow,
    );
    for (expected, target) in phases.iter().enumerate().skip(1) {
        state.transition_to(target, (expected - 1) as u64).unwrap();
    }
    state
}

fn insert_unit(id: IntentUnitId) {
    IntentUnits::<crate::mock::Test>::insert(id, revision_three_state(id));
}

fn accepted_events() -> Vec<Event<crate::mock::Test>> {
    System::events()
        .into_iter()
        .filter_map(|record| match record.event {
            RuntimeEvent::CubiKan(event @ Event::Accepted { .. }) => Some(event),
            _ => None,
        })
        .collect()
}

fn domain_snapshot(units: &[IntentUnitId]) -> Vec<u8> {
    let state: Vec<_> = units
        .iter()
        .map(|id| {
            (
                *id,
                IntentUnits::<crate::mock::Test>::get(id),
                ActiveAssociations::<crate::mock::Test>::get(id),
            )
        })
        .collect();
    (
        state,
        GlobalSequence::<crate::mock::Test>::get(),
        System::events(),
    )
        .encode()
}

fn assert_atomic_failure(
    units: &[IntentUnitId],
    expected: DispatchError,
    call: impl FnOnce() -> DispatchResult,
) {
    let before = domain_snapshot(units);
    assert_eq!(call(), Err(expected));
    assert_eq!(domain_snapshot(units), before);
}

fn set_associations(id: IntentUnitId, values: Vec<AssociationKey>) {
    let bounded: ActiveAssociationsOf = values
        .try_into()
        .expect("test association set must respect the 128-entry bound");
    ActiveAssociations::<crate::mock::Test>::insert(id, bounded);
}

#[test]
fn test_provenance_subjects_many_to_many_and_revision_exact() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let first = unit_id(1);
        let second = unit_id(2);
        insert_unit(first);
        insert_unit(second);
        let first_lifecycle = CubiKan::intent_units(first).unwrap().encode();
        let second_lifecycle = CubiKan::intent_units(second).unwrap().encode();
        let shared = reference("shared");
        let values = [
            AssociationKey::new(first, AssociationSubject::WholeUnit, shared.clone()),
            AssociationKey::new(first, AssociationSubject::Revision(0), shared.clone()),
            association(first, AssociationSubject::Revision(1), "interior"),
            association(first, AssociationSubject::Revision(3), "current"),
            association(first, AssociationSubject::Revision(1), "many"),
            AssociationKey::new(second, AssociationSubject::Revision(0), shared),
        ];

        for (index, selected) in values.iter().enumerate() {
            frame_support::assert_ok!(CubiKan::record_association(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                selected.clone(),
            ));
            assert_eq!(CubiKan::global_sequence(), Some((index + 1) as u64));
            assert_eq!(accepted_events().len(), index + 1);
            assert_eq!(
                accepted_events().last(),
                Some(&Event::Accepted {
                    deployment_id: DEPLOYMENT_ID,
                    event_schema_version: EVENT_SCHEMA_VERSION,
                    global_sequence: (index + 1) as u64,
                    signer: ALICE,
                    payload: DomainPayload::AssociationRecorded(selected.clone()),
                })
            );
        }

        assert_eq!(CubiKan::active_associations(first).as_slice(), &values[..5]);
        assert_eq!(
            CubiKan::active_associations(second).as_slice(),
            &values[5..]
        );
        assert_eq!(
            CubiKan::intent_units(first).unwrap().encode(),
            first_lifecycle
        );
        assert_eq!(
            CubiKan::intent_units(second).unwrap().encode(),
            second_lifecycle
        );
    });
}

#[test]
fn test_provenance_record_precedence_is_typed_and_atomic() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let id = unit_id(10);
        insert_unit(id);
        let valid = association(id, AssociationSubject::WholeUnit, "valid");
        let missing = association(
            unit_id(99),
            AssociationSubject::Revision(u64::MAX),
            "missing",
        );

        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::UnsupportedCommandSchemaVersion.into(),
            || CubiKan::record_association(RuntimeOrigin::signed(CHARLIE), 2, missing.clone()),
        );
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::UnauthorizedSubmitter.into(),
            || {
                CubiKan::record_association(
                    RuntimeOrigin::signed(CHARLIE),
                    COMMAND_SCHEMA_VERSION,
                    missing.clone(),
                )
            },
        );
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::AssociationUnitNotFound.into(),
            || {
                CubiKan::record_association(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    missing.clone(),
                )
            },
        );
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::AssociationRevisionNotFound.into(),
            || {
                CubiKan::record_association(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    association(id, AssociationSubject::Revision(4), "future"),
                )
            },
        );

        let mut malformed = valid.encode();
        *malformed
            .last_mut()
            .expect("encoded reference has a value byte") = 0;
        let before = domain_snapshot(&[id]);
        assert!(AssociationKey::decode(&mut malformed.as_slice()).is_err());
        assert_eq!(domain_snapshot(&[id]), before);

        let full: Vec<_> = (0..MAX_ACTIVE_ASSOCIATIONS)
            .map(|index| association(id, AssociationSubject::WholeUnit, index))
            .collect();
        set_associations(id, full.clone());
        GlobalSequence::<crate::mock::Test>::put(u64::MAX);
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::AssociationAlreadyExists.into(),
            || {
                CubiKan::record_association(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    full[0].clone(),
                )
            },
        );
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::AssociationCapacityExceeded.into(),
            || {
                CubiKan::record_association(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    association(id, AssociationSubject::WholeUnit, "129"),
                )
            },
        );

        set_associations(id, full[..127].to_vec());
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::GlobalSequenceExhausted.into(),
            || {
                CubiKan::record_association(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    association(id, AssociationSubject::WholeUnit, "global"),
                )
            },
        );

        GlobalSequence::<crate::mock::Test>::kill();
        set_associations(id, full[..127].to_vec());
        let boundary = association(id, AssociationSubject::WholeUnit, "boundary-128");
        frame_support::assert_ok!(CubiKan::record_association(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            boundary.clone(),
        ));
        assert_eq!(
            CubiKan::active_associations(id).len(),
            MAX_ACTIVE_ASSOCIATIONS
        );
        assert_eq!(CubiKan::active_associations(id).last(), Some(&boundary));
    });
}

#[test]
fn test_provenance_revoke_is_ordered_append_only_and_nonreplacement() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let id = unit_id(20);
        insert_unit(id);
        let target = association(id, AssociationSubject::Revision(1), "old");
        let neighbor = association(id, AssociationSubject::WholeUnit, "neighbor");
        for selected in [&target, &neighbor] {
            frame_support::assert_ok!(CubiKan::record_association(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                selected.clone(),
            ));
        }
        let missing = association(unit_id(99), AssociationSubject::Revision(u64::MAX), "missing");
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::UnsupportedCommandSchemaVersion.into(),
            || CubiKan::revoke_association(RuntimeOrigin::signed(CHARLIE), 2, missing.clone()),
        );
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::UnauthorizedSubmitter.into(),
            || {
                CubiKan::revoke_association(
                    RuntimeOrigin::signed(CHARLIE),
                    COMMAND_SCHEMA_VERSION,
                    missing.clone(),
                )
            },
        );
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::AssociationUnitNotFound.into(),
            || {
                CubiKan::revoke_association(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    missing.clone(),
                )
            },
        );
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::AssociationRevisionNotFound.into(),
            || {
                CubiKan::revoke_association(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    association(id, AssociationSubject::Revision(4), "future"),
                )
            },
        );

        let mut malformed = target.encode();
        *malformed.last_mut().unwrap() = 0;
        let before = domain_snapshot(&[id]);
        assert!(AssociationKey::decode(&mut malformed.as_slice()).is_err());
        assert_eq!(domain_snapshot(&[id]), before);

        GlobalSequence::<crate::mock::Test>::put(u64::MAX);
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::AssociationNotFound.into(),
            || {
                CubiKan::revoke_association(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    association(id, AssociationSubject::WholeUnit, "not-active"),
                )
            },
        );
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::GlobalSequenceExhausted.into(),
            || {
                CubiKan::revoke_association(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    target.clone(),
                )
            },
        );

        GlobalSequence::<crate::mock::Test>::put(2);
        let lifecycle_before = CubiKan::intent_units(id).unwrap().encode();
        frame_support::assert_ok!(CubiKan::revoke_association(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            target.clone(),
        ));
        assert_eq!(
            CubiKan::active_associations(id).as_slice(),
            core::slice::from_ref(&neighbor)
        );
        assert_eq!(CubiKan::intent_units(id).unwrap().encode(), lifecycle_before);
        assert_eq!(
            accepted_events().last(),
            Some(&Event::Accepted {
                deployment_id: DEPLOYMENT_ID,
                event_schema_version: EVENT_SCHEMA_VERSION,
                global_sequence: 3,
                signer: ALICE,
                payload: DomainPayload::AssociationRevoked(target.clone()),
            })
        );
        assert_atomic_failure(
            &[id],
            Error::<crate::mock::Test>::AssociationNotFound.into(),
            || {
                CubiKan::revoke_association(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    target.clone(),
                )
            },
        );

        let correction = association(id, AssociationSubject::Revision(1), "corrected");
        assert!(!CubiKan::active_associations(id).contains(&target));
        assert!(!CubiKan::active_associations(id).contains(&correction));
        frame_support::assert_ok!(CubiKan::record_association(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            correction.clone(),
        ));
        assert_eq!(
            CubiKan::active_associations(id).as_slice(),
            &[neighbor, correction.clone()]
        );
        assert!(accepted_events().iter().any(|event| matches!(
            event,
            Event::Accepted { payload: DomainPayload::AssociationRecorded(value), .. } if value == &target
        )));
        assert!(accepted_events().iter().any(|event| matches!(
            event,
            Event::Accepted { payload: DomainPayload::AssociationRevoked(value), .. } if value == &target
        )));
        assert!(matches!(
            accepted_events().last(),
            Some(Event::Accepted { payload: DomainPayload::AssociationRecorded(value), .. }) if value == &correction
        ));
    });
}

fn metadata_tokens<T: TypeInfo + 'static>() -> Vec<String> {
    let mut registry = Registry::new();
    registry.register_type(&meta_type::<T>());
    let portable = PortableRegistry::from(registry);
    let mut tokens = Vec::new();
    for entry in portable.types {
        tokens.extend(entry.ty.path.segments);
        match entry.ty.type_def {
            TypeDef::Composite(composite) => {
                for field in composite.fields {
                    tokens.extend(field.name);
                    tokens.extend(field.type_name);
                }
            }
            TypeDef::Variant(variant) => {
                for selected in variant.variants {
                    tokens.push(selected.name);
                    for field in selected.fields {
                        tokens.extend(field.name);
                        tokens.extend(field.type_name);
                    }
                }
            }
            _ => {}
        }
    }
    tokens
}

fn assert_metadata_type_inventory<T: TypeInfo + 'static>(
    expected_type: &str,
    tokens: &mut Vec<String>,
) {
    let selected = metadata_tokens::<T>();
    assert!(
        selected.iter().any(|token| token == expected_type),
        "metadata inventory omitted `{expected_type}`"
    );
    tokens.extend(selected);
}

#[test]
fn test_runtime_event_surfaces_exclude_attribution_and_secrets() {
    let storage_inventory = [
        "IntentUnits",
        "RelationshipDefinitions",
        "RelationshipEdges",
        "ActiveAssociations",
        "AuthorizedSubmitters",
        "GlobalSequence",
        "DeploymentAnchor",
        "PalletStorageVersion",
        "EventSchemaVersion",
    ];
    let mut tokens = storage_inventory
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_metadata_type_inventory::<RuntimeCall>("RuntimeCall", &mut tokens);
    assert_metadata_type_inventory::<RuntimeEvent>("RuntimeEvent", &mut tokens);
    assert_metadata_type_inventory::<crate::pallet::Call<crate::mock::Test>>("Call", &mut tokens);
    assert_metadata_type_inventory::<Event<crate::mock::Test>>("Event", &mut tokens);
    assert_metadata_type_inventory::<Error<crate::mock::Test>>("Error", &mut tokens);
    assert_metadata_type_inventory::<DomainPayload>("DomainPayload", &mut tokens);
    assert_metadata_type_inventory::<IntentUnitState>("IntentUnitState", &mut tokens);
    assert_metadata_type_inventory::<AssociationKey>("AssociationKey", &mut tokens);

    let normalized = tokens
        .iter()
        .map(|token| token.to_ascii_lowercase().replace(['-', ' '], "_"))
        .collect::<Vec<_>>();
    let forbidden_exact = [
        "owner",
        "ownership",
        "author",
        "authorship",
        "credential",
        "password",
        "secret",
        "prompt",
        "transcript",
        "source_body",
        "source_bodies",
        "private_locator",
        "provider_secret",
        "production_id",
        "production_identifier",
    ];
    for forbidden in forbidden_exact {
        assert!(
            normalized.iter().all(|token| token != forbidden),
            "forbidden canonical metadata token `{forbidden}` found in {normalized:?}"
        );
    }

    assert!(normalized
        .iter()
        .any(|token| token == "authorizedsubmitters"));
    assert!(normalized
        .iter()
        .any(|token| token == "unauthorizedsubmitter"));
    assert!(normalized.iter().any(|token| token == "signer"));
    assert!(normalized.iter().any(|token| token == "reference"));
    assert!(normalized
        .iter()
        .any(|token| token == "relationshipsourcenotfound"));
    assert!(normalized.iter().any(|token| token == "source_id"));
    assert!(normalized.iter().any(|token| token == "activeassociations"));
}
