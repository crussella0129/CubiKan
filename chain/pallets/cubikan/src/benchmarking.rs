//! Executable FRAME v2 benchmarks for every canonical pallet call.
//!
//! Domain fixtures deliberately exercise the production maxima. Transition
//! and completion setup fills 255 of 256 lifecycle records so the measured
//! call appends the largest permitted aggregate. T-1106 runs these benchmarks
//! in the benchmark-capable runtime and replaces the provisional weights.

use alloc::vec::Vec;

use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

use crate::{
    event::COMMAND_SCHEMA_VERSION,
    types::{
        AssociationKey, AssociationSubject, DefinitionKey, DefinitionVersion, ExternalReference,
        IntentSpecies, IntentUnitId, IntentUnitState, Namespace, PhaseId, ReferenceScope,
        ReferenceValue, RelationshipDefinition, RelationshipKey, RelationshipPolicy, Workflow,
        WorkflowEdge, WorkflowId, MAX_ACTIVE_ASSOCIATIONS, MAX_AUTHORIZED_SUBMITTERS,
        MAX_COMPLETION_PHASES, MAX_LIFECYCLE_RECORDS, MAX_NAMESPACE_BYTES, MAX_RELATIONSHIP_EDGES,
        MAX_TEXT_BYTES, MAX_WORKFLOW_EDGES, MAX_WORKFLOW_PHASES,
    },
    ActiveAssociations, ActiveAssociationsOf, AuthorizedSubmitterInput, AuthorizedSubmitters,
    AuthorizedSubmittersOf, Call, Config, GlobalSequence, IntentUnits, Pallet,
    RelationshipDefinitions, RelationshipEdges, RelationshipEdgesOf,
};

fn maximal_text(marker: u8, index: usize) -> [u8; MAX_TEXT_BYTES] {
    let mut bytes = [b'x'; MAX_TEXT_BYTES];
    bytes[0] = marker;
    bytes[MAX_TEXT_BYTES - 2] = b'0' + u8::try_from((index / 10) % 10).expect("one digit");
    bytes[MAX_TEXT_BYTES - 1] = b'0' + u8::try_from(index % 10).expect("one digit");
    bytes
}

fn maximal_phase(index: usize) -> PhaseId {
    PhaseId::try_from_bytes(&maximal_text(b'p', index)).expect("maximal phase is valid text")
}

fn maximal_origin() -> ExternalReference {
    let namespace = [b'n'; MAX_NAMESPACE_BYTES];
    ExternalReference::new(
        Namespace::try_from_bytes(&namespace).expect("maximal namespace is valid"),
        ReferenceScope::try_from_bytes(&maximal_text(b's', 0)).expect("valid scope"),
        ReferenceValue::try_from_bytes(&maximal_text(b'v', 0)).expect("valid value"),
    )
}

fn maximal_reference(index: usize) -> ExternalReference {
    let namespace = [b'p'; MAX_NAMESPACE_BYTES];
    let mut value = maximal_text(b'v', 0);
    value[MAX_TEXT_BYTES - 3] = b'0' + u8::try_from((index / 100) % 10).expect("one decimal digit");
    value[MAX_TEXT_BYTES - 2] = b'0' + u8::try_from((index / 10) % 10).expect("one decimal digit");
    value[MAX_TEXT_BYTES - 1] = b'0' + u8::try_from(index % 10).expect("one decimal digit");
    ExternalReference::new(
        Namespace::try_from_bytes(&namespace).expect("maximal namespace is valid"),
        ReferenceScope::try_from_bytes(&maximal_text(b'a', 0)).expect("valid scope"),
        ReferenceValue::try_from_bytes(&value).expect("valid indexed value"),
    )
}

fn maximal_species() -> IntentSpecies {
    IntentSpecies::try_from_bytes(&maximal_text(b'i', 0)).expect("valid species")
}

fn maximal_definition(
    self_policy: RelationshipPolicy,
    cycle_policy: RelationshipPolicy,
) -> RelationshipDefinition {
    let namespace = [b'r'; MAX_NAMESPACE_BYTES];
    RelationshipDefinition::new(
        DefinitionKey::new(
            Namespace::try_from_bytes(&namespace).expect("maximal definition id is valid"),
            DefinitionVersion::try_new(u64::MAX).expect("maximum version is positive"),
        ),
        Some(maximal_species()),
        Some(maximal_species()),
        self_policy,
        cycle_policy,
    )
}

fn maximal_workflow() -> Workflow {
    let phases: Vec<_> = (0..MAX_WORKFLOW_PHASES).map(maximal_phase).collect();
    let setup_forward = WorkflowEdge::new(phases[30].clone(), phases[31].clone());
    let setup_reverse = WorkflowEdge::new(phases[31].clone(), phases[30].clone());
    let measured_edge = WorkflowEdge::new(phases[31].clone(), phases[31].clone());
    let mut edges = Vec::with_capacity(MAX_WORKFLOW_EDGES);
    'sources: for from in &phases {
        for to in &phases {
            let candidate = WorkflowEdge::new(from.clone(), to.clone());
            if candidate == setup_forward
                || candidate == setup_reverse
                || candidate == measured_edge
            {
                continue;
            }
            edges.push(candidate);
            if edges.len() == MAX_WORKFLOW_EDGES - 3 {
                break 'sources;
            }
        }
    }
    edges.push(setup_forward.clone());
    edges.push(setup_reverse.clone());
    edges.push(measured_edge.clone());
    let completion_phases = phases.clone();
    assert_eq!(edges.len(), MAX_WORKFLOW_EDGES);
    assert_eq!(edges[MAX_WORKFLOW_EDGES - 3], setup_forward);
    assert_eq!(edges[MAX_WORKFLOW_EDGES - 2], setup_reverse);
    assert_eq!(edges[MAX_WORKFLOW_EDGES - 1], measured_edge);
    assert_eq!(completion_phases.len(), MAX_COMPLETION_PHASES);

    Workflow::try_new(
        WorkflowId::try_from_bytes(&maximal_text(b'w', 0)).expect("valid workflow id"),
        &phases,
        phases[30].clone(),
        &edges,
        &completion_phases,
    )
    .expect("maximal workflow fixture is valid")
}

fn authorize<T: Config>(caller: T::AccountId) {
    let mut unbounded = Vec::with_capacity(MAX_AUTHORIZED_SUBMITTERS);
    let mut index = 0;
    while unbounded.len() < MAX_AUTHORIZED_SUBMITTERS - 1 {
        let candidate = account("non-caller", index, 0);
        if candidate != caller && !unbounded.contains(&candidate) {
            unbounded.push(candidate);
        }
        index += 1;
    }
    unbounded.push(caller);
    let accounts: AuthorizedSubmittersOf<T> = unbounded
        .try_into()
        .expect("maximal account fixture is within the allowlist bound");
    AuthorizedSubmitters::<T>::put(accounts);
}

fn create_maximal_unit<T: Config>(caller: T::AccountId, id: IntentUnitId) {
    Pallet::<T>::create_unit(
        RawOrigin::Signed(caller).into(),
        COMMAND_SCHEMA_VERSION,
        id,
        maximal_origin(),
        maximal_species(),
        maximal_workflow(),
    )
    .expect("authorized maximal unit creation succeeds");
}

fn store_maximal_unit<T: Config>(marker: u8) -> IntentUnitId {
    let id = IntentUnitId::from_bytes([marker; 16]);
    IntentUnits::<T>::insert(
        id,
        IntentUnitState::new(id, maximal_origin(), maximal_species(), maximal_workflow()),
    );
    id
}

fn store_relationship_definition<T: Config>(definition: RelationshipDefinition) {
    let key = definition.key().clone();
    RelationshipDefinitions::<T>::insert(key, definition);
}

fn maximal_relationship_graph<T: Config>() -> (DefinitionKey, Vec<IntentUnitId>) {
    let definition = maximal_definition(RelationshipPolicy::Allow, RelationshipPolicy::Reject);
    let key = definition.key().clone();
    store_relationship_definition::<T>(definition);

    let units: Vec<_> = (0..=MAX_RELATIONSHIP_EDGES)
        .map(|index| {
            store_maximal_unit::<T>(
                u8::try_from(index + 1).expect("129 unit fixture markers fit u8"),
            )
        })
        .collect();
    let edges: RelationshipEdgesOf = (0..MAX_RELATIONSHIP_EDGES - 1)
        .rev()
        .map(|index| RelationshipKey::new(key.clone(), units[index], units[index + 1]))
        .collect::<Vec<_>>()
        .try_into()
        .expect("127-edge setup is within the relationship bound");
    RelationshipEdges::<T>::insert(&key, edges);
    (key, units)
}

fn fill_lifecycle_to_max_minus_one<T: Config>(caller: T::AccountId, id: IntentUnitId) {
    for revision in 0..u64::try_from(MAX_LIFECYCLE_RECORDS - 1).expect("bound fits u64") {
        let target = if revision % 2 == 0 {
            maximal_phase(31)
        } else {
            maximal_phase(30)
        };
        Pallet::<T>::transition_unit(
            RawOrigin::Signed(caller.clone()).into(),
            COMMAND_SCHEMA_VERSION,
            id,
            target,
            revision,
        )
        .expect("fixture transition is valid");
    }
}

fn fill_stored_lifecycle_to_capacity<T: Config>(id: IntentUnitId) {
    let mut state = IntentUnits::<T>::get(id).expect("benchmark endpoint is stored");
    for revision in 0..u64::try_from(MAX_LIFECYCLE_RECORDS).expect("bound fits u64") {
        let target = if revision % 2 == 0 {
            maximal_phase(31)
        } else {
            maximal_phase(30)
        };
        state
            .transition_to(&target, revision)
            .expect("direct fixture transition remains valid through capacity");
    }
    IntentUnits::<T>::insert(id, state);
}

fn maximal_association(unit_id: IntentUnitId, index: usize) -> AssociationKey {
    AssociationKey::new(
        unit_id,
        AssociationSubject::Revision(MAX_LIFECYCLE_RECORDS as u64),
        maximal_reference(index),
    )
}

fn store_maximal_associations<T: Config>(
    unit_id: IntentUnitId,
    count: usize,
) -> Vec<AssociationKey> {
    let associations: Vec<_> = (0..count)
        .map(|index| maximal_association(unit_id, index))
        .collect();
    let bounded: ActiveAssociationsOf = associations
        .clone()
        .try_into()
        .expect("association fixture count is within the active bound");
    ActiveAssociations::<T>::insert(unit_id, bounded);
    associations
}

fn seed_global_sequence_maximum_successor<T: Config>() {
    GlobalSequence::<T>::put(u64::MAX - 1);
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn create_unit() {
        let caller: T::AccountId = whitelisted_caller();
        authorize::<T>(caller.clone());
        let id = IntentUnitId::from_bytes([0x11; 16]);
        seed_global_sequence_maximum_successor::<T>();

        #[extrinsic_call]
        create_unit(
            RawOrigin::Signed(caller),
            COMMAND_SCHEMA_VERSION,
            id,
            maximal_origin(),
            maximal_species(),
            maximal_workflow(),
        );

        let stored = IntentUnits::<T>::get(id).expect("created unit is stored");
        assert_eq!(stored.revision(), 0);
        assert_eq!(stored.workflow().phases().len(), MAX_WORKFLOW_PHASES);
        assert_eq!(stored.workflow().edges().len(), MAX_WORKFLOW_EDGES);
        assert_eq!(GlobalSequence::<T>::get(), Some(u64::MAX));
    }

    #[benchmark]
    fn transition_unit() {
        let caller: T::AccountId = whitelisted_caller();
        authorize::<T>(caller.clone());
        let id = IntentUnitId::from_bytes([0x22; 16]);
        create_maximal_unit::<T>(caller.clone(), id);
        fill_lifecycle_to_max_minus_one::<T>(caller.clone(), id);
        let expected_revision = u64::try_from(MAX_LIFECYCLE_RECORDS - 1).expect("bound fits u64");
        seed_global_sequence_maximum_successor::<T>();

        #[extrinsic_call]
        transition_unit(
            RawOrigin::Signed(caller),
            COMMAND_SCHEMA_VERSION,
            id,
            maximal_phase(31),
            expected_revision,
        );

        let stored = IntentUnits::<T>::get(id).expect("transitioned unit remains stored");
        assert_eq!(stored.revision(), MAX_LIFECYCLE_RECORDS as u64);
        assert_eq!(stored.history().len(), MAX_LIFECYCLE_RECORDS);
        assert_eq!(stored.phase(), &maximal_phase(31));
        assert_eq!(GlobalSequence::<T>::get(), Some(u64::MAX));
    }

    #[benchmark]
    fn complete_unit() {
        let caller: T::AccountId = whitelisted_caller();
        authorize::<T>(caller.clone());
        let id = IntentUnitId::from_bytes([0x33; 16]);
        create_maximal_unit::<T>(caller.clone(), id);
        fill_lifecycle_to_max_minus_one::<T>(caller.clone(), id);
        let expected_revision = u64::try_from(MAX_LIFECYCLE_RECORDS - 1).expect("bound fits u64");
        seed_global_sequence_maximum_successor::<T>();

        #[extrinsic_call]
        complete_unit(
            RawOrigin::Signed(caller),
            COMMAND_SCHEMA_VERSION,
            id,
            expected_revision,
        );

        let stored = IntentUnits::<T>::get(id).expect("completed unit remains stored");
        assert_eq!(stored.revision(), MAX_LIFECYCLE_RECORDS as u64);
        assert_eq!(stored.history().len(), MAX_LIFECYCLE_RECORDS);
        assert_eq!(stored.status(), crate::types::IntentUnitStatus::Completed);
        assert_eq!(GlobalSequence::<T>::get(), Some(u64::MAX));
    }

    #[benchmark]
    fn replace_authorized_submitters(a: Linear<0, 16>) {
        let accounts: Vec<T::AccountId> = (0..a)
            .map(|index| account("authorized", index, 0))
            .collect();
        let input: AuthorizedSubmitterInput<T> = accounts
            .clone()
            .try_into()
            .expect("benchmark range fits decoded input bound");

        #[extrinsic_call]
        replace_authorized_submitters(RawOrigin::Root, input);

        assert_eq!(AuthorizedSubmitters::<T>::get().into_inner(), accounts);
        assert_eq!(GlobalSequence::<T>::get(), None);
        assert_eq!(frame_system::Pallet::<T>::event_count(), 1);
    }

    #[benchmark]
    fn create_relationship_definition() {
        let caller: T::AccountId = whitelisted_caller();
        authorize::<T>(caller.clone());
        let definition = maximal_definition(RelationshipPolicy::Reject, RelationshipPolicy::Reject);
        let key = definition.key().clone();
        seed_global_sequence_maximum_successor::<T>();

        #[extrinsic_call]
        create_relationship_definition(
            RawOrigin::Signed(caller),
            COMMAND_SCHEMA_VERSION,
            definition.clone(),
        );

        assert_eq!(RelationshipDefinitions::<T>::get(key), Some(definition));
        assert_eq!(GlobalSequence::<T>::get(), Some(u64::MAX));
    }

    #[benchmark]
    fn create_relationship() {
        let caller: T::AccountId = whitelisted_caller();
        authorize::<T>(caller.clone());
        let (definition, units) = maximal_relationship_graph::<T>();
        fill_stored_lifecycle_to_capacity::<T>(units[0]);
        fill_stored_lifecycle_to_capacity::<T>(units[MAX_RELATIONSHIP_EDGES]);
        let relationship =
            RelationshipKey::new(definition.clone(), units[MAX_RELATIONSHIP_EDGES], units[0]);
        seed_global_sequence_maximum_successor::<T>();

        #[extrinsic_call]
        create_relationship(
            RawOrigin::Signed(caller),
            COMMAND_SCHEMA_VERSION,
            relationship.clone(),
        );

        let edges = RelationshipEdges::<T>::get(definition);
        assert_eq!(edges.len(), MAX_RELATIONSHIP_EDGES);
        assert_eq!(edges.last(), Some(&relationship));
        assert_eq!(GlobalSequence::<T>::get(), Some(u64::MAX));
    }

    #[benchmark]
    fn delete_relationship() {
        let caller: T::AccountId = whitelisted_caller();
        authorize::<T>(caller.clone());
        let (definition, units) = maximal_relationship_graph::<T>();
        fill_stored_lifecycle_to_capacity::<T>(units[0]);
        fill_stored_lifecycle_to_capacity::<T>(units[MAX_RELATIONSHIP_EDGES]);
        let relationship =
            RelationshipKey::new(definition.clone(), units[MAX_RELATIONSHIP_EDGES], units[0]);
        RelationshipEdges::<T>::mutate(&definition, |edges| {
            edges
                .try_push(relationship.clone())
                .expect("128th edge reaches the exact capacity")
        });
        seed_global_sequence_maximum_successor::<T>();

        #[extrinsic_call]
        delete_relationship(
            RawOrigin::Signed(caller),
            COMMAND_SCHEMA_VERSION,
            relationship.clone(),
        );

        let edges = RelationshipEdges::<T>::get(definition);
        assert_eq!(edges.len(), MAX_RELATIONSHIP_EDGES - 1);
        assert!(!edges.contains(&relationship));
        assert_eq!(GlobalSequence::<T>::get(), Some(u64::MAX));
    }

    #[benchmark]
    fn record_association() {
        let caller: T::AccountId = whitelisted_caller();
        authorize::<T>(caller.clone());
        let unit_id = store_maximal_unit::<T>(0xe1);
        fill_stored_lifecycle_to_capacity::<T>(unit_id);
        store_maximal_associations::<T>(unit_id, MAX_ACTIVE_ASSOCIATIONS - 1);
        let association = maximal_association(unit_id, MAX_ACTIVE_ASSOCIATIONS - 1);
        seed_global_sequence_maximum_successor::<T>();

        #[extrinsic_call]
        record_association(
            RawOrigin::Signed(caller),
            COMMAND_SCHEMA_VERSION,
            association.clone(),
        );

        let active = ActiveAssociations::<T>::get(unit_id);
        let unit = IntentUnits::<T>::get(unit_id).expect("associated unit remains stored");
        assert_eq!(active.len(), MAX_ACTIVE_ASSOCIATIONS);
        assert_eq!(active.last(), Some(&association));
        assert_eq!(unit.revision(), MAX_LIFECYCLE_RECORDS as u64);
        assert_eq!(unit.history().len(), MAX_LIFECYCLE_RECORDS);
        assert_eq!(GlobalSequence::<T>::get(), Some(u64::MAX));
    }

    #[benchmark]
    fn revoke_association() {
        let caller: T::AccountId = whitelisted_caller();
        authorize::<T>(caller.clone());
        let unit_id = store_maximal_unit::<T>(0xe2);
        fill_stored_lifecycle_to_capacity::<T>(unit_id);
        let associations = store_maximal_associations::<T>(unit_id, MAX_ACTIVE_ASSOCIATIONS);
        let association = associations
            .last()
            .expect("maximal association fixture is nonempty")
            .clone();
        seed_global_sequence_maximum_successor::<T>();

        #[extrinsic_call]
        revoke_association(
            RawOrigin::Signed(caller),
            COMMAND_SCHEMA_VERSION,
            association.clone(),
        );

        let active = ActiveAssociations::<T>::get(unit_id);
        let unit = IntentUnits::<T>::get(unit_id).expect("associated unit remains stored");
        assert_eq!(active.len(), MAX_ACTIVE_ASSOCIATIONS - 1);
        assert!(!active.contains(&association));
        assert_eq!(unit.revision(), MAX_LIFECYCLE_RECORDS as u64);
        assert_eq!(unit.history().len(), MAX_LIFECYCLE_RECORDS);
        assert_eq!(GlobalSequence::<T>::get(), Some(u64::MAX));
    }

    #[cfg(test)]
    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(vec![]), crate::mock::Test);
}

const _: () = assert!(MAX_AUTHORIZED_SUBMITTERS == 16);
