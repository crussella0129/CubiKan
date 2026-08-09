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
