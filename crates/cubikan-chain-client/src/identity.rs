use std::{error::Error, fmt, io, num::NonZeroU32};

#[cfg(target_os = "linux")]
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    os::{
        fd::AsRawFd,
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Path, PathBuf},
};

use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

const DEPLOYMENT_ANCHOR_BYTES: &[u8] =
    include_bytes!("../../../chain/artifacts/local-deployment-anchor-v1.json");
const METADATA_BYTES: &[u8] = include_bytes!("../../../chain/metadata/cubikan-runtime-v1.scale");
const RUNTIME_WASM_BYTES: &[u8] =
    include_bytes!("../../../chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm");

const DEPLOYMENT_ANCHOR_SIZE: usize = 5_868;
const DEPLOYMENT_ANCHOR_SHA256: &str =
    "aa58c83fb0cfcb27be160aa8ca150f78ee3fff1cfc3868dbd073c92581c69887";
const METADATA_SIZE: usize = 63_327;
const METADATA_SHA256: &str = "171a323b1e6bf0122e549eecd5f5932e672a3e0835f32edf0b8808cfefd97302";
const RUNTIME_WASM_SIZE: usize = 637_930;
const RUNTIME_WASM_SHA256: &str =
    "640cc616674fe7393fc93928904f0fd92d77571209c8200f08b8da6290c6a275";
#[cfg(target_os = "linux")]
const MAX_PROC_CMDLINE_BYTES: u64 = 65_536;
#[cfg(target_os = "linux")]
const MAX_PROC_STAT_BYTES: u64 = 4_096;
#[cfg(target_os = "linux")]
const POLKADOT_OMNI_NODE_SIZE: u64 = 158_034_760;
#[cfg(target_os = "linux")]
const POLKADOT_OMNI_NODE_SHA256: &str =
    "ff8e5253e8a3e30b421c83d938a3245bdc5de222d807aaf3648575ae029faece";
#[cfg(target_os = "linux")]
const POLKADOT_OMNI_NODE_BASENAME: &[u8] = b"polkadot-omni-node";
#[cfg(target_os = "linux")]
const SEALED_NODE_EXECUTABLE_LINK: &[u8] = b"/memfd:cubikan-sealed-exec-v1 (deleted)";

/// A canonical, explicit-port WebSocket URL bound to loopback.
///
/// The host is either canonical dotted-decimal `127.0.0.0/8` or exact `[::1]`.
/// The port is canonical decimal `1..=65535` except `80`, and the root slash is
/// mandatory. Parsing never accepts a DNS name, credentials, query, fragment,
/// alternate IP spelling, TLS, or normalization of a different literal.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StrictLoopbackWsUrl {
    canonical: String,
    port: u16,
}

impl StrictLoopbackWsUrl {
    /// Parses the closed local archive-RPC URL grammar.
    pub fn parse(input: &str) -> Result<Self, LoopbackUrlError> {
        if !input.is_ascii() || !input.starts_with("ws://") || !input.ends_with('/') {
            return Err(LoopbackUrlError::NonCanonical);
        }

        let authority = input
            .strip_prefix("ws://")
            .and_then(|value| value.strip_suffix('/'))
            .ok_or(LoopbackUrlError::NonCanonical)?;
        let (host, port_text) = if let Some(port) = authority.strip_prefix("[::1]:") {
            ("[::1]", port)
        } else {
            let (host, port) = authority
                .rsplit_once(':')
                .ok_or(LoopbackUrlError::MissingPort)?;
            validate_ipv4_loopback(host)?;
            (host, port)
        };
        if port_text.is_empty()
            || !port_text.bytes().all(|byte| byte.is_ascii_digit())
            || (port_text.len() > 1 && port_text.starts_with('0'))
        {
            return Err(LoopbackUrlError::NonCanonical);
        }
        let port = port_text
            .parse::<u16>()
            .map_err(|_| LoopbackUrlError::NonCanonical)?;
        if port == 0 {
            return Err(LoopbackUrlError::ZeroPort);
        }
        if port == 80 {
            return Err(LoopbackUrlError::DefaultPort);
        }

        let parsed = Url::parse(input).map_err(LoopbackUrlError::Parse)?;
        if parsed.scheme() != "ws"
            || parsed.port() != Some(port)
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.path() != "/"
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(LoopbackUrlError::NonCanonical);
        }

        let canonical = format!("ws://{host}:{port}/");
        if input != canonical || parsed.as_str() != canonical {
            return Err(LoopbackUrlError::NonCanonical);
        }

        Ok(Self { canonical, port })
    }

    /// Returns the one canonical spelling, including its root slash.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.canonical
    }

    /// Returns the explicit nonzero RPC port.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }
}

fn validate_ipv4_loopback(host: &str) -> Result<(), LoopbackUrlError> {
    let octets = host.split('.').collect::<Vec<_>>();
    if octets.len() != 4 {
        return Err(LoopbackUrlError::NonCanonical);
    }
    let mut values = [0_u8; 4];
    for (index, octet) in octets.iter().enumerate() {
        if octet.is_empty()
            || !octet.bytes().all(|byte| byte.is_ascii_digit())
            || (octet.len() > 1 && octet.starts_with('0'))
        {
            return Err(LoopbackUrlError::NonCanonical);
        }
        values[index] = octet
            .parse::<u8>()
            .map_err(|_| LoopbackUrlError::NonCanonical)?;
    }
    if values[0] == 127 {
        Ok(())
    } else {
        Err(LoopbackUrlError::NonCanonical)
    }
}

impl fmt::Display for StrictLoopbackWsUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Rejection from the strict local archive-RPC URL grammar.
#[derive(Debug)]
pub enum LoopbackUrlError {
    /// The URL library rejected the input before the closed grammar was checked.
    Parse(url::ParseError),
    /// No explicit port was present.
    MissingPort,
    /// Port zero is never a usable listener identity.
    ZeroPort,
    /// WebSocket port 80 would be normalized away by the URL parser.
    DefaultPort,
    /// The input was valid URL syntax but outside the exact accepted spelling.
    NonCanonical,
}

impl fmt::Display for LoopbackUrlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "archive RPC URL is malformed: {error}"),
            Self::MissingPort => formatter.write_str("archive RPC URL must contain an explicit port"),
            Self::ZeroPort => formatter.write_str("archive RPC URL port must be nonzero"),
            Self::DefaultPort => {
                formatter.write_str("archive RPC URL must not use normalized default port 80")
            }
            Self::NonCanonical => formatter.write_str(
                "archive RPC URL must use a canonical loopback IP, explicit nondefault port, and literal root path",
            ),
        }
    }
}

impl Error for LoopbackUrlError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            _ => None,
        }
    }
}

/// Process-backed proof that the selected local node was started in archive mode.
///
/// This pins the Linux process and executable objects, authenticates the exact
/// reviewed executable bytes (and the launcher's complete memfd seal set when
/// applicable), reads bounded `/proc/<pid>/cmdline` bytes, and requires exact
/// pruning pairs plus one exact IPv4-only primary RPC endpoint. Legacy/global
/// RPC listener flags are rejected. Listener ownership is additionally checked
/// by the Sprint 11 network harness.
#[derive(Debug)]
pub struct ArchiveNodeEvidence {
    pid: NonZeroU32,
    endpoint: StrictLoopbackWsUrl,
    #[cfg(target_os = "linux")]
    _process: AuthenticatedNodeProcess,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct AuthenticatedNodeProcess {
    // Holding all three descriptors prevents the authenticated kernel objects
    // from being substituted while the evidence is consumed.
    _pidfd: rustix::fd::OwnedFd,
    _proc_directory: File,
    _executable: File,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeExecutableKind {
    Pathname,
    SealedMemfd,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct KernelFileIdentity {
    device: u64,
    inode: u64,
    size: u64,
    mode: u32,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(target_os = "linux")]
impl KernelFileIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.len(),
            mode: metadata.mode(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }

    fn same_object(self, other: Self) -> bool {
        self.device == other.device && self.inode == other.inode
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
struct ExpectedExecutable<'a> {
    size: u64,
    sha256: &'a str,
}

#[cfg(target_os = "linux")]
const PINNED_OMNI_NODE: ExpectedExecutable<'static> = ExpectedExecutable {
    size: POLKADOT_OMNI_NODE_SIZE,
    sha256: POLKADOT_OMNI_NODE_SHA256,
};

impl ArchiveNodeEvidence {
    /// Authenticates a stable Linux process/executable pair and requires each
    /// exact archive flag once, with no contradictory spelling.
    pub fn from_proc_pid(
        pid: u32,
        endpoint: &StrictLoopbackWsUrl,
    ) -> Result<Self, NodeEvidenceError> {
        let pid = NonZeroU32::new(pid).ok_or(NodeEvidenceError::ZeroPid)?;
        #[cfg(not(target_os = "linux"))]
        {
            let _ = endpoint;
            return Err(NodeEvidenceError::UnsupportedPlatform);
        }
        #[cfg(target_os = "linux")]
        {
            let process = authenticate_node_process(pid, endpoint)?;
            Ok(Self {
                pid,
                endpoint: endpoint.clone(),
                _process: process,
            })
        }
    }

    /// Returns the process whose command line supplied the evidence.
    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.pid.get()
    }

    pub(crate) fn endpoint(&self) -> &StrictLoopbackWsUrl {
        &self.endpoint
    }
}

#[cfg(target_os = "linux")]
fn authenticate_node_process(
    pid: NonZeroU32,
    endpoint: &StrictLoopbackWsUrl,
) -> Result<AuthenticatedNodeProcess, NodeEvidenceError> {
    let raw_pid = i32::try_from(pid.get()).map_err(|_| NodeEvidenceError::Executable)?;
    let rustix_pid = rustix::process::Pid::from_raw(raw_pid).ok_or(NodeEvidenceError::ZeroPid)?;

    let proc_path = PathBuf::from(format!("/proc/{pid}"));
    let proc_directory = File::open(&proc_path).map_err(|source| NodeEvidenceError::Io {
        operation: "open process evidence directory",
        source,
    })?;
    let proc_metadata = proc_directory
        .metadata()
        .map_err(|source| NodeEvidenceError::Io {
            operation: "inspect process evidence directory",
            source,
        })?;
    if !proc_metadata.is_dir()
        || rustix::fs::fstatfs(&proc_directory)
            .map_err(|source| NodeEvidenceError::Io {
                operation: "inspect process evidence filesystem",
                source: source.into(),
            })?
            .f_type
            != rustix::fs::PROC_SUPER_MAGIC
    {
        return Err(NodeEvidenceError::Executable);
    }
    let proc_identity = KernelFileIdentity::from_metadata(&proc_metadata);
    let pidfd = rustix::process::pidfd_open(rustix_pid, rustix::process::PidfdFlags::empty())
        .map_err(|source| NodeEvidenceError::Io {
            operation: "open stable process descriptor",
            source: source.into(),
        })?;
    require_same_proc_object(&proc_path, proc_identity)?;

    let proc_descriptor_path =
        PathBuf::from(format!("/proc/self/fd/{}", proc_directory.as_raw_fd()));
    let stat_path = proc_descriptor_path.join("stat");
    let start_time = read_process_start_time(&stat_path, pid)?;
    let executable_path = proc_descriptor_path.join("exe");
    let executable_link =
        fs::read_link(&executable_path).map_err(|source| NodeEvidenceError::Io {
            operation: "read archive node executable link",
            source,
        })?;
    let executable_kind = classify_executable_link(&executable_link)?;
    let executable = File::open(&executable_path).map_err(|source| NodeEvidenceError::Io {
        operation: "open archive node executable",
        source,
    })?;
    let executable_identity =
        KernelFileIdentity::from_metadata(&executable.metadata().map_err(|source| {
            NodeEvidenceError::Io {
                operation: "inspect archive node executable",
                source,
            }
        })?);
    require_same_file_object(&executable_path, executable_identity)?;

    let command_line = read_bounded_proc_file(
        &proc_descriptor_path.join("cmdline"),
        MAX_PROC_CMDLINE_BYTES,
    )?;
    verify_archive_cmdline(&command_line, endpoint.port())?;
    authenticate_executable(&executable, executable_kind, PINNED_OMNI_NODE)?;

    let after_executable_identity =
        KernelFileIdentity::from_metadata(&executable.metadata().map_err(|source| {
            NodeEvidenceError::Io {
                operation: "reinspect archive node executable",
                source,
            }
        })?);
    if after_executable_identity != executable_identity
        || read_process_start_time(&stat_path, pid)? != start_time
        || fs::read_link(&executable_path).map_err(|source| NodeEvidenceError::Io {
            operation: "reread archive node executable link",
            source,
        })? != executable_link
    {
        return Err(NodeEvidenceError::Executable);
    }
    require_same_file_object(&executable_path, executable_identity)?;
    require_same_proc_object(&proc_path, proc_identity)?;
    let after_proc_identity =
        KernelFileIdentity::from_metadata(&proc_directory.metadata().map_err(|source| {
            NodeEvidenceError::Io {
                operation: "reinspect process evidence directory",
                source,
            }
        })?);
    if !after_proc_identity.same_object(proc_identity) {
        return Err(NodeEvidenceError::Executable);
    }

    Ok(AuthenticatedNodeProcess {
        _pidfd: pidfd,
        _proc_directory: proc_directory,
        _executable: executable,
    })
}

#[cfg(target_os = "linux")]
fn require_same_proc_object(
    proc_path: &Path,
    expected: KernelFileIdentity,
) -> Result<(), NodeEvidenceError> {
    let actual = fs::metadata(proc_path)
        .map(|metadata| KernelFileIdentity::from_metadata(&metadata))
        .map_err(|source| NodeEvidenceError::Io {
            operation: "reinspect process identity",
            source,
        })?;
    if actual.same_object(expected) {
        Ok(())
    } else {
        Err(NodeEvidenceError::Executable)
    }
}

#[cfg(target_os = "linux")]
fn require_same_file_object(
    path: &Path,
    expected: KernelFileIdentity,
) -> Result<(), NodeEvidenceError> {
    let actual = fs::metadata(path)
        .map(|metadata| KernelFileIdentity::from_metadata(&metadata))
        .map_err(|source| NodeEvidenceError::Io {
            operation: "reinspect executable identity",
            source,
        })?;
    if actual == expected {
        Ok(())
    } else {
        Err(NodeEvidenceError::Executable)
    }
}

#[cfg(target_os = "linux")]
fn read_bounded_proc_file(path: &Path, limit: u64) -> Result<Vec<u8>, NodeEvidenceError> {
    let file = File::open(path).map_err(|source| NodeEvidenceError::Io {
        operation: "open process evidence",
        source,
    })?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| NodeEvidenceError::Io {
            operation: "read process evidence",
            source,
        })?;
    if bytes.len() as u64 > limit {
        Err(NodeEvidenceError::OverBound)
    } else {
        Ok(bytes)
    }
}

#[cfg(target_os = "linux")]
fn read_process_start_time(path: &Path, pid: NonZeroU32) -> Result<u64, NodeEvidenceError> {
    let bytes = match read_bounded_proc_file(path, MAX_PROC_STAT_BYTES) {
        Err(NodeEvidenceError::OverBound) => return Err(NodeEvidenceError::Executable),
        result => result?,
    };
    parse_process_start_time(&bytes, pid).ok_or(NodeEvidenceError::Executable)
}

#[cfg(target_os = "linux")]
fn parse_process_start_time(bytes: &[u8], expected_pid: NonZeroU32) -> Option<u64> {
    if bytes.is_empty() || bytes.contains(&0) {
        return None;
    }
    let open_parenthesis = bytes.windows(2).position(|pair| pair == b" (")?;
    if bytes[..open_parenthesis] != expected_pid.get().to_string().as_bytes()[..] {
        return None;
    }
    let close_parenthesis = bytes.iter().rposition(|byte| *byte == b')')?;
    if close_parenthesis <= open_parenthesis + 1 || bytes.get(close_parenthesis + 1) != Some(&b' ')
    {
        return None;
    }
    // The suffix starts at field 3 (`state`); starttime is Linux stat field 22.
    let start_time = bytes[close_parenthesis + 2..]
        .split(|byte| byte.is_ascii_whitespace())
        .filter(|field| !field.is_empty())
        .nth(19)?;
    std::str::from_utf8(start_time).ok()?.parse().ok()
}

#[cfg(target_os = "linux")]
fn classify_executable_link(path: &Path) -> Result<NodeExecutableKind, NodeEvidenceError> {
    let bytes = path.as_os_str().as_bytes();
    if bytes == SEALED_NODE_EXECUTABLE_LINK {
        return Ok(NodeExecutableKind::SealedMemfd);
    }
    if path.is_absolute()
        && path
            .file_name()
            .is_some_and(|name| name.as_bytes() == POLKADOT_OMNI_NODE_BASENAME)
    {
        Ok(NodeExecutableKind::Pathname)
    } else {
        Err(NodeEvidenceError::Executable)
    }
}

#[cfg(target_os = "linux")]
fn authenticate_executable(
    file: &File,
    kind: NodeExecutableKind,
    expected: ExpectedExecutable<'_>,
) -> Result<(), NodeEvidenceError> {
    let before = KernelFileIdentity::from_metadata(&file.metadata().map_err(|source| {
        NodeEvidenceError::Io {
            operation: "inspect executable bytes",
            source,
        }
    })?);
    if before.size != expected.size
        || !file
            .metadata()
            .map_err(|source| NodeEvidenceError::Io {
                operation: "inspect executable type",
                source,
            })?
            .is_file()
    {
        return Err(NodeEvidenceError::Executable);
    }
    if kind == NodeExecutableKind::SealedMemfd {
        let required = rustix::fs::SealFlags::SEAL
            | rustix::fs::SealFlags::SHRINK
            | rustix::fs::SealFlags::GROW
            | rustix::fs::SealFlags::WRITE;
        let actual =
            rustix::fs::fcntl_get_seals(file).map_err(|_| NodeEvidenceError::Executable)?;
        if actual != required {
            return Err(NodeEvidenceError::Executable);
        }
    }

    let mut reader = file;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|source| NodeEvidenceError::Io {
            operation: "seek executable bytes",
            source,
        })?;
    let mut digest = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|source| NodeEvidenceError::Io {
                operation: "hash executable bytes",
                source,
            })?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(u64::try_from(count).map_err(|_| NodeEvidenceError::Executable)?)
            .ok_or(NodeEvidenceError::Executable)?;
        if total > expected.size {
            return Err(NodeEvidenceError::Executable);
        }
        digest.update(&buffer[..count]);
    }
    let after = KernelFileIdentity::from_metadata(&file.metadata().map_err(|source| {
        NodeEvidenceError::Io {
            operation: "reinspect executable bytes",
            source,
        }
    })?);
    if total != expected.size || hex_lower(&digest.finalize()) != expected.sha256 || after != before
    {
        Err(NodeEvidenceError::Executable)
    } else {
        Ok(())
    }
}

pub(crate) fn verify_archive_cmdline(
    bytes: &[u8],
    expected_port: u16,
) -> Result<(), NodeEvidenceError> {
    if bytes.is_empty() || bytes.last() != Some(&0) {
        return Err(NodeEvidenceError::MalformedCmdline);
    }
    let arguments = bytes[..bytes.len() - 1]
        .split(|byte| *byte == 0)
        .collect::<Vec<_>>();
    if arguments.is_empty() || arguments.iter().any(|argument| argument.is_empty()) {
        return Err(NodeEvidenceError::MalformedCmdline);
    }
    let mut blocks = 0_u8;
    let mut state = 0_u8;
    let mut primary_rpc_endpoint = 0_u8;
    let mut relay_rpc_endpoints = 0_u8;
    let mut index = 1_usize;
    let mut after_separator = false;
    while index < arguments.len() {
        let argument = arguments[index];
        if argument == b"--" {
            if after_separator {
                return Err(NodeEvidenceError::MalformedCmdline);
            }
            after_separator = true;
            index += 1;
            continue;
        }
        if argument.starts_with(b"--blocks-pruning") {
            if after_separator
                || argument != b"--blocks-pruning"
                || arguments.get(index + 1).copied() != Some(b"archive")
            {
                return Err(NodeEvidenceError::ArchiveFlags);
            }
            blocks = blocks
                .checked_add(1)
                .ok_or(NodeEvidenceError::ArchiveFlags)?;
            index += 2;
            continue;
        }
        if argument.starts_with(b"--state-pruning") {
            if after_separator
                || argument != b"--state-pruning"
                || arguments.get(index + 1).copied() != Some(b"archive")
            {
                return Err(NodeEvidenceError::ArchiveFlags);
            }
            state = state
                .checked_add(1)
                .ok_or(NodeEvidenceError::ArchiveFlags)?;
            index += 2;
            continue;
        }
        if [
            b"--rpc-port".as_slice(),
            b"--ws-port".as_slice(),
            b"--rpc-cors".as_slice(),
            b"--rpc-methods".as_slice(),
            b"--rpc-external".as_slice(),
            b"--unsafe-rpc-external".as_slice(),
            b"--ws-external".as_slice(),
            b"--unsafe-ws-external".as_slice(),
        ]
        .iter()
        .any(|prefix| argument.starts_with(prefix))
        {
            return Err(NodeEvidenceError::RpcPort);
        }
        if argument.starts_with(b"--experimental-rpc-endpoint") {
            if argument != b"--experimental-rpc-endpoint" {
                return Err(NodeEvidenceError::RpcPort);
            }
            let value = arguments
                .get(index + 1)
                .copied()
                .ok_or(NodeEvidenceError::RpcPort)?;
            if after_separator {
                if !is_exact_loopback_rpc_endpoint(value, None) {
                    return Err(NodeEvidenceError::RpcPort);
                }
                relay_rpc_endpoints = relay_rpc_endpoints
                    .checked_add(1)
                    .ok_or(NodeEvidenceError::RpcPort)?;
            } else {
                if !is_exact_loopback_rpc_endpoint(value, Some(expected_port)) {
                    return Err(NodeEvidenceError::RpcPort);
                }
                primary_rpc_endpoint = primary_rpc_endpoint
                    .checked_add(1)
                    .ok_or(NodeEvidenceError::RpcPort)?;
            }
            index += 2;
            continue;
        }
        index += 1;
    }
    if blocks == 1 && state == 1 && primary_rpc_endpoint == 1 && relay_rpc_endpoints <= 1 {
        Ok(())
    } else {
        Err(NodeEvidenceError::ArchiveFlags)
    }
}

fn is_exact_loopback_rpc_endpoint(value: &[u8], expected_port: Option<u16>) -> bool {
    let Ok(value) = std::str::from_utf8(value) else {
        return false;
    };
    let Some(port) = value
        .strip_prefix("listen-addr=127.0.0.1:")
        .and_then(|value| value.strip_suffix(",methods=unsafe,cors=all"))
    else {
        return false;
    };
    port.parse::<u16>().ok().is_some_and(|parsed| {
        parsed != 0
            && parsed.to_string() == port
            && expected_port.is_none_or(|expected| parsed == expected)
    })
}

/// Failure to obtain exact archive-mode evidence from a local node process.
#[derive(Debug)]
pub enum NodeEvidenceError {
    UnsupportedPlatform,
    ZeroPid,
    Io {
        operation: &'static str,
        source: io::Error,
    },
    OverBound,
    MalformedCmdline,
    Executable,
    ArchiveFlags,
    RpcPort,
}

impl fmt::Display for NodeEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("archive node evidence requires Linux /proc")
            }
            Self::ZeroPid => formatter.write_str("archive node PID must be nonzero"),
            Self::Io { operation, source } => {
                write!(formatter, "could not {operation} for archive node evidence: {source}")
            }
            Self::OverBound => formatter.write_str("archive node command line exceeds 65536 bytes"),
            Self::MalformedCmdline => formatter.write_str("archive node command line is malformed"),
            Self::Executable => formatter.write_str(
                "archive node executable does not match the pinned polkadot-omni-node process identity",
            ),
            Self::ArchiveFlags => formatter.write_str(
                "archive node command line must contain exact unique archive pruning argv pairs before its separator",
            ),
            Self::RpcPort => formatter.write_str(
                "archive node command line must contain one exact IPv4-only primary RPC endpoint and no legacy/global RPC listener flags",
            ),
        }
    }
}

impl Error for NodeEvidenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Immutable identity authenticated from the pinned deployment artifacts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeploymentIdentity {
    namespace: &'static str,
    relay_genesis_hash: [u8; 32],
    parachain_genesis_hash: [u8; 32],
    para_id: u32,
    deployment_id: [u8; 32],
    pallet_storage_version: u16,
    event_schema_version: u16,
    runtime_spec_name: &'static str,
    runtime_impl_name: &'static str,
    runtime_authoring_version: u32,
    runtime_spec_version: u32,
    runtime_impl_version: u32,
    runtime_transaction_version: u32,
    runtime_state_version: u8,
    runtime_system_version: u8,
    runtime_code_hash: [u8; 32],
    runtime_apis: Vec<(String, u32)>,
}

impl DeploymentIdentity {
    pub(crate) fn load() -> Result<Self, IdentityError> {
        authenticate_artifact(
            "deployment anchor",
            DEPLOYMENT_ANCHOR_BYTES,
            DEPLOYMENT_ANCHOR_SIZE,
            DEPLOYMENT_ANCHOR_SHA256,
        )?;
        authenticate_artifact(
            "runtime metadata",
            METADATA_BYTES,
            METADATA_SIZE,
            METADATA_SHA256,
        )?;
        authenticate_artifact(
            "runtime Wasm",
            RUNTIME_WASM_BYTES,
            RUNTIME_WASM_SIZE,
            RUNTIME_WASM_SHA256,
        )?;

        let manifest: Value = serde_json::from_slice(DEPLOYMENT_ANCHOR_BYTES)
            .map_err(IdentityError::InvalidManifestJson)?;
        require_string(&manifest, &["format"], "cubikan-local-deployment-anchor-v1")?;
        require_string(&manifest, &["status"], "resolved")?;
        require_string(&manifest, &["namespace"], "polkadot-sdk-parachain")?;
        require_string(
            &manifest,
            &["artifacts", "metadata", "path"],
            "chain/metadata/cubikan-runtime-v1.scale",
        )?;
        require_string(
            &manifest,
            &["artifacts", "metadata", "provenance", "method"],
            "state_getMetadata",
        )?;
        require_string(
            &manifest,
            &["artifacts", "metadata", "provenance", "rpc_url"],
            "ws://127.0.0.1:9988/",
        )?;
        require_u64(&manifest, &["artifacts", "metadata", "size"], 63_327)?;
        require_string(
            &manifest,
            &["artifacts", "metadata", "sha256"],
            METADATA_SHA256,
        )?;
        require_string(
            &manifest,
            &["artifacts", "runtime_wasm", "path"],
            "chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm",
        )?;
        require_u64(&manifest, &["artifacts", "runtime_wasm", "size"], 637_930)?;
        require_string(
            &manifest,
            &["artifacts", "runtime_wasm", "sha256"],
            RUNTIME_WASM_SHA256,
        )?;
        require_u64(&manifest, &["relay_genesis", "block_number"], 0)?;
        require_string(
            &manifest,
            &["relay_genesis", "provenance", "method"],
            "chain_getBlockHash",
        )?;
        require_string(
            &manifest,
            &["relay_genesis", "provenance", "rpc_url"],
            "ws://127.0.0.1:9944/",
        )?;
        require_u64(&manifest, &["parachain_genesis", "block_number"], 0)?;
        require_string(
            &manifest,
            &["parachain_genesis", "provenance", "method"],
            "chain_getBlockHash",
        )?;
        require_string(
            &manifest,
            &["parachain_genesis", "provenance", "rpc_url"],
            "ws://127.0.0.1:9988/",
        )?;
        require_string(
            &manifest,
            &["runtime", "code", "provenance", "method"],
            "state_getStorage",
        )?;
        require_string(
            &manifest,
            &["runtime", "code", "provenance", "rpc_url"],
            "ws://127.0.0.1:9988/",
        )?;
        require_string(
            &manifest,
            &["runtime", "provenance", "method"],
            "state_getRuntimeVersion",
        )?;
        require_string(
            &manifest,
            &["runtime", "provenance", "rpc_url"],
            "ws://127.0.0.1:9988/",
        )?;
        for record in [
            "deployment_id",
            "event_schema_version",
            "pallet_storage_version",
            "para_id",
        ] {
            require_string(
                &manifest,
                &[
                    "deployment",
                    "state_records",
                    record,
                    "provenance",
                    "method",
                ],
                "state_getStorage",
            )?;
            require_string(
                &manifest,
                &[
                    "deployment",
                    "state_records",
                    record,
                    "provenance",
                    "rpc_url",
                ],
                "ws://127.0.0.1:9988/",
            )?;
        }
        require_string(&manifest, &["runtime", "spec_name"], "cubikan-runtime")?;
        require_string(&manifest, &["runtime", "impl_name"], "cubikan-runtime")?;

        let runtime_apis = value_at(&manifest, &["runtime", "apis"])
            .as_array()
            .ok_or(IdentityError::ManifestMismatch("runtime APIs"))?
            .iter()
            .map(|entry| {
                let pair = entry
                    .as_array()
                    .filter(|pair| pair.len() == 2)
                    .ok_or(IdentityError::ManifestMismatch("runtime API entry"))?;
                let id = pair[0]
                    .as_str()
                    .ok_or(IdentityError::ManifestMismatch("runtime API ID"))?
                    .to_owned();
                let version = pair[1]
                    .as_u64()
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or(IdentityError::ManifestMismatch("runtime API version"))?;
                Ok((id, version))
            })
            .collect::<Result<Vec<_>, IdentityError>>()?;

        Ok(Self {
            namespace: "polkadot-sdk-parachain",
            relay_genesis_hash: parse_hash(value_string(&manifest, &["relay_genesis", "hash"])?)?,
            parachain_genesis_hash: parse_hash(value_string(
                &manifest,
                &["parachain_genesis", "hash"],
            )?)?,
            para_id: value_u32(&manifest, &["deployment", "para_id"])?,
            deployment_id: parse_hash(value_string(&manifest, &["deployment", "deployment_id"])?)?,
            pallet_storage_version: value_u16(
                &manifest,
                &["deployment", "pallet_storage_version"],
            )?,
            event_schema_version: value_u16(&manifest, &["deployment", "event_schema_version"])?,
            runtime_spec_name: "cubikan-runtime",
            runtime_impl_name: "cubikan-runtime",
            runtime_authoring_version: value_u32(&manifest, &["runtime", "authoring_version"])?,
            runtime_spec_version: value_u32(&manifest, &["runtime", "spec_version"])?,
            runtime_impl_version: value_u32(&manifest, &["runtime", "impl_version"])?,
            runtime_transaction_version: value_u32(&manifest, &["runtime", "transaction_version"])?,
            runtime_state_version: value_u8(&manifest, &["runtime", "state_version"])?,
            runtime_system_version: value_u8(&manifest, &["runtime", "system_version"])?,
            runtime_code_hash: parse_hash(value_string(
                &manifest,
                &["runtime", "code", "blake2_256"],
            )?)?,
            runtime_apis,
        })
    }

    #[must_use]
    pub const fn namespace(&self) -> &'static str {
        self.namespace
    }

    #[must_use]
    pub const fn relay_genesis_hash(&self) -> &[u8; 32] {
        &self.relay_genesis_hash
    }

    #[must_use]
    pub const fn parachain_genesis_hash(&self) -> &[u8; 32] {
        &self.parachain_genesis_hash
    }

    #[must_use]
    pub const fn para_id(&self) -> u32 {
        self.para_id
    }

    #[must_use]
    pub const fn deployment_id(&self) -> &[u8; 32] {
        &self.deployment_id
    }

    #[must_use]
    pub const fn pallet_storage_version(&self) -> u16 {
        self.pallet_storage_version
    }

    #[must_use]
    pub const fn event_schema_version(&self) -> u16 {
        self.event_schema_version
    }

    #[must_use]
    pub const fn runtime_spec_name(&self) -> &'static str {
        self.runtime_spec_name
    }

    #[must_use]
    pub const fn runtime_spec_version(&self) -> u32 {
        self.runtime_spec_version
    }

    /// Returns the fixed native implementation name reported by the runtime.
    #[must_use]
    pub const fn runtime_impl_name(&self) -> &'static str {
        self.runtime_impl_name
    }

    /// Returns the fixed authoring compatibility version.
    #[must_use]
    pub const fn runtime_authoring_version(&self) -> u32 {
        self.runtime_authoring_version
    }

    /// Returns the fixed native implementation version.
    #[must_use]
    pub const fn runtime_impl_version(&self) -> u32 {
        self.runtime_impl_version
    }

    /// Returns the fixed transaction compatibility version.
    #[must_use]
    pub const fn runtime_transaction_version(&self) -> u32 {
        self.runtime_transaction_version
    }

    /// Returns the fixed state trie version.
    #[must_use]
    pub const fn runtime_state_version(&self) -> u8 {
        self.runtime_state_version
    }

    /// Returns the fixed system version.
    #[must_use]
    pub const fn runtime_system_version(&self) -> u8 {
        self.runtime_system_version
    }

    #[must_use]
    pub const fn runtime_code_hash(&self) -> &[u8; 32] {
        &self.runtime_code_hash
    }

    /// Returns the exact ordered runtime API identifiers and versions.
    #[must_use]
    pub fn runtime_apis(&self) -> &[(String, u32)] {
        &self.runtime_apis
    }
}

pub(crate) const fn metadata_bytes() -> &'static [u8] {
    METADATA_BYTES
}

pub(crate) const fn runtime_wasm_bytes() -> &'static [u8] {
    RUNTIME_WASM_BYTES
}

fn authenticate_artifact(
    label: &'static str,
    bytes: &[u8],
    size: usize,
    sha256: &str,
) -> Result<(), IdentityError> {
    if bytes.len() != size || hex_lower(&Sha256::digest(bytes)) != sha256 {
        Err(IdentityError::ArtifactMismatch(label))
    } else {
        Ok(())
    }
}

fn require_string(root: &Value, path: &[&str], expected: &str) -> Result<(), IdentityError> {
    if value_string(root, path)? == expected {
        Ok(())
    } else {
        Err(IdentityError::ManifestMismatch("string identity"))
    }
}

fn require_u64(root: &Value, path: &[&str], expected: u64) -> Result<(), IdentityError> {
    if value_at(root, path).as_u64() == Some(expected) {
        Ok(())
    } else {
        Err(IdentityError::ManifestMismatch("numeric identity"))
    }
}

fn value_at<'a>(root: &'a Value, path: &[&str]) -> &'a Value {
    let mut value = root;
    for key in path {
        value = &value[*key];
    }
    value
}

fn value_string<'a>(root: &'a Value, path: &[&str]) -> Result<&'a str, IdentityError> {
    value_at(root, path)
        .as_str()
        .ok_or(IdentityError::ManifestMismatch("string field"))
}

fn value_u32(root: &Value, path: &[&str]) -> Result<u32, IdentityError> {
    value_at(root, path)
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(IdentityError::ManifestMismatch("u32 field"))
}

fn value_u16(root: &Value, path: &[&str]) -> Result<u16, IdentityError> {
    value_at(root, path)
        .as_u64()
        .and_then(|value| u16::try_from(value).ok())
        .ok_or(IdentityError::ManifestMismatch("u16 field"))
}

fn value_u8(root: &Value, path: &[&str]) -> Result<u8, IdentityError> {
    value_at(root, path)
        .as_u64()
        .and_then(|value| u8::try_from(value).ok())
        .ok_or(IdentityError::ManifestMismatch("u8 field"))
}

fn parse_hash(input: &str) -> Result<[u8; 32], IdentityError> {
    let hex = input.strip_prefix("0x").unwrap_or(input);
    if hex.len() != 64 {
        return Err(IdentityError::ManifestMismatch("hash width"));
    }
    let mut output = [0_u8; 32];
    for (index, slot) in output.iter_mut().enumerate() {
        let offset = index * 2;
        *slot = u8::from_str_radix(&hex[offset..offset + 2], 16)
            .map_err(|_| IdentityError::ManifestMismatch("hash encoding"))?;
    }
    if hex_lower(&output) != hex {
        return Err(IdentityError::ManifestMismatch("hash canonical form"));
    }
    Ok(output)
}

fn hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

/// Failure to authenticate the immutable deployment artifact set.
#[derive(Debug)]
pub enum IdentityError {
    ArtifactMismatch(&'static str),
    InvalidManifestJson(serde_json::Error),
    ManifestMismatch(&'static str),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArtifactMismatch(label) => {
                write!(formatter, "pinned {label} size or SHA-256 does not match")
            }
            Self::InvalidManifestJson(error) => {
                write!(formatter, "deployment anchor JSON is invalid: {error}")
            }
            Self::ManifestMismatch(field) => {
                write!(
                    formatter,
                    "deployment anchor {field} does not match the fixed identity"
                )
            }
        }
    }
}

impl Error for IdentityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidManifestJson(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    use std::io::Write;

    fn preflight() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/finalized-events-v1/rpc-preflight-v1.json"
        ))
        .expect("independent preflight fixture should be JSON")
    }

    #[test]
    fn strict_url_matches_the_independent_fixture_inventory() {
        let fixture = preflight();
        for case in fixture["connection"]["url_cases"]
            .as_array()
            .expect("URL cases should be an array")
        {
            let input = case["input"].as_str().expect("URL input should be text");
            let expected = case["accepted"]
                .as_bool()
                .expect("URL result should be Boolean");
            let actual = StrictLoopbackWsUrl::parse(input);
            assert_eq!(actual.is_ok(), expected, "URL case {input}");
            if let Ok(url) = actual {
                assert_eq!(url.as_str(), input);
            }
        }
    }

    #[test]
    fn archive_flags_are_exact_unique_and_bounded() {
        assert!(verify_archive_cmdline(
            b"polkadot-omni-node\0--experimental-rpc-endpoint\0listen-addr=127.0.0.1:9988,methods=unsafe,cors=all\0--blocks-pruning\0archive\0--state-pruning\0archive\0",
            9988,
        )
        .is_ok());
        assert!(verify_archive_cmdline(
            b"polkadot-omni-node\0--experimental-rpc-endpoint\0listen-addr=127.0.0.1:9988,methods=unsafe,cors=all\0--blocks-pruning\0archive\0--state-pruning\0archive\0--\0--experimental-rpc-endpoint\0listen-addr=127.0.0.1:9990,methods=unsafe,cors=all\0",
            9988,
        )
        .is_ok());
        for bytes in [
            b"node\0--rpc-port\09988\0--blocks-pruning\0archive\0--state-pruning\0archive\0"
                .as_slice(),
            b"node\0--experimental-rpc-endpoint\0listen-addr=[::1]:9988,methods=unsafe,cors=all\0--blocks-pruning\0archive\0--state-pruning\0archive\0"
                .as_slice(),
            b"node\0--experimental-rpc-endpoint\0listen-addr=127.0.0.1:9989,methods=unsafe,cors=all\0--blocks-pruning\0archive\0--state-pruning\0archive\0"
                .as_slice(),
            b"node\0--experimental-rpc-endpoint=listen-addr=127.0.0.1:9988,methods=unsafe,cors=all\0--blocks-pruning\0archive\0--state-pruning\0archive\0"
                .as_slice(),
            b"node\0--experimental-rpc-endpoint\0listen-addr=127.0.0.1:9988,methods=unsafe,cors=all\0--rpc-cors\0all\0--blocks-pruning\0archive\0--state-pruning\0archive\0"
                .as_slice(),
            b"node\0--experimental-rpc-endpoint\0listen-addr=127.0.0.1:9988,methods=unsafe,cors=all\0--blocks-pruning=archive\0--state-pruning\0archive\0"
                .as_slice(),
            b"node\0--experimental-rpc-endpoint\0listen-addr=127.0.0.1:9988,methods=unsafe,cors=all\0--blocks-pruning\0archive\0"
                .as_slice(),
            b"node\0--experimental-rpc-endpoint\0listen-addr=127.0.0.1:9988,methods=unsafe,cors=all\0--\0--blocks-pruning\0archive\0--state-pruning\0archive\0"
                .as_slice(),
            b"node\0--experimental-rpc-endpoint\0listen-addr=127.0.0.1:9988,methods=unsafe,cors=all\0--blocks-pruning\0archive\0--state-pruning\0archive"
                .as_slice(),
        ] {
            assert!(verify_archive_cmdline(bytes, 9988).is_err());
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn executable_link_classification_is_closed() {
        assert_eq!(
            classify_executable_link(Path::new("/memfd:cubikan-sealed-exec-v1 (deleted)"))
                .expect("exact launcher memfd spelling should be accepted"),
            NodeExecutableKind::SealedMemfd
        );
        assert_eq!(
            classify_executable_link(Path::new("/reviewed/bin/polkadot-omni-node"))
                .expect("exact absolute pathname basename should be accepted"),
            NodeExecutableKind::Pathname
        );
        for rejected in [
            "polkadot-omni-node",
            "/reviewed/bin/polkadot-omni-node (deleted)",
            "/reviewed/bin/not-polkadot-omni-node",
            "memfd:cubikan-sealed-exec-v1 (deleted)",
            "/memfd:cubikan-sealed-exec-v1",
            "/memfd:cubikan-sealed-exec-v1 (deleted) ",
        ] {
            assert!(
                classify_executable_link(Path::new(rejected)).is_err(),
                "link spelling {rejected:?} must fail closed"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn process_start_time_parser_binds_pid_and_handles_parentheses() {
        let pid = NonZeroU32::new(42).expect("fixture PID is nonzero");
        let mut stat = String::from("42 (worker ) name) R");
        for value in 4..=21 {
            stat.push(' ');
            stat.push_str(&value.to_string());
        }
        stat.push_str(" 998877 23 24\n");
        assert_eq!(
            parse_process_start_time(stat.as_bytes(), pid),
            Some(998_877)
        );
        assert_eq!(
            parse_process_start_time(
                stat.as_bytes(),
                NonZeroU32::new(41).expect("fixture PID is nonzero")
            ),
            None
        );
        assert_eq!(parse_process_start_time(b"42 (worker) R 1 2", pid), None);
        assert_eq!(
            parse_process_start_time(b"42 (worker) R 1 2\0 3", pid),
            None
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn executable_authentication_requires_exact_bytes_and_complete_seals() {
        let bytes = b"\x7fELFsmall deterministic executable fixture";
        let digest = hex_lower(&Sha256::digest(bytes));
        let expected = ExpectedExecutable {
            size: u64::try_from(bytes.len()).expect("fixture length fits u64"),
            sha256: &digest,
        };

        let sealed = fixture_memfd(bytes, true);
        authenticate_executable(&sealed, NodeExecutableKind::SealedMemfd, expected)
            .expect("exact sealed bytes must authenticate");

        let unsealed = fixture_memfd(bytes, false);
        assert!(
            authenticate_executable(&unsealed, NodeExecutableKind::SealedMemfd, expected).is_err()
        );
        authenticate_executable(&unsealed, NodeExecutableKind::Pathname, expected)
            .expect("an exact regular pathname executable does not require memfd seals");

        let incomplete = fixture_memfd(bytes, false);
        rustix::fs::fcntl_add_seals(
            &incomplete,
            rustix::fs::SealFlags::SHRINK
                | rustix::fs::SealFlags::GROW
                | rustix::fs::SealFlags::WRITE,
        )
        .expect("fixture memfd should accept an incomplete seal set");
        assert!(
            authenticate_executable(&incomplete, NodeExecutableKind::SealedMemfd, expected)
                .is_err()
        );

        let wrong_digest = ExpectedExecutable {
            size: expected.size,
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        };
        assert!(
            authenticate_executable(&sealed, NodeExecutableKind::SealedMemfd, wrong_digest)
                .is_err()
        );
        let wrong_size = ExpectedExecutable {
            size: expected.size + 1,
            sha256: expected.sha256,
        };
        assert!(
            authenticate_executable(&sealed, NodeExecutableKind::SealedMemfd, wrong_size).is_err()
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn production_executable_identity_matches_the_pin_manifest() {
        let pins = include_str!("../../../chain/pins.toml");
        let section = pins
            .split_once("[assets.polkadot-omni-node]")
            .expect("pin manifest should contain the omni-node table")
            .1
            .split("\n[")
            .next()
            .expect("omni-node table should have a bounded section");
        assert!(section.contains(&format!("size = \"{}\"", PINNED_OMNI_NODE.size)));
        assert!(section.contains(&format!("sha256 = \"{}\"", PINNED_OMNI_NODE.sha256)));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn unrelated_current_process_cannot_mint_node_evidence() {
        let endpoint = StrictLoopbackWsUrl::parse("ws://127.0.0.1:9988/")
            .expect("fixture endpoint is canonical");
        assert!(matches!(
            ArchiveNodeEvidence::from_proc_pid(std::process::id(), &endpoint),
            Err(NodeEvidenceError::Executable)
        ));
    }

    #[cfg(target_os = "linux")]
    fn fixture_memfd(bytes: &[u8], seal: bool) -> File {
        let descriptor = rustix::fs::memfd_create(
            "cubikan-sealed-exec-v1",
            rustix::fs::MemfdFlags::ALLOW_SEALING,
        )
        .expect("test kernel should support sealable memfds");
        let mut file = File::from(descriptor);
        file.write_all(bytes).expect("fixture bytes should write");
        if seal {
            rustix::fs::fcntl_add_seals(
                &file,
                rustix::fs::SealFlags::SEAL
                    | rustix::fs::SealFlags::SHRINK
                    | rustix::fs::SealFlags::GROW
                    | rustix::fs::SealFlags::WRITE,
            )
            .expect("fixture memfd should accept the complete seal set");
        }
        file
    }

    #[test]
    fn embedded_identity_authenticates_exact_artifacts() {
        let identity = DeploymentIdentity::load().expect("pinned artifacts should authenticate");
        assert_eq!(identity.namespace(), "polkadot-sdk-parachain");
        assert_eq!(identity.para_id(), 1000);
        assert_eq!(identity.runtime_spec_version(), 1);
        assert_eq!(identity.pallet_storage_version(), 1);
        assert_eq!(identity.event_schema_version(), 1);
        let fixture = preflight();
        let runtime = &fixture["identity"]["runtime"];
        assert_eq!(identity.runtime_impl_name(), runtime["impl_name"]);
        assert_eq!(
            u64::from(identity.runtime_authoring_version()),
            runtime["authoring_version"]
        );
        assert_eq!(
            u64::from(identity.runtime_impl_version()),
            runtime["impl_version"]
        );
        assert_eq!(
            u64::from(identity.runtime_transaction_version()),
            runtime["transaction_version"]
        );
        assert_eq!(
            u64::from(identity.runtime_state_version()),
            runtime["state_version"]
        );
        assert_eq!(
            u64::from(identity.runtime_system_version()),
            runtime["system_version"]
        );
        let expected_apis = runtime["apis"].as_array().expect("runtime APIs");
        assert_eq!(identity.runtime_apis().len(), expected_apis.len());
        for ((id, version), expected) in identity.runtime_apis().iter().zip(expected_apis) {
            assert_eq!(
                id.strip_prefix("0x").unwrap_or(id),
                expected["id"].as_str().expect("runtime API ID")
            );
            assert_eq!(u64::from(*version), expected["version"]);
        }
    }
}
