//! Deterministic conversion from bounded lifecycle semantics to pallet errors.

use crate::{
    pallet::{Config, Error},
    types::LifecycleError,
};

pub(crate) fn map_lifecycle_error<T: Config>(error: LifecycleError) -> Error<T> {
    match error {
        LifecycleError::RevisionConflict { .. } => Error::<T>::StaleRevision,
        LifecycleError::HistoryCapacityExceeded { .. } => {
            Error::<T>::LifecycleHistoryCapacityExceeded
        }
        LifecycleError::RevisionExhausted => Error::<T>::LifecycleRevisionExhausted,
        LifecycleError::AlreadyCompleted => Error::<T>::IntentUnitAlreadyCompleted,
        LifecycleError::UnknownTarget { .. } => Error::<T>::UnknownTargetPhase,
        LifecycleError::TransitionNotAllowed { .. } => Error::<T>::TransitionNotAllowed,
        LifecycleError::CompletionPhaseNotEligible { .. } => Error::<T>::CompletionPhaseNotEligible,
    }
}
