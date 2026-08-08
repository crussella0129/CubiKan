mod common;

use cubikan_core::IntentUnit;
use serde_json::json;

use common::{linear_unit, phase};

#[test]
fn test_serialize_restore_and_continue_lifecycle() {
    let mut expected = linear_unit();
    expected
        .transition_to(&phase("doing"))
        .expect("first transition should succeed");
    let json = serde_json::to_string(&expected).expect("Intent Unit should serialize");
    let mut restored: IntentUnit =
        serde_json::from_str(&json).expect("valid Intent Unit should restore");

    expected
        .transition_to(&phase("done"))
        .expect("expected unit should continue");
    expected.complete().expect("expected unit should complete");
    restored
        .transition_to(&phase("done"))
        .expect("restored unit should continue");
    restored.complete().expect("restored unit should complete");

    assert_eq!(restored, expected);
}

#[test]
fn test_tampered_serialized_aggregate_is_rejected() {
    let mut unit = linear_unit();
    unit.transition_to(&phase("doing"))
        .expect("fixture transition should succeed");

    let mut invalid_topology = serde_json::to_value(&unit).expect("Intent Unit should serialize");
    invalid_topology["workflow"]["initial_phase"] = json!("missing");
    let mut invalid_history = serde_json::to_value(&unit).expect("Intent Unit should serialize");
    invalid_history["history"][0]["Transition"]["sequence"] = json!(9);

    assert!(serde_json::from_value::<IntentUnit>(invalid_topology).is_err());
    assert!(serde_json::from_value::<IntentUnit>(invalid_history).is_err());
}
