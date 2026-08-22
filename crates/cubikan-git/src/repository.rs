use std::{
    ffi::OsString,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::SystemTime,
};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use cubikan_core::{ExternalReference, ReferenceNamespace, ReferenceText};

use crate::{
    GitCommitResolver, GitReferenceError,
    process::{ProcessOutput, run_git},
};

const MINIMUM_GIT_VERSION: (u64, u64, u64) = (2, 45, 0);
const MAX_PACKED_REFS_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PACKED_REF_LINE_BYTES: usize = 4_096;
const MAX_OBJECT_STORE_ENTRIES: usize = 100_000;

pub(super) fn new_resolver(git_executable: &Path) -> Result<GitCommitResolver, GitReferenceError> {
    reject_dangerous_environment()?;
    let canonical =
        fs::canonicalize(git_executable).map_err(|_| GitReferenceError::InvalidGitExecutable)?;
    if !canonical.is_absolute() {
        return Err(GitReferenceError::InvalidGitExecutable);
    }
    let metadata =
        fs::symlink_metadata(&canonical).map_err(|_| GitReferenceError::InvalidGitExecutable)?;
    if !metadata.is_file() {
        return Err(GitReferenceError::InvalidGitExecutable);
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(GitReferenceError::InvalidGitExecutable);
    }
    let identity = FileIdentity::capture(&canonical)?;

    let version = run_git(&canonical, &[OsString::from("--version")])?;
    identity.recheck()?;
    if !version.success || !version_at_least(&version.stdout, MINIMUM_GIT_VERSION) {
        return Err(GitReferenceError::UnsupportedGitVersion);
    }
    let capability = run_git(
        &canonical,
        &[
            OsString::from("--no-lazy-fetch"),
            OsString::from("--version"),
        ],
    )?;
    identity.recheck()?;
    if !capability.success || !version_at_least(&capability.stdout, MINIMUM_GIT_VERSION) {
        return Err(GitReferenceError::NoLazyFetchUnsupported);
    }

    Ok(GitCommitResolver {
        git_executable: canonical,
        git_identity: identity,
    })
}

pub(super) fn resolve_commit(
    resolver: &GitCommitResolver,
    repository: &Path,
    repository_scope: &str,
    revision: &str,
) -> Result<ExternalReference, GitReferenceError> {
    reject_dangerous_environment()?;
    let scope =
        ReferenceText::new(repository_scope).map_err(|_| GitReferenceError::InvalidScope)?;
    validate_revision_shape(revision)?;

    let canonical_repository = canonical_repository(repository)?;
    let layout = RepositoryLayout::inspect(&canonical_repository)?;
    validate_repository_security(resolver, &canonical_repository, &layout)?;

    let top_level = run_in_repository(
        resolver,
        &canonical_repository,
        &["rev-parse", "--path-format=absolute", "--show-toplevel"],
    )?;
    if !top_level.success || top_level.stdout != exact_line(path_text(&canonical_repository)?) {
        return Err(GitReferenceError::NonCanonicalRepository);
    }

    let inspection = run_in_repository(
        resolver,
        &canonical_repository,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--absolute-git-dir",
            "--git-common-dir",
            "--git-path",
            "objects",
            "--show-object-format",
        ],
    )?;
    if !inspection.success {
        return Err(GitReferenceError::UnsupportedObjectFormat);
    }
    let lines =
        exact_utf8_lines(&inspection.stdout, 4).ok_or(GitReferenceError::UnsafeObjectStore)?;
    if lines[0] != path_text(&layout.git_dir)?
        || lines[1] != path_text(&layout.common_dir)?
        || lines[2] != path_text(&layout.object_dir)?
    {
        return Err(GitReferenceError::UnsafeObjectStore);
    }
    let object_format = ObjectFormat::parse(lines[3])?;
    if revision.len() != object_format.oid_length() {
        return Err(GitReferenceError::InvalidRevision);
    }

    validate_repository_security(resolver, &canonical_repository, &layout)?;
    let peeled = format!("{revision}^{{commit}}");
    let resolved = run_in_repository_os(
        resolver,
        &canonical_repository,
        &[
            OsString::from("rev-parse"),
            OsString::from("--verify"),
            OsString::from("--end-of-options"),
            OsString::from(peeled),
        ],
    )?;
    validate_repository_security(resolver, &canonical_repository, &layout)?;
    if !resolved.success {
        return Err(GitReferenceError::UnresolvableCommit);
    }
    if resolved.stdout != exact_line(revision) {
        return Err(GitReferenceError::NonCanonicalObjectId);
    }
    let namespace = ReferenceNamespace::new(object_format.namespace())
        .map_err(|_| GitReferenceError::CommandFailed)?;
    let value = ReferenceText::new(revision).map_err(|_| GitReferenceError::InvalidRevision)?;
    Ok(ExternalReference::new(namespace, scope, value))
}

#[derive(Clone, Copy)]
enum ObjectFormat {
    Sha1,
    Sha256,
}

impl ObjectFormat {
    fn parse(value: &str) -> Result<Self, GitReferenceError> {
        match value {
            "sha1" => Ok(Self::Sha1),
            "sha256" => Ok(Self::Sha256),
            _ => Err(GitReferenceError::UnsupportedObjectFormat),
        }
    }

    const fn oid_length(self) -> usize {
        match self {
            Self::Sha1 => 40,
            Self::Sha256 => 64,
        }
    }

    const fn namespace(self) -> &'static str {
        match self {
            Self::Sha1 => "git.commit.sha1",
            Self::Sha256 => "git.commit.sha256",
        }
    }
}

struct RepositoryLayout {
    root: DirectoryIdentity,
    git_dir: PathBuf,
    git_identity: DirectoryIdentity,
    common_dir: PathBuf,
    common_identity: DirectoryIdentity,
    object_dir: PathBuf,
    object_identity: DirectoryIdentity,
    object_snapshot: ObjectStoreSnapshot,
}

impl RepositoryLayout {
    fn inspect(repository: &Path) -> Result<Self, GitReferenceError> {
        let root = DirectoryIdentity::capture(repository, GitReferenceError::InvalidRepository)?;
        let git_dir = repository.join(".git");
        let git_identity =
            DirectoryIdentity::capture(&git_dir, GitReferenceError::NonCanonicalRepository)?;
        let common_dir = git_dir.clone();
        let common_identity = git_identity.clone();
        let object_dir = common_dir.join("objects");
        let object_identity =
            DirectoryIdentity::capture(&object_dir, GitReferenceError::UnsafeObjectStore)?;
        let object_snapshot = ObjectStoreSnapshot::capture(&object_dir)?;
        Ok(Self {
            root,
            git_dir,
            git_identity,
            common_dir,
            common_identity,
            object_dir,
            object_identity,
            object_snapshot,
        })
    }

    fn reject_alternates(&self) -> Result<(), GitReferenceError> {
        let mut checked = Vec::new();
        for object_dir in [
            self.git_dir.join("objects"),
            self.common_dir.join("objects"),
        ] {
            if checked.contains(&object_dir) {
                continue;
            }
            checked.push(object_dir.clone());
            let alternates = object_dir.join("info").join("alternates");
            match fs::symlink_metadata(&alternates) {
                Ok(metadata) if metadata.is_file() && metadata.len() == 0 => {}
                Ok(_) => return Err(GitReferenceError::AlternateObjectStore),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(GitReferenceError::UnsafeObjectStore),
            }
        }
        Ok(())
    }

    fn reject_replace_objects(&self) -> Result<(), GitReferenceError> {
        let loose = self.common_dir.join("refs").join("replace");
        match fs::symlink_metadata(&loose) {
            Ok(metadata) if metadata.is_dir() => {
                if fs::read_dir(&loose)
                    .map_err(|_| GitReferenceError::UnsafeObjectStore)?
                    .next()
                    .transpose()
                    .map_err(|_| GitReferenceError::UnsafeObjectStore)?
                    .is_some()
                {
                    return Err(GitReferenceError::ReplaceObjectsPresent);
                }
            }
            Ok(_) => return Err(GitReferenceError::ReplaceObjectsPresent),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(GitReferenceError::UnsafeObjectStore),
        }

        let packed = self.common_dir.join("packed-refs");
        match fs::symlink_metadata(&packed) {
            Ok(metadata) if metadata.is_file() => {
                if metadata.len() > MAX_PACKED_REFS_BYTES {
                    return Err(GitReferenceError::UnsafeObjectStore);
                }
                let mut reader = BufReader::new(
                    File::open(&packed).map_err(|_| GitReferenceError::UnsafeObjectStore)?,
                );
                let mut line = Vec::new();
                loop {
                    line.clear();
                    let count = reader
                        .read_until(b'\n', &mut line)
                        .map_err(|_| GitReferenceError::UnsafeObjectStore)?;
                    if count == 0 {
                        break;
                    }
                    if line.len() > MAX_PACKED_REF_LINE_BYTES {
                        return Err(GitReferenceError::UnsafeObjectStore);
                    }
                    if line
                        .windows(b" refs/replace/".len())
                        .any(|window| window == b" refs/replace/")
                    {
                        return Err(GitReferenceError::ReplaceObjectsPresent);
                    }
                }
            }
            Ok(_) => return Err(GitReferenceError::ReplaceObjectsPresent),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(GitReferenceError::UnsafeObjectStore),
        }
        Ok(())
    }

    fn reject_promisor_objects(&self) -> Result<(), GitReferenceError> {
        let pack = self.object_dir.join("pack");
        let entries = fs::read_dir(&pack).map_err(|_| GitReferenceError::UnsafeObjectStore)?;
        for entry in entries {
            let entry = entry.map_err(|_| GitReferenceError::UnsafeObjectStore)?;
            let file_type = entry
                .file_type()
                .map_err(|_| GitReferenceError::UnsafeObjectStore)?;
            if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "promisor")
            {
                if !file_type.is_file() {
                    return Err(GitReferenceError::UnsafeObjectStore);
                }
                return Err(GitReferenceError::PromisorObjectsPresent);
            }
        }
        Ok(())
    }

    fn recheck(&self) -> Result<(), GitReferenceError> {
        self.root
            .recheck(GitReferenceError::NonCanonicalRepository)?;
        self.git_identity
            .recheck(GitReferenceError::UnsafeObjectStore)?;
        self.common_identity
            .recheck(GitReferenceError::UnsafeObjectStore)?;
        self.object_identity
            .recheck(GitReferenceError::UnsafeObjectStore)?;
        if ObjectStoreSnapshot::capture(&self.object_dir)? != self.object_snapshot {
            return Err(GitReferenceError::UnsafeObjectStore);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObjectStoreSnapshot(Vec<ObjectStoreEntry>);

impl ObjectStoreSnapshot {
    fn capture(root: &Path) -> Result<Self, GitReferenceError> {
        let mut entries = Vec::new();
        capture_object_store(root, root, &mut entries)?;
        Ok(Self(entries))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObjectStoreEntry {
    relative_path: PathBuf,
    kind: ObjectStoreEntryKind,
    length: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObjectStoreEntryKind {
    Directory,
    File,
}

fn capture_object_store(
    root: &Path,
    directory: &Path,
    entries: &mut Vec<ObjectStoreEntry>,
) -> Result<(), GitReferenceError> {
    let mut children = fs::read_dir(directory)
        .map_err(|_| GitReferenceError::UnsafeObjectStore)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| GitReferenceError::UnsafeObjectStore)?;
    children.sort_by_key(fs::DirEntry::file_name);
    for child in children {
        if entries.len() >= MAX_OBJECT_STORE_ENTRIES {
            return Err(GitReferenceError::UnsafeObjectStore);
        }
        let path = child.path();
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| GitReferenceError::UnsafeObjectStore)?;
        if metadata.file_type().is_symlink() {
            return Err(GitReferenceError::UnsafeObjectStore);
        }
        let kind = if metadata.is_dir() {
            ObjectStoreEntryKind::Directory
        } else if metadata.is_file() {
            #[cfg(unix)]
            if metadata.nlink() != 1 {
                return Err(GitReferenceError::UnsafeObjectStore);
            }
            ObjectStoreEntryKind::File
        } else {
            return Err(GitReferenceError::UnsafeObjectStore);
        };
        let canonical =
            fs::canonicalize(&path).map_err(|_| GitReferenceError::UnsafeObjectStore)?;
        if canonical != path || !canonical.starts_with(root) {
            return Err(GitReferenceError::UnsafeObjectStore);
        }
        entries.push(ObjectStoreEntry {
            relative_path: path
                .strip_prefix(root)
                .map_err(|_| GitReferenceError::UnsafeObjectStore)?
                .to_path_buf(),
            kind,
            length: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        });
        if metadata.is_dir() {
            capture_object_store(root, &path, entries)?;
        }
    }
    Ok(())
}

#[derive(Clone, Eq, PartialEq)]
struct DirectoryIdentity {
    path: PathBuf,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl DirectoryIdentity {
    fn capture(path: &Path, error: GitReferenceError) -> Result<Self, GitReferenceError> {
        let metadata = fs::symlink_metadata(path).map_err(|_| error)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(error);
        }
        let canonical = fs::canonicalize(path).map_err(|_| error)?;
        if canonical != path {
            return Err(error);
        }
        Ok(Self {
            path: canonical,
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        })
    }

    fn recheck(&self, error: GitReferenceError) -> Result<(), GitReferenceError> {
        let current = Self::capture(&self.path, error)?;
        if current == *self { Ok(()) } else { Err(error) }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub(super) struct FileIdentity {
    path: PathBuf,
    length: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl FileIdentity {
    fn capture(path: &Path) -> Result<Self, GitReferenceError> {
        let metadata =
            fs::symlink_metadata(path).map_err(|_| GitReferenceError::InvalidGitExecutable)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(GitReferenceError::InvalidGitExecutable);
        }
        let canonical =
            fs::canonicalize(path).map_err(|_| GitReferenceError::InvalidGitExecutable)?;
        if canonical != path {
            return Err(GitReferenceError::InvalidGitExecutable);
        }
        Ok(Self {
            path: canonical,
            length: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        })
    }

    fn recheck(&self) -> Result<(), GitReferenceError> {
        if Self::capture(&self.path)? == *self {
            Ok(())
        } else {
            Err(GitReferenceError::InvalidGitExecutable)
        }
    }
}

fn canonical_repository(repository: &Path) -> Result<PathBuf, GitReferenceError> {
    if !repository.is_absolute() {
        return Err(GitReferenceError::NonCanonicalRepository);
    }
    let canonical =
        fs::canonicalize(repository).map_err(|_| GitReferenceError::InvalidRepository)?;
    if canonical != repository {
        return Err(GitReferenceError::NonCanonicalRepository);
    }
    let metadata =
        fs::symlink_metadata(&canonical).map_err(|_| GitReferenceError::InvalidRepository)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(GitReferenceError::InvalidRepository);
    }
    Ok(canonical)
}

fn validate_repository_security(
    resolver: &GitCommitResolver,
    repository: &Path,
    layout: &RepositoryLayout,
) -> Result<(), GitReferenceError> {
    layout.recheck()?;
    validate_repository_config(resolver, repository)?;
    layout.reject_alternates()?;
    layout.reject_replace_objects()?;
    layout.reject_promisor_objects()?;
    layout.recheck()
}

fn validate_repository_config(
    resolver: &GitCommitResolver,
    repository: &Path,
) -> Result<(), GitReferenceError> {
    let output = run_in_repository(
        resolver,
        repository,
        &[
            "config",
            "--local",
            "--no-includes",
            "--null",
            "--name-only",
            "--list",
        ],
    )?;
    if !output.success {
        return Err(GitReferenceError::DangerousRepositoryConfig);
    }
    if !output.stdout.is_empty() && !output.stdout.ends_with(&[0]) {
        return Err(GitReferenceError::DangerousRepositoryConfig);
    }
    let keys = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|key| !key.is_empty())
        .map(|key| {
            std::str::from_utf8(key)
                .map(str::to_owned)
                .map_err(|_| GitReferenceError::DangerousRepositoryConfig)
        })
        .collect::<Result<Vec<_>, _>>()?;
    for key in keys {
        let lowercase = key.to_ascii_lowercase();
        if dangerous_config_key(&lowercase) {
            return Err(
                if key.eq_ignore_ascii_case("extensions.partialclone")
                    || lowercase.ends_with(".promisor")
                    || lowercase.ends_with(".partialclonefilter")
                {
                    GitReferenceError::PromisorObjectsPresent
                } else {
                    GitReferenceError::DangerousRepositoryConfig
                },
            );
        }
        if lowercase.starts_with("remote.") && lowercase.ends_with(".url") {
            validate_remote_urls(resolver, repository, &key)?;
        }
    }
    Ok(())
}

fn validate_remote_urls(
    resolver: &GitCommitResolver,
    repository: &Path,
    key: &str,
) -> Result<(), GitReferenceError> {
    let output = run_in_repository_os(
        resolver,
        repository,
        &[
            OsString::from("config"),
            OsString::from("--local"),
            OsString::from("--no-includes"),
            OsString::from("--null"),
            OsString::from("--get-all"),
            OsString::from(key),
        ],
    )?;
    if !output.success || output.stdout.is_empty() || !output.stdout.ends_with(&[0]) {
        return Err(GitReferenceError::DangerousRepositoryConfig);
    }
    for value in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|value| !value.is_empty())
    {
        let value =
            std::str::from_utf8(value).map_err(|_| GitReferenceError::DangerousRepositoryConfig)?;
        if helper_shaped_remote_url(value) {
            return Err(GitReferenceError::DangerousRepositoryConfig);
        }
    }
    Ok(())
}

fn helper_shaped_remote_url(value: &str) -> bool {
    if value.split_once("::").is_some() {
        return true;
    }
    let Some((scheme, _)) = value.split_once("://") else {
        return false;
    };
    !matches!(
        scheme.to_ascii_lowercase().as_str(),
        "file" | "git" | "http" | "https" | "ssh"
    )
}

fn dangerous_config_key(key: &str) -> bool {
    key == "extensions.partialclone"
        || key == "core.worktree"
        || key == "core.askpass"
        || key == "core.sshcommand"
        || key == "core.gitproxy"
        || key == "core.alternaterefscommand"
        || key == "core.hookspath"
        || key == "core.fsmonitor"
        || key == "http.proxy"
        || key == "https.proxy"
        || key.starts_with("include.")
        || key.starts_with("includeif.")
        || key.starts_with("credential.")
        || key.starts_with("url.")
        || key.starts_with("filter.")
        || (key.starts_with("remote.") && key.ends_with(".vcs"))
        || key.ends_with(".promisor")
        || key.ends_with(".partialclonefilter")
        || key.ends_with(".proxy")
}

fn reject_dangerous_environment() -> Result<(), GitReferenceError> {
    for (name, value) in std::env::vars_os() {
        if value.is_empty() {
            continue;
        }
        let Some(name) = name.to_str() else {
            return Err(GitReferenceError::DangerousEnvironment);
        };
        let upper = name.to_ascii_uppercase();
        let dangerous_git = upper.starts_with("GIT_") && upper != "GIT_PAGER";
        let dangerous_proxy = matches!(upper.as_str(), "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY");
        let dangerous_credential = matches!(
            upper.as_str(),
            "SSH_AUTH_SOCK" | "SSH_ASKPASS" | "GCM_INTERACTIVE" | "GCM_CREDENTIAL_STORE"
        );
        if dangerous_git || dangerous_proxy || dangerous_credential {
            return Err(GitReferenceError::DangerousEnvironment);
        }
    }
    Ok(())
}

fn validate_revision_shape(revision: &str) -> Result<(), GitReferenceError> {
    if !matches!(revision.len(), 40 | 64)
        || !revision.as_bytes().iter().all(u8::is_ascii_hexdigit)
        || revision.as_bytes().iter().any(u8::is_ascii_uppercase)
    {
        return Err(GitReferenceError::InvalidRevision);
    }
    Ok(())
}

fn version_at_least(stdout: &[u8], minimum: (u64, u64, u64)) -> bool {
    let Some(version) = stdout
        .strip_prefix(b"git version ")
        .and_then(|value| value.strip_suffix(b"\n"))
        .and_then(|value| std::str::from_utf8(value).ok())
    else {
        return false;
    };
    let mut parts = version.split('.');
    let Some(major) = parts.next().and_then(|part| part.parse::<u64>().ok()) else {
        return false;
    };
    let Some(minor) = parts.next().and_then(|part| part.parse::<u64>().ok()) else {
        return false;
    };
    let Some(patch) = parts.next().and_then(|part| part.parse::<u64>().ok()) else {
        return false;
    };
    (major, minor, patch) >= minimum
}

fn run_in_repository(
    resolver: &GitCommitResolver,
    repository: &Path,
    arguments: &[&str],
) -> Result<ProcessOutput, GitReferenceError> {
    let arguments = arguments.iter().map(OsString::from).collect::<Vec<_>>();
    run_in_repository_os(resolver, repository, &arguments)
}

fn run_in_repository_os(
    resolver: &GitCommitResolver,
    repository: &Path,
    arguments: &[OsString],
) -> Result<ProcessOutput, GitReferenceError> {
    let mut complete = vec![
        OsString::from("--no-lazy-fetch"),
        OsString::from("-C"),
        repository.as_os_str().to_owned(),
    ];
    complete.extend_from_slice(arguments);
    resolver.git_identity.recheck()?;
    let output = run_git(&resolver.git_executable, &complete)?;
    resolver.git_identity.recheck()?;
    Ok(output)
}

fn exact_utf8_lines(stdout: &[u8], expected: usize) -> Option<Vec<&str>> {
    let body = stdout.strip_suffix(b"\n")?;
    let body = std::str::from_utf8(body).ok()?;
    let lines = body.split('\n').collect::<Vec<_>>();
    (lines.len() == expected && lines.iter().all(|line| !line.is_empty())).then_some(lines)
}

fn exact_line(value: &str) -> Vec<u8> {
    let mut line = Vec::with_capacity(value.len() + 1);
    line.extend_from_slice(value.as_bytes());
    line.push(b'\n');
    line
}

fn path_text(path: &Path) -> Result<&str, GitReferenceError> {
    path.to_str().ok_or(GitReferenceError::InvalidRepository)
}
