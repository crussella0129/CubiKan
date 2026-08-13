//! Executable FRAME v2 benchmarks for every lifecycle dispatchable.
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
        ExternalReference, IntentSpecies, IntentUnitId, Namespace, PhaseId, ReferenceScope,
        ReferenceValue, Workflow, WorkflowEdge, WorkflowId, MAX_AUTHORIZED_SUBMITTERS,
        MAX_COMPLETION_PHASES, MAX_LIFECYCLE_RECORDS, MAX_NAMESPACE_BYTES, MAX_TEXT_BYTES,
        MAX_WORKFLOW_EDGES, MAX_WORKFLOW_PHASES,
    },
    AuthorizedSubmitterInput, AuthorizedSubmitters, AuthorizedSubmittersOf, Call, Config,
    GlobalSequence, IntentUnits, Pallet,
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

fn maximal_species() -> IntentSpecies {
    IntentSpecies::try_from_bytes(&maximal_text(b'i', 0)).expect("valid species")
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

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn create_unit() {
        let caller: T::AccountId = whitelisted_caller();
        authorize::<T>(caller.clone());
        let id = IntentUnitId::from_bytes([0x11; 16]);

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
        assert_eq!(GlobalSequence::<T>::get(), Some(1));
    }

    #[benchmark]
    fn transition_unit() {
        let caller: T::AccountId = whitelisted_caller();
        authorize::<T>(caller.clone());
        let id = IntentUnitId::from_bytes([0x22; 16]);
        create_maximal_unit::<T>(caller.clone(), id);
        fill_lifecycle_to_max_minus_one::<T>(caller.clone(), id);
        let expected_revision = u64::try_from(MAX_LIFECYCLE_RECORDS - 1).expect("bound fits u64");

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
    }

    #[benchmark]
    fn complete_unit() {
        let caller: T::AccountId = whitelisted_caller();
        authorize::<T>(caller.clone());
        let id = IntentUnitId::from_bytes([0x33; 16]);
        create_maximal_unit::<T>(caller.clone(), id);
        fill_lifecycle_to_max_minus_one::<T>(caller.clone(), id);
        let expected_revision = u64::try_from(MAX_LIFECYCLE_RECORDS - 1).expect("bound fits u64");

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

    #[cfg(test)]
    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(vec![]), crate::mock::Test);
}

const _: () = assert!(MAX_AUTHORIZED_SUBMITTERS == 16);
