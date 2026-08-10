use std::path::Path;

use cubikan_backend::{BackendError, SqliteBackend};
use cubikan_core::{CompletionError, TransitionError};

use crate::protocol::{
    ErrorCode, ExecutedRequest, ProtocolFailure, ProtocolResult, ValidatedOperation,
    decode_request, failure_response, success_response,
};

/// Validates and executes one protocol request against one explicit database.
///
/// The complete request is syntactically, structurally, and semantically
/// validated before the selected path is opened.
#[must_use]
pub fn execute_request(path: impl AsRef<Path>, request: &[u8]) -> ExecutedRequest {
    let operation = match decode_request(request) {
        Ok(operation) => operation,
        Err(error) => return failure_response(error),
    };
    let mut backend = match SqliteBackend::open(path) {
        Ok(backend) => backend,
        Err(error) => return failure_response(map_backend_error(error)),
    };

    let result = match operation {
        ValidatedOperation::Create(command) => backend.create(command).map(ProtocolResult::Unit),
        ValidatedOperation::Get(id) => backend.get(id).map(ProtocolResult::Unit),
        ValidatedOperation::List(command) => backend.list(command).map(ProtocolResult::Page),
        ValidatedOperation::Transition(command) => {
            backend.transition(command).map(ProtocolResult::Mutation)
        }
        ValidatedOperation::Complete(command) => {
            backend.complete(command).map(ProtocolResult::Mutation)
        }
    };
    match result {
        Ok(result) => success_response(result),
        Err(error) => failure_response(map_backend_error(error)),
    }
}

fn map_backend_error(error: BackendError) -> ProtocolFailure {
    let message = error.to_string();
    match error {
        BackendError::DuplicateIntentUnit { .. } => {
            ProtocolFailure::plain(ErrorCode::DuplicateIntentUnit, message)
        }
        BackendError::IntentUnitNotFound { .. } => {
            ProtocolFailure::plain(ErrorCode::IntentUnitNotFound, message)
        }
        BackendError::RevisionConflict(conflict) => {
            ProtocolFailure::conflict(message, conflict.expected(), conflict.actual())
        }
        BackendError::TransitionRejected(TransitionError::AlreadyCompleted) => {
            ProtocolFailure::plain(ErrorCode::TransitionAlreadyCompleted, message)
        }
        BackendError::TransitionRejected(TransitionError::UnknownTarget { .. }) => {
            ProtocolFailure::plain(ErrorCode::TransitionUnknownTarget, message)
        }
        BackendError::TransitionRejected(TransitionError::NotAllowed { .. }) => {
            ProtocolFailure::plain(ErrorCode::TransitionNotAllowed, message)
        }
        BackendError::CompletionRejected(CompletionError::AlreadyCompleted) => {
            ProtocolFailure::plain(ErrorCode::CompletionAlreadyCompleted, message)
        }
        BackendError::CompletionRejected(CompletionError::PhaseNotEligible { .. }) => {
            ProtocolFailure::plain(ErrorCode::CompletionPhaseNotEligible, message)
        }
        BackendError::StorageBusy(_) => ProtocolFailure::plain(ErrorCode::StorageBusy, message),
        BackendError::UnownedDatabase => {
            ProtocolFailure::plain(ErrorCode::UnownedDatabase, message)
        }
        BackendError::UnsupportedSchemaVersion { .. } => {
            ProtocolFailure::plain(ErrorCode::UnsupportedSchemaVersion, message)
        }
        BackendError::CorruptSchema => ProtocolFailure::plain(ErrorCode::CorruptSchema, message),
        BackendError::UnsupportedEnvelopeVersion { .. } => {
            ProtocolFailure::plain(ErrorCode::UnsupportedEnvelopeVersion, message)
        }
        BackendError::CorruptEnvelope => {
            ProtocolFailure::plain(ErrorCode::CorruptEnvelope, message)
        }
        BackendError::ProjectionMismatch => {
            ProtocolFailure::plain(ErrorCode::ProjectionMismatch, message)
        }
        BackendError::ConcurrentStorageChange => {
            ProtocolFailure::plain(ErrorCode::ConcurrentStorageChange, message)
        }
        BackendError::Storage(_) => ProtocolFailure::plain(ErrorCode::StorageError, message),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use cubikan_backend::{CreateIntentUnit, TransitionIntentUnit};
    use cubikan_core::{
        IntentSpecies, IntentUnitId, IntentUnitRevision, PhaseId, Workflow, WorkflowEdge,
        WorkflowId,
    };
    use serde_json::Value;

    use super::*;
    use crate::protocol::{PROTOCOL_VERSION, ResponseClass};

    const BACKEND_MAPPING_CASES: usize = 17;
    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            for _ in 0..100 {
                let ordinal = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "cubikan-local-backend-error-mapping-{}-{ordinal}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => panic!("test directory should be created: {error}"),
                }
            }
            panic!("could not allocate a unique test directory");
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[derive(Clone, Copy)]
    struct ExpectedMapping {
        ordinal: usize,
        code: &'static str,
        class: ResponseClass,
        expected_revision: Option<&'static str>,
        actual_revision: Option<&'static str>,
    }

    fn expected_mapping(error: &BackendError) -> ExpectedMapping {
        let (ordinal, code, class, expected_revision, actual_revision) = match error {
            BackendError::DuplicateIntentUnit { .. } => (
                0,
                "duplicate_intent_unit",
                ResponseClass::CommandRejected,
                None,
                None,
            ),
            BackendError::IntentUnitNotFound { .. } => (
                1,
                "intent_unit_not_found",
                ResponseClass::CommandRejected,
                None,
                None,
            ),
            BackendError::RevisionConflict(_) => (
                2,
                "revision_conflict",
                ResponseClass::CommandRejected,
                Some("9"),
                Some("0"),
            ),
            BackendError::TransitionRejected(TransitionError::AlreadyCompleted) => (
                3,
                "transition_already_completed",
                ResponseClass::CommandRejected,
                None,
                None,
            ),
            BackendError::TransitionRejected(TransitionError::UnknownTarget { .. }) => (
                4,
                "transition_unknown_target",
                ResponseClass::CommandRejected,
                None,
                None,
            ),
            BackendError::TransitionRejected(TransitionError::NotAllowed { .. }) => (
                5,
                "transition_not_allowed",
                ResponseClass::CommandRejected,
                None,
                None,
            ),
            BackendError::CompletionRejected(CompletionError::AlreadyCompleted) => (
                6,
                "completion_already_completed",
                ResponseClass::CommandRejected,
                None,
                None,
            ),
            BackendError::CompletionRejected(CompletionError::PhaseNotEligible { .. }) => (
                7,
                "completion_phase_not_eligible",
                ResponseClass::CommandRejected,
                None,
                None,
            ),
            BackendError::UnownedDatabase => (
                8,
                "unowned_database",
                ResponseClass::StorageRejected,
                None,
                None,
            ),
            BackendError::UnsupportedSchemaVersion { .. } => (
                9,
                "unsupported_schema_version",
                ResponseClass::StorageRejected,
                None,
                None,
            ),
            BackendError::CorruptSchema => (
                10,
                "corrupt_schema",
                ResponseClass::StorageRejected,
                None,
                None,
            ),
            BackendError::UnsupportedEnvelopeVersion { .. } => (
                11,
                "unsupported_envelope_version",
                ResponseClass::StorageRejected,
                None,
                None,
            ),
            BackendError::CorruptEnvelope => (
                12,
                "corrupt_envelope",
                ResponseClass::StorageRejected,
                None,
                None,
            ),
            BackendError::ProjectionMismatch => (
                13,
                "projection_mismatch",
                ResponseClass::StorageRejected,
                None,
                None,
            ),
            BackendError::StorageBusy(_) => (
                14,
                "storage_busy",
                ResponseClass::StorageRejected,
                None,
                None,
            ),
            BackendError::ConcurrentStorageChange => (
                15,
                "concurrent_storage_change",
                ResponseClass::StorageRejected,
                None,
                None,
            ),
            BackendError::Storage(_) => (
                16,
                "storage_error",
                ResponseClass::StorageRejected,
                None,
                None,
            ),
        };
        ExpectedMapping {
            ordinal,
            code,
            class,
            expected_revision,
            actual_revision,
        }
    }

    fn phase(value: &str) -> PhaseId {
        PhaseId::new(value).expect("fixture phase should be valid")
    }

    fn assert_optional_revision(
        error: &serde_json::Map<String, Value>,
        field: &str,
        expected: Option<&str>,
    ) {
        match expected {
            Some(expected) => assert_eq!(error.get(field).and_then(Value::as_str), Some(expected)),
            None => assert!(!error.contains_key(field), "{field} must be absent"),
        }
    }

    #[test]
    fn test_backend_errors_map_exhaustively_to_protocol_codes() {
        let directory = TestDirectory::new();
        let database = directory.path().join("cubikan.sqlite3");
        let id: IntentUnitId = "70000000-0000-0000-0000-000000000007"
            .parse()
            .expect("fixture ID should parse");
        let queued = phase("queued");
        let doing = phase("doing");
        let workflow = Workflow::new(
            WorkflowId::new("delivery").expect("fixture workflow ID should be valid"),
            [queued.clone(), doing.clone()],
            queued.clone(),
            [WorkflowEdge::new(queued.clone(), doing.clone())],
            [doing.clone()],
        )
        .expect("fixture workflow should be valid");
        let mut backend = SqliteBackend::open(&database).expect("fixture database should open");
        backend
            .create(CreateIntentUnit::new(
                Some(id),
                IntentSpecies::new("feature").expect("fixture species should be valid"),
                workflow,
            ))
            .expect("fixture unit should be created");
        let revision_conflict = backend
            .transition(TransitionIntentUnit::new(
                id,
                doing.clone(),
                IntentUnitRevision::new(9),
            ))
            .expect_err("stale fixture command should conflict");
        assert!(matches!(
            &revision_conflict,
            BackendError::RevisionConflict(_)
        ));

        let unavailable = directory.path().join("missing-parent/cubikan.sqlite3");
        let storage_failure = match SqliteBackend::open(unavailable)
            .expect_err("missing parent should produce a storage failure")
        {
            BackendError::Storage(failure) => failure,
            other => panic!("expected storage failure, got {other:?}"),
        };

        let cases = vec![
            BackendError::DuplicateIntentUnit { id },
            BackendError::IntentUnitNotFound { id },
            revision_conflict,
            BackendError::TransitionRejected(TransitionError::AlreadyCompleted),
            BackendError::TransitionRejected(TransitionError::UnknownTarget {
                target: phase("unknown"),
            }),
            BackendError::TransitionRejected(TransitionError::NotAllowed {
                from: queued.clone(),
                to: doing.clone(),
            }),
            BackendError::CompletionRejected(CompletionError::AlreadyCompleted),
            BackendError::CompletionRejected(CompletionError::PhaseNotEligible { phase: queued }),
            BackendError::UnownedDatabase,
            BackendError::UnsupportedSchemaVersion { found: 2 },
            BackendError::CorruptSchema,
            BackendError::UnsupportedEnvelopeVersion { found: 2 },
            BackendError::CorruptEnvelope,
            BackendError::ProjectionMismatch,
            BackendError::StorageBusy(storage_failure.clone()),
            BackendError::ConcurrentStorageChange,
            BackendError::Storage(storage_failure),
        ];
        assert_eq!(cases.len(), BACKEND_MAPPING_CASES);

        let mut seen = [false; BACKEND_MAPPING_CASES];
        for error in cases {
            let expected = expected_mapping(&error);
            assert!(
                !std::mem::replace(&mut seen[expected.ordinal], true),
                "mapping case {} was exercised more than once",
                expected.ordinal
            );
            let expected_message = error.to_string();
            let response = failure_response(map_backend_error(error));
            assert_eq!(response.class(), expected.class);

            let body: Value = serde_json::from_slice(response.body())
                .expect("mapped response should be valid JSON");
            assert_eq!(body["protocol_version"], PROTOCOL_VERSION);
            assert_eq!(body["outcome"], "failure");
            let mapped = body["error"]
                .as_object()
                .expect("mapped response should contain an error object");
            assert_eq!(mapped["code"], expected.code);
            assert_eq!(mapped["message"], expected_message);
            assert!(!mapped.contains_key("field"));
            assert_optional_revision(mapped, "expected_revision", expected.expected_revision);
            assert_optional_revision(mapped, "actual_revision", expected.actual_revision);
        }
        assert!(seen.into_iter().all(|was_seen| was_seen));
    }
}
