use frame_support::{assert_noop, assert_ok, sp_runtime::DispatchError};
use parity_scale_codec::Encode;

use crate::{
    mock::{
        new_test_ext, AccountId, CubiKan, RuntimeEvent, RuntimeOrigin, System, ALICE, BOB, CHARLIE,
        DEPLOYMENT_ID,
    },
    pallet::{AuthorizedSubmitterInput, Error, Event, GlobalSequence, IntentUnits},
    types::{
        DomainPayload, ExternalReference, IntentSpecies, IntentUnitId, IntentUnitStatus, Namespace,
        PhaseId, ReferenceScope, ReferenceValue, Workflow, WorkflowEdge, WorkflowId,
    },
};

const COMMAND_SCHEMA_VERSION: u16 = 1;
const EVENT_SCHEMA_VERSION: u16 = 1;

fn phase(value: &str) -> PhaseId {
    PhaseId::try_from(value).expect("test phase must be valid")
}

fn workflow(
    phases: &[&str],
    initial: &str,
    edges: &[(&str, &str)],
    completion_phases: &[&str],
) -> Workflow {
    let phases: Vec<_> = phases.iter().map(|value| phase(value)).collect();
    let edges: Vec<_> = edges
        .iter()
        .map(|(from, to)| WorkflowEdge::new(phase(from), phase(to)))
        .collect();
    let completion_phases: Vec<_> = completion_phases.iter().map(|value| phase(value)).collect();
    Workflow::try_new(
        WorkflowId::try_from("workflow-v1").unwrap(),
        &phases,
        phase(initial),
        &edges,
        &completion_phases,
    )
    .expect("test workflow must be valid")
}

fn standard_workflow() -> Workflow {
    workflow(
        &["queued", "doing", "done"],
        "queued",
        &[("queued", "doing"), ("doing", "queued"), ("doing", "done")],
        &["done"],
    )
}

fn origin_reference() -> ExternalReference {
    ExternalReference::new(
        Namespace::try_from("git").unwrap(),
        ReferenceScope::try_from("repository").unwrap(),
        ReferenceValue::try_from("0123456789abcdef").unwrap(),
    )
}

fn maximal_create_values() -> (ExternalReference, IntentSpecies, Workflow) {
    let namespace = format!("a{}", "z".repeat(63));
    let origin = ExternalReference::new(
        Namespace::try_from(namespace.as_str()).unwrap(),
        ReferenceScope::try_from("s".repeat(256).as_str()).unwrap(),
        ReferenceValue::try_from("v".repeat(256).as_str()).unwrap(),
    );
    let species = IntentSpecies::try_from("x".repeat(256).as_str()).unwrap();
    let phases: Vec<_> = (0..32)
        .map(|index| PhaseId::try_from(format!("{index:02}{}", "p".repeat(254)).as_str()).unwrap())
        .collect();
    let edges: Vec<_> = (0..128)
        .map(|index| WorkflowEdge::new(phases[index / 32].clone(), phases[index % 32].clone()))
        .collect();
    let workflow = Workflow::try_new(
        WorkflowId::try_from("w".repeat(256).as_str()).unwrap(),
        &phases,
        phases[0].clone(),
        &edges,
        &phases,
    )
    .expect("maximal workflow fixture must be valid");
    (origin, species, workflow)
}

fn species() -> IntentSpecies {
    IntentSpecies::try_from("engineering-task").unwrap()
}

fn unit_id(byte: u8) -> IntentUnitId {
    IntentUnitId::from_bytes([byte; 16])
}

fn create(id: IntentUnitId, signer: AccountId, selected_workflow: Workflow) {
    assert_ok!(CubiKan::create_unit(
        RuntimeOrigin::signed(signer),
        COMMAND_SCHEMA_VERSION,
        id,
        origin_reference(),
        species(),
        selected_workflow,
    ));
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

fn input_accounts(accounts: Vec<AccountId>) -> AuthorizedSubmitterInput<crate::mock::Test> {
    accounts
        .try_into()
        .expect("test input has at most the decodable maximum of 17")
}

fn state_bytes(id: IntentUnitId) -> Vec<u8> {
    (
        IntentUnits::<crate::mock::Test>::get(id),
        CubiKan::global_sequence(),
    )
        .encode()
}

#[test]
fn test_create_stores_revision_zero_and_one_complete_event() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let id = unit_id(1);
        let (unit_origin, unit_species, unit_workflow) = maximal_create_values();

        assert_ok!(CubiKan::create_unit(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            id,
            unit_origin.clone(),
            unit_species.clone(),
            unit_workflow.clone(),
        ));

        let stored = CubiKan::intent_units(id).expect("created unit must exist");
        assert_eq!(stored.id(), id);
        assert_eq!(stored.origin(), &unit_origin);
        assert_eq!(stored.species(), &unit_species);
        assert_eq!(stored.workflow(), &unit_workflow);
        assert_eq!(stored.phase(), unit_workflow.initial_phase());
        assert_eq!(stored.status(), IntentUnitStatus::Active);
        assert!(stored.history().is_empty());
        assert_eq!(stored.revision(), 0);
        assert_eq!(CubiKan::global_sequence(), Some(1));

        assert_eq!(
            accepted_events(),
            vec![Event::Accepted {
                deployment_id: DEPLOYMENT_ID,
                event_schema_version: EVENT_SCHEMA_VERSION,
                global_sequence: 1,
                signer: ALICE,
                payload: DomainPayload::UnitCreated(crate::types::CreateUnitPayload {
                    command_schema_version: COMMAND_SCHEMA_VERSION,
                    id,
                    origin: unit_origin,
                    species: unit_species,
                    workflow: unit_workflow,
                }),
            }]
        );
    });
}

#[test]
fn test_decoded_lifecycle_rejections_are_typed_and_domain_atomic() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let id = unit_id(2);
        create(id, ALICE, standard_workflow());

        assert_noop!(
            CubiKan::create_unit(
                RuntimeOrigin::signed(CHARLIE),
                2,
                id,
                origin_reference(),
                species(),
                standard_workflow(),
            ),
            Error::<crate::mock::Test>::UnsupportedCommandSchemaVersion
        );
        assert_noop!(
            CubiKan::create_unit(
                RuntimeOrigin::signed(CHARLIE),
                COMMAND_SCHEMA_VERSION,
                id,
                origin_reference(),
                species(),
                standard_workflow(),
            ),
            Error::<crate::mock::Test>::UnauthorizedSubmitter
        );
        assert_noop!(
            CubiKan::create_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                id,
                origin_reference(),
                species(),
                standard_workflow(),
            ),
            Error::<crate::mock::Test>::IntentUnitAlreadyExists
        );

        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(CHARLIE),
                2,
                unit_id(99),
                phase("missing"),
                u64::MAX,
            ),
            Error::<crate::mock::Test>::UnsupportedCommandSchemaVersion
        );
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(CHARLIE),
                COMMAND_SCHEMA_VERSION,
                id,
                phase("doing"),
                0,
            ),
            Error::<crate::mock::Test>::UnauthorizedSubmitter
        );
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                unit_id(99),
                phase("missing"),
                u64::MAX,
            ),
            Error::<crate::mock::Test>::IntentUnitNotFound
        );

        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                id,
                phase("missing"),
                0,
            ),
            Error::<crate::mock::Test>::UnknownTargetPhase
        );
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                id,
                phase("done"),
                0,
            ),
            Error::<crate::mock::Test>::TransitionNotAllowed
        );
        assert_noop!(
            CubiKan::complete_unit(RuntimeOrigin::signed(ALICE), COMMAND_SCHEMA_VERSION, id, 0,),
            Error::<crate::mock::Test>::CompletionPhaseNotEligible
        );

        let completed_id = unit_id(20);
        create(completed_id, ALICE, standard_workflow());
        assert_ok!(CubiKan::transition_unit(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            completed_id,
            phase("doing"),
            0,
        ));
        assert_ok!(CubiKan::transition_unit(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            completed_id,
            phase("done"),
            1,
        ));
        assert_ok!(CubiKan::complete_unit(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            completed_id,
            2,
        ));
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                completed_id,
                phase("missing"),
                3,
            ),
            Error::<crate::mock::Test>::IntentUnitAlreadyCompleted
        );
        assert_noop!(
            CubiKan::complete_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                completed_id,
                3,
            ),
            Error::<crate::mock::Test>::IntentUnitAlreadyCompleted
        );
        assert_eq!(CubiKan::global_sequence(), Some(5));
        assert_eq!(accepted_events().len(), 5);
    });
}

#[test]
fn test_transition_and_completion_advance_once_and_preserve_identity() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let id = unit_id(3);
        create(id, ALICE, standard_workflow());
        let identity = CubiKan::intent_units(id).unwrap();
        let mut expected = identity.clone();

        for (revision, target) in [(1, "doing"), (2, "queued"), (3, "doing"), (4, "done")] {
            let before_events = accepted_events().len();
            let from = expected.phase().clone();
            let target = phase(target);
            expected
                .transition_to(&target, revision - 1)
                .expect("declared transition must succeed in the model");
            assert_ok!(CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                id,
                target.clone(),
                revision - 1,
            ));
            assert_eq!(CubiKan::intent_units(id), Some(expected.clone()));
            assert_eq!(CubiKan::global_sequence(), Some(revision + 1));
            assert_eq!(accepted_events().len(), before_events + 1);
            assert_eq!(
                accepted_events().last(),
                Some(&Event::Accepted {
                    deployment_id: DEPLOYMENT_ID,
                    event_schema_version: EVENT_SCHEMA_VERSION,
                    global_sequence: revision + 1,
                    signer: ALICE,
                    payload: DomainPayload::UnitTransitioned {
                        unit_id: id,
                        committed_revision: revision,
                        from,
                        to: target,
                    },
                })
            );
        }

        expected.complete(4).expect("done is completion eligible");
        assert_ok!(CubiKan::complete_unit(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            id,
            4,
        ));
        let stored = CubiKan::intent_units(id).unwrap();
        assert_eq!(stored, expected);
        assert_eq!(stored.id(), identity.id());
        assert_eq!(stored.origin(), identity.origin());
        assert_eq!(stored.species(), identity.species());
        assert_eq!(stored.workflow(), identity.workflow());
        assert_eq!(stored.revision(), 5);
        assert_eq!(stored.history().len(), 5);
        assert_eq!(stored.status(), IntentUnitStatus::Completed);
        assert_eq!(CubiKan::global_sequence(), Some(6));
        assert_eq!(accepted_events().len(), 6);
        assert_eq!(
            accepted_events().last(),
            Some(&Event::Accepted {
                deployment_id: DEPLOYMENT_ID,
                event_schema_version: EVENT_SCHEMA_VERSION,
                global_sequence: 6,
                signer: ALICE,
                payload: DomainPayload::UnitCompleted {
                    unit_id: id,
                    committed_revision: 5,
                    phase: phase("done"),
                },
            })
        );
    });
}

#[test]
fn test_stale_revision_precedes_lifecycle_domain_errors() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let id = unit_id(4);
        create(id, ALICE, standard_workflow());
        assert_ok!(CubiKan::transition_unit(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            id,
            phase("doing"),
            0,
        ));

        // Aggregate selection precedes revision comparison.
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                unit_id(99),
                phase("missing"),
                0,
            ),
            Error::<crate::mock::Test>::IntentUnitNotFound
        );

        for target in ["missing", "doing"] {
            assert_noop!(
                CubiKan::transition_unit(
                    RuntimeOrigin::signed(ALICE),
                    COMMAND_SCHEMA_VERSION,
                    id,
                    phase(target),
                    0,
                ),
                Error::<crate::mock::Test>::StaleRevision
            );
        }
        assert_noop!(
            CubiKan::complete_unit(RuntimeOrigin::signed(ALICE), COMMAND_SCHEMA_VERSION, id, 0,),
            Error::<crate::mock::Test>::StaleRevision
        );

        // Current expectations expose the remaining semantic error.
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                id,
                phase("missing"),
                1,
            ),
            Error::<crate::mock::Test>::UnknownTargetPhase
        );
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                id,
                phase("doing"),
                1,
            ),
            Error::<crate::mock::Test>::TransitionNotAllowed
        );
        assert_noop!(
            CubiKan::complete_unit(RuntimeOrigin::signed(ALICE), COMMAND_SCHEMA_VERSION, id, 1,),
            Error::<crate::mock::Test>::CompletionPhaseNotEligible
        );
        assert_eq!(CubiKan::intent_units(id).unwrap().revision(), 1);
        assert_eq!(CubiKan::global_sequence(), Some(2));
        assert_eq!(accepted_events().len(), 2);
    });

    new_test_ext(vec![ALICE]).execute_with(|| {
        let id = unit_id(44);
        create(id, ALICE, standard_workflow());
        assert_ok!(CubiKan::transition_unit(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            id,
            phase("doing"),
            0,
        ));
        assert_ok!(CubiKan::transition_unit(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            id,
            phase("done"),
            1,
        ));
        assert_ok!(CubiKan::complete_unit(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            id,
            2,
        ));

        let completed = state_bytes(id);
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                id,
                phase("missing"),
                2,
            ),
            Error::<crate::mock::Test>::StaleRevision
        );
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                id,
                phase("missing"),
                3,
            ),
            Error::<crate::mock::Test>::IntentUnitAlreadyCompleted
        );
        assert_eq!(state_bytes(id), completed);
        assert_eq!(accepted_events().len(), 4);
    });
}

#[test]
fn test_same_revision_extrinsics_accept_exactly_one() {
    fn run(first: AccountId, second: AccountId) -> Vec<u8> {
        new_test_ext(vec![ALICE, BOB]).execute_with(|| {
            let id = unit_id(5);
            create(id, ALICE, standard_workflow());

            assert_ok!(CubiKan::transition_unit(
                RuntimeOrigin::signed(first),
                COMMAND_SCHEMA_VERSION,
                id,
                phase("doing"),
                0,
            ));
            assert_noop!(
                CubiKan::transition_unit(
                    RuntimeOrigin::signed(second),
                    COMMAND_SCHEMA_VERSION,
                    id,
                    phase("doing"),
                    0,
                ),
                Error::<crate::mock::Test>::StaleRevision
            );

            let stored = CubiKan::intent_units(id).unwrap();
            assert_eq!(stored.revision(), 1);
            assert_eq!(stored.history().len(), 1);
            assert_eq!(stored.phase(), &phase("doing"));
            assert_eq!(CubiKan::global_sequence(), Some(2));
            assert_eq!(accepted_events().len(), 2);
            assert_eq!(stored.id(), id);
            assert_eq!(stored.origin(), &origin_reference());
            assert_eq!(stored.species(), &species());
            assert!(matches!(
                accepted_events().last(),
                Some(Event::Accepted { signer, .. }) if *signer == first
            ));
            // Signers occur only as supplemental event metadata; the aggregate
            // bytes are independent of which authorized caller won.
            stored.encode()
        })
    }

    assert_eq!(run(ALICE, BOB), run(BOB, ALICE));
}

#[test]
fn test_lifecycle_and_global_sequence_boundaries_never_wrap() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        let full_id = unit_id(7);
        let fresh_id = unit_id(8);
        let looping = workflow(&["loop"], "loop", &[("loop", "loop")], &["loop"]);
        create(full_id, ALICE, looping);
        create(fresh_id, ALICE, standard_workflow());

        for expected_revision in 0..255 {
            assert_ok!(CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                full_id,
                phase("loop"),
                expected_revision,
            ));
        }
        let at_255 = CubiKan::intent_units(full_id).unwrap();
        assert_eq!(at_255.revision(), 255);
        assert_eq!(at_255.history().len(), 255);

        assert_ok!(CubiKan::complete_unit(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            full_id,
            255,
        ));
        let at_capacity = CubiKan::intent_units(full_id).unwrap();
        assert_eq!(at_capacity.revision(), 256);
        assert_eq!(at_capacity.history().len(), 256);
        assert_eq!(at_capacity.status(), IntentUnitStatus::Completed);

        let before_capacity_rejection = state_bytes(full_id);
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                full_id,
                phase("missing"),
                256,
            ),
            Error::<crate::mock::Test>::LifecycleHistoryCapacityExceeded
        );
        assert_eq!(state_bytes(full_id), before_capacity_rejection);

        GlobalSequence::<crate::mock::Test>::put(u64::MAX - 1);
        let before_maximum = accepted_events().len();
        assert_ok!(CubiKan::transition_unit(
            RuntimeOrigin::signed(ALICE),
            COMMAND_SCHEMA_VERSION,
            fresh_id,
            phase("doing"),
            0,
        ));
        assert_eq!(CubiKan::global_sequence(), Some(u64::MAX));
        assert_eq!(accepted_events().len(), before_maximum + 1);
        assert!(matches!(
            accepted_events().last(),
            Some(Event::Accepted {
                global_sequence: u64::MAX,
                ..
            })
        ));

        let before_exhaustion = state_bytes(fresh_id);
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                fresh_id,
                phase("queued"),
                1,
            ),
            Error::<crate::mock::Test>::GlobalSequenceExhausted
        );
        assert_eq!(state_bytes(fresh_id), before_exhaustion);

        // History capacity precedes both terminal/target validity and global
        // exhaustion when the two independent counters are exhausted together.
        let before_combined = state_bytes(full_id);
        assert_noop!(
            CubiKan::transition_unit(
                RuntimeOrigin::signed(ALICE),
                COMMAND_SCHEMA_VERSION,
                full_id,
                phase("missing"),
                256,
            ),
            Error::<crate::mock::Test>::LifecycleHistoryCapacityExceeded
        );
        assert_eq!(state_bytes(full_id), before_combined);
        assert_eq!(CubiKan::global_sequence(), Some(u64::MAX));
    });
}

#[test]
fn test_root_allowlist_replacement_is_bounded_and_nondomain() {
    new_test_ext(vec![ALICE]).execute_with(|| {
        for length in 0_u64..=16 {
            let expected: Vec<_> = (100..100 + length).collect();
            let before_events = pallet_events().len();
            assert_ok!(CubiKan::replace_authorized_submitters(
                RuntimeOrigin::root(),
                input_accounts(expected.clone()),
            ));
            assert_eq!(
                CubiKan::authorized_submitters().into_inner(),
                expected,
                "root replacement at length {length}"
            );
            assert!(matches!(
                pallet_events().last(),
                Some(Event::AuthorizedSubmittersReplaced { accounts })
                    if accounts.as_slice() == CubiKan::authorized_submitters().as_slice()
            ));
            assert_eq!(pallet_events().len(), before_events + 1);
            assert_eq!(CubiKan::global_sequence(), None);
            assert!(accepted_events().is_empty());
        }

        let accepted = CubiKan::authorized_submitters();
        assert_noop!(
            CubiKan::replace_authorized_submitters(
                RuntimeOrigin::signed(ALICE),
                input_accounts(vec![ALICE]),
            ),
            DispatchError::BadOrigin
        );
        assert_noop!(
            CubiKan::replace_authorized_submitters(
                RuntimeOrigin::root(),
                input_accounts(vec![ALICE, ALICE]),
            ),
            Error::<crate::mock::Test>::DuplicateAuthorizedSubmitter
        );
        assert_noop!(
            CubiKan::replace_authorized_submitters(
                RuntimeOrigin::root(),
                input_accounts((0..17).collect()),
            ),
            Error::<crate::mock::Test>::TooManyAuthorizedSubmitters
        );
        assert_eq!(CubiKan::authorized_submitters(), accepted);
        assert_eq!(CubiKan::global_sequence(), None);
        assert!(accepted_events().is_empty());
    });
}
