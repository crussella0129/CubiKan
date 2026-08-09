mod common;

use cubikan_core::{
    CompletionError, IntentSpecies, IntentUnit, IntentUnitRevision, IntentUnitStatus,
    LifecycleRecord, PhaseId, RevisionConflict, RevisionedCompletionError,
    RevisionedTransitionError, TransitionError, Workflow, WorkflowEdge, WorkflowId,
};
use std::error::Error as _;

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

fn assert_transition_conflict(
    error: &RevisionedTransitionError,
    expected: IntentUnitRevision,
    actual: IntentUnitRevision,
) {
    let RevisionedTransitionError::Conflict(conflict) = error else {
        panic!("stale transition should return a revision conflict: {error:?}");
    };
    assert_eq!(conflict.expected(), expected);
    assert_eq!(conflict.actual(), actual);
    assert_eq!(
        error
            .source()
            .and_then(|source| source.downcast_ref::<RevisionConflict>()),
        Some(conflict)
    );
}

fn assert_completion_conflict(
    error: &RevisionedCompletionError,
    expected: IntentUnitRevision,
    actual: IntentUnitRevision,
) {
    let RevisionedCompletionError::Conflict(conflict) = error else {
        panic!("stale completion should return a revision conflict: {error:?}");
    };
    assert_eq!(conflict.expected(), expected);
    assert_eq!(conflict.actual(), actual);
    assert_eq!(
        error
            .source()
            .and_then(|source| source.downcast_ref::<RevisionConflict>()),
        Some(conflict)
    );
}

fn assert_current_transition_error(
    unit: &mut IntentUnit,
    target: &PhaseId,
    expected: TransitionError,
) {
    let before = unit.clone();
    let error = unit
        .transition_to_if_revision(target, unit.revision())
        .expect_err("current but invalid transition should retain its domain error");

    assert_eq!(
        error,
        RevisionedTransitionError::Transition(expected.clone())
    );
    assert_eq!(
        error
            .source()
            .and_then(|source| source.downcast_ref::<TransitionError>()),
        Some(&expected)
    );
    assert_eq!(unit, &before);
}

fn assert_current_completion_error(unit: &mut IntentUnit, expected: CompletionError) {
    let before = unit.clone();
    let error = unit
        .complete_if_revision(unit.revision())
        .expect_err("current but invalid completion should retain its domain error");

    assert_eq!(
        error,
        RevisionedCompletionError::Completion(expected.clone())
    );
    assert_eq!(
        error
            .source()
            .and_then(|source| source.downcast_ref::<CompletionError>()),
        Some(&expected)
    );
    assert_eq!(unit, &before);
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
fn test_guarded_transition_returns_exact_successor_revision() {
    let queued = phase("queued");
    let doing = phase("doing");
    let done = phase("done");
    let workflow = Workflow::new(
        WorkflowId::new("guarded-rework").expect("workflow ID should be valid"),
        vec![queued.clone(), doing.clone(), done.clone()],
        queued.clone(),
        vec![
            WorkflowEdge::new(queued.clone(), doing.clone()),
            WorkflowEdge::new(doing.clone(), queued.clone()),
            WorkflowEdge::new(queued.clone(), queued.clone()),
        ],
        vec![done],
    )
    .expect("guarded transition fixture should be valid");
    let mut unit = IntentUnit::new(fixed_id(), species(), workflow);
    let id = unit.id();
    let species = unit.species().clone();
    let workflow = unit.workflow().clone();
    let transitions = [
        ("forward", queued.clone(), doing.clone()),
        ("reverse", doing, queued.clone()),
        ("self", queued.clone(), queued),
    ];

    for (index, (edge_kind, expected_from, target)) in transitions.iter().enumerate() {
        let expected_revision = unit.revision();
        let previous_history = unit.history().to_vec();
        assert_eq!(unit.phase(), expected_from, "{edge_kind} edge source");

        let committed_revision = unit
            .transition_to_if_revision(target, expected_revision)
            .unwrap_or_else(|error| panic!("declared {edge_kind} edge should succeed: {error}"));
        let expected_sequence = index + 1;
        let exact_successor = IntentUnitRevision::new(
            u64::try_from(expected_sequence).expect("test sequence should fit in u64"),
        );

        assert_eq!(committed_revision, exact_successor);
        assert_eq!(unit.revision(), committed_revision);
        assert_one_lifecycle_advance(&unit, expected_revision, previous_history.len());
        assert_eq!(unit.id(), id);
        assert_eq!(unit.species(), &species);
        assert_eq!(unit.workflow(), &workflow);
        assert_eq!(unit.phase(), target);
        assert_eq!(unit.status(), IntentUnitStatus::Active);
        assert_eq!(
            &unit.history()[..previous_history.len()],
            previous_history.as_slice(),
            "{edge_kind} edge should preserve the full history prefix"
        );
        let Some(LifecycleRecord::Transition(record)) = unit.history().last() else {
            panic!("declared {edge_kind} edge should append a transition record");
        };
        assert_eq!(record.sequence(), expected_sequence);
        assert_eq!(record.from(), expected_from);
        assert_eq!(record.to(), target);
    }
}

#[test]
fn test_guarded_completion_returns_exact_successor_revision() {
    let mut unit = linear_unit();
    unit.transition_to(&phase("doing"))
        .expect("completion phase should be reachable");
    unit.transition_to(&phase("done"))
        .expect("completion phase should be reachable");
    let id = unit.id();
    let species = unit.species().clone();
    let workflow = unit.workflow().clone();
    let final_phase = unit.phase().clone();
    let expected_revision = unit.revision();
    let previous_history = unit.history().to_vec();

    let committed_revision = unit
        .complete_if_revision(expected_revision)
        .expect("current observer should be allowed to complete");
    let exact_successor = IntentUnitRevision::new(
        expected_revision
            .value()
            .checked_add(1)
            .expect("test revision should have a successor"),
    );

    assert_eq!(committed_revision, exact_successor);
    assert_eq!(unit.revision(), committed_revision);
    assert_one_lifecycle_advance(&unit, expected_revision, previous_history.len());
    assert_eq!(unit.id(), id);
    assert_eq!(unit.species(), &species);
    assert_eq!(unit.workflow(), &workflow);
    assert_eq!(unit.phase(), &final_phase);
    assert_eq!(unit.status(), IntentUnitStatus::Completed);
    assert_eq!(
        &unit.history()[..previous_history.len()],
        previous_history.as_slice()
    );
    let Some(LifecycleRecord::Completion(record)) = unit.history().last() else {
        panic!("successful guarded completion should append a completion record");
    };
    assert_eq!(record.sequence(), previous_history.len() + 1);
    assert_eq!(record.final_phase(), &final_phase);
}

#[test]
fn test_revision_conflict_exposes_expected_and_actual() {
    let mut unit = linear_unit();
    let first_observation = unit.revision();
    let second_observation = first_observation;
    unit.transition_to_if_revision(&phase("doing"), first_observation)
        .expect("first observer should commit");

    let error = unit
        .transition_to_if_revision(&phase("done"), second_observation)
        .expect_err("second observer should be stale");

    assert_transition_conflict(&error, second_observation, unit.revision());
    let RevisionedTransitionError::Conflict(conflict) = &error else {
        unreachable!("conflict was asserted above");
    };
    assert_eq!(
        conflict.to_string(),
        "revision conflict: expected 0, actual 1"
    );
    assert_eq!(error.to_string(), conflict.to_string());
}

#[test]
fn test_competing_observers_reject_second_command_and_allow_refresh() {
    let mut unit = linear_unit();
    let first_observation = unit.revision();
    let second_observation = first_observation;

    let first_commit = unit
        .transition_to_if_revision(&phase("doing"), first_observation)
        .expect("first observer should commit");
    let before_stale_command = unit.clone();
    let error = unit
        .transition_to_if_revision(&phase("done"), second_observation)
        .expect_err("second observer should be stale");

    assert_transition_conflict(&error, second_observation, first_commit);
    assert_eq!(unit, before_stale_command);

    let refreshed_revision = match error {
        RevisionedTransitionError::Conflict(conflict) => conflict.actual(),
        RevisionedTransitionError::Transition(_) => unreachable!("conflict was asserted above"),
    };
    let second_commit = unit
        .transition_to_if_revision(&phase("done"), refreshed_revision)
        .expect("refreshed observer should commit");
    assert_eq!(second_commit, IntentUnitRevision::new(2));
    assert_eq!(unit.revision(), second_commit);
    assert_eq!(unit.phase(), &phase("done"));
    assert_eq!(unit.history().len(), 2);
}

#[test]
fn test_stale_revision_rejects_otherwise_valid_command_atomically() {
    let mut unit = linear_unit();
    let stale_revision = unit.revision();
    unit.transition_to(&phase("doing"))
        .expect("fixture should advance beyond the stale observation");
    let actual_revision = unit.revision();
    let before = unit.clone();

    let error = unit
        .transition_to_if_revision(&phase("done"), stale_revision)
        .expect_err("valid command with stale revision should be rejected");

    assert_transition_conflict(&error, stale_revision, actual_revision);
    assert_eq!(unit, before);

    let mut completion_unit = linear_unit();
    completion_unit
        .transition_to(&phase("doing"))
        .expect("completion phase should be reachable");
    let stale_completion_revision = completion_unit.revision();
    completion_unit
        .transition_to(&phase("done"))
        .expect("completion phase should be reachable");
    let actual_completion_revision = completion_unit.revision();
    let before_completion = completion_unit.clone();

    let completion_error = completion_unit
        .complete_if_revision(stale_completion_revision)
        .expect_err("eligible completion with stale revision should be rejected");

    assert_completion_conflict(
        &completion_error,
        stale_completion_revision,
        actual_completion_revision,
    );
    assert_eq!(completion_unit, before_completion);
}

#[test]
fn test_stale_revision_precedes_transition_errors_atomically() {
    let mut unit = linear_unit();
    let stale_revision = unit.revision();
    unit.transition_to(&phase("doing"))
        .expect("fixture should advance beyond the stale observation");
    let actual_revision = unit.revision();

    for target in [phase("missing"), phase("queued")] {
        let before = unit.clone();
        let error = unit
            .transition_to_if_revision(&target, stale_revision)
            .expect_err("stale revision should precede transition validation");

        assert_transition_conflict(&error, stale_revision, actual_revision);
        assert_eq!(unit, before);
    }
}

#[test]
fn test_stale_revision_precedes_completion_errors_atomically() {
    let mut unit = linear_unit();
    let stale_revision = unit.revision();
    unit.transition_to(&phase("doing"))
        .expect("fixture should advance into an ineligible phase");
    let actual_revision = unit.revision();
    let before = unit.clone();

    let error = unit
        .complete_if_revision(stale_revision)
        .expect_err("stale revision should precede completion eligibility");

    assert_completion_conflict(&error, stale_revision, actual_revision);
    assert_eq!(unit, before);
}

#[test]
fn test_stale_revision_precedes_terminal_errors_atomically() {
    let mut unit = linear_unit();
    unit.transition_to(&phase("doing"))
        .expect("completion phase should be reachable");
    unit.transition_to(&phase("done"))
        .expect("completion phase should be reachable");
    let stale_revision = unit.revision();
    unit.complete().expect("fixture should be terminal");
    let actual_revision = unit.revision();

    for target in [phase("queued"), phase("missing")] {
        let before = unit.clone();
        let error = unit
            .transition_to_if_revision(&target, stale_revision)
            .expect_err("stale revision should precede terminal transition validation");

        assert_transition_conflict(&error, stale_revision, actual_revision);
        assert_eq!(unit, before);
    }

    let before = unit.clone();
    let error = unit
        .complete_if_revision(stale_revision)
        .expect_err("stale revision should precede repeated-completion validation");
    assert_completion_conflict(&error, stale_revision, actual_revision);
    assert_eq!(unit, before);
}

#[test]
fn test_current_revision_preserves_domain_errors_atomically() {
    let mut active_unit = linear_unit();
    assert_current_transition_error(
        &mut active_unit,
        &phase("missing"),
        TransitionError::UnknownTarget {
            target: phase("missing"),
        },
    );
    assert_current_transition_error(
        &mut active_unit,
        &phase("done"),
        TransitionError::NotAllowed {
            from: phase("queued"),
            to: phase("done"),
        },
    );
    assert_current_completion_error(
        &mut active_unit,
        CompletionError::PhaseNotEligible {
            phase: phase("queued"),
        },
    );

    let mut completed_unit = linear_unit();
    completed_unit
        .transition_to(&phase("doing"))
        .expect("completion phase should be reachable");
    completed_unit
        .transition_to(&phase("done"))
        .expect("completion phase should be reachable");
    completed_unit.complete().expect("fixture should complete");

    assert_current_transition_error(
        &mut completed_unit,
        &phase("missing"),
        TransitionError::AlreadyCompleted,
    );
    assert_current_completion_error(&mut completed_unit, CompletionError::AlreadyCompleted);
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
