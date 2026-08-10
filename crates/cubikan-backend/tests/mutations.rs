mod common;

use std::{
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

use common::{TestDatabase, fixed_id, phase, stored_rows};
use cubikan_backend::{
    BackendError, CompleteIntentUnit, CreateIntentUnit, SqliteBackend, TransitionIntentUnit,
};
use cubikan_core::{
    CompletionError, IntentSpecies, IntentUnitId, IntentUnitRevision, IntentUnitStatus,
    LifecycleRecord, TransitionError, Workflow, WorkflowEdge, WorkflowId,
};

const UNIT_ID: &str = "60000000-0000-0000-0000-000000000006";
const CORRUPT_ID: &str = "60000000-0000-0000-0000-000000000007";
const MISSING_ID: &str = "60000000-0000-0000-0000-000000000099";

fn revision(value: u64) -> IntentUnitRevision {
    IntentUnitRevision::new(value)
}

fn workflow() -> Workflow {
    let queued = phase("queued");
    let doing = phase("doing");
    let done = phase("done");
    Workflow::new(
        WorkflowId::new("delivery").expect("fixture workflow ID should be valid"),
        vec![done.clone(), queued.clone(), doing.clone()],
        queued.clone(),
        vec![
            WorkflowEdge::new(queued, doing.clone()),
            WorkflowEdge::new(doing, done.clone()),
        ],
        vec![done],
    )
    .expect("fixture workflow should be valid")
}

fn create_unit(backend: &mut SqliteBackend, id: IntentUnitId) {
    backend
        .create(CreateIntentUnit::new(
            Some(id),
            IntentSpecies::new("feature").expect("fixture species should be valid"),
            workflow(),
        ))
        .expect("fixture Intent Unit should create");
}

fn assert_conflict(error: BackendError, expected: u64, actual: u64) {
    let BackendError::RevisionConflict(conflict) = error else {
        panic!("expected revision conflict, got {error:?}");
    };
    assert_eq!(conflict.expected(), revision(expected));
    assert_eq!(conflict.actual(), revision(actual));
}

#[test]
fn test_guarded_transition_and_completion_commit_one_successor_before_return() {
    let database = TestDatabase::new("guarded-successors");
    let id = fixed_id(UNIT_ID);
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    create_unit(&mut backend, id);

    let first = backend
        .transition(TransitionIntentUnit::new(id, phase("doing"), revision(0)))
        .expect("first guarded transition should commit");
    assert_eq!(first.committed_revision(), revision(1));
    assert_eq!(first.intent_unit().revision(), revision(1));
    assert_eq!(first.intent_unit().phase(), &phase("doing"));
    assert_eq!(first.intent_unit().history().len(), 1);
    let fresh = SqliteBackend::open(database.path()).expect("database should reopen after commit");
    assert_eq!(fresh.get(id).unwrap(), *first.intent_unit());

    let second = backend
        .transition(TransitionIntentUnit::new(id, phase("done"), revision(1)))
        .expect("second guarded transition should commit");
    assert_eq!(second.committed_revision(), revision(2));
    assert_eq!(second.intent_unit().revision(), revision(2));
    assert_eq!(second.intent_unit().phase(), &phase("done"));
    assert_eq!(second.intent_unit().history().len(), 2);
    assert_eq!(
        &second.intent_unit().history()[..1],
        first.intent_unit().history()
    );
    let fresh = SqliteBackend::open(database.path()).expect("database should reopen after commit");
    assert_eq!(fresh.get(id).unwrap(), *second.intent_unit());

    let completed = backend
        .complete(CompleteIntentUnit::new(id, revision(2)))
        .expect("guarded completion should commit");
    assert_eq!(completed.committed_revision(), revision(3));
    assert_eq!(completed.intent_unit().revision(), revision(3));
    assert_eq!(
        completed.intent_unit().status(),
        IntentUnitStatus::Completed
    );
    assert_eq!(completed.intent_unit().history().len(), 3);
    assert_eq!(
        &completed.intent_unit().history()[..2],
        second.intent_unit().history()
    );
    let fresh = SqliteBackend::open(database.path()).expect("database should reopen after commit");
    assert_eq!(fresh.get(id).unwrap(), *completed.intent_unit());

    let [
        LifecycleRecord::Transition(first_record),
        LifecycleRecord::Transition(second_record),
        LifecycleRecord::Completion(completion_record),
    ] = completed.intent_unit().history()
    else {
        panic!("committed history should contain two transitions and one completion");
    };
    assert_eq!(first_record.sequence(), 1);
    assert_eq!(first_record.from(), &phase("queued"));
    assert_eq!(first_record.to(), &phase("doing"));
    assert_eq!(second_record.sequence(), 2);
    assert_eq!(second_record.from(), &phase("doing"));
    assert_eq!(second_record.to(), &phase("done"));
    assert_eq!(completion_record.sequence(), 3);
    assert_eq!(completion_record.final_phase(), &phase("done"));

    let rows = stored_rows(&database.connect());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].phase, "done");
    assert_eq!(rows[0].status, "completed");
    assert_eq!(rows[0].revision, 3_u64.to_be_bytes());
}

#[test]
fn test_stale_mutations_win_precedence_and_preserve_durable_state() {
    let database = TestDatabase::new("stale-precedence");
    let id = fixed_id(UNIT_ID);
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    create_unit(&mut backend, id);
    backend
        .transition(TransitionIntentUnit::new(id, phase("doing"), revision(0)))
        .expect("fixture should reach doing");

    let doing = stored_rows(&database.connect());
    let stale_unknown = backend
        .transition(TransitionIntentUnit::new(id, phase("unknown"), revision(0)))
        .expect_err("stale plus unknown target should conflict first");
    assert_conflict(stale_unknown, 0, 1);
    assert_eq!(stored_rows(&database.connect()), doing);

    let stale_not_allowed = backend
        .transition(TransitionIntentUnit::new(id, phase("queued"), revision(0)))
        .expect_err("stale plus undeclared edge should conflict first");
    assert_conflict(stale_not_allowed, 0, 1);
    assert_eq!(stored_rows(&database.connect()), doing);

    let stale_ineligible = backend
        .complete(CompleteIntentUnit::new(id, revision(0)))
        .expect_err("stale plus ineligible completion should conflict first");
    assert_conflict(stale_ineligible, 0, 1);
    assert_eq!(stored_rows(&database.connect()), doing);

    backend
        .transition(TransitionIntentUnit::new(id, phase("done"), revision(1)))
        .expect("fixture should reach done");
    backend
        .complete(CompleteIntentUnit::new(id, revision(2)))
        .expect("fixture should complete");
    let terminal = stored_rows(&database.connect());

    let stale_terminal_transition = backend
        .transition(TransitionIntentUnit::new(id, phase("queued"), revision(2)))
        .expect_err("stale plus terminal transition should conflict first");
    assert_conflict(stale_terminal_transition, 2, 3);
    assert_eq!(stored_rows(&database.connect()), terminal);

    let stale_terminal_completion = backend
        .complete(CompleteIntentUnit::new(id, revision(2)))
        .expect_err("stale plus terminal completion should conflict first");
    assert_conflict(stale_terminal_completion, 2, 3);
    assert_eq!(stored_rows(&database.connect()), terminal);
}

#[test]
fn test_current_revision_missing_corrupt_and_domain_rejections_preserve_state() {
    let database = TestDatabase::new("current-rejections");
    let id = fixed_id(UNIT_ID);
    let missing = fixed_id(MISSING_ID);
    let corrupt = fixed_id(CORRUPT_ID);
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    create_unit(&mut backend, id);

    let initial = stored_rows(&database.connect());
    assert_eq!(
        backend
            .transition(TransitionIntentUnit::new(
                missing,
                phase("doing"),
                revision(0),
            ))
            .expect_err("missing transition target should fail"),
        BackendError::IntentUnitNotFound { id: missing }
    );
    assert_eq!(
        backend
            .complete(CompleteIntentUnit::new(missing, revision(0)))
            .expect_err("missing completion target should fail"),
        BackendError::IntentUnitNotFound { id: missing }
    );
    assert_eq!(stored_rows(&database.connect()), initial);

    let unknown = phase("unknown");
    assert_eq!(
        backend
            .transition(TransitionIntentUnit::new(id, unknown.clone(), revision(0),))
            .expect_err("current unknown target should preserve core rejection"),
        BackendError::TransitionRejected(TransitionError::UnknownTarget { target: unknown })
    );
    assert_eq!(stored_rows(&database.connect()), initial);

    assert_eq!(
        backend
            .transition(TransitionIntentUnit::new(id, phase("done"), revision(0)))
            .expect_err("current undeclared edge should preserve core rejection"),
        BackendError::TransitionRejected(TransitionError::NotAllowed {
            from: phase("queued"),
            to: phase("done"),
        })
    );
    assert_eq!(stored_rows(&database.connect()), initial);

    assert_eq!(
        backend
            .complete(CompleteIntentUnit::new(id, revision(0)))
            .expect_err("current ineligible completion should preserve core rejection"),
        BackendError::CompletionRejected(CompletionError::PhaseNotEligible {
            phase: phase("queued"),
        })
    );
    assert_eq!(stored_rows(&database.connect()), initial);

    backend
        .transition(TransitionIntentUnit::new(id, phase("doing"), revision(0)))
        .expect("fixture should reach doing");
    backend
        .transition(TransitionIntentUnit::new(id, phase("done"), revision(1)))
        .expect("fixture should reach done");
    backend
        .complete(CompleteIntentUnit::new(id, revision(2)))
        .expect("fixture should complete");
    let terminal = stored_rows(&database.connect());
    assert_eq!(
        backend
            .transition(TransitionIntentUnit::new(id, phase("queued"), revision(3)))
            .expect_err("current terminal transition should preserve core rejection"),
        BackendError::TransitionRejected(TransitionError::AlreadyCompleted)
    );
    assert_eq!(
        backend
            .complete(CompleteIntentUnit::new(id, revision(3)))
            .expect_err("current terminal completion should preserve core rejection"),
        BackendError::CompletionRejected(CompletionError::AlreadyCompleted)
    );
    assert_eq!(stored_rows(&database.connect()), terminal);

    create_unit(&mut backend, corrupt);
    database
        .connect()
        .execute(
            "UPDATE intent_units SET envelope='{' WHERE id=?1",
            [corrupt.to_string()],
        )
        .expect("fixture envelope should be corrupted");
    let corrupted = stored_rows(&database.connect());
    assert_eq!(
        backend
            .transition(TransitionIntentUnit::new(
                corrupt,
                phase("doing"),
                revision(0),
            ))
            .expect_err("corrupt transition target should fail closed"),
        BackendError::CorruptEnvelope
    );
    assert_eq!(
        backend
            .complete(CompleteIntentUnit::new(corrupt, revision(0)))
            .expect_err("corrupt completion target should fail closed"),
        BackendError::CorruptEnvelope
    );
    assert_eq!(stored_rows(&database.connect()), corrupted);
}

#[test]
fn test_two_isolated_writers_commit_exactly_once() {
    let database = TestDatabase::new("two-writers");
    let id = fixed_id(UNIT_ID);
    {
        let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
        create_unit(&mut backend, id);
    }

    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let path = database.path().to_path_buf();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let mut backend = SqliteBackend::open(path).expect("writer should open independently");
            let observed = backend.get(id).expect("writer should observe the unit");
            assert_eq!(observed.revision(), revision(0));
            barrier.wait();
            backend.transition(TransitionIntentUnit::new(
                id,
                phase("doing"),
                observed.revision(),
            ))
        }));
    }
    barrier.wait();

    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().expect("writer thread should not panic"))
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    let conflicts = outcomes
        .into_iter()
        .filter_map(Result::err)
        .collect::<Vec<_>>();
    assert_eq!(conflicts.len(), 1);
    assert_conflict(conflicts.into_iter().next().unwrap(), 0, 1);

    let fresh = SqliteBackend::open(database.path()).expect("database should reopen");
    let committed = fresh.get(id).expect("committed successor should load");
    assert_eq!(committed.revision(), revision(1));
    assert_eq!(committed.phase(), &phase("doing"));
    assert_eq!(committed.history().len(), 1);
    assert_eq!(stored_rows(&database.connect()).len(), 1);
}

#[test]
fn test_revision_qualified_zero_row_update_rolls_back() {
    let database = TestDatabase::new("zero-row-cas");
    let id = fixed_id(UNIT_ID);
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    create_unit(&mut backend, id);
    database
        .connect()
        .execute_batch(
            "CREATE TRIGGER fixture_ignore_update
             BEFORE UPDATE ON intent_units
             BEGIN
                 SELECT RAISE(IGNORE);
             END;",
        )
        .expect("zero-row fixture trigger should install");
    let before = stored_rows(&database.connect());

    let error = backend
        .transition(TransitionIntentUnit::new(id, phase("doing"), revision(0)))
        .expect_err("ignored guarded update must fail closed");
    assert_eq!(error, BackendError::ConcurrentStorageChange);
    assert_eq!(stored_rows(&database.connect()), before);
    assert_eq!(backend.get(id).unwrap().revision(), revision(0));
}

#[test]
fn test_busy_writer_times_out_once_without_retry_or_mutation() {
    let database = TestDatabase::new("busy-writer");
    let id = fixed_id(UNIT_ID);
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    create_unit(&mut backend, id);
    let before = stored_rows(&database.connect());

    let lock_holder = database.connect();
    lock_holder
        .execute_batch("BEGIN IMMEDIATE")
        .expect("independent writer should acquire the lock");
    let started = Instant::now();
    let error = backend
        .transition(TransitionIntentUnit::new(id, phase("doing"), revision(0)))
        .expect_err("writer held past the busy timeout should be rejected");
    let elapsed = started.elapsed();
    lock_holder
        .execute_batch("ROLLBACK")
        .expect("fixture writer should release the lock");

    assert!(matches!(error, BackendError::StorageBusy(_)));
    assert!(
        elapsed >= Duration::from_millis(4_500),
        "busy timeout returned too early: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(9),
        "one bounded attempt should not retry: {elapsed:?}"
    );
    assert_eq!(stored_rows(&database.connect()), before);
    assert_eq!(backend.get(id).unwrap().revision(), revision(0));
}

#[test]
fn test_sqlite_update_abort_rolls_back_complete_row() {
    let database = TestDatabase::new("abort-update");
    let id = fixed_id(UNIT_ID);
    let mut backend = SqliteBackend::open(database.path()).expect("database should initialize");
    create_unit(&mut backend, id);
    database
        .connect()
        .execute_batch(
            "CREATE TRIGGER fixture_abort_update
             BEFORE UPDATE ON intent_units
             BEGIN
                 SELECT RAISE(ABORT, 'fixture update abort');
             END;",
        )
        .expect("abort fixture trigger should install");
    let before = stored_rows(&database.connect());

    let error = backend
        .transition(TransitionIntentUnit::new(id, phase("doing"), revision(0)))
        .expect_err("aborted SQLite update should fail");
    assert!(matches!(error, BackendError::Storage(_)));
    assert_eq!(stored_rows(&database.connect()), before);
    assert_eq!(backend.get(id).unwrap().revision(), revision(0));
}
