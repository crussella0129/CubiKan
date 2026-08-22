use std::{error::Error, ffi::OsStr, fmt, fs, path::Path};

use cubikan_backend::{
    AttestationError, BackendError, FinalizedProjector, GetIntentUnit, ListFilters,
    ListIntentUnits, PageLimit, ProjectionCheckpoint, ProjectionError, ReadError,
    RelationshipDefinitionId as BackendDefinitionId,
    RelationshipDefinitionKey as BackendDefinitionKey,
    RelationshipDefinitionVersion as BackendDefinitionVersion, VerifiedReadSnapshot,
    attest_finalized_projection,
};
use cubikan_chain_client::{
    AcceptedEffect, ArchiveError, ArchiveNodeEvidence, DevSigner, Mutation, NodeEvidenceError,
    StrictLoopbackWsUrl, SubmissionError, SubmissionErrorKind, SubmissionOutcome,
    SubmissionOutcomeKind, SubmissionResult, VerifiedArchiveClient, submit_finalized,
};

use crate::protocol::{
    AcceptedProjection, DecodedOperation, DecodedRequest, ErrorCode, ErrorDetail, ExecutedRequest,
    ResponseClass, association_page_response, decode_request, generic_error_response,
    intent_unit_page_response, intent_unit_response, projection_page_response,
    relationship_definition_response, relationship_page_response, submission_response,
};

/// Signals an argv form that can only be classified after the request is decoded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProcessUsage;

/// Source-preserving failure while clearing a resolved signer-lane journal.
#[derive(Debug)]
pub(crate) struct AcknowledgementError {
    source: Box<dyn Error + 'static>,
}

impl fmt::Display for AcknowledgementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("failed to acknowledge durable submission response")
    }
}

impl Error for AcknowledgementError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

trait DurableAcknowledgement {
    fn acknowledge(self: Box<Self>) -> Result<(), AcknowledgementError>;
}

struct SubmissionAcknowledgement(SubmissionResult);

impl DurableAcknowledgement for SubmissionAcknowledgement {
    fn acknowledge(self: Box<Self>) -> Result<(), AcknowledgementError> {
        self.0
            .acknowledge_response_durable()
            .map_err(|source| AcknowledgementError {
                source: Box::new(source),
            })
    }
}

/// One modeled response plus an optional resolved signer-lane acknowledgement.
///
/// The acknowledgement remains owned until the runner has written the body,
/// terminating LF, and final flush. Dropping this value before that point keeps
/// the resolved journal available for deterministic recovery.
pub(crate) struct Execution {
    response: ExecutedRequest,
    acknowledgement: Option<Box<dyn DurableAcknowledgement>>,
}

impl fmt::Debug for Execution {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Execution")
            .field("class", &self.response.class())
            .field("body_bytes", &self.response.body().len())
            .field("requires_acknowledgement", &self.acknowledgement.is_some())
            .finish()
    }
}

impl Execution {
    #[must_use]
    pub(crate) const fn modeled(response: ExecutedRequest) -> Self {
        Self {
            response,
            acknowledgement: None,
        }
    }

    fn with_acknowledgement(
        response: ExecutedRequest,
        acknowledgement: Option<Box<dyn DurableAcknowledgement>>,
    ) -> Self {
        Self {
            response,
            acknowledgement,
        }
    }

    #[must_use]
    pub(crate) fn body(&self) -> &[u8] {
        self.response.body()
    }

    #[must_use]
    pub(crate) const fn class(&self) -> ResponseClass {
        self.response.class()
    }

    pub(crate) fn acknowledge_response_durable(mut self) -> Result<(), AcknowledgementError> {
        match self.acknowledgement.take() {
            Some(acknowledgement) => acknowledgement.acknowledge(),
            None => Ok(()),
        }
    }
}

trait ExecutionServices {
    type Client;
    type Submission;

    async fn connect(&mut self, endpoint: StrictLoopbackWsUrl)
    -> Result<Self::Client, ErrorDetail>;

    async fn read(
        &mut self,
        client: &Self::Client,
        database_path: &Path,
        operation: DecodedOperation,
    ) -> Result<ExecutedRequest, ErrorDetail>;

    async fn submit(
        &mut self,
        client: &Self::Client,
        database_path: &Path,
        signer: DevSigner,
        mutation: Mutation,
    ) -> Result<Self::Submission, ErrorDetail>;

    async fn finish_submission(
        &mut self,
        client: &Self::Client,
        database_path: &Path,
        submission: Self::Submission,
    ) -> ServiceSubmission;
}

struct ServiceSubmission {
    response: ExecutedRequest,
    acknowledgement: Option<Box<dyn DurableAcknowledgement>>,
}

#[derive(Default)]
struct ProductionServices;

impl ExecutionServices for ProductionServices {
    type Client = VerifiedArchiveClient;
    type Submission = SubmissionResult;

    async fn connect(
        &mut self,
        endpoint: StrictLoopbackWsUrl,
    ) -> Result<Self::Client, ErrorDetail> {
        let evidence = discover_archive_node(&endpoint)?;
        VerifiedArchiveClient::connect(endpoint, evidence)
            .await
            .map_err(|error| archive_error(&error))
    }

    async fn read(
        &mut self,
        client: &Self::Client,
        database_path: &Path,
        operation: DecodedOperation,
    ) -> Result<ExecutedRequest, ErrorDetail> {
        execute_read(client, database_path, operation).await
    }

    async fn submit(
        &mut self,
        client: &Self::Client,
        database_path: &Path,
        signer: DevSigner,
        mutation: Mutation,
    ) -> Result<Self::Submission, ErrorDetail> {
        // This is intentionally the sole production call site for T-1111. In
        // particular, no projection path is statted, opened, created, or read
        // before this call has returned a terminal accepted outcome.
        let projection_directory = database_path
            .parent()
            .ok_or_else(|| ErrorDetail::plain(ErrorCode::InsecureProjectionPath))?;
        submit_finalized(client, projection_directory, signer, mutation)
            .await
            .map_err(|error| submission_error(&error))
    }

    async fn finish_submission(
        &mut self,
        client: &Self::Client,
        database_path: &Path,
        result: Self::Submission,
    ) -> ServiceSubmission {
        let projection = if result.outcome().kind() == SubmissionOutcomeKind::FinalizedAccepted {
            Some(accepted_projection(client, database_path, result.outcome()).await)
        } else {
            None
        };
        let response = submission_response(client.identity(), result.outcome(), projection);
        let acknowledgement = result.requires_acknowledgement().then(|| {
            Box::new(SubmissionAcknowledgement(result)) as Box<dyn DurableAcknowledgement>
        });
        ServiceSubmission {
            response,
            acknowledgement,
        }
    }
}

/// Decodes and executes one local-v2 request without exposing RPC, signer,
/// projection, or attested-capability authority through the public crate API.
pub(crate) async fn execute_request(
    database_path: &Path,
    rpc_text: &OsStr,
    signer_text: Option<&OsStr>,
    request: &[u8],
) -> Result<Execution, ProcessUsage> {
    let mut services = ProductionServices;
    execute_with_services(database_path, rpc_text, signer_text, request, &mut services).await
}

async fn execute_with_services<S: ExecutionServices>(
    database_path: &Path,
    rpc_text: &OsStr,
    signer_text: Option<&OsStr>,
    request: &[u8],
    services: &mut S,
) -> Result<Execution, ProcessUsage> {
    let decoded = match decode_request(request) {
        Ok(decoded) => decoded,
        Err(error) => return Ok(Execution::modeled(generic_error_response(error))),
    };
    execute_decoded_with_services(database_path, rpc_text, signer_text, decoded, services).await
}

async fn execute_decoded_with_services<S: ExecutionServices>(
    database_path: &Path,
    rpc_text: &OsStr,
    signer_text: Option<&OsStr>,
    decoded: DecodedRequest,
    services: &mut S,
) -> Result<Execution, ProcessUsage> {
    let is_mutation = decoded.operation().is_mutation();
    if is_mutation != signer_text.is_some() {
        return Err(ProcessUsage);
    }

    let endpoint = match rpc_text
        .to_str()
        .ok_or(())
        .and_then(|text| StrictLoopbackWsUrl::parse(text).map_err(|_| ()))
    {
        Ok(endpoint) => endpoint,
        Err(()) => {
            return Ok(Execution::modeled(generic_error_response(
                ErrorDetail::with_field(ErrorCode::InvalidRpcEndpoint, "/rpc"),
            )));
        }
    };
    let signer = if is_mutation {
        match signer_text.and_then(parse_dev_signer) {
            Some(signer) => Some(signer),
            None => return Err(ProcessUsage),
        }
    } else {
        None
    };

    let client = match services.connect(endpoint).await {
        Ok(client) => client,
        Err(error) => return Ok(Execution::modeled(generic_error_response(error))),
    };

    match decoded.into_operation() {
        DecodedOperation::Mutation(mutation) => {
            let submitted = match services
                .submit(
                    &client,
                    database_path,
                    signer.expect("mutation signer form was checked before connect"),
                    mutation,
                )
                .await
            {
                Ok(submission) => submission,
                Err(error) => return Ok(Execution::modeled(generic_error_response(error))),
            };
            let submission = services
                .finish_submission(&client, database_path, submitted)
                .await;
            Ok(Execution::with_acknowledgement(
                submission.response,
                submission.acknowledgement,
            ))
        }
        operation => match services.read(&client, database_path, operation).await {
            Ok(response) => Ok(Execution::modeled(response)),
            Err(error) => Ok(Execution::modeled(generic_error_response(error))),
        },
    }
}

fn parse_dev_signer(value: &OsStr) -> Option<DevSigner> {
    match value.to_str()? {
        "charlie" => Some(DevSigner::Charlie),
        "dave" => Some(DevSigner::Dave),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn discover_archive_node(
    endpoint: &StrictLoopbackWsUrl,
) -> Result<ArchiveNodeEvidence, ErrorDetail> {
    let entries =
        fs::read_dir("/proc").map_err(|_| ErrorDetail::plain(ErrorCode::UnsupportedPlatform))?;
    let mut pids = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|name| {
            name.parse::<u32>()
                .ok()
                .filter(|pid| pid.to_string() == name)
        })
        .collect::<Vec<_>>();
    pids.sort_unstable();
    pids.dedup();

    let mut matches = Vec::new();
    for pid in pids {
        match ArchiveNodeEvidence::from_proc_pid(pid, endpoint) {
            Ok(evidence) => matches.push(evidence),
            Err(NodeEvidenceError::UnsupportedPlatform) => {
                return Err(ErrorDetail::plain(ErrorCode::UnsupportedPlatform));
            }
            Err(_) => {}
        }
    }
    if matches.len() == 1 {
        Ok(matches
            .pop()
            .expect("one discovered archive node must exist"))
    } else {
        Err(ErrorDetail::plain(ErrorCode::ArchiveRpcUnavailable))
    }
}

#[cfg(not(target_os = "linux"))]
fn discover_archive_node(
    _endpoint: &StrictLoopbackWsUrl,
) -> Result<ArchiveNodeEvidence, ErrorDetail> {
    Err(ErrorDetail::plain(ErrorCode::UnsupportedPlatform))
}

async fn execute_read(
    client: &VerifiedArchiveClient,
    database_path: &Path,
    operation: DecodedOperation,
) -> Result<ExecutedRequest, ErrorDetail> {
    let projector = open_or_create_projector(database_path)?;
    projector
        .synchronize(client)
        .await
        .map_err(|error| projection_error(&error))?;
    let snapshot = attest_finalized_projection(&projector, client)
        .await
        .map_err(|error| attestation_error(&error))?;

    match operation {
        DecodedOperation::GetIntentUnit(command) => snapshot
            .get_intent_unit(command)
            .map(|result| intent_unit_response(&result))
            .map_err(|error| read_error(&error)),
        DecodedOperation::ListIntentUnits(command) => snapshot
            .list_intent_units(command)
            .map(|result| intent_unit_page_response(&result))
            .map_err(|error| read_error(&error)),
        DecodedOperation::GetRelationshipDefinition(key) => snapshot
            .get_relationship_definition(key)
            .map(|result| relationship_definition_response(&result))
            .map_err(|error| read_error(&error)),
        DecodedOperation::ListRelationships(query) => snapshot
            .list_relationships(query)
            .map(|result| relationship_page_response(&result))
            .map_err(|error| read_error(&error)),
        DecodedOperation::ProjectIntentUnitsV1(query) => snapshot
            .project_intent_units_v1(query)
            .map(|result| projection_page_response(&result))
            .map_err(|error| read_error(&error)),
        DecodedOperation::ListAssociationsByUnit(query) => snapshot
            .list_associations_by_unit(query)
            .map(|result| association_page_response(&result))
            .map_err(|error| read_error(&error)),
        DecodedOperation::ListAssociationsByReference(query) => snapshot
            .list_associations_by_reference(query)
            .map(|result| association_page_response(&result))
            .map_err(|error| read_error(&error)),
        DecodedOperation::Mutation(_) => Err(ErrorDetail::plain(ErrorCode::ProjectionError)),
    }
}

fn open_or_create_projector(path: &Path) -> Result<FinalizedProjector, ErrorDetail> {
    match fs::symlink_metadata(path) {
        Ok(_) => FinalizedProjector::open(path).map_err(|error| projection_error(&error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            FinalizedProjector::create(path).map_err(|error| projection_error(&error))
        }
        Err(_) => Err(ErrorDetail::plain(ErrorCode::InsecureProjectionPath)),
    }
}

async fn accepted_projection(
    client: &VerifiedArchiveClient,
    database_path: &Path,
    outcome: &SubmissionOutcome,
) -> AcceptedProjection {
    let Some(coordinate) = outcome.coordinate() else {
        return AcceptedProjection::lagging(None);
    };
    let Some(effect) = outcome.effect() else {
        return AcceptedProjection::lagging(None);
    };
    if coordinate.global_sequence() == 0 {
        return AcceptedProjection::lagging(None);
    }

    let mut services = ProductionProjectionServices;
    match resolve_accepted_projection(
        client,
        database_path,
        effect,
        coordinate.global_sequence(),
        &mut services,
    )
    .await
    {
        ProjectionResolution::CaughtUp(checkpoint) => AcceptedProjection::caught_up(checkpoint),
        ProjectionResolution::Lagging(checkpoint) => AcceptedProjection::lagging(checkpoint),
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ProjectionResolution<C> {
    CaughtUp(C),
    Lagging(Option<C>),
}

trait AcceptedProjectionServices<Client, Anchor> {
    type Projector;
    type Snapshot;
    type Checkpoint;

    fn open(&mut self, database_path: &Path) -> Result<Self::Projector, ()>;

    async fn synchronize(&mut self, projector: &Self::Projector, client: &Client)
    -> Result<(), ()>;

    async fn attest(
        &mut self,
        projector: &Self::Projector,
        client: &Client,
    ) -> Result<Self::Snapshot, ()>;

    fn semantic_anchor(
        &mut self,
        snapshot: Self::Snapshot,
        anchor: &Anchor,
    ) -> Result<Self::Checkpoint, ()>;

    fn global_sequence(checkpoint: &Self::Checkpoint) -> Option<u64>;

    async fn lagging_checkpoint(
        &mut self,
        projector: &Self::Projector,
        client: &Client,
    ) -> Option<Self::Checkpoint>;
}

async fn resolve_accepted_projection<Client, Anchor, Services>(
    client: &Client,
    database_path: &Path,
    anchor: &Anchor,
    accepted_sequence: u64,
    services: &mut Services,
) -> ProjectionResolution<Services::Checkpoint>
where
    Services: AcceptedProjectionServices<Client, Anchor>,
{
    let projector = match services.open(database_path) {
        Ok(projector) => projector,
        Err(()) => return ProjectionResolution::Lagging(None),
    };

    let caught_up = async {
        services.synchronize(&projector, client).await.ok()?;
        let snapshot = services.attest(&projector, client).await.ok()?;
        let checkpoint = services.semantic_anchor(snapshot, anchor).ok()?;
        Services::global_sequence(&checkpoint)
            .is_some_and(|sequence| sequence >= accepted_sequence)
            .then_some(checkpoint)
    }
    .await;
    if let Some(checkpoint) = caught_up {
        return ProjectionResolution::CaughtUp(checkpoint);
    }

    ProjectionResolution::Lagging(services.lagging_checkpoint(&projector, client).await)
}

struct ProductionProjectionServices;

impl AcceptedProjectionServices<VerifiedArchiveClient, AcceptedEffect>
    for ProductionProjectionServices
{
    type Projector = FinalizedProjector;
    type Snapshot = VerifiedReadSnapshot;
    type Checkpoint = ProjectionCheckpoint;

    fn open(&mut self, database_path: &Path) -> Result<Self::Projector, ()> {
        open_or_create_projector(database_path).map_err(|_| ())
    }

    async fn synchronize(
        &mut self,
        projector: &Self::Projector,
        client: &VerifiedArchiveClient,
    ) -> Result<(), ()> {
        projector
            .synchronize(client)
            .await
            .map(|_| ())
            .map_err(|_| ())
    }

    async fn attest(
        &mut self,
        projector: &Self::Projector,
        client: &VerifiedArchiveClient,
    ) -> Result<Self::Snapshot, ()> {
        attest_finalized_projection(projector, client)
            .await
            .map_err(|_| ())
    }

    fn semantic_anchor(
        &mut self,
        snapshot: Self::Snapshot,
        effect: &AcceptedEffect,
    ) -> Result<Self::Checkpoint, ()> {
        semantic_anchor_checkpoint(snapshot, effect).map_err(|_| ())
    }

    fn global_sequence(checkpoint: &Self::Checkpoint) -> Option<u64> {
        checkpoint
            .last_global_sequence()
            .map(|sequence| sequence.get())
    }

    async fn lagging_checkpoint(
        &mut self,
        projector: &Self::Projector,
        client: &VerifiedArchiveClient,
    ) -> Option<Self::Checkpoint> {
        probe_lagging_checkpoint(projector, client).await
    }
}

fn semantic_anchor_checkpoint(
    snapshot: VerifiedReadSnapshot,
    effect: &AcceptedEffect,
) -> Result<ProjectionCheckpoint, ReadError> {
    match effect {
        AcceptedEffect::UnitCreated { unit_id, .. }
        | AcceptedEffect::UnitTransitioned { unit_id, .. }
        | AcceptedEffect::UnitCompleted { unit_id, .. } => snapshot
            .get_intent_unit(GetIntentUnit::new(*unit_id))
            .map(|result| result.checkpoint().clone()),
        AcceptedEffect::RelationshipDefinitionCreated(definition) => {
            let key = backend_definition_key(definition.key())
                .map_err(|_| ReadError::ProjectionUnavailable)?;
            snapshot
                .get_relationship_definition(key)
                .map(|result| result.checkpoint().clone())
        }
        AcceptedEffect::RelationshipCreated(relationship)
        | AcceptedEffect::RelationshipDeleted(relationship) => snapshot
            .get_intent_unit(GetIntentUnit::new(relationship.source()))
            .map(|result| result.checkpoint().clone()),
        AcceptedEffect::AssociationRecorded(association)
        | AcceptedEffect::AssociationRevoked(association) => snapshot
            .get_intent_unit(GetIntentUnit::new(association.unit_id()))
            .map(|result| result.checkpoint().clone()),
    }
}

fn backend_definition_key(
    key: &cubikan_core::RelationshipDefinitionKey,
) -> Result<BackendDefinitionKey, ()> {
    let id = BackendDefinitionId::new(key.id().as_str()).map_err(|_| ())?;
    let version = BackendDefinitionVersion::new(key.version().value()).map_err(|_| ())?;
    Ok(BackendDefinitionKey::new(id, version))
}

async fn probe_lagging_checkpoint(
    projector: &FinalizedProjector,
    client: &VerifiedArchiveClient,
) -> Option<ProjectionCheckpoint> {
    let snapshot = attest_finalized_projection(projector, client).await.ok()?;
    let limit = PageLimit::new(1).expect("one is a valid projection page limit");
    snapshot
        .list_intent_units(ListIntentUnits::new(ListFilters::default(), limit, None))
        .ok()
        .map(|page| page.checkpoint().clone())
}

fn archive_error(error: &ArchiveError) -> ErrorDetail {
    let code = match error {
        ArchiveError::Identity(_)
        | ArchiveError::MalformedResponse(_)
        | ArchiveError::MalformedCanonicalPayload
        | ArchiveError::EventDecode { .. }
        | ArchiveError::OverBound(_) => ErrorCode::RuntimeMismatch,
        ArchiveError::NodeEvidence(NodeEvidenceError::UnsupportedPlatform) => {
            ErrorCode::UnsupportedPlatform
        }
        ArchiveError::NodeEvidence(_) | ArchiveError::NodeEvidenceEndpointMismatch => {
            ErrorCode::ArchiveRpcUnavailable
        }
        ArchiveError::Rpc { .. } | ArchiveError::Timeout { .. } => ErrorCode::ArchiveRpcUnavailable,
        ArchiveError::ArchiveHistoryUnavailable { .. } | ArchiveError::NotFinalized { .. } => {
            ErrorCode::ArchiveHistoryUnavailable
        }
        ArchiveError::DeploymentMismatch => ErrorCode::DeploymentMismatch,
        ArchiveError::RuntimeMismatch => ErrorCode::RuntimeMismatch,
        ArchiveError::UnsupportedEventSchemaVersion { .. } => {
            ErrorCode::UnsupportedEventSchemaVersion
        }
        ArchiveError::ForeignFinalizedHead
        | ArchiveError::DisplacedFinalizedHead
        | ArchiveError::NonContiguousFinalizedStream => ErrorCode::ConflictingFinalizedBlock,
    };
    ErrorDetail::plain(code)
}

fn backend_error(error: &BackendError) -> ErrorDetail {
    let code = match error {
        BackendError::IntentUnitNotFound { .. } => ErrorCode::ProjectionError,
        BackendError::UnownedDatabase | BackendError::InsecureProjectionPath => {
            ErrorCode::InsecureProjectionPath
        }
        BackendError::UnsupportedPlatform => ErrorCode::UnsupportedPlatform,
        BackendError::UnsupportedSchemaVersion { .. } => ErrorCode::UnsupportedSchemaVersion,
        BackendError::CorruptSchema => ErrorCode::CorruptSchema,
        BackendError::UnsupportedEnvelopeVersion { .. } => ErrorCode::UnsupportedEnvelopeVersion,
        BackendError::CorruptEnvelope => ErrorCode::CorruptEnvelope,
        BackendError::ProjectionMismatch => ErrorCode::ProjectionMismatch,
        BackendError::StorageBusy(_) => ErrorCode::ProjectionBusy,
        BackendError::ConcurrentStorageChange => ErrorCode::RefreshRequired,
        BackendError::DuplicateIntentUnit { .. }
        | BackendError::RevisionConflict(_)
        | BackendError::TransitionRejected(_)
        | BackendError::CompletionRejected(_)
        | BackendError::StorageFull(_)
        | BackendError::Storage(_) => ErrorCode::ProjectionError,
    };
    ErrorDetail::plain(code)
}

fn projection_error(error: &ProjectionError) -> ErrorDetail {
    match error {
        ProjectionError::Archive(error) => archive_error(error),
        ProjectionError::Backend(error) => backend_error(error),
        ProjectionError::ConflictingFinalizedBlock => {
            ErrorDetail::plain(ErrorCode::ConflictingFinalizedBlock)
        }
        ProjectionError::RefreshRequired => ErrorDetail::plain(ErrorCode::RefreshRequired),
        ProjectionError::InvalidFinalizedStream => ErrorDetail::plain(ErrorCode::ProjectionError),
    }
}

fn attestation_error(error: &AttestationError) -> ErrorDetail {
    match error {
        AttestationError::Archive(error) => archive_error(error),
        AttestationError::Backend(error) => backend_error(error),
        AttestationError::RefreshRequired => ErrorDetail::plain(ErrorCode::RefreshRequired),
        AttestationError::ProjectionMismatch => ErrorDetail::plain(ErrorCode::ProjectionMismatch),
        AttestationError::InvalidFinalizedStream | AttestationError::ProjectionUnavailable => {
            ErrorDetail::plain(ErrorCode::ProjectionError)
        }
    }
}

fn read_error(error: &ReadError) -> ErrorDetail {
    match error {
        ReadError::Backend(BackendError::IntentUnitNotFound { .. }) => {
            ErrorDetail::plain(ErrorCode::IntentUnitNotFound)
        }
        ReadError::Backend(error) => backend_error(error),
        ReadError::RefreshRequired => ErrorDetail::plain(ErrorCode::RefreshRequired),
        ReadError::ProjectionUnavailable => ErrorDetail::plain(ErrorCode::ProjectionError),
        ReadError::RelationshipDefinitionNotFound { .. } => {
            ErrorDetail::plain(ErrorCode::RelationshipDefinitionNotFound)
        }
    }
}

fn submission_error(error: &SubmissionError) -> ErrorDetail {
    let code = match error.kind() {
        SubmissionErrorKind::UnsupportedPlatform => ErrorCode::UnsupportedPlatform,
        SubmissionErrorKind::InsecureProjectionPath => ErrorCode::InsecureProjectionPath,
        SubmissionErrorKind::SubmissionLaneCorrupt => ErrorCode::SubmissionLaneCorrupt,
        SubmissionErrorKind::ArchiveRpcUnavailable => ErrorCode::ArchiveRpcUnavailable,
        SubmissionErrorKind::ArchiveHistoryUnavailable => ErrorCode::ArchiveHistoryUnavailable,
        SubmissionErrorKind::DeploymentMismatch => ErrorCode::DeploymentMismatch,
        SubmissionErrorKind::RuntimeMismatch | SubmissionErrorKind::ArithmeticOverflow => {
            ErrorCode::RuntimeMismatch
        }
        SubmissionErrorKind::DevSignerUnavailable => ErrorCode::DevSignerUnavailable,
        SubmissionErrorKind::AcknowledgementUnavailable => unreachable!(
            "acknowledgement is attempted only by the runner after body, LF, and flush"
        ),
    };
    ErrorDetail::plain(code)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{cell::Cell, collections::BTreeSet, ffi::OsStr, path::Path, rc::Rc};

    use serde_json::Value;

    use super::*;

    const EXPECTED_OPERATIONS: &[(&str, &str, &[&str])] = &[
        (
            "create_intent_unit",
            "mutation",
            &["type", "intent_unit", "workflow"],
        ),
        ("get_intent_unit", "read", &["type", "id"]),
        (
            "list_intent_units",
            "read",
            &["type", "filters", "limit", "after?"],
        ),
        (
            "transition_intent_unit",
            "mutation",
            &["type", "id", "target", "expected_revision"],
        ),
        (
            "complete_intent_unit",
            "mutation",
            &["type", "id", "expected_revision"],
        ),
        (
            "create_relationship_definition",
            "mutation",
            &[
                "type",
                "definition",
                "source_species?",
                "target_species?",
                "self_policy",
                "cycle_policy",
            ],
        ),
        (
            "get_relationship_definition",
            "read",
            &["type", "definition"],
        ),
        ("create_relationship", "mutation", &["type", "relationship"]),
        ("delete_relationship", "mutation", &["type", "relationship"]),
        (
            "list_relationships",
            "read",
            &[
                "type",
                "definition",
                "source_id?",
                "target_id?",
                "limit",
                "after?",
            ],
        ),
        (
            "project_intent_units_v1",
            "read",
            &[
                "type",
                "query_version",
                "filters",
                "predicate?",
                "limit",
                "after?",
            ],
        ),
        ("record_association", "mutation", &["type", "association"]),
        ("revoke_association", "mutation", &["type", "association"]),
        (
            "list_associations_by_unit",
            "read",
            &["type", "unit_id", "subject?", "limit", "after?"],
        ),
        (
            "list_associations_by_reference",
            "read",
            &["type", "reference", "limit", "after?"],
        ),
    ];

    const EXPECTED_OUTCOMES: &[&str] = &[
        "submission_rejected",
        "submission_lane_unresolved",
        "expired_not_included",
        "finalized_dispatch_rejected",
        "finalized_invariant_failed",
        "delivery_indeterminate",
        "finalized_accepted",
    ];

    const EXPECTED_EFFECTS: &[&str] = &[
        "unit_created",
        "unit_transitioned",
        "unit_completed",
        "relationship_definition_created",
        "relationship_created",
        "relationship_deleted",
        "association_recorded",
        "association_revoked",
    ];

    const REQUESTS: &[(&str, bool, &[u8])] = &[
        (
            "create_intent_unit",
            true,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_create_intent_unit.json"
            ),
        ),
        (
            "get_intent_unit",
            false,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_get_intent_unit.json"
            ),
        ),
        (
            "list_intent_units",
            false,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_list_intent_units.json"
            ),
        ),
        (
            "transition_intent_unit",
            true,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_transition_intent_unit.json"
            ),
        ),
        (
            "complete_intent_unit",
            true,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_complete_intent_unit.json"
            ),
        ),
        (
            "create_relationship_definition",
            true,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_create_relationship_definition.json"
            ),
        ),
        (
            "get_relationship_definition",
            false,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_get_relationship_definition.json"
            ),
        ),
        (
            "create_relationship",
            true,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_create_relationship.json"
            ),
        ),
        (
            "delete_relationship",
            true,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_delete_relationship.json"
            ),
        ),
        (
            "list_relationships",
            false,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_list_relationships.json"
            ),
        ),
        (
            "project_intent_units_v1",
            false,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_project_intent_units_v1.json"
            ),
        ),
        (
            "record_association",
            true,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_record_association.json"
            ),
        ),
        (
            "revoke_association",
            true,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_revoke_association.json"
            ),
        ),
        (
            "list_associations_by_unit",
            false,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_list_associations_by_unit.json"
            ),
        ),
        (
            "list_associations_by_reference",
            false,
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_list_associations_by_reference.json"
            ),
        ),
    ];

    struct SpyAcknowledgement(Rc<Cell<usize>>);

    impl DurableAcknowledgement for SpyAcknowledgement {
        fn acknowledge(self: Box<Self>) -> Result<(), AcknowledgementError> {
            self.0.set(self.0.get() + 1);
            Ok(())
        }
    }

    #[derive(Default)]
    struct SpyServices {
        calls: Vec<&'static str>,
        acknowledgements: Rc<Cell<usize>>,
    }

    impl ExecutionServices for SpyServices {
        type Client = ();
        type Submission = ();

        async fn connect(
            &mut self,
            _endpoint: StrictLoopbackWsUrl,
        ) -> Result<Self::Client, ErrorDetail> {
            self.calls.push("connect");
            Ok(())
        }

        async fn read(
            &mut self,
            _client: &Self::Client,
            _database_path: &Path,
            operation: DecodedOperation,
        ) -> Result<ExecutedRequest, ErrorDetail> {
            assert!(!operation.is_mutation());
            self.calls.push("sqlite_read");
            Ok(generic_error_response(ErrorDetail::plain(
                ErrorCode::ProjectionError,
            )))
        }

        async fn submit(
            &mut self,
            _client: &Self::Client,
            _database_path: &Path,
            _signer: DevSigner,
            _mutation: Mutation,
        ) -> Result<Self::Submission, ErrorDetail> {
            self.calls.push("submit_finalized");
            Ok(())
        }

        async fn finish_submission(
            &mut self,
            _client: &Self::Client,
            _database_path: &Path,
            _submission: Self::Submission,
        ) -> ServiceSubmission {
            self.calls.push("projection_after_terminal_acceptance");
            ServiceSubmission {
                response: generic_error_response(ErrorDetail::plain(ErrorCode::ProjectionError)),
                acknowledgement: Some(Box::new(SpyAcknowledgement(self.acknowledgements.clone()))),
            }
        }
    }

    struct ProjectionSpy {
        calls: Vec<&'static str>,
        synchronize: bool,
        attest: bool,
        anchor_sequence: Option<u64>,
        lagging_sequence: Option<u64>,
    }

    impl AcceptedProjectionServices<(), ()> for ProjectionSpy {
        type Projector = ();
        type Snapshot = ();
        type Checkpoint = u64;

        fn open(&mut self, _database_path: &Path) -> Result<Self::Projector, ()> {
            self.calls.push("open");
            Ok(())
        }

        async fn synchronize(
            &mut self,
            _projector: &Self::Projector,
            _client: &(),
        ) -> Result<(), ()> {
            self.calls.push("synchronize");
            self.synchronize.then_some(()).ok_or(())
        }

        async fn attest(
            &mut self,
            _projector: &Self::Projector,
            _client: &(),
        ) -> Result<Self::Snapshot, ()> {
            self.calls.push("attest");
            self.attest.then_some(()).ok_or(())
        }

        fn semantic_anchor(
            &mut self,
            _snapshot: Self::Snapshot,
            _anchor: &(),
        ) -> Result<Self::Checkpoint, ()> {
            self.calls.push("semantic_anchor");
            self.anchor_sequence.ok_or(())
        }

        fn global_sequence(checkpoint: &Self::Checkpoint) -> Option<u64> {
            Some(*checkpoint)
        }

        async fn lagging_checkpoint(
            &mut self,
            _projector: &Self::Projector,
            _client: &(),
        ) -> Option<Self::Checkpoint> {
            self.calls.push("fresh_attest_and_bounded_fallback");
            self.lagging_sequence
        }
    }

    #[tokio::test]
    async fn test_local_v2_has_exactly_fifteen_operations_and_one_submission_path() {
        let inventory: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/protocol-v2/cubikan-local/inventory-v1.json"
        ))
        .expect("frozen inventory must be valid JSON");
        let public_schema = inventory
            .get("public_schema")
            .expect("inventory must contain public_schema");
        assert_eq!(public_schema["operation_count"], 15);
        assert_eq!(public_schema["command_schema_version"], 1);
        assert_eq!(
            public_schema["caller_selectable_command_schema_version"],
            false
        );

        let operations = public_schema["operations"]
            .as_array()
            .expect("operations must be an array");
        assert_eq!(operations.len(), EXPECTED_OPERATIONS.len());
        for (actual, &(name, kind, fields)) in operations.iter().zip(EXPECTED_OPERATIONS) {
            assert_eq!(actual["name"], name);
            assert_eq!(actual["kind"], kind);
            let actual_fields = actual["fields"]
                .as_array()
                .expect("operation fields must be an array")
                .iter()
                .map(|field| field.as_str().expect("field name must be text"))
                .collect::<Vec<_>>();
            assert_eq!(actual_fields.as_slice(), fields, "field drift for {name}");
        }

        let mut services = SpyServices::default();
        for &(name, mutation, request) in REQUESTS {
            let decoded = decode_request(request)
                .unwrap_or_else(|_| panic!("frozen positive request must decode: {name}"));
            assert_eq!(
                decoded.operation().is_mutation(),
                mutation,
                "wrong route for {name}"
            );
            let signer = mutation.then_some(OsStr::new("charlie"));
            let execution = execute_with_services(
                Path::new("/no-sqlite-access-before-terminal-acceptance.sqlite"),
                OsStr::new("ws://127.0.0.1:9944/"),
                signer,
                request,
                &mut services,
            )
            .await
            .unwrap_or_else(|_| panic!("valid signer form must execute: {name}"));
            assert_eq!(execution.acknowledgement.is_some(), mutation);
            execution
                .acknowledge_response_durable()
                .unwrap_or_else(|_| panic!("spy acknowledgement must succeed: {name}"));
        }

        assert_eq!(
            services
                .calls
                .iter()
                .filter(|call| **call == "connect")
                .count(),
            15
        );
        assert_eq!(
            services
                .calls
                .iter()
                .filter(|call| **call == "submit_finalized")
                .count(),
            8
        );
        assert_eq!(
            services
                .calls
                .iter()
                .filter(|call| **call == "projection_after_terminal_acceptance")
                .count(),
            8
        );
        assert_eq!(
            services
                .calls
                .iter()
                .filter(|call| **call == "sqlite_read")
                .count(),
            7
        );
        assert_eq!(services.acknowledgements.get(), 8);

        let production = include_str!("execution.rs");
        assert_eq!(
            production.matches(concat!("submit_", "finalized(")).count(),
            1,
            "production must expose exactly one T-1111 submission call site"
        );
    }

    #[tokio::test]
    async fn test_local_v2_outcomes_are_exact_and_signing_never_reads_sqlite() {
        let inventory: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/protocol-v2/cubikan-local/inventory-v1.json"
        ))
        .expect("frozen inventory must be valid JSON");
        let public_schema = &inventory["public_schema"];
        assert_text_array_eq(
            &public_schema["mutation_outcomes"],
            EXPECTED_OUTCOMES,
            "mutation outcomes",
        );
        assert_text_array_eq(
            &public_schema["accepted_effect_tags"],
            EXPECTED_EFFECTS,
            "accepted effects",
        );

        let source_mapping: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/protocol-v2/cubikan-local/source-mapping-v1.json"
        ))
        .expect("frozen source mapping must be valid JSON");
        assert_source_mapping_inventory(&source_mapping);
        assert_projection_resolution_order().await;

        let mut services = SpyServices::default();
        let execution = execute_with_services(
            Path::new("/must-not-be-read-before-submit.sqlite"),
            OsStr::new("ws://127.0.0.1:9944/"),
            Some(OsStr::new("dave")),
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_complete_intent_unit.json"
            ),
            &mut services,
        )
        .await
        .expect("valid mutation signer form must execute");
        assert_eq!(
            services.calls,
            [
                "connect",
                "submit_finalized",
                "projection_after_terminal_acceptance"
            ]
        );
        assert!(execution.acknowledgement.is_some());
        assert_eq!(services.acknowledgements.get(), 0);
        execution
            .acknowledge_response_durable()
            .expect("post-flush acknowledgement spy must succeed");
        assert_eq!(services.acknowledgements.get(), 1);

        let mut invalid_signer_services = SpyServices::default();
        let invalid_signer = execute_with_services(
            Path::new("/must-not-be-read-for-invalid-signer.sqlite"),
            OsStr::new("ws://127.0.0.1:9944/"),
            Some(OsStr::new("Charlie")),
            include_bytes!(
                "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_complete_intent_unit.json"
            ),
            &mut invalid_signer_services,
        )
        .await;
        assert!(matches!(invalid_signer, Err(ProcessUsage)));
        assert!(invalid_signer_services.calls.is_empty());

        assert_eq!(
            backend_error(&missing_unit()).code(),
            ErrorCode::ProjectionError
        );
        assert_eq!(
            projection_error(&ProjectionError::Backend(missing_unit())).code(),
            ErrorCode::ProjectionError
        );
        assert_eq!(
            attestation_error(&AttestationError::Backend(missing_unit())).code(),
            ErrorCode::ProjectionError
        );
        assert_eq!(
            read_error(&ReadError::Backend(missing_unit())).code(),
            ErrorCode::IntentUnitNotFound
        );

        let production = include_str!("execution.rs");
        assert!(
            production.contains("SubmissionErrorKind::AcknowledgementUnavailable => unreachable!")
        );
    }

    async fn assert_projection_resolution_order() {
        let mut caught_up = ProjectionSpy {
            calls: Vec::new(),
            synchronize: true,
            attest: true,
            anchor_sequence: Some(9),
            lagging_sequence: Some(8),
        };
        assert_eq!(
            resolve_accepted_projection(&(), Path::new("/unused"), &(), 9, &mut caught_up).await,
            ProjectionResolution::CaughtUp(9)
        );
        assert_eq!(
            caught_up.calls,
            ["open", "synchronize", "attest", "semantic_anchor"]
        );

        let mut behind = ProjectionSpy {
            calls: Vec::new(),
            synchronize: true,
            attest: true,
            anchor_sequence: Some(8),
            lagging_sequence: Some(8),
        };
        assert_eq!(
            resolve_accepted_projection(&(), Path::new("/unused"), &(), 9, &mut behind).await,
            ProjectionResolution::Lagging(Some(8))
        );
        assert_eq!(
            behind.calls,
            [
                "open",
                "synchronize",
                "attest",
                "semantic_anchor",
                "fresh_attest_and_bounded_fallback"
            ]
        );

        let mut unattested = ProjectionSpy {
            calls: Vec::new(),
            synchronize: true,
            attest: false,
            anchor_sequence: Some(99),
            lagging_sequence: None,
        };
        assert_eq!(
            resolve_accepted_projection(&(), Path::new("/unused"), &(), 9, &mut unattested).await,
            ProjectionResolution::Lagging(None)
        );
        assert_eq!(
            unattested.calls,
            [
                "open",
                "synchronize",
                "attest",
                "fresh_attest_and_bounded_fallback"
            ]
        );
    }

    pub(crate) async fn assert_strict_url_dial_contract_for_e4() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/protocol-v2/cubikan-local/process-v1.json"
        ))
        .expect("frozen process inventory must be valid JSON");
        let invalid_rpc_line = include_bytes!(
            "../../../tests/fixtures/protocol-v2/cubikan-local/stdout/error_invalid_rpc_endpoint.jsonl"
        );
        let invalid_rpc_body = invalid_rpc_line
            .strip_suffix(b"\n")
            .expect("frozen stdout must end in one LF");
        let request = include_bytes!(
            "../../../tests/fixtures/protocol-v2/cubikan-local/requests/success_get_intent_unit.json"
        );
        let cases = fixture["cases"]
            .as_array()
            .expect("process cases must be an array");
        let mut accepted = 0_usize;
        let mut rejected = 0_usize;

        for case in cases {
            let id = case["id"].as_str().expect("case id must be text");
            if !id.starts_with("url_") {
                continue;
            }
            let argv = case["argv"].as_array().expect("argv must be an array");
            let rpc_index = argv
                .iter()
                .position(|argument| argument.as_str() == Some("--rpc"))
                .expect("URL case must contain --rpc");
            let rpc = argv[rpc_index + 1]
                .as_str()
                .expect("URL token must be text");
            let should_accept = id.starts_with("url_accept_");
            assert_eq!(
                StrictLoopbackWsUrl::parse(rpc).is_ok(),
                should_accept,
                "strict parser disagrees with {id}"
            );

            let mut services = SpyServices::default();
            let execution = execute_with_services(
                Path::new("/must-not-be-read-for-rejected-url.sqlite"),
                OsStr::new(rpc),
                None,
                request,
                &mut services,
            )
            .await
            .unwrap_or_else(|_| panic!("URL case has a valid read argv form: {id}"));
            if should_accept {
                accepted += 1;
                assert_eq!(
                    services
                        .calls
                        .iter()
                        .filter(|call| **call == "connect")
                        .count(),
                    1,
                    "accepted URL must reach connect exactly once: {id}"
                );
                assert_eq!(case["dial_count"], 1, "oracle dial count drift: {id}");
            } else {
                rejected += 1;
                assert!(
                    services.calls.is_empty(),
                    "rejected URL accessed services: {id}"
                );
                assert_eq!(
                    execution.body(),
                    invalid_rpc_body,
                    "wrong response for {id}"
                );
                let body: Value =
                    serde_json::from_slice(execution.body()).expect("response must be JSON");
                assert_eq!(body.pointer("/error/field"), Some(&Value::from("/rpc")));
                assert_eq!(case["dial_count"], 0, "oracle dial count drift: {id}");
            }
        }
        assert_eq!(accepted, 3);
        assert_eq!(rejected, 33);
    }

    fn missing_unit() -> BackendError {
        BackendError::IntentUnitNotFound {
            id: "00112233-4455-4677-8899-aabbccddeeff"
                .parse()
                .expect("fixture UUID must parse"),
        }
    }

    fn assert_text_array_eq(actual: &Value, expected: &[&str], label: &str) {
        let actual = actual
            .as_array()
            .unwrap_or_else(|| panic!("{label} must be an array"))
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .unwrap_or_else(|| panic!("{label} entries must be text"))
            })
            .collect::<Vec<_>>();
        assert_eq!(actual.as_slice(), expected, "{label} drifted");
    }

    fn assert_source_mapping_inventory(fixture: &Value) {
        let public = fixture["public_error_variant_mappings"]
            .as_array()
            .expect("public mappings must be an array");
        let proc = fixture["private_proc_discovery_mappings"]
            .as_array()
            .expect("proc mappings must be an array");
        assert_eq!(public.len(), 148);
        assert_eq!(proc.len(), 5);

        let mut sources = BTreeSet::new();
        for cell in public {
            let source = cell["source"]
                .as_str()
                .expect("public mapping source must be text");
            assert!(sources.insert(source), "duplicate public mapping: {source}");
            assert_mapping(cell, expected_public_mapping(source));
        }
        for cell in proc {
            let source = cell["source"]
                .as_str()
                .expect("proc mapping source must be text");
            assert!(sources.insert(source), "duplicate proc mapping: {source}");
            assert_mapping(cell, expected_proc_mapping(source));
        }
        assert_eq!(sources.len(), 153);
    }

    #[derive(Clone, Copy)]
    struct ExpectedMapping {
        disposition: &'static str,
        outcome: Option<&'static str>,
        code: Option<&'static str>,
        exit_code: Option<u64>,
        stderr: Option<&'static str>,
    }

    fn assert_mapping(cell: &Value, expected: ExpectedMapping) {
        let source = cell["source"].as_str().expect("source must be text");
        assert_eq!(
            cell["disposition"].as_str(),
            Some(expected.disposition),
            "disposition drift for {source}"
        );
        assert_eq!(
            cell["outcome"].as_str(),
            expected.outcome,
            "outcome drift for {source}"
        );
        assert_eq!(
            cell["code"].as_str(),
            expected.code,
            "code drift for {source}"
        );
        assert_eq!(
            cell["exit_code"].as_u64(),
            expected.exit_code,
            "exit drift for {source}"
        );
        assert_eq!(
            cell.get("stderr").and_then(Value::as_str),
            expected.stderr,
            "stderr drift for {source}"
        );
    }

    fn expected_public_mapping(source: &str) -> ExpectedMapping {
        if source == "SubmissionErrorKind::AcknowledgementUnavailable" {
            return ExpectedMapping {
                disposition: "operational_failure_after_stdout_flush",
                outcome: None,
                code: None,
                exit_code: Some(1),
                stderr: Some("cubikan-local: failed to acknowledge durable submission response\n"),
            };
        }

        let code = match source {
            "ReadMiss::IntentUnit" => "intent_unit_not_found",
            "ReadMiss::RelationshipDefinition" => "relationship_definition_not_found",
            "ProjectionError::ConflictingFinalizedBlock" => "conflicting_finalized_block",
            "ProjectionError::RefreshRequired" | "AttestationError::RefreshRequired" => {
                "refresh_required"
            }
            "ProjectionError::InvalidFinalizedStream"
            | "AttestationError::InvalidFinalizedStream"
            | "AttestationError::ProjectionUnavailable" => "projection_error",
            "AttestationError::ProjectionMismatch" => "projection_mismatch",
            "SubmissionErrorKind::UnsupportedPlatform" => "unsupported_platform",
            "SubmissionErrorKind::InsecureProjectionPath" => "insecure_projection_path",
            "SubmissionErrorKind::SubmissionLaneCorrupt" => "submission_lane_corrupt",
            "SubmissionErrorKind::ArchiveRpcUnavailable" => "archive_rpc_unavailable",
            "SubmissionErrorKind::ArchiveHistoryUnavailable" => "archive_history_unavailable",
            "SubmissionErrorKind::DeploymentMismatch" => "deployment_mismatch",
            "SubmissionErrorKind::RuntimeMismatch" | "SubmissionErrorKind::ArithmeticOverflow" => {
                "runtime_mismatch"
            }
            "SubmissionErrorKind::DevSignerUnavailable" => "dev_signer_unavailable",
            _ => archive_mapping_code(source)
                .or_else(|| backend_mapping_code(source))
                .unwrap_or_else(|| panic!("unclassified frozen public mapping: {source}")),
        };
        let exit_code = match code {
            "submission_lane_corrupt" | "dev_signer_unavailable" => 1,
            "intent_unit_not_found" | "relationship_definition_not_found" => 3,
            _ => 4,
        };
        ExpectedMapping {
            disposition: "modeled_error",
            outcome: Some("error"),
            code: Some(code),
            exit_code: Some(exit_code),
            stderr: None,
        }
    }

    fn archive_mapping_code(source: &str) -> Option<&'static str> {
        if !source.contains("ArchiveError::") {
            return None;
        }
        if source.contains("ArchiveError::NodeEvidence(NodeEvidenceError::UnsupportedPlatform)") {
            Some("unsupported_platform")
        } else if source.contains("ArchiveError::NodeEvidence(")
            || source.contains("ArchiveError::NodeEvidenceEndpointMismatch")
            || source.contains("ArchiveError::Rpc")
            || source.contains("ArchiveError::Timeout")
        {
            Some("archive_rpc_unavailable")
        } else if source.contains("ArchiveError::ArchiveHistoryUnavailable")
            || source.contains("ArchiveError::NotFinalized")
        {
            Some("archive_history_unavailable")
        } else if source.contains("ArchiveError::DeploymentMismatch") {
            Some("deployment_mismatch")
        } else if source.contains("ArchiveError::UnsupportedEventSchemaVersion") {
            Some("unsupported_event_schema_version")
        } else if source.contains("ArchiveError::ForeignFinalizedHead")
            || source.contains("ArchiveError::DisplacedFinalizedHead")
            || source.contains("ArchiveError::NonContiguousFinalizedStream")
        {
            Some("conflicting_finalized_block")
        } else if source.contains("ArchiveError::Identity(")
            || source.contains("ArchiveError::RuntimeMismatch")
            || source.contains("ArchiveError::MalformedResponse")
            || source.contains("ArchiveError::MalformedCanonicalPayload")
            || source.contains("ArchiveError::EventDecode")
            || source.contains("ArchiveError::OverBound")
        {
            Some("runtime_mismatch")
        } else {
            None
        }
    }

    fn backend_mapping_code(source: &str) -> Option<&'static str> {
        if !source.contains("BackendError::") {
            return None;
        }
        if source.contains("BackendError::UnownedDatabase")
            || source.contains("BackendError::InsecureProjectionPath")
        {
            Some("insecure_projection_path")
        } else if source.contains("BackendError::UnsupportedPlatform") {
            Some("unsupported_platform")
        } else if source.contains("BackendError::UnsupportedSchemaVersion") {
            Some("unsupported_schema_version")
        } else if source.contains("BackendError::CorruptSchema") {
            Some("corrupt_schema")
        } else if source.contains("BackendError::UnsupportedEnvelopeVersion") {
            Some("unsupported_envelope_version")
        } else if source.contains("BackendError::CorruptEnvelope") {
            Some("corrupt_envelope")
        } else if source.contains("BackendError::ProjectionMismatch") {
            Some("projection_mismatch")
        } else if source.contains("BackendError::StorageBusy") {
            Some("projection_busy")
        } else if source.contains("BackendError::ConcurrentStorageChange") {
            Some("refresh_required")
        } else if source.contains("BackendError::DuplicateIntentUnit")
            || source.contains("BackendError::IntentUnitNotFound")
            || source.contains("BackendError::RevisionConflict")
            || source.contains("BackendError::TransitionRejected")
            || source.contains("BackendError::CompletionRejected")
            || source.contains("BackendError::StorageFull")
            || source.contains("BackendError::Storage")
        {
            Some("projection_error")
        } else {
            None
        }
    }

    fn expected_proc_mapping(source: &str) -> ExpectedMapping {
        match source {
            "ProcDiscovery::NonLinux" | "ProcDiscovery::UnreadableRoot" => ExpectedMapping {
                disposition: "modeled_error",
                outcome: Some("error"),
                code: Some("unsupported_platform"),
                exit_code: Some(4),
                stderr: None,
            },
            "ProcDiscovery::ZeroSuccessfulArchiveProcesses"
            | "ProcDiscovery::MultipleSuccessfulArchiveProcesses" => ExpectedMapping {
                disposition: "modeled_error",
                outcome: Some("error"),
                code: Some("archive_rpc_unavailable"),
                exit_code: Some(4),
                stderr: None,
            },
            "ProcDiscovery::ExactlyOneSuccessfulArchiveProcess" => ExpectedMapping {
                disposition: "continue",
                outcome: None,
                code: None,
                exit_code: None,
                stderr: None,
            },
            _ => panic!("unclassified frozen proc mapping: {source}"),
        }
    }
}
