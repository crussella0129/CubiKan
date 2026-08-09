mod common;

use cubikan_core::{IntentUnit, IntentUnitRevision, LifecycleRecord};
use serde_json::{Value, json};

use common::{linear_unit, phase};

fn active_unit() -> IntentUnit {
    let mut unit = linear_unit();
    unit.transition_to(&phase("doing"))
        .expect("fixture transition should succeed");
    unit
}

fn completed_unit() -> IntentUnit {
    let mut unit = active_unit();
    unit.transition_to(&phase("done"))
        .expect("completion phase should be reachable");
    unit.complete().expect("fixture completion should succeed");
    unit
}

fn serialized_value(unit: &IntentUnit) -> Value {
    serde_json::to_value(unit).expect("Intent Unit should serialize")
}

#[test]
fn test_revision_round_trip_preserves_active_and_completed_units() {
    let active = active_unit();
    let active_json = serde_json::to_string(&active).expect("active unit should serialize");
    let restored_active: IntentUnit =
        serde_json::from_str(&active_json).expect("active unit should restore");

    assert_eq!(restored_active.revision(), IntentUnitRevision::new(1));
    assert_eq!(restored_active.revision(), active.revision());
    assert_eq!(restored_active, active);

    let completed = completed_unit();
    let completed_json =
        serde_json::to_string(&completed).expect("completed unit should serialize");
    let restored_completed: IntentUnit =
        serde_json::from_str(&completed_json).expect("completed unit should restore");

    assert_eq!(restored_completed.revision(), IntentUnitRevision::new(3));
    assert_eq!(restored_completed.revision(), completed.revision());
    assert_eq!(restored_completed, completed);
}

#[test]
fn test_restored_unit_continues_from_exact_revision() {
    let unit = active_unit();
    let json = serde_json::to_string(&unit).expect("active unit should serialize");
    let mut restored: IntentUnit = serde_json::from_str(&json).expect("active unit should restore");
    let restored_revision = restored.revision();

    let committed_revision = restored
        .transition_to_if_revision(&phase("done"), restored_revision)
        .expect("guarded command should accept the restored revision");

    assert_eq!(restored_revision, IntentUnitRevision::new(1));
    assert_eq!(committed_revision, IntentUnitRevision::new(2));
    assert_eq!(restored.revision(), committed_revision);
    assert_eq!(restored.history().len(), 2);
    assert_eq!(restored.history()[1].sequence(), 2);
    let LifecycleRecord::Transition(record) = &restored.history()[1] else {
        panic!("continued lifecycle entry should be a transition");
    };
    assert_eq!(record.from(), &phase("doing"));
    assert_eq!(record.to(), &phase("done"));
}

#[test]
fn test_restore_rejects_missing_or_mismatched_revision() {
    let unit = active_unit();
    let valid = serialized_value(&unit);

    let mut missing = valid.clone();
    missing
        .as_object_mut()
        .expect("serialized aggregate should be an object")
        .remove("revision");
    assert!(serde_json::from_value::<IntentUnit>(missing).is_err());

    let out_of_range: Value = serde_json::from_str("18446744073709551616")
        .expect("out-of-range integer should still parse as a JSON number");
    for non_u64 in [json!(-1), json!(1.5), json!("1"), out_of_range] {
        let mut malformed = valid.clone();
        malformed["revision"] = non_u64;
        assert!(serde_json::from_value::<IntentUnit>(malformed).is_err());
    }

    for mismatched in [json!(0), json!(2)] {
        let mut value = valid.clone();
        value["revision"] = mismatched;
        assert!(serde_json::from_value::<IntentUnit>(value).is_err());
    }
}

#[test]
fn test_restore_rejects_history_phase_or_status_disagreement() {
    let unit = completed_unit();
    let valid = serialized_value(&unit);

    let mut broken_sequence = valid.clone();
    broken_sequence["history"][0]["Transition"]["sequence"] = json!(9);
    let mut broken_source = valid.clone();
    broken_source["history"][0]["Transition"]["from"] = json!("doing");
    let mut broken_completion_phase = valid.clone();
    broken_completion_phase["history"][2]["Completion"]["final_phase"] = json!("doing");
    let mut wrong_final_phase = valid.clone();
    wrong_final_phase["phase"] = json!("doing");
    let mut wrong_final_status = valid.clone();
    wrong_final_status["status"] = json!("Active");

    for corrupted in [
        broken_sequence,
        broken_source,
        broken_completion_phase,
        wrong_final_phase,
        wrong_final_status,
    ] {
        assert_eq!(corrupted["revision"], json!(3));
        assert!(serde_json::from_value::<IntentUnit>(corrupted).is_err());
    }
}

#[test]
fn test_restore_rejects_invalid_workflow_topology() {
    let unit = active_unit();

    let mut invalid_topology = serialized_value(&unit);
    invalid_topology["workflow"]["initial_phase"] = json!("missing");

    assert!(serde_json::from_value::<IntentUnit>(invalid_topology).is_err());
}
