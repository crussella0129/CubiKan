use std::{
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, Read},
    num::NonZeroU32,
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
    "38f795fb3bbb666f571b3bd1e4fa3ad1666476f3fff20dee9d93feb9c925dee7";
const METADATA_SIZE: usize = 63_327;
const METADATA_SHA256: &str = "171a323b1e6bf0122e549eecd5f5932e672a3e0835f32edf0b8808cfefd97302";
const RUNTIME_WASM_SIZE: usize = 637_930;
const RUNTIME_WASM_SHA256: &str =
    "640cc616674fe7393fc93928904f0fd92d77571209c8200f08b8da6290c6a275";
const MAX_PROC_CMDLINE_BYTES: u64 = 65_536;

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
/// This authenticates the executable basename, bounded `/proc/<pid>/cmdline`
/// bytes, exact pruning pairs, and the primary `--rpc-port` value. Listener
/// ownership is additionally checked by the Sprint 11 network harness.
#[derive(Debug)]
pub struct ArchiveNodeEvidence {
    pid: NonZeroU32,
    endpoint: StrictLoopbackWsUrl,
}

impl ArchiveNodeEvidence {
    /// Reads a Linux process command line once and requires each exact archive
    /// flag exactly once, with no contradictory spelling.
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
            let executable = fs::read_link(format!("/proc/{pid}/exe")).map_err(|source| {
                NodeEvidenceError::Io {
                    operation: "read archive node executable link",
                    source,
                }
            })?;
            if executable.file_name().and_then(|name| name.to_str()) != Some("polkadot-omni-node") {
                return Err(NodeEvidenceError::Executable);
            }
            let path = format!("/proc/{pid}/cmdline");
            let file = File::open(&path).map_err(|source| NodeEvidenceError::Io {
                operation: "open",
                source,
            })?;
            let mut bytes = Vec::new();
            file.take(MAX_PROC_CMDLINE_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|source| NodeEvidenceError::Io {
                    operation: "read",
                    source,
                })?;
            if bytes.len() as u64 > MAX_PROC_CMDLINE_BYTES {
                return Err(NodeEvidenceError::OverBound);
            }
            verify_archive_cmdline(&bytes, endpoint.port())?;
            Ok(Self {
                pid,
                endpoint: endpoint.clone(),
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
    let mut rpc_port = 0_u8;
    let expected_port = expected_port.to_string();
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
        if argument.starts_with(b"--rpc-port") || argument.starts_with(b"--ws-port") {
            if after_separator
                || argument != b"--rpc-port"
                || arguments.get(index + 1).copied() != Some(expected_port.as_bytes())
            {
                return Err(NodeEvidenceError::RpcPort);
            }
            rpc_port = rpc_port.checked_add(1).ok_or(NodeEvidenceError::RpcPort)?;
            index += 2;
            continue;
        }
        index += 1;
    }
    if blocks == 1 && state == 1 && rpc_port == 1 {
        Ok(())
    } else {
        Err(NodeEvidenceError::ArchiveFlags)
    }
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
                write!(formatter, "could not {operation} archive node command line: {source}")
            }
            Self::OverBound => formatter.write_str("archive node command line exceeds 65536 bytes"),
            Self::MalformedCmdline => formatter.write_str("archive node command line is malformed"),
            Self::Executable => {
                formatter.write_str("archive node executable must be polkadot-omni-node")
            }
            Self::ArchiveFlags => formatter.write_str(
                "archive node command line must contain exact unique archive pruning argv pairs before its separator",
            ),
            Self::RpcPort => formatter.write_str(
                "archive node command line must contain one primary --rpc-port matching the endpoint",
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
            b"polkadot-omni-node\0--rpc-port\09988\0--blocks-pruning\0archive\0--state-pruning\0archive\0",
            9988,
        )
        .is_ok());
        for bytes in [
            b"node\0--rpc-port\09988\0--blocks-pruning=archive\0--state-pruning\0archive\0"
                .as_slice(),
            b"node\0--rpc-port\09988\0--blocks-pruning\0archive\0".as_slice(),
            b"node\0--rpc-port\09989\0--blocks-pruning\0archive\0--state-pruning\0archive\0"
                .as_slice(),
            b"node\0--rpc-port\09988\0--\0--blocks-pruning\0archive\0--state-pruning\0archive\0"
                .as_slice(),
            b"node\0--rpc-port\09988\0--blocks-pruning\0archive\0--state-pruning\0archive"
                .as_slice(),
        ] {
            assert!(verify_archive_cmdline(bytes, 9988).is_err());
        }
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
