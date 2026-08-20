use std::{error::Error, fmt};

use cubikan_chain_client::{ArchiveError, VerifiedArchiveClient};

use crate::{
    BackendError, FinalizedProjector, ProjectionCheckpoint, ProjectionError, VerifiedReadSnapshot,
    projector::{
        PreparedArchive, fetch_prepared_archive_through, load_projection_checkpoint,
        load_stored_projection_if_checkpoint,
    },
    sqlite::{ProjectionReaderConnection, VerifiedQueryStatement, open_projection_reader},
    verified_read::mint_attested_snapshot,
};

/// Failure while comparing a complete verified archive range to one pinned
/// schema-v3 read transaction.
#[derive(Debug)]
pub enum AttestationError {
    /// The configured verified archive source failed before SQLite was opened.
    Archive(ArchiveError),
    /// The finalized stream failed independent continuity or domain replay.
    InvalidFinalizedStream,
    /// The hardened local projection boundary rejected the read.
    Backend(BackendError),
    /// No first finalized block has been projected yet.
    ProjectionUnavailable,
    /// The database checkpoint did not equal the candidate fetched before the
    /// read transaction was pinned.
    RefreshRequired,
    /// One or more block, raw event, joined coordinate, derived row, envelope,
    /// anchor, or checkpoint values disagreed.
    ProjectionMismatch,
}

impl fmt::Display for AttestationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Archive(error) => error.fmt(formatter),
            Self::InvalidFinalizedStream => {
                formatter.write_str("the finalized CubiKan event stream is inconsistent")
            }
            Self::Backend(error) => error.fmt(formatter),
            Self::ProjectionUnavailable => {
                formatter.write_str("the projection has no finalized checkpoint")
            }
            Self::RefreshRequired => {
                formatter.write_str("the projection changed before attestation was pinned")
            }
            Self::ProjectionMismatch => {
                formatter.write_str("the pinned projection does not equal the finalized archive")
            }
        }
    }
}

impl Error for AttestationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Archive(error) => Some(error),
            Self::Backend(error) => Some(error),
            Self::InvalidFinalizedStream
            | Self::ProjectionUnavailable
            | Self::RefreshRequired
            | Self::ProjectionMismatch => None,
        }
    }
}

impl From<BackendError> for AttestationError {
    fn from(error: BackendError) -> Self {
        Self::Backend(error)
    }
}

impl From<ProjectionError> for AttestationError {
    fn from(error: ProjectionError) -> Self {
        match error {
            ProjectionError::Archive(error) => Self::Archive(error),
            ProjectionError::Backend(error) => Self::Backend(error),
            ProjectionError::ConflictingFinalizedBlock => Self::ProjectionMismatch,
            ProjectionError::RefreshRequired => Self::RefreshRequired,
            ProjectionError::InvalidFinalizedStream => Self::InvalidFinalizedStream,
        }
    }
}

/// Reads the current database candidate, fetches and independently replays the
/// exact verified archive range `0..=candidate` outside SQLite, then compares
/// all eight projection tables inside one newly pinned read transaction before
/// minting a single-use query capability.
pub async fn attest_finalized_projection(
    projector: &FinalizedProjector,
    client: &VerifiedArchiveClient,
) -> Result<VerifiedReadSnapshot, AttestationError> {
    attest_finalized_projection_from(projector, client, || Ok(())).await
}

trait AttestationArchiveSource {
    async fn fetch_prepared_through(
        &self,
        block_number: u64,
    ) -> Result<PreparedArchive, ProjectionError>;
}

impl AttestationArchiveSource for VerifiedArchiveClient {
    async fn fetch_prepared_through(
        &self,
        block_number: u64,
    ) -> Result<PreparedArchive, ProjectionError> {
        fetch_prepared_archive_through(self, block_number).await
    }
}

async fn attest_finalized_projection_from<S, F>(
    projector: &FinalizedProjector,
    source: &S,
    before_pin: F,
) -> Result<VerifiedReadSnapshot, AttestationError>
where
    S: AttestationArchiveSource,
    F: FnOnce() -> Result<(), AttestationError>,
{
    let candidate = read_candidate_checkpoint(projector)?;
    let archive = source
        .fetch_prepared_through(candidate.block_number())
        .await?;
    if archive.checkpoint()? != candidate {
        return Err(AttestationError::ProjectionMismatch);
    }
    attest_prepared_projection_at_candidate(projector, &archive, candidate, before_pin)
}

fn read_candidate_checkpoint(
    projector: &FinalizedProjector,
) -> Result<ProjectionCheckpoint, AttestationError> {
    let (directory, basename) = projector.parts()?;
    let mut reader = open_projection_reader(directory, basename)?;
    reader.begin_verified_read()?;
    let result = reader.with_verified_query(
        VerifiedQueryStatement::FullProjectionCompare,
        load_projection_checkpoint,
    );
    finish_ephemeral_read(&mut reader, result)?.ok_or(AttestationError::ProjectionUnavailable)
}

fn finish_ephemeral_read<T>(
    reader: &mut ProjectionReaderConnection,
    result: Result<T, BackendError>,
) -> Result<T, AttestationError> {
    let rollback = reader.rollback_verified_read();
    match result {
        Ok(value) => {
            rollback?;
            Ok(value)
        }
        Err(error) => {
            let _ = rollback;
            Err(error.into())
        }
    }
}

#[cfg(test)]
fn attest_prepared_projection(
    projector: &FinalizedProjector,
    archive: &PreparedArchive,
) -> Result<VerifiedReadSnapshot, AttestationError> {
    let candidate = archive.checkpoint()?;
    attest_prepared_projection_at_candidate(projector, archive, candidate, || Ok(()))
}

fn attest_prepared_projection_at_candidate<F>(
    projector: &FinalizedProjector,
    archive: &PreparedArchive,
    candidate: ProjectionCheckpoint,
    before_pin: F,
) -> Result<VerifiedReadSnapshot, AttestationError>
where
    F: FnOnce() -> Result<(), AttestationError>,
{
    if archive.checkpoint()? != candidate {
        return Err(AttestationError::ProjectionMismatch);
    }
    let expected = archive.complete_expected_projection()?;
    before_pin()?;
    let (directory, basename) = projector.parts()?;
    let mut reader = open_projection_reader(directory, basename)?;
    let data_version = reader.data_version()?;
    reader.begin_verified_read()?;

    let comparison = reader.with_verified_query(
        VerifiedQueryStatement::FullProjectionCompare,
        |connection| load_stored_projection_if_checkpoint(connection, &candidate),
    );
    let actual = match comparison {
        Ok(Some(actual)) => actual,
        Ok(None) => {
            let _ = reader.rollback_verified_read();
            return Err(AttestationError::RefreshRequired);
        }
        Err(error) => {
            let _ = reader.rollback_verified_read();
            return Err(error.into());
        }
    };
    if actual != expected {
        let _ = reader.rollback_verified_read();
        return Err(AttestationError::ProjectionMismatch);
    }

    Ok(mint_attested_snapshot(reader, candidate, data_version))
}

#[cfg(test)]
#[path = "attestation/tests.rs"]
mod tests;
