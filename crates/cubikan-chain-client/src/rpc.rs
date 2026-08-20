use std::{
    error::Error,
    fmt,
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use parity_scale_codec::{Compact, Decode, Encode};
use serde_json::Value;
use subxt::{
    Metadata, OfflineClient,
    config::{
        Hasher, RpcConfigFor,
        substrate::{H256, SpecVersionForRange, SubstrateConfig, SubstrateHeader},
    },
    events::Phase,
    rpcs::{LegacyRpcMethods, RpcClient},
};
use tokio::time::{self, error::Elapsed};

use crate::{
    AcceptedEvent, ArchiveNodeEvidence, DeploymentIdentity, IdentityError, NodeEvidenceError,
    PayloadDecodeError, StrictLoopbackWsUrl, decode_canonical_payload,
    identity::{metadata_bytes, runtime_wasm_bytes},
};

type RpcConfiguration = RpcConfigFor<SubstrateConfig>;
type RpcHeader = SubstrateHeader<H256>;
type BoxError = Box<dyn Error + Send + Sync + 'static>;
type RpcFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, BoxError>> + Send + 'a>>;

const RPC_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_SYSTEM_EVENTS_BYTES: usize = 16 * 1_048_576;
const MAX_SYSTEM_EVENT_RECORDS: u32 = 65_536;
const MAX_BLOCK_EXTRINSICS: usize = 65_536;
const MAX_BLOCK_BODY_BYTES: usize = 16 * 1_048_576;
const SYSTEM_EVENTS_KEY_HEX: &str =
    "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
const DEPLOYMENT_ID_KEY_HEX: &str =
    "2609aef1a1450f1b658394d12c417d249cc1dff5d6cf049a569219f2724d2e09";
const EVENT_SCHEMA_VERSION_KEY_HEX: &str =
    "2609aef1a1450f1b658394d12c417d24be4eb7443b9de1b5f041137c841f02ac";
const PALLET_STORAGE_VERSION_KEY_HEX: &str =
    "2609aef1a1450f1b658394d12c417d241601562ebcdff856cb2f34e65f3b2659";
const PARA_ID_KEY_HEX: &str = "0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f";
const RUNTIME_CODE_KEY_HEX: &str = "3a636f6465";
const RELAY_RPC_URL: &str = "ws://127.0.0.1:9944/";

/// A node-asserted finalized head returned by one verified archive client.
///
/// Fields are private so callers cannot manufacture a finalized bound for
/// [`VerifiedArchiveClient::finalized_block`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedHead {
    number: u64,
    hash: [u8; 32],
    parachain_genesis_hash: [u8; 32],
    endpoint: String,
}

impl FinalizedHead {
    #[must_use]
    pub const fn number(&self) -> u64 {
        self.number
    }

    #[must_use]
    pub const fn hash(&self) -> &[u8; 32] {
        &self.hash
    }
}

/// One complete canonical block decoded under a verified finalized-head bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedBlock {
    number: u64,
    hash: [u8; 32],
    parent_hash: [u8; 32],
    runtime_spec_version: u32,
    runtime_code_hash: [u8; 32],
    raw_extrinsics: Vec<Vec<u8>>,
    raw_system_events: Vec<u8>,
    extrinsic_hashes: Vec<[u8; 32]>,
    system_event_record_count: u32,
    events: Vec<AcceptedEvent>,
}

impl FinalizedBlock {
    #[must_use]
    pub const fn number(&self) -> u64 {
        self.number
    }

    #[must_use]
    pub const fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    #[must_use]
    pub const fn parent_hash(&self) -> &[u8; 32] {
        &self.parent_hash
    }

    #[must_use]
    pub const fn runtime_spec_version(&self) -> u32 {
        self.runtime_spec_version
    }

    #[must_use]
    pub const fn runtime_code_hash(&self) -> &[u8; 32] {
        &self.runtime_code_hash
    }

    /// Returns every body extrinsic hash in exact body index order.
    #[must_use]
    pub fn extrinsic_hashes(&self) -> &[[u8; 32]] {
        &self.extrinsic_hashes
    }

    /// Returns the total number of System event records, including non-CubiKan events.
    #[must_use]
    pub const fn system_event_record_count(&self) -> u32 {
        self.system_event_record_count
    }

    /// Returns accepted CubiKan events in System event order.
    #[must_use]
    pub fn events(&self) -> &[AcceptedEvent] {
        &self.events
    }

    #[allow(dead_code)] // Reserved for the crate-private T-1111 reconciliation lane.
    pub(crate) fn raw_extrinsics(&self) -> &[Vec<u8>] {
        &self.raw_extrinsics
    }

    #[allow(dead_code)] // Reserved for the crate-private T-1111 reconciliation lane.
    pub(crate) fn raw_system_events(&self) -> &[u8] {
        &self.raw_system_events
    }
}

/// Archive-RPC client sealed by fixed artifacts, node argv evidence, and live probes.
pub struct VerifiedArchiveClient {
    endpoint: StrictLoopbackWsUrl,
    identity: DeploymentIdentity,
    rpc: Arc<dyn ArchiveRpc>,
    relay_rpc: Arc<dyn ArchiveRpc>,
    decoder: OfflineClient<SubstrateConfig>,
    decode_spy: Option<Arc<AtomicUsize>>,
}

impl fmt::Debug for VerifiedArchiveClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedArchiveClient")
            .field("endpoint", &self.endpoint)
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}

impl VerifiedArchiveClient {
    /// Connects only after process evidence is bound to the same strict endpoint,
    /// then authenticates the full fixed identity and historical archive probes.
    pub async fn connect(
        endpoint: StrictLoopbackWsUrl,
        evidence: ArchiveNodeEvidence,
    ) -> Result<Self, ArchiveError> {
        if evidence.endpoint() != &endpoint {
            return Err(ArchiveError::NodeEvidenceEndpointMismatch);
        }
        let identity = DeploymentIdentity::load().map_err(ArchiveError::Identity)?;
        ArchiveNodeEvidence::from_proc_pid(evidence.pid(), &endpoint)
            .map_err(ArchiveError::NodeEvidence)?;
        let rpc_client = connect_rpc("connect parachain archive RPC", endpoint.as_str()).await?;
        let relay_endpoint = StrictLoopbackWsUrl::parse(RELAY_RPC_URL)
            .map_err(|_| ArchiveError::MalformedResponse("fixed relay RPC URL"))?;
        let relay_client =
            connect_rpc("connect relay genesis RPC", relay_endpoint.as_str()).await?;
        let rpc = Arc::new(SubxtArchiveRpc::new(rpc_client));
        let relay_rpc = Arc::new(SubxtArchiveRpc::new(relay_client));
        Self::connect_with_rpc(endpoint, identity, rpc, relay_rpc).await
    }

    /// Returns the immutable deployment identity authenticated at connection.
    #[must_use]
    pub const fn identity(&self) -> &DeploymentIdentity {
        &self.identity
    }

    #[allow(dead_code)] // Reserved for the crate-private T-1111 online-client lane.
    pub(crate) fn endpoint(&self) -> &StrictLoopbackWsUrl {
        &self.endpoint
    }

    /// Returns a fresh node-asserted finalized head after rechecking its header.
    pub async fn finalized_head(&self) -> Result<FinalizedHead, ArchiveError> {
        let hash = self
            .call("chain_getFinalizedHead", self.rpc.finalized_head())
            .await?;
        let header = self
            .call("chain_getHeader(finalized)", self.rpc.header(hash))
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "finalized head header",
                number: None,
            })?;
        let canonical = self
            .call(
                "chain_getBlockHash(finalized-number)",
                self.rpc.block_hash(header.number),
            )
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "finalized head hash by number",
                number: Some(header.number),
            })?;
        if canonical != hash {
            return Err(ArchiveError::DisplacedFinalizedHead);
        }
        Ok(FinalizedHead {
            number: header.number,
            hash,
            parachain_genesis_hash: *self.identity.parachain_genesis_hash(),
            endpoint: self.endpoint.as_str().to_owned(),
        })
    }

    /// Fetches one canonical block at or below a client-issued finalized head.
    ///
    /// The call rechecks the head coordinate, canonical number/hash mapping,
    /// parent, body, System events, complete runtime identity, and exact code
    /// bytes before returning a closed block value.
    pub async fn finalized_block(
        &self,
        head: &FinalizedHead,
        number: u64,
    ) -> Result<FinalizedBlock, ArchiveError> {
        self.validate_head_token(head, number).await?;
        let raw = self.fetch_raw_block(number).await?;
        let (system_event_record_count, events) =
            self.decode_events(number, &raw.extrinsics, raw.event_bytes.clone())?;
        Ok(FinalizedBlock {
            number,
            hash: raw.hash,
            parent_hash: raw.parent_hash,
            runtime_spec_version: raw.runtime_spec_version,
            runtime_code_hash: *self.identity.runtime_code_hash(),
            raw_extrinsics: raw.extrinsics,
            raw_system_events: raw.event_bytes,
            extrinsic_hashes: raw.extrinsic_hashes,
            system_event_record_count,
            events,
        })
    }

    async fn connect_with_rpc(
        endpoint: StrictLoopbackWsUrl,
        identity: DeploymentIdentity,
        rpc: Arc<dyn ArchiveRpc>,
        relay_rpc: Arc<dyn ArchiveRpc>,
    ) -> Result<Self, ArchiveError> {
        Self::connect_with_rpc_inner(endpoint, identity, rpc, relay_rpc, None).await
    }

    #[cfg(test)]
    async fn connect_with_rpc_spied(
        endpoint: StrictLoopbackWsUrl,
        identity: DeploymentIdentity,
        rpc: Arc<dyn ArchiveRpc>,
        relay_rpc: Arc<dyn ArchiveRpc>,
        decode_spy: Arc<AtomicUsize>,
    ) -> Result<Self, ArchiveError> {
        Self::connect_with_rpc_inner(endpoint, identity, rpc, relay_rpc, Some(decode_spy)).await
    }

    async fn connect_with_rpc_inner(
        endpoint: StrictLoopbackWsUrl,
        identity: DeploymentIdentity,
        rpc: Arc<dyn ArchiveRpc>,
        relay_rpc: Arc<dyn ArchiveRpc>,
        decode_spy: Option<Arc<AtomicUsize>>,
    ) -> Result<Self, ArchiveError> {
        let metadata = Metadata::decode_from(metadata_bytes())
            .map_err(|_| ArchiveError::MalformedResponse("pinned runtime metadata"))?;
        let config = SubstrateConfig::builder()
            .set_genesis_hash(H256::from(*identity.parachain_genesis_hash()))
            .set_spec_version_for_block_ranges([SpecVersionForRange {
                block_range: 0..u64::MAX,
                spec_version: identity.runtime_spec_version(),
                transaction_version: identity.runtime_transaction_version(),
            }])
            .set_metadata_for_spec_versions([(identity.runtime_spec_version(), Arc::new(metadata))])
            .build();
        let client = Self {
            endpoint,
            identity,
            rpc,
            relay_rpc,
            decoder: OfflineClient::new_with_config(config),
            decode_spy,
        };
        if client.hash_bytes(0, runtime_wasm_bytes())? != *client.identity.runtime_code_hash() {
            return Err(ArchiveError::Identity(IdentityError::ArtifactMismatch(
                "runtime Wasm BLAKE2-256",
            )));
        }
        client.preflight().await?;
        Ok(client)
    }

    async fn preflight(&self) -> Result<(), ArchiveError> {
        let head = self.finalized_head().await?;
        let relay_genesis_hash = self
            .call("relay chain_getBlockHash(0)", self.relay_rpc.block_hash(0))
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "relay genesis hash",
                number: Some(0),
            })?;
        if relay_genesis_hash != *self.identity.relay_genesis_hash() {
            return Err(ArchiveError::DeploymentMismatch);
        }
        let genesis_hash = self
            .call("chain_getBlockHash(0)", self.rpc.block_hash(0))
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "genesis hash",
                number: Some(0),
            })?;
        if genesis_hash != *self.identity.parachain_genesis_hash() {
            return Err(ArchiveError::DeploymentMismatch);
        }

        self.verify_genesis_identity(genesis_hash).await?;
        for number in historical_probe_numbers(head.number) {
            self.probe_historical_block(number).await?;
        }
        Ok(())
    }

    async fn verify_genesis_identity(&self, genesis_hash: [u8; 32]) -> Result<(), ArchiveError> {
        let probes = [
            (
                DEPLOYMENT_ID_KEY_HEX,
                self.identity.deployment_id().to_vec(),
                "deployment ID",
            ),
            (
                EVENT_SCHEMA_VERSION_KEY_HEX,
                self.identity.event_schema_version().to_le_bytes().to_vec(),
                "event schema version",
            ),
            (
                PALLET_STORAGE_VERSION_KEY_HEX,
                self.identity
                    .pallet_storage_version()
                    .to_le_bytes()
                    .to_vec(),
                "pallet storage version",
            ),
            (
                PARA_ID_KEY_HEX,
                self.identity.para_id().to_le_bytes().to_vec(),
                "ParaId",
            ),
        ];
        for (key, expected, operation) in probes {
            let actual = self
                .call(operation, self.rpc.storage(fixed_hex(key), genesis_hash))
                .await?
                .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                    operation,
                    number: Some(0),
                })?;
            if actual != expected {
                return Err(ArchiveError::DeploymentMismatch);
            }
        }
        let metadata = self
            .call(
                "state_getMetadata(genesis)",
                self.rpc.metadata(genesis_hash),
            )
            .await?;
        if metadata != metadata_bytes() {
            return Err(ArchiveError::RuntimeMismatch);
        }
        let version = self
            .call(
                "state_getRuntimeVersion(genesis)",
                self.rpc.runtime_version(genesis_hash),
            )
            .await?;
        self.verify_runtime_version(&version)?;
        let code = self
            .call(
                "state_getStorage(:code,genesis)",
                self.rpc
                    .storage(fixed_hex(RUNTIME_CODE_KEY_HEX), genesis_hash),
            )
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "genesis runtime code",
                number: Some(0),
            })?;
        self.verify_runtime_code(0, &code)
    }

    async fn probe_historical_block(&self, number: u64) -> Result<(), ArchiveError> {
        let hash = self
            .call("chain_getBlockHash(probe)", self.rpc.block_hash(number))
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "historical block hash",
                number: Some(number),
            })?;
        let header = self
            .call("chain_getHeader(probe)", self.rpc.header(hash))
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "historical block header",
                number: Some(number),
            })?;
        let block = self
            .call("chain_getBlock(probe)", self.rpc.block(hash))
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "historical block body",
                number: Some(number),
            })?;
        if block.header != header || header.number != number {
            return Err(ArchiveError::NonContiguousFinalizedStream);
        }
        if number == 0 {
            if block.header.parent_hash != [0; 32] {
                return Err(ArchiveError::NonContiguousFinalizedStream);
            }
        } else {
            let previous = self
                .call(
                    "chain_getBlockHash(probe-parent)",
                    self.rpc.block_hash(number - 1),
                )
                .await?
                .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                    operation: "historical parent hash",
                    number: Some(number - 1),
                })?;
            if block.header.parent_hash != previous {
                return Err(ArchiveError::NonContiguousFinalizedStream);
            }
        }
        validate_body_bounds(&block.extrinsics)?;
        let event_bytes = canonical_event_storage(
            number,
            self.call(
                "state_getStorage(System::Events,probe)",
                self.rpc.storage(fixed_hex(SYSTEM_EVENTS_KEY_HEX), hash),
            )
            .await?,
            "historical System::Events",
        )?;
        validate_event_envelope(&event_bytes)?;
        let version = self
            .call(
                "state_getRuntimeVersion(probe)",
                self.rpc.runtime_version(hash),
            )
            .await?;
        self.verify_runtime_version(&version)?;
        let code = self
            .call(
                "state_getStorage(:code,probe)",
                self.rpc.storage(fixed_hex(RUNTIME_CODE_KEY_HEX), hash),
            )
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "historical runtime code",
                number: Some(number),
            })?;
        self.verify_runtime_code(number, &code)
    }

    async fn validate_head_token(
        &self,
        head: &FinalizedHead,
        number: u64,
    ) -> Result<(), ArchiveError> {
        if head.endpoint != self.endpoint.as_str()
            || head.parachain_genesis_hash != *self.identity.parachain_genesis_hash()
        {
            return Err(ArchiveError::ForeignFinalizedHead);
        }
        if number > head.number {
            return Err(ArchiveError::NotFinalized {
                requested: number,
                finalized: head.number,
            });
        }
        let canonical = self
            .call(
                "chain_getBlockHash(bound-head)",
                self.rpc.block_hash(head.number),
            )
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "bound finalized head",
                number: Some(head.number),
            })?;
        if canonical != head.hash {
            return Err(ArchiveError::DisplacedFinalizedHead);
        }
        Ok(())
    }

    async fn fetch_raw_block(&self, number: u64) -> Result<RawVerifiedBlock, ArchiveError> {
        let hash = self
            .call("chain_getBlockHash", self.rpc.block_hash(number))
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "finalized block hash",
                number: Some(number),
            })?;
        if number == 0 && hash != *self.identity.parachain_genesis_hash() {
            return Err(ArchiveError::DeploymentMismatch);
        }
        let header = self
            .call("chain_getHeader", self.rpc.header(hash))
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "finalized block header",
                number: Some(number),
            })?;
        let block = self
            .call("chain_getBlock", self.rpc.block(hash))
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "finalized block body",
                number: Some(number),
            })?;
        if block.header != header || header.number != number {
            return Err(ArchiveError::NonContiguousFinalizedStream);
        }
        let expected_parent = if number == 0 {
            [0; 32]
        } else {
            self.call(
                "chain_getBlockHash(parent)",
                self.rpc.block_hash(number - 1),
            )
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "finalized parent hash",
                number: Some(number - 1),
            })?
        };
        if block.header.parent_hash != expected_parent {
            return Err(ArchiveError::NonContiguousFinalizedStream);
        }
        validate_body_bounds(&block.extrinsics)?;
        let extrinsic_hashes = block
            .extrinsics
            .iter()
            .map(|extrinsic| self.hash_bytes(number, extrinsic))
            .collect::<Result<Vec<_>, _>>()?;
        let event_bytes = canonical_event_storage(
            number,
            self.call(
                "state_getStorage(System::Events)",
                self.rpc.storage(fixed_hex(SYSTEM_EVENTS_KEY_HEX), hash),
            )
            .await?,
            "finalized System::Events",
        )?;
        let version = self
            .call("state_getRuntimeVersion", self.rpc.runtime_version(hash))
            .await?;
        self.verify_runtime_version(&version)?;
        let code = self
            .call(
                "state_getStorage(:code)",
                self.rpc.storage(fixed_hex(RUNTIME_CODE_KEY_HEX), hash),
            )
            .await?
            .ok_or(ArchiveError::ArchiveHistoryUnavailable {
                operation: "finalized runtime code",
                number: Some(number),
            })?;
        self.verify_runtime_code(number, &code)?;
        Ok(RawVerifiedBlock {
            hash,
            parent_hash: block.header.parent_hash,
            runtime_spec_version: version.spec_version,
            extrinsics: block.extrinsics,
            extrinsic_hashes,
            event_bytes,
        })
    }

    fn decode_events(
        &self,
        block_number: u64,
        extrinsics: &[Vec<u8>],
        event_bytes: Vec<u8>,
    ) -> Result<(u32, Vec<AcceptedEvent>), ArchiveError> {
        if let Some(spy) = &self.decode_spy {
            spy.fetch_add(1, Ordering::SeqCst);
        }
        let event_count = validate_event_envelope(&event_bytes)?;
        let at = self
            .decoder
            .at_block(block_number)
            .map_err(|_| ArchiveError::MalformedResponse("offline event decoder block"))?;
        let decoded = at.events().from_bytes(event_bytes);
        if decoded.len() != event_count {
            return Err(ArchiveError::MalformedResponse("System::Events count"));
        }

        let prefix_length = compact_prefix_length(decoded.bytes())?;
        let mut consumed = prefix_length;
        let mut accepted = Vec::new();
        let mut last_extrinsic_index = None;
        let mut iterated = 0_u32;
        for result in decoded.iter() {
            let event = result.map_err(|source| ArchiveError::EventDecode {
                source: Box::new(source),
            })?;
            if event.index() != iterated {
                return Err(ArchiveError::MalformedResponse("System event index"));
            }
            iterated = iterated
                .checked_add(1)
                .ok_or(ArchiveError::OverBound("System event count"))?;
            consumed = consumed
                .checked_add(event.bytes().len())
                .ok_or(ArchiveError::OverBound("System event bytes"))?;

            if event.pallet_name() != "Cubikan" || event.event_name() != "Accepted" {
                continue;
            }
            if !event.topics().is_empty() {
                return Err(ArchiveError::MalformedResponse(
                    "accepted event topics are not empty",
                ));
            }
            let extrinsic_index = match event.phase() {
                Phase::ApplyExtrinsic(index) => index,
                Phase::Initialization | Phase::Finalization => {
                    return Err(ArchiveError::MalformedResponse(
                        "accepted event phase is not ApplyExtrinsic",
                    ));
                }
            };
            if last_extrinsic_index.is_some_and(|previous| extrinsic_index < previous) {
                return Err(ArchiveError::MalformedResponse("accepted event body order"));
            }
            last_extrinsic_index = Some(extrinsic_index);
            let body_index = usize::try_from(extrinsic_index)
                .map_err(|_| ArchiveError::OverBound("extrinsic index"))?;
            let extrinsic = extrinsics
                .get(body_index)
                .ok_or(ArchiveError::MalformedResponse("accepted event body join"))?;
            let extrinsic_hash = self.hash_bytes(block_number, extrinsic)?;
            let mut fields = FieldCursor::new(event.field_bytes());
            let deployment_id = fields.array()?;
            let event_schema_version = fields.u16()?;
            let global_sequence = fields.u64()?;
            let signer = fields.array()?;
            let raw_payload = fields.rest();
            if deployment_id != *self.identity.deployment_id() {
                return Err(ArchiveError::DeploymentMismatch);
            }
            if event_schema_version != self.identity.event_schema_version() || global_sequence == 0
            {
                return Err(ArchiveError::MalformedResponse(
                    "accepted event schema or sequence",
                ));
            }
            let payload = decode_canonical_payload(raw_payload).map_err(map_payload_error)?;
            accepted.push(AcceptedEvent::new(
                extrinsic_index,
                event.index(),
                global_sequence,
                deployment_id,
                event_schema_version,
                signer,
                extrinsic_hash,
                raw_payload.to_vec(),
                payload,
            ));
        }
        if iterated != event_count || consumed != decoded.bytes().len() {
            return Err(ArchiveError::MalformedResponse(
                "System::Events trailing or missing bytes",
            ));
        }
        Ok((event_count, accepted))
    }

    fn verify_runtime_version(&self, version: &RpcRuntimeVersion) -> Result<(), ArchiveError> {
        if version.spec_version != self.identity.runtime_spec_version()
            || version.transaction_version != self.identity.runtime_transaction_version()
        {
            return Err(ArchiveError::RuntimeMismatch);
        }
        let expected_keys = [
            "apis",
            "authoringVersion",
            "implName",
            "implVersion",
            "specName",
            "stateVersion",
            "systemVersion",
        ];
        if version.other.len() != expected_keys.len()
            || expected_keys
                .iter()
                .any(|key| !version.other.contains_key(*key))
            || version.other["specName"].as_str() != Some(self.identity.runtime_spec_name())
            || version.other["implName"].as_str() != Some(self.identity.runtime_impl_name())
            || version.other["authoringVersion"].as_u64()
                != Some(u64::from(self.identity.runtime_authoring_version()))
            || version.other["implVersion"].as_u64()
                != Some(u64::from(self.identity.runtime_impl_version()))
            || version.other["stateVersion"].as_u64()
                != Some(u64::from(self.identity.runtime_state_version()))
            || version.other["systemVersion"].as_u64()
                != Some(u64::from(self.identity.runtime_system_version()))
        {
            return Err(ArchiveError::RuntimeMismatch);
        }
        let apis = version.other["apis"]
            .as_array()
            .ok_or(ArchiveError::RuntimeMismatch)?;
        if apis.len() != self.identity.runtime_apis().len() {
            return Err(ArchiveError::RuntimeMismatch);
        }
        for (actual, (expected_id, expected_version)) in
            apis.iter().zip(self.identity.runtime_apis())
        {
            let pair = actual
                .as_array()
                .filter(|pair| pair.len() == 2)
                .ok_or(ArchiveError::RuntimeMismatch)?;
            if pair[0].as_str() != Some(expected_id.as_str())
                || pair[1].as_u64() != Some(u64::from(*expected_version))
            {
                return Err(ArchiveError::RuntimeMismatch);
            }
        }
        Ok(())
    }

    fn verify_runtime_code(&self, number: u64, code: &[u8]) -> Result<(), ArchiveError> {
        if code != runtime_wasm_bytes()
            || self.hash_bytes(number, code)? != *self.identity.runtime_code_hash()
        {
            Err(ArchiveError::RuntimeMismatch)
        } else {
            Ok(())
        }
    }

    fn hash_bytes(&self, number: u64, bytes: &[u8]) -> Result<[u8; 32], ArchiveError> {
        let at = self
            .decoder
            .at_block(number)
            .map_err(|_| ArchiveError::MalformedResponse("offline runtime identity"))?;
        Ok(*at.hasher().hash(bytes).as_fixed_bytes())
    }

    async fn call<T>(
        &self,
        operation: &'static str,
        future: RpcFuture<'_, T>,
    ) -> Result<T, ArchiveError> {
        match time::timeout(RPC_TIMEOUT, future).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(source)) => Err(ArchiveError::Rpc { operation, source }),
            Err(source) => Err(ArchiveError::Timeout { operation, source }),
        }
    }
}

struct RawVerifiedBlock {
    hash: [u8; 32],
    parent_hash: [u8; 32],
    runtime_spec_version: u32,
    extrinsics: Vec<Vec<u8>>,
    extrinsic_hashes: Vec<[u8; 32]>,
    event_bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
struct RpcBlock {
    header: MinimalHeader,
    extrinsics: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MinimalHeader {
    number: u64,
    parent_hash: [u8; 32],
    state_root: [u8; 32],
    extrinsics_root: [u8; 32],
    digest: Vec<u8>,
}

#[derive(Clone, Debug)]
struct RpcRuntimeVersion {
    spec_version: u32,
    transaction_version: u32,
    other: serde_json::Map<String, Value>,
}

trait ArchiveRpc: Send + Sync {
    fn finalized_head(&self) -> RpcFuture<'_, [u8; 32]>;
    fn block_hash(&self, number: u64) -> RpcFuture<'_, Option<[u8; 32]>>;
    fn header(&self, hash: [u8; 32]) -> RpcFuture<'_, Option<MinimalHeader>>;
    fn block(&self, hash: [u8; 32]) -> RpcFuture<'_, Option<RpcBlock>>;
    fn storage(&self, key: Vec<u8>, hash: [u8; 32]) -> RpcFuture<'_, Option<Vec<u8>>>;
    fn metadata(&self, hash: [u8; 32]) -> RpcFuture<'_, Vec<u8>>;
    fn runtime_version(&self, hash: [u8; 32]) -> RpcFuture<'_, RpcRuntimeVersion>;
}

struct SubxtArchiveRpc {
    methods: LegacyRpcMethods<RpcConfiguration>,
}

impl SubxtArchiveRpc {
    fn new(client: RpcClient) -> Self {
        Self {
            methods: LegacyRpcMethods::new(client),
        }
    }
}

impl ArchiveRpc for SubxtArchiveRpc {
    fn finalized_head(&self) -> RpcFuture<'_, [u8; 32]> {
        Box::pin(async move {
            self.methods
                .chain_get_finalized_head()
                .await
                .map(h256_bytes)
                .map_err(boxed)
        })
    }

    fn block_hash(&self, number: u64) -> RpcFuture<'_, Option<[u8; 32]>> {
        Box::pin(async move {
            self.methods
                .chain_get_block_hash(Some(number.into()))
                .await
                .map(|hash| hash.map(h256_bytes))
                .map_err(boxed)
        })
    }

    fn header(&self, hash: [u8; 32]) -> RpcFuture<'_, Option<MinimalHeader>> {
        Box::pin(async move {
            self.methods
                .chain_get_header(Some(H256::from(hash)))
                .await
                .map(|header| header.map(minimal_header))
                .map_err(boxed)
        })
    }

    fn block(&self, hash: [u8; 32]) -> RpcFuture<'_, Option<RpcBlock>> {
        Box::pin(async move {
            self.methods
                .chain_get_block(Some(H256::from(hash)))
                .await
                .map(|details| {
                    details.map(|details| RpcBlock {
                        header: minimal_header(details.block.header),
                        extrinsics: details
                            .block
                            .extrinsics
                            .into_iter()
                            .map(|bytes| bytes.0)
                            .collect(),
                    })
                })
                .map_err(boxed)
        })
    }

    fn storage(&self, key: Vec<u8>, hash: [u8; 32]) -> RpcFuture<'_, Option<Vec<u8>>> {
        Box::pin(async move {
            self.methods
                .state_get_storage(&key, Some(H256::from(hash)))
                .await
                .map_err(boxed)
        })
    }

    fn metadata(&self, hash: [u8; 32]) -> RpcFuture<'_, Vec<u8>> {
        Box::pin(async move {
            self.methods
                .state_get_metadata(Some(H256::from(hash)))
                .await
                .map(|metadata| metadata.into_raw())
                .map_err(boxed)
        })
    }

    fn runtime_version(&self, hash: [u8; 32]) -> RpcFuture<'_, RpcRuntimeVersion> {
        Box::pin(async move {
            self.methods
                .state_get_runtime_version(Some(H256::from(hash)))
                .await
                .map(|version| RpcRuntimeVersion {
                    spec_version: version.spec_version,
                    transaction_version: version.transaction_version,
                    other: version.other.into_iter().collect(),
                })
                .map_err(boxed)
        })
    }
}

fn minimal_header(header: RpcHeader) -> MinimalHeader {
    MinimalHeader {
        number: header.number,
        parent_hash: h256_bytes(header.parent_hash),
        state_root: h256_bytes(header.state_root),
        extrinsics_root: h256_bytes(header.extrinsics_root),
        digest: header.digest.encode(),
    }
}

fn h256_bytes(hash: H256) -> [u8; 32] {
    *hash.as_fixed_bytes()
}

fn boxed(error: impl Error + Send + Sync + 'static) -> BoxError {
    Box::new(error)
}

async fn connect_rpc(operation: &'static str, endpoint: &str) -> Result<RpcClient, ArchiveError> {
    match time::timeout(CONNECT_TIMEOUT, RpcClient::from_insecure_url(endpoint)).await {
        Ok(Ok(client)) => Ok(client),
        Ok(Err(source)) => Err(ArchiveError::Rpc {
            operation,
            source: Box::new(source),
        }),
        Err(source) => Err(ArchiveError::Timeout { operation, source }),
    }
}

fn validate_body_bounds(extrinsics: &[Vec<u8>]) -> Result<(), ArchiveError> {
    if extrinsics.len() > MAX_BLOCK_EXTRINSICS {
        return Err(ArchiveError::OverBound("block extrinsic count"));
    }
    let mut total = 0_usize;
    for extrinsic in extrinsics {
        total = total
            .checked_add(extrinsic.len())
            .ok_or(ArchiveError::OverBound("block body bytes"))?;
        if total > MAX_BLOCK_BODY_BYTES {
            return Err(ArchiveError::OverBound("block body bytes"));
        }
    }
    Ok(())
}

fn historical_probe_numbers(finalized: u64) -> Vec<u64> {
    let candidates = [
        0,
        finalized.min(1),
        finalized / 2 + finalized % 2,
        finalized,
    ];
    let mut probes = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if probes.last() != Some(&candidate) {
            probes.push(candidate);
        }
    }
    probes
}

fn validate_event_envelope(bytes: &[u8]) -> Result<u32, ArchiveError> {
    if bytes.is_empty() || bytes.len() > MAX_SYSTEM_EVENTS_BYTES {
        return Err(ArchiveError::OverBound("System::Events bytes"));
    }
    let prefix_length = compact_prefix_length(bytes)?;
    let mut cursor = bytes;
    let count = Compact::<u32>::decode(&mut cursor)
        .map_err(|_| ArchiveError::MalformedResponse("System::Events count prefix"))?
        .0;
    if count > MAX_SYSTEM_EVENT_RECORDS
        || Compact(count).encode().as_slice() != &bytes[..prefix_length]
    {
        return Err(ArchiveError::MalformedResponse(
            "noncanonical or overbound System::Events count",
        ));
    }
    Ok(count)
}

fn canonical_event_storage(
    number: u64,
    bytes: Option<Vec<u8>>,
    operation: &'static str,
) -> Result<Vec<u8>, ArchiveError> {
    match bytes {
        Some(bytes) => Ok(bytes),
        None if number == 0 => Ok(vec![0]),
        None => Err(ArchiveError::ArchiveHistoryUnavailable {
            operation,
            number: Some(number),
        }),
    }
}

fn compact_prefix_length(bytes: &[u8]) -> Result<usize, ArchiveError> {
    let first = *bytes
        .first()
        .ok_or(ArchiveError::MalformedResponse("missing compact prefix"))?;
    let length = match first & 0b11 {
        0 => 1,
        1 => 2,
        2 => 4,
        _ => usize::from(first >> 2) + 5,
    };
    if length > bytes.len() {
        Err(ArchiveError::MalformedResponse("truncated compact prefix"))
    } else {
        Ok(length)
    }
}

fn fixed_hex(hex: &str) -> Vec<u8> {
    debug_assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_digit(pair[0]);
            let low = hex_digit(pair[1]);
            (high << 4) | low
        })
        .collect()
}

const fn hex_digit(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => 0xff,
    }
}

fn map_payload_error(error: PayloadDecodeError) -> ArchiveError {
    match error {
        PayloadDecodeError::OverBound => ArchiveError::OverBound("accepted SCALE payload"),
        _ => ArchiveError::MalformedCanonicalPayload,
    }
}

struct FieldCursor<'a> {
    remaining: &'a [u8],
}

impl<'a> FieldCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ArchiveError> {
        self.take(N)?
            .try_into()
            .map_err(|_| ArchiveError::MalformedResponse("accepted event fields"))
    }

    fn u16(&mut self) -> Result<u16, ArchiveError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, ArchiveError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn rest(&mut self) -> &'a [u8] {
        let value = self.remaining;
        self.remaining = &[];
        value
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ArchiveError> {
        if self.remaining.len() < length {
            return Err(ArchiveError::MalformedResponse("accepted event fields"));
        }
        let (value, remaining) = self.remaining.split_at(length);
        self.remaining = remaining;
        Ok(value)
    }
}

/// Typed archive connection, identity, history, and decode failure.
#[derive(Debug)]
pub enum ArchiveError {
    Identity(IdentityError),
    NodeEvidence(NodeEvidenceError),
    NodeEvidenceEndpointMismatch,
    Rpc {
        operation: &'static str,
        source: BoxError,
    },
    Timeout {
        operation: &'static str,
        source: Elapsed,
    },
    ArchiveHistoryUnavailable {
        operation: &'static str,
        number: Option<u64>,
    },
    DeploymentMismatch,
    RuntimeMismatch,
    ForeignFinalizedHead,
    DisplacedFinalizedHead,
    NotFinalized {
        requested: u64,
        finalized: u64,
    },
    NonContiguousFinalizedStream,
    MalformedResponse(&'static str),
    MalformedCanonicalPayload,
    EventDecode {
        source: BoxError,
    },
    OverBound(&'static str),
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(error) => error.fmt(formatter),
            Self::NodeEvidence(error) => error.fmt(formatter),
            Self::NodeEvidenceEndpointMismatch => {
                formatter.write_str("archive node evidence belongs to a different endpoint")
            }
            Self::Rpc { operation, source } => {
                write!(formatter, "archive RPC {operation} failed: {source}")
            }
            Self::Timeout { operation, source } => {
                write!(formatter, "archive RPC {operation} timed out: {source}")
            }
            Self::ArchiveHistoryUnavailable { operation, number } => {
                write!(
                    formatter,
                    "archive history unavailable during {operation} at {number:?}"
                )
            }
            Self::DeploymentMismatch => {
                formatter.write_str("archive deployment identity does not match")
            }
            Self::RuntimeMismatch => formatter.write_str("archive runtime identity does not match"),
            Self::ForeignFinalizedHead => {
                formatter.write_str("finalized head belongs to a different archive client")
            }
            Self::DisplacedFinalizedHead => {
                formatter.write_str("node displaced a previously finalized coordinate")
            }
            Self::NotFinalized {
                requested,
                finalized,
            } => write!(
                formatter,
                "block {requested} is above finalized head {finalized}"
            ),
            Self::NonContiguousFinalizedStream => {
                formatter.write_str("finalized block stream is not contiguous")
            }
            Self::MalformedResponse(detail) => {
                write!(formatter, "archive response is malformed: {detail}")
            }
            Self::MalformedCanonicalPayload => {
                formatter.write_str("accepted event SCALE payload is malformed")
            }
            Self::EventDecode { source } => {
                write!(formatter, "System event decode failed: {source}")
            }
            Self::OverBound(resource) => write!(formatter, "archive {resource} exceeds its bound"),
        }
    }
}

impl Error for ArchiveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Identity(error) => Some(error),
            Self::NodeEvidence(error) => Some(error),
            Self::Rpc { source, .. } | Self::EventDecode { source } => Some(source.as_ref()),
            Self::Timeout { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{
            Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use sha2::{Digest, Sha256};

    use super::*;
    use crate::identity::verify_archive_cmdline;

    const PREFLIGHT_BYTES: &[u8] =
        include_bytes!("../../../tests/fixtures/finalized-events-v1/rpc-preflight-v1.json");
    const STREAM_JSON: &str =
        include_str!("../../../tests/fixtures/finalized-events-v1/valid-stream-v1.json");
    const BODY_JSON: [&str; 6] = [
        include_str!("../../../tests/fixtures/finalized-events-v1/raw/bodies/block-0-body.json"),
        include_str!("../../../tests/fixtures/finalized-events-v1/raw/bodies/block-1-body.json"),
        include_str!("../../../tests/fixtures/finalized-events-v1/raw/bodies/block-2-body.json"),
        include_str!("../../../tests/fixtures/finalized-events-v1/raw/bodies/block-3-body.json"),
        include_str!("../../../tests/fixtures/finalized-events-v1/raw/bodies/block-4-body.json"),
        include_str!("../../../tests/fixtures/finalized-events-v1/raw/bodies/block-5-body.json"),
    ];
    const EVENT_HEX: [&str; 6] = [
        include_str!(
            "../../../tests/fixtures/finalized-events-v1/raw/events/block-0-system-events.scale.hex"
        ),
        include_str!(
            "../../../tests/fixtures/finalized-events-v1/raw/events/block-1-system-events.scale.hex"
        ),
        include_str!(
            "../../../tests/fixtures/finalized-events-v1/raw/events/block-2-system-events.scale.hex"
        ),
        include_str!(
            "../../../tests/fixtures/finalized-events-v1/raw/events/block-3-system-events.scale.hex"
        ),
        include_str!(
            "../../../tests/fixtures/finalized-events-v1/raw/events/block-4-system-events.scale.hex"
        ),
        include_str!(
            "../../../tests/fixtures/finalized-events-v1/raw/events/block-5-system-events.scale.hex"
        ),
    ];
    const PREFLIGHT_SHA256: &str =
        "04a3d9e8dca2e10d435edf69be0b14626d184dd7c4acd11fb261dc388b12a8bb";

    #[derive(Clone)]
    struct FixtureBlock {
        number: u64,
        hash: [u8; 32],
        header: MinimalHeader,
        extrinsics: Vec<Vec<u8>>,
        event_bytes: Vec<u8>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum RpcRole {
        Parachain,
        Relay,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum ScriptFault {
        None,
        RpcBlock(u64),
        RpcHeader(u64),
        HeaderBodyMismatch(u64),
        MissingEvents(u64),
        MissingCode(u64),
        RelayGenesisMismatch,
        ParachainGenesisMismatch,
        StorageMismatch(&'static str),
        MetadataMismatch,
        CodeMismatch,
        RuntimeField(&'static str),
        RuntimeApi(&'static str),
    }

    #[derive(Debug)]
    struct ScriptedError(&'static str);

    impl fmt::Display for ScriptedError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl Error for ScriptedError {}

    struct ScriptedRpc {
        role: RpcRole,
        identity: DeploymentIdentity,
        blocks: Vec<FixtureBlock>,
        fault: ScriptFault,
        calls: Mutex<Vec<String>>,
        global_calls: Arc<Mutex<Vec<String>>>,
        finalized_head_override: Mutex<Option<u64>>,
        block_hash_overrides: Mutex<BTreeMap<u64, Option<[u8; 32]>>>,
        event_overrides: Mutex<BTreeMap<u64, Option<Vec<u8>>>>,
    }

    impl ScriptedRpc {
        fn new(
            role: RpcRole,
            identity: DeploymentIdentity,
            fault: ScriptFault,
            global_calls: Arc<Mutex<Vec<String>>>,
        ) -> Self {
            Self {
                role,
                identity,
                blocks: fixture_blocks(),
                fault,
                calls: Mutex::new(Vec::new()),
                global_calls,
                finalized_head_override: Mutex::new(None),
                block_hash_overrides: Mutex::new(BTreeMap::new()),
                event_overrides: Mutex::new(BTreeMap::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("call log mutex").clone()
        }

        fn global_calls(&self) -> Vec<String> {
            self.global_calls
                .lock()
                .expect("global call log mutex")
                .clone()
        }

        fn record(&self, call: impl Into<String>) {
            let call = call.into();
            self.calls
                .lock()
                .expect("call log mutex")
                .push(call.clone());
            self.global_calls
                .lock()
                .expect("global call log mutex")
                .push(call);
        }

        fn block_by_hash(&self, hash: [u8; 32]) -> Option<&FixtureBlock> {
            self.blocks.iter().find(|block| block.hash == hash)
        }

        fn block_by_number(&self, number: u64) -> Option<&FixtureBlock> {
            self.blocks.iter().find(|block| block.number == number)
        }

        fn override_events(&self, number: u64, bytes: Option<Vec<u8>>) {
            self.event_overrides
                .lock()
                .expect("event override mutex")
                .insert(number, bytes);
        }

        fn override_block_hash(&self, number: u64, hash: Option<[u8; 32]>) {
            self.block_hash_overrides
                .lock()
                .expect("block-hash override mutex")
                .insert(number, hash);
        }

        fn override_finalized_head(&self, number: u64) {
            *self
                .finalized_head_override
                .lock()
                .expect("finalized-head override mutex") = Some(number);
        }

        fn ready<T: Send + 'static>(value: T) -> RpcFuture<'static, T> {
            Box::pin(async move { Ok(value) })
        }

        fn failed<T: Send + 'static>(detail: &'static str) -> RpcFuture<'static, T> {
            Box::pin(async move { Err(Box::new(ScriptedError(detail)) as BoxError) })
        }

        fn runtime_version(&self) -> RpcRuntimeVersion {
            let mut other = serde_json::Map::new();
            other.insert(
                "specName".to_owned(),
                Value::String(self.identity.runtime_spec_name().to_owned()),
            );
            other.insert(
                "implName".to_owned(),
                Value::String(self.identity.runtime_impl_name().to_owned()),
            );
            other.insert(
                "authoringVersion".to_owned(),
                Value::from(self.identity.runtime_authoring_version()),
            );
            other.insert(
                "implVersion".to_owned(),
                Value::from(self.identity.runtime_impl_version()),
            );
            other.insert(
                "stateVersion".to_owned(),
                Value::from(self.identity.runtime_state_version()),
            );
            other.insert(
                "systemVersion".to_owned(),
                Value::from(self.identity.runtime_system_version()),
            );
            let mut apis = self
                .identity
                .runtime_apis()
                .iter()
                .map(|(id, version)| {
                    Value::Array(vec![Value::String(id.clone()), Value::from(*version)])
                })
                .collect::<Vec<_>>();
            match self.fault {
                ScriptFault::RuntimeField("spec_name") => {
                    other.insert("specName".to_owned(), Value::String("drift".to_owned()));
                }
                ScriptFault::RuntimeField("impl_name") => {
                    other.insert("implName".to_owned(), Value::String("drift".to_owned()));
                }
                ScriptFault::RuntimeField("authoring_version") => {
                    other.insert("authoringVersion".to_owned(), Value::from(2));
                }
                ScriptFault::RuntimeField("impl_version") => {
                    other.insert("implVersion".to_owned(), Value::from(1));
                }
                ScriptFault::RuntimeField("state_version") => {
                    other.insert("stateVersion".to_owned(), Value::from(2));
                }
                ScriptFault::RuntimeField("system_version") => {
                    other.insert("systemVersion".to_owned(), Value::from(2));
                }
                ScriptFault::RuntimeApi("remove") => {
                    apis.pop();
                }
                ScriptFault::RuntimeApi("append") => {
                    apis.push(Value::Array(vec![
                        Value::String("0x0000000000000000".to_owned()),
                        Value::from(1),
                    ]));
                }
                ScriptFault::RuntimeApi("swap") => apis.swap(0, 1),
                ScriptFault::RuntimeApi("id") => {
                    apis[0][0] = Value::String("0x0000000000000000".to_owned());
                }
                ScriptFault::RuntimeApi("version") => apis[0][1] = Value::from(99),
                _ => {}
            }
            other.insert("apis".to_owned(), Value::Array(apis));
            RpcRuntimeVersion {
                spec_version: if self.fault == ScriptFault::RuntimeField("spec_version") {
                    2
                } else {
                    self.identity.runtime_spec_version()
                },
                transaction_version: if self.fault
                    == ScriptFault::RuntimeField("transaction_version")
                {
                    2
                } else {
                    self.identity.runtime_transaction_version()
                },
                other,
            }
        }
    }

    impl ArchiveRpc for ScriptedRpc {
        fn finalized_head(&self) -> RpcFuture<'_, [u8; 32]> {
            self.record("parachain:chain_getFinalizedHead");
            let number = self
                .finalized_head_override
                .lock()
                .expect("finalized-head override mutex")
                .unwrap_or_else(|| self.blocks.last().expect("fixture head").number);
            Self::ready(
                self.block_by_number(number)
                    .expect("fixture head number")
                    .hash,
            )
        }

        fn block_hash(&self, number: u64) -> RpcFuture<'_, Option<[u8; 32]>> {
            self.record(format!("{}:chain_getBlockHash:{number}", self.role_name()));
            if self.role == RpcRole::Parachain
                && let Some(value) = self
                    .block_hash_overrides
                    .lock()
                    .expect("block-hash override mutex")
                    .get(&number)
                    .copied()
            {
                return Self::ready(value);
            }
            let value = match self.role {
                RpcRole::Relay if number == 0 => {
                    let mut hash = *self.identity.relay_genesis_hash();
                    if self.fault == ScriptFault::RelayGenesisMismatch {
                        hash = [0xff; 32];
                    }
                    Some(hash)
                }
                RpcRole::Relay => None,
                RpcRole::Parachain
                    if number == 0 && self.fault == ScriptFault::ParachainGenesisMismatch =>
                {
                    Some([0xff; 32])
                }
                RpcRole::Parachain => self.block_by_number(number).map(|block| block.hash),
            };
            Self::ready(value)
        }

        fn header(&self, hash: [u8; 32]) -> RpcFuture<'_, Option<MinimalHeader>> {
            let number = self.block_by_hash(hash).map(|block| block.number);
            self.record(format!(
                "parachain:chain_getHeader:{}",
                number.map_or_else(|| "missing".to_owned(), |value| value.to_string())
            ));
            if number.is_some_and(|number| self.fault == ScriptFault::RpcHeader(number)) {
                return Self::failed("scripted header interruption");
            }
            let mut header = self.block_by_hash(hash).map(|block| block.header.clone());
            if number.is_some_and(|number| self.fault == ScriptFault::HeaderBodyMismatch(number))
                && let Some(header) = &mut header
            {
                header.state_root[0] ^= 1;
            }
            Self::ready(header)
        }

        fn block(&self, hash: [u8; 32]) -> RpcFuture<'_, Option<RpcBlock>> {
            let number = self.block_by_hash(hash).map(|block| block.number);
            self.record(format!(
                "parachain:chain_getBlock:{}",
                number.map_or_else(|| "missing".to_owned(), |value| value.to_string())
            ));
            if number.is_some_and(|number| self.fault == ScriptFault::RpcBlock(number)) {
                return Self::failed("scripted block interruption");
            }
            Self::ready(self.block_by_hash(hash).map(|block| RpcBlock {
                header: block.header.clone(),
                extrinsics: block.extrinsics.clone(),
            }))
        }

        fn storage(&self, key: Vec<u8>, hash: [u8; 32]) -> RpcFuture<'_, Option<Vec<u8>>> {
            let number = self
                .block_by_hash(hash)
                .map_or(u64::MAX, |block| block.number);
            if key == fixed_hex(SYSTEM_EVENTS_KEY_HEX) {
                self.record(format!("parachain:state_getStorage:events:{number}"));
                if self.fault == ScriptFault::MissingEvents(number) {
                    return Self::ready(None);
                }
                if let Some(value) = self
                    .event_overrides
                    .lock()
                    .expect("event override mutex")
                    .get(&number)
                    .cloned()
                {
                    return Self::ready(value);
                }
                return Self::ready(if number == 0 {
                    None
                } else {
                    self.block_by_number(number)
                        .map(|block| block.event_bytes.clone())
                });
            }
            if key == fixed_hex(RUNTIME_CODE_KEY_HEX) {
                self.record(format!("parachain:state_getStorage:code:{number}"));
                if self.fault == ScriptFault::MissingCode(number) {
                    return Self::ready(None);
                }
                let mut code = runtime_wasm_bytes().to_vec();
                if self.fault == ScriptFault::CodeMismatch {
                    code[0] ^= 1;
                }
                return Self::ready(Some(code));
            }
            let (label, mut value) = if key == fixed_hex(DEPLOYMENT_ID_KEY_HEX) {
                ("deployment_id", self.identity.deployment_id().to_vec())
            } else if key == fixed_hex(EVENT_SCHEMA_VERSION_KEY_HEX) {
                (
                    "event_schema_version",
                    self.identity.event_schema_version().to_le_bytes().to_vec(),
                )
            } else if key == fixed_hex(PALLET_STORAGE_VERSION_KEY_HEX) {
                (
                    "pallet_storage_version",
                    self.identity
                        .pallet_storage_version()
                        .to_le_bytes()
                        .to_vec(),
                )
            } else if key == fixed_hex(PARA_ID_KEY_HEX) {
                ("para_id", self.identity.para_id().to_le_bytes().to_vec())
            } else {
                self.record(format!("parachain:state_getStorage:unknown:{number}"));
                return Self::ready(None);
            };
            self.record(format!("parachain:state_getStorage:{label}:{number}"));
            if self.fault == ScriptFault::StorageMismatch(label) {
                value[0] ^= 1;
            }
            Self::ready(Some(value))
        }

        fn metadata(&self, hash: [u8; 32]) -> RpcFuture<'_, Vec<u8>> {
            let number = self
                .block_by_hash(hash)
                .map_or(u64::MAX, |block| block.number);
            self.record(format!("parachain:state_getMetadata:{number}"));
            let mut metadata = metadata_bytes().to_vec();
            if self.fault == ScriptFault::MetadataMismatch {
                let last = metadata.len() - 1;
                metadata[last] ^= 1;
            }
            Self::ready(metadata)
        }

        fn runtime_version(&self, hash: [u8; 32]) -> RpcFuture<'_, RpcRuntimeVersion> {
            let number = self
                .block_by_hash(hash)
                .map_or(u64::MAX, |block| block.number);
            self.record(format!("parachain:state_getRuntimeVersion:{number}"));
            Self::ready(self.runtime_version())
        }
    }

    impl ScriptedRpc {
        const fn role_name(&self) -> &'static str {
            match self.role {
                RpcRole::Parachain => "parachain",
                RpcRole::Relay => "relay",
            }
        }
    }

    fn fixture_blocks() -> Vec<FixtureBlock> {
        let stream: Value = serde_json::from_str(STREAM_JSON).expect("stream fixture JSON");
        stream["blocks"]
            .as_array()
            .expect("fixture blocks")
            .iter()
            .enumerate()
            .map(|(index, block)| {
                let number = block["block_number"]
                    .as_str()
                    .expect("block number text")
                    .parse::<u64>()
                    .expect("block number");
                assert_eq!(usize::try_from(number).expect("fixture usize"), index);
                let body: Value =
                    serde_json::from_str(BODY_JSON[index]).expect("body fixture JSON");
                let extrinsics = body["extrinsics"]
                    .as_array()
                    .expect("fixture extrinsics")
                    .iter()
                    .map(|extrinsic| {
                        decode_hex(extrinsic["scale_hex"].as_str().expect("extrinsic hex"))
                    })
                    .collect();
                FixtureBlock {
                    number,
                    hash: decode_hash(block["block_hash"].as_str().expect("block hash")),
                    header: MinimalHeader {
                        number,
                        parent_hash: decode_hash(
                            block["parent_hash"].as_str().expect("parent hash"),
                        ),
                        state_root: [0x55; 32],
                        extrinsics_root: [0x77; 32],
                        digest: vec![0],
                    },
                    extrinsics,
                    event_bytes: decode_hex(EVENT_HEX[index]),
                }
            })
            .collect()
    }

    fn decode_hash(input: &str) -> [u8; 32] {
        decode_hex(input).try_into().expect("32-byte fixture hash")
    }

    fn decode_hex(input: &str) -> Vec<u8> {
        let input = input.trim().strip_prefix("0x").unwrap_or(input.trim());
        assert_eq!(input.len() % 2, 0);
        input
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII fixture hex"), 16)
                    .expect("fixture hex")
            })
            .collect()
    }

    async fn scripted_connect(
        fault: ScriptFault,
        spy: Arc<AtomicUsize>,
    ) -> (
        Result<VerifiedArchiveClient, ArchiveError>,
        Arc<ScriptedRpc>,
        Arc<ScriptedRpc>,
    ) {
        let endpoint =
            StrictLoopbackWsUrl::parse("ws://127.0.0.1:9988/").expect("fixed fixture endpoint");
        let identity = DeploymentIdentity::load().expect("fixed identity");
        let global_calls = Arc::new(Mutex::new(Vec::new()));
        let parachain = Arc::new(ScriptedRpc::new(
            RpcRole::Parachain,
            identity.clone(),
            fault,
            global_calls.clone(),
        ));
        let relay = Arc::new(ScriptedRpc::new(
            RpcRole::Relay,
            identity.clone(),
            fault,
            global_calls,
        ));
        let result = VerifiedArchiveClient::connect_with_rpc_spied(
            endpoint,
            identity,
            parachain.clone(),
            relay.clone(),
            spy,
        )
        .await;
        (result, parachain, relay)
    }

    fn expected_preflight_calls() -> Vec<String> {
        [
            "parachain:chain_getFinalizedHead",
            "parachain:chain_getHeader:5",
            "parachain:chain_getBlockHash:5",
            "parachain:chain_getBlockHash:0",
            "parachain:state_getStorage:deployment_id:0",
            "parachain:state_getStorage:event_schema_version:0",
            "parachain:state_getStorage:pallet_storage_version:0",
            "parachain:state_getStorage:para_id:0",
            "parachain:state_getMetadata:0",
            "parachain:state_getRuntimeVersion:0",
            "parachain:state_getStorage:code:0",
            "parachain:chain_getBlockHash:0",
            "parachain:chain_getHeader:0",
            "parachain:chain_getBlock:0",
            "parachain:state_getStorage:events:0",
            "parachain:state_getRuntimeVersion:0",
            "parachain:state_getStorage:code:0",
            "parachain:chain_getBlockHash:1",
            "parachain:chain_getHeader:1",
            "parachain:chain_getBlock:1",
            "parachain:chain_getBlockHash:0",
            "parachain:state_getStorage:events:1",
            "parachain:state_getRuntimeVersion:1",
            "parachain:state_getStorage:code:1",
            "parachain:chain_getBlockHash:3",
            "parachain:chain_getHeader:3",
            "parachain:chain_getBlock:3",
            "parachain:chain_getBlockHash:2",
            "parachain:state_getStorage:events:3",
            "parachain:state_getRuntimeVersion:3",
            "parachain:state_getStorage:code:3",
            "parachain:chain_getBlockHash:5",
            "parachain:chain_getHeader:5",
            "parachain:chain_getBlock:5",
            "parachain:chain_getBlockHash:4",
            "parachain:state_getStorage:events:5",
            "parachain:state_getRuntimeVersion:5",
            "parachain:state_getStorage:code:5",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }

    fn expected_global_preflight_calls() -> Vec<String> {
        let mut calls = expected_preflight_calls();
        calls.insert(3, "relay:chain_getBlockHash:0".to_owned());
        calls
    }

    #[tokio::test]
    async fn test_rpc_archive_anchor_and_runtime_preflight_precedes_projection() {
        assert_eq!(
            hex_bytes(&Sha256::digest(PREFLIGHT_BYTES)),
            PREFLIGHT_SHA256,
            "the independent preflight oracle must not drift with this test"
        );
        let preflight: Value =
            serde_json::from_slice(PREFLIGHT_BYTES).expect("preflight fixture JSON");
        for case in preflight["connection"]["url_cases"]
            .as_array()
            .expect("URL cases")
        {
            let input = case["input"].as_str().expect("URL input");
            assert_eq!(
                StrictLoopbackWsUrl::parse(input).is_ok(),
                case["accepted"].as_bool().expect("URL decision"),
                "URL oracle case {input}"
            );
        }

        let identity = DeploymentIdentity::load().expect("fixed artifacts and provenance");
        assert_eq!(
            identity.runtime_apis().len(),
            preflight["identity"]["runtime"]["apis"]
                .as_array()
                .expect("runtime API inventory")
                .len()
        );
        let valid_argv =
            b"polkadot-omni-node\0--rpc-port\09988\0--blocks-pruning\0archive\0--state-pruning\0archive\0";
        assert!(verify_archive_cmdline(valid_argv, 9988).is_ok());
        for rejected in preflight["connection"]["rejected_raw_argv_tokens"]
            .as_array()
            .expect("rejected argv tokens")
        {
            let token = rejected.as_str().expect("rejected argv token");
            let cmdline = format!(
                "polkadot-omni-node\0--rpc-port\09988\0{token}\0--blocks-pruning\0archive\0--state-pruning\0archive\0"
            );
            assert!(verify_archive_cmdline(cmdline.as_bytes(), 9988).is_err());
        }
        assert_eq!(historical_probe_numbers(0), [0]);
        assert_eq!(historical_probe_numbers(1), [0, 1]);
        assert_eq!(historical_probe_numbers(5), [0, 1, 3, 5]);
        assert_eq!(
            historical_probe_numbers(u64::MAX),
            [0, 1, 1_u64 << 63, u64::MAX]
        );

        let decode_spy = Arc::new(AtomicUsize::new(0));
        let (result, parachain, relay) =
            scripted_connect(ScriptFault::None, decode_spy.clone()).await;
        let client = result.expect("the fixed scripted preflight should pass");
        assert_eq!(decode_spy.load(Ordering::SeqCst), 0);
        assert_eq!(relay.calls(), ["relay:chain_getBlockHash:0"]);
        assert_eq!(parachain.calls(), expected_preflight_calls());
        assert_eq!(parachain.global_calls(), expected_global_preflight_calls());
        assert_eq!(client.endpoint().as_str(), "ws://127.0.0.1:9988/");

        let faults = [
            ScriptFault::RpcBlock(0),
            ScriptFault::MissingEvents(1),
            ScriptFault::MissingCode(3),
            ScriptFault::RpcHeader(5),
            ScriptFault::RelayGenesisMismatch,
            ScriptFault::ParachainGenesisMismatch,
            ScriptFault::StorageMismatch("deployment_id"),
            ScriptFault::StorageMismatch("event_schema_version"),
            ScriptFault::StorageMismatch("pallet_storage_version"),
            ScriptFault::StorageMismatch("para_id"),
            ScriptFault::MetadataMismatch,
            ScriptFault::CodeMismatch,
            ScriptFault::RuntimeField("spec_name"),
            ScriptFault::RuntimeField("impl_name"),
            ScriptFault::RuntimeField("authoring_version"),
            ScriptFault::RuntimeField("spec_version"),
            ScriptFault::RuntimeField("impl_version"),
            ScriptFault::RuntimeField("transaction_version"),
            ScriptFault::RuntimeField("state_version"),
            ScriptFault::RuntimeField("system_version"),
            ScriptFault::RuntimeApi("remove"),
            ScriptFault::RuntimeApi("append"),
            ScriptFault::RuntimeApi("swap"),
            ScriptFault::RuntimeApi("id"),
            ScriptFault::RuntimeApi("version"),
        ];
        for fault in faults {
            let spy = Arc::new(AtomicUsize::new(0));
            let (result, _, _) = scripted_connect(fault, spy.clone()).await;
            let error = result.expect_err("each scripted preflight mutation must reject");
            assert_eq!(spy.load(Ordering::SeqCst), 0, "preflight fault {fault:?}");
            if matches!(fault, ScriptFault::RpcBlock(_) | ScriptFault::RpcHeader(_)) {
                let source = error.source().expect("RPC source must be preserved");
                assert!(source.downcast_ref::<ScriptedError>().is_some());
            }
        }
    }

    #[tokio::test]
    async fn scripted_finalized_stream_decodes_every_coordinate_and_body_hash() {
        let spy = Arc::new(AtomicUsize::new(0));
        let (result, _, _) = scripted_connect(ScriptFault::None, spy.clone()).await;
        let client = result.expect("scripted preflight");
        let head = client
            .finalized_head()
            .await
            .expect("scripted finalized head");
        assert_eq!(head.number(), 5);
        let stream: Value = serde_json::from_str(STREAM_JSON).expect("stream fixture JSON");
        let mut global_sequences = Vec::new();
        for expected in stream["blocks"].as_array().expect("fixture blocks") {
            let number = expected["block_number"]
                .as_str()
                .expect("block number")
                .parse::<u64>()
                .expect("numeric block number");
            let block = client
                .finalized_block(&head, number)
                .await
                .expect("canonical fixture block");
            assert_eq!(block.number(), number);
            assert_eq!(
                block.hash(),
                &decode_hash(expected["block_hash"].as_str().unwrap())
            );
            assert_eq!(
                block.parent_hash(),
                &decode_hash(expected["parent_hash"].as_str().unwrap())
            );
            assert_eq!(
                u64::from(block.system_event_record_count()),
                expected["system_event_record_count"].as_u64().unwrap()
            );
            assert_eq!(
                block.events().len(),
                expected["cubikan_event_count"].as_u64().unwrap() as usize
            );
            assert_eq!(
                block.raw_system_events(),
                decode_hex(EVENT_HEX[number as usize])
            );
            assert_eq!(block.raw_extrinsics().len(), block.extrinsic_hashes().len());
            for (event, oracle) in block.events().iter().zip(
                expected["events"]
                    .as_array()
                    .expect("fixture accepted events"),
            ) {
                assert_eq!(
                    u64::from(event.extrinsic_index()),
                    oracle["extrinsic_index"].as_u64().unwrap()
                );
                assert_eq!(
                    u64::from(event.system_event_index()),
                    oracle["system_event_index"].as_u64().unwrap()
                );
                assert_eq!(
                    event.global_sequence(),
                    oracle["global_sequence"]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                );
                assert_eq!(
                    event.signer(),
                    &decode_hash(oracle["signer"].as_str().unwrap())
                );
                assert_eq!(
                    event.extrinsic_hash(),
                    &decode_hash(oracle["extrinsic_hash"].as_str().unwrap())
                );
                assert_eq!(
                    event.raw_payload(),
                    fixture_payload(oracle["payload"].as_str().unwrap())
                );
                global_sequences.push(event.global_sequence());
            }
        }
        assert_eq!(global_sequences, (1_u64..=11).collect::<Vec<_>>());
        assert_eq!(spy.load(Ordering::SeqCst), 6);
    }

    #[tokio::test]
    async fn scripted_finality_and_fetch_faults_return_no_block_or_decode_authority() {
        let spy = Arc::new(AtomicUsize::new(0));
        let (result, rpc, _) = scripted_connect(ScriptFault::None, spy.clone()).await;
        let client = result.expect("scripted preflight");
        let head = client.finalized_head().await.expect("scripted head");
        rpc.override_finalized_head(3);
        let finalized_three = client
            .finalized_head()
            .await
            .expect("scripted earlier finalized head");
        assert_eq!(finalized_three.number(), 3);
        assert!(rpc.block_by_number(4).is_some(), "best-only block exists");
        let calls_before_best_only = rpc.calls().len();
        assert!(matches!(
            client.finalized_block(&finalized_three, 4).await,
            Err(ArchiveError::NotFinalized {
                requested: 4,
                finalized: 3
            })
        ));
        assert_eq!(rpc.calls().len(), calls_before_best_only);
        assert_eq!(spy.load(Ordering::SeqCst), 0);

        rpc.override_finalized_head(5);
        rpc.override_block_hash(5, Some([0xaa; 32]));
        assert!(matches!(
            client.finalized_block(&head, 2).await,
            Err(ArchiveError::DisplacedFinalizedHead)
        ));
        assert!(matches!(
            client.finalized_head().await,
            Err(ArchiveError::DisplacedFinalizedHead)
        ));
        assert_eq!(spy.load(Ordering::SeqCst), 0);

        let spy = Arc::new(AtomicUsize::new(0));
        let (result, _, _) = scripted_connect(ScriptFault::RpcBlock(2), spy.clone()).await;
        let client = result.expect("block-2 interruption is after preflight roles");
        let head = client.finalized_head().await.expect("scripted head");
        let error = client
            .finalized_block(&head, 2)
            .await
            .expect_err("finalized body RPC interruption must reject");
        assert!(matches!(
            &error,
            ArchiveError::Rpc {
                operation: "chain_getBlock",
                ..
            }
        ));
        assert!(
            error
                .source()
                .and_then(|source| source.downcast_ref::<ScriptedError>())
                .is_some()
        );
        assert_eq!(spy.load(Ordering::SeqCst), 0);

        let spy = Arc::new(AtomicUsize::new(0));
        let (result, _, _) =
            scripted_connect(ScriptFault::HeaderBodyMismatch(2), spy.clone()).await;
        let client = result.expect("block-2 header mismatch is after preflight roles");
        let head = client.finalized_head().await.expect("scripted head");
        assert!(matches!(
            client.finalized_block(&head, 2).await,
            Err(ArchiveError::NonContiguousFinalizedStream)
        ));
        assert_eq!(spy.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn scripted_event_decoder_rejects_absence_topics_phase_count_trailing_and_body_join() {
        async fn connected() -> (VerifiedArchiveClient, Arc<ScriptedRpc>, FinalizedHead) {
            let spy = Arc::new(AtomicUsize::new(0));
            let (result, rpc, _) = scripted_connect(ScriptFault::None, spy).await;
            let client = result.expect("scripted preflight");
            let head = client.finalized_head().await.expect("scripted head");
            (client, rpc, head)
        }

        let (client, rpc, head) = connected().await;
        rpc.override_events(1, None);
        assert!(matches!(
            client.finalized_block(&head, 1).await,
            Err(ArchiveError::ArchiveHistoryUnavailable {
                number: Some(1),
                ..
            })
        ));

        let (client, rpc, head) = connected().await;
        let mut topics = decode_hex(EVENT_HEX[4]);
        assert_eq!(topics.pop(), Some(0));
        topics.push(4);
        topics.extend_from_slice(&[0x42; 32]);
        rpc.override_events(4, Some(topics));
        assert!(matches!(
            client.finalized_block(&head, 4).await,
            Err(ArchiveError::MalformedResponse(
                "accepted event topics are not empty"
            ))
        ));

        let (client, rpc, head) = connected().await;
        let mut phase = decode_hex(EVENT_HEX[3]);
        phase[1] = 1;
        phase.drain(2..6);
        rpc.override_events(3, Some(phase));
        assert!(matches!(
            client.finalized_block(&head, 3).await,
            Err(ArchiveError::MalformedResponse(
                "accepted event phase is not ApplyExtrinsic"
            ))
        ));

        let (client, rpc, head) = connected().await;
        let mut initialization_phase = decode_hex(EVENT_HEX[3]);
        initialization_phase[1] = 2;
        initialization_phase.drain(2..6);
        rpc.override_events(3, Some(initialization_phase));
        assert!(matches!(
            client.finalized_block(&head, 3).await,
            Err(ArchiveError::MalformedResponse(
                "accepted event phase is not ApplyExtrinsic"
            ))
        ));

        let (client, rpc, head) = connected().await;
        let mut count = decode_hex(EVENT_HEX[3]);
        count[0] = 20;
        rpc.override_events(3, Some(count));
        assert!(client.finalized_block(&head, 3).await.is_err());

        let (client, rpc, head) = connected().await;
        let mut count_too_small = decode_hex(EVENT_HEX[3]);
        count_too_small[0] = 12;
        rpc.override_events(3, Some(count_too_small));
        assert!(matches!(
            client.finalized_block(&head, 3).await,
            Err(ArchiveError::MalformedResponse(
                "System::Events trailing or missing bytes"
            ))
        ));

        let (client, rpc, head) = connected().await;
        rpc.override_events(3, Some(vec![3]));
        assert!(matches!(
            client.finalized_block(&head, 3).await,
            Err(ArchiveError::MalformedResponse("truncated compact prefix"))
        ));

        let (client, rpc, head) = connected().await;
        let mut trailing = decode_hex(EVENT_HEX[3]);
        trailing.push(0);
        rpc.override_events(3, Some(trailing));
        assert!(client.finalized_block(&head, 3).await.is_err());

        let (client, rpc, head) = connected().await;
        let mut body_join = decode_hex(EVENT_HEX[3]);
        body_join[2..6].copy_from_slice(&u32::MAX.to_le_bytes());
        rpc.override_events(3, Some(body_join));
        assert!(matches!(
            client.finalized_block(&head, 3).await,
            Err(ArchiveError::MalformedResponse("accepted event body join"))
        ));

        let (client, rpc, head) = connected().await;
        assert_eq!(
            decode_canonical_payload(&[0xff]),
            Err(PayloadDecodeError::InvalidVariant)
        );
        let mut unknown_payload = decode_hex(EVENT_HEX[2]);
        let known_payload = fixture_payload("raw/payloads/0001-unit-created-a.scale.hex");
        let payload_offset = unknown_payload
            .windows(known_payload.len())
            .position(|window| window == known_payload)
            .expect("accepted payload bytes in System::Events oracle");
        unknown_payload[payload_offset] = 0xff;
        rpc.override_events(2, Some(unknown_payload));
        assert!(matches!(
            client.finalized_block(&head, 2).await,
            Err(ArchiveError::EventDecode { .. } | ArchiveError::MalformedCanonicalPayload)
        ));
    }

    fn fixture_payload(path: &str) -> Vec<u8> {
        let index = path
            .strip_prefix("raw/payloads/")
            .and_then(|name| name.get(..4))
            .and_then(|number| number.parse::<usize>().ok())
            .expect("fixture payload index");
        const PAYLOADS: [&str; 11] = [
            include_str!(
                "../../../tests/fixtures/finalized-events-v1/raw/payloads/0001-unit-created-a.scale.hex"
            ),
            include_str!(
                "../../../tests/fixtures/finalized-events-v1/raw/payloads/0002-unit-created-b.scale.hex"
            ),
            include_str!(
                "../../../tests/fixtures/finalized-events-v1/raw/payloads/0003-relationship-definition-created.scale.hex"
            ),
            include_str!(
                "../../../tests/fixtures/finalized-events-v1/raw/payloads/0004-relationship-created.scale.hex"
            ),
            include_str!(
                "../../../tests/fixtures/finalized-events-v1/raw/payloads/0005-association-recorded-a.scale.hex"
            ),
            include_str!(
                "../../../tests/fixtures/finalized-events-v1/raw/payloads/0006-unit-transitioned-a.scale.hex"
            ),
            include_str!(
                "../../../tests/fixtures/finalized-events-v1/raw/payloads/0007-unit-completed-a.scale.hex"
            ),
            include_str!(
                "../../../tests/fixtures/finalized-events-v1/raw/payloads/0008-relationship-deleted.scale.hex"
            ),
            include_str!(
                "../../../tests/fixtures/finalized-events-v1/raw/payloads/0009-association-revoked-a.scale.hex"
            ),
            include_str!(
                "../../../tests/fixtures/finalized-events-v1/raw/payloads/0010-association-recorded-b.scale.hex"
            ),
            include_str!(
                "../../../tests/fixtures/finalized-events-v1/raw/payloads/0011-relationship-recreated.scale.hex"
            ),
        ];
        decode_hex(PAYLOADS[index - 1])
    }

    fn hex_bytes(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            output.push(char::from(DIGITS[usize::from(byte >> 4)]));
            output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        output
    }
}
