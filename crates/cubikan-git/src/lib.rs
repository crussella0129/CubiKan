#![forbid(unsafe_code)]

//! Provider-neutral, offline resolution of exact Git commit identities.

use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

use cubikan_core::ExternalReference;

mod process;
mod repository;

/// A validated resolver bound to one canonical absolute Git executable.
pub struct GitCommitResolver {
    git_executable: PathBuf,
    git_identity: repository::FileIdentity,
}

impl GitCommitResolver {
    /// Validates and binds one Git executable for every later probe.
    pub fn new(git_executable: impl AsRef<Path>) -> Result<Self, GitReferenceError> {
        repository::new_resolver(git_executable.as_ref())
    }

    /// Resolves one exact, full, lowercase commit object ID without fetching.
    pub fn resolve_commit(
        &self,
        repository: impl AsRef<Path>,
        repository_scope: &str,
        revision: &str,
    ) -> Result<ExternalReference, GitReferenceError> {
        repository::resolve_commit(self, repository.as_ref(), repository_scope, revision)
    }
}

impl fmt::Debug for GitCommitResolver {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GitCommitResolver")
            .field("git_executable", &"<validated>")
            .finish()
    }
}

/// Closed, path-free failure taxonomy for Git reference resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitReferenceError {
    InvalidGitExecutable,
    DangerousEnvironment,
    UnsupportedGitVersion,
    NoLazyFetchUnsupported,
    InvalidRepository,
    NonCanonicalRepository,
    UnsafeObjectStore,
    AlternateObjectStore,
    ReplaceObjectsPresent,
    PromisorObjectsPresent,
    DangerousRepositoryConfig,
    InvalidScope,
    InvalidRevision,
    UnsupportedObjectFormat,
    UnresolvableCommit,
    NonCanonicalObjectId,
    OutputLimitExceeded,
    CommandTimedOut,
    CommandFailed,
}

impl GitReferenceError {
    /// Stable machine-readable code that never embeds caller or Git content.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidGitExecutable => "invalid_git_executable",
            Self::DangerousEnvironment => "dangerous_environment",
            Self::UnsupportedGitVersion => "unsupported_git_version",
            Self::NoLazyFetchUnsupported => "no_lazy_fetch_unsupported",
            Self::InvalidRepository => "invalid_repository",
            Self::NonCanonicalRepository => "noncanonical_repository",
            Self::UnsafeObjectStore => "unsafe_object_store",
            Self::AlternateObjectStore => "alternate_object_store",
            Self::ReplaceObjectsPresent => "replace_objects_present",
            Self::PromisorObjectsPresent => "promisor_objects_present",
            Self::DangerousRepositoryConfig => "dangerous_repository_config",
            Self::InvalidScope => "invalid_scope",
            Self::InvalidRevision => "invalid_revision",
            Self::UnsupportedObjectFormat => "unsupported_object_format",
            Self::UnresolvableCommit => "unresolvable_commit",
            Self::NonCanonicalObjectId => "noncanonical_object_id",
            Self::OutputLimitExceeded => "output_limit_exceeded",
            Self::CommandTimedOut => "command_timed_out",
            Self::CommandFailed => "command_failed",
        }
    }
}

impl fmt::Display for GitReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl Error for GitReferenceError {}
