//! Closed, node-trusted finalized archive-RPC primitives for CubiKan.
//!
//! This crate authenticates the fixed local deployment artifacts and one
//! process-backed archive endpoint before it exposes finalized blocks. It does
//! not verify GRANDPA proofs, promise perpetual archive retention, or turn the
//! configured local node into an independent finality authority.

#![forbid(unsafe_code)]

mod identity;
mod rpc;

use cubikan_core::{
    AssociationSubject, ExternalReference, IntentSpecies, IntentUnit, IntentUnitId, PhaseId,
    RecordedAssociation, ReferenceNamespace, ReferenceText, RelationshipDefinition,
    RelationshipDefinitionKey, RelationshipDefinitionVersion, RelationshipIdentity,
    RelationshipPolicy, Workflow, WorkflowEdge, WorkflowId,
};
use uuid::Uuid;

pub use identity::{
    ArchiveNodeEvidence, DeploymentIdentity, IdentityError, LoopbackUrlError, NodeEvidenceError,
    StrictLoopbackWsUrl,
};
pub use rpc::{ArchiveError, FinalizedBlock, FinalizedHead, VerifiedArchiveClient};

/// Exact maximum for one accepted event's SCALE domain payload.
pub const MAX_ACCEPTED_PAYLOAD_BYTES: usize = 1_048_576;

/// Closed accepted-event discriminator used by the projection store.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AcceptedEventKind {
    UnitCreated,
    UnitTransitioned,
    UnitCompleted,
    RelationshipDefinitionCreated,
    RelationshipCreated,
    RelationshipDeleted,
    AssociationRecorded,
    AssociationRevoked,
}

/// One canonical domain payload decoded through CubiKan's chain-neutral core.
///
/// There is no public raw-byte constructor. Instances reachable from an
/// [`AcceptedEvent`] were decoded from the fixed runtime's bounded SCALE bytes
/// and rejected if any byte trailed the canonical value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanonicalPayload {
    UnitCreated(IntentUnit),
    UnitTransitioned {
        unit_id: IntentUnitId,
        committed_revision: u64,
        from: PhaseId,
        to: PhaseId,
    },
    UnitCompleted {
        unit_id: IntentUnitId,
        committed_revision: u64,
        phase: PhaseId,
    },
    RelationshipDefinitionCreated(RelationshipDefinition),
    RelationshipCreated(RelationshipIdentity),
    RelationshipDeleted(RelationshipIdentity),
    AssociationRecorded(RecordedAssociation),
    AssociationRevoked(RecordedAssociation),
}

impl CanonicalPayload {
    /// Returns the stable projection discriminator for this payload.
    #[must_use]
    pub const fn kind(&self) -> AcceptedEventKind {
        match self {
            Self::UnitCreated(_) => AcceptedEventKind::UnitCreated,
            Self::UnitTransitioned { .. } => AcceptedEventKind::UnitTransitioned,
            Self::UnitCompleted { .. } => AcceptedEventKind::UnitCompleted,
            Self::RelationshipDefinitionCreated(_) => {
                AcceptedEventKind::RelationshipDefinitionCreated
            }
            Self::RelationshipCreated(_) => AcceptedEventKind::RelationshipCreated,
            Self::RelationshipDeleted(_) => AcceptedEventKind::RelationshipDeleted,
            Self::AssociationRecorded(_) => AcceptedEventKind::AssociationRecorded,
            Self::AssociationRevoked(_) => AcceptedEventKind::AssociationRevoked,
        }
    }
}

/// One accepted CubiKan event joined to its finalized block-body coordinate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedEvent {
    extrinsic_index: u32,
    system_event_index: u32,
    global_sequence: u64,
    deployment_id: [u8; 32],
    event_schema_version: u16,
    signer: [u8; 32],
    extrinsic_hash: [u8; 32],
    raw_payload: Vec<u8>,
    payload: CanonicalPayload,
}

impl AcceptedEvent {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        extrinsic_index: u32,
        system_event_index: u32,
        global_sequence: u64,
        deployment_id: [u8; 32],
        event_schema_version: u16,
        signer: [u8; 32],
        extrinsic_hash: [u8; 32],
        raw_payload: Vec<u8>,
        payload: CanonicalPayload,
    ) -> Self {
        Self {
            extrinsic_index,
            system_event_index,
            global_sequence,
            deployment_id,
            event_schema_version,
            signer,
            extrinsic_hash,
            raw_payload,
            payload,
        }
    }

    #[must_use]
    pub const fn extrinsic_index(&self) -> u32 {
        self.extrinsic_index
    }

    #[must_use]
    pub const fn system_event_index(&self) -> u32 {
        self.system_event_index
    }

    #[must_use]
    pub const fn global_sequence(&self) -> u64 {
        self.global_sequence
    }

    #[must_use]
    pub const fn deployment_id(&self) -> &[u8; 32] {
        &self.deployment_id
    }

    #[must_use]
    pub const fn event_schema_version(&self) -> u16 {
        self.event_schema_version
    }

    #[must_use]
    pub const fn signer(&self) -> &[u8; 32] {
        &self.signer
    }

    #[must_use]
    pub const fn extrinsic_hash(&self) -> &[u8; 32] {
        &self.extrinsic_hash
    }

    /// Returns the exact complete SCALE `DomainPayload`, including its variant.
    #[must_use]
    pub fn raw_payload(&self) -> &[u8] {
        &self.raw_payload
    }

    #[must_use]
    pub const fn payload(&self) -> &CanonicalPayload {
        &self.payload
    }

    #[must_use]
    pub const fn kind(&self) -> AcceptedEventKind {
        self.payload.kind()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PayloadDecodeError {
    Empty,
    OverBound,
    Truncated,
    NonCanonicalCompact,
    InvalidVariant,
    InvalidDomain,
    TrailingBytes,
}

pub(crate) fn decode_canonical_payload(
    bytes: &[u8],
) -> Result<CanonicalPayload, PayloadDecodeError> {
    if bytes.is_empty() {
        return Err(PayloadDecodeError::Empty);
    }
    if bytes.len() > MAX_ACCEPTED_PAYLOAD_BYTES {
        return Err(PayloadDecodeError::OverBound);
    }
    let mut input = ScaleCursor::new(bytes);
    let payload = match input.byte()? {
        0 => decode_unit_created(&mut input)?,
        1 => decode_unit_transitioned(&mut input)?,
        2 => decode_unit_completed(&mut input)?,
        3 => CanonicalPayload::RelationshipDefinitionCreated(decode_definition(&mut input)?),
        4 => CanonicalPayload::RelationshipCreated(decode_relationship(&mut input)?),
        5 => CanonicalPayload::RelationshipDeleted(decode_relationship(&mut input)?),
        6 => CanonicalPayload::AssociationRecorded(decode_association(&mut input)?),
        7 => CanonicalPayload::AssociationRevoked(decode_association(&mut input)?),
        _ => return Err(PayloadDecodeError::InvalidVariant),
    };
    if input.is_empty() {
        Ok(payload)
    } else {
        Err(PayloadDecodeError::TrailingBytes)
    }
}

fn decode_unit_created(
    input: &mut ScaleCursor<'_>,
) -> Result<CanonicalPayload, PayloadDecodeError> {
    if input.u16()? != 1 {
        return Err(PayloadDecodeError::InvalidDomain);
    }
    let id = decode_unit_id(input)?;
    let origin = decode_external_reference(input)?;
    let species = decode_species(input)?;
    let workflow = decode_workflow(input)?;
    Ok(CanonicalPayload::UnitCreated(IntentUnit::new(
        id, origin, species, workflow,
    )))
}

fn decode_unit_transitioned(
    input: &mut ScaleCursor<'_>,
) -> Result<CanonicalPayload, PayloadDecodeError> {
    let unit_id = decode_unit_id(input)?;
    let committed_revision = input.u64()?;
    if committed_revision == 0 {
        return Err(PayloadDecodeError::InvalidDomain);
    }
    Ok(CanonicalPayload::UnitTransitioned {
        unit_id,
        committed_revision,
        from: decode_phase(input)?,
        to: decode_phase(input)?,
    })
}

fn decode_unit_completed(
    input: &mut ScaleCursor<'_>,
) -> Result<CanonicalPayload, PayloadDecodeError> {
    let unit_id = decode_unit_id(input)?;
    let committed_revision = input.u64()?;
    if committed_revision == 0 {
        return Err(PayloadDecodeError::InvalidDomain);
    }
    Ok(CanonicalPayload::UnitCompleted {
        unit_id,
        committed_revision,
        phase: decode_phase(input)?,
    })
}

fn decode_external_reference(
    input: &mut ScaleCursor<'_>,
) -> Result<ExternalReference, PayloadDecodeError> {
    let namespace = ReferenceNamespace::from_bytes(input.bounded_bytes(64)?)
        .map_err(|_| PayloadDecodeError::InvalidDomain)?;
    let scope = ReferenceText::from_bytes(input.bounded_bytes(256)?)
        .map_err(|_| PayloadDecodeError::InvalidDomain)?;
    let value = ReferenceText::from_bytes(input.bounded_bytes(256)?)
        .map_err(|_| PayloadDecodeError::InvalidDomain)?;
    Ok(ExternalReference::new(namespace, scope, value))
}

fn decode_species(input: &mut ScaleCursor<'_>) -> Result<IntentSpecies, PayloadDecodeError> {
    IntentSpecies::from_bytes(input.bounded_bytes(256)?)
        .map_err(|_| PayloadDecodeError::InvalidDomain)
}

fn decode_phase(input: &mut ScaleCursor<'_>) -> Result<PhaseId, PayloadDecodeError> {
    PhaseId::from_bytes(input.bounded_bytes(256)?).map_err(|_| PayloadDecodeError::InvalidDomain)
}

fn decode_workflow(input: &mut ScaleCursor<'_>) -> Result<Workflow, PayloadDecodeError> {
    let id = WorkflowId::from_bytes(input.bounded_bytes(256)?)
        .map_err(|_| PayloadDecodeError::InvalidDomain)?;
    let phase_count = input.compact_len(32)?;
    let mut phases = Vec::with_capacity(phase_count);
    for _ in 0..phase_count {
        phases.push(decode_phase(input)?);
    }
    let initial_phase = decode_phase(input)?;
    let edge_count = input.compact_len(128)?;
    let mut edges = Vec::with_capacity(edge_count);
    for _ in 0..edge_count {
        edges.push(WorkflowEdge::new(
            decode_phase(input)?,
            decode_phase(input)?,
        ));
    }
    let completion_count = input.compact_len(32)?;
    let mut completion_phases = Vec::with_capacity(completion_count);
    for _ in 0..completion_count {
        completion_phases.push(decode_phase(input)?);
    }
    Workflow::new_bounded(id, phases, initial_phase, edges, completion_phases)
        .map_err(|_| PayloadDecodeError::InvalidDomain)
}

fn decode_definition_key(
    input: &mut ScaleCursor<'_>,
) -> Result<RelationshipDefinitionKey, PayloadDecodeError> {
    let id = ReferenceNamespace::from_bytes(input.bounded_bytes(64)?)
        .map_err(|_| PayloadDecodeError::InvalidDomain)?;
    let version = RelationshipDefinitionVersion::new(input.u64()?)
        .map_err(|_| PayloadDecodeError::InvalidDomain)?;
    Ok(RelationshipDefinitionKey::new(id, version))
}

fn decode_optional_species(
    input: &mut ScaleCursor<'_>,
) -> Result<Option<IntentSpecies>, PayloadDecodeError> {
    match input.byte()? {
        0 => Ok(None),
        1 => decode_species(input).map(Some),
        _ => Err(PayloadDecodeError::InvalidVariant),
    }
}

fn decode_policy(input: &mut ScaleCursor<'_>) -> Result<RelationshipPolicy, PayloadDecodeError> {
    match input.byte()? {
        0 => Ok(RelationshipPolicy::Allow),
        1 => Ok(RelationshipPolicy::Reject),
        _ => Err(PayloadDecodeError::InvalidVariant),
    }
}

fn decode_definition(
    input: &mut ScaleCursor<'_>,
) -> Result<RelationshipDefinition, PayloadDecodeError> {
    let key = decode_definition_key(input)?;
    if input.byte()? != 0 {
        return Err(PayloadDecodeError::InvalidVariant);
    }
    let source_species = decode_optional_species(input)?;
    let target_species = decode_optional_species(input)?;
    let self_policy = decode_policy(input)?;
    let cycle_policy = decode_policy(input)?;
    Ok(RelationshipDefinition::new(
        key,
        source_species,
        target_species,
        self_policy,
        cycle_policy,
    ))
}

fn decode_relationship(
    input: &mut ScaleCursor<'_>,
) -> Result<RelationshipIdentity, PayloadDecodeError> {
    Ok(RelationshipIdentity::new(
        decode_definition_key(input)?,
        decode_unit_id(input)?,
        decode_unit_id(input)?,
    ))
}

fn decode_association(
    input: &mut ScaleCursor<'_>,
) -> Result<RecordedAssociation, PayloadDecodeError> {
    let unit_id = decode_unit_id(input)?;
    let subject = match input.byte()? {
        0 => AssociationSubject::WholeUnit,
        1 => AssociationSubject::Revision(input.u64()?),
        _ => return Err(PayloadDecodeError::InvalidVariant),
    };
    Ok(RecordedAssociation::new(
        unit_id,
        subject,
        decode_external_reference(input)?,
    ))
}

fn decode_unit_id(input: &mut ScaleCursor<'_>) -> Result<IntentUnitId, PayloadDecodeError> {
    Ok(IntentUnitId::from_uuid(Uuid::from_bytes(input.array()?)))
}

struct ScaleCursor<'a> {
    remaining: &'a [u8],
}

impl<'a> ScaleCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    const fn is_empty(&self) -> bool {
        self.remaining.is_empty()
    }

    fn byte(&mut self) -> Result<u8, PayloadDecodeError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, PayloadDecodeError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, PayloadDecodeError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], PayloadDecodeError> {
        self.take(N)?
            .try_into()
            .map_err(|_| PayloadDecodeError::Truncated)
    }

    fn bounded_bytes(&mut self, maximum: usize) -> Result<&'a [u8], PayloadDecodeError> {
        let length = self.compact_len(maximum)?;
        self.take(length)
    }

    fn compact_len(&mut self, maximum: usize) -> Result<usize, PayloadDecodeError> {
        let first = self.byte()?;
        let value = match first & 0b11 {
            0 => u32::from(first >> 2),
            1 => {
                let second = self.byte()?;
                let value = u32::from(u16::from_le_bytes([first, second]) >> 2);
                if value < 1 << 6 {
                    return Err(PayloadDecodeError::NonCanonicalCompact);
                }
                value
            }
            2 => {
                let tail: [u8; 3] = self.array()?;
                let value = u32::from_le_bytes([first, tail[0], tail[1], tail[2]]) >> 2;
                if value < 1 << 14 {
                    return Err(PayloadDecodeError::NonCanonicalCompact);
                }
                value
            }
            _ => return Err(PayloadDecodeError::OverBound),
        };
        let value = usize::try_from(value).map_err(|_| PayloadDecodeError::OverBound)?;
        if value > maximum {
            return Err(PayloadDecodeError::OverBound);
        }
        Ok(value)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], PayloadDecodeError> {
        if self.remaining.len() < length {
            return Err(PayloadDecodeError::Truncated);
        }
        let (value, remaining) = self.remaining.split_at(length);
        self.remaining = remaining;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use cubikan_core::IntentUnitStatus;

    use super::*;

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

    fn decode_hex(input: &str) -> Vec<u8> {
        let input = input.trim();
        assert_eq!(input.len() % 2, 0);
        input
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII hex"), 16)
                    .expect("lowercase fixture hex")
            })
            .collect()
    }

    #[test]
    fn independent_payload_corpus_maps_to_the_closed_inventory() {
        let decoded = PAYLOADS
            .iter()
            .map(|payload| {
                decode_canonical_payload(&decode_hex(payload))
                    .expect("fixture payload should decode")
            })
            .collect::<Vec<_>>();

        let unit_a = match &decoded[0] {
            CanonicalPayload::UnitCreated(unit) => unit,
            other => panic!("unexpected first payload: {other:?}"),
        };
        assert_unit_created(unit_a, "00112233-4455-4677-8899-aabbccddeeff", "INT-0008");
        let unit_b = match &decoded[1] {
            CanonicalPayload::UnitCreated(unit) => unit,
            other => panic!("unexpected second payload: {other:?}"),
        };
        assert_unit_created(unit_b, "10213243-5465-4767-98a9-bacbdcedfe0f", "INT-0014");

        let definition = match &decoded[2] {
            CanonicalPayload::RelationshipDefinitionCreated(definition) => definition,
            other => panic!("unexpected definition payload: {other:?}"),
        };
        assert_eq!(definition.key().id().as_str(), "depends_on");
        assert_eq!(definition.key().version().value(), 7);
        assert_eq!(definition.source_species().unwrap().as_str(), "task");
        assert_eq!(definition.target_species().unwrap().as_str(), "task");
        assert_eq!(definition.self_policy(), RelationshipPolicy::Reject);
        assert_eq!(definition.cycle_policy(), RelationshipPolicy::Reject);

        let relationship_created = match &decoded[3] {
            CanonicalPayload::RelationshipCreated(relationship) => relationship,
            other => panic!("unexpected relationship-create payload: {other:?}"),
        };
        assert_relationship(relationship_created);

        let association_a = match &decoded[4] {
            CanonicalPayload::AssociationRecorded(association) => association,
            other => panic!("unexpected association payload: {other:?}"),
        };
        assert_association(
            association_a,
            "00112233-4455-4677-8899-aabbccddeeff",
            AssociationSubject::Revision(0),
        );

        match &decoded[5] {
            CanonicalPayload::UnitTransitioned {
                unit_id,
                committed_revision,
                from,
                to,
            } => {
                assert_eq!(
                    unit_id.as_uuid().to_string(),
                    "00112233-4455-4677-8899-aabbccddeeff"
                );
                assert_eq!(*committed_revision, 1);
                assert_eq!(from.as_str(), "queued");
                assert_eq!(to.as_str(), "doing");
            }
            other => panic!("unexpected transition payload: {other:?}"),
        }
        match &decoded[6] {
            CanonicalPayload::UnitCompleted {
                unit_id,
                committed_revision,
                phase,
            } => {
                assert_eq!(
                    unit_id.as_uuid().to_string(),
                    "00112233-4455-4677-8899-aabbccddeeff"
                );
                assert_eq!(*committed_revision, 2);
                assert_eq!(phase.as_str(), "doing");
            }
            other => panic!("unexpected completion payload: {other:?}"),
        }

        let relationship_deleted = match &decoded[7] {
            CanonicalPayload::RelationshipDeleted(relationship) => relationship,
            other => panic!("unexpected relationship-delete payload: {other:?}"),
        };
        assert_relationship(relationship_deleted);

        let revoked_a = match &decoded[8] {
            CanonicalPayload::AssociationRevoked(association) => association,
            other => panic!("unexpected revocation payload: {other:?}"),
        };
        assert_association(
            revoked_a,
            "00112233-4455-4677-8899-aabbccddeeff",
            AssociationSubject::Revision(0),
        );
        let association_b = match &decoded[9] {
            CanonicalPayload::AssociationRecorded(association) => association,
            other => panic!("unexpected second association payload: {other:?}"),
        };
        assert_association(
            association_b,
            "10213243-5465-4767-98a9-bacbdcedfe0f",
            AssociationSubject::WholeUnit,
        );
        let relationship_recreated = match &decoded[10] {
            CanonicalPayload::RelationshipCreated(relationship) => relationship,
            other => panic!("unexpected relationship-recreate payload: {other:?}"),
        };
        assert_relationship(relationship_recreated);
    }

    #[test]
    fn payload_decoder_rejects_trailing_noncanonical_and_overbound_bytes() {
        let mut valid = decode_hex(PAYLOADS[0]);
        valid.push(0);
        assert_eq!(
            decode_canonical_payload(&valid),
            Err(PayloadDecodeError::TrailingBytes)
        );
        assert_eq!(
            decode_canonical_payload(&vec![0; MAX_ACCEPTED_PAYLOAD_BYTES + 1]),
            Err(PayloadDecodeError::OverBound)
        );
        let mut wrong_schema = decode_hex(PAYLOADS[0]);
        wrong_schema[1..3].copy_from_slice(&2_u16.to_le_bytes());
        assert_eq!(
            decode_canonical_payload(&wrong_schema),
            Err(PayloadDecodeError::InvalidDomain)
        );
        assert_eq!(
            decode_canonical_payload(&[0]),
            Err(PayloadDecodeError::Truncated)
        );

        let mut noncanonical = decode_hex(PAYLOADS[0]);
        assert_eq!(noncanonical[19], 11 << 2);
        noncanonical[19] = (11 << 2) | 1;
        noncanonical.insert(20, 0);
        assert_eq!(
            decode_canonical_payload(&noncanonical),
            Err(PayloadDecodeError::NonCanonicalCompact)
        );
    }

    fn assert_unit_created(unit: &IntentUnit, id: &str, origin_value: &str) {
        assert_eq!(unit.id().as_uuid().to_string(), id);
        assert_eq!(unit.origin().namespace().as_str(), "book.intent");
        assert_eq!(unit.origin().scope().as_str(), "sprint-11");
        assert_eq!(unit.origin().value().as_str(), origin_value);
        assert_eq!(unit.species().as_str(), "task");
        assert_eq!(unit.workflow().id().as_str(), "lifecycle-v1");
        assert_eq!(
            unit.workflow()
                .phases()
                .iter()
                .map(PhaseId::as_str)
                .collect::<Vec<_>>(),
            ["queued", "doing"]
        );
        assert_eq!(unit.workflow().initial_phase().as_str(), "queued");
        assert_eq!(unit.workflow().edges().len(), 1);
        assert_eq!(unit.workflow().edges()[0].from().as_str(), "queued");
        assert_eq!(unit.workflow().edges()[0].to().as_str(), "doing");
        assert_eq!(unit.workflow().completion_phases().len(), 1);
        assert_eq!(unit.workflow().completion_phases()[0].as_str(), "doing");
        assert_eq!(unit.phase().as_str(), "queued");
        assert_eq!(unit.status(), IntentUnitStatus::Active);
        assert_eq!(unit.revision().value(), 0);
        assert!(unit.history().is_empty());
    }

    fn assert_relationship(relationship: &RelationshipIdentity) {
        assert_eq!(relationship.definition().id().as_str(), "depends_on");
        assert_eq!(relationship.definition().version().value(), 7);
        assert_eq!(
            relationship.source().as_uuid().to_string(),
            "00112233-4455-4677-8899-aabbccddeeff"
        );
        assert_eq!(
            relationship.target().as_uuid().to_string(),
            "10213243-5465-4767-98a9-bacbdcedfe0f"
        );
    }

    fn assert_association(
        association: &RecordedAssociation,
        unit_id: &str,
        subject: AssociationSubject,
    ) {
        assert_eq!(association.unit_id().as_uuid().to_string(), unit_id);
        assert_eq!(association.subject(), subject);
        assert_eq!(
            association.reference().namespace().as_str(),
            "git.commit.sha256"
        );
        assert_eq!(
            association.reference().scope().as_str(),
            "public-synthetic/repository"
        );
        assert_eq!(
            association.reference().value().as_str(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
    }
}
