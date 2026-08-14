use cubikan_core::{
    AssociationSubject, IntentUnit, IntentUnitRevision, RecordedAssociation,
    RelationshipDefinition, RelationshipIdentity, RelationshipPolicy,
};
use rusqlite::{Params, params};

use crate::{BackendError, stored};

const MAX_SCALE_PAYLOAD_BYTES: usize = 1_048_576;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProjectionStatement {
    InsertAnchor,
    InsertBlock,
    InsertEvent,
    InsertCheckpoint,
    UpdateCheckpoint,
    InsertIntentUnit,
    UpdateIntentUnit,
    InsertRelationshipDefinition,
    InsertRelationship,
    DeleteRelationship,
    InsertAssociation,
    DeleteAssociation,
}

impl ProjectionStatement {
    pub(crate) const ALL: [Self; 12] = [
        Self::InsertAnchor,
        Self::InsertBlock,
        Self::InsertEvent,
        Self::InsertCheckpoint,
        Self::UpdateCheckpoint,
        Self::InsertIntentUnit,
        Self::UpdateIntentUnit,
        Self::InsertRelationshipDefinition,
        Self::InsertRelationship,
        Self::DeleteRelationship,
        Self::InsertAssociation,
        Self::DeleteAssociation,
    ];

    pub(crate) const fn sql(self) -> &'static str {
        match self {
            Self::InsertAnchor => {
                "INSERT INTO projection_anchor(singleton,namespace,relay_genesis_hash,parachain_genesis_hash,para_id,deployment_id,pallet_storage_version,event_schema_version,initial_runtime_spec_version,initial_runtime_code_hash) VALUES(1,'polkadot-sdk-parachain',?1,?2,1000,?3,1,1,?4,?5)"
            }
            Self::InsertBlock => {
                "INSERT INTO projected_blocks(anchor_singleton,block_number,block_hash,parent_hash,runtime_spec_version,runtime_code_hash,cubikan_event_count,first_global_sequence,last_global_sequence) VALUES(1,?1,?2,?3,?4,?5,?6,?7,?8)"
            }
            Self::InsertEvent => {
                "INSERT INTO projected_events(block_number,extrinsic_index,system_event_index,global_sequence,deployment_id,event_schema_version,event_kind,scale_payload,signer,extrinsic_hash) VALUES(?1,?2,?3,?4,?5,1,?6,?7,?8,?9)"
            }
            Self::InsertCheckpoint => {
                "INSERT INTO projection_checkpoint(singleton,block_number,block_hash,last_global_sequence,runtime_spec_version,runtime_code_hash) VALUES(1,?1,?2,?3,?4,?5)"
            }
            Self::UpdateCheckpoint => {
                "UPDATE projection_checkpoint SET block_number=?1,block_hash=?2,last_global_sequence=?3,runtime_spec_version=?4,runtime_code_hash=?5 WHERE singleton=1 AND block_number=?6 AND block_hash=?7"
            }
            Self::InsertIntentUnit => {
                "INSERT INTO intent_units(id,envelope_version,envelope,origin_namespace,origin_scope,origin_value,workflow_id,species,phase,status,revision,last_global_sequence) VALUES(?1,2,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)"
            }
            Self::UpdateIntentUnit => {
                "UPDATE intent_units SET envelope=?1,phase=?2,status=?3,revision=?4,last_global_sequence=?5 WHERE id=?6 COLLATE BINARY AND revision=?7"
            }
            Self::InsertRelationshipDefinition => {
                "INSERT INTO relationship_definitions(definition_id,definition_version,directed,source_species,target_species,self_policy,cycle_policy,created_global_sequence) VALUES(?1,?2,1,?3,?4,?5,?6,?7)"
            }
            Self::InsertRelationship => {
                "INSERT INTO intent_unit_relationships(definition_id,definition_version,source_id,target_id,created_global_sequence) VALUES(?1,?2,?3,?4,?5)"
            }
            Self::DeleteRelationship => {
                "DELETE FROM intent_unit_relationships WHERE definition_id=?1 COLLATE BINARY AND definition_version=?2 AND source_id=?3 COLLATE BINARY AND target_id=?4 COLLATE BINARY"
            }
            Self::InsertAssociation => {
                "INSERT INTO recorded_associations(unit_id,subject_kind,subject_revision_key,namespace,scope,value,created_global_sequence) VALUES(?1,?2,?3,?4,?5,?6,?7)"
            }
            Self::DeleteAssociation => {
                "DELETE FROM recorded_associations WHERE unit_id=?1 COLLATE BINARY AND subject_kind=?2 COLLATE BINARY AND subject_revision_key=?3 AND namespace=?4 COLLATE BINARY AND scope=?5 COLLATE BINARY AND value=?6 COLLATE BINARY"
            }
        }
    }
}

/// Narrow execution boundary implemented by the SQLite lane's role wrapper.
///
/// The wrapper installs the exact authorizer policy for `statement` before it
/// executes the corresponding static SQL. There is deliberately no production
/// implementation for a raw [`rusqlite::Connection`].
pub(crate) trait ProjectionWriter {
    fn execute<P: Params>(
        &mut self,
        statement: ProjectionStatement,
        parameters: P,
    ) -> Result<usize, BackendError>;
}

/// Keeps the closed T-1110 projection boundary compile-reachable while the
/// current public backend remains an unconditional retired-generation guard.
pub(crate) fn retain_projection_store_symbols<W: ProjectionWriter>() {
    let _ = ProjectionStatement::ALL;
    let _ = [
        ProjectedEventKind::UnitCreated,
        ProjectedEventKind::UnitTransitioned,
        ProjectedEventKind::UnitCompleted,
        ProjectedEventKind::RelationshipDefinitionCreated,
        ProjectedEventKind::RelationshipCreated,
        ProjectedEventKind::RelationshipDeleted,
        ProjectedEventKind::AssociationRecorded,
        ProjectedEventKind::AssociationRevoked,
    ];
    let _ = insert_anchor::<W>;
    let _ = insert_block::<W>;
    let _ = insert_event::<W>;
    let _ = insert_checkpoint::<W>;
    let _ = update_checkpoint::<W>;
    let _ = insert_intent_unit::<W>;
    let _ = update_intent_unit::<W>;
    let _ = insert_relationship_definition::<W>;
    let _ = insert_relationship::<W>;
    let _ = delete_relationship::<W>;
    let _ = insert_association::<W>;
    let _ = delete_association::<W>;
    stored::retain_stored_projection_symbols();
}

#[derive(Clone, Copy)]
pub(crate) struct ProjectionAnchor<'a> {
    pub(crate) relay_genesis_hash: &'a [u8; 32],
    pub(crate) parachain_genesis_hash: &'a [u8; 32],
    pub(crate) deployment_id: &'a [u8; 32],
    pub(crate) initial_runtime_spec_version: u32,
    pub(crate) initial_runtime_code_hash: &'a [u8; 32],
}

#[derive(Clone, Copy)]
pub(crate) struct ProjectedBlock<'a> {
    pub(crate) block_number: u64,
    pub(crate) block_hash: &'a [u8; 32],
    pub(crate) parent_hash: &'a [u8; 32],
    pub(crate) runtime_spec_version: u32,
    pub(crate) runtime_code_hash: &'a [u8; 32],
    pub(crate) event_count: u32,
    pub(crate) first_global_sequence: Option<u64>,
    pub(crate) last_global_sequence: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProjectedEventKind {
    UnitCreated,
    UnitTransitioned,
    UnitCompleted,
    RelationshipDefinitionCreated,
    RelationshipCreated,
    RelationshipDeleted,
    AssociationRecorded,
    AssociationRevoked,
}

impl ProjectedEventKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::UnitCreated => "unit_created",
            Self::UnitTransitioned => "unit_transitioned",
            Self::UnitCompleted => "unit_completed",
            Self::RelationshipDefinitionCreated => "relationship_definition_created",
            Self::RelationshipCreated => "relationship_created",
            Self::RelationshipDeleted => "relationship_deleted",
            Self::AssociationRecorded => "association_recorded",
            Self::AssociationRevoked => "association_revoked",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ProjectedEvent<'a> {
    pub(crate) block_number: u64,
    pub(crate) extrinsic_index: u32,
    pub(crate) system_event_index: u32,
    pub(crate) global_sequence: u64,
    pub(crate) deployment_id: &'a [u8; 32],
    pub(crate) kind: ProjectedEventKind,
    pub(crate) scale_payload: &'a [u8],
    pub(crate) signer: &'a [u8; 32],
    pub(crate) extrinsic_hash: &'a [u8; 32],
}

#[derive(Clone, Copy)]
pub(crate) struct ProjectionCheckpoint<'a> {
    pub(crate) block_number: u64,
    pub(crate) block_hash: &'a [u8; 32],
    pub(crate) last_global_sequence: Option<u64>,
    pub(crate) runtime_spec_version: u32,
    pub(crate) runtime_code_hash: &'a [u8; 32],
}

pub(crate) fn insert_anchor<W: ProjectionWriter>(
    writer: &mut W,
    anchor: ProjectionAnchor<'_>,
) -> Result<(), BackendError> {
    require_one(writer.execute(
        ProjectionStatement::InsertAnchor,
        params![
            anchor.relay_genesis_hash.as_slice(),
            anchor.parachain_genesis_hash.as_slice(),
            anchor.deployment_id.as_slice(),
            i64::from(anchor.initial_runtime_spec_version),
            anchor.initial_runtime_code_hash.as_slice(),
        ],
    )?)
}

pub(crate) fn insert_block<W: ProjectionWriter>(
    writer: &mut W,
    block: ProjectedBlock<'_>,
) -> Result<(), BackendError> {
    validate_block_sequences(block)?;
    let block_number = stored::encode_u64_blob(block.block_number);
    let first = block.first_global_sequence.map(stored::encode_u64_blob);
    let last = block.last_global_sequence.map(stored::encode_u64_blob);
    require_one(writer.execute(
        ProjectionStatement::InsertBlock,
        params![
            block_number.as_slice(),
            block.block_hash.as_slice(),
            block.parent_hash.as_slice(),
            i64::from(block.runtime_spec_version),
            block.runtime_code_hash.as_slice(),
            i64::from(block.event_count),
            first.as_ref().map(<[u8; 8]>::as_slice),
            last.as_ref().map(<[u8; 8]>::as_slice),
        ],
    )?)
}

pub(crate) fn insert_event<W: ProjectionWriter>(
    writer: &mut W,
    event: ProjectedEvent<'_>,
) -> Result<(), BackendError> {
    if event.global_sequence == 0
        || event.scale_payload.is_empty()
        || event.scale_payload.len() > MAX_SCALE_PAYLOAD_BYTES
    {
        return Err(BackendError::ProjectionMismatch);
    }
    let block_number = stored::encode_u64_blob(event.block_number);
    let global_sequence = stored::encode_u64_blob(event.global_sequence);
    require_one(writer.execute(
        ProjectionStatement::InsertEvent,
        params![
            block_number.as_slice(),
            i64::from(event.extrinsic_index),
            i64::from(event.system_event_index),
            global_sequence.as_slice(),
            event.deployment_id.as_slice(),
            event.kind.as_str(),
            event.scale_payload,
            event.signer.as_slice(),
            event.extrinsic_hash.as_slice(),
        ],
    )?)
}

pub(crate) fn insert_checkpoint<W: ProjectionWriter>(
    writer: &mut W,
    checkpoint: ProjectionCheckpoint<'_>,
) -> Result<(), BackendError> {
    validate_checkpoint(checkpoint)?;
    let block_number = stored::encode_u64_blob(checkpoint.block_number);
    let sequence = checkpoint.last_global_sequence.map(stored::encode_u64_blob);
    require_one(writer.execute(
        ProjectionStatement::InsertCheckpoint,
        params![
            block_number.as_slice(),
            checkpoint.block_hash.as_slice(),
            sequence.as_ref().map(<[u8; 8]>::as_slice),
            i64::from(checkpoint.runtime_spec_version),
            checkpoint.runtime_code_hash.as_slice(),
        ],
    )?)
}

pub(crate) fn update_checkpoint<W: ProjectionWriter>(
    writer: &mut W,
    checkpoint: ProjectionCheckpoint<'_>,
    expected_block_number: u64,
    expected_block_hash: &[u8; 32],
) -> Result<(), BackendError> {
    validate_checkpoint(checkpoint)?;
    let block_number = stored::encode_u64_blob(checkpoint.block_number);
    let sequence = checkpoint.last_global_sequence.map(stored::encode_u64_blob);
    let expected_block_number = stored::encode_u64_blob(expected_block_number);
    require_one(writer.execute(
        ProjectionStatement::UpdateCheckpoint,
        params![
            block_number.as_slice(),
            checkpoint.block_hash.as_slice(),
            sequence.as_ref().map(<[u8; 8]>::as_slice),
            i64::from(checkpoint.runtime_spec_version),
            checkpoint.runtime_code_hash.as_slice(),
            expected_block_number.as_slice(),
            expected_block_hash.as_slice(),
        ],
    )?)
}

pub(crate) fn insert_intent_unit<W: ProjectionWriter>(
    writer: &mut W,
    unit: &IntentUnit,
    accepted_global_sequence: u64,
) -> Result<(), BackendError> {
    if accepted_global_sequence == 0 {
        return Err(BackendError::ProjectionMismatch);
    }
    let envelope = stored::encode_envelope(unit)?;
    let revision = stored::encode_revision_blob(unit.revision());
    let sequence = stored::encode_u64_blob(accepted_global_sequence);
    require_one(writer.execute(
        ProjectionStatement::InsertIntentUnit,
        params![
            unit.id().to_string(),
            envelope,
            unit.origin().namespace().as_str(),
            unit.origin().scope().as_str(),
            unit.origin().value().as_str(),
            unit.workflow_id().as_str(),
            unit.species().as_str(),
            unit.phase().as_str(),
            status_projection(unit),
            revision.as_slice(),
            sequence.as_slice(),
        ],
    )?)
}

pub(crate) fn update_intent_unit<W: ProjectionWriter>(
    writer: &mut W,
    unit: &IntentUnit,
    previous_revision: IntentUnitRevision,
    accepted_global_sequence: u64,
) -> Result<(), BackendError> {
    if accepted_global_sequence == 0
        || previous_revision.value().checked_add(1) != Some(unit.revision().value())
    {
        return Err(BackendError::ProjectionMismatch);
    }
    let envelope = stored::encode_envelope(unit)?;
    let revision = stored::encode_revision_blob(unit.revision());
    let previous_revision = stored::encode_revision_blob(previous_revision);
    let sequence = stored::encode_u64_blob(accepted_global_sequence);
    require_one(writer.execute(
        ProjectionStatement::UpdateIntentUnit,
        params![
            envelope,
            unit.phase().as_str(),
            status_projection(unit),
            revision.as_slice(),
            sequence.as_slice(),
            unit.id().to_string(),
            previous_revision.as_slice(),
        ],
    )?)
}

pub(crate) fn insert_relationship_definition<W: ProjectionWriter>(
    writer: &mut W,
    definition: &RelationshipDefinition,
    created_global_sequence: u64,
) -> Result<(), BackendError> {
    if created_global_sequence == 0
        || definition.source_species().is_some_and(|species| {
            cubikan_core::IntentSpecies::from_bytes(species.as_str().as_bytes()).is_err()
        })
        || definition.target_species().is_some_and(|species| {
            cubikan_core::IntentSpecies::from_bytes(species.as_str().as_bytes()).is_err()
        })
    {
        return Err(BackendError::ProjectionMismatch);
    }
    let version = stored::encode_u64_blob(definition.key().version().value());
    let sequence = stored::encode_u64_blob(created_global_sequence);
    require_one(writer.execute(
        ProjectionStatement::InsertRelationshipDefinition,
        params![
            definition.key().id().as_str(),
            version.as_slice(),
            definition.source_species().map(|species| species.as_str()),
            definition.target_species().map(|species| species.as_str()),
            policy_projection(definition.self_policy()),
            policy_projection(definition.cycle_policy()),
            sequence.as_slice(),
        ],
    )?)
}

pub(crate) fn insert_relationship<W: ProjectionWriter>(
    writer: &mut W,
    relationship: &RelationshipIdentity,
    created_global_sequence: u64,
) -> Result<(), BackendError> {
    if created_global_sequence == 0 {
        return Err(BackendError::ProjectionMismatch);
    }
    let version = stored::encode_u64_blob(relationship.definition().version().value());
    let sequence = stored::encode_u64_blob(created_global_sequence);
    require_one(writer.execute(
        ProjectionStatement::InsertRelationship,
        params![
            relationship.definition().id().as_str(),
            version.as_slice(),
            relationship.source().to_string(),
            relationship.target().to_string(),
            sequence.as_slice(),
        ],
    )?)
}

pub(crate) fn delete_relationship<W: ProjectionWriter>(
    writer: &mut W,
    relationship: &RelationshipIdentity,
) -> Result<(), BackendError> {
    let version = stored::encode_u64_blob(relationship.definition().version().value());
    require_one(writer.execute(
        ProjectionStatement::DeleteRelationship,
        params![
            relationship.definition().id().as_str(),
            version.as_slice(),
            relationship.source().to_string(),
            relationship.target().to_string(),
        ],
    )?)
}

pub(crate) fn insert_association<W: ProjectionWriter>(
    writer: &mut W,
    association: &RecordedAssociation,
    created_global_sequence: u64,
) -> Result<(), BackendError> {
    if created_global_sequence == 0 {
        return Err(BackendError::ProjectionMismatch);
    }
    let (kind, revision) = association_subject(association.subject());
    let sequence = stored::encode_u64_blob(created_global_sequence);
    require_one(writer.execute(
        ProjectionStatement::InsertAssociation,
        params![
            association.unit_id().to_string(),
            kind,
            revision.as_slice(),
            association.reference().namespace().as_str(),
            association.reference().scope().as_str(),
            association.reference().value().as_str(),
            sequence.as_slice(),
        ],
    )?)
}

pub(crate) fn delete_association<W: ProjectionWriter>(
    writer: &mut W,
    association: &RecordedAssociation,
) -> Result<(), BackendError> {
    let (kind, revision) = association_subject(association.subject());
    require_one(writer.execute(
        ProjectionStatement::DeleteAssociation,
        params![
            association.unit_id().to_string(),
            kind,
            revision.as_slice(),
            association.reference().namespace().as_str(),
            association.reference().scope().as_str(),
            association.reference().value().as_str(),
        ],
    )?)
}

fn validate_block_sequences(block: ProjectedBlock<'_>) -> Result<(), BackendError> {
    match (
        block.event_count,
        block.first_global_sequence,
        block.last_global_sequence,
    ) {
        (0, None, None) => Ok(()),
        (count, Some(first), Some(last))
            if count > 0
                && first > 0
                && last >= first
                && last.checked_sub(first).and_then(|span| span.checked_add(1))
                    == Some(u64::from(count)) =>
        {
            Ok(())
        }
        _ => Err(BackendError::ProjectionMismatch),
    }
}

fn validate_checkpoint(checkpoint: ProjectionCheckpoint<'_>) -> Result<(), BackendError> {
    if checkpoint.last_global_sequence == Some(0) {
        Err(BackendError::ProjectionMismatch)
    } else {
        Ok(())
    }
}

fn require_one(changed: usize) -> Result<(), BackendError> {
    if changed == 1 {
        Ok(())
    } else {
        Err(BackendError::ConcurrentStorageChange)
    }
}

const fn status_projection(unit: &IntentUnit) -> &'static str {
    match unit.status() {
        cubikan_core::IntentUnitStatus::Active => "active",
        cubikan_core::IntentUnitStatus::Completed => "completed",
    }
}

const fn policy_projection(policy: RelationshipPolicy) -> &'static str {
    match policy {
        RelationshipPolicy::Allow => "allow",
        RelationshipPolicy::Reject => "reject",
    }
}

fn association_subject(subject: AssociationSubject) -> (&'static str, Vec<u8>) {
    match subject {
        AssociationSubject::WholeUnit => ("whole_unit", Vec::new()),
        AssociationSubject::Revision(revision) => {
            ("revision", stored::encode_u64_blob(revision).to_vec())
        }
    }
}

#[cfg(test)]
#[path = "projection_store/tests.rs"]
mod tests;
