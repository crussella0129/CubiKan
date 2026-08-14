use std::str::FromStr;

use cubikan_core::{
    ExternalReference, IntentSpecies, IntentUnit, IntentUnitId, PhaseId, ReferenceNamespace,
    ReferenceText, Workflow, WorkflowEdge, WorkflowId,
};

pub fn fixed_id() -> IntentUnitId {
    IntentUnitId::from_str("67e55044-10b1-426f-9247-bb680e5fe0c8").expect("fixed ID should parse")
}

pub fn phase(value: &str) -> PhaseId {
    PhaseId::new(value).expect("fixture phase should be valid")
}

pub fn species() -> IntentSpecies {
    IntentSpecies::new("feature").expect("fixture species should be valid")
}

pub fn origin() -> ExternalReference {
    ExternalReference::new(
        ReferenceNamespace::new("book.intent").expect("fixture namespace should be valid"),
        ReferenceText::new("core-tests").expect("fixture scope should be valid"),
        ReferenceText::new("INT-0008").expect("fixture value should be valid"),
    )
}

pub fn linear_workflow() -> Workflow {
    let queued = phase("queued");
    let doing = phase("doing");
    let done = phase("done");
    Workflow::new(
        WorkflowId::new("delivery").expect("fixture workflow ID should be valid"),
        vec![queued.clone(), doing.clone(), done.clone()],
        queued.clone(),
        vec![
            WorkflowEdge::new(queued, doing.clone()),
            WorkflowEdge::new(doing, done.clone()),
        ],
        vec![done],
    )
    .expect("fixture workflow should be valid")
}

pub fn linear_unit() -> IntentUnit {
    IntentUnit::new(fixed_id(), origin(), species(), linear_workflow())
}
