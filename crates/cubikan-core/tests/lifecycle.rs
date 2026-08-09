mod common;

use cubikan_core::{
    CompletionError, IntentSpecies, IntentUnit, IntentUnitRevision, IntentUnitStatus,
    LifecycleRecord, PhaseId, TransitionError, Workflow, WorkflowEdge, WorkflowId,
};

use common::{fixed_id, linear_unit, phase, species};

fn assert_one_lifecycle_advance(
    unit: &IntentUnit,
    previous_revision: IntentUnitRevision,
    previous_history_len: usize,
) {
    let expected_revision = IntentUnitRevision::new(
        previous_revision
            .value()
            .checked_add(1)
            .expect("test revision should have a successor"),
    );
    assert_eq!(unit.revision(), expected_revision);
    assert_eq!(unit.history().len(), previous_history_len + 1);
    assert_eq!(
        u64::try_from(
            unit.history()
                .last()
                .expect("successful mutation should append a record")
                .sequence(),
        )
        .expect("test sequence should fit in u64"),
        unit.revision().value()
    );
}

#[test]
fn test_intent_unit_starts_at_zero_revision() {
    let unit = linear_unit();

    assert_eq!(unit.revision(), IntentUnitRevision::INITIAL);
    assert_eq!(unit.revision(), IntentUnitRevision::new(0));
    assert_eq!(unit.revision().value(), 0);
    assert_eq!(unit.revision().to_string(), "0");
    assert_eq!(unit.status(), IntentUnitStatus::Active);
    assert_eq!(unit.phase(), &phase("queued"));
    assert!(unit.history().is_empty());
}

#[test]
fn test_unconditioned_mutations_advance_revision_once_per_record() {
    let queued = phase("queued");
    let doing = phase("doing");
    let done = phase("done");
    let workflow = Workflow::new(
        WorkflowId::new("revisioned-rework").expect("workflow ID should be valid"),
        vec![queued.clone(), doing.clone(), done.clone()],
        queued.clone(),
        vec![
            WorkflowEdge::new(queued.clone(), doing.clone()),
            WorkflowEdge::new(doing.clone(), queued.clone()),
            WorkflowEdge::new(queued.clone(), queued.clone()),
            WorkflowEdge::new(queued.clone(), done.clone()),
        ],
        vec![done.clone()],
    )
    .expect("revision fixture workflow should be valid");
    let mut unit = IntentUnit::new(fixed_id(), species(), workflow);

    for target in [&doing, &queued, &queued, &done] {
        let previous_revision = unit.revision();
        let previous_history_len = unit.history().len();
        unit.transition_to(target)
            .expect("declared transition should succeed");
        assert_one_lifecycle_advance(&unit, previous_revision, previous_history_len);
    }

    let previous_revision = unit.revision();
    let previous_history_len = unit.history().len();
    unit.complete().expect("eligible completion should succeed");
    assert_one_lifecycle_advance(&unit, previous_revision, previous_history_len);
    assert_eq!(unit.status(), IntentUnitStatus::Completed);
}

#[test]
fn test_failed_unconditioned_commands_preserve_revision_and_aggregate() {
    let mut unit = linear_unit();

    let before_unknown = unit.clone();
    assert_eq!(
        unit.transition_to(&phase("missing")),
        Err(TransitionError::UnknownTarget {
            target: phase("missing")
        })
    );
    assert_eq!(unit, before_unknown);

    let before_undeclared = unit.clone();
    assert_eq!(
        unit.transition_to(&phase("done")),
        Err(TransitionError::NotAllowed {
            from: phase("queued"),
            to: phase("done")
        })
    );
    assert_eq!(unit, before_undeclared);

    let before_ineligible_completion = unit.clone();
    assert_eq!(
        unit.complete(),
        Err(CompletionError::PhaseNotEligible {
            phase: phase("queued")
        })
    );
    assert_eq!(unit, before_ineligible_completion);

    unit.transition_to(&phase("doing"))
        .expect("first transition should succeed");
    unit.transition_to(&phase("done"))
        .expect("completion phase should be reachable");
    unit.complete().expect("completion should succeed");

    let before_terminal_transition = unit.clone();
    assert_eq!(
        unit.transition_to(&phase("queued")),
        Err(TransitionError::AlreadyCompleted)
    );
    assert_eq!(unit, before_terminal_transition);

    let before_repeated_completion = unit.clone();
    assert_eq!(unit.complete(), Err(CompletionError::AlreadyCompleted));
    assert_eq!(unit, before_repeated_completion);
}

#[test]
fn test_custom_workflow_configuration_composes_domain_values() {
    let discovery = PhaseId::new("KPI: discovery throughput").expect("phase should be valid");
    let review = PhaseId::new("custom/review").expect("phase should be valid");
    let workflow = Workflow::new(
        WorkflowId::new("顧客-support").expect("workflow ID should be valid"),
        vec![discovery.clone(), review.clone()],
        discovery.clone(),
        vec![WorkflowEdge::new(discovery.clone(), review.clone())],
        vec![review.clone()],
    )
    .expect("caller-defined workflow should be valid");
    let unit = IntentUnit::new(
        fixed_id(),
        IntentSpecies::new("customer-request").expect("species should be valid"),
        workflow,
    );

    assert_eq!(unit.phase(), &discovery);
    assert!(unit.workflow().allows_transition(&discovery, &review));
    assert!(unit.workflow().allows_completion(&review));
}

#[test]
fn test_intent_lifecycle_create_transition_complete() {
    let mut unit = linear_unit();
    let id = unit.id();
    let species = unit.species().clone();
    let workflow = unit.workflow().clone();

    unit.transition_to(&phase("doing"))
        .expect("first transition should succeed");
    unit.transition_to(&phase("done"))
        .expect("second transition should succeed");
    unit.complete().expect("completion should succeed");

    assert_eq!(unit.id(), id);
    assert_eq!(unit.species(), &species);
    assert_eq!(unit.workflow(), &workflow);
    assert_eq!(unit.phase(), &phase("done"));
    assert_eq!(unit.status(), IntentUnitStatus::Completed);
    let [
        LifecycleRecord::Transition(first),
        LifecycleRecord::Transition(second),
        LifecycleRecord::Completion(completion),
    ] = unit.history()
    else {
        panic!("history should contain two transitions followed by completion");
    };
    assert_eq!(first.sequence(), 1);
    assert_eq!(first.from(), &phase("queued"));
    assert_eq!(first.to(), &phase("doing"));
    assert_eq!(second.sequence(), 2);
    assert_eq!(second.from(), &phase("doing"));
    assert_eq!(second.to(), &phase("done"));
    assert_eq!(completion.sequence(), 3);
    assert_eq!(completion.final_phase(), &phase("done"));
    assert_eq!(
        unit.transition_to(&phase("queued")),
        Err(TransitionError::AlreadyCompleted)
    );
}

#[test]
fn test_failed_operations_are_atomic_and_recoverable() {
    let mut unit = linear_unit();

    let before_unknown = unit.clone();
    assert_eq!(
        unit.transition_to(&phase("missing")),
        Err(TransitionError::UnknownTarget {
            target: phase("missing")
        })
    );
    assert_eq!(unit, before_unknown);

    let before_undeclared = unit.clone();
    assert_eq!(
        unit.transition_to(&phase("done")),
        Err(TransitionError::NotAllowed {
            from: phase("queued"),
            to: phase("done")
        })
    );
    assert_eq!(unit, before_undeclared);

    let before_completion = unit.clone();
    assert_eq!(
        unit.complete(),
        Err(CompletionError::PhaseNotEligible {
            phase: phase("queued")
        })
    );
    assert_eq!(unit, before_completion);

    unit.transition_to(&phase("doing"))
        .expect("recovery transition should succeed");
    unit.transition_to(&phase("done"))
        .expect("completion phase should remain reachable");
    unit.complete()
        .expect("unit should remain completable after rejected operations");
    assert_eq!(unit.status(), IntentUnitStatus::Completed);
}

#[test]
fn test_explicit_rework_cycle_is_honored() {
    let queued = phase("queued");
    let doing = phase("doing");
    let done = phase("done");
    let workflow = Workflow::new(
        WorkflowId::new("rework").expect("workflow ID should be valid"),
        vec![queued.clone(), doing.clone(), done.clone()],
        queued.clone(),
        vec![
            WorkflowEdge::new(queued.clone(), doing.clone()),
            WorkflowEdge::new(doing.clone(), queued.clone()),
            WorkflowEdge::new(doing.clone(), done.clone()),
        ],
        vec![done.clone()],
    )
    .expect("rework workflow should be valid");
    let mut unit = IntentUnit::new(fixed_id(), species(), workflow);

    unit.transition_to(&doing)
        .expect("forward transition should succeed");
    unit.transition_to(&queued)
        .expect("declared reverse transition should succeed");
    unit.transition_to(&doing)
        .expect("forward transition should remain available");
    unit.transition_to(&done)
        .expect("completion phase should be reachable");
    unit.complete().expect("completion should succeed");

    assert_eq!(unit.phase(), &done);
    assert_eq!(unit.history().len(), 5);
}
