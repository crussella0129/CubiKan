use std::{fs, path::Path, str::FromStr};

use cubikan_backend::{
    BackendError, CompleteIntentUnit, CreateIntentUnit, GetIntentUnit, IntentUnitPage,
    IntentUnitSummary, IntentUnitView, ListCursor, ListCursorError, ListFilters, ListIntentUnits,
    MutationResult, PageLimit, TransitionIntentUnit,
};
use cubikan_core::{
    IntentSpecies, IntentUnit, IntentUnitId, IntentUnitRevision, IntentUnitStatus, PhaseId,
    Workflow, WorkflowEdge, WorkflowId,
};

fn phase(value: &str) -> PhaseId {
    PhaseId::new(value).expect("fixture phase should be valid")
}

fn workflow() -> Workflow {
    let queued = phase("queued");
    let done = phase("done");
    Workflow::new(
        WorkflowId::new("delivery").expect("workflow ID should be valid"),
        vec![queued.clone(), done.clone()],
        queued.clone(),
        vec![WorkflowEdge::new(queued, done.clone())],
        vec![done],
    )
    .expect("workflow should be valid")
}

fn fixed_id(text: &str) -> IntentUnitId {
    text.parse().expect("fixture UUID should be valid")
}

#[test]
fn test_workspace_adds_isolated_backend_crate() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("backend crate should live below the workspace root");
    let root_manifest = fs::read_to_string(workspace.join("Cargo.toml"))
        .expect("workspace manifest should be readable");
    let backend_manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("backend manifest should be readable");
    let core_manifest = fs::read_to_string(workspace.join("crates/cubikan-core/Cargo.toml"))
        .expect("core manifest should be readable");
    let cli_manifest = fs::read_to_string(workspace.join("crates/cubikan-cli/Cargo.toml"))
        .expect("CLI manifest should be readable");

    assert!(root_manifest.contains("\"crates/cubikan-backend\""));
    assert!(backend_manifest.contains("edition.workspace = true"));
    assert!(backend_manifest.contains("cubikan-core = { path = \"../cubikan-core\" }"));
    assert!(backend_manifest.contains("serde.workspace = true"));
    assert!(backend_manifest.contains("serde_json.workspace = true"));
    assert!(backend_manifest.contains("rusqlite.workspace = true"));
    assert!(!core_manifest.contains("rusqlite"));
    assert!(!cli_manifest.contains("rusqlite"));
    assert!(!core_manifest.contains("cubikan-backend"));
    assert!(!cli_manifest.contains("cubikan-backend"));
}

#[test]
fn test_public_backend_model_exposes_complete_commands_and_results() {
    let id = fixed_id("00000000-0000-0000-0000-000000000001");
    let species = IntentSpecies::new("feature").expect("species should be valid");
    let workflow = workflow();
    let create = CreateIntentUnit::new(Some(id), species.clone(), workflow.clone());
    let get = GetIntentUnit::new(id);
    let transition = TransitionIntentUnit::new(id, phase("done"), IntentUnitRevision::INITIAL);
    let complete = CompleteIntentUnit::new(id, IntentUnitRevision::new(1));
    let filters = ListFilters::new(
        Some(workflow.id().clone()),
        Some(species.clone()),
        Some(workflow.initial_phase().clone()),
        Some(IntentUnitStatus::Active),
    );
    let list = ListIntentUnits::new(filters, PageLimit::new(10).unwrap(), None);

    assert_eq!(create.id(), Some(id));
    assert_eq!(create.species(), &species);
    assert_eq!(create.workflow(), &workflow);
    assert_eq!(get.id(), id);
    assert_eq!(transition.id(), id);
    assert_eq!(transition.target(), &phase("done"));
    assert_eq!(transition.expected_revision(), IntentUnitRevision::INITIAL);
    assert_eq!(complete.id(), id);
    assert_eq!(complete.expected_revision(), IntentUnitRevision::new(1));
    assert_eq!(list.limit().value(), 10);
    assert_eq!(list.filters().workflow_id(), Some(workflow.id()));

    let unit = IntentUnit::new(id, species, workflow);
    let view = IntentUnitView::from_intent_unit(&unit);
    let summary = IntentUnitSummary::from_view(&view);
    let cursor = ListCursor::from_str("00000000-0000-0000-0000-000000000001")
        .expect("fixture cursor should be canonical");
    let page = IntentUnitPage::new(vec![summary.clone()], Some(cursor));
    let mutation = MutationResult::new(view.revision(), view.clone());

    assert_eq!(view.id(), id);
    assert_eq!(view.species().as_str(), "feature");
    assert_eq!(view.workflow_id().as_str(), "delivery");
    assert_eq!(view.phase().as_str(), "queued");
    assert_eq!(view.status(), IntentUnitStatus::Active);
    assert_eq!(view.revision(), IntentUnitRevision::INITIAL);
    assert!(view.history().is_empty());
    assert_eq!(summary.id(), id);
    assert_eq!(summary.workflow_id().as_str(), "delivery");
    assert_eq!(summary.species().as_str(), "feature");
    assert_eq!(summary.phase().as_str(), "queued");
    assert_eq!(summary.status(), IntentUnitStatus::Active);
    assert_eq!(summary.revision(), IntentUnitRevision::INITIAL);
    assert_eq!(page.items(), &[summary]);
    assert_eq!(page.next_cursor(), Some(cursor));
    assert_eq!(mutation.committed_revision(), IntentUnitRevision::INITIAL);
    assert_eq!(mutation.intent_unit(), &view);
    assert_eq!(
        BackendError::IntentUnitNotFound { id }.to_string(),
        format!("Intent Unit `{id}` was not found")
    );
}

#[test]
fn test_command_models_preserve_typed_u64_revisions() {
    let id = fixed_id("00000000-0000-0000-0000-000000000001");
    for value in [0, i64::MAX as u64 + 1, u64::MAX] {
        let revision = IntentUnitRevision::new(value);
        let transition = TransitionIntentUnit::new(id, phase("done"), revision);
        let completion = CompleteIntentUnit::new(id, revision);
        assert_eq!(transition.expected_revision().value(), value);
        assert_eq!(completion.expected_revision().value(), value);
    }
}

#[test]
fn test_query_limit_and_cursor_validation() {
    assert_eq!(PageLimit::new(1).unwrap().value(), 1);
    assert_eq!(PageLimit::new(100).unwrap().value(), 100);
    let below = PageLimit::new(0).expect_err("zero should be rejected");
    let above = PageLimit::new(101).expect_err("101 should be rejected");
    assert_eq!(below.value(), 0);
    assert_eq!(above.value(), 101);

    let canonical = "67e55044-10b1-426f-9247-bb680e5fe0c8";
    let nil = "00000000-0000-0000-0000-000000000000";
    assert_eq!(
        ListCursor::from_str(canonical).unwrap().to_string(),
        canonical
    );
    assert_eq!(ListCursor::from_str(nil).unwrap().to_string(), nil);

    for value in [
        "67E55044-10B1-426F-9247-BB680E5FE0C8",
        "67e5504410b1426f9247bb680e5fe0c8",
    ] {
        assert_eq!(
            ListCursor::from_str(value),
            Err(ListCursorError::NonCanonical)
        );
    }
    assert_eq!(
        ListCursor::from_str(" 67e55044-10b1-426f-9247-bb680e5fe0c8"),
        Err(ListCursorError::Malformed)
    );
    assert_eq!(
        ListCursor::from_str("not-a-uuid"),
        Err(ListCursorError::Malformed)
    );
}
