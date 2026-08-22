use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::{
    net::TcpListener,
    os::unix::fs::{PermissionsExt, symlink},
};

use cubikan_core::{AssociationSubject, ExternalReference, IntentUnitId, RecordedAssociation};
use cubikan_git::{GitCommitResolver, GitReferenceError};
use serde_json::Value;
use sha2::{Digest, Sha256};

const FIXTURE: &str = include_str!("../../../tests/fixtures/git/manifest-v1.json");
const ASSOCIATION_FIXTURE: &[u8] =
    include_bytes!("../../../tests/fixtures/git/recorded-association-v1.json");
const SOURCE_V1: &[u8] = include_bytes!("../../../tests/fixtures/git/source-v1.txt");
const SOURCE_V2: &[u8] = include_bytes!("../../../tests/fixtures/git/source-v2.txt");
const CRATE_MANIFEST: &str = include_str!("../Cargo.toml");
const HOSTILE_CHILD: &str = "CUBIKAN_GIT_HOSTILE_CHILD";
const HOSTILE_TEST_NAME: &str = "test_git_rejects_noncanonical_inputs_without_submission";
const GIT_MINIMUM: (u64, u64, u64) = (2, 45, 0);
const EXPECTED_ASSOCIATION: &[u8] = br#"{"unit_id":"01890f47-0ed1-7c68-a7c5-02f69b5a5fe3","subject":{"kind":"revision","revision":7},"reference":{"namespace":"git.commit.sha1","scope":"workspace/cubikan","value":"15f37f16e70096c2624ca92fed9d55750640ec88"}}"#;

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_git_resolves_full_algorithm_specific_commit_without_network() {
    let fixture = validate_independent_fixture();
    let git = installed_git();
    assert_required_git(&git);
    assert_global_no_lazy_fetch(&git);

    let repository = RepositoryFixture::new(&git, ObjectFormat::Sha1);
    assert_eq!(
        repository.commit_oid,
        fixture.commit_oid(ObjectFormat::Sha1)
    );
    let before = ByteTree::capture(repository.path());

    let resolver = resolver(&git);
    let reference = resolver
        .resolve_commit(repository.path(), fixture.scope(), &repository.commit_oid)
        .expect("resolve exact full SHA-1 commit without provider authority");
    assert_reference(
        &reference,
        "git.commit.sha1",
        fixture.scope(),
        &repository.commit_oid,
    );
    assert_eq!(ByteTree::capture(repository.path()), before);

    #[cfg(unix)]
    assert_exact_process_contract(&fixture, repository.path(), &repository.commit_oid);
}

#[test]
fn test_git_sha256_capability_gate_and_fixture_are_exact() {
    let fixture = validate_independent_fixture();
    validate_sha256_object_graph(&fixture);
    let git = installed_git();
    assert_required_git(&git);

    let sha1 = RepositoryFixture::new(&git, ObjectFormat::Sha1);
    let sha1_reference = resolver(&git)
        .resolve_commit(sha1.path(), fixture.scope(), &sha1.commit_oid)
        .expect("real SHA-1 fixture is mandatory");
    assert_reference(
        &sha1_reference,
        "git.commit.sha1",
        fixture.scope(),
        fixture.commit_oid(ObjectFormat::Sha1),
    );

    match git_supports_sha256_repositories(&git) {
        Ok(true) => {
            let sha256 = RepositoryFixture::new(&git, ObjectFormat::Sha256);
            assert_eq!(
                sha256.commit_oid,
                fixture.commit_oid(ObjectFormat::Sha256),
                "live SHA-256 repository must match independently pinned commit bytes"
            );
            assert_eq!(sha256.tree_oid, fixture.tree_oid(ObjectFormat::Sha256));
            assert_eq!(sha256.blob_oid, fixture.blob_oid(ObjectFormat::Sha256));
            let before = ByteTree::capture(sha256.path());
            let reference = resolver(&git)
                .resolve_commit(sha256.path(), fixture.scope(), &sha256.commit_oid)
                .expect("resolve real SHA-256 commit when installed Git supports it");
            assert_reference(
                &reference,
                "git.commit.sha256",
                fixture.scope(),
                &sha256.commit_oid,
            );
            assert_eq!(ByteTree::capture(sha256.path()), before);
        }
        Ok(false) => eprintln!(
            "T-1114-E2 SKIP live SHA-256: installed Git explicitly rejected the sha256 object format; independently hashed SHA-256 blob/tree/commit fixture remains enforced"
        ),
        Err(failure) => panic!("SHA-256 capability probe failed unexpectedly: {failure}"),
    }
}

#[test]
fn test_git_reference_survives_move_and_blame_change() {
    let fixture = validate_independent_fixture();
    let git = installed_git();
    assert_required_git(&git);
    let mut repository = RepositoryFixture::new(&git, ObjectFormat::Sha1);
    let resolver = resolver(&git);

    let original_reference = resolver
        .resolve_commit(repository.path(), fixture.scope(), &repository.commit_oid)
        .expect("record original immutable commit identity");
    let association = RecordedAssociation::new(
        "01890f47-0ed1-7c68-a7c5-02f69b5a5fe3"
            .parse::<IntentUnitId>()
            .expect("fixture unit UUID"),
        AssociationSubject::Revision(7),
        original_reference,
    );
    let original_bytes = serde_json::to_vec(&association).expect("serialize recorded association");
    assert_eq!(original_bytes, EXPECTED_ASSOCIATION);
    assert_eq!(without_one_lf(ASSOCIATION_FIXTURE), EXPECTED_ASSOCIATION);

    repository.rename_edit_and_recommit(&git);
    let blame = git_stdout(
        &git,
        Some(repository.path()),
        ["blame", "--line-porcelain", "--", "moved.txt"],
    );
    assert!(blame.contains("author Blame Changer\n"));
    assert_ne!(
        repository.commit_oid,
        fixture.commit_oid(ObjectFormat::Sha1)
    );
    let latest = resolver
        .resolve_commit(repository.path(), fixture.scope(), &repository.commit_oid)
        .expect("resolve changed repository head");
    assert_ne!(latest, *association.reference());

    let original_again = resolver
        .resolve_commit(
            repository.path(),
            fixture.scope(),
            fixture.commit_oid(ObjectFormat::Sha1),
        )
        .expect("old full commit remains addressable after rename and blame change");
    let rebuilt =
        RecordedAssociation::new(association.unit_id(), association.subject(), original_again);
    let rebuilt_bytes = serde_json::to_vec(&rebuilt).expect("serialize rebuilt association");
    assert_eq!(rebuilt_bytes, original_bytes);
    assert_eq!(
        serde_json::to_vec(&association).expect("re-serialize original association"),
        original_bytes
    );
    let rendered = String::from_utf8(original_bytes).expect("association JSON is UTF-8");
    for forbidden in [
        "Fixture Author",
        "Fixture Committer",
        "Blame Changer",
        "Different Committer",
        "author",
        "committer",
        "signer",
        "causality",
        "verification",
        "satisfaction",
    ] {
        assert!(
            !rendered.contains(forbidden),
            "recorded association promoted forbidden attribution field `{forbidden}`"
        );
    }
}

#[test]
fn test_git_rejects_noncanonical_inputs_without_submission() {
    if let Ok(specification) = std::env::var(HOSTILE_CHILD) {
        run_hostile_environment_child(&specification);
        return;
    }

    let fixture = validate_independent_fixture();
    let git = installed_git();
    assert_required_git(&git);
    let repository = RepositoryFixture::new(&git, ObjectFormat::Sha1);
    let resolver = resolver(&git);
    let tag_oid = repository.annotated_tag(&git);
    let before = ByteTree::capture(repository.path());
    let rejection_inputs = fixture.value["rejection_inputs"]
        .as_object()
        .expect("fixture rejection_inputs object");

    for name in [
        "sha1_39",
        "sha1_40_nonhex",
        "sha1_41",
        "sha1_uppercase",
        "sha256_63",
        "sha256_64_nonhex",
        "sha256_65",
        "sha256_uppercase",
        "leading_dash",
        "embedded_nul",
    ] {
        let candidate = rejection_inputs[name]
            .as_str()
            .unwrap_or_else(|| panic!("fixture rejection `{name}` must be text"));
        let error = resolver
            .resolve_commit(repository.path(), fixture.scope(), candidate)
            .expect_err("noncanonical OID must be rejected");
        assert_noncanonical_oid(error, name);
    }

    for invalid_scope in ["", "   ", "scope\0suffix"] {
        let error = resolver
            .resolve_commit(repository.path(), invalid_scope, &repository.commit_oid)
            .expect_err("invalid scope must fail before Git execution");
        assert!(
            matches!(error, GitReferenceError::InvalidScope),
            "invalid scope must report InvalidScope, got {error:?}"
        );
        assert_path_free(&error, &[repository.path()]);
    }

    for (label, oid) in [
        ("blob", repository.blob_oid.as_str()),
        ("tree", repository.tree_oid.as_str()),
    ] {
        let error = resolver
            .resolve_commit(repository.path(), fixture.scope(), oid)
            .expect_err("noncommit object must be rejected");
        assert_noncommit(error, label, repository.path(), oid);
    }
    let error = resolver
        .resolve_commit(repository.path(), fixture.scope(), &tag_oid)
        .expect_err("annotated tag OID must not be normalized by peeling");
    assert_noncommit(error, "tag", repository.path(), &tag_oid);

    let missing = "ffffffffffffffffffffffffffffffffffffffff";
    let error = resolver
        .resolve_commit(repository.path(), fixture.scope(), missing)
        .expect_err("missing full OID must be rejected");
    assert_noncommit(error, "missing", repository.path(), missing);

    let wrong_algorithm = fixture.commit_oid(ObjectFormat::Sha256);
    let error = resolver
        .resolve_commit(repository.path(), fixture.scope(), wrong_algorithm)
        .expect_err("a canonical SHA-256 OID must not resolve in a SHA-1 repository");
    assert!(
        matches!(error, GitReferenceError::InvalidRevision),
        "wrong-algorithm OID must report InvalidRevision, got {error:?}"
    );
    assert_path_free(&error, &[repository.path(), Path::new(wrong_algorithm)]);

    let nested = repository.path().join("nested");
    fs::create_dir(&nested).expect("create repository subdirectory");
    let nested_before = ByteTree::capture(repository.path());
    let error = resolver
        .resolve_commit(&nested, fixture.scope(), &repository.commit_oid)
        .expect_err("repository subdirectory must not be accepted as canonical -C root");
    assert_noncanonical_repository(error, "subdirectory", &nested);
    assert_eq!(ByteTree::capture(repository.path()), nested_before);

    let outside = TestDirectory::new("git-outside");
    let error = resolver
        .resolve_commit(outside.path(), fixture.scope(), &repository.commit_oid)
        .expect_err("directory outside a Git worktree must be rejected");
    assert_noncanonical_repository(error, "outside", outside.path());

    assert_eq!(
        ByteTree::capture(repository.path()),
        before.with_empty_directory("nested")
    );
    run_hostile_environment_cases(&git, &repository, fixture.scope());

    #[cfg(unix)]
    {
        assert_on_disk_alternates_rejected(&git, &fixture);
        assert_replace_state_rejected(&git, &fixture);
        assert_promisor_and_helper_state_rejected(&git, &fixture);
        assert_incompatible_git_and_process_faults(&fixture, &repository);
    }

    for forbidden_dependency in [
        "cubikan-backend",
        "cubikan-chain-client",
        "cubikan-local",
        "rusqlite",
        "subxt",
    ] {
        assert!(
            !CRATE_MANIFEST.contains(forbidden_dependency),
            "Git adapter gained submission/projection dependency `{forbidden_dependency}`"
        );
    }
}

struct Fixture {
    value: Value,
}

impl Fixture {
    fn scope(&self) -> &str {
        self.value["scope"].as_str().expect("fixture scope")
    }

    fn object(&self, format: ObjectFormat) -> &Value {
        &self.value[format.name()]
    }

    fn commit_oid(&self, format: ObjectFormat) -> &str {
        self.object(format)["commit_oid"]
            .as_str()
            .expect("fixture commit OID")
    }

    fn tree_oid(&self, format: ObjectFormat) -> &str {
        self.object(format)["tree_oid"]
            .as_str()
            .expect("fixture tree OID")
    }

    fn blob_oid(&self, format: ObjectFormat) -> &str {
        self.object(format)["blob_oid"]
            .as_str()
            .expect("fixture blob OID")
    }
}

#[derive(Clone, Copy, Debug)]
enum ObjectFormat {
    Sha1,
    Sha256,
}

impl ObjectFormat {
    const fn name(self) -> &'static str {
        match self {
            Self::Sha1 => "sha1",
            Self::Sha256 => "sha256",
        }
    }

    const fn oid_length(self) -> usize {
        match self {
            Self::Sha1 => 40,
            Self::Sha256 => 64,
        }
    }
}

struct RepositoryFixture {
    directory: TestDirectory,
    commit_oid: String,
    tree_oid: String,
    blob_oid: String,
}

impl RepositoryFixture {
    fn new(git: &Path, format: ObjectFormat) -> Self {
        Self::try_new(git, format)
            .unwrap_or_else(|error| panic!("create deterministic {format:?} repository: {error}"))
    }

    fn try_new(git: &Path, format: ObjectFormat) -> Result<Self, String> {
        let directory = TestDirectory::new(&format!("git-{}", format.name()));
        fs::write(directory.path().join("source.txt"), SOURCE_V1)
            .map_err(|error| format!("write seed: {error}"))?;
        let output = git_output(
            git,
            None,
            [
                OsString::from("init"),
                OsString::from("--quiet"),
                OsString::from(format!("--object-format={}", format.name())),
                OsString::from("--initial-branch=main"),
                directory.path().as_os_str().to_owned(),
            ],
            &[],
        );
        if !output.status.success() {
            return Err(format!(
                "git init failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        assert_git_success(
            git_output(
                git,
                Some(directory.path()),
                ["add", "--", "source.txt"],
                &[],
            ),
            "stage fixture source",
        );
        assert_git_success(
            git_output(
                git,
                Some(directory.path()),
                ["commit", "--quiet", "-m", "fixture root"],
                &fixture_identity_environment(),
            ),
            "commit fixture root",
        );
        let commit_oid = git_stdout(git, Some(directory.path()), ["rev-parse", "HEAD"]);
        let tree_oid = git_stdout(git, Some(directory.path()), ["rev-parse", "HEAD^{tree}"]);
        let blob_oid = git_stdout(
            git,
            Some(directory.path()),
            ["rev-parse", "HEAD:source.txt"],
        );
        for (label, oid) in [
            ("commit", commit_oid.as_str()),
            ("tree", tree_oid.as_str()),
            ("blob", blob_oid.as_str()),
        ] {
            assert_eq!(oid.len(), format.oid_length(), "{label} OID length");
            assert!(is_lower_hex(oid), "{label} OID must be lowercase hex");
        }
        Ok(Self {
            directory,
            commit_oid,
            tree_oid,
            blob_oid,
        })
    }

    fn path(&self) -> &Path {
        self.directory.path()
    }

    fn annotated_tag(&self, git: &Path) -> String {
        assert_git_success(
            git_output(
                git,
                Some(self.path()),
                ["tag", "-a", "fixture-tag", "-m", "fixture tag", "HEAD"],
                &fixture_identity_environment(),
            ),
            "create deterministic annotated tag",
        );
        git_stdout(
            git,
            Some(self.path()),
            ["rev-parse", "refs/tags/fixture-tag"],
        )
    }

    fn rename_edit_and_recommit(&mut self, git: &Path) {
        fs::rename(
            self.path().join("source.txt"),
            self.path().join("moved.txt"),
        )
        .expect("rename fixture source");
        fs::write(self.path().join("moved.txt"), SOURCE_V2).expect("edit moved fixture source");
        assert_git_success(
            git_output(git, Some(self.path()), ["add", "-A"], &[]),
            "stage rename and edit",
        );
        let changed_environment = [
            ("GIT_AUTHOR_NAME", "Blame Changer"),
            ("GIT_AUTHOR_EMAIL", "blame@example.invalid"),
            ("GIT_AUTHOR_DATE", "2002-03-04T05:06:07+0000"),
            ("GIT_COMMITTER_NAME", "Different Committer"),
            ("GIT_COMMITTER_EMAIL", "committer@example.invalid"),
            ("GIT_COMMITTER_DATE", "2002-03-04T05:06:07+0000"),
        ];
        assert_git_success(
            git_output(
                git,
                Some(self.path()),
                ["commit", "--quiet", "-m", "move and change blame"],
                &changed_environment,
            ),
            "commit renamed source with different attribution",
        );
        self.commit_oid = git_stdout(git, Some(self.path()), ["rev-parse", "HEAD"]);
        self.tree_oid = git_stdout(git, Some(self.path()), ["rev-parse", "HEAD^{tree}"]);
        self.blob_oid = git_stdout(git, Some(self.path()), ["rev-parse", "HEAD:moved.txt"]);
    }
}

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let serial = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "cubikan-git-{}-{label}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap_or_else(|error| {
            panic!("create isolated test directory {}: {error}", path.display())
        });
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ByteTree(BTreeMap<PathBuf, TreeEntry>);

#[derive(Clone, Debug, Eq, PartialEq)]
enum TreeEntry {
    Directory,
    File(Vec<u8>),
    Symlink(PathBuf),
}

impl ByteTree {
    fn capture(root: &Path) -> Self {
        let mut entries = BTreeMap::new();
        capture_tree(root, root, &mut entries).expect("capture recursive repository byte tree");
        Self(entries)
    }

    fn with_empty_directory(mut self, relative: &str) -> Self {
        self.0.insert(PathBuf::from(relative), TreeEntry::Directory);
        self
    }
}

fn capture_tree(
    root: &Path,
    directory: &Path,
    entries: &mut BTreeMap<PathBuf, TreeEntry>,
) -> io::Result<()> {
    let mut children = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    children.sort_by_key(fs::DirEntry::file_name);
    for child in children {
        let path = child.path();
        let relative = path
            .strip_prefix(root)
            .expect("captured path remains under root")
            .to_path_buf();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            entries.insert(relative, TreeEntry::Symlink(fs::read_link(path)?));
        } else if metadata.is_dir() {
            entries.insert(relative, TreeEntry::Directory);
            capture_tree(root, &path, entries)?;
        } else {
            entries.insert(relative, TreeEntry::File(fs::read(path)?));
        }
    }
    Ok(())
}

fn validate_independent_fixture() -> Fixture {
    assert_eq!(SOURCE_V1, b"alpha\n");
    assert_eq!(SOURCE_V2, b"alpha moved and edited\n");
    assert_eq!(without_one_lf(ASSOCIATION_FIXTURE), EXPECTED_ASSOCIATION);
    let value: Value = serde_json::from_str(FIXTURE).expect("parse independent Git fixture");
    assert_eq!(value["version"], 1);
    assert_eq!(value["minimum_git_version"], "2.45.0");
    assert_eq!(value["scope"], "workspace/cubikan");
    assert_eq!(value["seed"]["path"], "source.txt");
    assert_eq!(value["seed"]["mode"], "100644");
    assert_eq!(value["seed"]["initial_bytes"], "alpha\n");
    assert_eq!(value["seed"]["moved_path"], "moved.txt");
    assert_eq!(value["seed"]["moved_bytes"], "alpha moved and edited\n");
    assert_eq!(value["seed"]["commit_message"], "fixture root\n");
    assert_eq!(value["seed"]["timestamp"], "2001-02-03T04:05:06+0000");

    let expected_objects = [
        (
            ObjectFormat::Sha1,
            "git.commit.sha1",
            "15f37f16e70096c2624ca92fed9d55750640ec88",
            "5cd196672d408ab320aa7ddc30a90d0b68e11b4f",
            "4a58007052a65fbc2fc3f910f2855f45a4058e74",
        ),
        (
            ObjectFormat::Sha256,
            "git.commit.sha256",
            "a83c0f81c7ffc97d687a6b5bb9fe20b518beea3bee026273fe9bf2bae24a575c",
            "0b0967271d8137b2c6e693563dc4cb168b1014ec40e887c08e284cbdcbd2c070",
            "9f8bf964b2f278e643f6ee93dd5980698a5f515048b2a27134a294e5e3376180",
        ),
    ];
    for (format, namespace, commit, tree, blob) in expected_objects {
        let object = &value[format.name()];
        assert_eq!(object["object_format"], format.name());
        assert_eq!(object["namespace"], namespace);
        assert_eq!(object["commit_oid"], commit);
        assert_eq!(object["tree_oid"], tree);
        assert_eq!(object["blob_oid"], blob);
        assert_eq!(
            object["oid_length"].as_u64(),
            Some(format.oid_length() as u64)
        );
        for oid in [commit, tree, blob] {
            assert_eq!(oid.len(), format.oid_length());
            assert!(is_lower_hex(oid));
        }
    }

    assert_eq!(
        value["resolution_argv"],
        serde_json::json!([
            "--no-lazy-fetch",
            "-C",
            "<canonical-repository>",
            "rev-parse",
            "--verify",
            "--end-of-options",
            "<full-lower-oid>^{commit}"
        ])
    );
    assert_eq!(
        value["literal_environment"],
        serde_json::json!({
            "GIT_TERMINAL_PROMPT": "0",
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": "/dev/null",
            "GIT_NO_REPLACE_OBJECTS": "1",
            "GIT_NO_LAZY_FETCH": "1",
            "GIT_OPTIONAL_LOCKS": "0",
            "LC_ALL": "C"
        })
    );

    let typed: RecordedAssociation =
        serde_json::from_slice(EXPECTED_ASSOCIATION).expect("typed fixture association");
    assert_eq!(
        value["recorded_association"],
        serde_json::from_slice::<Value>(EXPECTED_ASSOCIATION)
            .expect("parse exact association bytes for semantic comparison")
    );
    assert_eq!(
        serde_json::to_vec(&typed).expect("round-trip typed fixture association"),
        EXPECTED_ASSOCIATION
    );

    let rejection = value["rejection_inputs"]
        .as_object()
        .expect("rejection fixture object");
    let exact_lengths = [
        ("sha1_39", 39),
        ("sha1_40_nonhex", 40),
        ("sha1_41", 41),
        ("sha1_uppercase", 40),
        ("sha256_63", 63),
        ("sha256_64_nonhex", 64),
        ("sha256_65", 65),
        ("sha256_uppercase", 64),
        ("leading_dash", 40),
        ("embedded_nul", 40),
    ];
    for (name, length) in exact_lengths {
        assert_eq!(
            rejection[name].as_str().map(str::len),
            Some(length),
            "exact rejection fixture length for {name}"
        );
    }
    assert!(
        rejection["embedded_nul"]
            .as_str()
            .expect("NUL fixture")
            .contains('\0')
    );

    Fixture { value }
}

fn validate_sha256_object_graph(fixture: &Fixture) {
    let blob = git_sha256("blob", SOURCE_V1);
    assert_eq!(hex_lower(&blob), fixture.blob_oid(ObjectFormat::Sha256));

    let mut tree_content = b"100644 source.txt\0".to_vec();
    tree_content.extend_from_slice(&blob);
    let tree = git_sha256("tree", &tree_content);
    assert_eq!(hex_lower(&tree), fixture.tree_oid(ObjectFormat::Sha256));

    let commit_content = format!(
        concat!(
            "tree {}\n",
            "author Fixture Author <fixture@example.invalid> 981173106 +0000\n",
            "committer Fixture Committer <fixture@example.invalid> 981173106 +0000\n",
            "\n",
            "fixture root\n"
        ),
        fixture.tree_oid(ObjectFormat::Sha256)
    );
    let commit = git_sha256("commit", commit_content.as_bytes());
    assert_eq!(hex_lower(&commit), fixture.commit_oid(ObjectFormat::Sha256));
}

fn git_sha256(kind: &str, content: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(format!("{kind} {}\0", content.len()).as_bytes());
    hasher.update(content);
    hasher.finalize().into()
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn without_one_lf(bytes: &[u8]) -> &[u8] {
    bytes.strip_suffix(b"\n").unwrap_or(bytes)
}

fn installed_git() -> PathBuf {
    if let Some(configured) = std::env::var_os("CUBIKAN_TEST_GIT") {
        return fs::canonicalize(configured).expect("canonical configured CUBIKAN_TEST_GIT");
    }
    let path = std::env::var_os("PATH").expect("PATH must locate installed Git for tests");
    for directory in std::env::split_paths(&path) {
        for executable in ["git", "git.exe"] {
            let candidate = directory.join(executable);
            if candidate.is_file() {
                return fs::canonicalize(&candidate).unwrap_or(candidate);
            }
        }
    }
    panic!("installed Git executable not found on PATH")
}

fn assert_required_git(git: &Path) {
    let stdout = git_stdout(git, None, ["--version"]);
    let version = parse_git_version(&stdout)
        .unwrap_or_else(|| panic!("installed Git version output is incompatible: {stdout:?}"));
    assert!(
        version >= GIT_MINIMUM,
        "T-1114 requires Git >=2.45.0; installed version is {version:?}"
    );
}

fn assert_global_no_lazy_fetch(git: &Path) {
    assert_git_success(
        git_output(git, None, ["--no-lazy-fetch", "--version"], &[]),
        "installed Git global --no-lazy-fetch capability",
    );
}

fn git_supports_sha256_repositories(git: &Path) -> Result<bool, String> {
    let directory = TestDirectory::new("sha256-capability");
    let output = git_output(
        git,
        None,
        [
            OsString::from("init"),
            OsString::from("--quiet"),
            OsString::from("--object-format=sha256"),
            OsString::from("--initial-branch=main"),
            directory.path().as_os_str().to_owned(),
        ],
        &[],
    );
    if output.status.success() {
        return Ok(true);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    if stderr.contains("unknown hash algorithm")
        || stderr.contains("unsupported hash algorithm")
        || (stderr.contains("unknown option") && stderr.contains("object-format"))
    {
        return Ok(false);
    }
    Err(format!(
        "git init exited {:?} without an explicit unsupported-SHA-256 diagnostic",
        output.status.code()
    ))
}

fn parse_git_version(stdout: &str) -> Option<(u64, u64, u64)> {
    let token = stdout
        .strip_prefix("git version ")?
        .split_whitespace()
        .next()?;
    let mut parts = token.split('.');
    let major = numeric_prefix(parts.next()?)?;
    let minor = numeric_prefix(parts.next()?)?;
    let patch = parts.next().and_then(numeric_prefix).unwrap_or(0);
    Some((major, minor, patch))
}

fn numeric_prefix(value: &str) -> Option<u64> {
    let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn resolver(git: &Path) -> GitCommitResolver {
    GitCommitResolver::new(git).expect("construct resolver from one canonical Git executable")
}

fn fixture_identity_environment() -> [(&'static str, &'static str); 6] {
    [
        ("GIT_AUTHOR_NAME", "Fixture Author"),
        ("GIT_AUTHOR_EMAIL", "fixture@example.invalid"),
        ("GIT_AUTHOR_DATE", "2001-02-03T04:05:06+0000"),
        ("GIT_COMMITTER_NAME", "Fixture Committer"),
        ("GIT_COMMITTER_EMAIL", "fixture@example.invalid"),
        ("GIT_COMMITTER_DATE", "2001-02-03T04:05:06+0000"),
    ]
}

fn git_output<I, S>(
    git: &Path,
    repository: Option<&Path>,
    arguments: I,
    extra_environment: &[(&str, &str)],
) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(git);
    command.env_clear().env("LC_ALL", "C");
    for (key, value) in [
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_NO_REPLACE_OBJECTS", "1"),
        ("GIT_NO_LAZY_FETCH", "1"),
        ("GIT_OPTIONAL_LOCKS", "0"),
    ] {
        command.env(key, value);
    }
    for (key, value) in extra_environment {
        command.env(key, value);
    }
    if let Some(repository) = repository {
        command.arg("-C").arg(repository);
    }
    command.args(arguments);
    command.output().expect("execute isolated installed Git")
}

fn git_stdout<I, S>(git: &Path, repository: Option<&Path>, arguments: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = git_output(git, repository, arguments, &[]);
    assert_git_success(output.clone(), "Git query");
    String::from_utf8(output.stdout)
        .expect("Git query stdout must be UTF-8")
        .trim_end_matches(['\r', '\n'])
        .to_owned()
}

fn assert_git_success(output: Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_reference(reference: &ExternalReference, namespace: &str, scope: &str, value: &str) {
    assert_eq!(reference.namespace().as_str(), namespace);
    assert_eq!(reference.scope().as_str(), scope);
    assert_eq!(reference.value().as_str(), value);
}

fn assert_noncanonical_oid(error: GitReferenceError, label: &str) {
    assert!(
        matches!(error, GitReferenceError::InvalidRevision),
        "{label} must report InvalidRevision, got {error:?}"
    );
    assert_path_free(&error, &[]);
}

fn assert_noncommit(error: GitReferenceError, label: &str, repository: &Path, revision: &str) {
    assert!(
        matches!(
            error,
            GitReferenceError::UnresolvableCommit | GitReferenceError::NonCanonicalObjectId
        ),
        "{label} must report a typed noncommit failure, got {error:?}"
    );
    assert_path_free(&error, &[repository, Path::new(revision)]);
}

fn assert_noncanonical_repository(error: GitReferenceError, label: &str, repository: &Path) {
    assert!(
        matches!(
            error,
            GitReferenceError::InvalidRepository | GitReferenceError::NonCanonicalRepository
        ),
        "{label} must report a typed repository failure, got {error:?}"
    );
    assert_path_free(&error, &[repository]);
}

fn assert_path_free(error: &GitReferenceError, sensitive_paths: &[&Path]) {
    let rendered = error.to_string();
    for path in sensitive_paths {
        let path = path.to_string_lossy();
        if !path.is_empty() {
            assert!(
                !rendered.contains(path.as_ref()),
                "error text leaked sensitive path or revision: {rendered:?}"
            );
        }
    }
    assert!(!rendered.contains("cubikan-git-"));
}

fn run_hostile_environment_cases(git: &Path, repository: &RepositoryFixture, scope: &str) {
    let executable = ExecutableDirectory::new("hostile-environment");
    let marker = executable.path().join("forbidden-helper-marker");
    let helper = executable.path().join("forbidden-helper");
    write_executable(
        &helper,
        &format!("#!/bin/sh\n: > '{}'\nexit 99\n", shell_quote(&marker)),
    );
    let hostile_root = TestDirectory::new("hostile-environment-target");
    let hostile_config = hostile_root.path().join("hostile.gitconfig");
    fs::write(
        &hostile_config,
        format!("[credential]\n\thelper = !{}\n", helper.display()),
    )
    .expect("write hostile config fixture");
    let cases: Vec<(&str, OsString)> = vec![
        ("GIT_DIR", hostile_root.path().as_os_str().to_owned()),
        ("GIT_WORK_TREE", hostile_root.path().as_os_str().to_owned()),
        (
            "GIT_OBJECT_DIRECTORY",
            hostile_root.path().as_os_str().to_owned(),
        ),
        (
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            hostile_root.path().as_os_str().to_owned(),
        ),
        ("GIT_COMMON_DIR", hostile_root.path().as_os_str().to_owned()),
        (
            "GIT_INDEX_FILE",
            hostile_root.path().join("index").into_os_string(),
        ),
        ("GIT_CONFIG_SYSTEM", hostile_config.as_os_str().to_owned()),
        ("GIT_CONFIG_GLOBAL", hostile_config.as_os_str().to_owned()),
        ("GIT_CONFIG_COUNT", OsString::from("1")),
        ("GIT_REPLACE_REF_BASE", OsString::from("refs/hostile/")),
        ("GIT_PROXY_COMMAND", helper.as_os_str().to_owned()),
        ("GIT_ASKPASS", helper.as_os_str().to_owned()),
        ("SSH_ASKPASS", helper.as_os_str().to_owned()),
        ("GIT_SSH", helper.as_os_str().to_owned()),
        ("GIT_SSH_COMMAND", helper.as_os_str().to_owned()),
        ("GIT_CREDENTIAL_HELPER", helper.as_os_str().to_owned()),
        ("HTTP_PROXY", OsString::from("http://127.0.0.1:9")),
        ("HTTPS_PROXY", OsString::from("http://127.0.0.1:9")),
        ("ALL_PROXY", OsString::from("socks5://127.0.0.1:9")),
    ];
    let before = ByteTree::capture(repository.path());
    for (name, value) in cases {
        let specification = format!(
            "{}|{}|{}|{}",
            git.display(),
            repository.path().display(),
            scope,
            repository.commit_oid
        );
        let output = Command::new(std::env::current_exe().expect("current test executable"))
            .arg("--exact")
            .arg(HOSTILE_TEST_NAME)
            .arg("--nocapture")
            .env_clear()
            .env(HOSTILE_CHILD, &specification)
            .env(name, value)
            .output()
            .unwrap_or_else(|error| {
                panic!("run serialized hostile environment case {name}: {error}")
            });
        assert!(
            output.status.success(),
            "hostile environment case {name} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!marker.exists(), "{name} invoked a forbidden helper");
        assert_eq!(ByteTree::capture(repository.path()), before);
    }
}

fn run_hostile_environment_child(specification: &str) {
    let fields: Vec<_> = specification.split('|').collect();
    assert_eq!(fields.len(), 4, "hostile child specification");
    let git = PathBuf::from(fields[0]);
    let repository = PathBuf::from(fields[1]);
    let scope = fields[2];
    let revision = fields[3];
    let error = match GitCommitResolver::new(&git) {
        Err(error) => error,
        Ok(resolver) => resolver
            .resolve_commit(&repository, scope, revision)
            .expect_err("hostile inherited environment must fail typed before Git execution"),
    };
    assert!(
        matches!(error, GitReferenceError::DangerousEnvironment),
        "hostile inherited environment must report DangerousEnvironment, got {error:?}"
    );
    assert_path_free(&error, &[&repository]);
}

struct ExecutableDirectory {
    path: PathBuf,
}

impl ExecutableDirectory {
    fn new(label: &str) -> Self {
        let serial = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let name = format!("cubikan-git-{}-{label}-{serial}", std::process::id());
        let execution_root = Path::new("/run/cubikan-exec");
        let path = if execution_root.is_dir() {
            execution_root.join(name)
        } else {
            std::env::temp_dir().join(name)
        };
        fs::create_dir(&path).unwrap_or_else(|error| {
            panic!(
                "create executable fixture directory {}: {error}",
                path.display()
            )
        });
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ExecutableDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents)
        .unwrap_or_else(|error| panic!("write executable fixture {}: {error}", path.display()));
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .unwrap_or_else(|error| panic!("chmod executable fixture {}: {error}", path.display()));
}

fn shell_quote(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "'\\''")
}

#[cfg(unix)]
fn assert_exact_process_contract(fixture: &Fixture, repository: &Path, revision: &str) {
    let before = ByteTree::capture(repository);
    let fake = FakeGit::new(repository, revision, "sha1", FakeBehavior::Exact);
    let reference = resolver(&fake.executable)
        .resolve_commit(repository, fixture.scope(), revision)
        .expect("exact fake-Git process trace must resolve");
    assert_reference(&reference, "git.commit.sha1", fixture.scope(), revision);
    assert_eq!(ByteTree::capture(repository), before);

    let trace = fs::read_to_string(&fake.log).expect("read exact fake-Git trace");
    let executable = fake.executable.to_string_lossy();
    let final_call = format!(
        "CALL\t--no-lazy-fetch\t-C\t{}\trev-parse\t--verify\t--end-of-options\t{}^{{commit}}",
        repository.display(),
        revision
    );
    assert!(
        trace.lines().any(|line| line == final_call),
        "missing exact final argv in trace:\n{trace}"
    );
    let mut execution_count = 0;
    let mut environment_count = 0;
    for line in trace.lines() {
        if let Some(observed) = line.strip_prefix("EXEC\t") {
            execution_count += 1;
            assert_eq!(observed, executable, "all probes use the same executable");
        }
        if let Some(observed) = line.strip_prefix("ENV\t") {
            environment_count += 1;
            assert_eq!(
                observed, "C\t0\t1\t/dev/null\t1\t1\t0",
                "literal child environment"
            );
        }
    }
    assert!(
        execution_count >= 3,
        "version/capability/format/final probes"
    );
    assert_eq!(environment_count, execution_count);
    assert!(!fake.helper_marker.exists());
}

#[cfg(unix)]
fn assert_on_disk_alternates_rejected(git: &Path, fixture: &Fixture) {
    let repository = RepositoryFixture::new(git, ObjectFormat::Sha1);
    let alternate_root = TestDirectory::new("alternate-object-root");
    let alternates = repository.path().join(".git/objects/info/alternates");
    fs::write(
        &alternates,
        format!("{}\n", alternate_root.path().display()),
    )
    .expect("write nonempty common-dir alternates file");
    let before = ByteTree::capture(repository.path());
    let error = resolver(git)
        .resolve_commit(repository.path(), fixture.scope(), &repository.commit_oid)
        .expect_err("common-dir alternates must fail closed");
    assert!(
        matches!(error, GitReferenceError::AlternateObjectStore),
        "common-dir alternates must report AlternateObjectStore, got {error:?}"
    );
    assert_path_free(&error, &[repository.path(), alternate_root.path()]);
    assert_eq!(ByteTree::capture(repository.path()), before);

    let repository = RepositoryFixture::new(git, ObjectFormat::Sha1);
    let worktree_parent = TestDirectory::new("linked-worktree-parent");
    let worktree = worktree_parent.path().join("worktree");
    assert_git_success(
        git_output(
            git,
            Some(repository.path()),
            [
                OsString::from("worktree"),
                OsString::from("add"),
                OsString::from("--quiet"),
                OsString::from("-b"),
                OsString::from("fixture-worktree"),
                worktree.as_os_str().to_owned(),
            ],
            &[],
        ),
        "create linked worktree",
    );
    let gitdir_file = fs::read_to_string(worktree.join(".git")).expect("read worktree gitdir");
    let gitdir = PathBuf::from(
        gitdir_file
            .trim()
            .strip_prefix("gitdir: ")
            .expect("worktree .git indirection"),
    );
    let worktree_alternates = gitdir.join("objects/info/alternates");
    fs::create_dir_all(
        worktree_alternates
            .parent()
            .expect("worktree objects/info parent"),
    )
    .expect("create worktree objects/info");
    fs::write(
        &worktree_alternates,
        format!("{}\n", alternate_root.path().display()),
    )
    .expect("write nonempty worktree alternates file");
    let before_main = ByteTree::capture(repository.path());
    let before_worktree = ByteTree::capture(&worktree);
    let error = resolver(git)
        .resolve_commit(&worktree, fixture.scope(), &repository.commit_oid)
        .expect_err("worktree-local alternates must fail closed");
    assert!(
        matches!(
            error,
            GitReferenceError::AlternateObjectStore | GitReferenceError::NonCanonicalRepository
        ),
        "linked worktree must fail at its canonical or alternate-store gate, got {error:?}"
    );
    assert_path_free(&error, &[&worktree, alternate_root.path()]);
    assert_eq!(ByteTree::capture(repository.path()), before_main);
    assert_eq!(ByteTree::capture(&worktree), before_worktree);

    let repository = RepositoryFixture::new(git, ObjectFormat::Sha1);
    let real_objects = repository.path().join(".git/objects-real");
    fs::rename(repository.path().join(".git/objects"), &real_objects)
        .expect("move real object directory");
    symlink("objects-real", repository.path().join(".git/objects"))
        .expect("symlink object directory");
    let before = ByteTree::capture(repository.path());
    let error = resolver(git)
        .resolve_commit(repository.path(), fixture.scope(), &repository.commit_oid)
        .expect_err("symlink object store must fail closed");
    assert!(
        matches!(error, GitReferenceError::UnsafeObjectStore),
        "symlink object store must report UnsafeObjectStore, got {error:?}"
    );
    assert_path_free(&error, &[repository.path(), &real_objects]);
    assert_eq!(ByteTree::capture(repository.path()), before);

    let repository = RepositoryFixture::new(git, ObjectFormat::Sha1);
    assert_git_success(
        git_output(git, Some(repository.path()), ["gc", "--quiet"], &[]),
        "pack repository for external-pack rejection",
    );
    let outside = TestDirectory::new("external-pack-store");
    let pack = repository.path().join(".git/objects/pack");
    let outside_pack = outside.path().join("pack");
    fs::rename(&pack, &outside_pack).expect("move pack directory outside repository");
    symlink(&outside_pack, &pack).expect("symlink external pack directory");
    let before_repository = ByteTree::capture(repository.path());
    let before_outside = ByteTree::capture(outside.path());
    let error = resolver(git)
        .resolve_commit(repository.path(), fixture.scope(), &repository.commit_oid)
        .expect_err("external symlinked pack directory must fail closed");
    assert!(
        matches!(error, GitReferenceError::UnsafeObjectStore),
        "external pack directory must report UnsafeObjectStore, got {error:?}"
    );
    assert_path_free(&error, &[repository.path(), outside.path()]);
    assert_eq!(ByteTree::capture(repository.path()), before_repository);
    assert_eq!(ByteTree::capture(outside.path()), before_outside);

    let repository = RepositoryFixture::new(git, ObjectFormat::Sha1);
    let outside = TestDirectory::new("external-loose-store");
    let prefix = &repository.commit_oid[..2];
    let loose = repository.path().join(".git/objects").join(prefix);
    let outside_loose = outside.path().join(prefix);
    fs::rename(&loose, &outside_loose).expect("move loose-object directory outside repository");
    symlink(&outside_loose, &loose).expect("symlink external loose-object directory");
    let before_repository = ByteTree::capture(repository.path());
    let before_outside = ByteTree::capture(outside.path());
    let error = resolver(git)
        .resolve_commit(repository.path(), fixture.scope(), &repository.commit_oid)
        .expect_err("external symlinked loose-object directory must fail closed");
    assert!(
        matches!(error, GitReferenceError::UnsafeObjectStore),
        "external loose objects must report UnsafeObjectStore, got {error:?}"
    );
    assert_path_free(&error, &[repository.path(), outside.path()]);
    assert_eq!(ByteTree::capture(repository.path()), before_repository);
    assert_eq!(ByteTree::capture(outside.path()), before_outside);
}

#[cfg(unix)]
fn assert_replace_state_rejected(git: &Path, fixture: &Fixture) {
    let repository = RepositoryFixture::new(git, ObjectFormat::Sha1);
    let replace = repository
        .path()
        .join(".git/refs/replace")
        .join(&repository.commit_oid);
    fs::create_dir_all(replace.parent().expect("replace ref parent"))
        .expect("create real replace ref namespace");
    fs::write(&replace, format!("{}\n", repository.tree_oid))
        .expect("write on-disk replacement ref");
    let before = ByteTree::capture(repository.path());
    let error = resolver(git)
        .resolve_commit(repository.path(), fixture.scope(), &repository.commit_oid)
        .expect_err("real on-disk replace state must fail closed");
    assert!(
        matches!(error, GitReferenceError::ReplaceObjectsPresent),
        "replace state must report ReplaceObjectsPresent, got {error:?}"
    );
    assert_path_free(&error, &[repository.path()]);
    assert_eq!(ByteTree::capture(repository.path()), before);
}

#[cfg(unix)]
fn assert_promisor_and_helper_state_rejected(git: &Path, fixture: &Fixture) {
    let promisor = RepositoryFixture::new(git, ObjectFormat::Sha1);
    assert_git_success(
        git_output(
            git,
            Some(promisor.path()),
            ["config", "remote.origin.promisor", "true"],
            &[],
        ),
        "configure promisor repository state",
    );
    assert_git_success(
        git_output(
            git,
            Some(promisor.path()),
            ["config", "extensions.partialClone", "origin"],
            &[],
        ),
        "configure partial clone repository state",
    );
    let before = ByteTree::capture(promisor.path());
    let error = resolver(git)
        .resolve_commit(promisor.path(), fixture.scope(), &promisor.commit_oid)
        .expect_err("promisor/partial-clone state must fail closed");
    assert!(
        matches!(error, GitReferenceError::PromisorObjectsPresent),
        "promisor state must report PromisorObjectsPresent, got {error:?}"
    );
    assert_path_free(&error, &[promisor.path()]);
    assert_eq!(ByteTree::capture(promisor.path()), before);

    for (key, value) in [
        ("remote.origin.vcs", "forbidden-helper"),
        ("remote.origin.url", "forbidden-helper::opaque"),
    ] {
        let remote = RepositoryFixture::new(git, ObjectFormat::Sha1);
        assert_git_success(
            git_output(git, Some(remote.path()), ["config", key, value], &[]),
            "configure fake remote-helper state",
        );
        let before = ByteTree::capture(remote.path());
        let error = resolver(git)
            .resolve_commit(remote.path(), fixture.scope(), &remote.commit_oid)
            .expect_err("fake remote-helper state must fail independently");
        assert!(
            matches!(error, GitReferenceError::DangerousRepositoryConfig),
            "{key} must report DangerousRepositoryConfig, got {error:?}"
        );
        assert_path_free(&error, &[remote.path()]);
        assert_eq!(ByteTree::capture(remote.path()), before);
    }

    let repository = RepositoryFixture::new(git, ObjectFormat::Sha1);
    let executable = ExecutableDirectory::new("forbidden-helper");
    let marker = executable.path.join("helper-marker");
    let helper = executable.path.join("credential-helper");
    write_executable(
        &helper,
        &format!("#!/bin/sh\n: > '{}'\nexit 98\n", shell_quote(&marker)),
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind synthetic loopback listener");
    listener
        .set_nonblocking(true)
        .expect("make synthetic listener nonblocking");
    let proxy = format!(
        "http://{}",
        listener.local_addr().expect("listener address")
    );
    assert_git_success(
        git_output(
            git,
            Some(repository.path()),
            [
                OsString::from("config"),
                OsString::from("credential.helper"),
                OsString::from(format!("!{}", helper.display())),
            ],
            &[],
        ),
        "configure forbidden credential helper",
    );
    assert_git_success(
        git_output(
            git,
            Some(repository.path()),
            [
                "config",
                "remote.origin.url",
                "https://example.invalid/repo",
            ],
            &[],
        ),
        "configure fake remote",
    );
    assert_git_success(
        git_output(
            git,
            Some(repository.path()),
            ["config", "http.proxy", proxy.as_str()],
            &[],
        ),
        "configure synthetic proxy listener",
    );
    let before = ByteTree::capture(repository.path());
    let error = resolver(git)
        .resolve_commit(repository.path(), fixture.scope(), &repository.commit_oid)
        .expect_err("dangerous repository helper/config state must fail closed");
    assert!(
        matches!(error, GitReferenceError::DangerousRepositoryConfig),
        "helper/config state must report DangerousRepositoryConfig, got {error:?}"
    );
    assert_path_free(&error, &[repository.path(), &helper]);
    assert_eq!(ByteTree::capture(repository.path()), before);
    let helper_invocations = u64::from(marker.exists());
    assert_eq!(helper_invocations, 0, "credential helper invocation count");
    let network_connections = match listener.accept() {
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => 0,
        Ok(_) => 1,
        Err(error) => panic!("inspect synthetic listener: {error}"),
    };
    assert_eq!(network_connections, 0, "synthetic network connection count");
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum FakeBehavior {
    Exact,
    OldVersion,
    NoLazyFetchUnsupported,
    UnsupportedFormat,
    ExtraStdout,
    OutputOverflow,
    Timeout,
    CommandFailure,
}

#[cfg(unix)]
struct FakeGit {
    _directory: ExecutableDirectory,
    executable: PathBuf,
    log: PathBuf,
    helper_marker: PathBuf,
    descendant_marker: PathBuf,
}

#[cfg(unix)]
impl FakeGit {
    fn new(repository: &Path, oid: &str, format: &str, behavior: FakeBehavior) -> Self {
        let directory = ExecutableDirectory::new("fake-git");
        let executable = directory.path.join("git");
        let log = directory.path.join("trace");
        let helper_marker = directory.path.join("helper-marker");
        let descendant_marker = directory.path.join("descendant-pid");
        let (version, no_lazy, object_format, final_action) = match behavior {
            FakeBehavior::Exact => (
                "git version 2.53.0",
                "version",
                format,
                format!("printf '%s\\n' '{}'", oid),
            ),
            FakeBehavior::OldVersion => (
                "git version 2.44.9",
                "version",
                format,
                format!("printf '%s\\n' '{}'", oid),
            ),
            FakeBehavior::NoLazyFetchUnsupported => (
                "git version 2.53.0",
                "unsupported",
                format,
                format!("printf '%s\\n' '{}'", oid),
            ),
            FakeBehavior::UnsupportedFormat => (
                "git version 2.53.0",
                "version",
                "sha512",
                format!("printf '%s\\n' '{}'", oid),
            ),
            FakeBehavior::ExtraStdout => (
                "git version 2.53.0",
                "version",
                format,
                format!("printf '%s\\nextra\\n' '{}'", oid),
            ),
            FakeBehavior::OutputOverflow => (
                "git version 2.53.0",
                "version",
                format,
                "i=0; while [ \"$i\" -lt 256 ]; do printf '%s' xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; i=$((i + 1)); done"
                    .to_owned(),
            ),
            FakeBehavior::Timeout => (
                "git version 2.53.0",
                "version",
                format,
                format!(
                    "/bin/sh -c 'while :; do :; done' & descendant=$!; printf '%s\\n' \"$descendant\" > '{}'; wait \"$descendant\"",
                    shell_quote(&descendant_marker)
                ),
            ),
            FakeBehavior::CommandFailure => (
                "git version 2.53.0",
                "version",
                format,
                "printf '%s\\n' failure >&2; exit 66".to_owned(),
            ),
        };
        let template = r#"#!/bin/sh
log='@LOG@'
printf 'EXEC\t%s\n' "$0" >> "$log"
printf 'CALL' >> "$log"
for argument in "$@"; do
  printf '\t%s' "$argument" >> "$log"
done
printf '\n' >> "$log"
printf 'ENV\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$LC_ALL" "$GIT_TERMINAL_PROMPT" "$GIT_CONFIG_NOSYSTEM" "$GIT_CONFIG_GLOBAL" "$GIT_NO_REPLACE_OBJECTS" "$GIT_NO_LAZY_FETCH" "$GIT_OPTIONAL_LOCKS" >> "$log"
if [ "${GIT_DIR+x}" = x ] || [ "${GIT_WORK_TREE+x}" = x ] || [ "${GIT_OBJECT_DIRECTORY+x}" = x ] || [ "${GIT_ALTERNATE_OBJECT_DIRECTORIES+x}" = x ] || [ "${GIT_COMMON_DIR+x}" = x ] || [ "${GIT_INDEX_FILE+x}" = x ] || [ "${GIT_CONFIG_SYSTEM+x}" = x ] || [ "${GIT_CONFIG_COUNT+x}" = x ] || [ "${GIT_REPLACE_REF_BASE+x}" = x ] || [ "${GIT_PROXY_COMMAND+x}" = x ] || [ "${GIT_ASKPASS+x}" = x ] || [ "${GIT_PAGER+x}" = x ] || [ "${SSH_ASKPASS+x}" = x ] || [ "${GIT_SSH+x}" = x ] || [ "${GIT_SSH_COMMAND+x}" = x ] || [ "${GIT_CREDENTIAL_HELPER+x}" = x ] || [ "${HTTP_PROXY+x}" = x ] || [ "${HTTPS_PROXY+x}" = x ] || [ "${ALL_PROXY+x}" = x ]; then
  : > '@HELPER_MARKER@'
  exit 91
fi
if [ "$LC_ALL" != C ] || [ "$GIT_TERMINAL_PROMPT" != 0 ] || [ "$GIT_CONFIG_NOSYSTEM" != 1 ] || [ "$GIT_CONFIG_GLOBAL" != /dev/null ] || [ "$GIT_NO_REPLACE_OBJECTS" != 1 ] || [ "$GIT_NO_LAZY_FETCH" != 1 ] || [ "$GIT_OPTIONAL_LOCKS" != 0 ]; then
  exit 92
fi
if [ "$#" -eq 1 ] && [ "$1" = --version ]; then
  printf '%s\n' '@VERSION@'
  exit 0
fi
if [ "$#" -eq 2 ] && [ "$1" = --no-lazy-fetch ] && [ "$2" = --version ]; then
  if [ '@NO_LAZY@' = unsupported ]; then
    exit 129
  fi
  printf '%s\n' '@VERSION@'
  exit 0
fi
case " $* " in
  *" config --local --no-includes --null --name-only --list "*) printf 'core.repositoryformatversion\000core.filemode\000core.bare\000core.logallrefupdates\000'; exit 0 ;;
  *" rev-parse --path-format=absolute --absolute-git-dir --git-common-dir --git-path objects --show-object-format "*) printf '%s/.git\n%s/.git\n%s/.git/objects\n%s\n' '@REPOSITORY@' '@REPOSITORY@' '@REPOSITORY@' '@FORMAT@'; exit 0 ;;
  *" rev-parse --path-format=absolute --show-toplevel "*) printf '%s\n' '@REPOSITORY@'; exit 0 ;;
  *" rev-parse --show-object-format "*) printf '%s\n' '@FORMAT@'; exit 0 ;;
  *" rev-parse --show-toplevel "*) printf '%s\n' '@REPOSITORY@'; exit 0 ;;
  *" rev-parse --absolute-git-dir "*) printf '%s/.git\n' '@REPOSITORY@'; exit 0 ;;
  *" rev-parse --git-common-dir "*) printf '%s/.git\n' '@REPOSITORY@'; exit 0 ;;
  *" rev-parse --git-dir "*) printf '%s/.git\n' '@REPOSITORY@'; exit 0 ;;
  *" rev-parse --is-inside-work-tree "*) printf 'true\n'; exit 0 ;;
esac
if [ "$#" -eq 7 ] && [ "$1" = --no-lazy-fetch ] && [ "$2" = -C ] && [ "$3" = '@REPOSITORY@' ] && [ "$4" = rev-parse ] && [ "$5" = --verify ] && [ "$6" = --end-of-options ] && [ "$7" = '@OID@^{commit}' ]; then
  @FINAL_ACTION@
  exit 0
fi
printf 'unsupported fake Git invocation\n' >&2
exit 97
"#;
        let script = template
            .replace("@LOG@", &shell_quote(&log))
            .replace("@HELPER_MARKER@", &shell_quote(&helper_marker))
            .replace("@VERSION@", version)
            .replace("@NO_LAZY@", no_lazy)
            .replace("@FORMAT@", object_format)
            .replace("@REPOSITORY@", &shell_quote(repository))
            .replace("@OID@", oid)
            .replace("@FINAL_ACTION@", &final_action);
        write_executable(&executable, &script);
        Self {
            _directory: directory,
            executable,
            log,
            helper_marker,
            descendant_marker,
        }
    }
}

#[cfg(unix)]
fn assert_incompatible_git_and_process_faults(fixture: &Fixture, repository: &RepositoryFixture) {
    let revision = repository.commit_oid.as_str();
    let cases = [
        (
            "old-version",
            FakeBehavior::OldVersion,
            ExpectedError::UnsupportedGitVersion,
        ),
        (
            "no-lazy-fetch",
            FakeBehavior::NoLazyFetchUnsupported,
            ExpectedError::NoLazyFetchUnsupported,
        ),
        (
            "unsupported-format",
            FakeBehavior::UnsupportedFormat,
            ExpectedError::UnsupportedObjectFormat,
        ),
        (
            "extra-stdout",
            FakeBehavior::ExtraStdout,
            ExpectedError::NonCanonicalObjectId,
        ),
        (
            "output-overflow",
            FakeBehavior::OutputOverflow,
            ExpectedError::OutputLimitExceeded,
        ),
        (
            "timeout",
            FakeBehavior::Timeout,
            ExpectedError::CommandTimedOut,
        ),
        (
            "command-failure",
            FakeBehavior::CommandFailure,
            ExpectedError::UnresolvableCommit,
        ),
    ];
    for (label, behavior, expected) in cases {
        let fake = FakeGit::new(repository.path(), revision, "sha1", behavior);
        let before = ByteTree::capture(repository.path());
        let error = match GitCommitResolver::new(&fake.executable) {
            Err(error) => error,
            Ok(resolver) => resolver
                .resolve_commit(repository.path(), fixture.scope(), revision)
                .expect_err("incompatible Git/process fault must not produce a reference"),
        };
        assert_expected_error(error, expected, label, repository.path(), &fake.executable);
        assert_eq!(ByteTree::capture(repository.path()), before);
        assert!(!fake.helper_marker.exists());
        if matches!(behavior, FakeBehavior::Timeout) {
            let pid = fs::read_to_string(&fake.descendant_marker)
                .expect("timeout fake must record its descendant PID");
            let pid = pid.trim();
            for _ in 0..100 {
                if !process_is_running(pid) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            assert!(
                !process_is_running(pid),
                "timeout left descendant process {pid} running"
            );
        }
    }

    let directory = ExecutableDirectory::new("invalid-executable");
    let invalid = directory.path.join("not-executable");
    fs::write(&invalid, b"not executable\n").expect("write invalid executable fixture");
    let error = GitCommitResolver::new(&invalid).expect_err("non-executable Git path must fail");
    assert!(
        matches!(error, GitReferenceError::InvalidGitExecutable),
        "invalid executable must report InvalidGitExecutable, got {error:?}"
    );
    assert_path_free(&error, &[&invalid]);

    let broken = directory.path.join("broken-interpreter-git");
    write_executable(&broken, "#!/definitely/not/a/real/interpreter\n");
    let error = match GitCommitResolver::new(&broken) {
        Err(error) => error,
        Ok(resolver) => resolver
            .resolve_commit(repository.path(), fixture.scope(), revision)
            .expect_err("unspawnable executable must fail typed"),
    };
    assert!(
        matches!(error, GitReferenceError::CommandFailed),
        "spawn failure must report CommandFailed, got {error:?}"
    );
    assert_path_free(&error, &[&broken, repository.path()]);
}

#[cfg(unix)]
fn process_is_running(pid: &str) -> bool {
    let Ok(stat) = fs::read_to_string(Path::new("/proc").join(pid).join("stat")) else {
        return false;
    };
    let Some((_, suffix)) = stat.rsplit_once(") ") else {
        return true;
    };
    !suffix.starts_with('Z')
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum ExpectedError {
    UnsupportedGitVersion,
    NoLazyFetchUnsupported,
    UnsupportedObjectFormat,
    NonCanonicalObjectId,
    OutputLimitExceeded,
    CommandTimedOut,
    UnresolvableCommit,
}

#[cfg(unix)]
fn assert_expected_error(
    error: GitReferenceError,
    expected: ExpectedError,
    label: &str,
    repository: &Path,
    executable: &Path,
) {
    let matches = match expected {
        ExpectedError::UnsupportedGitVersion => {
            matches!(error, GitReferenceError::UnsupportedGitVersion)
        }
        ExpectedError::NoLazyFetchUnsupported => {
            matches!(error, GitReferenceError::NoLazyFetchUnsupported)
        }
        ExpectedError::UnsupportedObjectFormat => {
            matches!(error, GitReferenceError::UnsupportedObjectFormat)
        }
        ExpectedError::NonCanonicalObjectId => {
            matches!(error, GitReferenceError::NonCanonicalObjectId)
        }
        ExpectedError::OutputLimitExceeded => {
            matches!(error, GitReferenceError::OutputLimitExceeded)
        }
        ExpectedError::CommandTimedOut => matches!(error, GitReferenceError::CommandTimedOut),
        ExpectedError::UnresolvableCommit => {
            matches!(error, GitReferenceError::UnresolvableCommit)
        }
    };
    assert!(
        matches,
        "{label} returned unexpected typed error: {error:?}"
    );
    assert_path_free(&error, &[repository, executable]);
}
