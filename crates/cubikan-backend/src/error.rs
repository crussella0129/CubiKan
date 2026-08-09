use std::{error::Error, fmt};

use cubikan_core::{CompletionError, IntentUnitId, RevisionConflict, TransitionError};

/// Rejection from constructing a bounded list limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageLimitError {
    value: usize,
}

impl PageLimitError {
    pub(crate) const fn new(value: usize) -> Self {
        Self { value }
    }

    /// Returns the rejected value.
    #[must_use]
    pub const fn value(self) -> usize {
        self.value
    }
}

impl fmt::Display for PageLimitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "list limit {} is outside 1..=100", self.value)
    }
}

impl Error for PageLimitError {}

/// Rejection from parsing a canonical keyset cursor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListCursorError {
    /// The text is not a syntactically valid Intent Unit UUID.
    Malformed,
    /// The text parses, but is not canonical lowercase hyphenated UUID text.
    NonCanonical,
}

impl fmt::Display for ListCursorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => formatter.write_str("list cursor is not a valid Intent Unit ID"),
            Self::NonCanonical => {
                formatter.write_str("list cursor is not canonical Intent Unit ID text")
            }
        }
    }
}

impl Error for ListCursorError {}

/// Typed failure returned by the durable backend boundary.
///
/// Storage-specific variants are populated by later persistence tasks. The
/// enum is intentionally adapter-owned so callers never need to classify raw
/// SQLite messages or provisional core serialization errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendError {
    /// Create attempted to reuse an existing immutable identity.
    DuplicateIntentUnit { id: IntentUnitId },
    /// The requested identity is not present.
    IntentUnitNotFound { id: IntentUnitId },
    /// A guarded command observed a different aggregate revision.
    RevisionConflict(RevisionConflict),
    /// Core transition validation rejected a current-revision command.
    TransitionRejected(TransitionError),
    /// Core completion validation rejected a current-revision command.
    CompletionRejected(CompletionError),
    /// The database contains user-owned state that CubiKan cannot adopt.
    UnownedDatabase,
    /// The SQLite schema version is not supported.
    UnsupportedSchemaVersion { found: i64 },
    /// Schema version 1 does not match the exact owned shape.
    CorruptSchema,
    /// The stored envelope version is not supported.
    UnsupportedEnvelopeVersion { found: u64 },
    /// A stored envelope is malformed or cannot be replayed.
    CorruptEnvelope,
    /// A checked SQL projection disagrees with the replayed aggregate.
    ProjectionMismatch,
    /// SQLite could not acquire its local writer within the configured bound.
    StorageBusy,
    /// A revision-qualified update violated the backend's CAS invariant.
    ConcurrentStorageChange,
    /// Another SQLite or local-filesystem operation failed.
    Storage,
}

impl fmt::Display for BackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateIntentUnit { id } => {
                write!(formatter, "Intent Unit `{id}` already exists")
            }
            Self::IntentUnitNotFound { id } => {
                write!(formatter, "Intent Unit `{id}` was not found")
            }
            Self::RevisionConflict(error) => error.fmt(formatter),
            Self::TransitionRejected(error) => error.fmt(formatter),
            Self::CompletionRejected(error) => error.fmt(formatter),
            Self::UnownedDatabase => {
                formatter.write_str("database is not an owned CubiKan database")
            }
            Self::UnsupportedSchemaVersion { found } => {
                write!(formatter, "unsupported CubiKan schema version {found}")
            }
            Self::CorruptSchema => formatter.write_str("CubiKan schema is malformed"),
            Self::UnsupportedEnvelopeVersion { found } => {
                write!(formatter, "unsupported stored envelope version {found}")
            }
            Self::CorruptEnvelope => formatter.write_str("stored Intent Unit envelope is corrupt"),
            Self::ProjectionMismatch => {
                formatter.write_str("stored Intent Unit projection disagrees with its envelope")
            }
            Self::StorageBusy => formatter.write_str("CubiKan storage is busy"),
            Self::ConcurrentStorageChange => {
                formatter.write_str("stored revision changed during guarded update")
            }
            Self::Storage => formatter.write_str("CubiKan storage operation failed"),
        }
    }
}

impl Error for BackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RevisionConflict(error) => Some(error),
            Self::TransitionRejected(error) => Some(error),
            Self::CompletionRejected(error) => Some(error),
            _ => None,
        }
    }
}
