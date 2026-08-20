use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::OsStr,
    fmt,
    num::NonZeroU64,
    path::{Path, PathBuf},
};

use cubikan_chain_client::{ArchiveError, CanonicalPayload, VerifiedArchiveClient};
use cubikan_core::{
    AssociationSubject, IntentUnit, IntentUnitRevision, IntentUnitStatus, RecordedAssociation,
    RelationshipDefinition, RelationshipIdentity, RelationshipPolicy,
};
use rusqlite::{Connection, Row};

use crate::{
    BackendError, ProjectionCheckpoint,
    projection_store::{
        self, ProjectedBlock as StoredBlockInput, ProjectedEvent as StoredEventInput,
        ProjectedEventKind, ProjectionAnchor as StoredAnchorInput,
        ProjectionCheckpoint as StoredCheckpointInput, ProjectionWriter,
    },
    sqlite::{
        ProjectionWriterConnection, VerifiedQueryStatement, create_fresh_projection,
        open_projection_reader, open_projection_writer,
    },
    stored,
};

const EVENT_SCHEMA_VERSION: u16 = 1;
const MAX_SCALE_PAYLOAD_BYTES: usize = 1_048_576;
const MAX_RELATIONSHIPS_PER_DEFINITION: usize = 128;
const MAX_ASSOCIATIONS_PER_UNIT: usize = 128;

/// Failure while creating or advancing a finalized SQLite projection.
#[derive(Debug)]
pub enum ProjectionError {
    /// The configured verified archive client could not supply the complete
    /// finalized range.
    Archive(ArchiveError),
    /// The hardened local projection boundary rejected an operation.
    Backend(BackendError),
    /// A previously projected finalized height carries a different block hash
    /// from the verified archive range.
    ConflictingFinalizedBlock,
    /// The candidate finalized checkpoint moved before a database snapshot
    /// could be reconciled.
    RefreshRequired,
    /// The finalized range or its independent domain replay was inconsistent.
    InvalidFinalizedStream,
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Archive(error) => error.fmt(formatter),
            Self::Backend(error) => error.fmt(formatter),
            Self::ConflictingFinalizedBlock => formatter
                .write_str("the stored and archive-finalized block hashes conflict at one height"),
            Self::RefreshRequired => {
                formatter.write_str("the finalized projection changed before it was pinned")
            }
            Self::InvalidFinalizedStream => {
                formatter.write_str("the finalized CubiKan event stream is inconsistent")
            }
        }
    }
}

impl Error for ProjectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Archive(error) => Some(error),
            Self::Backend(error) => Some(error),
            Self::ConflictingFinalizedBlock
            | Self::RefreshRequired
            | Self::InvalidFinalizedStream => None,
        }
    }
}

impl From<ArchiveError> for ProjectionError {
    fn from(error: ArchiveError) -> Self {
        Self::Archive(error)
    }
}

impl From<BackendError> for ProjectionError {
    fn from(error: BackendError) -> Self {
        Self::Backend(error)
    }
}

/// A path-bound finalized projector.
///
/// Its only production input is a [`VerifiedArchiveClient`]. The type exposes
/// no event, row, checkpoint, SQL, or capability-construction seam.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedProjector {
    path: PathBuf,
}

impl FinalizedProjector {
    /// Creates a fresh schema-v3 projection with no anchor, block, event, or
    /// checkpoint rows. The first successful sync owns block-zero bootstrap.
    pub fn create(path: impl AsRef<Path>) -> Result<Self, ProjectionError> {
        let projector = Self::from_path(path.as_ref())?;
        let (directory, basename) = projector.parts()?;
        drop(create_fresh_projection(directory, basename)?);
        Ok(projector)
    }

    /// Opens and fully validates an existing schema-v3 projection path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectionError> {
        let projector = Self::from_path(path.as_ref())?;
        let (directory, basename) = projector.parts()?;
        drop(open_projection_writer(directory, basename)?);
        Ok(projector)
    }

    /// Fetches and independently replays the client's complete finalized
    /// archive range before atomically advancing the projection one block at a
    /// time.
    pub async fn synchronize(
        &self,
        client: &VerifiedArchiveClient,
    ) -> Result<ProjectionCheckpoint, ProjectionError> {
        let fetched = fetch_prepared_archive(client).await;
        synchronize_fetched(self, fetched)
    }

    fn from_path(path: &Path) -> Result<Self, ProjectionError> {
        if path.file_name().is_none() || path.parent().is_none() {
            return Err(BackendError::InsecureProjectionPath.into());
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub(crate) fn parts(&self) -> Result<(&Path, &OsStr), BackendError> {
        let directory = self
            .path
            .parent()
            .ok_or(BackendError::InsecureProjectionPath)?;
        let basename = self
            .path
            .file_name()
            .ok_or(BackendError::InsecureProjectionPath)?;
        if basename.is_empty() {
            return Err(BackendError::InsecureProjectionPath);
        }
        Ok((directory, basename))
    }
}

fn synchronize_fetched(
    projector: &FinalizedProjector,
    fetched: Result<PreparedArchive, ProjectionError>,
) -> Result<ProjectionCheckpoint, ProjectionError> {
    let archive = fetched?;
    synchronize_prepared(projector, &archive)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArchiveIdentity {
    pub(crate) relay_genesis_hash: [u8; 32],
    pub(crate) parachain_genesis_hash: [u8; 32],
    pub(crate) deployment_id: [u8; 32],
    pub(crate) initial_runtime_spec_version: u32,
    pub(crate) initial_runtime_code_hash: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceBlock {
    number: u64,
    hash: [u8; 32],
    parent_hash: [u8; 32],
    runtime_spec_version: u32,
    runtime_code_hash: [u8; 32],
    extrinsic_hashes: Vec<[u8; 32]>,
    system_event_record_count: u32,
    events: Vec<SourceEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceEvent {
    extrinsic_index: u32,
    system_event_index: u32,
    global_sequence: u64,
    deployment_id: [u8; 32],
    event_schema_version: u16,
    signer: [u8; 32],
    extrinsic_hash: [u8; 32],
    raw_scale_payload: Vec<u8>,
    payload: SourcePayload,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SourcePayload {
    UnitCreated(IntentUnit),
    UnitTransitioned {
        unit_id: cubikan_core::IntentUnitId,
        committed_revision: u64,
        from: cubikan_core::PhaseId,
        to: cubikan_core::PhaseId,
    },
    UnitCompleted {
        unit_id: cubikan_core::IntentUnitId,
        committed_revision: u64,
        phase: cubikan_core::PhaseId,
    },
    RelationshipDefinitionCreated(RelationshipDefinition),
    RelationshipCreated(RelationshipIdentity),
    RelationshipDeleted(RelationshipIdentity),
    AssociationRecorded(RecordedAssociation),
    AssociationRevoked(RecordedAssociation),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PreparedEffect {
    UnitCreated(IntentUnit),
    UnitUpdated {
        unit: IntentUnit,
        previous_revision: IntentUnitRevision,
    },
    RelationshipDefinitionCreated(RelationshipDefinition),
    RelationshipCreated(RelationshipIdentity),
    RelationshipDeleted(RelationshipIdentity),
    AssociationRecorded(RecordedAssociation),
    AssociationRevoked(RecordedAssociation),
}

impl PreparedEffect {
    const fn event_kind(&self) -> ProjectedEventKind {
        match self {
            Self::UnitCreated(_) => ProjectedEventKind::UnitCreated,
            Self::UnitUpdated { unit, .. } => match unit.status() {
                IntentUnitStatus::Active => ProjectedEventKind::UnitTransitioned,
                IntentUnitStatus::Completed => ProjectedEventKind::UnitCompleted,
            },
            Self::RelationshipDefinitionCreated(_) => {
                ProjectedEventKind::RelationshipDefinitionCreated
            }
            Self::RelationshipCreated(_) => ProjectedEventKind::RelationshipCreated,
            Self::RelationshipDeleted(_) => ProjectedEventKind::RelationshipDeleted,
            Self::AssociationRecorded(_) => ProjectedEventKind::AssociationRecorded,
            Self::AssociationRevoked(_) => ProjectedEventKind::AssociationRevoked,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreparedEvent {
    block_number: u64,
    block_hash: [u8; 32],
    extrinsic_index: u32,
    system_event_index: u32,
    global_sequence: u64,
    deployment_id: [u8; 32],
    signer: [u8; 32],
    extrinsic_hash: [u8; 32],
    raw_scale_payload: Vec<u8>,
    effect: PreparedEffect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreparedBlock {
    number: u64,
    hash: [u8; 32],
    parent_hash: [u8; 32],
    runtime_spec_version: u32,
    runtime_code_hash: [u8; 32],
    first_global_sequence: Option<u64>,
    last_global_sequence: Option<u64>,
    checkpoint_sequence: Option<u64>,
    events: Vec<PreparedEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PreparedArchive {
    pub(crate) identity: ArchiveIdentity,
    blocks: Vec<PreparedBlock>,
}

impl PreparedArchive {
    pub(crate) fn checkpoint(&self) -> Result<ProjectionCheckpoint, BackendError> {
        let block = self.blocks.last().ok_or(BackendError::ProjectionMismatch)?;
        let last_global_sequence = block
            .checkpoint_sequence
            .map(|sequence| {
                std::num::NonZeroU64::new(sequence).ok_or(BackendError::ProjectionMismatch)
            })
            .transpose()?;
        Ok(ProjectionCheckpoint::new(
            block.number,
            block.hash,
            last_global_sequence,
            block.runtime_spec_version,
            block.runtime_code_hash,
        ))
    }

    pub(crate) fn complete_expected_projection(&self) -> Result<StoredProjection, BackendError> {
        let mut expected = ExpectedProjection::new(self.identity.clone());
        for block in &self.blocks {
            expected.append(block)?;
        }
        expected.stored()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ReplayState {
    units: BTreeMap<String, UnitState>,
    definitions: BTreeMap<DefinitionKey, DefinitionState>,
    relationships: BTreeMap<RelationshipKey, RelationshipState>,
    associations: BTreeMap<AssociationKey, AssociationState>,
    last_global_sequence: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UnitState {
    unit: IntentUnit,
    accepted_global_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DefinitionState {
    definition: RelationshipDefinition,
    created_global_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RelationshipState {
    relationship: RelationshipIdentity,
    created_global_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AssociationState {
    association: RecordedAssociation,
    created_global_sequence: u64,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DefinitionKey {
    id: String,
    version: u64,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RelationshipKey {
    definition: DefinitionKey,
    source: String,
    target: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct AssociationKey {
    unit_id: String,
    subject_kind: String,
    subject_revision_key: Vec<u8>,
    namespace: String,
    scope: String,
    value: String,
}

impl ReplayState {
    fn apply(
        &mut self,
        payload: &SourcePayload,
        global_sequence: u64,
    ) -> Result<PreparedEffect, ProjectionError> {
        let expected_sequence = match self.last_global_sequence {
            Some(sequence) => sequence.checked_add(1),
            None => Some(1),
        };
        if expected_sequence != Some(global_sequence) {
            return Err(ProjectionError::InvalidFinalizedStream);
        }

        let effect = match payload {
            SourcePayload::UnitCreated(unit) => self.apply_unit_created(unit, global_sequence)?,
            SourcePayload::UnitTransitioned {
                unit_id,
                committed_revision,
                from,
                to,
            } => self.apply_unit_transition(
                *unit_id,
                *committed_revision,
                from,
                to,
                global_sequence,
            )?,
            SourcePayload::UnitCompleted {
                unit_id,
                committed_revision,
                phase,
            } => {
                self.apply_unit_completion(*unit_id, *committed_revision, phase, global_sequence)?
            }
            SourcePayload::RelationshipDefinitionCreated(definition) => {
                self.apply_definition_created(definition, global_sequence)?
            }
            SourcePayload::RelationshipCreated(relationship) => {
                self.apply_relationship_created(relationship, global_sequence)?
            }
            SourcePayload::RelationshipDeleted(relationship) => {
                self.apply_relationship_deleted(relationship)?
            }
            SourcePayload::AssociationRecorded(association) => {
                self.apply_association_recorded(association, global_sequence)?
            }
            SourcePayload::AssociationRevoked(association) => {
                self.apply_association_revoked(association)?
            }
        };
        self.last_global_sequence = Some(global_sequence);
        Ok(effect)
    }

    fn apply_unit_created(
        &mut self,
        unit: &IntentUnit,
        global_sequence: u64,
    ) -> Result<PreparedEffect, ProjectionError> {
        let id = unit.id().to_string();
        if unit.revision() != IntentUnitRevision::INITIAL
            || unit.status() != IntentUnitStatus::Active
            || !unit.history().is_empty()
            || unit.phase() != unit.workflow().initial_phase()
            || self.units.contains_key(&id)
        {
            return Err(ProjectionError::InvalidFinalizedStream);
        }
        self.units.insert(
            id,
            UnitState {
                unit: unit.clone(),
                accepted_global_sequence: global_sequence,
            },
        );
        Ok(PreparedEffect::UnitCreated(unit.clone()))
    }

    fn apply_unit_transition(
        &mut self,
        unit_id: cubikan_core::IntentUnitId,
        committed_revision: u64,
        from: &cubikan_core::PhaseId,
        to: &cubikan_core::PhaseId,
        global_sequence: u64,
    ) -> Result<PreparedEffect, ProjectionError> {
        let state = self
            .units
            .get_mut(&unit_id.to_string())
            .ok_or(ProjectionError::InvalidFinalizedStream)?;
        let previous_revision = state.unit.revision();
        if state.unit.phase() != from
            || previous_revision.value().checked_add(1) != Some(committed_revision)
            || state.unit.transition_to(to).is_err()
            || state.unit.revision().value() != committed_revision
        {
            return Err(ProjectionError::InvalidFinalizedStream);
        }
        state.accepted_global_sequence = global_sequence;
        Ok(PreparedEffect::UnitUpdated {
            unit: state.unit.clone(),
            previous_revision,
        })
    }

    fn apply_unit_completion(
        &mut self,
        unit_id: cubikan_core::IntentUnitId,
        committed_revision: u64,
        phase: &cubikan_core::PhaseId,
        global_sequence: u64,
    ) -> Result<PreparedEffect, ProjectionError> {
        let state = self
            .units
            .get_mut(&unit_id.to_string())
            .ok_or(ProjectionError::InvalidFinalizedStream)?;
        let previous_revision = state.unit.revision();
        if state.unit.phase() != phase
            || previous_revision.value().checked_add(1) != Some(committed_revision)
            || state.unit.complete().is_err()
            || state.unit.revision().value() != committed_revision
        {
            return Err(ProjectionError::InvalidFinalizedStream);
        }
        state.accepted_global_sequence = global_sequence;
        Ok(PreparedEffect::UnitUpdated {
            unit: state.unit.clone(),
            previous_revision,
        })
    }

    fn apply_definition_created(
        &mut self,
        definition: &RelationshipDefinition,
        global_sequence: u64,
    ) -> Result<PreparedEffect, ProjectionError> {
        let key = definition_key(definition);
        if self.definitions.contains_key(&key) {
            return Err(ProjectionError::InvalidFinalizedStream);
        }
        self.definitions.insert(
            key,
            DefinitionState {
                definition: definition.clone(),
                created_global_sequence: global_sequence,
            },
        );
        Ok(PreparedEffect::RelationshipDefinitionCreated(
            definition.clone(),
        ))
    }

    fn apply_relationship_created(
        &mut self,
        relationship: &RelationshipIdentity,
        global_sequence: u64,
    ) -> Result<PreparedEffect, ProjectionError> {
        let key = relationship_key(relationship);
        let definition = self
            .definitions
            .get(&key.definition)
            .ok_or(ProjectionError::InvalidFinalizedStream)?;
        let source = self
            .units
            .get(&key.source)
            .ok_or(ProjectionError::InvalidFinalizedStream)?;
        let target = self
            .units
            .get(&key.target)
            .ok_or(ProjectionError::InvalidFinalizedStream)?;
        if definition
            .definition
            .source_species()
            .is_some_and(|species| species != source.unit.species())
            || definition
                .definition
                .target_species()
                .is_some_and(|species| species != target.unit.species())
            || (key.source == key.target
                && definition.definition.self_policy() == RelationshipPolicy::Reject)
            || self.relationships.contains_key(&key)
            || self
                .relationships
                .keys()
                .filter(|candidate| candidate.definition == key.definition)
                .count()
                >= MAX_RELATIONSHIPS_PER_DEFINITION
            || (definition.definition.cycle_policy() == RelationshipPolicy::Reject
                && self.would_create_cycle(&key))
        {
            return Err(ProjectionError::InvalidFinalizedStream);
        }
        self.relationships.insert(
            key,
            RelationshipState {
                relationship: relationship.clone(),
                created_global_sequence: global_sequence,
            },
        );
        Ok(PreparedEffect::RelationshipCreated(relationship.clone()))
    }

    fn apply_relationship_deleted(
        &mut self,
        relationship: &RelationshipIdentity,
    ) -> Result<PreparedEffect, ProjectionError> {
        let key = relationship_key(relationship);
        let definition = self
            .definitions
            .get(&key.definition)
            .ok_or(ProjectionError::InvalidFinalizedStream)?;
        let source = self
            .units
            .get(&key.source)
            .ok_or(ProjectionError::InvalidFinalizedStream)?;
        let target = self
            .units
            .get(&key.target)
            .ok_or(ProjectionError::InvalidFinalizedStream)?;
        if definition
            .definition
            .source_species()
            .is_some_and(|species| species != source.unit.species())
            || definition
                .definition
                .target_species()
                .is_some_and(|species| species != target.unit.species())
            || self.relationships.remove(&key).is_none()
        {
            return Err(ProjectionError::InvalidFinalizedStream);
        }
        Ok(PreparedEffect::RelationshipDeleted(relationship.clone()))
    }

    fn apply_association_recorded(
        &mut self,
        association: &RecordedAssociation,
        global_sequence: u64,
    ) -> Result<PreparedEffect, ProjectionError> {
        let key = association_key(association);
        let unit = self
            .units
            .get(&key.unit_id)
            .ok_or(ProjectionError::InvalidFinalizedStream)?;
        if matches!(association.subject(), AssociationSubject::Revision(revision) if revision > unit.unit.revision().value())
            || self.associations.contains_key(&key)
            || self
                .associations
                .keys()
                .filter(|candidate| candidate.unit_id == key.unit_id)
                .count()
                >= MAX_ASSOCIATIONS_PER_UNIT
        {
            return Err(ProjectionError::InvalidFinalizedStream);
        }
        self.associations.insert(
            key,
            AssociationState {
                association: association.clone(),
                created_global_sequence: global_sequence,
            },
        );
        Ok(PreparedEffect::AssociationRecorded(association.clone()))
    }

    fn apply_association_revoked(
        &mut self,
        association: &RecordedAssociation,
    ) -> Result<PreparedEffect, ProjectionError> {
        let key = association_key(association);
        let unit = self
            .units
            .get(&key.unit_id)
            .ok_or(ProjectionError::InvalidFinalizedStream)?;
        if matches!(association.subject(), AssociationSubject::Revision(revision) if revision > unit.unit.revision().value())
            || self.associations.remove(&key).is_none()
        {
            return Err(ProjectionError::InvalidFinalizedStream);
        }
        Ok(PreparedEffect::AssociationRevoked(association.clone()))
    }

    fn would_create_cycle(&self, candidate: &RelationshipKey) -> bool {
        let mut frontier = vec![candidate.target.clone()];
        let mut visited = BTreeSet::new();
        while let Some(node) = frontier.pop() {
            if node == candidate.source {
                return true;
            }
            if !visited.insert(node.clone()) {
                continue;
            }
            frontier.extend(
                self.relationships
                    .keys()
                    .filter(|relationship| {
                        relationship.definition == candidate.definition
                            && relationship.source == node
                    })
                    .map(|relationship| relationship.target.clone()),
            );
        }
        false
    }
}

fn definition_key(definition: &RelationshipDefinition) -> DefinitionKey {
    DefinitionKey {
        id: definition.key().id().as_str().to_owned(),
        version: definition.key().version().value(),
    }
}

fn relationship_key(relationship: &RelationshipIdentity) -> RelationshipKey {
    RelationshipKey {
        definition: DefinitionKey {
            id: relationship.definition().id().as_str().to_owned(),
            version: relationship.definition().version().value(),
        },
        source: relationship.source().to_string(),
        target: relationship.target().to_string(),
    }
}

fn association_key(association: &RecordedAssociation) -> AssociationKey {
    let (subject_kind, subject_revision_key) = match association.subject() {
        AssociationSubject::WholeUnit => ("whole_unit".to_owned(), Vec::new()),
        AssociationSubject::Revision(revision) => (
            "revision".to_owned(),
            stored::encode_u64_blob(revision).to_vec(),
        ),
    };
    AssociationKey {
        unit_id: association.unit_id().to_string(),
        subject_kind,
        subject_revision_key,
        namespace: association.reference().namespace().as_str().to_owned(),
        scope: association.reference().scope().as_str().to_owned(),
        value: association.reference().value().as_str().to_owned(),
    }
}

fn prepare_archive(
    identity: ArchiveIdentity,
    blocks: Vec<SourceBlock>,
) -> Result<PreparedArchive, ProjectionError> {
    if blocks.is_empty() {
        return Err(ProjectionError::InvalidFinalizedStream);
    }
    let mut replay = ReplayState::default();
    let mut prepared = Vec::with_capacity(blocks.len());
    let mut previous_hash = None;

    for (offset, block) in blocks.into_iter().enumerate() {
        let expected_number =
            u64::try_from(offset).map_err(|_| ProjectionError::InvalidFinalizedStream)?;
        if block.number != expected_number
            || (block.number == 0 && block.hash != identity.parachain_genesis_hash)
            || (block.number == 0 && block.parent_hash != [0_u8; 32])
            || (block.number == 0
                && (!block.events.is_empty() || block.system_event_record_count != 0))
            || previous_hash.is_some_and(|hash| block.parent_hash != hash)
            || block.runtime_spec_version != identity.initial_runtime_spec_version
            || block.runtime_code_hash != identity.initial_runtime_code_hash
            || u32::try_from(block.events.len()).is_err()
            || u32::try_from(block.events.len())
                .is_ok_and(|count| count > block.system_event_record_count)
        {
            return Err(ProjectionError::InvalidFinalizedStream);
        }

        let mut previous_coordinate = None;
        let mut events = Vec::with_capacity(block.events.len());
        for event in block.events {
            let coordinate = (event.extrinsic_index, event.system_event_index);
            if previous_coordinate.is_some_and(|(previous_extrinsic, previous_system)| {
                event.extrinsic_index < previous_extrinsic
                    || event.system_event_index <= previous_system
            }) || event.deployment_id != identity.deployment_id
                || event.event_schema_version != EVENT_SCHEMA_VERSION
                || event.raw_scale_payload.is_empty()
                || event.raw_scale_payload.len() > MAX_SCALE_PAYLOAD_BYTES
                || event.system_event_index >= block.system_event_record_count
                || usize::try_from(event.extrinsic_index)
                    .ok()
                    .and_then(|index| block.extrinsic_hashes.get(index))
                    != Some(&event.extrinsic_hash)
            {
                return Err(ProjectionError::InvalidFinalizedStream);
            }
            let effect = replay.apply(&event.payload, event.global_sequence)?;
            events.push(PreparedEvent {
                block_number: block.number,
                block_hash: block.hash,
                extrinsic_index: event.extrinsic_index,
                system_event_index: event.system_event_index,
                global_sequence: event.global_sequence,
                deployment_id: event.deployment_id,
                signer: event.signer,
                extrinsic_hash: event.extrinsic_hash,
                raw_scale_payload: event.raw_scale_payload,
                effect,
            });
            previous_coordinate = Some(coordinate);
        }
        let first_global_sequence = events.first().map(|event| event.global_sequence);
        let last_global_sequence = events.last().map(|event| event.global_sequence);
        prepared.push(PreparedBlock {
            number: block.number,
            hash: block.hash,
            parent_hash: block.parent_hash,
            runtime_spec_version: block.runtime_spec_version,
            runtime_code_hash: block.runtime_code_hash,
            first_global_sequence,
            last_global_sequence,
            checkpoint_sequence: replay.last_global_sequence,
            events,
        });
        previous_hash = Some(block.hash);
    }

    Ok(PreparedArchive {
        identity,
        blocks: prepared,
    })
}

/// Private materialized-source seam. Production reaches it only after the
/// chain client has completed archive preflight; unit tests use a generic fake
/// without exposing caller-made blocks through the crate API.
trait ArchiveSource {
    fn identity(&self) -> &ArchiveIdentity;
    fn blocks(&self) -> &[SourceBlock];
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MaterializedArchiveSource {
    identity: ArchiveIdentity,
    blocks: Vec<SourceBlock>,
}

impl ArchiveSource for MaterializedArchiveSource {
    fn identity(&self) -> &ArchiveIdentity {
        &self.identity
    }

    fn blocks(&self) -> &[SourceBlock] {
        &self.blocks
    }
}

fn prepare_from_source<S: ArchiveSource>(source: &S) -> Result<PreparedArchive, ProjectionError> {
    prepare_archive(source.identity().clone(), source.blocks().to_vec())
}

// Production RPC conversion is kept in one small adapter and is completed
// against the closed chain-client getter surface below.
pub(crate) async fn fetch_prepared_archive(
    client: &VerifiedArchiveClient,
) -> Result<PreparedArchive, ProjectionError> {
    let (identity, blocks) = source_from_verified_client(client, None).await?;
    prepare_from_source(&MaterializedArchiveSource { identity, blocks })
}

pub(crate) async fn fetch_prepared_archive_through(
    client: &VerifiedArchiveClient,
    block_number: u64,
) -> Result<PreparedArchive, ProjectionError> {
    let (identity, blocks) = source_from_verified_client(client, Some(block_number)).await?;
    prepare_from_source(&MaterializedArchiveSource { identity, blocks })
}

async fn source_from_verified_client(
    client: &VerifiedArchiveClient,
    through: Option<u64>,
) -> Result<(ArchiveIdentity, Vec<SourceBlock>), ProjectionError> {
    let verified = client.identity();
    if verified.namespace() != "polkadot-sdk-parachain"
        || verified.para_id() != 1_000
        || verified.pallet_storage_version() != 1
        || verified.event_schema_version() != EVENT_SCHEMA_VERSION
    {
        return Err(ProjectionError::InvalidFinalizedStream);
    }
    let identity = ArchiveIdentity {
        relay_genesis_hash: *verified.relay_genesis_hash(),
        parachain_genesis_hash: *verified.parachain_genesis_hash(),
        deployment_id: *verified.deployment_id(),
        initial_runtime_spec_version: verified.runtime_spec_version(),
        initial_runtime_code_hash: *verified.runtime_code_hash(),
    };
    let head = client.finalized_head().await?;
    let last = through.unwrap_or(head.number());
    if last > head.number() {
        return Err(ProjectionError::RefreshRequired);
    }
    let mut blocks = reserve_archive_blocks(last)?;
    for number in 0..=last {
        let block = client.finalized_block(&head, number).await?;
        let events = block
            .events()
            .iter()
            .map(|event| SourceEvent {
                extrinsic_index: event.extrinsic_index(),
                system_event_index: event.system_event_index(),
                global_sequence: event.global_sequence(),
                deployment_id: *event.deployment_id(),
                event_schema_version: event.event_schema_version(),
                signer: *event.signer(),
                extrinsic_hash: *event.extrinsic_hash(),
                raw_scale_payload: event.raw_payload().to_vec(),
                payload: match event.payload() {
                    CanonicalPayload::UnitCreated(unit) => SourcePayload::UnitCreated(unit.clone()),
                    CanonicalPayload::UnitTransitioned {
                        unit_id,
                        committed_revision,
                        from,
                        to,
                    } => SourcePayload::UnitTransitioned {
                        unit_id: *unit_id,
                        committed_revision: *committed_revision,
                        from: from.clone(),
                        to: to.clone(),
                    },
                    CanonicalPayload::UnitCompleted {
                        unit_id,
                        committed_revision,
                        phase,
                    } => SourcePayload::UnitCompleted {
                        unit_id: *unit_id,
                        committed_revision: *committed_revision,
                        phase: phase.clone(),
                    },
                    CanonicalPayload::RelationshipDefinitionCreated(definition) => {
                        SourcePayload::RelationshipDefinitionCreated(definition.clone())
                    }
                    CanonicalPayload::RelationshipCreated(relationship) => {
                        SourcePayload::RelationshipCreated(relationship.clone())
                    }
                    CanonicalPayload::RelationshipDeleted(relationship) => {
                        SourcePayload::RelationshipDeleted(relationship.clone())
                    }
                    CanonicalPayload::AssociationRecorded(association) => {
                        SourcePayload::AssociationRecorded(association.clone())
                    }
                    CanonicalPayload::AssociationRevoked(association) => {
                        SourcePayload::AssociationRevoked(association.clone())
                    }
                },
            })
            .collect();
        blocks.push(SourceBlock {
            number: block.number(),
            hash: *block.hash(),
            parent_hash: *block.parent_hash(),
            runtime_spec_version: block.runtime_spec_version(),
            runtime_code_hash: *block.runtime_code_hash(),
            extrinsic_hashes: block.extrinsic_hashes().to_vec(),
            system_event_record_count: block.system_event_record_count(),
            events,
        });
    }
    if last == head.number() && blocks.last().map(|block| &block.hash) != Some(head.hash()) {
        return Err(ProjectionError::InvalidFinalizedStream);
    }
    Ok((identity, blocks))
}

fn reserve_archive_blocks(last: u64) -> Result<Vec<SourceBlock>, ProjectionError> {
    let count = last
        .checked_add(1)
        .ok_or(ProjectionError::InvalidFinalizedStream)?;
    let capacity = usize::try_from(count).map_err(|_| ProjectionError::InvalidFinalizedStream)?;
    let mut blocks = Vec::new();
    blocks
        .try_reserve_exact(capacity)
        .map_err(|_| ProjectionError::InvalidFinalizedStream)?;
    Ok(blocks)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoredProjection {
    anchor: Vec<AnchorRow>,
    blocks: Vec<BlockRow>,
    events: Vec<EventRow>,
    checkpoint: Vec<CheckpointRow>,
    units: Vec<UnitRow>,
    definitions: Vec<DefinitionRow>,
    relationships: Vec<RelationshipRow>,
    associations: Vec<AssociationRow>,
}

impl StoredProjection {
    pub(crate) fn checkpoint_block_number(&self) -> Result<Option<u64>, BackendError> {
        match self.checkpoint.as_slice() {
            [] => Ok(None),
            [checkpoint] => stored::decode_u64_blob(&checkpoint.block_number).map(Some),
            _ => Err(BackendError::ProjectionMismatch),
        }
    }

    pub(crate) fn checkpoint_row(&self) -> Result<Option<&CheckpointRow>, BackendError> {
        match self.checkpoint.as_slice() {
            [] => Ok(None),
            [checkpoint] => Ok(Some(checkpoint)),
            _ => Err(BackendError::ProjectionMismatch),
        }
    }
}

fn decode_checkpoint_row(row: &CheckpointRow) -> Result<ProjectionCheckpoint, BackendError> {
    if row.singleton != 1 {
        return Err(BackendError::ProjectionMismatch);
    }
    let block_number = stored::decode_u64_blob(&row.block_number)?;
    let block_hash = row
        .block_hash
        .as_slice()
        .try_into()
        .map_err(|_| BackendError::ProjectionMismatch)?;
    let last_global_sequence = row
        .last_global_sequence
        .as_deref()
        .map(stored::decode_u64_blob)
        .transpose()?
        .map(|sequence| NonZeroU64::new(sequence).ok_or(BackendError::ProjectionMismatch))
        .transpose()?;
    let runtime_spec_version =
        u32::try_from(row.runtime_spec_version).map_err(|_| BackendError::ProjectionMismatch)?;
    let runtime_code_hash = row
        .runtime_code_hash
        .as_slice()
        .try_into()
        .map_err(|_| BackendError::ProjectionMismatch)?;
    Ok(ProjectionCheckpoint::new(
        block_number,
        block_hash,
        last_global_sequence,
        runtime_spec_version,
        runtime_code_hash,
    ))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AnchorRow {
    singleton: i64,
    namespace: String,
    relay_genesis_hash: Vec<u8>,
    parachain_genesis_hash: Vec<u8>,
    para_id: i64,
    deployment_id: Vec<u8>,
    pallet_storage_version: i64,
    event_schema_version: i64,
    initial_runtime_spec_version: i64,
    initial_runtime_code_hash: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BlockRow {
    anchor_singleton: i64,
    block_number: Vec<u8>,
    block_hash: Vec<u8>,
    parent_hash: Vec<u8>,
    runtime_spec_version: i64,
    runtime_code_hash: Vec<u8>,
    cubikan_event_count: i64,
    first_global_sequence: Option<Vec<u8>>,
    last_global_sequence: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EventRow {
    block_number: Vec<u8>,
    block_hash: Vec<u8>,
    extrinsic_index: i64,
    system_event_index: i64,
    global_sequence: Vec<u8>,
    deployment_id: Vec<u8>,
    event_schema_version: i64,
    event_kind: String,
    scale_payload: Vec<u8>,
    signer: Vec<u8>,
    extrinsic_hash: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CheckpointRow {
    singleton: i64,
    block_number: Vec<u8>,
    block_hash: Vec<u8>,
    last_global_sequence: Option<Vec<u8>>,
    runtime_spec_version: i64,
    runtime_code_hash: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UnitRow {
    id: String,
    envelope_version: i64,
    envelope: String,
    origin_namespace: String,
    origin_scope: String,
    origin_value: String,
    workflow_id: String,
    species: String,
    phase: String,
    status: String,
    revision: Vec<u8>,
    last_global_sequence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DefinitionRow {
    definition_id: String,
    definition_version: Vec<u8>,
    directed: i64,
    source_species: Option<String>,
    target_species: Option<String>,
    self_policy: String,
    cycle_policy: String,
    created_global_sequence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RelationshipRow {
    definition_id: String,
    definition_version: Vec<u8>,
    source_id: String,
    target_id: String,
    created_global_sequence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AssociationRow {
    unit_id: String,
    subject_kind: String,
    subject_revision_key: Vec<u8>,
    namespace: String,
    scope: String,
    value: String,
    created_global_sequence: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedProjection {
    identity: ArchiveIdentity,
    blocks: Vec<BlockRow>,
    events: Vec<EventRow>,
    checkpoint: Option<CheckpointRow>,
    replay: ReplayState,
}

impl ExpectedProjection {
    fn new(identity: ArchiveIdentity) -> Self {
        Self {
            identity,
            blocks: Vec::new(),
            events: Vec::new(),
            checkpoint: None,
            replay: ReplayState::default(),
        }
    }

    fn height(&self) -> Option<u64> {
        self.checkpoint
            .as_ref()
            .and_then(|checkpoint| stored::decode_u64_blob(&checkpoint.block_number).ok())
    }

    fn append(&mut self, block: &PreparedBlock) -> Result<(), BackendError> {
        let expected_number =
            u64::try_from(self.blocks.len()).map_err(|_| BackendError::ProjectionMismatch)?;
        if block.number != expected_number {
            return Err(BackendError::ProjectionMismatch);
        }
        self.blocks.push(BlockRow {
            anchor_singleton: 1,
            block_number: stored::encode_u64_blob(block.number).to_vec(),
            block_hash: block.hash.to_vec(),
            parent_hash: block.parent_hash.to_vec(),
            runtime_spec_version: i64::from(block.runtime_spec_version),
            runtime_code_hash: block.runtime_code_hash.to_vec(),
            cubikan_event_count: i64::try_from(block.events.len())
                .map_err(|_| BackendError::ProjectionMismatch)?,
            first_global_sequence: block
                .first_global_sequence
                .map(|sequence| stored::encode_u64_blob(sequence).to_vec()),
            last_global_sequence: block
                .last_global_sequence
                .map(|sequence| stored::encode_u64_blob(sequence).to_vec()),
        });
        for event in &block.events {
            self.replay.apply_prepared(event)?;
            self.events.push(EventRow {
                block_number: stored::encode_u64_blob(event.block_number).to_vec(),
                block_hash: event.block_hash.to_vec(),
                extrinsic_index: i64::from(event.extrinsic_index),
                system_event_index: i64::from(event.system_event_index),
                global_sequence: stored::encode_u64_blob(event.global_sequence).to_vec(),
                deployment_id: event.deployment_id.to_vec(),
                event_schema_version: i64::from(EVENT_SCHEMA_VERSION),
                event_kind: event_kind_name(event.effect.event_kind()).to_owned(),
                scale_payload: event.raw_scale_payload.clone(),
                signer: event.signer.to_vec(),
                extrinsic_hash: event.extrinsic_hash.to_vec(),
            });
        }
        if self.replay.last_global_sequence != block.checkpoint_sequence {
            return Err(BackendError::ProjectionMismatch);
        }
        self.checkpoint = Some(CheckpointRow {
            singleton: 1,
            block_number: stored::encode_u64_blob(block.number).to_vec(),
            block_hash: block.hash.to_vec(),
            last_global_sequence: block
                .checkpoint_sequence
                .map(|sequence| stored::encode_u64_blob(sequence).to_vec()),
            runtime_spec_version: i64::from(block.runtime_spec_version),
            runtime_code_hash: block.runtime_code_hash.to_vec(),
        });
        Ok(())
    }

    fn stored(&self) -> Result<StoredProjection, BackendError> {
        let anchor = if self.checkpoint.is_some() {
            vec![AnchorRow {
                singleton: 1,
                namespace: "polkadot-sdk-parachain".to_owned(),
                relay_genesis_hash: self.identity.relay_genesis_hash.to_vec(),
                parachain_genesis_hash: self.identity.parachain_genesis_hash.to_vec(),
                para_id: 1_000,
                deployment_id: self.identity.deployment_id.to_vec(),
                pallet_storage_version: 1,
                event_schema_version: i64::from(EVENT_SCHEMA_VERSION),
                initial_runtime_spec_version: i64::from(self.identity.initial_runtime_spec_version),
                initial_runtime_code_hash: self.identity.initial_runtime_code_hash.to_vec(),
            }]
        } else {
            Vec::new()
        };
        let units = self
            .replay
            .units
            .values()
            .map(|state| {
                let unit = &state.unit;
                Ok(UnitRow {
                    id: unit.id().to_string(),
                    envelope_version: 2,
                    envelope: stored::encode_envelope(unit)?,
                    origin_namespace: unit.origin().namespace().as_str().to_owned(),
                    origin_scope: unit.origin().scope().as_str().to_owned(),
                    origin_value: unit.origin().value().as_str().to_owned(),
                    workflow_id: unit.workflow_id().as_str().to_owned(),
                    species: unit.species().as_str().to_owned(),
                    phase: unit.phase().as_str().to_owned(),
                    status: match unit.status() {
                        IntentUnitStatus::Active => "active",
                        IntentUnitStatus::Completed => "completed",
                    }
                    .to_owned(),
                    revision: stored::encode_revision_blob(unit.revision()).to_vec(),
                    last_global_sequence: stored::encode_u64_blob(state.accepted_global_sequence)
                        .to_vec(),
                })
            })
            .collect::<Result<Vec<_>, BackendError>>()?;
        let definitions = self
            .replay
            .definitions
            .values()
            .map(|state| DefinitionRow {
                definition_id: state.definition.key().id().as_str().to_owned(),
                definition_version: stored::encode_u64_blob(
                    state.definition.key().version().value(),
                )
                .to_vec(),
                directed: 1,
                source_species: state
                    .definition
                    .source_species()
                    .map(|species| species.as_str().to_owned()),
                target_species: state
                    .definition
                    .target_species()
                    .map(|species| species.as_str().to_owned()),
                self_policy: policy_name(state.definition.self_policy()).to_owned(),
                cycle_policy: policy_name(state.definition.cycle_policy()).to_owned(),
                created_global_sequence: stored::encode_u64_blob(state.created_global_sequence)
                    .to_vec(),
            })
            .collect();
        let relationships = self
            .replay
            .relationships
            .values()
            .map(|state| RelationshipRow {
                definition_id: state.relationship.definition().id().as_str().to_owned(),
                definition_version: stored::encode_u64_blob(
                    state.relationship.definition().version().value(),
                )
                .to_vec(),
                source_id: state.relationship.source().to_string(),
                target_id: state.relationship.target().to_string(),
                created_global_sequence: stored::encode_u64_blob(state.created_global_sequence)
                    .to_vec(),
            })
            .collect();
        let associations = self
            .replay
            .associations
            .values()
            .map(|state| {
                let key = association_key(&state.association);
                AssociationRow {
                    unit_id: key.unit_id,
                    subject_kind: key.subject_kind,
                    subject_revision_key: key.subject_revision_key,
                    namespace: key.namespace,
                    scope: key.scope,
                    value: key.value,
                    created_global_sequence: stored::encode_u64_blob(state.created_global_sequence)
                        .to_vec(),
                }
            })
            .collect();
        Ok(StoredProjection {
            anchor,
            blocks: self.blocks.clone(),
            events: self.events.clone(),
            checkpoint: self.checkpoint.iter().cloned().collect(),
            units,
            definitions,
            relationships,
            associations,
        })
    }
}

impl ReplayState {
    fn apply_prepared(&mut self, event: &PreparedEvent) -> Result<(), BackendError> {
        let expected_sequence = match self.last_global_sequence {
            Some(sequence) => sequence.checked_add(1),
            None => Some(1),
        };
        if expected_sequence != Some(event.global_sequence) {
            return Err(BackendError::ProjectionMismatch);
        }
        match &event.effect {
            PreparedEffect::UnitCreated(unit) => {
                if self
                    .units
                    .insert(
                        unit.id().to_string(),
                        UnitState {
                            unit: unit.clone(),
                            accepted_global_sequence: event.global_sequence,
                        },
                    )
                    .is_some()
                {
                    return Err(BackendError::ProjectionMismatch);
                }
            }
            PreparedEffect::UnitUpdated { unit, .. } => {
                let state = self
                    .units
                    .get_mut(&unit.id().to_string())
                    .ok_or(BackendError::ProjectionMismatch)?;
                state.unit = unit.clone();
                state.accepted_global_sequence = event.global_sequence;
            }
            PreparedEffect::RelationshipDefinitionCreated(definition) => {
                if self
                    .definitions
                    .insert(
                        definition_key(definition),
                        DefinitionState {
                            definition: definition.clone(),
                            created_global_sequence: event.global_sequence,
                        },
                    )
                    .is_some()
                {
                    return Err(BackendError::ProjectionMismatch);
                }
            }
            PreparedEffect::RelationshipCreated(relationship) => {
                if self
                    .relationships
                    .insert(
                        relationship_key(relationship),
                        RelationshipState {
                            relationship: relationship.clone(),
                            created_global_sequence: event.global_sequence,
                        },
                    )
                    .is_some()
                {
                    return Err(BackendError::ProjectionMismatch);
                }
            }
            PreparedEffect::RelationshipDeleted(relationship) => {
                if self
                    .relationships
                    .remove(&relationship_key(relationship))
                    .is_none()
                {
                    return Err(BackendError::ProjectionMismatch);
                }
            }
            PreparedEffect::AssociationRecorded(association) => {
                if self
                    .associations
                    .insert(
                        association_key(association),
                        AssociationState {
                            association: association.clone(),
                            created_global_sequence: event.global_sequence,
                        },
                    )
                    .is_some()
                {
                    return Err(BackendError::ProjectionMismatch);
                }
            }
            PreparedEffect::AssociationRevoked(association) => {
                if self
                    .associations
                    .remove(&association_key(association))
                    .is_none()
                {
                    return Err(BackendError::ProjectionMismatch);
                }
            }
        }
        self.last_global_sequence = Some(event.global_sequence);
        Ok(())
    }
}

const fn policy_name(policy: RelationshipPolicy) -> &'static str {
    match policy {
        RelationshipPolicy::Allow => "allow",
        RelationshipPolicy::Reject => "reject",
    }
}

const fn event_kind_name(kind: ProjectedEventKind) -> &'static str {
    match kind {
        ProjectedEventKind::UnitCreated => "unit_created",
        ProjectedEventKind::UnitTransitioned => "unit_transitioned",
        ProjectedEventKind::UnitCompleted => "unit_completed",
        ProjectedEventKind::RelationshipDefinitionCreated => "relationship_definition_created",
        ProjectedEventKind::RelationshipCreated => "relationship_created",
        ProjectedEventKind::RelationshipDeleted => "relationship_deleted",
        ProjectedEventKind::AssociationRecorded => "association_recorded",
        ProjectedEventKind::AssociationRevoked => "association_revoked",
    }
}

pub(crate) fn load_stored_projection(
    connection: &Connection,
) -> Result<StoredProjection, BackendError> {
    let checkpoint = load_checkpoint_rows(connection)?;
    load_stored_projection_after_checkpoint(connection, checkpoint)
}

pub(crate) fn load_projection_checkpoint(
    connection: &Connection,
) -> Result<Option<ProjectionCheckpoint>, BackendError> {
    match load_checkpoint_rows(connection)?.as_slice() {
        [] => Ok(None),
        [checkpoint] => decode_checkpoint_row(checkpoint).map(Some),
        _ => Err(BackendError::ProjectionMismatch),
    }
}

pub(crate) fn load_stored_projection_if_checkpoint(
    connection: &Connection,
    candidate: &ProjectionCheckpoint,
) -> Result<Option<StoredProjection>, BackendError> {
    let checkpoint = load_checkpoint_rows(connection)?;
    let pinned = match checkpoint.as_slice() {
        [] => return Ok(None),
        [checkpoint] => decode_checkpoint_row(checkpoint)?,
        _ => return Err(BackendError::ProjectionMismatch),
    };
    if pinned != *candidate {
        return Ok(None);
    }
    load_stored_projection_after_checkpoint(connection, checkpoint).map(Some)
}

fn load_checkpoint_rows(connection: &Connection) -> Result<Vec<CheckpointRow>, BackendError> {
    query_rows(
        connection,
        "SELECT singleton,block_number,block_hash,last_global_sequence,runtime_spec_version,runtime_code_hash FROM projection_checkpoint ORDER BY singleton",
        |row| {
            Ok(CheckpointRow {
                singleton: row.get(0)?,
                block_number: row.get(1)?,
                block_hash: row.get(2)?,
                last_global_sequence: row.get(3)?,
                runtime_spec_version: row.get(4)?,
                runtime_code_hash: row.get(5)?,
            })
        },
    )
}

fn load_stored_projection_after_checkpoint(
    connection: &Connection,
    checkpoint: Vec<CheckpointRow>,
) -> Result<StoredProjection, BackendError> {
    let anchor = query_rows(
        connection,
        "SELECT singleton,namespace,relay_genesis_hash,parachain_genesis_hash,para_id,deployment_id,pallet_storage_version,event_schema_version,initial_runtime_spec_version,initial_runtime_code_hash FROM projection_anchor ORDER BY singleton",
        |row| {
            Ok(AnchorRow {
                singleton: row.get(0)?,
                namespace: row.get(1)?,
                relay_genesis_hash: row.get(2)?,
                parachain_genesis_hash: row.get(3)?,
                para_id: row.get(4)?,
                deployment_id: row.get(5)?,
                pallet_storage_version: row.get(6)?,
                event_schema_version: row.get(7)?,
                initial_runtime_spec_version: row.get(8)?,
                initial_runtime_code_hash: row.get(9)?,
            })
        },
    )?;
    let blocks = query_rows(
        connection,
        "SELECT anchor_singleton,block_number,block_hash,parent_hash,runtime_spec_version,runtime_code_hash,cubikan_event_count,first_global_sequence,last_global_sequence FROM projected_blocks ORDER BY block_number",
        |row| {
            Ok(BlockRow {
                anchor_singleton: row.get(0)?,
                block_number: row.get(1)?,
                block_hash: row.get(2)?,
                parent_hash: row.get(3)?,
                runtime_spec_version: row.get(4)?,
                runtime_code_hash: row.get(5)?,
                cubikan_event_count: row.get(6)?,
                first_global_sequence: row.get(7)?,
                last_global_sequence: row.get(8)?,
            })
        },
    )?;
    let events = query_rows(
        connection,
        "SELECT e.block_number,b.block_hash,e.extrinsic_index,e.system_event_index,e.global_sequence,e.deployment_id,e.event_schema_version,e.event_kind,e.scale_payload,e.signer,e.extrinsic_hash FROM projected_events AS e JOIN projected_blocks AS b ON b.block_number=e.block_number ORDER BY e.block_number,e.extrinsic_index,e.system_event_index",
        |row| {
            Ok(EventRow {
                block_number: row.get(0)?,
                block_hash: row.get(1)?,
                extrinsic_index: row.get(2)?,
                system_event_index: row.get(3)?,
                global_sequence: row.get(4)?,
                deployment_id: row.get(5)?,
                event_schema_version: row.get(6)?,
                event_kind: row.get(7)?,
                scale_payload: row.get(8)?,
                signer: row.get(9)?,
                extrinsic_hash: row.get(10)?,
            })
        },
    )?;
    let units = query_rows(
        connection,
        "SELECT id,envelope_version,envelope,origin_namespace,origin_scope,origin_value,workflow_id,species,phase,status,revision,last_global_sequence FROM intent_units ORDER BY id COLLATE BINARY",
        |row| {
            Ok(UnitRow {
                id: row.get(0)?,
                envelope_version: row.get(1)?,
                envelope: row.get(2)?,
                origin_namespace: row.get(3)?,
                origin_scope: row.get(4)?,
                origin_value: row.get(5)?,
                workflow_id: row.get(6)?,
                species: row.get(7)?,
                phase: row.get(8)?,
                status: row.get(9)?,
                revision: row.get(10)?,
                last_global_sequence: row.get(11)?,
            })
        },
    )?;
    let definitions = query_rows(
        connection,
        "SELECT definition_id,definition_version,directed,source_species,target_species,self_policy,cycle_policy,created_global_sequence FROM relationship_definitions ORDER BY definition_id COLLATE BINARY,definition_version",
        |row| {
            Ok(DefinitionRow {
                definition_id: row.get(0)?,
                definition_version: row.get(1)?,
                directed: row.get(2)?,
                source_species: row.get(3)?,
                target_species: row.get(4)?,
                self_policy: row.get(5)?,
                cycle_policy: row.get(6)?,
                created_global_sequence: row.get(7)?,
            })
        },
    )?;
    let relationships = query_rows(
        connection,
        "SELECT definition_id,definition_version,source_id,target_id,created_global_sequence FROM intent_unit_relationships ORDER BY definition_id COLLATE BINARY,definition_version,source_id COLLATE BINARY,target_id COLLATE BINARY",
        |row| {
            Ok(RelationshipRow {
                definition_id: row.get(0)?,
                definition_version: row.get(1)?,
                source_id: row.get(2)?,
                target_id: row.get(3)?,
                created_global_sequence: row.get(4)?,
            })
        },
    )?;
    let associations = query_rows(
        connection,
        "SELECT unit_id,subject_kind,subject_revision_key,namespace,scope,value,created_global_sequence FROM recorded_associations ORDER BY unit_id COLLATE BINARY,subject_kind COLLATE BINARY,subject_revision_key,namespace COLLATE BINARY,scope COLLATE BINARY,value COLLATE BINARY",
        |row| {
            Ok(AssociationRow {
                unit_id: row.get(0)?,
                subject_kind: row.get(1)?,
                subject_revision_key: row.get(2)?,
                namespace: row.get(3)?,
                scope: row.get(4)?,
                value: row.get(5)?,
                created_global_sequence: row.get(6)?,
            })
        },
    )?;
    Ok(StoredProjection {
        anchor,
        blocks,
        events,
        checkpoint,
        units,
        definitions,
        relationships,
        associations,
    })
}

fn query_rows<T, F>(
    connection: &Connection,
    statement: &'static str,
    mut map: F,
) -> Result<Vec<T>, BackendError>
where
    F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
    let mut prepared = connection
        .prepare(statement)
        .map_err(crate::sqlite::classify_runtime_error)?;
    let rows = prepared
        .query_map([], |row| map(row))
        .map_err(crate::sqlite::classify_runtime_error)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(crate::sqlite::classify_runtime_error)
}

fn read_projection(directory: &Path, basename: &OsStr) -> Result<StoredProjection, BackendError> {
    let mut reader = open_projection_reader(directory, basename)?;
    reader.begin_verified_read()?;
    let result = reader.with_verified_query(
        VerifiedQueryStatement::FullProjectionCompare,
        load_stored_projection,
    );
    let rollback = reader.rollback_verified_read();
    match result {
        Ok(projection) => {
            rollback?;
            Ok(projection)
        }
        Err(error) => {
            let _ = rollback;
            Err(error)
        }
    }
}

pub(crate) fn synchronize_prepared(
    projector: &FinalizedProjector,
    archive: &PreparedArchive,
) -> Result<ProjectionCheckpoint, ProjectionError> {
    let checkpoint = archive.checkpoint()?;
    let (directory, basename) = projector.parts()?;
    let mut writer = open_projection_writer(directory, basename)?;
    let mut expected = ExpectedProjection::new(archive.identity.clone());

    loop {
        writer.begin_projection()?;
        let reconciliation =
            reconcile_under_reservation(directory, basename, archive, &mut expected);
        let next_index = match reconciliation {
            Ok(index) => index,
            Err(error) => {
                let _ = writer.rollback_projection();
                return Err(error);
            }
        };
        let Some(block) = archive.blocks.get(next_index) else {
            writer.rollback_projection()?;
            return Ok(checkpoint);
        };
        if let Err(error) = commit_prepared_block(
            &mut writer,
            &archive.identity,
            block,
            next_index
                .checked_sub(1)
                .and_then(|previous| archive.blocks.get(previous)),
        ) {
            return Err(error.into());
        }
        expected.append(block)?;
    }
}

fn reconcile_under_reservation(
    directory: &Path,
    basename: &OsStr,
    archive: &PreparedArchive,
    expected: &mut ExpectedProjection,
) -> Result<usize, ProjectionError> {
    let actual = read_projection(directory, basename)?;
    let actual_height = actual.checkpoint_block_number()?;
    if actual_height < expected.height() {
        return Err(ProjectionError::RefreshRequired);
    }
    if let Some(height) = actual_height {
        let index = usize::try_from(height).map_err(|_| ProjectionError::InvalidFinalizedStream)?;
        if index >= archive.blocks.len() {
            return Err(ProjectionError::RefreshRequired);
        }
        for stored_block in &actual.blocks {
            let number = stored::decode_u64_blob(&stored_block.block_number)
                .map_err(|_| BackendError::ProjectionMismatch)?;
            let expected_block = usize::try_from(number)
                .ok()
                .and_then(|number| archive.blocks.get(number))
                .ok_or(BackendError::ProjectionMismatch)?;
            if stored_block.block_hash.as_slice() != expected_block.hash {
                return Err(ProjectionError::ConflictingFinalizedBlock);
            }
        }
        while expected.height().is_none_or(|current| current < height) {
            let next = expected
                .height()
                .and_then(|current| current.checked_add(1))
                .unwrap_or(0);
            let next =
                usize::try_from(next).map_err(|_| ProjectionError::InvalidFinalizedStream)?;
            expected.append(
                archive
                    .blocks
                    .get(next)
                    .ok_or(ProjectionError::RefreshRequired)?,
            )?;
        }
        if actual.checkpoint_row()?.is_some_and(|checkpoint| {
            checkpoint.block_hash.as_slice() != archive.blocks[index].hash
        }) {
            return Err(ProjectionError::ConflictingFinalizedBlock);
        }
    }
    if actual != expected.stored()? {
        return Err(BackendError::ProjectionMismatch.into());
    }
    actual_height
        .map(|height| {
            height
                .checked_add(1)
                .and_then(|next| usize::try_from(next).ok())
                .ok_or(ProjectionError::InvalidFinalizedStream)
        })
        .unwrap_or(Ok(0))
}

fn write_prepared_block<W: ProjectionWriter>(
    writer: &mut W,
    identity: &ArchiveIdentity,
    block: &PreparedBlock,
    previous: Option<&PreparedBlock>,
) -> Result<(), BackendError> {
    if block.number == 0 {
        if previous.is_some() {
            return Err(BackendError::ProjectionMismatch);
        }
        projection_store::insert_anchor(
            writer,
            StoredAnchorInput {
                relay_genesis_hash: &identity.relay_genesis_hash,
                parachain_genesis_hash: &identity.parachain_genesis_hash,
                deployment_id: &identity.deployment_id,
                initial_runtime_spec_version: identity.initial_runtime_spec_version,
                initial_runtime_code_hash: &identity.initial_runtime_code_hash,
            },
        )?;
    } else if previous.is_none_or(|previous| {
        let sequences_follow = match (
            block.first_global_sequence,
            block.last_global_sequence,
            previous.checkpoint_sequence,
            block.checkpoint_sequence,
        ) {
            (None, None, previous_sequence, checkpoint_sequence) => {
                previous_sequence == checkpoint_sequence
            }
            (Some(first), Some(last), previous_sequence, Some(checkpoint_sequence)) => {
                let expected_first = match previous_sequence {
                    Some(sequence) => sequence.checked_add(1),
                    None => Some(1),
                };
                expected_first == Some(first) && checkpoint_sequence == last
            }
            _ => false,
        };
        previous.number.checked_add(1) != Some(block.number)
            || previous.hash != block.parent_hash
            || !sequences_follow
    }) {
        return Err(BackendError::ProjectionMismatch);
    }

    projection_store::insert_block(
        writer,
        StoredBlockInput {
            block_number: block.number,
            block_hash: &block.hash,
            parent_hash: &block.parent_hash,
            runtime_spec_version: block.runtime_spec_version,
            runtime_code_hash: &block.runtime_code_hash,
            event_count: u32::try_from(block.events.len())
                .map_err(|_| BackendError::ProjectionMismatch)?,
            first_global_sequence: block.first_global_sequence,
            last_global_sequence: block.last_global_sequence,
        },
    )?;
    for event in &block.events {
        projection_store::insert_event(
            writer,
            StoredEventInput {
                block_number: event.block_number,
                extrinsic_index: event.extrinsic_index,
                system_event_index: event.system_event_index,
                global_sequence: event.global_sequence,
                deployment_id: &event.deployment_id,
                kind: event.effect.event_kind(),
                scale_payload: &event.raw_scale_payload,
                signer: &event.signer,
                extrinsic_hash: &event.extrinsic_hash,
            },
        )?;
        write_effect(writer, &event.effect, event.global_sequence)?;
    }
    let checkpoint = StoredCheckpointInput {
        block_number: block.number,
        block_hash: &block.hash,
        last_global_sequence: block.checkpoint_sequence,
        runtime_spec_version: block.runtime_spec_version,
        runtime_code_hash: &block.runtime_code_hash,
    };
    if let Some(previous) = previous {
        projection_store::update_checkpoint(writer, checkpoint, previous.number, &previous.hash)
    } else {
        projection_store::insert_checkpoint(writer, checkpoint)
    }
}

trait AtomicProjectionWriter: ProjectionWriter {
    fn commit_atomic_projection(&mut self) -> Result<(), BackendError>;
    fn rollback_atomic_projection(&mut self) -> Result<(), BackendError>;
}

impl AtomicProjectionWriter for ProjectionWriterConnection {
    fn commit_atomic_projection(&mut self) -> Result<(), BackendError> {
        self.commit_projection()
    }

    fn rollback_atomic_projection(&mut self) -> Result<(), BackendError> {
        self.rollback_projection()
    }
}

fn commit_prepared_block<W: AtomicProjectionWriter>(
    writer: &mut W,
    identity: &ArchiveIdentity,
    block: &PreparedBlock,
    previous: Option<&PreparedBlock>,
) -> Result<(), BackendError> {
    if let Err(error) = write_prepared_block(writer, identity, block, previous) {
        let _ = writer.rollback_atomic_projection();
        return Err(error);
    }
    if let Err(error) = writer.commit_atomic_projection() {
        let _ = writer.rollback_atomic_projection();
        return Err(error);
    }
    Ok(())
}

fn write_effect<W: ProjectionWriter>(
    writer: &mut W,
    effect: &PreparedEffect,
    global_sequence: u64,
) -> Result<(), BackendError> {
    match effect {
        PreparedEffect::UnitCreated(unit) => {
            projection_store::insert_intent_unit(writer, unit, global_sequence)
        }
        PreparedEffect::UnitUpdated {
            unit,
            previous_revision,
        } => {
            projection_store::update_intent_unit(writer, unit, *previous_revision, global_sequence)
        }
        PreparedEffect::RelationshipDefinitionCreated(definition) => {
            projection_store::insert_relationship_definition(writer, definition, global_sequence)
        }
        PreparedEffect::RelationshipCreated(relationship) => {
            projection_store::insert_relationship(writer, relationship, global_sequence)
        }
        PreparedEffect::RelationshipDeleted(relationship) => {
            projection_store::delete_relationship(writer, relationship)
        }
        PreparedEffect::AssociationRecorded(association) => {
            projection_store::insert_association(writer, association, global_sequence)
        }
        PreparedEffect::AssociationRevoked(association) => {
            projection_store::delete_association(writer, association)
        }
    }
}

#[cfg(test)]
#[path = "projector/tests.rs"]
pub(crate) mod tests;
