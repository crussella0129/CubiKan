use std::{error::Error, fmt, num::NonZeroU64};

#[cfg(test)]
use std::{ffi::OsStr, path::Path};

use cubikan_core::RelationshipDefinitionKey;
use rusqlite::Connection;

#[cfg(test)]
use rusqlite::OptionalExtension;

use crate::{
    BackendError,
    sqlite::{ProjectionReaderConnection, VerifiedQueryStatement},
};

#[cfg(test)]
use crate::{sqlite::open_projection_reader, stored};

#[cfg(test)]
const READ_CHECKPOINT_SQL: &str = "SELECT block_number,block_hash,last_global_sequence,runtime_spec_version,runtime_code_hash FROM projection_checkpoint WHERE singleton=1";

#[cfg(test)]
type StoredCheckpointFields = (Vec<u8>, Vec<u8>, Option<Vec<u8>>, i64, Vec<u8>);

/// Exact finalized projection checkpoint carried by every verified read result.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ProjectionCheckpoint {
    block_number: u64,
    block_hash: [u8; 32],
    last_global_sequence: Option<NonZeroU64>,
    runtime_spec_version: u32,
    runtime_code_hash: [u8; 32],
}

impl ProjectionCheckpoint {
    /// Constructs a checkpoint from already-validated finalized projection values.
    #[must_use]
    pub const fn new(
        block_number: u64,
        block_hash: [u8; 32],
        last_global_sequence: Option<NonZeroU64>,
        runtime_spec_version: u32,
        runtime_code_hash: [u8; 32],
    ) -> Self {
        Self {
            block_number,
            block_hash,
            last_global_sequence,
            runtime_spec_version,
            runtime_code_hash,
        }
    }

    /// Returns the finalized parachain block number.
    #[must_use]
    pub const fn block_number(&self) -> u64 {
        self.block_number
    }

    /// Returns the finalized parachain block hash.
    #[must_use]
    pub const fn block_hash(&self) -> &[u8; 32] {
        &self.block_hash
    }

    /// Returns the latest accepted CubiKan event sequence, when one exists.
    #[must_use]
    pub const fn last_global_sequence(&self) -> Option<NonZeroU64> {
        self.last_global_sequence
    }

    /// Returns the runtime specification version at this checkpoint.
    #[must_use]
    pub const fn runtime_spec_version(&self) -> u32 {
        self.runtime_spec_version
    }

    /// Returns the runtime code hash at this checkpoint.
    #[must_use]
    pub const fn runtime_code_hash(&self) -> &[u8; 32] {
        &self.runtime_code_hash
    }
}

/// Failure from issuing or consuming a verified projection read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadError {
    /// The pinned projection failed its local storage boundary.
    Backend(BackendError),
    /// The candidate checkpoint advanced before its read transaction was pinned.
    RefreshRequired,
    /// The schema is valid but has not projected its first finalized checkpoint.
    ProjectionUnavailable,
    /// The requested immutable relationship definition is absent.
    RelationshipDefinitionNotFound {
        definition: RelationshipDefinitionKey,
    },
}

impl fmt::Display for ReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => error.fmt(formatter),
            Self::RefreshRequired => {
                formatter.write_str("the projection advanced before the read snapshot was pinned")
            }
            Self::ProjectionUnavailable => {
                formatter.write_str("the projection has no finalized checkpoint")
            }
            Self::RelationshipDefinitionNotFound { definition } => {
                write!(
                    formatter,
                    "relationship definition {definition:?} was not found"
                )
            }
        }
    }
}

impl Error for ReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::RefreshRequired
            | Self::ProjectionUnavailable
            | Self::RelationshipDefinitionNotFound { .. } => None,
        }
    }
}

impl From<BackendError> for ReadError {
    fn from(error: BackendError) -> Self {
        Self::Backend(error)
    }
}

/// A single-use capability for one query against an attested, pinned snapshot.
///
/// The capability owns its read-only SQLite connection and transaction. It is
/// deliberately neither clonable nor serializable, and every public query
/// consumes it. Callers cannot construct one from a path, connection, database
/// contents, checkpoint, or token.
///
/// ```compile_fail
/// use cubikan_backend::VerifiedReadSnapshot;
///
/// // Construction is reserved for the backend's finalized-stream attestor.
/// let _snapshot = VerifiedReadSnapshot::new();
/// ```
///
/// ```compile_fail
/// use cubikan_backend::VerifiedReadSnapshot;
///
/// fn duplicate(snapshot: VerifiedReadSnapshot) {
///     let _copy = snapshot.clone();
/// }
/// ```
///
/// ```compile_fail
/// use cubikan_backend::VerifiedReadSnapshot;
///
/// fn require_serializable<T: serde::Serialize>() {}
/// require_serializable::<VerifiedReadSnapshot>();
/// ```
pub struct VerifiedReadSnapshot {
    reader: ProjectionReaderConnection,
    checkpoint: ProjectionCheckpoint,
    _data_version: i64,
}

impl fmt::Debug for VerifiedReadSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedReadSnapshot")
            .field("checkpoint", &self.checkpoint)
            .field("capability", &"opaque single-use read")
            .finish_non_exhaustive()
    }
}

impl VerifiedReadSnapshot {
    pub(crate) fn consume<R, F>(self, operation: F) -> Result<R, ReadError>
    where
        F: FnOnce(&Connection, &ProjectionCheckpoint) -> Result<R, ReadError>,
    {
        let Self {
            mut reader,
            checkpoint,
            _data_version: _,
        } = self;
        let mut query_error = None;
        let operation_result = reader
            .with_verified_query(VerifiedQueryStatement::OneBoundedRead, |connection| {
                match operation(connection, &checkpoint) {
                    Ok(value) => Ok(Some(value)),
                    Err(error) => {
                        query_error = Some(error);
                        Ok(None)
                    }
                }
            })
            .map_err(ReadError::Backend)
            .and_then(|value| match value {
                Some(value) => Ok(value),
                None => Err(query_error
                    .take()
                    .expect("a consumed query without a value must retain its typed error")),
            });
        let rollback_result = reader.rollback_verified_read().map_err(ReadError::Backend);

        match operation_result {
            Ok(value) => {
                rollback_result?;
                Ok(value)
            }
            Err(error) => {
                let _ = rollback_result;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn issue_test_snapshot(
    directory: &Path,
    basename: &OsStr,
    candidate: &ProjectionCheckpoint,
) -> Result<VerifiedReadSnapshot, ReadError> {
    let mut reader = open_projection_reader(directory, basename).map_err(ReadError::Backend)?;
    let data_version = reader.data_version().map_err(ReadError::Backend)?;
    reader.begin_verified_read().map_err(ReadError::Backend)?;

    let pinned = reader
        .with_verified_query(
            VerifiedQueryStatement::FullProjectionCompare,
            read_checkpoint,
        )
        .map_err(ReadError::Backend)?;
    let Some(checkpoint) = pinned else {
        let _ = reader.rollback_verified_read();
        return Err(ReadError::ProjectionUnavailable);
    };
    if checkpoint != *candidate {
        let _ = reader.rollback_verified_read();
        return Err(ReadError::RefreshRequired);
    }

    Ok(VerifiedReadSnapshot {
        reader,
        checkpoint,
        _data_version: data_version,
    })
}

#[cfg(test)]
fn read_checkpoint(connection: &Connection) -> Result<Option<ProjectionCheckpoint>, BackendError> {
    connection
        .query_row(READ_CHECKPOINT_SQL, [], |row| {
            let block_number = row.get::<_, Vec<u8>>(0)?;
            let block_hash = row.get::<_, Vec<u8>>(1)?;
            let last_global_sequence = row.get::<_, Option<Vec<u8>>>(2)?;
            let runtime_spec_version = row.get::<_, i64>(3)?;
            let runtime_code_hash = row.get::<_, Vec<u8>>(4)?;
            Ok((
                block_number,
                block_hash,
                last_global_sequence,
                runtime_spec_version,
                runtime_code_hash,
            ))
        })
        .optional()
        .map_err(crate::sqlite::classify_runtime_error)?
        .map(decode_checkpoint)
        .transpose()
}

#[cfg(test)]
fn decode_checkpoint(stored: StoredCheckpointFields) -> Result<ProjectionCheckpoint, BackendError> {
    let (block_number, block_hash, last_global_sequence, runtime_spec_version, runtime_code_hash) =
        stored;
    let block_number =
        stored::decode_u64_blob(&block_number).map_err(|_| BackendError::ProjectionMismatch)?;
    let block_hash = decode_hash(block_hash)?;
    let last_global_sequence = last_global_sequence
        .map(|sequence| {
            stored::decode_u64_blob(&sequence)
                .map_err(|_| BackendError::ProjectionMismatch)
                .and_then(|sequence| {
                    NonZeroU64::new(sequence).ok_or(BackendError::ProjectionMismatch)
                })
        })
        .transpose()?;
    let runtime_spec_version =
        u32::try_from(runtime_spec_version).map_err(|_| BackendError::ProjectionMismatch)?;
    let runtime_code_hash = decode_hash(runtime_code_hash)?;

    Ok(ProjectionCheckpoint::new(
        block_number,
        block_hash,
        last_global_sequence,
        runtime_spec_version,
        runtime_code_hash,
    ))
}

#[cfg(test)]
fn decode_hash(bytes: Vec<u8>) -> Result<[u8; 32], BackendError> {
    bytes
        .try_into()
        .map_err(|_| BackendError::ProjectionMismatch)
}
