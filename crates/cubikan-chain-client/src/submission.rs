//! Crash-recoverable, finalized-only mutation submission.
//!
//! This module deliberately has no SQLite dependency and accepts no raw RPC,
//! signer key, nonce, block, event, or journal path from its caller.  The only
//! online authority is an already verified archive client; the only signers are
//! the two development submitters fixed by the local chain specification.

use std::{
    borrow::Cow, error::Error, fmt, future::Future, path::Path, pin::Pin, sync::Arc, time::Duration,
};

use cubikan_core::{
    AssociationSubject, ExternalReference, IntentSpecies, IntentUnitId, PhaseId,
    RecordedAssociation, RelationshipDefinition, RelationshipIdentity, RelationshipPolicy,
    Workflow,
};
use parity_scale_codec::{Compact, Decode, Encode};
use subxt::ext::frame_decode::storage::StorageTypeInfo;
use subxt::{
    Metadata, OfflineClient,
    config::{
        ClientState, Config, HashFor, Hasher, RpcConfigFor, TransactionExtension,
        substrate::{H256, MultiSignature, SpecVersionForRange, SubstrateConfig},
        transaction_extensions::{
            ChargeTransactionPayment, ChargeTransactionPaymentParams, CheckGenesis, CheckMortality,
            CheckMortalityParams, CheckNonce, CheckNonceParams, CheckSpecVersion, CheckTxVersion,
        },
    },
    dynamic::{Value, storage, tx},
    error::DispatchError,
    events::Phase,
    ext::{
        frame_decode,
        scale_encode::EncodeAsType,
        scale_value::{At, scale::decode_as_type},
    },
    rpcs::{LegacyRpcMethods, RpcClient},
    utils::Era,
};
use subxt_signer::sr25519::{self, Keypair};

use crate::{
    AcceptedEvent, CanonicalPayload, FinalizedBlock, VerifiedArchiveClient,
    identity::metadata_bytes,
    submission_journal::{
        JournalError, JournalRecord, JournalState, MutationOperation as JournalOperation,
        SignerLane,
    },
};

type BoxError = Box<dyn Error + Send + Sync + 'static>;
type ChainFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ChainFailure>> + Send + 'a>>;

const COMMAND_SCHEMA_VERSION: u16 = 1;
const EVENT_SCHEMA_VERSION: u16 = 1;
const MORTAL_PERIOD: u64 = 64;
const DEFAULT_FINALITY_WAIT: Duration = Duration::from_secs(120);
const CUBIKAN_PALLET: &str = "Cubikan";
const EXACT_EXTENSION_NAMES: [&str; 9] = [
    "CheckNonZeroSender",
    "CheckSpecVersion",
    "CheckTxVersion",
    "CheckGenesis",
    "CheckMortality",
    "CheckNonce",
    "CheckWeight",
    "ChargeTransactionPayment",
    "StorageWeightReclaim",
];

/// The only development signers accepted by the local submission boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DevSigner {
    Charlie,
    Dave,
}

impl DevSigner {
    fn keypair(self) -> Keypair {
        match self {
            Self::Charlie => sr25519::dev::charlie(),
            Self::Dave => sr25519::dev::dave(),
        }
    }

    /// Returns the public `AccountId32`; no secret material is exposed.
    #[must_use]
    pub fn account_id(self) -> [u8; 32] {
        self.keypair().public_key().0
    }
}

/// One closed CubiKan mutation.  Command schema version 1 is injected here and
/// is never caller-selectable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Mutation {
    CreateUnit {
        id: IntentUnitId,
        origin: ExternalReference,
        species: IntentSpecies,
        workflow: Workflow,
    },
    TransitionUnit {
        id: IntentUnitId,
        target: PhaseId,
        expected_revision: u64,
    },
    CompleteUnit {
        id: IntentUnitId,
        expected_revision: u64,
    },
    CreateRelationshipDefinition(RelationshipDefinition),
    CreateRelationship(RelationshipIdentity),
    DeleteRelationship(RelationshipIdentity),
    RecordAssociation(RecordedAssociation),
    RevokeAssociation(RecordedAssociation),
}

impl Mutation {
    /// Returns the stable operation discriminator persisted in the signer journal.
    #[must_use]
    pub const fn operation(&self) -> MutationOperation {
        match self {
            Self::CreateUnit { .. } => MutationOperation::CreateUnit,
            Self::TransitionUnit { .. } => MutationOperation::TransitionUnit,
            Self::CompleteUnit { .. } => MutationOperation::CompleteUnit,
            Self::CreateRelationshipDefinition(_) => {
                MutationOperation::CreateRelationshipDefinition
            }
            Self::CreateRelationship(_) => MutationOperation::CreateRelationship,
            Self::DeleteRelationship(_) => MutationOperation::DeleteRelationship,
            Self::RecordAssociation(_) => MutationOperation::RecordAssociation,
            Self::RevokeAssociation(_) => MutationOperation::RevokeAssociation,
        }
    }
}

/// Stable original-operation discriminator.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MutationOperation {
    CreateUnit,
    TransitionUnit,
    CompleteUnit,
    CreateRelationshipDefinition,
    CreateRelationship,
    DeleteRelationship,
    RecordAssociation,
    RevokeAssociation,
}

impl MutationOperation {
    const fn journal(self) -> JournalOperation {
        match self {
            Self::CreateUnit => JournalOperation::CreateUnit,
            Self::TransitionUnit => JournalOperation::TransitionUnit,
            Self::CompleteUnit => JournalOperation::CompleteUnit,
            Self::CreateRelationshipDefinition => JournalOperation::CreateDefinition,
            Self::CreateRelationship => JournalOperation::CreateRelationship,
            Self::DeleteRelationship => JournalOperation::DeleteRelationship,
            Self::RecordAssociation => JournalOperation::RecordAssociation,
            Self::RevokeAssociation => JournalOperation::RevokeAssociation,
        }
    }

    const fn from_journal(value: JournalOperation) -> Self {
        match value {
            JournalOperation::CreateUnit => Self::CreateUnit,
            JournalOperation::TransitionUnit => Self::TransitionUnit,
            JournalOperation::CompleteUnit => Self::CompleteUnit,
            JournalOperation::CreateDefinition => Self::CreateRelationshipDefinition,
            JournalOperation::CreateRelationship => Self::CreateRelationship,
            JournalOperation::DeleteRelationship => Self::DeleteRelationship,
            JournalOperation::RecordAssociation => Self::RecordAssociation,
            JournalOperation::RevokeAssociation => Self::RevokeAssociation,
        }
    }
}

/// Absolute inclusive 64-block mortality window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MortalEra {
    birth: u64,
    death: u64,
}

impl MortalEra {
    #[must_use]
    pub const fn birth(self) -> u64 {
        self.birth
    }

    #[must_use]
    pub const fn death(self) -> u64 {
        self.death
    }
}

/// Exact finalized body coordinate for an included signed extrinsic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FinalizedExtrinsic {
    block_number: u64,
    block_hash: [u8; 32],
    extrinsic_index: u32,
    extrinsic_hash: [u8; 32],
}

impl FinalizedExtrinsic {
    #[must_use]
    pub const fn block_number(self) -> u64 {
        self.block_number
    }

    #[must_use]
    pub const fn block_hash(self) -> [u8; 32] {
        self.block_hash
    }

    #[must_use]
    pub const fn extrinsic_index(self) -> u32 {
        self.extrinsic_index
    }

    #[must_use]
    pub const fn extrinsic_hash(self) -> [u8; 32] {
        self.extrinsic_hash
    }
}

/// Stable accepted-event coordinate joined to the exact finalized extrinsic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcceptedCoordinate {
    finalized_extrinsic: FinalizedExtrinsic,
    system_event_index: u32,
    global_sequence: u64,
}

impl AcceptedCoordinate {
    #[must_use]
    pub const fn finalized_extrinsic(self) -> FinalizedExtrinsic {
        self.finalized_extrinsic
    }

    #[must_use]
    pub const fn system_event_index(self) -> u32 {
        self.system_event_index
    }

    #[must_use]
    pub const fn global_sequence(self) -> u64 {
        self.global_sequence
    }
}

/// Canonical effect proven by the one matching accepted event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AcceptedEffect {
    UnitCreated {
        unit_id: IntentUnitId,
        committed_revision: u64,
    },
    UnitTransitioned {
        unit_id: IntentUnitId,
        committed_revision: u64,
    },
    UnitCompleted {
        unit_id: IntentUnitId,
        committed_revision: u64,
    },
    RelationshipDefinitionCreated(RelationshipDefinition),
    RelationshipCreated(RelationshipIdentity),
    RelationshipDeleted(RelationshipIdentity),
    AssociationRecorded(RecordedAssociation),
    AssociationRevoked(RecordedAssociation),
}

/// Machine-readable outcome family.  The containing outcome is constructor-closed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmissionOutcomeKind {
    SubmissionRejected,
    SubmissionLaneUnresolved,
    ExpiredNotIncluded,
    FinalizedDispatchRejected,
    FinalizedInvariantFailed,
    DeliveryIndeterminate,
    FinalizedAccepted,
}

/// Closed preparation, delivery, and finalized-dispatch failure codes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmissionFailureCode {
    NonceConflict,
    InsufficientBalance,
    TransactionInvalid,
    RpcSubmissionRejected,
    SubmissionWatchLost,
    SubmissionTimeout,
    SubmissionLaneUnresolved,
    FinalizedInvariantFailed,
    ExpiredNotIncluded,
    RuntimeMismatch,
    UnsupportedCommandSchemaVersion,
    UnsignedCall,
    UnauthorizedSubmitter,
    DuplicateIntentUnit,
    IntentUnitNotFound,
    RevisionConflict,
    LifecycleHistoryCapacityExceeded,
    TransitionAlreadyCompleted,
    TransitionUnknownTarget,
    TransitionNotAllowed,
    CompletionAlreadyCompleted,
    CompletionPhaseNotEligible,
    GlobalSequenceExhausted,
    RelationshipDefinitionAlreadyExists,
    RelationshipDefinitionNotFound,
    RelationshipSourceNotFound,
    RelationshipTargetNotFound,
    RelationshipSourceSpeciesMismatch,
    RelationshipTargetSpeciesMismatch,
    SelfRelationshipRejected,
    DuplicateRelationship,
    CycleRejected,
    RelationshipCapacityExceeded,
    RelationshipNotFound,
    AssociationRevisionOutOfRange,
    DuplicateAssociation,
    AssociationCapacityExceeded,
    AssociationNotFound,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SubmissionOutcomeDetail {
    SubmissionRejected {
        operation: MutationOperation,
        error: SubmissionFailureCode,
    },
    SubmissionLaneUnresolved {
        operation: MutationOperation,
        expected_extrinsic_hash: [u8; 32],
        era: MortalEra,
    },
    ExpiredNotIncluded {
        operation: MutationOperation,
        expected_extrinsic_hash: [u8; 32],
        era: MortalEra,
    },
    FinalizedDispatchRejected {
        operation: MutationOperation,
        finalized_extrinsic: FinalizedExtrinsic,
        error: SubmissionFailureCode,
    },
    FinalizedInvariantFailed {
        operation: MutationOperation,
        finalized_extrinsic: FinalizedExtrinsic,
    },
    DeliveryIndeterminate {
        operation: MutationOperation,
        expected_extrinsic_hash: [u8; 32],
        era: MortalEra,
        error: SubmissionFailureCode,
    },
    FinalizedAccepted {
        operation: MutationOperation,
        coordinate: AcceptedCoordinate,
        effect: AcceptedEffect,
    },
}

/// Constructor-closed mutation outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmissionOutcome(SubmissionOutcomeDetail);

impl SubmissionOutcome {
    #[must_use]
    pub const fn kind(&self) -> SubmissionOutcomeKind {
        match self.0 {
            SubmissionOutcomeDetail::SubmissionRejected { .. } => {
                SubmissionOutcomeKind::SubmissionRejected
            }
            SubmissionOutcomeDetail::SubmissionLaneUnresolved { .. } => {
                SubmissionOutcomeKind::SubmissionLaneUnresolved
            }
            SubmissionOutcomeDetail::ExpiredNotIncluded { .. } => {
                SubmissionOutcomeKind::ExpiredNotIncluded
            }
            SubmissionOutcomeDetail::FinalizedDispatchRejected { .. } => {
                SubmissionOutcomeKind::FinalizedDispatchRejected
            }
            SubmissionOutcomeDetail::FinalizedInvariantFailed { .. } => {
                SubmissionOutcomeKind::FinalizedInvariantFailed
            }
            SubmissionOutcomeDetail::DeliveryIndeterminate { .. } => {
                SubmissionOutcomeKind::DeliveryIndeterminate
            }
            SubmissionOutcomeDetail::FinalizedAccepted { .. } => {
                SubmissionOutcomeKind::FinalizedAccepted
            }
        }
    }

    #[must_use]
    pub const fn operation(&self) -> MutationOperation {
        match self.0 {
            SubmissionOutcomeDetail::SubmissionRejected { operation, .. }
            | SubmissionOutcomeDetail::SubmissionLaneUnresolved { operation, .. }
            | SubmissionOutcomeDetail::ExpiredNotIncluded { operation, .. }
            | SubmissionOutcomeDetail::FinalizedDispatchRejected { operation, .. }
            | SubmissionOutcomeDetail::FinalizedInvariantFailed { operation, .. }
            | SubmissionOutcomeDetail::DeliveryIndeterminate { operation, .. }
            | SubmissionOutcomeDetail::FinalizedAccepted { operation, .. } => operation,
        }
    }

    #[must_use]
    pub const fn expected_extrinsic_hash(&self) -> Option<[u8; 32]> {
        match self.0 {
            SubmissionOutcomeDetail::SubmissionLaneUnresolved {
                expected_extrinsic_hash,
                ..
            }
            | SubmissionOutcomeDetail::ExpiredNotIncluded {
                expected_extrinsic_hash,
                ..
            }
            | SubmissionOutcomeDetail::DeliveryIndeterminate {
                expected_extrinsic_hash,
                ..
            } => Some(expected_extrinsic_hash),
            _ => None,
        }
    }

    #[must_use]
    pub const fn era(&self) -> Option<MortalEra> {
        match self.0 {
            SubmissionOutcomeDetail::SubmissionLaneUnresolved { era, .. }
            | SubmissionOutcomeDetail::ExpiredNotIncluded { era, .. }
            | SubmissionOutcomeDetail::DeliveryIndeterminate { era, .. } => Some(era),
            _ => None,
        }
    }

    #[must_use]
    pub const fn finalized_extrinsic(&self) -> Option<FinalizedExtrinsic> {
        match self.0 {
            SubmissionOutcomeDetail::FinalizedDispatchRejected {
                finalized_extrinsic,
                ..
            }
            | SubmissionOutcomeDetail::FinalizedInvariantFailed {
                finalized_extrinsic,
                ..
            } => Some(finalized_extrinsic),
            SubmissionOutcomeDetail::FinalizedAccepted { coordinate, .. } => {
                Some(coordinate.finalized_extrinsic)
            }
            _ => None,
        }
    }

    #[must_use]
    pub const fn coordinate(&self) -> Option<AcceptedCoordinate> {
        match self.0 {
            SubmissionOutcomeDetail::FinalizedAccepted { coordinate, .. } => Some(coordinate),
            _ => None,
        }
    }

    #[must_use]
    pub const fn effect(&self) -> Option<&AcceptedEffect> {
        match &self.0 {
            SubmissionOutcomeDetail::FinalizedAccepted { effect, .. } => Some(effect),
            _ => None,
        }
    }

    #[must_use]
    pub const fn failure_code(&self) -> Option<SubmissionFailureCode> {
        match self.0 {
            SubmissionOutcomeDetail::SubmissionRejected { error, .. }
            | SubmissionOutcomeDetail::FinalizedDispatchRejected { error, .. }
            | SubmissionOutcomeDetail::DeliveryIndeterminate { error, .. } => Some(error),
            SubmissionOutcomeDetail::SubmissionLaneUnresolved { .. } => {
                Some(SubmissionFailureCode::SubmissionLaneUnresolved)
            }
            SubmissionOutcomeDetail::ExpiredNotIncluded { .. } => {
                Some(SubmissionFailureCode::ExpiredNotIncluded)
            }
            SubmissionOutcomeDetail::FinalizedInvariantFailed { .. } => {
                Some(SubmissionFailureCode::FinalizedInvariantFailed)
            }
            SubmissionOutcomeDetail::FinalizedAccepted { .. } => None,
        }
    }
}

/// A resolved result retains the signer-lane lock and terminal journal until
/// the adapter confirms its response bytes are durable.
pub struct SubmissionResult {
    outcome: SubmissionOutcome,
    acknowledgement: Option<SignerLane>,
    delivery_source: Option<BoxError>,
}

impl fmt::Debug for SubmissionResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubmissionResult")
            .field("outcome", &self.outcome)
            .field("requires_acknowledgement", &self.acknowledgement.is_some())
            .field("has_delivery_source", &self.delivery_source.is_some())
            .finish()
    }
}

impl SubmissionResult {
    #[must_use]
    pub const fn outcome(&self) -> &SubmissionOutcome {
        &self.outcome
    }

    #[must_use]
    pub const fn requires_acknowledgement(&self) -> bool {
        self.acknowledgement.is_some()
    }

    /// Returns the original post-send transport/watcher source, when one was
    /// available.  The modeled outcome remains delivery-indeterminate.
    #[must_use]
    pub fn delivery_source(&self) -> Option<&(dyn Error + 'static)> {
        self.delivery_source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }

    /// Removes a durably resolved record only after the caller has durably
    /// completed its semantic response.  This consumes the held lane guard.
    pub fn acknowledge_response_durable(mut self) -> Result<(), SubmissionError> {
        let lane = self.acknowledgement.take().ok_or_else(|| {
            SubmissionError::without_source(
                SubmissionErrorKind::AcknowledgementUnavailable,
                "submission outcome has no resolved journal to acknowledge",
            )
        })?;
        lane.acknowledge_resolved().map_err(journal_error)
    }
}

/// High-level non-delivery error class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmissionErrorKind {
    UnsupportedPlatform,
    InsecureProjectionPath,
    SubmissionLaneCorrupt,
    ArchiveRpcUnavailable,
    ArchiveHistoryUnavailable,
    DeploymentMismatch,
    RuntimeMismatch,
    DevSignerUnavailable,
    ArithmeticOverflow,
    AcknowledgementUnavailable,
}

/// Source-preserving failure outside the seven mutation delivery outcomes.
#[derive(Debug)]
pub struct SubmissionError {
    kind: SubmissionErrorKind,
    message: Cow<'static, str>,
    source: Option<BoxError>,
}

impl SubmissionError {
    fn without_source(kind: SubmissionErrorKind, message: &'static str) -> Self {
        Self {
            kind,
            message: Cow::Borrowed(message),
            source: None,
        }
    }

    fn with_source(
        kind: SubmissionErrorKind,
        message: impl Into<Cow<'static, str>>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> SubmissionErrorKind {
        self.kind
    }
}

impl fmt::Display for SubmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SubmissionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

/// Submits one mutation and waits at most 120 seconds for watcher finalization.
/// No path, nonce, raw call, RPC response, or signer key can be injected.
pub async fn submit_finalized(
    client: &VerifiedArchiveClient,
    projection_directory: &Path,
    signer: DevSigner,
    mutation: Mutation,
) -> Result<SubmissionResult, SubmissionError> {
    let chain = RealSubmissionChain::connect(client).await?;
    submit_with_chain(
        &chain,
        projection_directory,
        signer,
        mutation,
        DEFAULT_FINALITY_WAIT,
    )
    .await
}

// The runtime metadata advertises these nine extensions in this exact order.
// Three carry unit value/implicit types, but they are still encoded through
// their metadata type IDs so a future non-unit shape fails rather than being
// silently accepted.
#[derive(Clone, Debug)]
struct CheckNonZeroSender;

#[derive(Clone, Debug)]
struct CheckWeight;

#[derive(Clone, Debug)]
struct StorageWeightReclaim;

macro_rules! unit_extension {
    ($type:ty, $name:literal) => {
        impl TransactionExtension<CubiKanConfig> for $type {
            type Decoded = ();
            type Params = ();

            fn new(
                _client: &ClientState<CubiKanConfig>,
                _params: Self::Params,
            ) -> Result<Self, subxt::error::TransactionExtensionError> {
                Ok(Self)
            }
        }

        impl frame_decode::extrinsics::TransactionExtension<scale_info::PortableRegistry>
            for $type
        {
            const NAME: &str = $name;

            fn encode_value_to(
                &self,
                type_id: u32,
                types: &scale_info::PortableRegistry,
                output: &mut Vec<u8>,
            ) -> Result<(), frame_decode::extrinsics::TransactionExtensionError> {
                ().encode_as_type_to(type_id, types, output)
                    .map_err(|error| Box::new(error) as BoxError)
            }

            fn encode_implicit_to(
                &self,
                type_id: u32,
                types: &scale_info::PortableRegistry,
                output: &mut Vec<u8>,
            ) -> Result<(), frame_decode::extrinsics::TransactionExtensionError> {
                ().encode_as_type_to(type_id, types, output)
                    .map_err(|error| Box::new(error) as BoxError)
            }
        }
    };
}

unit_extension!(CheckNonZeroSender, "CheckNonZeroSender");
unit_extension!(CheckWeight, "CheckWeight");
unit_extension!(StorageWeightReclaim, "StorageWeightReclaim");

type ExactTransactionExtensions = (
    CheckNonZeroSender,
    CheckSpecVersion,
    CheckTxVersion,
    CheckGenesis<CubiKanConfig>,
    CheckMortality<CubiKanConfig>,
    CheckNonce,
    CheckWeight,
    ChargeTransactionPayment,
    StorageWeightReclaim,
);

type ExactTransactionParams = (
    (),
    (),
    (),
    (),
    CheckMortalityParams<CubiKanConfig>,
    CheckNonceParams,
    (),
    ChargeTransactionPaymentParams,
    (),
);

/// A configuration with the exact committed CubiKan runtime extension set.
#[derive(Clone, Debug)]
struct CubiKanConfig(SubstrateConfig);

impl Config for CubiKanConfig {
    type AccountId = <SubstrateConfig as Config>::AccountId;
    type Address = <SubstrateConfig as Config>::Address;
    type Signature = <SubstrateConfig as Config>::Signature;
    type Header = <SubstrateConfig as Config>::Header;
    type TransactionExtensions = ExactTransactionExtensions;
    type AssetId = <SubstrateConfig as Config>::AssetId;
    type Hasher = <SubstrateConfig as Config>::Hasher;

    fn genesis_hash(&self) -> Option<HashFor<Self>> {
        <SubstrateConfig as Config>::genesis_hash(&self.0)
    }

    fn spec_and_transaction_version_for_block_number(
        &self,
        block_number: u64,
    ) -> Option<(u32, u32)> {
        <SubstrateConfig as Config>::spec_and_transaction_version_for_block_number(
            &self.0,
            block_number,
        )
    }

    fn metadata_for_spec_version(&self, spec_version: u32) -> Option<subxt::ArcMetadata> {
        <SubstrateConfig as Config>::metadata_for_spec_version(&self.0, spec_version)
    }

    fn set_metadata_for_spec_version(&self, spec_version: u32, metadata: subxt::ArcMetadata) {
        <SubstrateConfig as Config>::set_metadata_for_spec_version(&self.0, spec_version, metadata);
    }
}

fn offline_client(
    identity: &ChainIdentity,
) -> Result<OfflineClient<CubiKanConfig>, SubmissionError> {
    let metadata = pinned_metadata()?;
    let substrate = SubstrateConfig::builder()
        .set_genesis_hash(H256::from(identity.genesis_hash))
        .set_spec_version_for_block_ranges([SpecVersionForRange {
            block_range: 0..u64::MAX,
            spec_version: identity.spec_version,
            transaction_version: identity.transaction_version,
        }])
        .set_metadata_for_spec_versions([(identity.spec_version, Arc::new(metadata))])
        .build();
    Ok(OfflineClient::new_with_config(CubiKanConfig(substrate)))
}

fn pinned_metadata() -> Result<Metadata, SubmissionError> {
    let metadata = Metadata::decode_from(metadata_bytes()).map_err(|error| {
        SubmissionError::with_source(
            SubmissionErrorKind::RuntimeMismatch,
            "pinned runtime metadata is not decodable",
            error,
        )
    })?;
    assert_exact_extension_inventory(&metadata)?;
    Ok(metadata)
}

fn assert_exact_extension_inventory(metadata: &Metadata) -> Result<(), SubmissionError> {
    let actual: Vec<_> = metadata
        .extrinsic()
        .transaction_extensions_to_use_for_encoding()
        .map(|extension| extension.identifier())
        .collect();
    if actual == EXACT_EXTENSION_NAMES {
        Ok(())
    } else {
        Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "runtime transaction-extension inventory differs from the pinned CubiKan contract",
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ChainIdentity {
    deployment_id: [u8; 32],
    genesis_hash: [u8; 32],
    spec_version: u32,
    transaction_version: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ChainHead {
    number: u64,
    hash: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SubmissionBlock {
    number: u64,
    hash: [u8; 32],
    raw_extrinsics: Vec<Vec<u8>>,
    raw_system_events: Vec<u8>,
    extrinsic_hashes: Vec<[u8; 32]>,
    accepted_events: Vec<AcceptedEvent>,
}

#[derive(Debug)]
struct FinalizedDispatchEvidence {
    successes: usize,
    errors: Vec<DecodedDispatchFailure>,
}

#[derive(Debug)]
enum DecodedDispatchFailure {
    Known(DispatchError),
    // The bytes decoded exactly as the pinned metadata's DispatchError, but
    // Subxt's convenience enum does not yet represent that runtime variant.
    RuntimeMismatch,
}

impl From<FinalizedBlock> for SubmissionBlock {
    fn from(block: FinalizedBlock) -> Self {
        Self {
            number: block.number(),
            hash: *block.hash(),
            raw_extrinsics: block.raw_extrinsics().to_vec(),
            raw_system_events: block.raw_system_events().to_vec(),
            extrinsic_hashes: block.extrinsic_hashes().to_vec(),
            accepted_events: block.events().to_vec(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DryRunOutcome {
    Valid,
    Invalid(SubmissionFailureCode),
}

#[derive(Debug)]
enum WatchOutcome {
    Finalized([u8; 32]),
    RpcRejected(ChainFailure),
    Lost(ChainFailure),
    Timeout,
}

#[derive(Debug)]
struct ChainFailure {
    operation: &'static str,
    source: BoxError,
}

impl ChainFailure {
    fn new(operation: &'static str, source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            operation,
            source: Box::new(source),
        }
    }
}

impl fmt::Display for ChainFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.operation, self.source)
    }
}

impl Error for ChainFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

trait SubmissionChain: Send + Sync {
    fn identity(&self) -> ChainIdentity;
    fn finalized_head(&self) -> ChainFuture<'_, ChainHead>;
    fn finalized_block(&self, number: u64) -> ChainFuture<'_, SubmissionBlock>;
    fn finalized_block_by_hash(&self, hash: [u8; 32]) -> ChainFuture<'_, SubmissionBlock>;
    fn finalized_dispatch_evidence<'a>(
        &'a self,
        block: &'a SubmissionBlock,
        extrinsic_index: u32,
    ) -> ChainFuture<'a, FinalizedDispatchEvidence>;
    fn storage(&self, key: Vec<u8>, at: [u8; 32]) -> ChainFuture<'_, Option<Vec<u8>>>;
    fn dry_run(&self, extrinsic: Vec<u8>, at: [u8; 32]) -> ChainFuture<'_, DryRunOutcome>;
    fn submit_and_watch(
        &self,
        extrinsic: Vec<u8>,
        timeout: Duration,
    ) -> ChainFuture<'_, WatchOutcome>;
}

struct RealSubmissionChain<'a> {
    verified: &'a VerifiedArchiveClient,
    rpc: LegacyRpcMethods<RpcConfigFor<CubiKanConfig>>,
    metadata: subxt::ArcMetadata,
    identity: ChainIdentity,
}

impl<'a> RealSubmissionChain<'a> {
    async fn connect(verified: &'a VerifiedArchiveClient) -> Result<Self, SubmissionError> {
        let rpc_client = RpcClient::from_insecure_url(verified.endpoint().as_str())
            .await
            .map_err(|error| {
                SubmissionError::with_source(
                    SubmissionErrorKind::ArchiveRpcUnavailable,
                    "connect verified submission RPC",
                    error,
                )
            })?;
        let deployment = verified.identity();
        let identity = ChainIdentity {
            deployment_id: *deployment.deployment_id(),
            genesis_hash: *deployment.parachain_genesis_hash(),
            spec_version: deployment.runtime_spec_version(),
            transaction_version: deployment.runtime_transaction_version(),
        };
        let metadata = Arc::new(pinned_metadata()?);
        let _ = offline_client(&identity)?;
        Ok(Self {
            verified,
            rpc: LegacyRpcMethods::new(rpc_client),
            metadata,
            identity,
        })
    }

    async fn verified_block(&self, number: u64) -> Result<SubmissionBlock, ChainFailure> {
        let head = self
            .verified
            .finalized_head()
            .await
            .map_err(|error| ChainFailure::new("read finalized head", error))?;
        self.verified
            .finalized_block(&head, number)
            .await
            .map(SubmissionBlock::from)
            .map_err(|error| ChainFailure::new("read finalized block", error))
    }
}

impl SubmissionChain for RealSubmissionChain<'_> {
    fn identity(&self) -> ChainIdentity {
        self.identity
    }

    fn finalized_head(&self) -> ChainFuture<'_, ChainHead> {
        Box::pin(async move {
            let head = self
                .verified
                .finalized_head()
                .await
                .map_err(|error| ChainFailure::new("read finalized head", error))?;
            Ok(ChainHead {
                number: head.number(),
                hash: *head.hash(),
            })
        })
    }

    fn finalized_block(&self, number: u64) -> ChainFuture<'_, SubmissionBlock> {
        Box::pin(self.verified_block(number))
    }

    fn finalized_block_by_hash(&self, hash: [u8; 32]) -> ChainFuture<'_, SubmissionBlock> {
        Box::pin(async move {
            let header = self
                .rpc
                .chain_get_header(Some(H256::from(hash)))
                .await
                .map_err(|error| ChainFailure::new("read finalized inclusion header", error))?
                .ok_or_else(|| {
                    ChainFailure::new(
                        "read finalized inclusion header",
                        StaticFailure("block header is unavailable"),
                    )
                })?;
            let block = self.verified_block(header.number).await?;
            if block.hash != hash {
                return Err(ChainFailure::new(
                    "verify finalized inclusion hash",
                    StaticFailure("watcher hash is not canonical finalized history"),
                ));
            }
            Ok(block)
        })
    }

    fn finalized_dispatch_evidence<'a>(
        &'a self,
        block: &'a SubmissionBlock,
        extrinsic_index: u32,
    ) -> ChainFuture<'a, FinalizedDispatchEvidence> {
        Box::pin(async move {
            decode_finalized_dispatch_evidence(self.identity, block, extrinsic_index)
        })
    }

    fn storage(&self, key: Vec<u8>, at: [u8; 32]) -> ChainFuture<'_, Option<Vec<u8>>> {
        Box::pin(async move {
            self.rpc
                .state_get_storage(&key, Some(H256::from(at)))
                .await
                .map_err(|error| ChainFailure::new("read exact finalized account storage", error))
        })
    }

    fn dry_run(&self, extrinsic: Vec<u8>, at: [u8; 32]) -> ChainFuture<'_, DryRunOutcome> {
        Box::pin(async move {
            let response = self
                .rpc
                .dry_run(&extrinsic, Some(H256::from(at)))
                .await
                .map_err(|error| ChainFailure::new("dry-run signed extrinsic", error))?;
            decode_dry_run(&response.0, self.metadata.clone())
                .map_err(|error| ChainFailure::new("decode dry-run", error))
        })
    }

    fn submit_and_watch(
        &self,
        extrinsic: Vec<u8>,
        timeout: Duration,
    ) -> ChainFuture<'_, WatchOutcome> {
        Box::pin(async move {
            let watch = async {
                let mut subscription =
                    match self.rpc.author_submit_and_watch_extrinsic(&extrinsic).await {
                        Ok(subscription) => subscription,
                        Err(error) => {
                            return WatchOutcome::RpcRejected(ChainFailure::new(
                                "invoke submit_and_watch",
                                error,
                            ));
                        }
                    };
                while let Some(status) = subscription.next().await {
                    let status = match status {
                        Ok(status) => status,
                        Err(error) => {
                            return WatchOutcome::Lost(ChainFailure::new(
                                "read submission watcher",
                                error,
                            ));
                        }
                    };
                    if let Some(outcome) = classify_watcher_status(status) {
                        return outcome;
                    }
                }
                watcher_stream_ended()
            };
            match tokio::time::timeout(timeout, watch).await {
                Ok(outcome) => Ok(outcome),
                Err(_) => Ok(WatchOutcome::Timeout),
            }
        })
    }
}

fn decode_finalized_dispatch_evidence(
    identity: ChainIdentity,
    block: &SubmissionBlock,
    extrinsic_index: u32,
) -> Result<FinalizedDispatchEvidence, ChainFailure> {
    let offline = offline_client(&identity)
        .map_err(|error| ChainFailure::new("construct pinned finalized-event decoder", error))?;
    let at = offline
        .at_block(block.number)
        .map_err(|error| ChainFailure::new("construct exact finalized-event decoder", error))?;
    let events_info = at
        .metadata_ref()
        .storage_info("System", "Events")
        .map_err(|error| {
            ChainFailure::new(
                "resolve System::Events storage metadata",
                error.into_owned(),
            )
        })?;

    // `Events::from_bytes` intentionally provides a streaming view. Validate
    // the complete storage value first so a valid prefix followed by garbage
    // cannot become authoritative finalized dispatch evidence.
    let mut complete_input = block.raw_system_events.as_slice();
    decode_as_type(
        &mut complete_input,
        events_info.value_id,
        at.metadata_ref().types(),
    )
    .map_err(|error| ChainFailure::new("decode exact System::Events storage value", error))?;
    if !complete_input.is_empty() {
        return Err(ChainFailure::new(
            "decode exact System::Events storage value",
            StaticFailure("System::Events storage has trailing bytes"),
        ));
    }

    let events = at.events().from_bytes(block.raw_system_events.clone());
    let metadata = at.metadata();
    let dispatch_error_ty = metadata.dispatch_error_ty().ok_or_else(|| {
        ChainFailure::new(
            "resolve pinned DispatchError metadata",
            StaticFailure("pinned metadata has no DispatchError type"),
        )
    })?;
    let mut evidence = FinalizedDispatchEvidence {
        successes: 0,
        errors: Vec::new(),
    };

    for event in events.iter() {
        let event = event
            .map_err(|error| ChainFailure::new("decode finalized System::Events entry", error))?;
        if event.phase() != Phase::ApplyExtrinsic(extrinsic_index)
            || event.pallet_name() != "System"
        {
            continue;
        }

        match event.event_name() {
            "ExtrinsicSuccess" => {
                evidence.successes += 1;
            }
            "ExtrinsicFailed" => {
                let variant = metadata
                    .pallet_by_event_index(event.pallet_index())
                    .and_then(|pallet| pallet.event_variant_by_index(event.event_index()))
                    .ok_or_else(|| {
                        ChainFailure::new(
                            "resolve System::ExtrinsicFailed metadata",
                            StaticFailure("decoded event index is absent from pinned metadata"),
                        )
                    })?;
                let [dispatch_field, _dispatch_info_field] = variant.fields.as_slice() else {
                    return Err(ChainFailure::new(
                        "validate System::ExtrinsicFailed metadata",
                        StaticFailure("ExtrinsicFailed does not have exactly two fields"),
                    ));
                };
                if dispatch_field.ty.id != dispatch_error_ty {
                    return Err(ChainFailure::new(
                        "validate System::ExtrinsicFailed metadata",
                        StaticFailure("ExtrinsicFailed first field is not DispatchError"),
                    ));
                }

                let fields = event.field_bytes();
                let mut trailing = fields;
                decode_as_type(&mut trailing, dispatch_field.ty.id, metadata.types()).map_err(
                    |error| ChainFailure::new("decode finalized DispatchError boundary", error),
                )?;
                let dispatch_len = fields.len().checked_sub(trailing.len()).ok_or_else(|| {
                    ChainFailure::new(
                        "decode finalized DispatchError boundary",
                        StaticFailure("DispatchError field boundary underflow"),
                    )
                })?;
                let dispatch_bytes = &fields[..dispatch_len];
                let dispatch = match DispatchError::decode_from(dispatch_bytes, metadata.clone()) {
                    Ok(dispatch) => DecodedDispatchFailure::Known(dispatch),
                    Err(_) => DecodedDispatchFailure::RuntimeMismatch,
                };
                evidence.errors.push(dispatch);
            }
            _ => {}
        }
    }
    Ok(evidence)
}

fn classify_watcher_status(
    status: subxt::rpcs::methods::legacy::TransactionStatus<H256>,
) -> Option<WatchOutcome> {
    use subxt::rpcs::methods::legacy::TransactionStatus;

    match status {
        TransactionStatus::Finalized(hash) => Some(WatchOutcome::Finalized(hash.into())),
        TransactionStatus::Invalid
        | TransactionStatus::Dropped
        | TransactionStatus::Usurped(_)
        | TransactionStatus::FinalityTimeout(_) => Some(WatchOutcome::Lost(ChainFailure::new(
            "read submission watcher",
            StaticFailure("watcher reported a non-proving terminal status"),
        ))),
        TransactionStatus::Future
        | TransactionStatus::Ready
        | TransactionStatus::Broadcast(_)
        | TransactionStatus::InBlock(_)
        | TransactionStatus::Retracted(_) => None,
    }
}

fn watcher_stream_ended() -> WatchOutcome {
    WatchOutcome::Lost(ChainFailure::new(
        "read submission watcher",
        StaticFailure("watcher stream ended before finalization"),
    ))
}

#[derive(Debug)]
struct StaticFailure(&'static str);

impl fmt::Display for StaticFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl Error for StaticFailure {}

#[derive(Clone, Debug)]
struct PreparedSubmission {
    record: JournalRecord,
    encoded: Vec<u8>,
}

async fn submit_with_chain(
    chain: &dyn SubmissionChain,
    projection_directory: &Path,
    signer: DevSigner,
    mutation: Mutation,
    wait: Duration,
) -> Result<SubmissionResult, SubmissionError> {
    if wait > DEFAULT_FINALITY_WAIT {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "submission watcher deadline exceeds 120 seconds",
        ));
    }
    let identity = chain.identity();
    let signer_id = signer.account_id();
    let mut lane = SignerLane::open(projection_directory, identity.deployment_id, signer_id)
        .map_err(journal_error)?;

    if let Some(record) = lane.record().cloned() {
        return reconcile_existing(chain, lane, record).await;
    }

    let operation = mutation.operation();
    let head = chain
        .finalized_head()
        .await
        .map_err(chain_error_before_send)?;
    let prepared = prepare_submission(chain, signer, &mutation, head).await?;
    match chain
        .dry_run(prepared.encoded.clone(), head.hash)
        .await
        .map_err(chain_error_before_send)?
    {
        DryRunOutcome::Valid => {}
        DryRunOutcome::Invalid(error) => {
            return Ok(result_without_ack(
                SubmissionOutcomeDetail::SubmissionRejected { operation, error },
            ));
        }
    }

    lane.publish_prepared(prepared.record.clone())
        .map_err(journal_error)?;
    let era = record_era(&prepared.record);
    let expected_extrinsic_hash = *prepared.record.extrinsic_hash();
    match chain.submit_and_watch(prepared.encoded, wait).await {
        Ok(WatchOutcome::Finalized(hash)) => {
            match resolve_finalized(chain, &prepared.record, hash).await {
                Ok((resolved, outcome)) => {
                    lane.publish_resolved(resolved).map_err(journal_error)?;
                    Ok(result_with_ack(outcome, lane))
                }
                Err(source) => Ok(result_indeterminate(
                    operation,
                    expected_extrinsic_hash,
                    era,
                    SubmissionFailureCode::SubmissionWatchLost,
                    Some(Box::new(source)),
                )),
            }
        }
        Ok(WatchOutcome::RpcRejected(source)) => Ok(result_indeterminate(
            operation,
            expected_extrinsic_hash,
            era,
            SubmissionFailureCode::RpcSubmissionRejected,
            Some(Box::new(source)),
        )),
        Ok(WatchOutcome::Lost(source)) => Ok(result_indeterminate(
            operation,
            expected_extrinsic_hash,
            era,
            SubmissionFailureCode::SubmissionWatchLost,
            Some(Box::new(source)),
        )),
        Ok(WatchOutcome::Timeout) => Ok(result_indeterminate(
            operation,
            expected_extrinsic_hash,
            era,
            SubmissionFailureCode::SubmissionTimeout,
            None,
        )),
        Err(source) => Ok(result_indeterminate(
            operation,
            expected_extrinsic_hash,
            era,
            SubmissionFailureCode::SubmissionWatchLost,
            Some(Box::new(source)),
        )),
    }
}

fn result_without_ack(detail: SubmissionOutcomeDetail) -> SubmissionResult {
    SubmissionResult {
        outcome: SubmissionOutcome(detail),
        acknowledgement: None,
        delivery_source: None,
    }
}

fn result_with_ack(detail: SubmissionOutcomeDetail, lane: SignerLane) -> SubmissionResult {
    SubmissionResult {
        outcome: SubmissionOutcome(detail),
        acknowledgement: Some(lane),
        delivery_source: None,
    }
}

fn result_indeterminate(
    operation: MutationOperation,
    expected_extrinsic_hash: [u8; 32],
    era: MortalEra,
    error: SubmissionFailureCode,
    delivery_source: Option<BoxError>,
) -> SubmissionResult {
    SubmissionResult {
        outcome: SubmissionOutcome(SubmissionOutcomeDetail::DeliveryIndeterminate {
            operation,
            expected_extrinsic_hash,
            era,
            error,
        }),
        acknowledgement: None,
        delivery_source,
    }
}

async fn prepare_submission(
    chain: &dyn SubmissionChain,
    signer: DevSigner,
    mutation: &Mutation,
    head: ChainHead,
) -> Result<PreparedSubmission, SubmissionError> {
    let identity = chain.identity();
    let offline = offline_client(&identity)?;
    let at = offline.at_block(head.number).map_err(|error| {
        SubmissionError::with_source(
            SubmissionErrorKind::RuntimeMismatch,
            "instantiate pinned offline client at signing block",
            error,
        )
    })?;
    let signer_keypair = signer.keypair();
    let signer_id = signer.account_id();
    let account_key = system_account_key(&at, signer_id)?;
    let account_bytes = chain
        .storage(account_key, head.hash)
        .await
        .map_err(chain_error_before_send)?
        .ok_or_else(|| {
            SubmissionError::without_source(
                SubmissionErrorKind::DevSignerUnavailable,
                "authorized development signer account is absent at the signing block",
            )
        })?;
    let nonce = decode_account_nonce(&at, &account_bytes)?;

    let call = dynamic_call(mutation);
    let params: ExactTransactionParams = (
        (),
        (),
        (),
        (),
        CheckMortalityParams::mortal_from_unchecked(
            MORTAL_PERIOD,
            head.number,
            H256::from(head.hash),
        ),
        CheckNonceParams::with_nonce(nonce),
        (),
        ChargeTransactionPaymentParams::no_tip(),
        (),
    );
    let mut signable = at
        .transactions()
        .create_signable_offline(&call, params)
        .map_err(|error| {
            SubmissionError::with_source(
                SubmissionErrorKind::RuntimeMismatch,
                "create exact offline signable transaction",
                error,
            )
        })?;
    let actual_payload = signable.signer_payload().map_err(|error| {
        SubmissionError::with_source(
            SubmissionErrorKind::RuntimeMismatch,
            "encode offline signer payload",
            error,
        )
    })?;
    let expected_payload = expected_signer_payload(&at, mutation, nonce, head)?;
    if actual_payload != expected_payload {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "Subxt signer payload differs from the independently encoded pinned payload",
        ));
    }

    let signature = signer_keypair.sign(&actual_payload);
    if !sr25519::verify(&signature, &actual_payload, &signer_keypair.public_key()) {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::DevSignerUnavailable,
            "development signer failed immediate signature verification",
        ));
    }
    let account_id = signer_keypair.public_key().to_account_id();
    let signed = signable
        .sign_with_account_and_signature(&account_id, &MultiSignature::Sr25519(signature.0))
        .map_err(|error| {
            SubmissionError::with_source(
                SubmissionErrorKind::RuntimeMismatch,
                "encode signed extrinsic",
                error,
            )
        })?;
    let encoded = signed.into_encoded();
    let extrinsic_hash: [u8; 32] = at.hasher().hash(&encoded).into();
    inspect_signed_extrinsic(
        &offline,
        head.number,
        &encoded,
        signer_id,
        Some(signature.0),
        nonce,
        head.number,
        mutation.operation(),
        Some(&manual_call_data(mutation)?),
    )
    .await?;
    let record = JournalRecord::prepared(
        identity.deployment_id,
        signer_id,
        nonce,
        extrinsic_hash,
        head.number,
        head.hash,
        mutation.operation().journal(),
    )
    .map_err(journal_error)?;
    Ok(PreparedSubmission { record, encoded })
}

fn system_account_key(
    at: &subxt::OfflineClientAtBlock<CubiKanConfig>,
    signer: [u8; 32],
) -> Result<Vec<u8>, SubmissionError> {
    let address = storage::<Vec<Value>, Value>("System", "Account");
    let storage_client = at.storage();
    let entry = storage_client.entry(address).map_err(|error| {
        SubmissionError::with_source(
            SubmissionErrorKind::RuntimeMismatch,
            "resolve System::Account storage metadata",
            error,
        )
    })?;
    entry
        .fetch_key(vec![Value::from_bytes(signer)])
        .map_err(|error| {
            SubmissionError::with_source(
                SubmissionErrorKind::RuntimeMismatch,
                "encode System::Account storage key",
                error,
            )
        })
}

fn decode_account_nonce(
    at: &subxt::OfflineClientAtBlock<CubiKanConfig>,
    bytes: &[u8],
) -> Result<u64, SubmissionError> {
    let info = at
        .metadata_ref()
        .storage_info("System", "Account")
        .map_err(|error| {
            SubmissionError::with_source(
                SubmissionErrorKind::RuntimeMismatch,
                "resolve System::Account value metadata",
                error.into_owned(),
            )
        })?;
    let mut input = bytes;
    let value =
        decode_as_type(&mut input, info.value_id, at.metadata_ref().types()).map_err(|error| {
            SubmissionError::with_source(
                SubmissionErrorKind::RuntimeMismatch,
                "decode System::Account at exact finalized hash",
                error,
            )
        })?;
    if !input.is_empty() {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "System::Account storage has trailing bytes",
        ));
    }
    let nonce = value
        .at("nonce")
        .and_then(|nonce| nonce.as_u128())
        .and_then(|nonce| u32::try_from(nonce).ok())
        .ok_or_else(|| {
            SubmissionError::without_source(
                SubmissionErrorKind::RuntimeMismatch,
                "System::Account nonce is not the pinned u32 field",
            )
        })?;
    Ok(u64::from(nonce))
}

fn expected_signer_payload(
    at: &subxt::OfflineClientAtBlock<CubiKanConfig>,
    mutation: &Mutation,
    nonce: u64,
    head: ChainHead,
) -> Result<Vec<u8>, SubmissionError> {
    let mut expected = manual_call_data(mutation)?;
    Era::mortal(MORTAL_PERIOD, head.number).encode_to(&mut expected);
    Compact(nonce).encode_to(&mut expected);
    Compact(0_u128).encode_to(&mut expected);
    at.spec_version().encode_to(&mut expected);
    at.transaction_version().encode_to(&mut expected);
    at.genesis_hash()
        .ok_or_else(|| {
            SubmissionError::without_source(
                SubmissionErrorKind::RuntimeMismatch,
                "offline signing client has no genesis hash",
            )
        })?
        .encode_to(&mut expected);
    H256::from(head.hash).encode_to(&mut expected);
    if expected.len() > 256 {
        Ok(at.hasher().hash(&expected).as_ref().to_vec())
    } else {
        Ok(expected)
    }
}

fn dynamic_call(mutation: &Mutation) -> subxt::transactions::DynamicPayload<Vec<Value>> {
    let schema = Value::u128(u128::from(COMMAND_SCHEMA_VERSION));
    match mutation {
        Mutation::CreateUnit {
            id,
            origin,
            species,
            workflow,
        } => tx(
            CUBIKAN_PALLET,
            "create_unit",
            vec![
                schema,
                unit_id_value(*id),
                reference_value(origin),
                text_value(species.as_str()),
                workflow_value(workflow),
            ],
        ),
        Mutation::TransitionUnit {
            id,
            target,
            expected_revision,
        } => tx(
            CUBIKAN_PALLET,
            "transition_unit",
            vec![
                schema,
                unit_id_value(*id),
                text_value(target.as_str()),
                Value::u128(u128::from(*expected_revision)),
            ],
        ),
        Mutation::CompleteUnit {
            id,
            expected_revision,
        } => tx(
            CUBIKAN_PALLET,
            "complete_unit",
            vec![
                schema,
                unit_id_value(*id),
                Value::u128(u128::from(*expected_revision)),
            ],
        ),
        Mutation::CreateRelationshipDefinition(definition) => tx(
            CUBIKAN_PALLET,
            "create_relationship_definition",
            vec![schema, definition_value(definition)],
        ),
        Mutation::CreateRelationship(relationship) => tx(
            CUBIKAN_PALLET,
            "create_relationship",
            vec![schema, relationship_value(relationship)],
        ),
        Mutation::DeleteRelationship(relationship) => tx(
            CUBIKAN_PALLET,
            "delete_relationship",
            vec![schema, relationship_value(relationship)],
        ),
        Mutation::RecordAssociation(association) => tx(
            CUBIKAN_PALLET,
            "record_association",
            vec![schema, association_value(association)],
        ),
        Mutation::RevokeAssociation(association) => tx(
            CUBIKAN_PALLET,
            "revoke_association",
            vec![schema, association_value(association)],
        ),
    }
}

fn unit_id_value(id: IntentUnitId) -> Value {
    Value::from_bytes(id.as_uuid().as_bytes())
}

fn text_value(value: &str) -> Value {
    Value::from_bytes(value.as_bytes())
}

fn reference_value(reference: &ExternalReference) -> Value {
    Value::unnamed_composite([
        text_value(reference.namespace().as_str()),
        text_value(reference.scope().as_str()),
        text_value(reference.value().as_str()),
    ])
}

fn workflow_value(workflow: &Workflow) -> Value {
    Value::unnamed_composite([
        text_value(workflow.id().as_str()),
        Value::unnamed_composite(
            workflow
                .phases()
                .iter()
                .map(|phase| text_value(phase.as_str())),
        ),
        text_value(workflow.initial_phase().as_str()),
        Value::unnamed_composite(workflow.edges().iter().map(|edge| {
            Value::unnamed_composite([
                text_value(edge.from().as_str()),
                text_value(edge.to().as_str()),
            ])
        })),
        Value::unnamed_composite(
            workflow
                .completion_phases()
                .iter()
                .map(|phase| text_value(phase.as_str())),
        ),
    ])
}

fn option_text_value(value: Option<&IntentSpecies>) -> Value {
    match value {
        Some(value) => Value::unnamed_variant("Some", [text_value(value.as_str())]),
        None => Value::unnamed_variant("None", []),
    }
}

fn policy_value(value: RelationshipPolicy) -> Value {
    Value::unnamed_variant(
        match value {
            RelationshipPolicy::Allow => "Allow",
            RelationshipPolicy::Reject => "Reject",
        },
        [],
    )
}

fn definition_key_value(key: &cubikan_core::RelationshipDefinitionKey) -> Value {
    Value::unnamed_composite([
        text_value(key.id().as_str()),
        Value::u128(u128::from(key.version().value())),
    ])
}

fn definition_value(definition: &RelationshipDefinition) -> Value {
    Value::unnamed_composite([
        definition_key_value(definition.key()),
        Value::unnamed_variant("Directed", []),
        option_text_value(definition.source_species()),
        option_text_value(definition.target_species()),
        policy_value(definition.self_policy()),
        policy_value(definition.cycle_policy()),
    ])
}

fn relationship_value(relationship: &RelationshipIdentity) -> Value {
    Value::unnamed_composite([
        definition_key_value(relationship.definition()),
        unit_id_value(relationship.source()),
        unit_id_value(relationship.target()),
    ])
}

fn association_value(association: &RecordedAssociation) -> Value {
    let subject = match association.subject() {
        AssociationSubject::WholeUnit => Value::unnamed_variant("WholeUnit", []),
        AssociationSubject::Revision(revision) => {
            Value::unnamed_variant("Revision", [Value::u128(u128::from(revision))])
        }
    };
    Value::unnamed_composite([
        unit_id_value(association.unit_id()),
        subject,
        reference_value(association.reference()),
    ])
}

fn manual_call_data(mutation: &Mutation) -> Result<Vec<u8>, SubmissionError> {
    let mut bytes = Vec::new();
    bytes.push(50);
    bytes.push(match mutation.operation() {
        MutationOperation::CreateUnit => 0,
        MutationOperation::TransitionUnit => 1,
        MutationOperation::CompleteUnit => 2,
        MutationOperation::CreateRelationshipDefinition => 4,
        MutationOperation::CreateRelationship => 5,
        MutationOperation::DeleteRelationship => 6,
        MutationOperation::RecordAssociation => 7,
        MutationOperation::RevokeAssociation => 8,
    });
    COMMAND_SCHEMA_VERSION.encode_to(&mut bytes);
    match mutation {
        Mutation::CreateUnit {
            id,
            origin,
            species,
            workflow,
        } => {
            encode_unit_id(*id, &mut bytes);
            encode_reference(origin, &mut bytes)?;
            encode_text(species.as_str(), &mut bytes)?;
            encode_workflow(workflow, &mut bytes)?;
        }
        Mutation::TransitionUnit {
            id,
            target,
            expected_revision,
        } => {
            encode_unit_id(*id, &mut bytes);
            encode_text(target.as_str(), &mut bytes)?;
            expected_revision.encode_to(&mut bytes);
        }
        Mutation::CompleteUnit {
            id,
            expected_revision,
        } => {
            encode_unit_id(*id, &mut bytes);
            expected_revision.encode_to(&mut bytes);
        }
        Mutation::CreateRelationshipDefinition(definition) => {
            encode_definition(definition, &mut bytes)?;
        }
        Mutation::CreateRelationship(relationship) | Mutation::DeleteRelationship(relationship) => {
            encode_relationship(relationship, &mut bytes)?;
        }
        Mutation::RecordAssociation(association) | Mutation::RevokeAssociation(association) => {
            encode_association(association, &mut bytes)?;
        }
    }
    Ok(bytes)
}

fn encode_unit_id(id: IntentUnitId, output: &mut Vec<u8>) {
    output.extend_from_slice(id.as_uuid().as_bytes());
}

fn encode_text(value: &str, output: &mut Vec<u8>) -> Result<(), SubmissionError> {
    let length = u32::try_from(value.len()).map_err(|_| {
        SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "bounded runtime text length exceeds u32",
        )
    })?;
    Compact(length).encode_to(output);
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_reference(
    reference: &ExternalReference,
    output: &mut Vec<u8>,
) -> Result<(), SubmissionError> {
    encode_text(reference.namespace().as_str(), output)?;
    encode_text(reference.scope().as_str(), output)?;
    encode_text(reference.value().as_str(), output)
}

fn encode_workflow(workflow: &Workflow, output: &mut Vec<u8>) -> Result<(), SubmissionError> {
    encode_text(workflow.id().as_str(), output)?;
    encode_length(workflow.phases().len(), output)?;
    for phase in workflow.phases() {
        encode_text(phase.as_str(), output)?;
    }
    encode_text(workflow.initial_phase().as_str(), output)?;
    encode_length(workflow.edges().len(), output)?;
    for edge in workflow.edges() {
        encode_text(edge.from().as_str(), output)?;
        encode_text(edge.to().as_str(), output)?;
    }
    encode_length(workflow.completion_phases().len(), output)?;
    for phase in workflow.completion_phases() {
        encode_text(phase.as_str(), output)?;
    }
    Ok(())
}

fn encode_length(length: usize, output: &mut Vec<u8>) -> Result<(), SubmissionError> {
    let length = u32::try_from(length).map_err(|_| {
        SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "bounded runtime sequence length exceeds u32",
        )
    })?;
    Compact(length).encode_to(output);
    Ok(())
}

fn encode_definition_key(
    key: &cubikan_core::RelationshipDefinitionKey,
    output: &mut Vec<u8>,
) -> Result<(), SubmissionError> {
    encode_text(key.id().as_str(), output)?;
    key.version().value().encode_to(output);
    Ok(())
}

fn encode_optional_species(
    species: Option<&IntentSpecies>,
    output: &mut Vec<u8>,
) -> Result<(), SubmissionError> {
    match species {
        Some(species) => {
            output.push(1);
            encode_text(species.as_str(), output)
        }
        None => {
            output.push(0);
            Ok(())
        }
    }
}

fn encode_policy(policy: RelationshipPolicy, output: &mut Vec<u8>) {
    output.push(match policy {
        RelationshipPolicy::Allow => 0,
        RelationshipPolicy::Reject => 1,
    });
}

fn encode_definition(
    definition: &RelationshipDefinition,
    output: &mut Vec<u8>,
) -> Result<(), SubmissionError> {
    encode_definition_key(definition.key(), output)?;
    output.push(0);
    encode_optional_species(definition.source_species(), output)?;
    encode_optional_species(definition.target_species(), output)?;
    encode_policy(definition.self_policy(), output);
    encode_policy(definition.cycle_policy(), output);
    Ok(())
}

fn encode_relationship(
    relationship: &RelationshipIdentity,
    output: &mut Vec<u8>,
) -> Result<(), SubmissionError> {
    encode_definition_key(relationship.definition(), output)?;
    encode_unit_id(relationship.source(), output);
    encode_unit_id(relationship.target(), output);
    Ok(())
}

fn encode_association(
    association: &RecordedAssociation,
    output: &mut Vec<u8>,
) -> Result<(), SubmissionError> {
    encode_unit_id(association.unit_id(), output);
    match association.subject() {
        AssociationSubject::WholeUnit => output.push(0),
        AssociationSubject::Revision(revision) => {
            output.push(1);
            revision.encode_to(output);
        }
    }
    encode_reference(association.reference(), output)
}

#[derive(Clone, Debug)]
struct InspectedExtrinsic {
    call_args: Vec<u8>,
}

#[allow(clippy::too_many_arguments)]
async fn inspect_signed_extrinsic(
    offline: &OfflineClient<CubiKanConfig>,
    block_number: u64,
    encoded: &[u8],
    expected_signer: [u8; 32],
    expected_signature: Option<[u8; 64]>,
    expected_nonce: u64,
    signing_block_number: u64,
    operation: MutationOperation,
    expected_call_data: Option<&[u8]>,
) -> Result<InspectedExtrinsic, SubmissionError> {
    let at = offline.at_block(block_number).map_err(|error| {
        SubmissionError::with_source(
            SubmissionErrorKind::RuntimeMismatch,
            "instantiate offline extrinsic decoder",
            error,
        )
    })?;
    let body = at.extrinsics().from_bytes(vec![encoded.to_vec()]).await;
    let mut decoded = body.iter();
    let extrinsic = decoded
        .next()
        .ok_or_else(|| {
            SubmissionError::without_source(
                SubmissionErrorKind::RuntimeMismatch,
                "signed extrinsic decoder returned no body item",
            )
        })?
        .map_err(|error| {
            SubmissionError::with_source(
                SubmissionErrorKind::RuntimeMismatch,
                "decode exact signed extrinsic",
                error,
            )
        })?;
    if decoded.next().is_some()
        || !extrinsic.is_signed()
        || extrinsic.pallet_name() != CUBIKAN_PALLET
        || extrinsic.call_name() != operation_call_name(operation)
    {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "signed extrinsic signer or CubiKan call identity differs",
        ));
    }
    if expected_call_data
        .is_some_and(|expected_call_data| extrinsic.call_data_bytes() != expected_call_data)
    {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "signed extrinsic call bytes differ from independent encoding",
        ));
    }

    let expected_address = [&[0_u8][..], &expected_signer].concat();
    if extrinsic.address_bytes() != Some(expected_address.as_slice()) {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "signed extrinsic AccountId32 differs from the chosen signer",
        ));
    }
    let signature_bytes = extrinsic.signature_bytes().ok_or_else(|| {
        SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "signed extrinsic has no signature bytes",
        )
    })?;
    if signature_bytes.len() != 65 || signature_bytes[0] != 1 {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "signed extrinsic signature is not exact sr25519 MultiSignature",
        ));
    }
    if expected_signature.is_some_and(|expected| signature_bytes[1..] != expected) {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "signed extrinsic signature bytes differ from the verified signature",
        ));
    }

    let extensions = extrinsic.transaction_extensions().ok_or_else(|| {
        SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "signed extrinsic has no transaction extensions",
        )
    })?;
    let names: Vec<_> = extensions
        .iter()
        .map(|extension| extension.name())
        .collect();
    if names != EXACT_EXTENSION_NAMES {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "signed extrinsic transaction-extension order differs",
        ));
    }
    for extension in extensions.iter() {
        match extension.name() {
            "CheckNonZeroSender" => {
                extension
                    .decode_as::<CheckNonZeroSender>()
                    .transpose()
                    .map_err(runtime_decode_error)?;
            }
            "CheckWeight" => {
                extension
                    .decode_as::<CheckWeight>()
                    .transpose()
                    .map_err(runtime_decode_error)?;
            }
            "StorageWeightReclaim" => {
                extension
                    .decode_as::<StorageWeightReclaim>()
                    .transpose()
                    .map_err(runtime_decode_error)?;
            }
            _ => {}
        }
    }
    let mortality = extensions
        .find::<CheckMortality<CubiKanConfig>>()
        .ok_or_else(|| {
            SubmissionError::without_source(
                SubmissionErrorKind::RuntimeMismatch,
                "signed extrinsic omits CheckMortality",
            )
        })?
        .map_err(runtime_decode_error)?;
    if mortality != Era::mortal(MORTAL_PERIOD, signing_block_number)
        || extensions.nonce() != Some(expected_nonce)
        || extensions.tip() != Some(0)
    {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "signed extrinsic nonce, mortal period/phase, or zero tip differs",
        ));
    }

    Ok(InspectedExtrinsic {
        call_args: extrinsic.call_data_field_bytes().to_vec(),
    })
}

fn runtime_decode_error(error: subxt::error::ExtrinsicError) -> SubmissionError {
    SubmissionError::with_source(
        SubmissionErrorKind::RuntimeMismatch,
        "decode signed extrinsic transaction extension",
        error,
    )
}

const fn operation_call_name(operation: MutationOperation) -> &'static str {
    match operation {
        MutationOperation::CreateUnit => "create_unit",
        MutationOperation::TransitionUnit => "transition_unit",
        MutationOperation::CompleteUnit => "complete_unit",
        MutationOperation::CreateRelationshipDefinition => "create_relationship_definition",
        MutationOperation::CreateRelationship => "create_relationship",
        MutationOperation::DeleteRelationship => "delete_relationship",
        MutationOperation::RecordAssociation => "record_association",
        MutationOperation::RevokeAssociation => "revoke_association",
    }
}

fn decode_dry_run(
    bytes: &[u8],
    metadata: subxt::ArcMetadata,
) -> Result<DryRunOutcome, StaticFailure> {
    let Some((&outer, rest)) = bytes.split_first() else {
        return Err(StaticFailure("empty system_dryRun response"));
    };
    match outer {
        // ApplyExtrinsicResult::Ok(DispatchOutcome). A dispatch error is still
        // valid for inclusion, so submission proceeds and its finalized
        // System event supplies the authoritative terminal rejection.
        0 => match rest.split_first() {
            Some((&0, [])) => Ok(DryRunOutcome::Valid),
            Some((&1, dispatch_bytes)) if !dispatch_bytes.is_empty() => {
                let type_id = metadata
                    .dispatch_error_ty()
                    .ok_or(StaticFailure("pinned metadata has no DispatchError type"))?;
                let mut input = dispatch_bytes;
                decode_as_type(&mut input, type_id, metadata.types())
                    .map_err(|_| StaticFailure("malformed dry-run DispatchError"))?;
                if !input.is_empty() {
                    return Err(StaticFailure("dry-run DispatchError has trailing bytes"));
                }
                Ok(DryRunOutcome::Valid)
            }
            _ => Err(StaticFailure("malformed dry-run DispatchOutcome")),
        },
        // ApplyExtrinsicResult::Err(TransactionValidityError). These pinned
        // enums have only unit variants plus Custom(u8); consume their exact
        // shape so truncation or trailing bytes fail closed.
        1 => {
            let Some((&validity_kind, validity)) = rest.split_first() else {
                return Err(StaticFailure("truncated transaction-validity error"));
            };
            let Some((&variant, fields)) = validity.split_first() else {
                return Err(StaticFailure("truncated transaction-validity variant"));
            };
            let valid_shape = match validity_kind {
                // InvalidTransaction::Custom is variant 7 and carries one u8.
                0 => match variant {
                    0..=6 | 8..=12 => fields.is_empty(),
                    7 => fields.len() == 1,
                    _ => false,
                },
                // UnknownTransaction::Custom is variant 2 and carries one u8.
                1 => match variant {
                    0..=1 => fields.is_empty(),
                    2 => fields.len() == 1,
                    _ => false,
                },
                _ => false,
            };
            if !valid_shape {
                return Err(StaticFailure(
                    "malformed or trailing transaction-validity error",
                ));
            }
            let code = match (validity_kind, variant) {
                (0, 1) => SubmissionFailureCode::InsufficientBalance,
                (0, 2 | 3) => SubmissionFailureCode::NonceConflict,
                _ => SubmissionFailureCode::TransactionInvalid,
            };
            Ok(DryRunOutcome::Invalid(code))
        }
        _ => Err(StaticFailure("invalid system_dryRun Result variant")),
    }
}

async fn reconcile_existing(
    chain: &dyn SubmissionChain,
    mut lane: SignerLane,
    record: JournalRecord,
) -> Result<SubmissionResult, SubmissionError> {
    let operation = MutationOperation::from_journal(record.operation());
    let era = record_era(&record);
    match record.state() {
        JournalState::Prepared => {
            let head = match chain.finalized_head().await {
                Ok(head) => head,
                Err(source) => {
                    return Ok(result_unresolved(
                        operation,
                        *record.extrinsic_hash(),
                        era,
                        Some(Box::new(source)),
                    ));
                }
            };
            let inclusion = match scan_prepared_history(chain, &record, head).await {
                Ok(inclusion) => inclusion,
                Err(source) => {
                    return Ok(result_unresolved(
                        operation,
                        *record.extrinsic_hash(),
                        era,
                        Some(Box::new(source)),
                    ));
                }
            };
            if let Some(block) = inclusion {
                match resolve_in_block(chain, &record, block).await {
                    Ok((resolved, detail)) => {
                        lane.publish_resolved(resolved).map_err(journal_error)?;
                        Ok(result_with_ack(detail, lane))
                    }
                    Err(source) => Ok(result_unresolved(
                        operation,
                        *record.extrinsic_hash(),
                        era,
                        Some(Box::new(source)),
                    )),
                }
            } else if head.number > record.death() {
                let resolved = record
                    .resolved(JournalState::ExpiredNotIncluded, head.number, head.hash)
                    .map_err(journal_error)?;
                lane.publish_resolved(resolved).map_err(journal_error)?;
                Ok(result_with_ack(
                    SubmissionOutcomeDetail::ExpiredNotIncluded {
                        operation,
                        expected_extrinsic_hash: *record.extrinsic_hash(),
                        era,
                    },
                    lane,
                ))
            } else {
                Ok(result_unresolved(
                    operation,
                    *record.extrinsic_hash(),
                    era,
                    None,
                ))
            }
        }
        JournalState::FinalizedAccepted
        | JournalState::FinalizedDispatchRejected
        | JournalState::FinalizedInvariantFailed => {
            let block = match chain
                .finalized_block(record.resolution_block_number())
                .await
            {
                Ok(block) if block.hash == *record.resolution_block_hash() => block,
                Ok(_) => {
                    return Ok(result_unresolved(
                        operation,
                        *record.extrinsic_hash(),
                        era,
                        Some(Box::new(StaticFailure(
                            "resolved journal block hash is no longer canonical",
                        ))),
                    ));
                }
                Err(source) => {
                    return Ok(result_unresolved(
                        operation,
                        *record.extrinsic_hash(),
                        era,
                        Some(Box::new(source)),
                    ));
                }
            };
            match resolve_in_block(chain, &record_as_prepared(&record)?, block).await {
                Ok((resolved, detail)) if resolved.state() == record.state() => {
                    Ok(result_with_ack(detail, lane))
                }
                Ok(_) => Ok(result_unresolved(
                    operation,
                    *record.extrinsic_hash(),
                    era,
                    Some(Box::new(StaticFailure(
                        "reconstructed terminal outcome differs from durable journal state",
                    ))),
                )),
                Err(source) => Ok(result_unresolved(
                    operation,
                    *record.extrinsic_hash(),
                    era,
                    Some(Box::new(source)),
                )),
            }
        }
        JournalState::ExpiredNotIncluded => recover_expired(chain, lane, record).await,
    }
}

async fn scan_prepared_history(
    chain: &dyn SubmissionChain,
    record: &JournalRecord,
    head: ChainHead,
) -> Result<Option<SubmissionBlock>, ChainFailure> {
    let scan_end = head.number.min(record.death());
    let mut inclusion = None;
    if scan_end < record.birth() {
        return Ok(None);
    }
    for number in record.birth()..=scan_end {
        let block = chain.finalized_block(number).await?;
        let count = block
            .extrinsic_hashes
            .iter()
            .filter(|hash| *hash == record.extrinsic_hash())
            .count();
        if count > 1 || (count == 1 && inclusion.replace(block).is_some()) {
            return Err(ChainFailure::new(
                "scan persisted extrinsic era",
                StaticFailure("expected extrinsic hash is duplicated in finalized history"),
            ));
        }
    }
    Ok(inclusion)
}

fn result_unresolved(
    operation: MutationOperation,
    expected_extrinsic_hash: [u8; 32],
    era: MortalEra,
    delivery_source: Option<BoxError>,
) -> SubmissionResult {
    SubmissionResult {
        outcome: SubmissionOutcome(SubmissionOutcomeDetail::SubmissionLaneUnresolved {
            operation,
            expected_extrinsic_hash,
            era,
        }),
        acknowledgement: None,
        delivery_source,
    }
}

fn record_era(record: &JournalRecord) -> MortalEra {
    MortalEra {
        birth: record.birth(),
        death: record.death(),
    }
}

fn record_as_prepared(record: &JournalRecord) -> Result<JournalRecord, SubmissionError> {
    JournalRecord::prepared(
        *record.deployment_id(),
        *record.signer(),
        record.nonce(),
        *record.extrinsic_hash(),
        record.signing_block_number(),
        *record.signing_block_hash(),
        record.operation(),
    )
    .map_err(journal_error)
}

async fn recover_expired(
    chain: &dyn SubmissionChain,
    lane: SignerLane,
    record: JournalRecord,
) -> Result<SubmissionResult, SubmissionError> {
    let operation = MutationOperation::from_journal(record.operation());
    let era = record_era(&record);
    let resolution = match chain
        .finalized_block(record.resolution_block_number())
        .await
    {
        Ok(block) if block.hash == *record.resolution_block_hash() => block,
        Ok(_) => {
            return Ok(result_unresolved(
                operation,
                *record.extrinsic_hash(),
                era,
                Some(Box::new(StaticFailure(
                    "stored post-death finalized coordinate is no longer canonical",
                ))),
            ));
        }
        Err(source) => {
            return Ok(result_unresolved(
                operation,
                *record.extrinsic_hash(),
                era,
                Some(Box::new(source)),
            ));
        }
    };
    if resolution.number <= record.death() {
        return Ok(result_unresolved(
            operation,
            *record.extrinsic_hash(),
            era,
            Some(Box::new(StaticFailure(
                "stored expiry coordinate is not strictly after death",
            ))),
        ));
    }
    for number in record.birth()..=record.death() {
        let block = match chain.finalized_block(number).await {
            Ok(block) => block,
            Err(source) => {
                return Ok(result_unresolved(
                    operation,
                    *record.extrinsic_hash(),
                    era,
                    Some(Box::new(source)),
                ));
            }
        };
        if block
            .extrinsic_hashes
            .iter()
            .any(|hash| hash == record.extrinsic_hash())
        {
            return Ok(result_unresolved(
                operation,
                *record.extrinsic_hash(),
                era,
                Some(Box::new(StaticFailure(
                    "stored expiry conflicts with finalized inclusion evidence",
                ))),
            ));
        }
    }
    Ok(result_with_ack(
        SubmissionOutcomeDetail::ExpiredNotIncluded {
            operation,
            expected_extrinsic_hash: *record.extrinsic_hash(),
            era,
        },
        lane,
    ))
}

async fn resolve_finalized(
    chain: &dyn SubmissionChain,
    prepared: &JournalRecord,
    finalized_hash: [u8; 32],
) -> Result<(JournalRecord, SubmissionOutcomeDetail), ChainFailure> {
    let block = chain.finalized_block_by_hash(finalized_hash).await?;
    if !(prepared.birth()..=prepared.death()).contains(&block.number) {
        return Err(ChainFailure::new(
            "verify finalized inclusion era",
            StaticFailure("finalized inclusion lies outside the persisted mortal era"),
        ));
    }
    resolve_in_block(chain, prepared, block).await
}

async fn resolve_in_block(
    chain: &dyn SubmissionChain,
    prepared: &JournalRecord,
    block: SubmissionBlock,
) -> Result<(JournalRecord, SubmissionOutcomeDetail), ChainFailure> {
    if !(prepared.birth()..=prepared.death()).contains(&block.number) {
        return Err(ChainFailure::new(
            "verify finalized inclusion era",
            StaticFailure("recovered inclusion lies outside the persisted mortal era"),
        ));
    }
    let indexes: Vec<_> = block
        .extrinsic_hashes
        .iter()
        .enumerate()
        .filter_map(|(index, hash)| (hash == prepared.extrinsic_hash()).then_some(index))
        .collect();
    let [index] = indexes.as_slice() else {
        return Err(ChainFailure::new(
            "locate exact finalized extrinsic",
            StaticFailure("expected extrinsic hash is absent or duplicated in finalized body"),
        ));
    };
    let raw = block.raw_extrinsics.get(*index).ok_or_else(|| {
        ChainFailure::new(
            "join finalized extrinsic body",
            StaticFailure("extrinsic hash/body index join is inconsistent"),
        )
    })?;
    let identity = chain.identity();
    let offline = offline_client(&identity)
        .map_err(|error| ChainFailure::new("instantiate recovery decoder", error))?;
    let operation = MutationOperation::from_journal(prepared.operation());
    let inspected = inspect_signed_extrinsic(
        &offline,
        block.number,
        raw,
        *prepared.signer(),
        None,
        prepared.nonce(),
        prepared.signing_block_number(),
        operation,
        None,
    )
    .await
    .map_err(|error| ChainFailure::new("decode recovered signed extrinsic", error))?;
    verify_recovered_signature(&offline, prepared, raw, &inspected)
        .map_err(|error| ChainFailure::new("verify recovered signed extrinsic", error))?;

    let extrinsic_index = u32::try_from(*index).map_err(|_| {
        ChainFailure::new(
            "convert finalized extrinsic index",
            StaticFailure("extrinsic index exceeds u32"),
        )
    })?;
    let finalized_extrinsic = FinalizedExtrinsic {
        block_number: block.number,
        block_hash: block.hash,
        extrinsic_index,
        extrinsic_hash: *prepared.extrinsic_hash(),
    };

    let evidence = chain
        .finalized_dispatch_evidence(&block, extrinsic_index)
        .await?;

    classify_finalized_evidence(
        prepared,
        block.number,
        block.hash,
        finalized_extrinsic,
        operation,
        &inspected.call_args,
        evidence.successes,
        &evidence.errors,
        &block.accepted_events,
    )
}

#[allow(clippy::too_many_arguments)]
fn classify_finalized_evidence(
    prepared: &JournalRecord,
    block_number: u64,
    block_hash: [u8; 32],
    finalized_extrinsic: FinalizedExtrinsic,
    operation: MutationOperation,
    call_args: &[u8],
    successes: usize,
    dispatch_errors: &[DecodedDispatchFailure],
    accepted_events: &[AcceptedEvent],
) -> Result<(JournalRecord, SubmissionOutcomeDetail), ChainFailure> {
    let accepted_in_extrinsic: Vec<_> = accepted_events
        .iter()
        .filter(|event| event.extrinsic_index() == finalized_extrinsic.extrinsic_index)
        .collect();
    let matching = accepted_in_extrinsic
        .iter()
        .copied()
        .filter(|event| {
            event.extrinsic_hash() == prepared.extrinsic_hash()
                && event.deployment_id() == prepared.deployment_id()
                && event.event_schema_version() == EVENT_SCHEMA_VERSION
                && event.signer() == prepared.signer()
                && accepted_matches_call(operation, call_args, event)
        })
        .collect::<Vec<_>>();

    if successes == 0 && dispatch_errors.len() == 1 && accepted_in_extrinsic.is_empty() {
        let error = map_decoded_dispatch_failure(operation, &dispatch_errors[0]);
        let resolved = prepared
            .resolved(
                JournalState::FinalizedDispatchRejected,
                block_number,
                block_hash,
            )
            .map_err(|error| ChainFailure::new("publish dispatch resolution", error))?;
        return Ok((
            resolved,
            SubmissionOutcomeDetail::FinalizedDispatchRejected {
                operation,
                finalized_extrinsic,
                error,
            },
        ));
    }

    if successes == 1
        && dispatch_errors.is_empty()
        && accepted_in_extrinsic.len() == 1
        && matching.len() == 1
    {
        let event = matching[0];
        let effect = accepted_effect(event.payload());
        let resolved = prepared
            .resolved(JournalState::FinalizedAccepted, block_number, block_hash)
            .map_err(|error| ChainFailure::new("publish accepted resolution", error))?;
        return Ok((
            resolved,
            SubmissionOutcomeDetail::FinalizedAccepted {
                operation,
                coordinate: AcceptedCoordinate {
                    finalized_extrinsic,
                    system_event_index: event.system_event_index(),
                    global_sequence: event.global_sequence(),
                },
                effect,
            },
        ));
    }

    let resolved = prepared
        .resolved(
            JournalState::FinalizedInvariantFailed,
            block_number,
            block_hash,
        )
        .map_err(|error| ChainFailure::new("publish invariant resolution", error))?;
    Ok((
        resolved,
        SubmissionOutcomeDetail::FinalizedInvariantFailed {
            operation,
            finalized_extrinsic,
        },
    ))
}

fn verify_recovered_signature(
    offline: &OfflineClient<CubiKanConfig>,
    record: &JournalRecord,
    raw: &[u8],
    inspected: &InspectedExtrinsic,
) -> Result<(), SubmissionError> {
    let at = offline
        .at_block(record.signing_block_number())
        .map_err(|error| {
            SubmissionError::with_source(
                SubmissionErrorKind::RuntimeMismatch,
                "instantiate signing-block payload verifier",
                error,
            )
        })?;
    let mut call_data = Vec::with_capacity(2 + inspected.call_args.len());
    call_data.push(50);
    call_data.push(operation_call_index(MutationOperation::from_journal(
        record.operation(),
    )));
    call_data.extend_from_slice(&inspected.call_args);
    let payload = expected_payload_from_call_data(
        &at,
        &call_data,
        record.nonce(),
        record.signing_block_number(),
        *record.signing_block_hash(),
    )?;
    let (_, signature) = signed_address_and_signature(raw)?;
    if !sr25519::verify(
        &sr25519::Signature(signature),
        &payload,
        &sr25519::PublicKey(*record.signer()),
    ) {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "recovered signed extrinsic signature does not verify against persisted signer/payload",
        ));
    }
    Ok(())
}

fn signed_address_and_signature(raw: &[u8]) -> Result<([u8; 32], [u8; 64]), SubmissionError> {
    let mut input = raw;
    let encoded_length = Compact::<u32>::decode(&mut input).map_err(|error| {
        SubmissionError::with_source(
            SubmissionErrorKind::RuntimeMismatch,
            "decode signed extrinsic length",
            error,
        )
    })?;
    if usize::try_from(encoded_length.0).ok() != Some(input.len())
        || input.first() != Some(&0x84)
        || input.get(1) != Some(&0)
    {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "signed extrinsic length/version/address prefix differs",
        ));
    }
    let account: [u8; 32] = input
        .get(2..34)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| {
            SubmissionError::without_source(
                SubmissionErrorKind::RuntimeMismatch,
                "signed extrinsic AccountId32 is truncated",
            )
        })?;
    if input.get(34) != Some(&1) {
        return Err(SubmissionError::without_source(
            SubmissionErrorKind::RuntimeMismatch,
            "signed extrinsic is not sr25519",
        ));
    }
    let signature = input
        .get(35..99)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| {
            SubmissionError::without_source(
                SubmissionErrorKind::RuntimeMismatch,
                "signed extrinsic signature is truncated",
            )
        })?;
    Ok((account, signature))
}

fn expected_payload_from_call_data(
    at: &subxt::OfflineClientAtBlock<CubiKanConfig>,
    call_data: &[u8],
    nonce: u64,
    signing_number: u64,
    signing_hash: [u8; 32],
) -> Result<Vec<u8>, SubmissionError> {
    let mut payload = call_data.to_vec();
    Era::mortal(MORTAL_PERIOD, signing_number).encode_to(&mut payload);
    Compact(nonce).encode_to(&mut payload);
    Compact(0_u128).encode_to(&mut payload);
    at.spec_version().encode_to(&mut payload);
    at.transaction_version().encode_to(&mut payload);
    at.genesis_hash()
        .ok_or_else(|| {
            SubmissionError::without_source(
                SubmissionErrorKind::RuntimeMismatch,
                "offline recovery client has no genesis hash",
            )
        })?
        .encode_to(&mut payload);
    H256::from(signing_hash).encode_to(&mut payload);
    if payload.len() > 256 {
        Ok(at.hasher().hash(&payload).as_ref().to_vec())
    } else {
        Ok(payload)
    }
}

const fn operation_call_index(operation: MutationOperation) -> u8 {
    match operation {
        MutationOperation::CreateUnit => 0,
        MutationOperation::TransitionUnit => 1,
        MutationOperation::CompleteUnit => 2,
        MutationOperation::CreateRelationshipDefinition => 4,
        MutationOperation::CreateRelationship => 5,
        MutationOperation::DeleteRelationship => 6,
        MutationOperation::RecordAssociation => 7,
        MutationOperation::RevokeAssociation => 8,
    }
}

fn accepted_matches_call(
    operation: MutationOperation,
    call_args: &[u8],
    event: &AcceptedEvent,
) -> bool {
    if call_args.get(..2) != Some(COMMAND_SCHEMA_VERSION.encode().as_slice()) {
        return false;
    }
    let raw = event.raw_payload();
    match operation {
        MutationOperation::CreateUnit => raw.first() == Some(&0) && raw.get(1..) == Some(call_args),
        MutationOperation::TransitionUnit => {
            let Some((id, target, expected_revision)) = parse_transition_call(call_args) else {
                return false;
            };
            let Some(committed_revision) = expected_revision.checked_add(1) else {
                return false;
            };
            matches!(
                event.payload(),
                CanonicalPayload::UnitTransitioned {
                    unit_id,
                    committed_revision: actual_revision,
                    to,
                    ..
                } if *unit_id == id
                    && *actual_revision == committed_revision
                    && *to == target
            )
        }
        MutationOperation::CompleteUnit => {
            let Some((id, expected_revision)) = parse_completion_call(call_args) else {
                return false;
            };
            let Some(committed_revision) = expected_revision.checked_add(1) else {
                return false;
            };
            matches!(
                event.payload(),
                CanonicalPayload::UnitCompleted {
                    unit_id,
                    committed_revision: actual_revision,
                    ..
                } if *unit_id == id && *actual_revision == committed_revision
            )
        }
        MutationOperation::CreateRelationshipDefinition => {
            raw.first() == Some(&3) && raw.get(1..) == call_args.get(2..)
        }
        MutationOperation::CreateRelationship => {
            raw.first() == Some(&4) && raw.get(1..) == call_args.get(2..)
        }
        MutationOperation::DeleteRelationship => {
            raw.first() == Some(&5) && raw.get(1..) == call_args.get(2..)
        }
        MutationOperation::RecordAssociation => {
            raw.first() == Some(&6) && raw.get(1..) == call_args.get(2..)
        }
        MutationOperation::RevokeAssociation => {
            raw.first() == Some(&7) && raw.get(1..) == call_args.get(2..)
        }
    }
}

fn parse_transition_call(call_args: &[u8]) -> Option<(IntentUnitId, PhaseId, u64)> {
    let mut input = call_args;
    if u16::decode(&mut input).ok()? != COMMAND_SCHEMA_VERSION {
        return None;
    }
    let id = decode_call_unit_id(&mut input)?;
    let target = PhaseId::from_bytes(decode_call_text(&mut input)?).ok()?;
    let expected_revision = u64::decode(&mut input).ok()?;
    input.is_empty().then_some((id, target, expected_revision))
}

fn parse_completion_call(call_args: &[u8]) -> Option<(IntentUnitId, u64)> {
    let mut input = call_args;
    if u16::decode(&mut input).ok()? != COMMAND_SCHEMA_VERSION {
        return None;
    }
    let id = decode_call_unit_id(&mut input)?;
    let expected_revision = u64::decode(&mut input).ok()?;
    input.is_empty().then_some((id, expected_revision))
}

fn decode_call_unit_id(input: &mut &[u8]) -> Option<IntentUnitId> {
    let (bytes, rest) = input.split_at_checked(16)?;
    *input = rest;
    let uuid_bytes: [u8; 16] = bytes.try_into().ok()?;
    Some(IntentUnitId::from_uuid(uuid::Uuid::from_bytes(uuid_bytes)))
}

fn decode_call_text<'a>(input: &mut &'a [u8]) -> Option<&'a [u8]> {
    let before = *input;
    let length = Compact::<u32>::decode(input).ok()?;
    let compact_bytes = before.len().checked_sub(input.len())?;
    if Compact(length.0).encode().as_slice() != before.get(..compact_bytes)? {
        return None;
    }
    let length = usize::try_from(length.0).ok()?;
    let (text, rest) = input.split_at_checked(length)?;
    *input = rest;
    Some(text)
}

fn accepted_effect(payload: &CanonicalPayload) -> AcceptedEffect {
    match payload {
        CanonicalPayload::UnitCreated(unit) => AcceptedEffect::UnitCreated {
            unit_id: unit.id(),
            committed_revision: unit.revision().value(),
        },
        CanonicalPayload::UnitTransitioned {
            unit_id,
            committed_revision,
            ..
        } => AcceptedEffect::UnitTransitioned {
            unit_id: *unit_id,
            committed_revision: *committed_revision,
        },
        CanonicalPayload::UnitCompleted {
            unit_id,
            committed_revision,
            ..
        } => AcceptedEffect::UnitCompleted {
            unit_id: *unit_id,
            committed_revision: *committed_revision,
        },
        CanonicalPayload::RelationshipDefinitionCreated(definition) => {
            AcceptedEffect::RelationshipDefinitionCreated(definition.clone())
        }
        CanonicalPayload::RelationshipCreated(relationship) => {
            AcceptedEffect::RelationshipCreated(relationship.clone())
        }
        CanonicalPayload::RelationshipDeleted(relationship) => {
            AcceptedEffect::RelationshipDeleted(relationship.clone())
        }
        CanonicalPayload::AssociationRecorded(association) => {
            AcceptedEffect::AssociationRecorded(association.clone())
        }
        CanonicalPayload::AssociationRevoked(association) => {
            AcceptedEffect::AssociationRevoked(association.clone())
        }
    }
}

fn map_decoded_dispatch_failure(
    operation: MutationOperation,
    error: &DecodedDispatchFailure,
) -> SubmissionFailureCode {
    match error {
        DecodedDispatchFailure::Known(error) => map_dispatch_error(operation, error),
        DecodedDispatchFailure::RuntimeMismatch => SubmissionFailureCode::RuntimeMismatch,
    }
}

fn map_dispatch_error(
    operation: MutationOperation,
    error: &DispatchError,
) -> SubmissionFailureCode {
    let DispatchError::Module(module) = error else {
        return match error {
            DispatchError::BadOrigin => SubmissionFailureCode::UnsignedCall,
            _ => SubmissionFailureCode::RuntimeMismatch,
        };
    };
    let Ok(details) = module.details() else {
        return SubmissionFailureCode::RuntimeMismatch;
    };
    if details.pallet.name() != CUBIKAN_PALLET {
        return SubmissionFailureCode::RuntimeMismatch;
    }
    match details.variant.name.as_str() {
        "UnsupportedCommandSchemaVersion" => SubmissionFailureCode::UnsupportedCommandSchemaVersion,
        "UnauthorizedSubmitter" => SubmissionFailureCode::UnauthorizedSubmitter,
        "IntentUnitAlreadyExists" => SubmissionFailureCode::DuplicateIntentUnit,
        "IntentUnitNotFound" | "AssociationUnitNotFound" => {
            SubmissionFailureCode::IntentUnitNotFound
        }
        "StaleRevision" => SubmissionFailureCode::RevisionConflict,
        "LifecycleHistoryCapacityExceeded" => {
            SubmissionFailureCode::LifecycleHistoryCapacityExceeded
        }
        "IntentUnitAlreadyCompleted" => match operation {
            MutationOperation::TransitionUnit => SubmissionFailureCode::TransitionAlreadyCompleted,
            MutationOperation::CompleteUnit => SubmissionFailureCode::CompletionAlreadyCompleted,
            _ => SubmissionFailureCode::RuntimeMismatch,
        },
        "UnknownTargetPhase" => SubmissionFailureCode::TransitionUnknownTarget,
        "TransitionNotAllowed" => SubmissionFailureCode::TransitionNotAllowed,
        "CompletionPhaseNotEligible" => SubmissionFailureCode::CompletionPhaseNotEligible,
        "RelationshipDefinitionAlreadyExists" => {
            SubmissionFailureCode::RelationshipDefinitionAlreadyExists
        }
        "RelationshipDefinitionNotFound" => SubmissionFailureCode::RelationshipDefinitionNotFound,
        "RelationshipSourceNotFound" => SubmissionFailureCode::RelationshipSourceNotFound,
        "RelationshipTargetNotFound" => SubmissionFailureCode::RelationshipTargetNotFound,
        "RelationshipSourceSpeciesMismatch" => {
            SubmissionFailureCode::RelationshipSourceSpeciesMismatch
        }
        "RelationshipTargetSpeciesMismatch" => {
            SubmissionFailureCode::RelationshipTargetSpeciesMismatch
        }
        "RelationshipSelfEdgeRejected" => SubmissionFailureCode::SelfRelationshipRejected,
        "RelationshipAlreadyExists" => SubmissionFailureCode::DuplicateRelationship,
        "RelationshipCycleRejected" => SubmissionFailureCode::CycleRejected,
        "RelationshipCapacityExceeded" => SubmissionFailureCode::RelationshipCapacityExceeded,
        "RelationshipNotFound" => SubmissionFailureCode::RelationshipNotFound,
        "AssociationRevisionNotFound" => SubmissionFailureCode::AssociationRevisionOutOfRange,
        "AssociationAlreadyExists" => SubmissionFailureCode::DuplicateAssociation,
        "AssociationCapacityExceeded" => SubmissionFailureCode::AssociationCapacityExceeded,
        "AssociationNotFound" => SubmissionFailureCode::AssociationNotFound,
        "GlobalSequenceExhausted" => SubmissionFailureCode::GlobalSequenceExhausted,
        // Structural decoding prevents invalid references, and overflow is a
        // pinned-runtime invariant rather than a stable adapter rejection.
        "AssociationReferenceInvalid" | "LifecycleRevisionExhausted" => {
            SubmissionFailureCode::RuntimeMismatch
        }
        _ => SubmissionFailureCode::RuntimeMismatch,
    }
}

fn journal_error(error: JournalError) -> SubmissionError {
    let kind = match &error {
        JournalError::UnsupportedPlatform => SubmissionErrorKind::UnsupportedPlatform,
        JournalError::UnsupportedFilesystem | JournalError::InsecurePath => {
            SubmissionErrorKind::InsecureProjectionPath
        }
        JournalError::CorruptJournal
        | JournalError::InvalidRecord
        | JournalError::InvalidTransition
        | JournalError::Io { .. } => SubmissionErrorKind::SubmissionLaneCorrupt,
        #[cfg(test)]
        JournalError::InjectedFault(_) => SubmissionErrorKind::SubmissionLaneCorrupt,
    };
    SubmissionError::with_source(kind, "submission signer-lane failure", error)
}

fn chain_error_before_send(error: ChainFailure) -> SubmissionError {
    let kind = if error.operation.contains("block") || error.operation.contains("storage") {
        SubmissionErrorKind::ArchiveHistoryUnavailable
    } else {
        SubmissionErrorKind::ArchiveRpcUnavailable
    };
    SubmissionError::with_source(
        kind,
        "submission archive operation failed before send",
        error,
    )
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
        sync::Mutex,
    };

    use sha2::{Digest, Sha256};

    use super::*;
    use crate::submission_journal::LaneNames;

    const SIGNING_ORACLE: &[u8] =
        include_bytes!("../../../tests/fixtures/submission-journal-v1/signed-extrinsic-v1.json");
    const CALL_HEX: &str =
        include_str!("../../../tests/fixtures/submission-journal-v1/raw/signing/call.scale.hex");
    const PAYLOAD_HEX: &str = include_str!(
        "../../../tests/fixtures/submission-journal-v1/raw/signing/signer-payload.scale.hex"
    );
    const SIGNATURE_HEX: &str = include_str!(
        "../../../tests/fixtures/submission-journal-v1/raw/signing/signature.scale.hex"
    );
    const EXTRINSIC_HEX: &str = include_str!(
        "../../../tests/fixtures/submission-journal-v1/raw/signing/signed-extrinsic.scale.hex"
    );
    const PREPARED_HEX: &str =
        include_str!("../../../tests/fixtures/submission-journal-v1/raw/journal/prepared.hex");
    const RECONCILIATION_ORACLE: &str =
        include_str!("../../../tests/fixtures/submission-journal-v1/reconciliation-cases-v1.json");
    const FINALIZED_ACCEPTED_HEX: &str = include_str!(
        "../../../tests/fixtures/submission-journal-v1/raw/journal/finalized-accepted.hex"
    );
    const UNIT_CREATED_HEX: &str = include_str!(
        "../../../tests/fixtures/finalized-events-v1/raw/payloads/0001-unit-created-a.scale.hex"
    );
    const UNIT_TRANSITIONED_HEX: &str = include_str!(
        "../../../tests/fixtures/finalized-events-v1/raw/payloads/0006-unit-transitioned-a.scale.hex"
    );
    const UNIT_COMPLETED_HEX: &str = include_str!(
        "../../../tests/fixtures/finalized-events-v1/raw/payloads/0007-unit-completed-a.scale.hex"
    );
    const DEFINITION_HEX: &str = include_str!(
        "../../../tests/fixtures/finalized-events-v1/raw/payloads/0003-relationship-definition-created.scale.hex"
    );
    const RELATIONSHIP_HEX: &str = include_str!(
        "../../../tests/fixtures/finalized-events-v1/raw/payloads/0004-relationship-created.scale.hex"
    );
    const ASSOCIATION_HEX: &str = include_str!(
        "../../../tests/fixtures/finalized-events-v1/raw/payloads/0005-association-recorded-a.scale.hex"
    );
    const ORACLE_GENESIS: [u8; 32] = [
        0x62, 0x7f, 0x53, 0xb3, 0xab, 0xc0, 0x11, 0x30, 0xec, 0x27, 0x3e, 0xf8, 0x57, 0x59, 0xf9,
        0x07, 0x79, 0xe8, 0x49, 0x76, 0x14, 0xa4, 0x28, 0xa6, 0x6d, 0x86, 0x2a, 0x62, 0x4e, 0xe0,
        0x1a, 0x17,
    ];
    const ORACLE_SIGNING_HASH: [u8; 32] = [
        0x83, 0x82, 0x81, 0x80, 0x7f, 0x7e, 0x7d, 0x7c, 0x7b, 0x7a, 0x79, 0x78, 0x77, 0x76, 0x75,
        0x74, 0x73, 0x72, 0x71, 0x70, 0x6f, 0x6e, 0x6d, 0x6c, 0x6b, 0x6a, 0x69, 0x68, 0x67, 0x66,
        0x65, 0x64,
    ];
    const ALICE: [u8; 32] = [
        0xd4, 0x35, 0x93, 0xc7, 0x15, 0xfd, 0xd3, 0x1c, 0x61, 0x14, 0x1a, 0xbd, 0x04, 0xa9, 0x9f,
        0xd6, 0x82, 0x2c, 0x85, 0x58, 0x85, 0x4c, 0xcd, 0xe3, 0x9a, 0x56, 0x84, 0xe7, 0xa5, 0x6d,
        0xa2, 0x7d,
    ];

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum TraceCall {
        FinalizedHead,
        FinalizedBlock(u64),
        FinalizedBlockByHash([u8; 32]),
        FinalizedDispatchEvidence { block: u64, extrinsic_index: u32 },
        Storage { key: Vec<u8>, at: [u8; 32] },
        DryRun { at: [u8; 32] },
        SubmitAndWatch { timeout: Duration },
    }

    #[derive(Clone, Copy, Debug)]
    enum FakeWatch {
        Finalized([u8; 32]),
        RpcRejected,
        Lost,
        Timeout,
    }

    #[derive(Clone, Copy, Debug)]
    enum FakeDispatch {
        Success,
        Failed,
        Missing,
        DuplicateSuccess,
    }

    struct FakeState {
        head: ChainHead,
        blocks: BTreeMap<u64, SubmissionBlock>,
        account: Option<Vec<u8>>,
        dry_run: DryRunOutcome,
        watch: FakeWatch,
        dispatch: FakeDispatch,
        trace: Vec<TraceCall>,
        prepared_probe: Option<PathBuf>,
        prepared_seen_before_send: bool,
    }

    struct FakeChain {
        identity: ChainIdentity,
        state: Mutex<FakeState>,
    }

    impl FakeChain {
        fn new(head: ChainHead) -> Self {
            Self {
                identity: ChainIdentity {
                    deployment_id: [0x30; 32],
                    genesis_hash: ORACLE_GENESIS,
                    spec_version: 1,
                    transaction_version: 1,
                },
                state: Mutex::new(FakeState {
                    head,
                    blocks: BTreeMap::new(),
                    account: Some(account_info(66_051)),
                    dry_run: DryRunOutcome::Valid,
                    watch: FakeWatch::Timeout,
                    dispatch: FakeDispatch::Success,
                    trace: Vec::new(),
                    prepared_probe: None,
                    prepared_seen_before_send: false,
                }),
            }
        }

        fn state(&self) -> std::sync::MutexGuard<'_, FakeState> {
            self.state.lock().expect("fake chain mutex must not poison")
        }
    }

    impl SubmissionChain for FakeChain {
        fn identity(&self) -> ChainIdentity {
            self.identity
        }

        fn finalized_head(&self) -> ChainFuture<'_, ChainHead> {
            let head = {
                let mut state = self.state();
                state.trace.push(TraceCall::FinalizedHead);
                state.head
            };
            Box::pin(async move { Ok(head) })
        }

        fn finalized_block(&self, number: u64) -> ChainFuture<'_, SubmissionBlock> {
            let block = {
                let mut state = self.state();
                state.trace.push(TraceCall::FinalizedBlock(number));
                state.blocks.get(&number).cloned()
            };
            Box::pin(async move {
                block.ok_or_else(|| {
                    ChainFailure::new(
                        "fake finalized block",
                        StaticFailure("requested fake block is unavailable"),
                    )
                })
            })
        }

        fn finalized_block_by_hash(&self, hash: [u8; 32]) -> ChainFuture<'_, SubmissionBlock> {
            let block = {
                let mut state = self.state();
                state.trace.push(TraceCall::FinalizedBlockByHash(hash));
                state
                    .blocks
                    .values()
                    .find(|block| block.hash == hash)
                    .cloned()
            };
            Box::pin(async move {
                block.ok_or_else(|| {
                    ChainFailure::new(
                        "fake finalized block hash",
                        StaticFailure("requested fake block hash is unavailable"),
                    )
                })
            })
        }

        fn finalized_dispatch_evidence<'a>(
            &'a self,
            block: &'a SubmissionBlock,
            extrinsic_index: u32,
        ) -> ChainFuture<'a, FinalizedDispatchEvidence> {
            let dispatch = {
                let mut state = self.state();
                state.trace.push(TraceCall::FinalizedDispatchEvidence {
                    block: block.number,
                    extrinsic_index,
                });
                state.dispatch
            };
            Box::pin(async move {
                Ok(match dispatch {
                    FakeDispatch::Success => FinalizedDispatchEvidence {
                        successes: 1,
                        errors: Vec::new(),
                    },
                    FakeDispatch::Failed => FinalizedDispatchEvidence {
                        successes: 0,
                        errors: vec![DecodedDispatchFailure::Known(DispatchError::BadOrigin)],
                    },
                    FakeDispatch::Missing => FinalizedDispatchEvidence {
                        successes: 0,
                        errors: Vec::new(),
                    },
                    FakeDispatch::DuplicateSuccess => FinalizedDispatchEvidence {
                        successes: 2,
                        errors: Vec::new(),
                    },
                })
            })
        }

        fn storage(&self, key: Vec<u8>, at: [u8; 32]) -> ChainFuture<'_, Option<Vec<u8>>> {
            let account = {
                let mut state = self.state();
                state.trace.push(TraceCall::Storage { key, at });
                state.account.clone()
            };
            Box::pin(async move { Ok(account) })
        }

        fn dry_run(&self, _extrinsic: Vec<u8>, at: [u8; 32]) -> ChainFuture<'_, DryRunOutcome> {
            let outcome = {
                let mut state = self.state();
                state.trace.push(TraceCall::DryRun { at });
                state.dry_run
            };
            Box::pin(async move { Ok(outcome) })
        }

        fn submit_and_watch(
            &self,
            _extrinsic: Vec<u8>,
            timeout: Duration,
        ) -> ChainFuture<'_, WatchOutcome> {
            let watch = {
                let mut state = self.state();
                state.trace.push(TraceCall::SubmitAndWatch { timeout });
                if let Some(path) = &state.prepared_probe {
                    let bytes = fs::read(path).expect("prepared journal must exist before send");
                    let record = JournalRecord::decode(&bytes)
                        .expect("prepared journal must be complete and checksummed before send");
                    assert_eq!(record.state(), JournalState::Prepared);
                    state.prepared_seen_before_send = true;
                }
                state.watch
            };
            Box::pin(async move {
                Ok(match watch {
                    FakeWatch::Finalized(hash) => WatchOutcome::Finalized(hash),
                    FakeWatch::RpcRejected => WatchOutcome::RpcRejected(ChainFailure::new(
                        "fake RPC rejection",
                        StaticFailure("fixture RPC rejection"),
                    )),
                    FakeWatch::Lost => WatchOutcome::Lost(ChainFailure::new(
                        "fake watcher loss",
                        StaticFailure("fixture watcher loss"),
                    )),
                    FakeWatch::Timeout => WatchOutcome::Timeout,
                })
            })
        }
    }

    fn account_info(nonce: u32) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(80);
        nonce.encode_to(&mut bytes);
        bytes.resize(80, 0);
        bytes
    }

    #[derive(Clone, Copy)]
    enum RawEventPhase {
        ApplyExtrinsic(u32),
        Finalization,
    }

    #[derive(Clone, Copy)]
    enum RawSystemDispatch {
        Success,
        BadOrigin,
        TrieInvalidStateRoot,
    }

    fn raw_system_dispatch_events(records: &[(RawEventPhase, RawSystemDispatch)]) -> Vec<u8> {
        assert!(
            records.len() < 64,
            "test vector uses one-byte Compact count"
        );
        let mut bytes = vec![(records.len() as u8) << 2];
        for (phase, dispatch) in records {
            match phase {
                RawEventPhase::ApplyExtrinsic(index) => {
                    bytes.push(0);
                    index.encode_to(&mut bytes);
                }
                RawEventPhase::Finalization => bytes.push(1),
            }
            // RuntimeEvent::System.
            bytes.push(0);
            match dispatch {
                RawSystemDispatch::Success => bytes.push(0),
                RawSystemDispatch::BadOrigin => {
                    // System::ExtrinsicFailed then DispatchError::BadOrigin.
                    bytes.extend([1, 2]);
                }
                RawSystemDispatch::TrieInvalidStateRoot => {
                    // System::ExtrinsicFailed, DispatchError::Trie, then the
                    // unit TrieError::InvalidStateRoot variant.
                    bytes.extend([1, 14, 0]);
                }
            }
            // DispatchEventInfo under the pinned metadata: Weight's ref_time
            // and proof_size are compact u64 values, followed by
            // DispatchClass::Normal and Pays::Yes.
            bytes.extend([0; 4]);
            // EventRecord topics: Vec<Hash>::new().
            bytes.push(0);
        }
        bytes
    }

    fn empty_block(number: u64) -> SubmissionBlock {
        let marker = u8::try_from(number % 251 + 1).expect("fixture marker fits u8");
        SubmissionBlock {
            number,
            hash: [marker; 32],
            raw_extrinsics: Vec::new(),
            raw_system_events: vec![0],
            extrinsic_hashes: Vec::new(),
            accepted_events: Vec::new(),
        }
    }

    fn decode_hex(value: &str) -> Vec<u8> {
        let value = value.trim();
        assert_eq!(value.len() % 2, 0, "hex fixture must have complete bytes");
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let high = hex_nibble(pair[0]);
                let low = hex_nibble(pair[1]);
                high << 4 | low
            })
            .collect()
    }

    const fn hex_nibble(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            _ => panic!("fixture hex must be canonical lowercase"),
        }
    }

    fn lower_hex(bytes: impl AsRef<[u8]>) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let bytes = bytes.as_ref();
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        output
    }

    #[cfg(target_os = "linux")]
    struct TestDirectory {
        path: PathBuf,
    }

    #[cfg(target_os = "linux")]
    impl TestDirectory {
        fn new(label: &str) -> Option<Self> {
            use std::os::unix::fs::DirBuilderExt;

            let root = PathBuf::from(std::env::var_os("CUBIKAN_TEST_SUPPORTED_ROOT")?);
            let root = fs::canonicalize(root).expect("canonicalize configured supported root");
            let path = root.join(format!("cubikan-submission-{}-{label}", std::process::id()));
            let _ = fs::remove_dir(&path);
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&path)
                .expect("create owner-only submission test directory");
            Some(Self { path })
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    #[cfg(target_os = "linux")]
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o700))
                .expect("restore submission test directory permissions");
            for entry in fs::read_dir(&self.path).expect("read submission test directory") {
                let entry = entry.expect("read submission test entry");
                fs::remove_file(entry.path()).expect("remove derived submission test file");
            }
            fs::remove_dir(&self.path).expect("remove submission test directory");
        }
    }

    #[cfg(not(target_os = "linux"))]
    struct TestDirectory;

    #[cfg(not(target_os = "linux"))]
    impl TestDirectory {
        fn new(_label: &str) -> Option<Self> {
            None
        }

        fn path(&self) -> &Path {
            Path::new("")
        }
    }

    fn completion_mutation() -> Mutation {
        Mutation::CompleteUnit {
            id: IntentUnitId::from_uuid(uuid::Uuid::from_bytes([
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff,
            ])),
            expected_revision: 0x0102_0304_0506_0708,
        }
    }

    fn all_mutations() -> Vec<Mutation> {
        let CanonicalPayload::UnitCreated(unit) =
            crate::decode_canonical_payload(&decode_hex(UNIT_CREATED_HEX))
                .expect("decode create-unit oracle")
        else {
            panic!("create-unit oracle has wrong variant")
        };
        let CanonicalPayload::UnitTransitioned {
            unit_id: transition_id,
            committed_revision: transition_revision,
            to,
            ..
        } = crate::decode_canonical_payload(&decode_hex(UNIT_TRANSITIONED_HEX))
            .expect("decode transition oracle")
        else {
            panic!("transition oracle has wrong variant")
        };
        let CanonicalPayload::UnitCompleted {
            unit_id: completion_id,
            committed_revision: completion_revision,
            ..
        } = crate::decode_canonical_payload(&decode_hex(UNIT_COMPLETED_HEX))
            .expect("decode completion oracle")
        else {
            panic!("completion oracle has wrong variant")
        };
        let CanonicalPayload::RelationshipDefinitionCreated(definition) =
            crate::decode_canonical_payload(&decode_hex(DEFINITION_HEX))
                .expect("decode definition oracle")
        else {
            panic!("definition oracle has wrong variant")
        };
        let CanonicalPayload::RelationshipCreated(relationship) =
            crate::decode_canonical_payload(&decode_hex(RELATIONSHIP_HEX))
                .expect("decode relationship oracle")
        else {
            panic!("relationship oracle has wrong variant")
        };
        let CanonicalPayload::AssociationRecorded(association) =
            crate::decode_canonical_payload(&decode_hex(ASSOCIATION_HEX))
                .expect("decode association oracle")
        else {
            panic!("association oracle has wrong variant")
        };
        vec![
            Mutation::CreateUnit {
                id: unit.id(),
                origin: unit.origin().clone(),
                species: unit.species().clone(),
                workflow: unit.workflow().clone(),
            },
            Mutation::TransitionUnit {
                id: transition_id,
                target: to,
                expected_revision: transition_revision - 1,
            },
            Mutation::CompleteUnit {
                id: completion_id,
                expected_revision: completion_revision - 1,
            },
            Mutation::CreateRelationshipDefinition(definition),
            Mutation::CreateRelationship(relationship.clone()),
            Mutation::DeleteRelationship(relationship),
            Mutation::RecordAssociation(association.clone()),
            Mutation::RevokeAssociation(association),
        ]
    }

    #[tokio::test]
    async fn test_submission_journal_is_durable_before_send() {
        assert_eq!(
            lower_hex(Sha256::digest(SIGNING_ORACLE)),
            "ed971f3032334f8d99de5d0a41000191f8985aea28c3c5b8277d1e0d195385b1"
        );
        let oracle: serde_json::Value =
            serde_json::from_slice(SIGNING_ORACLE).expect("decode independent signing oracle");
        assert_eq!(
            oracle["authority"]["signer_authorized_by_local_runtime"],
            false
        );
        assert_eq!(oracle["authority"]["production_submitters"][0], "Charlie");
        assert_eq!(oracle["authority"]["production_submitters"][1], "Dave");
        assert_ne!(DevSigner::Charlie.account_id(), ALICE);
        assert_ne!(DevSigner::Dave.account_id(), ALICE);
        assert_eq!(
            lower_hex(Sha256::digest(metadata_bytes())),
            "171a323b1e6bf0122e549eecd5f5932e672a3e0835f32edf0b8808cfefd97302"
        );
        let metadata = Arc::new(pinned_metadata().expect("decode pinned dry-run metadata"));
        assert_eq!(
            decode_dry_run(&[0, 0], metadata.clone()).expect("exact dry-run success"),
            DryRunOutcome::Valid
        );
        assert_eq!(
            decode_dry_run(&[0, 1, 2], metadata.clone())
                .expect("exact dry-run BadOrigin dispatch outcome"),
            DryRunOutcome::Valid
        );
        assert_eq!(
            decode_dry_run(&[0, 1, 14, 0], metadata.clone())
                .expect("metadata-valid dry-run Trie dispatch outcome"),
            DryRunOutcome::Valid
        );
        assert_eq!(
            decode_dry_run(&[1, 0, 1], metadata.clone()).expect("exact payment validity rejection"),
            DryRunOutcome::Invalid(SubmissionFailureCode::InsufficientBalance)
        );
        assert_eq!(
            decode_dry_run(&[1, 0, 2], metadata.clone())
                .expect("exact future nonce validity rejection"),
            DryRunOutcome::Invalid(SubmissionFailureCode::NonceConflict)
        );
        assert_eq!(
            decode_dry_run(&[1, 0, 12], metadata.clone())
                .expect("exact UnknownOrigin validity rejection"),
            DryRunOutcome::Invalid(SubmissionFailureCode::TransactionInvalid)
        );
        for malformed in [
            &[][..],
            &[0, 1][..],
            &[0, 0, 0][..],
            &[1, 0, 2, 0][..],
            &[1, 0, 13][..],
            &[1, 1, 3][..],
        ] {
            assert!(
                decode_dry_run(malformed, metadata.clone()).is_err(),
                "malformed/trailing dry-run response must fail closed: {malformed:?}"
            );
        }

        let call = decode_hex(CALL_HEX);
        let expected_payload = decode_hex(PAYLOAD_HEX);
        let signature_bytes: [u8; 64] = decode_hex(SIGNATURE_HEX)
            .try_into()
            .expect("signature fixture has 64 bytes");
        let signed_extrinsic = decode_hex(EXTRINSIC_HEX);
        let prepared_bytes = decode_hex(PREPARED_HEX);
        let identity = ChainIdentity {
            deployment_id: [0x30; 32],
            genesis_hash: ORACLE_GENESIS,
            spec_version: 1,
            transaction_version: 1,
        };
        let offline = offline_client(&identity).expect("instantiate pinned fixture client");
        let at = offline
            .at_block(131_u64)
            .expect("instantiate non-aligned signing block");
        let mutations = all_mutations();
        assert_eq!(mutations.len(), 8);
        for mutation in &mutations {
            assert_eq!(
                at.transactions()
                    .call_data(&dynamic_call(mutation))
                    .expect("encode dynamic mutation through pinned metadata"),
                manual_call_data(mutation).expect("independently encode mutation call"),
                "dynamic/manual call mismatch for {:?}",
                mutation.operation()
            );
        }
        assert_eq!(
            expected_payload_from_call_data(&at, &call, 66_051, 131, ORACLE_SIGNING_HASH,)
                .expect("independently encode fixture payload"),
            expected_payload
        );
        assert_eq!(
            expected_signer_payload(
                &at,
                &completion_mutation(),
                66_051,
                ChainHead {
                    number: 131,
                    hash: ORACLE_SIGNING_HASH,
                },
            )
            .expect("encode independent mutation payload"),
            expected_payload
        );
        assert!(sr25519::verify(
            &sr25519::Signature(signature_bytes),
            &expected_payload,
            &sr25519::PublicKey(ALICE),
        ));
        let inspected = inspect_signed_extrinsic(
            &offline,
            131,
            &signed_extrinsic,
            ALICE,
            Some(signature_bytes),
            66_051,
            131,
            MutationOperation::CompleteUnit,
            Some(&call),
        )
        .await
        .expect("decode exact independently signed extrinsic");
        assert_eq!(inspected.call_args, call[2..]);
        assert_eq!(
            lower_hex(at.hasher().hash(&signed_extrinsic)),
            "8dc2048ba261737a5d52bb1320df571e2aa3028c22783fc60a0dd021649cb5ca"
        );

        let prepared = JournalRecord::decode(&prepared_bytes)
            .expect("decode independent exact prepared record");
        assert_eq!(prepared.state(), JournalState::Prepared);
        assert_eq!(prepared.signer(), &ALICE);
        assert_eq!(prepared.nonce(), 66_051);
        assert_eq!(prepared.signing_block_number(), 131);
        assert_eq!(prepared.signing_block_hash(), &ORACLE_SIGNING_HASH);
        assert_eq!(prepared.birth(), 131);
        assert_eq!(prepared.death(), 194);
        assert_eq!(prepared.operation(), JournalOperation::CompleteUnit);
        assert_eq!(
            lower_hex(Sha256::digest(&prepared_bytes)),
            "0c89cb12fce2b96a94c8eb028ab53922c975e0a2e207c6b85c7840e99552a7d5"
        );
        assert_eq!(
            prepared
                .encode()
                .expect("re-encode prepared fixture")
                .as_slice(),
            prepared_bytes
        );

        let head = ChainHead {
            number: 131,
            hash: ORACLE_SIGNING_HASH,
        };
        let prepare_chain = FakeChain::new(head);
        let live_prepared = prepare_submission(
            &prepare_chain,
            DevSigner::Charlie,
            &completion_mutation(),
            head,
        )
        .await
        .expect("prepare production-authorized Charlie submission");
        assert_eq!(live_prepared.record.nonce(), 66_051);
        assert_eq!(live_prepared.record.birth(), 131);
        assert_eq!(live_prepared.record.death(), 194);
        {
            let prepare_state = prepare_chain.state();
            let prepare_trace = &prepare_state.trace;
            assert_eq!(prepare_trace.len(), 1);
            assert!(matches!(
                &prepare_trace[0],
                TraceCall::Storage { at, .. } if *at == ORACLE_SIGNING_HASH
            ));
        }

        let Some(directory) = TestDirectory::new("e2-durable-before-send") else {
            return;
        };
        let chain = FakeChain::new(head);
        let names = LaneNames::derive(
            directory.path(),
            &chain.identity.deployment_id,
            &DevSigner::Charlie.account_id(),
        )
        .expect("derive test lane names");
        let journal_path = directory.path().join(names.journal());
        chain.state().prepared_probe = Some(journal_path.clone());
        let result = submit_with_chain(
            &chain,
            directory.path(),
            DevSigner::Charlie,
            completion_mutation(),
            DEFAULT_FINALITY_WAIT,
        )
        .await
        .expect("submit through private deterministic chain");
        assert_eq!(
            result.outcome().kind(),
            SubmissionOutcomeKind::DeliveryIndeterminate
        );
        assert_eq!(
            result.outcome().failure_code(),
            Some(SubmissionFailureCode::SubmissionTimeout)
        );
        assert!(!result.requires_acknowledgement());
        let state = chain.state();
        assert!(state.prepared_seen_before_send);
        assert!(journal_path.is_file());
        assert_eq!(state.trace.len(), 4);
        assert_eq!(state.trace[0], TraceCall::FinalizedHead);
        assert!(matches!(
            &state.trace[1],
            TraceCall::Storage { at, .. } if *at == ORACLE_SIGNING_HASH
        ));
        assert_eq!(
            state.trace[2],
            TraceCall::DryRun {
                at: ORACLE_SIGNING_HASH
            }
        );
        assert_eq!(
            state.trace[3],
            TraceCall::SubmitAndWatch {
                timeout: DEFAULT_FINALITY_WAIT,
            }
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn completion_event(
        _prepared: &JournalRecord,
        extrinsic_index: u32,
        deployment_id: [u8; 32],
        event_schema_version: u16,
        signer: [u8; 32],
        extrinsic_hash: [u8; 32],
        committed_revision: u64,
    ) -> AcceptedEvent {
        let Mutation::CompleteUnit { id, .. } = completion_mutation() else {
            unreachable!("fixture mutation is complete_unit")
        };
        let phase = PhaseId::try_from("doing").expect("fixture phase is valid");
        let mut raw_payload = vec![2];
        encode_unit_id(id, &mut raw_payload);
        committed_revision.encode_to(&mut raw_payload);
        encode_text(phase.as_str(), &mut raw_payload).expect("encode fixture phase");
        AcceptedEvent::new(
            extrinsic_index,
            7,
            42,
            deployment_id,
            event_schema_version,
            signer,
            extrinsic_hash,
            raw_payload,
            CanonicalPayload::UnitCompleted {
                unit_id: id,
                committed_revision,
                phase,
            },
        )
    }

    #[tokio::test]
    async fn test_finalized_submission_outcomes_match_exact_extrinsic_and_event() {
        let prepared = JournalRecord::decode(&decode_hex(PREPARED_HEX))
            .expect("decode prepared classification fixture");
        let block_number = 137;
        let block_hash = [0x89; 32];
        let finalized_extrinsic = FinalizedExtrinsic {
            block_number,
            block_hash,
            extrinsic_index: 3,
            extrinsic_hash: *prepared.extrinsic_hash(),
        };
        let call = decode_hex(CALL_HEX);
        let call_args = &call[2..];
        let committed_revision = 0x0102_0304_0506_0709;
        let matching = completion_event(
            &prepared,
            finalized_extrinsic.extrinsic_index,
            *prepared.deployment_id(),
            EVENT_SCHEMA_VERSION,
            *prepared.signer(),
            *prepared.extrinsic_hash(),
            committed_revision,
        );

        // Exercise the exact raw metadata decoder used by
        // `RealSubmissionChain`, rather than trusting the fake chain's
        // pre-classified dispatch seam for acceptance-critical evidence.
        let dispatch_identity = ChainIdentity {
            deployment_id: *prepared.deployment_id(),
            genesis_hash: ORACLE_GENESIS,
            spec_version: 1,
            transaction_version: 1,
        };
        let raw_event_block = |raw_system_events| SubmissionBlock {
            number: block_number,
            hash: block_hash,
            raw_extrinsics: Vec::new(),
            raw_system_events,
            extrinsic_hashes: Vec::new(),
            accepted_events: Vec::new(),
        };
        let raw_success = raw_system_dispatch_events(&[(
            RawEventPhase::ApplyExtrinsic(finalized_extrinsic.extrinsic_index),
            RawSystemDispatch::Success,
        )]);
        assert_eq!(
            lower_hex(&raw_success),
            "04000300000000000000000000",
            "raw success bytes are an independent pinned EventRecord oracle"
        );
        let success = decode_finalized_dispatch_evidence(
            dispatch_identity,
            &raw_event_block(raw_success.clone()),
            finalized_extrinsic.extrinsic_index,
        )
        .expect("decode exact-index raw System::ExtrinsicSuccess");
        assert_eq!(success.successes, 1);
        assert!(success.errors.is_empty());

        let raw_failed = raw_system_dispatch_events(&[(
            RawEventPhase::ApplyExtrinsic(finalized_extrinsic.extrinsic_index),
            RawSystemDispatch::BadOrigin,
        )]);
        assert_eq!(
            lower_hex(&raw_failed),
            "0400030000000001020000000000",
            "raw failure bytes are an independent pinned EventRecord oracle"
        );
        let failed = decode_finalized_dispatch_evidence(
            dispatch_identity,
            &raw_event_block(raw_failed),
            finalized_extrinsic.extrinsic_index,
        )
        .expect("decode exact-index typed System::ExtrinsicFailed");
        assert_eq!(failed.successes, 0);
        assert!(matches!(
            failed.errors.as_slice(),
            [DecodedDispatchFailure::Known(DispatchError::BadOrigin)]
        ));

        let raw_trie = raw_system_dispatch_events(&[(
            RawEventPhase::ApplyExtrinsic(finalized_extrinsic.extrinsic_index),
            RawSystemDispatch::TrieInvalidStateRoot,
        )]);
        assert_eq!(
            lower_hex(&raw_trie),
            "04000300000000010e000000000000",
            "raw Trie failure bytes are an independent pinned EventRecord oracle"
        );
        let trie = decode_finalized_dispatch_evidence(
            dispatch_identity,
            &raw_event_block(raw_trie),
            finalized_extrinsic.extrinsic_index,
        )
        .expect("metadata-valid Trie failure must not depend on Subxt enum coverage");
        assert_eq!(trie.successes, 0);
        assert!(matches!(
            trie.errors.as_slice(),
            [DecodedDispatchFailure::RuntimeMismatch]
        ));
        let (trie_record, trie_outcome) = classify_finalized_evidence(
            &prepared,
            block_number,
            block_hash,
            finalized_extrinsic,
            MutationOperation::CompleteUnit,
            call_args,
            trie.successes,
            &trie.errors,
            &[],
        )
        .expect("resolve metadata-valid unsupported dispatch variant");
        assert_eq!(trie_record.state(), JournalState::FinalizedDispatchRejected);
        assert!(matches!(
            trie_outcome,
            SubmissionOutcomeDetail::FinalizedDispatchRejected {
                error: SubmissionFailureCode::RuntimeMismatch,
                ..
            }
        ));

        for ignored in [
            raw_system_dispatch_events(&[]),
            raw_system_dispatch_events(&[(
                RawEventPhase::ApplyExtrinsic(finalized_extrinsic.extrinsic_index + 1),
                RawSystemDispatch::Success,
            )]),
            raw_system_dispatch_events(&[(
                RawEventPhase::Finalization,
                RawSystemDispatch::Success,
            )]),
        ] {
            let evidence = decode_finalized_dispatch_evidence(
                dispatch_identity,
                &raw_event_block(ignored),
                finalized_extrinsic.extrinsic_index,
            )
            .expect("ignore missing, wrong-index, and wrong-phase dispatch records");
            assert_eq!(evidence.successes, 0);
            assert!(evidence.errors.is_empty());
        }

        let duplicate = decode_finalized_dispatch_evidence(
            dispatch_identity,
            &raw_event_block(raw_system_dispatch_events(&[
                (
                    RawEventPhase::ApplyExtrinsic(finalized_extrinsic.extrinsic_index),
                    RawSystemDispatch::Success,
                ),
                (
                    RawEventPhase::ApplyExtrinsic(finalized_extrinsic.extrinsic_index),
                    RawSystemDispatch::Success,
                ),
            ])),
            finalized_extrinsic.extrinsic_index,
        )
        .expect("retain duplicate exact-index success evidence for invariant classification");
        assert_eq!(duplicate.successes, 2);
        assert!(duplicate.errors.is_empty());

        let mut malformed = raw_success.clone();
        malformed.pop();
        assert!(
            decode_finalized_dispatch_evidence(
                dispatch_identity,
                &raw_event_block(malformed),
                finalized_extrinsic.extrinsic_index,
            )
            .is_err(),
            "truncated System::Events must fail closed"
        );
        let mut trailing = raw_success;
        trailing.push(0);
        assert!(
            decode_finalized_dispatch_evidence(
                dispatch_identity,
                &raw_event_block(trailing),
                finalized_extrinsic.extrinsic_index,
            )
            .is_err(),
            "trailing System::Events bytes must fail closed"
        );

        let mut exact_chain = FakeChain::new(ChainHead {
            number: block_number,
            hash: block_hash,
        });
        exact_chain.identity.deployment_id = *prepared.deployment_id();
        let exact_event = completion_event(
            &prepared,
            0,
            *prepared.deployment_id(),
            EVENT_SCHEMA_VERSION,
            *prepared.signer(),
            *prepared.extrinsic_hash(),
            committed_revision,
        );
        let exact_block = SubmissionBlock {
            number: block_number,
            hash: block_hash,
            raw_extrinsics: vec![decode_hex(EXTRINSIC_HEX)],
            raw_system_events: Vec::new(),
            extrinsic_hashes: vec![*prepared.extrinsic_hash()],
            accepted_events: vec![exact_event],
        };
        let (exact_resolved, exact_outcome) =
            resolve_in_block(&exact_chain, &prepared, exact_block.clone())
                .await
                .expect("join exact body hash/index and verify recovered signature");
        assert_eq!(exact_resolved.state(), JournalState::FinalizedAccepted);
        assert!(matches!(
            exact_outcome,
            SubmissionOutcomeDetail::FinalizedAccepted { .. }
        ));
        assert!(matches!(
            exact_chain.state().trace.as_slice(),
            [TraceCall::FinalizedDispatchEvidence {
                block: 137,
                extrinsic_index: 0,
            }]
        ));
        for dispatch in [
            FakeDispatch::Missing,
            FakeDispatch::DuplicateSuccess,
            FakeDispatch::Failed,
        ] {
            exact_chain.state().dispatch = dispatch;
            let (_, outcome) = resolve_in_block(&exact_chain, &prepared, exact_block.clone())
                .await
                .expect("classify exact-body contradictory dispatch evidence");
            assert!(matches!(
                outcome,
                SubmissionOutcomeDetail::FinalizedInvariantFailed { .. }
            ));
        }
        let mut duplicate_body = exact_block.clone();
        duplicate_body
            .raw_extrinsics
            .push(decode_hex(EXTRINSIC_HEX));
        duplicate_body
            .extrinsic_hashes
            .push(*prepared.extrinsic_hash());
        assert!(
            resolve_in_block(&exact_chain, &prepared, duplicate_body)
                .await
                .is_err(),
            "duplicate exact body hashes must not resolve"
        );

        let (record, accepted) = classify_finalized_evidence(
            &prepared,
            block_number,
            block_hash,
            finalized_extrinsic,
            MutationOperation::CompleteUnit,
            call_args,
            1,
            &[],
            std::slice::from_ref(&matching),
        )
        .expect("classify exact finalized acceptance");
        assert_eq!(record.state(), JournalState::FinalizedAccepted);
        match accepted {
            SubmissionOutcomeDetail::FinalizedAccepted {
                operation,
                coordinate,
                effect:
                    AcceptedEffect::UnitCompleted {
                        unit_id,
                        committed_revision: actual_revision,
                    },
            } => {
                assert_eq!(operation, MutationOperation::CompleteUnit);
                assert_eq!(coordinate.finalized_extrinsic, finalized_extrinsic);
                assert_eq!(coordinate.system_event_index, 7);
                assert_eq!(coordinate.global_sequence, 42);
                assert_eq!(actual_revision, committed_revision);
                assert_eq!(unit_id.as_uuid().as_bytes(), &call_args[2..18]);
            }
            other => panic!("unexpected accepted classification: {other:?}"),
        }

        let wrong_cases = [
            Vec::new(),
            vec![matching.clone(), matching.clone()],
            vec![completion_event(
                &prepared,
                finalized_extrinsic.extrinsic_index,
                [0x44; 32],
                EVENT_SCHEMA_VERSION,
                *prepared.signer(),
                *prepared.extrinsic_hash(),
                committed_revision,
            )],
            vec![completion_event(
                &prepared,
                finalized_extrinsic.extrinsic_index,
                *prepared.deployment_id(),
                EVENT_SCHEMA_VERSION + 1,
                *prepared.signer(),
                *prepared.extrinsic_hash(),
                committed_revision,
            )],
            vec![completion_event(
                &prepared,
                finalized_extrinsic.extrinsic_index,
                *prepared.deployment_id(),
                EVENT_SCHEMA_VERSION,
                [0x55; 32],
                *prepared.extrinsic_hash(),
                committed_revision,
            )],
            vec![completion_event(
                &prepared,
                finalized_extrinsic.extrinsic_index,
                *prepared.deployment_id(),
                EVENT_SCHEMA_VERSION,
                *prepared.signer(),
                [0x66; 32],
                committed_revision,
            )],
            vec![completion_event(
                &prepared,
                finalized_extrinsic.extrinsic_index,
                *prepared.deployment_id(),
                EVENT_SCHEMA_VERSION,
                *prepared.signer(),
                *prepared.extrinsic_hash(),
                committed_revision + 1,
            )],
            vec![completion_event(
                &prepared,
                finalized_extrinsic.extrinsic_index + 1,
                *prepared.deployment_id(),
                EVENT_SCHEMA_VERSION,
                *prepared.signer(),
                *prepared.extrinsic_hash(),
                committed_revision,
            )],
        ];
        for events in wrong_cases {
            let (record, outcome) = classify_finalized_evidence(
                &prepared,
                block_number,
                block_hash,
                finalized_extrinsic,
                MutationOperation::CompleteUnit,
                call_args,
                1,
                &[],
                &events,
            )
            .expect("classify successful inclusion invariant");
            assert_eq!(record.state(), JournalState::FinalizedInvariantFailed);
            assert!(matches!(
                outcome,
                SubmissionOutcomeDetail::FinalizedInvariantFailed {
                    operation: MutationOperation::CompleteUnit,
                    finalized_extrinsic: actual,
                } if actual == finalized_extrinsic
            ));
        }

        let dispatch_errors = [DecodedDispatchFailure::Known(DispatchError::BadOrigin)];
        let (record, rejected) = classify_finalized_evidence(
            &prepared,
            block_number,
            block_hash,
            finalized_extrinsic,
            MutationOperation::CompleteUnit,
            call_args,
            0,
            &dispatch_errors,
            &[],
        )
        .expect("classify exact finalized dispatch rejection");
        assert_eq!(record.state(), JournalState::FinalizedDispatchRejected);
        assert!(matches!(
            rejected,
            SubmissionOutcomeDetail::FinalizedDispatchRejected {
                operation: MutationOperation::CompleteUnit,
                finalized_extrinsic: actual,
                error: SubmissionFailureCode::UnsignedCall,
            } if actual == finalized_extrinsic
        ));

        let (record, outcome) = classify_finalized_evidence(
            &prepared,
            block_number,
            block_hash,
            finalized_extrinsic,
            MutationOperation::CompleteUnit,
            call_args,
            0,
            &dispatch_errors,
            std::slice::from_ref(&matching),
        )
        .expect("reject dispatch/event contradiction as invariant failure");
        assert_eq!(record.state(), JournalState::FinalizedInvariantFailed);
        assert!(matches!(
            outcome,
            SubmissionOutcomeDetail::FinalizedInvariantFailed { .. }
        ));

        let Some(rejection_directory) = TestDirectory::new("e4-dry-run-rejection") else {
            return;
        };
        let rejection_chain = FakeChain::new(ChainHead {
            number: 131,
            hash: ORACLE_SIGNING_HASH,
        });
        rejection_chain.state().dry_run =
            DryRunOutcome::Invalid(SubmissionFailureCode::NonceConflict);
        let rejected = submit_with_chain(
            &rejection_chain,
            rejection_directory.path(),
            DevSigner::Charlie,
            completion_mutation(),
            DEFAULT_FINALITY_WAIT,
        )
        .await
        .expect("return typed pre-send dry-run rejection");
        assert_eq!(
            rejected.outcome().kind(),
            SubmissionOutcomeKind::SubmissionRejected
        );
        assert_eq!(
            rejected.outcome().failure_code(),
            Some(SubmissionFailureCode::NonceConflict)
        );
        let rejection_names = LaneNames::derive(
            rejection_directory.path(),
            &rejection_chain.identity.deployment_id,
            &DevSigner::Charlie.account_id(),
        )
        .expect("derive rejected lane names");
        assert!(
            !rejection_directory
                .path()
                .join(rejection_names.journal())
                .exists()
        );
        assert!(
            !rejection_chain
                .state()
                .trace
                .iter()
                .any(|call| matches!(call, TraceCall::SubmitAndWatch { .. }))
        );

        let Some(resolution_directory) = TestDirectory::new("e4-durable-resolution") else {
            return;
        };
        let (resolved, detail) = classify_finalized_evidence(
            &prepared,
            block_number,
            block_hash,
            finalized_extrinsic,
            MutationOperation::CompleteUnit,
            call_args,
            1,
            &[],
            std::slice::from_ref(&matching),
        )
        .expect("rebuild exact accepted resolution for durable publication");
        let names = LaneNames::derive(
            resolution_directory.path(),
            prepared.deployment_id(),
            prepared.signer(),
        )
        .expect("derive durable resolution lane");
        let journal_path = resolution_directory.path().join(names.journal());
        let mut lane = SignerLane::open(
            resolution_directory.path(),
            *prepared.deployment_id(),
            *prepared.signer(),
        )
        .expect("open durable resolution lane");
        lane.publish_prepared(prepared.clone())
            .expect("publish prepared before terminal resolution");
        lane.publish_resolved(resolved)
            .expect("durably publish exact terminal resolution");
        assert_eq!(
            JournalRecord::decode(&fs::read(&journal_path).expect("read terminal journal"))
                .expect("decode terminal journal")
                .state(),
            JournalState::FinalizedAccepted
        );
        let result = result_with_ack(detail, lane);
        assert!(result.requires_acknowledgement());
        assert!(journal_path.exists());
        result
            .acknowledge_response_durable()
            .expect("acknowledge only after caller response durability");
        assert!(!journal_path.exists());
    }

    #[tokio::test]
    async fn test_unresolved_lane_scans_birth_through_death_without_sqlite_or_retry() {
        let reconciliation: serde_json::Value = serde_json::from_str(RECONCILIATION_ORACLE)
            .expect("decode independent reconciliation oracle");
        assert_eq!(reconciliation["persisted_operation"], "complete_unit");
        assert_eq!(
            reconciliation["incoming_operation_sentinel"],
            "create_relationship"
        );
        let cases = reconciliation["cases"]
            .as_array()
            .expect("reconciliation cases are an array");
        assert_eq!(cases.len(), 38, "consume the complete frozen case registry");
        for case in cases {
            for field in ["id", "kind", "precondition", "outcome", "journal_action"] {
                assert!(
                    case[field].as_str().is_some_and(|value| !value.is_empty()),
                    "every reconciliation case must define {field}"
                );
            }
        }
        let case_ids = cases
            .iter()
            .map(|case| case["id"].as_str().expect("case has id"))
            .collect::<Vec<_>>();
        for required in [
            "complete_birth_through_death_absence",
            "terminal_recovery_exact_unique_evidence",
            "terminal_evidence_unavailable",
            "terminal_evidence_duplicate_hash",
            "terminal_evidence_mismatch",
            "incoming_operation_b_cannot_replace_a",
            "watcher_invalid",
            "watcher_dropped",
            "watcher_error",
            "watcher_stream_end",
            "watcher_unknown_status",
            "watch_timeout",
            "transport_or_response_loss",
            "sqlite_claims_acceptance",
            "nonce_moved",
        ] {
            assert!(
                case_ids.contains(&required),
                "missing oracle case {required}"
            );
        }

        let prepared =
            JournalRecord::decode(&decode_hex(PREPARED_HEX)).expect("decode prepared scan fixture");
        let head = ChainHead {
            number: 195,
            hash: [0xc3; 32],
        };
        let chain = FakeChain::new(head);
        {
            let mut state = chain.state();
            for number in prepared.birth()..=prepared.death() {
                state.blocks.insert(number, empty_block(number));
            }
        }
        let inclusion = scan_prepared_history(&chain, &prepared, head)
            .await
            .expect("scan complete inclusive mortality window");
        assert!(inclusion.is_none());
        {
            let state = chain.state();
            let scanned = state
                .trace
                .iter()
                .filter_map(|call| match call {
                    TraceCall::FinalizedBlock(number) => Some(*number),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                scanned,
                (prepared.birth()..=prepared.death()).collect::<Vec<_>>()
            );
            assert!(!state.trace.iter().any(|call| matches!(
                call,
                TraceCall::Storage { .. }
                    | TraceCall::DryRun { .. }
                    | TraceCall::SubmitAndWatch { .. }
            )));
        }

        let inclusion_chain = FakeChain::new(head);
        {
            let mut state = inclusion_chain.state();
            for number in prepared.birth()..=prepared.death() {
                state.blocks.insert(number, empty_block(number));
            }
            let included = state.blocks.get_mut(&160).expect("fixture block 160");
            included.extrinsic_hashes.push(*prepared.extrinsic_hash());
        }
        let inclusion = scan_prepared_history(&inclusion_chain, &prepared, head)
            .await
            .expect("scan unique inclusion while retaining full duplicate check")
            .expect("find exact persisted extrinsic hash");
        assert_eq!(inclusion.number, 160);
        assert_eq!(
            inclusion_chain
                .state()
                .trace
                .iter()
                .filter(|call| matches!(call, TraceCall::FinalizedBlock(_)))
                .count(),
            64
        );

        let duplicate_chain = FakeChain::new(head);
        {
            let mut state = duplicate_chain.state();
            for number in prepared.birth()..=prepared.death() {
                state.blocks.insert(number, empty_block(number));
            }
            state
                .blocks
                .get_mut(&160)
                .expect("fixture block 160")
                .extrinsic_hashes
                .extend([*prepared.extrinsic_hash(), *prepared.extrinsic_hash()]);
        }
        assert!(
            scan_prepared_history(&duplicate_chain, &prepared, head)
                .await
                .is_err(),
            "duplicate exact hash evidence must retain the unresolved lane"
        );

        let terminal = JournalRecord::decode(&decode_hex(FINALIZED_ACCEPTED_HEX))
            .expect("decode terminal recovery fixture");
        let terminal_prepared = record_as_prepared(&terminal)
            .expect("reconstruct terminal record's prepared authority");
        let terminal_coordinate = FinalizedExtrinsic {
            block_number: terminal.resolution_block_number(),
            block_hash: *terminal.resolution_block_hash(),
            extrinsic_index: 0,
            extrinsic_hash: *terminal.extrinsic_hash(),
        };
        let (_, mismatched) = classify_finalized_evidence(
            &terminal_prepared,
            terminal.resolution_block_number(),
            *terminal.resolution_block_hash(),
            terminal_coordinate,
            MutationOperation::from_journal(terminal.operation()),
            &decode_hex(CALL_HEX)[2..],
            1,
            &[],
            &[],
        )
        .expect("classify unavailable accepted evidence");
        assert!(matches!(
            mismatched,
            SubmissionOutcomeDetail::FinalizedInvariantFailed { .. }
        ));
        assert_ne!(terminal.state(), JournalState::FinalizedInvariantFailed);

        let indeterminate = result_indeterminate(
            MutationOperation::CompleteUnit,
            *prepared.extrinsic_hash(),
            record_era(&prepared),
            SubmissionFailureCode::SubmissionWatchLost,
            Some(Box::new(ChainFailure::new(
                "fixture watcher loss",
                StaticFailure("preserved watcher source"),
            ))),
        );
        assert_eq!(
            indeterminate.outcome().kind(),
            SubmissionOutcomeKind::DeliveryIndeterminate
        );
        assert!(indeterminate.delivery_source().is_some());

        use subxt::rpcs::methods::legacy::TransactionStatus;
        for status in [
            TransactionStatus::Invalid,
            TransactionStatus::Dropped,
            TransactionStatus::Usurped(H256::from([0x44; 32])),
            TransactionStatus::FinalityTimeout(H256::from([0x55; 32])),
        ] {
            assert!(matches!(
                classify_watcher_status(status),
                Some(WatchOutcome::Lost(source)) if source.source().is_some()
            ));
        }
        assert!(classify_watcher_status(TransactionStatus::Ready).is_none());
        assert!(matches!(
            classify_watcher_status(TransactionStatus::Finalized(H256::from([0x66; 32]))),
            Some(WatchOutcome::Finalized(hash)) if hash == [0x66; 32]
        ));
        assert!(matches!(
            watcher_stream_ended(),
            WatchOutcome::Lost(source) if source.source().is_some()
        ));

        let watcher_chain = FakeChain::new(head);
        for (watch, expected) in [
            (
                FakeWatch::Finalized([0xaa; 32]),
                SubmissionFailureCode::RuntimeMismatch,
            ),
            (
                FakeWatch::RpcRejected,
                SubmissionFailureCode::RpcSubmissionRejected,
            ),
            (FakeWatch::Lost, SubmissionFailureCode::SubmissionWatchLost),
        ] {
            watcher_chain.state().watch = watch;
            let outcome = watcher_chain
                .submit_and_watch(Vec::new(), DEFAULT_FINALITY_WAIT)
                .await
                .expect("fake watcher call");
            match (outcome, expected) {
                (WatchOutcome::Finalized(hash), SubmissionFailureCode::RuntimeMismatch) => {
                    assert_eq!(hash, [0xaa; 32]);
                }
                (
                    WatchOutcome::RpcRejected(source),
                    SubmissionFailureCode::RpcSubmissionRejected,
                )
                | (WatchOutcome::Lost(source), SubmissionFailureCode::SubmissionWatchLost) => {
                    assert!(source.source().is_some())
                }
                other => panic!("unexpected watcher fixture: {other:?}"),
            }
        }

        for (label, watch, failure, expects_source) in [
            (
                "e5-timeout",
                FakeWatch::Timeout,
                SubmissionFailureCode::SubmissionTimeout,
                false,
            ),
            (
                "e5-rpc-rejected",
                FakeWatch::RpcRejected,
                SubmissionFailureCode::RpcSubmissionRejected,
                true,
            ),
            (
                "e5-watcher-lost",
                FakeWatch::Lost,
                SubmissionFailureCode::SubmissionWatchLost,
                true,
            ),
        ] {
            let Some(directory) = TestDirectory::new(label) else {
                break;
            };
            let delivery_chain = FakeChain::new(ChainHead {
                number: 131,
                hash: ORACLE_SIGNING_HASH,
            });
            delivery_chain.state().watch = watch;
            let names = LaneNames::derive(
                directory.path(),
                &delivery_chain.identity.deployment_id,
                &DevSigner::Charlie.account_id(),
            )
            .expect("derive indeterminate lane names");
            let journal_path = directory.path().join(names.journal());
            let result = submit_with_chain(
                &delivery_chain,
                directory.path(),
                DevSigner::Charlie,
                completion_mutation(),
                DEFAULT_FINALITY_WAIT,
            )
            .await
            .expect("return prepared delivery-indeterminate result");
            assert_eq!(
                result.outcome().kind(),
                SubmissionOutcomeKind::DeliveryIndeterminate
            );
            assert_eq!(result.outcome().failure_code(), Some(failure));
            assert_eq!(result.delivery_source().is_some(), expects_source);
            assert!(!result.requires_acknowledgement());
            assert!(journal_path.is_file());
            assert_eq!(
                delivery_chain
                    .state()
                    .trace
                    .iter()
                    .filter(|call| matches!(call, TraceCall::SubmitAndWatch { .. }))
                    .count(),
                1
            );
        }

        let Some(directory) = TestDirectory::new("e5-recovery") else {
            return;
        };
        let chain = FakeChain::new(head);
        let signer = DevSigner::Charlie;
        let persisted = JournalRecord::prepared(
            chain.identity.deployment_id,
            signer.account_id(),
            7,
            [0x99; 32],
            131,
            ORACLE_SIGNING_HASH,
            JournalOperation::CompleteUnit,
        )
        .expect("construct persisted operation-A record");
        let names = LaneNames::derive(
            directory.path(),
            &chain.identity.deployment_id,
            &signer.account_id(),
        )
        .expect("derive recovery lane names");
        let journal_path = directory.path().join(names.journal());
        {
            let mut lane = SignerLane::open(
                directory.path(),
                chain.identity.deployment_id,
                signer.account_id(),
            )
            .expect("open recovery signer lane");
            lane.publish_prepared(persisted.clone())
                .expect("publish persisted prepared record");
        }
        {
            let mut state = chain.state();
            for number in 131..=194 {
                state.blocks.insert(number, empty_block(number));
            }
        }
        let incoming_b = Mutation::TransitionUnit {
            id: IntentUnitId::from_uuid(uuid::Uuid::from_bytes([0x55; 16])),
            target: PhaseId::try_from("doing").expect("fixture target"),
            expected_revision: 0,
        };
        let result = submit_with_chain(
            &chain,
            directory.path(),
            signer,
            incoming_b.clone(),
            DEFAULT_FINALITY_WAIT,
        )
        .await
        .expect("reconcile persisted operation without retry");
        assert_eq!(
            result.outcome().kind(),
            SubmissionOutcomeKind::ExpiredNotIncluded
        );
        assert_eq!(
            result.outcome().operation(),
            MutationOperation::CompleteUnit
        );
        assert!(result.requires_acknowledgement());
        assert!(
            journal_path.is_file(),
            "terminal record remains before response durability"
        );
        {
            let state = chain.state();
            assert_eq!(state.trace.first(), Some(&TraceCall::FinalizedHead));
            assert_eq!(
                state
                    .trace
                    .iter()
                    .filter(|call| matches!(call, TraceCall::FinalizedBlock(_)))
                    .count(),
                64
            );
            assert!(!state.trace.iter().any(|call| matches!(
                call,
                TraceCall::Storage { .. }
                    | TraceCall::DryRun { .. }
                    | TraceCall::SubmitAndWatch { .. }
            )));
        }
        result
            .acknowledge_response_durable()
            .expect("remove terminal only after response durability acknowledgement");
        assert!(!journal_path.exists());

        let expired = persisted
            .resolved(JournalState::ExpiredNotIncluded, head.number, head.hash)
            .expect("construct persisted post-death terminal record");
        let Some(expired_directory) = TestDirectory::new("e5-expired-terminal-recovery") else {
            return;
        };
        let expired_names = LaneNames::derive(
            expired_directory.path(),
            expired.deployment_id(),
            expired.signer(),
        )
        .expect("derive expired-terminal recovery lane names");
        let expired_path = expired_directory.path().join(expired_names.journal());
        {
            let mut lane = SignerLane::open(
                expired_directory.path(),
                *expired.deployment_id(),
                *expired.signer(),
            )
            .expect("open expired-terminal recovery lane");
            lane.publish_prepared(persisted.clone())
                .expect("publish expired recovery prepared authority");
            lane.publish_resolved(expired.clone())
                .expect("publish expired terminal before simulated restart");
        }
        let expired_chain = FakeChain::new(head);
        {
            let mut state = expired_chain.state();
            for number in expired.birth()..=expired.death() {
                state.blocks.insert(number, empty_block(number));
            }
            let mut resolution = empty_block(expired.resolution_block_number());
            resolution.hash = *expired.resolution_block_hash();
            // The stored post-death block is a coordinate witness only. A
            // same-hash body entry there is outside the mortal era and must
            // not replace the inclusive birth..=death absence proof.
            resolution.extrinsic_hashes.push(*expired.extrinsic_hash());
            state
                .blocks
                .insert(expired.resolution_block_number(), resolution);
        }
        let recovered_expiry = submit_with_chain(
            &expired_chain,
            expired_directory.path(),
            signer,
            incoming_b.clone(),
            DEFAULT_FINALITY_WAIT,
        )
        .await
        .expect("reopen and verify persisted expiry without retry");
        assert_eq!(
            recovered_expiry.outcome().kind(),
            SubmissionOutcomeKind::ExpiredNotIncluded
        );
        assert_eq!(
            recovered_expiry.outcome().operation(),
            MutationOperation::CompleteUnit
        );
        assert!(recovered_expiry.requires_acknowledgement());
        assert!(expired_path.is_file());
        {
            let state = expired_chain.state();
            assert_eq!(
                state.trace.first(),
                Some(&TraceCall::FinalizedBlock(
                    expired.resolution_block_number()
                ))
            );
            let scanned = state
                .trace
                .iter()
                .skip(1)
                .filter_map(|call| match call {
                    TraceCall::FinalizedBlock(number) => Some(*number),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                scanned,
                (expired.birth()..=expired.death()).collect::<Vec<_>>(),
                "persisted expiry recovery rescans the inclusive original era"
            );
            assert!(!state.trace.iter().any(|call| matches!(
                call,
                TraceCall::FinalizedHead
                    | TraceCall::Storage { .. }
                    | TraceCall::DryRun { .. }
                    | TraceCall::SubmitAndWatch { .. }
            )));
        }
        recovered_expiry
            .acknowledge_response_durable()
            .expect("remove recovered expiry only after response durability");
        assert!(!expired_path.exists());

        #[derive(Clone, Copy)]
        enum ExpiredEvidenceCase {
            CoordinateUnavailable,
            CoordinateMismatch,
            InclusionAtDeath,
        }

        for (label, case) in [
            (
                "e5-expired-coordinate-unavailable",
                ExpiredEvidenceCase::CoordinateUnavailable,
            ),
            (
                "e5-expired-coordinate-mismatch",
                ExpiredEvidenceCase::CoordinateMismatch,
            ),
            (
                "e5-expired-inclusion-conflict",
                ExpiredEvidenceCase::InclusionAtDeath,
            ),
        ] {
            let Some(directory) = TestDirectory::new(label) else {
                break;
            };
            let names =
                LaneNames::derive(directory.path(), expired.deployment_id(), expired.signer())
                    .expect("derive negative expired recovery lane names");
            let journal_path = directory.path().join(names.journal());
            {
                let mut lane = SignerLane::open(
                    directory.path(),
                    *expired.deployment_id(),
                    *expired.signer(),
                )
                .expect("open negative expired recovery lane");
                lane.publish_prepared(persisted.clone())
                    .expect("publish negative expiry prepared authority");
                lane.publish_resolved(expired.clone())
                    .expect("publish negative expiry terminal");
            }
            let negative_chain = FakeChain::new(head);
            match case {
                ExpiredEvidenceCase::CoordinateUnavailable => {}
                ExpiredEvidenceCase::CoordinateMismatch => {
                    negative_chain.state().blocks.insert(
                        expired.resolution_block_number(),
                        empty_block(expired.resolution_block_number()),
                    );
                }
                ExpiredEvidenceCase::InclusionAtDeath => {
                    let mut state = negative_chain.state();
                    for number in expired.birth()..=expired.death() {
                        state.blocks.insert(number, empty_block(number));
                    }
                    state
                        .blocks
                        .get_mut(&expired.death())
                        .expect("inclusive death block exists")
                        .extrinsic_hashes
                        .push(*expired.extrinsic_hash());
                    let mut resolution = empty_block(expired.resolution_block_number());
                    resolution.hash = *expired.resolution_block_hash();
                    state
                        .blocks
                        .insert(expired.resolution_block_number(), resolution);
                }
            }
            let unresolved = submit_with_chain(
                &negative_chain,
                directory.path(),
                signer,
                incoming_b.clone(),
                DEFAULT_FINALITY_WAIT,
            )
            .await
            .expect("retain persisted expiry on contradictory/unavailable evidence");
            assert_eq!(
                unresolved.outcome().kind(),
                SubmissionOutcomeKind::SubmissionLaneUnresolved
            );
            assert_eq!(
                unresolved.outcome().operation(),
                MutationOperation::CompleteUnit
            );
            assert!(!unresolved.requires_acknowledgement());
            assert!(journal_path.is_file());
            let state = negative_chain.state();
            assert_eq!(
                state.trace.first(),
                Some(&TraceCall::FinalizedBlock(
                    expired.resolution_block_number()
                ))
            );
            if matches!(case, ExpiredEvidenceCase::InclusionAtDeath) {
                assert_eq!(
                    state
                        .trace
                        .iter()
                        .filter(|call| matches!(call, TraceCall::FinalizedBlock(_)))
                        .count(),
                    65,
                    "death-block inclusion conflict is found by the complete era rescan"
                );
            } else {
                assert_eq!(state.trace.len(), 1);
            }
            assert!(!state.trace.iter().any(|call| matches!(
                call,
                TraceCall::FinalizedHead
                    | TraceCall::Storage { .. }
                    | TraceCall::DryRun { .. }
                    | TraceCall::SubmitAndWatch { .. }
            )));
        }

        let signing_head = ChainHead {
            number: 131,
            hash: ORACLE_SIGNING_HASH,
        };
        let signing_chain = FakeChain::new(signing_head);
        let live = prepare_submission(
            &signing_chain,
            DevSigner::Charlie,
            &completion_mutation(),
            signing_head,
        )
        .await
        .expect("prepare exact terminal recovery extrinsic");
        let terminal_block_number = 137;
        let terminal_block_hash = [0x77; 32];
        let terminal_event = completion_event(
            &live.record,
            0,
            *live.record.deployment_id(),
            EVENT_SCHEMA_VERSION,
            *live.record.signer(),
            *live.record.extrinsic_hash(),
            0x0102_0304_0506_0709,
        );
        let terminal_block = SubmissionBlock {
            number: terminal_block_number,
            hash: terminal_block_hash,
            raw_extrinsics: vec![live.encoded.clone()],
            raw_system_events: Vec::new(),
            extrinsic_hashes: vec![*live.record.extrinsic_hash()],
            accepted_events: vec![terminal_event],
        };
        let terminal_chain = FakeChain::new(ChainHead {
            number: terminal_block_number,
            hash: terminal_block_hash,
        });
        let (terminal_record, terminal_detail) =
            resolve_in_block(&terminal_chain, &live.record, terminal_block.clone())
                .await
                .expect("construct exact durable accepted terminal");
        assert_eq!(terminal_record.state(), JournalState::FinalizedAccepted);
        assert!(matches!(
            terminal_detail,
            SubmissionOutcomeDetail::FinalizedAccepted { .. }
        ));

        let Some(directory) = TestDirectory::new("e5-terminal-recovery") else {
            return;
        };
        let terminal_names = LaneNames::derive(
            directory.path(),
            live.record.deployment_id(),
            live.record.signer(),
        )
        .expect("derive terminal recovery lane names");
        let terminal_path = directory.path().join(terminal_names.journal());
        {
            let mut lane = SignerLane::open(
                directory.path(),
                *live.record.deployment_id(),
                *live.record.signer(),
            )
            .expect("open terminal recovery lane");
            lane.publish_prepared(live.record.clone())
                .expect("publish terminal recovery prepared");
            lane.publish_resolved(terminal_record.clone())
                .expect("publish terminal recovery accepted");
        }
        let recovery_chain = FakeChain::new(ChainHead {
            number: terminal_block_number,
            hash: terminal_block_hash,
        });
        recovery_chain
            .state()
            .blocks
            .insert(terminal_block_number, terminal_block.clone());
        let recovered = submit_with_chain(
            &recovery_chain,
            directory.path(),
            DevSigner::Charlie,
            Mutation::TransitionUnit {
                id: IntentUnitId::from_uuid(uuid::Uuid::from_bytes([0x88; 16])),
                target: PhaseId::try_from("ignored-incoming-operation")
                    .expect("valid incoming sentinel phase"),
                expected_revision: 0,
            },
            DEFAULT_FINALITY_WAIT,
        )
        .await
        .expect("reconstruct accepted terminal without resend");
        assert_eq!(
            recovered.outcome().kind(),
            SubmissionOutcomeKind::FinalizedAccepted
        );
        assert_eq!(
            recovered.outcome().operation(),
            MutationOperation::CompleteUnit
        );
        assert!(matches!(
            recovered.outcome().effect(),
            Some(AcceptedEffect::UnitCompleted {
                committed_revision: 0x0102_0304_0506_0709,
                ..
            })
        ));
        assert!(terminal_path.exists());
        {
            let state = recovery_chain.state();
            assert!(!state.trace.iter().any(|call| matches!(
                call,
                TraceCall::Storage { .. }
                    | TraceCall::DryRun { .. }
                    | TraceCall::SubmitAndWatch { .. }
            )));
            assert_eq!(
                state
                    .trace
                    .iter()
                    .filter(|call| matches!(call, TraceCall::FinalizedBlock(137)))
                    .count(),
                1
            );
        }
        recovered
            .acknowledge_response_durable()
            .expect("acknowledge reconstructed terminal response");
        assert!(!terminal_path.exists());

        for (label, evidence) in [
            ("e5-terminal-unavailable", None),
            (
                "e5-terminal-duplicate",
                Some({
                    let mut block = terminal_block.clone();
                    block.raw_extrinsics.push(live.encoded.clone());
                    block.extrinsic_hashes.push(*live.record.extrinsic_hash());
                    block
                }),
            ),
            (
                "e5-terminal-mismatch",
                Some({
                    let mut block = terminal_block.clone();
                    block.accepted_events.clear();
                    block
                }),
            ),
        ] {
            let Some(directory) = TestDirectory::new(label) else {
                break;
            };
            let names = LaneNames::derive(
                directory.path(),
                live.record.deployment_id(),
                live.record.signer(),
            )
            .expect("derive negative recovery lane names");
            let journal_path = directory.path().join(names.journal());
            {
                let mut lane = SignerLane::open(
                    directory.path(),
                    *live.record.deployment_id(),
                    *live.record.signer(),
                )
                .expect("open negative recovery lane");
                lane.publish_prepared(live.record.clone())
                    .expect("publish negative recovery prepared");
                lane.publish_resolved(terminal_record.clone())
                    .expect("publish negative recovery terminal");
            }
            let negative_chain = FakeChain::new(ChainHead {
                number: terminal_block_number,
                hash: terminal_block_hash,
            });
            if let Some(block) = evidence {
                negative_chain
                    .state()
                    .blocks
                    .insert(terminal_block_number, block);
            }
            let unresolved = submit_with_chain(
                &negative_chain,
                directory.path(),
                DevSigner::Charlie,
                completion_mutation(),
                DEFAULT_FINALITY_WAIT,
            )
            .await
            .expect("retain terminal on unavailable/duplicate/mismatched evidence");
            assert_eq!(
                unresolved.outcome().kind(),
                SubmissionOutcomeKind::SubmissionLaneUnresolved
            );
            assert_eq!(
                unresolved.outcome().operation(),
                MutationOperation::CompleteUnit
            );
            assert!(!unresolved.requires_acknowledgement());
            assert!(journal_path.is_file());
            assert!(!negative_chain.state().trace.iter().any(|call| matches!(
                call,
                TraceCall::Storage { .. }
                    | TraceCall::DryRun { .. }
                    | TraceCall::SubmitAndWatch { .. }
            )));
        }
    }
}
