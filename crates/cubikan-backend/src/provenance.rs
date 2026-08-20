use std::{cmp::Ordering, error::Error, fmt};

use cubikan_core::{
    AssociationSubject, ExternalReference, IntentUnitId, RecordedAssociation, ReferenceNamespace,
    ReferenceText,
};
use rusqlite::{Connection, Row, params};

use crate::{
    BackendError, LedgerCoordinate, PageLimit, ProjectionCheckpoint, ReadError,
    VerifiedReadSnapshot, query, sqlite::classify_runtime_error, stored,
};

const SELECT_ASSOCIATIONS_BY_UNIT_SQL: &str = "SELECT association.unit_id,association.subject_kind,association.subject_revision_key,association.namespace,association.scope,association.value,anchor.parachain_genesis_hash,event.deployment_id,event.block_number,block.block_hash,event.extrinsic_index,event.extrinsic_hash,event.system_event_index,event.global_sequence,event.event_schema_version,event.event_kind FROM recorded_associations AS association LEFT JOIN projected_events AS event ON event.global_sequence=association.created_global_sequence LEFT JOIN projected_blocks AS block ON block.block_number=event.block_number LEFT JOIN projection_anchor AS anchor ON anchor.singleton=block.anchor_singleton AND anchor.deployment_id=event.deployment_id WHERE association.unit_id=?1 COLLATE BINARY AND (?2 IS NULL OR (association.subject_kind=?2 COLLATE BINARY AND association.subject_revision_key=?3)) AND (?4 IS NULL OR association.subject_kind>?4 COLLATE BINARY OR (association.subject_kind=?4 COLLATE BINARY AND (association.subject_revision_key>?5 OR (association.subject_revision_key=?5 AND (association.namespace>?6 COLLATE BINARY OR (association.namespace=?6 COLLATE BINARY AND (association.scope>?7 COLLATE BINARY OR (association.scope=?7 COLLATE BINARY AND association.value>?8 COLLATE BINARY)))))))) ORDER BY association.subject_kind COLLATE BINARY,association.subject_revision_key,association.namespace COLLATE BINARY,association.scope COLLATE BINARY,association.value COLLATE BINARY LIMIT ?9";
const SELECT_ASSOCIATIONS_BY_REFERENCE_SQL: &str = "SELECT association.unit_id,association.subject_kind,association.subject_revision_key,association.namespace,association.scope,association.value,anchor.parachain_genesis_hash,event.deployment_id,event.block_number,block.block_hash,event.extrinsic_index,event.extrinsic_hash,event.system_event_index,event.global_sequence,event.event_schema_version,event.event_kind FROM recorded_associations AS association LEFT JOIN projected_events AS event ON event.global_sequence=association.created_global_sequence LEFT JOIN projected_blocks AS block ON block.block_number=event.block_number LEFT JOIN projection_anchor AS anchor ON anchor.singleton=block.anchor_singleton AND anchor.deployment_id=event.deployment_id WHERE association.namespace=?1 COLLATE BINARY AND association.scope=?2 COLLATE BINARY AND association.value=?3 COLLATE BINARY AND (?4 IS NULL OR association.unit_id>?4 COLLATE BINARY OR (association.unit_id=?4 COLLATE BINARY AND (association.subject_kind>?5 COLLATE BINARY OR (association.subject_kind=?5 COLLATE BINARY AND association.subject_revision_key>?6)))) ORDER BY association.unit_id COLLATE BINARY,association.subject_kind COLLATE BINARY,association.subject_revision_key LIMIT ?7";

/// Input for one bounded forward provenance query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListAssociationsByUnit {
    unit_id: IntentUnitId,
    subject: Option<AssociationSubject>,
    limit: PageLimit,
    after: Option<RecordedAssociation>,
}

impl ListAssociationsByUnit {
    pub fn new(
        unit_id: IntentUnitId,
        subject: Option<AssociationSubject>,
        limit: PageLimit,
        after: Option<RecordedAssociation>,
    ) -> Result<Self, AssociationQueryError> {
        if let Some(cursor) = &after {
            if cursor.unit_id() != unit_id {
                return Err(AssociationQueryError::CursorUnitMismatch {
                    expected: unit_id,
                    actual: cursor.unit_id(),
                });
            }
            if let Some(expected) = subject
                && cursor.subject() != expected
            {
                return Err(AssociationQueryError::CursorSubjectMismatch {
                    expected,
                    actual: cursor.subject(),
                });
            }
        }
        Ok(Self {
            unit_id,
            subject,
            limit,
            after,
        })
    }

    #[must_use]
    pub const fn unit_id(&self) -> IntentUnitId {
        self.unit_id
    }

    #[must_use]
    pub const fn subject(&self) -> Option<AssociationSubject> {
        self.subject
    }

    #[must_use]
    pub const fn limit(&self) -> PageLimit {
        self.limit
    }

    #[must_use]
    pub const fn after(&self) -> Option<&RecordedAssociation> {
        self.after.as_ref()
    }
}

/// Input for one bounded reverse provenance query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListAssociationsByReference {
    reference: ExternalReference,
    limit: PageLimit,
    after: Option<RecordedAssociation>,
}

impl ListAssociationsByReference {
    pub fn new(
        reference: ExternalReference,
        limit: PageLimit,
        after: Option<RecordedAssociation>,
    ) -> Result<Self, AssociationQueryError> {
        if let Some(cursor) = &after
            && cursor.reference() != &reference
        {
            return Err(AssociationQueryError::CursorReferenceMismatch {
                expected: Box::new(reference),
                actual: Box::new(cursor.reference().clone()),
            });
        }
        Ok(Self {
            reference,
            limit,
            after,
        })
    }

    #[must_use]
    pub const fn reference(&self) -> &ExternalReference {
        &self.reference
    }

    #[must_use]
    pub const fn limit(&self) -> PageLimit {
        self.limit
    }

    #[must_use]
    pub const fn after(&self) -> Option<&RecordedAssociation> {
        self.after.as_ref()
    }
}

/// Structural rejection from an association cursor that escapes its query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssociationQueryError {
    CursorUnitMismatch {
        expected: IntentUnitId,
        actual: IntentUnitId,
    },
    CursorSubjectMismatch {
        expected: AssociationSubject,
        actual: AssociationSubject,
    },
    CursorReferenceMismatch {
        expected: Box<ExternalReference>,
        actual: Box<ExternalReference>,
    },
}

impl fmt::Display for AssociationQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CursorUnitMismatch { expected, actual } => write!(
                formatter,
                "association cursor unit `{actual}` does not match query unit `{expected}`"
            ),
            Self::CursorSubjectMismatch { expected, actual } => write!(
                formatter,
                "association cursor subject {actual:?} does not match query subject {expected:?}"
            ),
            Self::CursorReferenceMismatch { expected, actual } => write!(
                formatter,
                "association cursor reference {actual:?} does not match query reference {expected:?}"
            ),
        }
    }
}

impl Error for AssociationQueryError {}

/// Forward or reverse complete-key order used for one association page.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssociationDirection {
    ByUnit,
    ByReference,
}

/// One active canonical association and the event that recorded it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedAssociation {
    key: RecordedAssociation,
    created_coordinate: LedgerCoordinate,
}

impl ProjectedAssociation {
    pub(crate) const fn new(
        key: RecordedAssociation,
        created_coordinate: LedgerCoordinate,
    ) -> Self {
        Self {
            key,
            created_coordinate,
        }
    }

    #[must_use]
    pub const fn key(&self) -> &RecordedAssociation {
        &self.key
    }

    #[must_use]
    pub const fn created_coordinate(&self) -> &LedgerCoordinate {
        &self.created_coordinate
    }
}

/// One bounded association page at an attested projection checkpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssociationPage {
    direction: AssociationDirection,
    items: Vec<ProjectedAssociation>,
    next_cursor: Option<RecordedAssociation>,
    checkpoint: ProjectionCheckpoint,
}

impl AssociationPage {
    pub(crate) const fn new(
        direction: AssociationDirection,
        items: Vec<ProjectedAssociation>,
        next_cursor: Option<RecordedAssociation>,
        checkpoint: ProjectionCheckpoint,
    ) -> Self {
        Self {
            direction,
            items,
            next_cursor,
            checkpoint,
        }
    }

    #[must_use]
    pub const fn direction(&self) -> AssociationDirection {
        self.direction
    }

    #[must_use]
    pub fn items(&self) -> &[ProjectedAssociation] {
        &self.items
    }

    #[must_use]
    pub const fn next_cursor(&self) -> Option<&RecordedAssociation> {
        self.next_cursor.as_ref()
    }

    #[must_use]
    pub const fn checkpoint(&self) -> &ProjectionCheckpoint {
        &self.checkpoint
    }
}

impl VerifiedReadSnapshot {
    /// Reads one bounded forward association page from this attested snapshot.
    pub fn list_associations_by_unit(
        self,
        query: ListAssociationsByUnit,
    ) -> Result<AssociationPage, ReadError> {
        self.consume(move |connection, checkpoint| {
            list_by_unit(connection, query, checkpoint).map_err(ReadError::from)
        })
    }

    /// Reads one bounded reverse association page from this attested snapshot.
    pub fn list_associations_by_reference(
        self,
        query: ListAssociationsByReference,
    ) -> Result<AssociationPage, ReadError> {
        self.consume(move |connection, checkpoint| {
            list_by_reference(connection, query, checkpoint).map_err(ReadError::from)
        })
    }
}

pub(crate) fn list_by_unit(
    connection: &Connection,
    query: ListAssociationsByUnit,
    checkpoint: &ProjectionCheckpoint,
) -> Result<AssociationPage, BackendError> {
    let unit = query::load_projected_unit(connection, query.unit_id())?;
    query::validate_projected_coordinate(unit.last_coordinate(), checkpoint)?;
    let subject = query.subject().map(encode_subject);
    let cursor = query.after().map(association_storage_key);
    let fetch_limit = query
        .limit()
        .value()
        .checked_add(1)
        .ok_or(BackendError::ProjectionMismatch)?;
    let fetch_limit_sql =
        i64::try_from(fetch_limit).map_err(|_| BackendError::ProjectionMismatch)?;
    let mut statement = connection
        .prepare(SELECT_ASSOCIATIONS_BY_UNIT_SQL)
        .map_err(classify_runtime_error)?;
    let mut rows = statement
        .query(params![
            query.unit_id().to_string(),
            subject.as_ref().map(|(kind, _)| *kind),
            subject.as_ref().map(|(_, revision)| revision.as_slice()),
            cursor.as_ref().map(|key| key.subject_kind.as_str()),
            cursor.as_ref().map(|key| key.subject_revision.as_slice()),
            cursor.as_ref().map(|key| key.namespace.as_str()),
            cursor.as_ref().map(|key| key.scope.as_str()),
            cursor.as_ref().map(|key| key.value.as_str()),
            fetch_limit_sql,
        ])
        .map_err(classify_runtime_error)?;

    let mut items = Vec::with_capacity(fetch_limit);
    let mut previous = query.after().cloned();
    while let Some(row) = rows.next().map_err(classify_runtime_error)? {
        let projected = decode_projected_association(row)?;
        query::validate_projected_coordinate(projected.created_coordinate(), checkpoint)?;
        if projected.key().unit_id() != query.unit_id()
            || query
                .subject()
                .is_some_and(|subject| subject != projected.key().subject())
            || previous.as_ref().is_some_and(|previous| {
                compare_by_unit(previous, projected.key()) != Ordering::Less
            })
        {
            return Err(BackendError::ProjectionMismatch);
        }
        validate_association_subject(&unit, projected.key())?;
        previous = Some(projected.key().clone());
        items.push(projected);
    }
    Ok(finish_page(
        AssociationDirection::ByUnit,
        query.limit(),
        items,
        checkpoint,
    ))
}

pub(crate) fn list_by_reference(
    connection: &Connection,
    query: ListAssociationsByReference,
    checkpoint: &ProjectionCheckpoint,
) -> Result<AssociationPage, BackendError> {
    let cursor = query.after().map(association_storage_key);
    let fetch_limit = query
        .limit()
        .value()
        .checked_add(1)
        .ok_or(BackendError::ProjectionMismatch)?;
    let fetch_limit_sql =
        i64::try_from(fetch_limit).map_err(|_| BackendError::ProjectionMismatch)?;
    let mut statement = connection
        .prepare(SELECT_ASSOCIATIONS_BY_REFERENCE_SQL)
        .map_err(classify_runtime_error)?;
    let mut rows = statement
        .query(params![
            query.reference().namespace().as_str(),
            query.reference().scope().as_str(),
            query.reference().value().as_str(),
            cursor.as_ref().map(|key| key.unit_id.as_str()),
            cursor.as_ref().map(|key| key.subject_kind.as_str()),
            cursor.as_ref().map(|key| key.subject_revision.as_slice()),
            fetch_limit_sql,
        ])
        .map_err(classify_runtime_error)?;

    let mut items = Vec::with_capacity(fetch_limit);
    let mut previous = query.after().cloned();
    while let Some(row) = rows.next().map_err(classify_runtime_error)? {
        let projected = decode_projected_association(row)?;
        query::validate_projected_coordinate(projected.created_coordinate(), checkpoint)?;
        if projected.key().reference() != query.reference()
            || previous.as_ref().is_some_and(|previous| {
                compare_by_reference(previous, projected.key()) != Ordering::Less
            })
        {
            return Err(BackendError::ProjectionMismatch);
        }
        let unit = query::load_projected_unit(connection, projected.key().unit_id())
            .map_err(|_| BackendError::ProjectionMismatch)?;
        query::validate_projected_coordinate(unit.last_coordinate(), checkpoint)?;
        validate_association_subject(&unit, projected.key())?;
        previous = Some(projected.key().clone());
        items.push(projected);
    }
    Ok(finish_page(
        AssociationDirection::ByReference,
        query.limit(),
        items,
        checkpoint,
    ))
}

fn finish_page(
    direction: AssociationDirection,
    limit: PageLimit,
    mut items: Vec<ProjectedAssociation>,
    checkpoint: &ProjectionCheckpoint,
) -> AssociationPage {
    let has_more = items.len() > limit.value();
    items.truncate(limit.value());
    let next_cursor = if has_more {
        items.last().map(|item| item.key().clone())
    } else {
        None
    };
    AssociationPage::new(direction, items, next_cursor, checkpoint.clone())
}

fn decode_projected_association(row: &Row<'_>) -> Result<ProjectedAssociation, BackendError> {
    let unit_text = row.get::<_, String>(0).map_err(classify_runtime_error)?;
    let subject_kind = row.get::<_, String>(1).map_err(classify_runtime_error)?;
    let subject_revision = row.get::<_, Vec<u8>>(2).map_err(classify_runtime_error)?;
    let namespace = row.get::<_, String>(3).map_err(classify_runtime_error)?;
    let scope = row.get::<_, String>(4).map_err(classify_runtime_error)?;
    let value = row.get::<_, String>(5).map_err(classify_runtime_error)?;
    let unit_id = unit_text
        .parse::<IntentUnitId>()
        .map_err(|_| BackendError::ProjectionMismatch)?;
    if unit_id.to_string() != unit_text {
        return Err(BackendError::ProjectionMismatch);
    }
    let subject = decode_subject(&subject_kind, &subject_revision)?;
    let reference = ExternalReference::new(
        ReferenceNamespace::new(namespace).map_err(|_| BackendError::ProjectionMismatch)?,
        ReferenceText::new(scope).map_err(|_| BackendError::ProjectionMismatch)?,
        ReferenceText::new(value).map_err(|_| BackendError::ProjectionMismatch)?,
    );
    query::validate_projected_event_binding(row, 14, "association_recorded")?;
    let coordinate = query::decode_ledger_coordinate(row, 6)?;
    Ok(ProjectedAssociation::new(
        RecordedAssociation::new(unit_id, subject, reference),
        coordinate,
    ))
}

fn validate_association_subject(
    unit: &crate::ProjectedUnit,
    association: &RecordedAssociation,
) -> Result<(), BackendError> {
    if unit.intent_unit().id() != association.unit_id()
        || matches!(
            association.subject(),
            AssociationSubject::Revision(revision)
                if revision > unit.intent_unit().revision().value()
        )
    {
        return Err(BackendError::ProjectionMismatch);
    }
    Ok(())
}

fn encode_subject(subject: AssociationSubject) -> (&'static str, Vec<u8>) {
    match subject {
        AssociationSubject::WholeUnit => ("whole_unit", Vec::new()),
        AssociationSubject::Revision(revision) => {
            ("revision", stored::encode_u64_blob(revision).to_vec())
        }
    }
}

fn decode_subject(kind: &str, revision: &[u8]) -> Result<AssociationSubject, BackendError> {
    match kind {
        "whole_unit" if revision.is_empty() => Ok(AssociationSubject::WholeUnit),
        "revision" => stored::decode_u64_blob(revision)
            .map(AssociationSubject::Revision)
            .map_err(|_| BackendError::ProjectionMismatch),
        _ => Err(BackendError::ProjectionMismatch),
    }
}

struct AssociationStorageKey {
    unit_id: String,
    subject_kind: String,
    subject_revision: Vec<u8>,
    namespace: String,
    scope: String,
    value: String,
}

fn association_storage_key(association: &RecordedAssociation) -> AssociationStorageKey {
    let (subject_kind, subject_revision) = encode_subject(association.subject());
    AssociationStorageKey {
        unit_id: association.unit_id().to_string(),
        subject_kind: subject_kind.to_owned(),
        subject_revision,
        namespace: association.reference().namespace().as_str().to_owned(),
        scope: association.reference().scope().as_str().to_owned(),
        value: association.reference().value().as_str().to_owned(),
    }
}

fn compare_by_unit(left: &RecordedAssociation, right: &RecordedAssociation) -> Ordering {
    let left = association_storage_key(left);
    let right = association_storage_key(right);
    compare_bytes(&left.subject_kind, &right.subject_kind)
        .then_with(|| left.subject_revision.cmp(&right.subject_revision))
        .then_with(|| compare_bytes(&left.namespace, &right.namespace))
        .then_with(|| compare_bytes(&left.scope, &right.scope))
        .then_with(|| compare_bytes(&left.value, &right.value))
}

fn compare_by_reference(left: &RecordedAssociation, right: &RecordedAssociation) -> Ordering {
    let left = association_storage_key(left);
    let right = association_storage_key(right);
    compare_bytes(&left.unit_id, &right.unit_id)
        .then_with(|| compare_bytes(&left.subject_kind, &right.subject_kind))
        .then_with(|| left.subject_revision.cmp(&right.subject_revision))
}

fn compare_bytes(left: &str, right: &str) -> Ordering {
    left.as_bytes().cmp(right.as_bytes())
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) fn exercise_supported_snapshot_query_matrix() {
    tests::exercise_supported_snapshot_query_matrix();
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroU64, str::FromStr};

    #[cfg(target_os = "linux")]
    use std::{
        ffi::OsString,
        fs,
        os::unix::fs::DirBuilderExt,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering as AtomicOrdering},
    };

    #[cfg(target_os = "linux")]
    use cubikan_core::IntentUnitStatus;
    use cubikan_core::{
        IntentSpecies, IntentUnit, PhaseId, RelationshipDefinition,
        RelationshipDefinitionKey as CoreDefinitionKey,
        RelationshipDefinitionVersion as CoreDefinitionVersion,
        RelationshipIdentity as CoreRelationshipIdentity, RelationshipPolicy, Workflow, WorkflowId,
    };
    use rusqlite::{Connection, Params, TransactionBehavior};

    use super::*;
    use crate::{
        DirectRelationshipPredicate, ListFilters, ListRelationships, ProjectionQueryV1,
        RelationshipDefinitionId, RelationshipDefinitionKey, RelationshipDefinitionVersion,
        projection_store::{
            ProjectedBlock, ProjectedEvent, ProjectedEventKind, ProjectionAnchor,
            ProjectionCheckpoint as StoredCheckpoint, ProjectionStatement, ProjectionWriter,
            insert_anchor, insert_association, insert_block, insert_checkpoint, insert_event,
            insert_intent_unit, insert_relationship, insert_relationship_definition,
        },
    };

    #[cfg(target_os = "linux")]
    use crate::{
        GetIntentUnit, RelationshipCursor, RelationshipIdentity, sqlite::create_fresh_projection,
        verified_read::issue_test_snapshot,
    };

    const BLOCK_NUMBER: u64 = 9;
    const EVENT_COUNT: u64 = 10;

    #[cfg(target_os = "linux")]
    const SUPPORTED_EVENT_COUNT: u64 = 12;
    #[cfg(target_os = "linux")]
    const SUPPORTED_BASENAME: &str = "projection.sqlite3";
    #[cfg(target_os = "linux")]
    const BAD_DEPLOYMENT_ID: [u8; 32] = [0xee; 32];
    #[cfg(target_os = "linux")]
    static NEXT_SUPPORTED_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct TestWriter<'connection> {
        connection: &'connection Connection,
    }

    impl ProjectionWriter for TestWriter<'_> {
        fn execute<P: Params>(
            &mut self,
            statement: ProjectionStatement,
            parameters: P,
        ) -> Result<usize, BackendError> {
            self.connection
                .execute(statement.sql(), parameters)
                .map_err(classify_runtime_error)
        }
    }

    #[cfg(target_os = "linux")]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum SupportedCorruption {
        None,
        DefinitionWrongKind,
        EdgeWrongKind,
        EdgeDeploymentMismatch,
        AssociationWrongKind,
        AssociationDeploymentMismatch,
    }

    #[cfg(target_os = "linux")]
    struct SupportedFixture {
        directory: PathBuf,
        basename: OsString,
        checkpoint: ProjectionCheckpoint,
        anchor: IntentUnitId,
        first: IntentUnitId,
        second: IntentUnitId,
        first_definition: RelationshipDefinitionKey,
        second_definition: RelationshipDefinitionKey,
        shared_reference: ExternalReference,
    }

    #[cfg(target_os = "linux")]
    impl SupportedFixture {
        fn create(corruption: SupportedCorruption) -> Option<Self> {
            let root = std::env::var_os("CUBIKAN_TEST_SUPPORTED_ROOT").map(PathBuf::from)?;
            let unique = NEXT_SUPPORTED_FIXTURE.fetch_add(1, AtomicOrdering::Relaxed);
            let directory = root.join(format!(
                "cubikan-t1109-relationships-{}-{unique}",
                std::process::id()
            ));
            let mut builder = fs::DirBuilder::new();
            builder.mode(0o700);
            builder
                .create(&directory)
                .expect("create owner-only relationship fixture directory");
            let basename = OsString::from(SUPPORTED_BASENAME);
            let mut writer = create_fresh_projection(&directory, &basename)
                .expect("create hardened relationship projection");
            writer.begin_projection().expect("begin fixture projection");

            let relay_hash = [0x31_u8; 32];
            let parachain_hash = [0x32_u8; 32];
            let deployment_id = [0x33_u8; 32];
            let runtime_hash = [0x34_u8; 32];
            let parent_hash = [0x35_u8; 32];
            let block_hash = [0x36_u8; 32];
            let signer = [0x37_u8; 32];
            let anchor = supported_unit(
                "00000000-0000-4000-8000-000000000001",
                "anchor",
                "anchor-flow",
            );
            let first = supported_unit(
                "00000000-0000-4000-8000-000000000002",
                "first",
                "incoming-flow",
            );
            let second = supported_unit(
                "00000000-0000-4000-8000-000000000003",
                "second",
                "outgoing-flow",
            );
            let first_core_definition = core_definition(1);
            let second_core_definition = core_definition(2);
            let first_relationship = CoreRelationshipIdentity::new(
                first_core_definition.key().clone(),
                anchor.id(),
                first.id(),
            );
            let first_relationship_lookahead = CoreRelationshipIdentity::new(
                first_core_definition.key().clone(),
                anchor.id(),
                second.id(),
            );
            let outgoing_relationship = CoreRelationshipIdentity::new(
                second_core_definition.key().clone(),
                anchor.id(),
                second.id(),
            );
            let incoming_relationship = CoreRelationshipIdentity::new(
                second_core_definition.key().clone(),
                first.id(),
                anchor.id(),
            );
            let shared_reference = reference("artifact", "shared");
            let other_reference = reference("artifact", "other");
            let associations = [
                RecordedAssociation::new(
                    anchor.id(),
                    AssociationSubject::Revision(0),
                    other_reference,
                ),
                RecordedAssociation::new(
                    anchor.id(),
                    AssociationSubject::WholeUnit,
                    shared_reference.clone(),
                ),
                RecordedAssociation::new(
                    first.id(),
                    AssociationSubject::Revision(0),
                    shared_reference.clone(),
                ),
            ];

            insert_anchor(
                &mut writer,
                ProjectionAnchor {
                    relay_genesis_hash: &relay_hash,
                    parachain_genesis_hash: &parachain_hash,
                    deployment_id: &deployment_id,
                    initial_runtime_spec_version: 17,
                    initial_runtime_code_hash: &runtime_hash,
                },
            )
            .expect("insert supported fixture anchor");
            insert_block(
                &mut writer,
                ProjectedBlock {
                    block_number: BLOCK_NUMBER,
                    block_hash: &block_hash,
                    parent_hash: &parent_hash,
                    runtime_spec_version: 17,
                    runtime_code_hash: &runtime_hash,
                    event_count: u32::try_from(SUPPORTED_EVENT_COUNT)
                        .expect("supported event count"),
                    first_global_sequence: Some(1),
                    last_global_sequence: Some(SUPPORTED_EVENT_COUNT),
                },
            )
            .expect("insert supported fixture block");

            let expected_kinds = [
                ProjectedEventKind::UnitCreated,
                ProjectedEventKind::UnitCreated,
                ProjectedEventKind::UnitCreated,
                ProjectedEventKind::RelationshipDefinitionCreated,
                ProjectedEventKind::RelationshipDefinitionCreated,
                ProjectedEventKind::RelationshipCreated,
                ProjectedEventKind::RelationshipCreated,
                ProjectedEventKind::RelationshipCreated,
                ProjectedEventKind::RelationshipCreated,
                ProjectedEventKind::AssociationRecorded,
                ProjectedEventKind::AssociationRecorded,
                ProjectedEventKind::AssociationRecorded,
            ];
            for (index, expected_kind) in expected_kinds.into_iter().enumerate() {
                let sequence = u64::try_from(index + 1).expect("supported sequence");
                let kind = match (corruption, sequence) {
                    (SupportedCorruption::DefinitionWrongKind, 4) => {
                        ProjectedEventKind::RelationshipCreated
                    }
                    (SupportedCorruption::EdgeWrongKind, 7) => {
                        ProjectedEventKind::AssociationRecorded
                    }
                    (SupportedCorruption::AssociationWrongKind, 11) => {
                        ProjectedEventKind::RelationshipCreated
                    }
                    _ => expected_kind,
                };
                let event_deployment = match (corruption, sequence) {
                    (SupportedCorruption::EdgeDeploymentMismatch, 7)
                    | (SupportedCorruption::AssociationDeploymentMismatch, 12) => {
                        &BAD_DEPLOYMENT_ID
                    }
                    _ => &deployment_id,
                };
                let extrinsic_hash =
                    [u8::try_from(sequence).expect("supported fixture sequence byte"); 32];
                insert_event(
                    &mut writer,
                    ProjectedEvent {
                        block_number: BLOCK_NUMBER,
                        extrinsic_index: u32::try_from(index).expect("supported extrinsic index"),
                        system_event_index: u32::try_from(index)
                            .expect("supported system event index"),
                        global_sequence: sequence,
                        deployment_id: event_deployment,
                        kind,
                        scale_payload: &[1],
                        signer: &signer,
                        extrinsic_hash: &extrinsic_hash,
                    },
                )
                .expect("insert supported fixture event");
            }
            insert_intent_unit(&mut writer, &anchor, 1).expect("insert supported anchor unit");
            insert_intent_unit(&mut writer, &first, 2).expect("insert supported first unit");
            insert_intent_unit(&mut writer, &second, 3).expect("insert supported second unit");
            insert_relationship_definition(&mut writer, &first_core_definition, 4)
                .expect("insert supported first definition");
            insert_relationship_definition(&mut writer, &second_core_definition, 5)
                .expect("insert supported second definition");
            for (relationship, sequence) in [
                (&first_relationship, 6),
                (&first_relationship_lookahead, 7),
                (&outgoing_relationship, 8),
                (&incoming_relationship, 9),
            ] {
                insert_relationship(&mut writer, relationship, sequence)
                    .expect("insert supported relationship");
            }
            for (association, sequence) in associations.iter().zip(10_u64..=12) {
                insert_association(&mut writer, association, sequence)
                    .expect("insert supported association");
            }
            insert_checkpoint(
                &mut writer,
                StoredCheckpoint {
                    block_number: BLOCK_NUMBER,
                    block_hash: &block_hash,
                    last_global_sequence: Some(SUPPORTED_EVENT_COUNT),
                    runtime_spec_version: 17,
                    runtime_code_hash: &runtime_hash,
                },
            )
            .expect("insert supported checkpoint");
            writer
                .commit_projection()
                .expect("commit supported fixture projection");
            drop(writer);

            Some(Self {
                directory,
                basename,
                checkpoint: ProjectionCheckpoint::new(
                    BLOCK_NUMBER,
                    block_hash,
                    NonZeroU64::new(SUPPORTED_EVENT_COUNT),
                    17,
                    runtime_hash,
                ),
                anchor: anchor.id(),
                first: first.id(),
                second: second.id(),
                first_definition: backend_definition(1),
                second_definition: backend_definition(2),
                shared_reference,
            })
        }

        fn snapshot(&self) -> VerifiedReadSnapshot {
            issue_test_snapshot(&self.directory, &self.basename, &self.checkpoint)
                .expect("issue supported private snapshot")
        }
    }

    #[cfg(target_os = "linux")]
    impl Drop for SupportedFixture {
        fn drop(&mut self) {
            for suffix in ["-journal", "-wal", "-shm"] {
                let mut sidecar = self.directory.join(&self.basename);
                let mut name = self.basename.clone();
                name.push(suffix);
                sidecar.set_file_name(name);
                let _ = fs::remove_file(sidecar);
            }
            let _ = fs::remove_file(self.directory.join(&self.basename));
            let _ = fs::remove_dir(&self.directory);
        }
    }

    struct Fixture {
        connection: Connection,
        checkpoint: ProjectionCheckpoint,
        anchor: IntentUnitId,
        first: IntentUnitId,
        second: IntentUnitId,
        first_definition: RelationshipDefinitionKey,
        second_definition: RelationshipDefinitionKey,
        shared_reference: ExternalReference,
    }

    #[test]
    fn test_query_semantics_preserve_versions_and_many_to_many_identity() {
        let fixture = fixture();
        let before = query::load_projected_unit(&fixture.connection, fixture.anchor)
            .expect("load lifecycle state before relationship/provenance reads");

        let first_page = crate::relationship::list_projected_relationships(
            &fixture.connection,
            ListRelationships::new(
                fixture.first_definition.clone(),
                Some(fixture.anchor),
                None,
                PageLimit::new(1).expect("limit"),
                None,
            )
            .expect("first-version query"),
            &fixture.checkpoint,
        )
        .expect("read first exact relationship version");
        assert_eq!(first_page.items().len(), 1);
        assert_eq!(first_page.items()[0].key().source(), fixture.anchor);
        assert_eq!(first_page.items()[0].key().target(), fixture.first);
        assert_eq!(
            first_page.items()[0].key().definition().version().value(),
            1
        );
        assert!(first_page.next_cursor().is_none());
        assert_eq!(first_page.checkpoint(), &fixture.checkpoint);

        let second_page = crate::relationship::list_projected_relationships(
            &fixture.connection,
            ListRelationships::new(
                fixture.second_definition.clone(),
                Some(fixture.anchor),
                None,
                PageLimit::new(1).expect("limit"),
                None,
            )
            .expect("second-version query"),
            &fixture.checkpoint,
        )
        .expect("read second exact relationship version");
        assert_eq!(second_page.items().len(), 1);
        assert_eq!(second_page.items()[0].key().target(), fixture.second);
        assert_eq!(
            second_page.items()[0].key().definition().version().value(),
            2
        );

        let projected = crate::relationship::project_relationships(
            &fixture.connection,
            ProjectionQueryV1::new(
                ListFilters::default(),
                Some(DirectRelationshipPredicate::Outgoing {
                    definition: fixture.second_definition.clone(),
                    anchor: fixture.anchor,
                }),
                PageLimit::new(1).expect("limit"),
                None,
            ),
            &fixture.checkpoint,
        )
        .expect("project exact second-version membership");
        assert_eq!(projected.query().version(), 1);
        assert_eq!(projected.items().len(), 1);
        assert_eq!(projected.items()[0].id(), fixture.second);
        assert_eq!(projected.checkpoint(), &fixture.checkpoint);

        let forward_first = list_by_unit(
            &fixture.connection,
            ListAssociationsByUnit::new(
                fixture.anchor,
                None,
                PageLimit::new(1).expect("limit"),
                None,
            )
            .expect("forward query"),
            &fixture.checkpoint,
        )
        .expect("first forward page");
        assert_eq!(forward_first.direction(), AssociationDirection::ByUnit);
        assert_eq!(forward_first.items().len(), 1);
        assert_eq!(
            forward_first.items()[0].key().subject(),
            AssociationSubject::Revision(0)
        );
        let forward_cursor = forward_first
            .next_cursor()
            .expect("forward lookahead produces cursor")
            .clone();
        let forward_second = list_by_unit(
            &fixture.connection,
            ListAssociationsByUnit::new(
                fixture.anchor,
                None,
                PageLimit::new(1).expect("limit"),
                Some(forward_cursor),
            )
            .expect("continued forward query"),
            &fixture.checkpoint,
        )
        .expect("second forward page");
        assert_eq!(forward_second.items().len(), 1);
        assert_eq!(
            forward_second.items()[0].key().subject(),
            AssociationSubject::WholeUnit
        );
        assert_eq!(
            forward_second.items()[0].key().reference(),
            &fixture.shared_reference
        );
        assert!(forward_second.next_cursor().is_none());

        let reverse_first = list_by_reference(
            &fixture.connection,
            ListAssociationsByReference::new(
                fixture.shared_reference.clone(),
                PageLimit::new(1).expect("limit"),
                None,
            )
            .expect("reverse query"),
            &fixture.checkpoint,
        )
        .expect("first reverse page");
        assert_eq!(reverse_first.direction(), AssociationDirection::ByReference);
        assert_eq!(reverse_first.items()[0].key().unit_id(), fixture.anchor);
        let reverse_second = list_by_reference(
            &fixture.connection,
            ListAssociationsByReference::new(
                fixture.shared_reference.clone(),
                PageLimit::new(1).expect("limit"),
                Some(
                    reverse_first
                        .next_cursor()
                        .expect("reverse lookahead produces cursor")
                        .clone(),
                ),
            )
            .expect("continued reverse query"),
            &fixture.checkpoint,
        )
        .expect("second reverse page");
        assert_eq!(reverse_second.items()[0].key().unit_id(), fixture.first);
        assert_eq!(
            reverse_second.items()[0].key().subject(),
            AssociationSubject::Revision(0)
        );
        assert!(reverse_second.next_cursor().is_none());

        for coordinate in [
            first_page.items()[0].created_coordinate(),
            second_page.items()[0].created_coordinate(),
            forward_first.items()[0].created_coordinate(),
            reverse_second.items()[0].created_coordinate(),
        ] {
            assert_eq!(coordinate.parachain_genesis_hash(), &[2_u8; 32]);
            assert_eq!(coordinate.deployment_id(), &[3_u8; 32]);
            assert_eq!(coordinate.block_number(), BLOCK_NUMBER);
            assert_eq!(coordinate.block_hash(), &[9_u8; 32]);
            query::validate_projected_coordinate(coordinate, &fixture.checkpoint)
                .expect("coordinate belongs to checkpoint C");
        }

        let after = query::load_projected_unit(&fixture.connection, fixture.anchor)
            .expect("load lifecycle state after relationship/provenance reads");
        assert_eq!(before, after);
        assert_eq!(after.intent_unit().revision().value(), 0);
        assert!(after.intent_unit().history().is_empty());

        #[cfg(target_os = "linux")]
        exercise_supported_snapshot_semantics();
    }

    #[cfg(target_os = "linux")]
    pub(super) fn exercise_supported_snapshot_query_matrix() {
        let Some(fixture) = SupportedFixture::create(SupportedCorruption::None) else {
            return;
        };
        assert_supported_snapshot_semantics(&fixture);
        drop(fixture);

        let definition_corrupt = SupportedFixture::create(SupportedCorruption::DefinitionWrongKind)
            .expect("supported root remains available");
        assert!(matches!(
            definition_corrupt
                .snapshot()
                .get_relationship_definition(definition_corrupt.first_definition.clone()),
            Err(ReadError::Backend(BackendError::ProjectionMismatch))
        ));

        let edge_kind_corrupt = SupportedFixture::create(SupportedCorruption::EdgeWrongKind)
            .expect("supported root remains available");
        assert!(matches!(
            edge_kind_corrupt.snapshot().list_relationships(
                ListRelationships::new(
                    edge_kind_corrupt.first_definition.clone(),
                    Some(edge_kind_corrupt.anchor),
                    None,
                    PageLimit::new(1).expect("limit"),
                    None,
                )
                .expect("corrupt edge query"),
            ),
            Err(ReadError::Backend(BackendError::ProjectionMismatch))
        ));

        let edge_deployment_corrupt =
            SupportedFixture::create(SupportedCorruption::EdgeDeploymentMismatch)
                .expect("supported root remains available");
        assert!(matches!(
            edge_deployment_corrupt
                .snapshot()
                .project_intent_units_v1(ProjectionQueryV1::new(
                    ListFilters::default(),
                    Some(DirectRelationshipPredicate::Outgoing {
                        definition: edge_deployment_corrupt.first_definition.clone(),
                        anchor: edge_deployment_corrupt.anchor,
                    }),
                    PageLimit::new(1).expect("limit"),
                    None,
                )),
            Err(ReadError::Backend(BackendError::ProjectionMismatch))
        ));

        let association_kind_corrupt =
            SupportedFixture::create(SupportedCorruption::AssociationWrongKind)
                .expect("supported root remains available");
        assert!(matches!(
            association_kind_corrupt
                .snapshot()
                .list_associations_by_unit(
                    ListAssociationsByUnit::new(
                        association_kind_corrupt.anchor,
                        None,
                        PageLimit::new(1).expect("limit"),
                        None,
                    )
                    .expect("corrupt forward query"),
                ),
            Err(ReadError::Backend(BackendError::ProjectionMismatch))
        ));

        let association_deployment_corrupt =
            SupportedFixture::create(SupportedCorruption::AssociationDeploymentMismatch)
                .expect("supported root remains available");
        assert!(matches!(
            association_deployment_corrupt
                .snapshot()
                .list_associations_by_reference(
                    ListAssociationsByReference::new(
                        association_deployment_corrupt.shared_reference.clone(),
                        PageLimit::new(1).expect("limit"),
                        None,
                    )
                    .expect("corrupt reverse query"),
                ),
            Err(ReadError::Backend(BackendError::ProjectionMismatch))
        ));
    }

    #[cfg(target_os = "linux")]
    pub(super) fn exercise_supported_snapshot_semantics() {
        let Some(fixture) = SupportedFixture::create(SupportedCorruption::None) else {
            return;
        };
        assert_supported_snapshot_semantics(&fixture);
    }

    #[cfg(target_os = "linux")]
    fn assert_supported_snapshot_semantics(fixture: &SupportedFixture) {
        let before = fixture
            .snapshot()
            .get_intent_unit(GetIntentUnit::new(fixture.anchor))
            .expect("read anchor before relationship queries");
        assert_eq!(before.checkpoint(), &fixture.checkpoint);

        let definition = fixture
            .snapshot()
            .get_relationship_definition(fixture.first_definition.clone())
            .expect("read exact definition through capability");
        assert_eq!(
            definition.definition().definition().key().version().value(),
            1
        );
        assert_eq!(definition.checkpoint(), &fixture.checkpoint);
        assert_supported_coordinate(
            definition.definition().created_coordinate(),
            &fixture.checkpoint,
            4,
        );

        let direct_first = fixture
            .snapshot()
            .list_relationships(
                ListRelationships::new(
                    fixture.first_definition.clone(),
                    Some(fixture.anchor),
                    None,
                    PageLimit::new(1).expect("limit"),
                    None,
                )
                .expect("first exact relationship query"),
            )
            .expect("read first exact relationship page through capability");
        assert_eq!(direct_first.items().len(), 1);
        assert_eq!(direct_first.items()[0].key().source(), fixture.anchor);
        assert_eq!(direct_first.items()[0].key().target(), fixture.first);
        assert_eq!(
            direct_first.items()[0].key().definition().version().value(),
            1
        );
        assert_eq!(direct_first.checkpoint(), &fixture.checkpoint);
        assert_supported_coordinate(
            direct_first.items()[0].created_coordinate(),
            &fixture.checkpoint,
            6,
        );
        let direct_cursor = direct_first
            .next_cursor()
            .expect("direct relationship lookahead cursor")
            .clone();
        let direct_second = fixture
            .snapshot()
            .list_relationships(
                ListRelationships::new(
                    fixture.first_definition.clone(),
                    Some(fixture.anchor),
                    None,
                    PageLimit::new(100).expect("limit"),
                    Some(direct_cursor),
                )
                .expect("continued exact relationship query"),
            )
            .expect("read continued exact relationship page through capability");
        assert_eq!(direct_second.items().len(), 1);
        assert_eq!(direct_second.items()[0].key().target(), fixture.second);
        assert!(direct_second.next_cursor().is_none());

        let wrong_definition_cursor = RelationshipCursor::new(RelationshipIdentity::new(
            fixture.second_definition.clone(),
            fixture.anchor,
            fixture.first,
        ));
        assert!(
            ListRelationships::new(
                fixture.first_definition.clone(),
                Some(fixture.anchor),
                None,
                PageLimit::new(1).expect("limit"),
                Some(wrong_definition_cursor),
            )
            .is_err()
        );

        let outgoing = fixture
            .snapshot()
            .project_intent_units_v1(ProjectionQueryV1::new(
                supported_filters("outgoing-flow"),
                Some(DirectRelationshipPredicate::Outgoing {
                    definition: fixture.second_definition.clone(),
                    anchor: fixture.anchor,
                }),
                PageLimit::new(1).expect("limit"),
                None,
            ))
            .expect("read filtered outgoing projection through capability");
        assert_eq!(outgoing.items().len(), 1);
        assert_eq!(outgoing.items()[0].id(), fixture.second);
        assert_eq!(outgoing.checkpoint(), &fixture.checkpoint);

        let incoming = fixture
            .snapshot()
            .project_intent_units_v1(ProjectionQueryV1::new(
                supported_filters("incoming-flow"),
                Some(DirectRelationshipPredicate::Incoming {
                    definition: fixture.second_definition.clone(),
                    anchor: fixture.anchor,
                }),
                PageLimit::new(1).expect("limit"),
                None,
            ))
            .expect("read filtered incoming projection through capability");
        assert_eq!(incoming.items().len(), 1);
        assert_eq!(incoming.items()[0].id(), fixture.first);
        assert_eq!(incoming.checkpoint(), &fixture.checkpoint);

        let excluded = fixture
            .snapshot()
            .project_intent_units_v1(ProjectionQueryV1::new(
                supported_filters("anchor-flow"),
                Some(DirectRelationshipPredicate::Outgoing {
                    definition: fixture.second_definition.clone(),
                    anchor: fixture.anchor,
                }),
                PageLimit::new(1).expect("limit"),
                None,
            ))
            .expect("AND lifecycle filters with outgoing predicate");
        assert!(excluded.items().is_empty());

        let forward_first = fixture
            .snapshot()
            .list_associations_by_unit(
                ListAssociationsByUnit::new(
                    fixture.anchor,
                    None,
                    PageLimit::new(1).expect("limit"),
                    None,
                )
                .expect("first forward query"),
            )
            .expect("read first forward page through capability");
        assert_eq!(forward_first.direction(), AssociationDirection::ByUnit);
        assert_eq!(
            forward_first.items()[0].key().subject(),
            AssociationSubject::Revision(0)
        );
        assert_eq!(forward_first.checkpoint(), &fixture.checkpoint);
        assert_supported_coordinate(
            forward_first.items()[0].created_coordinate(),
            &fixture.checkpoint,
            10,
        );
        let forward_cursor = forward_first
            .next_cursor()
            .expect("forward lookahead cursor")
            .clone();
        let forward_second = fixture
            .snapshot()
            .list_associations_by_unit(
                ListAssociationsByUnit::new(
                    fixture.anchor,
                    None,
                    PageLimit::new(100).expect("limit"),
                    Some(forward_cursor),
                )
                .expect("continued forward query"),
            )
            .expect("read continued forward page through capability");
        assert_eq!(forward_second.items().len(), 1);
        assert_eq!(
            forward_second.items()[0].key().subject(),
            AssociationSubject::WholeUnit
        );
        assert_eq!(
            forward_second.items()[0].key().reference(),
            &fixture.shared_reference
        );
        assert!(forward_second.next_cursor().is_none());

        let subject_cursor = RecordedAssociation::new(
            fixture.anchor,
            AssociationSubject::Revision(0),
            fixture.shared_reference.clone(),
        );
        assert!(
            ListAssociationsByUnit::new(
                fixture.anchor,
                Some(AssociationSubject::WholeUnit),
                PageLimit::new(1).expect("limit"),
                Some(subject_cursor),
            )
            .is_err()
        );

        let reverse_first = fixture
            .snapshot()
            .list_associations_by_reference(
                ListAssociationsByReference::new(
                    fixture.shared_reference.clone(),
                    PageLimit::new(1).expect("limit"),
                    None,
                )
                .expect("first reverse query"),
            )
            .expect("read first reverse page through capability");
        assert_eq!(reverse_first.direction(), AssociationDirection::ByReference);
        assert_eq!(reverse_first.items()[0].key().unit_id(), fixture.anchor);
        let reverse_cursor = reverse_first
            .next_cursor()
            .expect("reverse lookahead cursor")
            .clone();
        let reverse_second = fixture
            .snapshot()
            .list_associations_by_reference(
                ListAssociationsByReference::new(
                    fixture.shared_reference.clone(),
                    PageLimit::new(100).expect("limit"),
                    Some(reverse_cursor),
                )
                .expect("continued reverse query"),
            )
            .expect("read continued reverse page through capability");
        assert_eq!(reverse_second.items().len(), 1);
        assert_eq!(reverse_second.items()[0].key().unit_id(), fixture.first);
        assert_eq!(
            reverse_second.items()[0].key().subject(),
            AssociationSubject::Revision(0)
        );
        assert!(reverse_second.next_cursor().is_none());
        assert_eq!(reverse_second.checkpoint(), &fixture.checkpoint);

        let after = fixture
            .snapshot()
            .get_intent_unit(GetIntentUnit::new(fixture.anchor))
            .expect("read anchor after relationship queries");
        assert_eq!(before.intent_unit(), after.intent_unit());
        assert_eq!(after.intent_unit().intent_unit().revision().value(), 0);
        assert!(after.intent_unit().intent_unit().history().is_empty());
    }

    #[cfg(target_os = "linux")]
    fn supported_filters(workflow: &str) -> ListFilters {
        ListFilters::new(
            Some(WorkflowId::new(workflow).expect("supported workflow filter")),
            Some(IntentSpecies::new("work").expect("supported species filter")),
            Some(PhaseId::new("ready").expect("supported phase filter")),
            Some(IntentUnitStatus::Active),
        )
    }

    #[cfg(target_os = "linux")]
    fn assert_supported_coordinate(
        coordinate: &crate::LedgerCoordinate,
        checkpoint: &ProjectionCheckpoint,
        sequence: u64,
    ) {
        assert_eq!(coordinate.parachain_genesis_hash(), &[0x32; 32]);
        assert_eq!(coordinate.deployment_id(), &[0x33; 32]);
        assert_eq!(coordinate.block_number(), BLOCK_NUMBER);
        assert_eq!(coordinate.block_hash(), checkpoint.block_hash());
        assert_eq!(coordinate.global_sequence().get(), sequence);
        query::validate_projected_coordinate(coordinate, checkpoint)
            .expect("supported coordinate belongs to checkpoint");
    }

    fn fixture() -> Fixture {
        let connection = initialized_connection();
        let anchor = unit("00000000-0000-4000-8000-000000000001", "anchor");
        let first = unit("00000000-0000-4000-8000-000000000002", "first");
        let second = unit("00000000-0000-4000-8000-000000000003", "second");
        let first_core_definition = core_definition(1);
        let second_core_definition = core_definition(2);
        let first_relationship = CoreRelationshipIdentity::new(
            first_core_definition.key().clone(),
            anchor.id(),
            first.id(),
        );
        let second_relationship = CoreRelationshipIdentity::new(
            second_core_definition.key().clone(),
            anchor.id(),
            second.id(),
        );
        let shared_reference = reference("artifact", "shared");
        let other_reference = reference("artifact", "other");
        let associations = [
            RecordedAssociation::new(
                anchor.id(),
                AssociationSubject::WholeUnit,
                shared_reference.clone(),
            ),
            RecordedAssociation::new(
                first.id(),
                AssociationSubject::Revision(0),
                shared_reference.clone(),
            ),
            RecordedAssociation::new(
                anchor.id(),
                AssociationSubject::Revision(0),
                other_reference,
            ),
        ];

        let relay_hash = [1_u8; 32];
        let parachain_hash = [2_u8; 32];
        let deployment_id = [3_u8; 32];
        let runtime_hash = [4_u8; 32];
        let parent_hash = [8_u8; 32];
        let block_hash = [9_u8; 32];
        let signer = [10_u8; 32];
        let event_kinds = [
            ProjectedEventKind::UnitCreated,
            ProjectedEventKind::UnitCreated,
            ProjectedEventKind::UnitCreated,
            ProjectedEventKind::RelationshipDefinitionCreated,
            ProjectedEventKind::RelationshipDefinitionCreated,
            ProjectedEventKind::RelationshipCreated,
            ProjectedEventKind::RelationshipCreated,
            ProjectedEventKind::AssociationRecorded,
            ProjectedEventKind::AssociationRecorded,
            ProjectedEventKind::AssociationRecorded,
        ];
        let mut writer = TestWriter {
            connection: &connection,
        };
        insert_anchor(
            &mut writer,
            ProjectionAnchor {
                relay_genesis_hash: &relay_hash,
                parachain_genesis_hash: &parachain_hash,
                deployment_id: &deployment_id,
                initial_runtime_spec_version: 1,
                initial_runtime_code_hash: &runtime_hash,
            },
        )
        .expect("insert anchor");
        insert_block(
            &mut writer,
            ProjectedBlock {
                block_number: BLOCK_NUMBER,
                block_hash: &block_hash,
                parent_hash: &parent_hash,
                runtime_spec_version: 1,
                runtime_code_hash: &runtime_hash,
                event_count: u32::try_from(EVENT_COUNT).expect("event count"),
                first_global_sequence: Some(1),
                last_global_sequence: Some(EVENT_COUNT),
            },
        )
        .expect("insert block");
        for (index, kind) in event_kinds.into_iter().enumerate() {
            let sequence = u64::try_from(index + 1).expect("sequence");
            let extrinsic_hash = [u8::try_from(index + 1).expect("fixture byte"); 32];
            insert_event(
                &mut writer,
                ProjectedEvent {
                    block_number: BLOCK_NUMBER,
                    extrinsic_index: u32::try_from(index).expect("extrinsic index"),
                    system_event_index: u32::try_from(index).expect("event index"),
                    global_sequence: sequence,
                    deployment_id: &deployment_id,
                    kind,
                    scale_payload: &[1],
                    signer: &signer,
                    extrinsic_hash: &extrinsic_hash,
                },
            )
            .expect("insert event");
        }
        insert_intent_unit(&mut writer, &anchor, 1).expect("insert anchor unit");
        insert_intent_unit(&mut writer, &first, 2).expect("insert first unit");
        insert_intent_unit(&mut writer, &second, 3).expect("insert second unit");
        insert_relationship_definition(&mut writer, &first_core_definition, 4)
            .expect("insert first definition");
        insert_relationship_definition(&mut writer, &second_core_definition, 5)
            .expect("insert second definition");
        insert_relationship(&mut writer, &first_relationship, 6)
            .expect("insert first relationship");
        insert_relationship(&mut writer, &second_relationship, 7)
            .expect("insert second relationship");
        for (association, sequence) in associations.iter().zip(8_u64..=EVENT_COUNT) {
            insert_association(&mut writer, association, sequence).expect("insert association");
        }
        insert_checkpoint(
            &mut writer,
            StoredCheckpoint {
                block_number: BLOCK_NUMBER,
                block_hash: &block_hash,
                last_global_sequence: Some(EVENT_COUNT),
                runtime_spec_version: 1,
                runtime_code_hash: &runtime_hash,
            },
        )
        .expect("insert checkpoint");

        Fixture {
            connection,
            checkpoint: ProjectionCheckpoint::new(
                BLOCK_NUMBER,
                block_hash,
                NonZeroU64::new(EVENT_COUNT),
                1,
                runtime_hash,
            ),
            anchor: anchor.id(),
            first: first.id(),
            second: second.id(),
            first_definition: backend_definition(1),
            second_definition: backend_definition(2),
            shared_reference,
        }
    }

    fn initialized_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open fixture projection");
        connection
            .pragma_update(None, "foreign_keys", true)
            .expect("enable foreign keys");
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("begin schema transaction");
        crate::schema::initialize_v3(&transaction).expect("initialize schema v3");
        transaction.commit().expect("commit schema");
        connection
    }

    fn unit(id: &str, value: &str) -> IntentUnit {
        let ready = PhaseId::new("ready").expect("phase");
        IntentUnit::new(
            IntentUnitId::from_str(id).expect("fixture ID"),
            reference("intent", value),
            IntentSpecies::new("work").expect("species"),
            Workflow::new(
                WorkflowId::new("one-step").expect("workflow"),
                [ready.clone()],
                ready.clone(),
                [],
                [ready],
            )
            .expect("workflow topology"),
        )
    }

    #[cfg(target_os = "linux")]
    fn supported_unit(id: &str, value: &str, workflow: &str) -> IntentUnit {
        let ready = PhaseId::new("ready").expect("phase");
        IntentUnit::new(
            IntentUnitId::from_str(id).expect("supported fixture ID"),
            reference("intent", value),
            IntentSpecies::new("work").expect("species"),
            Workflow::new(
                WorkflowId::new(workflow).expect("supported workflow"),
                [ready.clone()],
                ready.clone(),
                [],
                [ready],
            )
            .expect("supported workflow topology"),
        )
    }

    fn reference(scope: &str, value: &str) -> ExternalReference {
        ExternalReference::new(
            ReferenceNamespace::new("fixture").expect("namespace"),
            ReferenceText::new(scope).expect("scope"),
            ReferenceText::new(value).expect("value"),
        )
    }

    fn core_definition(version: u64) -> RelationshipDefinition {
        RelationshipDefinition::new(
            CoreDefinitionKey::new(
                ReferenceNamespace::new("depends.on").expect("definition ID"),
                CoreDefinitionVersion::new(version).expect("definition version"),
            ),
            Some(IntentSpecies::new("work").expect("source species")),
            Some(IntentSpecies::new("work").expect("target species")),
            RelationshipPolicy::Reject,
            RelationshipPolicy::Reject,
        )
    }

    fn backend_definition(version: u64) -> RelationshipDefinitionKey {
        RelationshipDefinitionKey::new(
            RelationshipDefinitionId::new("depends.on").expect("definition ID"),
            RelationshipDefinitionVersion::new(version).expect("definition version"),
        )
    }
}
