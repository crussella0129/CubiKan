use std::{num::NonZeroU64, str};

use cubikan_core::{IntentUnitId, LifecycleRecord};
use rusqlite::{
    Connection, Row, params, params_from_iter,
    types::{Value, ValueRef},
};

use crate::{
    BackendError, GetIntentUnit, IntentUnitPage, IntentUnitSummary, LedgerCoordinate, ListCursor,
    ListIntentUnits, ProjectedProjectionPage, ProjectedUnit, ProjectedUnitPage,
    ProjectedUnitResult, ProjectedUnitSummary, ProjectionCheckpoint, ProjectionPage,
    ProjectionQueryV1, ReadError, VerifiedReadSnapshot,
    model::LedgerCoordinateParts,
    sqlite::{StoredRow, classify_runtime_error, status_projection},
    stored::{self, UnitProjection},
};

const GET_PROJECTED_UNIT_SQL: &str = "SELECT unit.id,unit.envelope_version,unit.envelope,unit.origin_namespace,unit.origin_scope,unit.origin_value,unit.workflow_id,unit.species,unit.phase,unit.status,unit.revision,unit.last_global_sequence,anchor.parachain_genesis_hash,event.deployment_id,event.block_number,block.block_hash,event.extrinsic_index,event.extrinsic_hash,event.system_event_index,event.global_sequence,event.event_kind FROM intent_units AS unit LEFT JOIN projected_events AS event ON event.global_sequence=unit.last_global_sequence LEFT JOIN projected_blocks AS block ON block.block_number=event.block_number LEFT JOIN projection_anchor AS anchor ON anchor.singleton=block.anchor_singleton AND anchor.deployment_id=event.deployment_id WHERE unit.id COLLATE BINARY=?1";
const LIST_PROJECTED_UNITS_SQL: &str = "SELECT unit.id,unit.envelope_version,unit.envelope,unit.origin_namespace,unit.origin_scope,unit.origin_value,unit.workflow_id,unit.species,unit.phase,unit.status,unit.revision,unit.last_global_sequence,anchor.parachain_genesis_hash,event.deployment_id,event.block_number,block.block_hash,event.extrinsic_index,event.extrinsic_hash,event.system_event_index,event.global_sequence,event.event_kind FROM intent_units AS unit LEFT JOIN projected_events AS event ON event.global_sequence=unit.last_global_sequence LEFT JOIN projected_blocks AS block ON block.block_number=event.block_number LEFT JOIN projection_anchor AS anchor ON anchor.singleton=block.anchor_singleton AND anchor.deployment_id=event.deployment_id WHERE (?1 IS NULL OR unit.workflow_id COLLATE BINARY=?1) AND (?2 IS NULL OR unit.species COLLATE BINARY=?2) AND (?3 IS NULL OR unit.phase COLLATE BINARY=?3) AND (?4 IS NULL OR unit.status COLLATE BINARY=?4) AND (?5 IS NULL OR unit.id COLLATE BINARY>?5) ORDER BY unit.id COLLATE BINARY ASC LIMIT ?6";

const UNIT_PROJECTION_OFFSET: usize = 0;
const COORDINATE_OFFSET: usize = 12;
const EVENT_KIND_OFFSET: usize = 20;

impl VerifiedReadSnapshot {
    /// Retrieves one replay-validated unit and its joined ledger coordinate.
    pub fn get_intent_unit(self, command: GetIntentUnit) -> Result<ProjectedUnitResult, ReadError> {
        self.consume(|connection, checkpoint| {
            let unit = load_projected_unit(connection, command.id()).map_err(ReadError::from)?;
            validate_projected_coordinate(unit.last_coordinate(), checkpoint)
                .map_err(ReadError::from)?;
            Ok(ProjectedUnitResult::new(unit, checkpoint.clone()))
        })
    }

    /// Lists one bounded, exclusive-cursor page from this pinned snapshot.
    pub fn list_intent_units(
        self,
        command: ListIntentUnits,
    ) -> Result<ProjectedUnitPage, ReadError> {
        self.consume(|connection, checkpoint| {
            list_projected_units(connection, &command, checkpoint).map_err(ReadError::from)
        })
    }

    /// Evaluates one version-1 lifecycle/relationship projection page.
    pub fn project_intent_units_v1(
        self,
        query: ProjectionQueryV1,
    ) -> Result<ProjectedProjectionPage, ReadError> {
        self.consume(|connection, checkpoint| {
            if query.predicate().is_some() {
                crate::relationship::project_relationships(connection, query, checkpoint)
            } else {
                project_lifecycle_verified(connection, query, checkpoint).map_err(ReadError::from)
            }
        })
    }
}

pub(crate) fn load_projected_unit(
    connection: &Connection,
    id: IntentUnitId,
) -> Result<ProjectedUnit, BackendError> {
    let mut statement = connection
        .prepare(GET_PROJECTED_UNIT_SQL)
        .map_err(classify_runtime_error)?;
    let mut rows = statement
        .query(params![id.to_string()])
        .map_err(classify_runtime_error)?;
    let Some(row) = rows.next().map_err(classify_runtime_error)? else {
        return Err(BackendError::IntentUnitNotFound { id });
    };
    let unit = decode_projected_unit(row, UNIT_PROJECTION_OFFSET)?;
    if unit.intent_unit().id() != id {
        return Err(BackendError::ProjectionMismatch);
    }
    if rows.next().map_err(classify_runtime_error)?.is_some() {
        return Err(BackendError::ProjectionMismatch);
    }
    Ok(unit)
}

pub(crate) fn list_projected_units(
    connection: &Connection,
    command: &ListIntentUnits,
    checkpoint: &ProjectionCheckpoint,
) -> Result<ProjectedUnitPage, BackendError> {
    let filters = command.filters();
    let workflow_id = filters.workflow_id().map(|value| value.as_str().to_owned());
    let species = filters.species().map(|value| value.as_str().to_owned());
    let phase = filters.phase().map(|value| value.as_str().to_owned());
    let status = filters
        .status()
        .map(|value| status_projection(value).to_owned());
    let after = command.after().map(|cursor| cursor.to_string());
    let limit = command.limit().value();
    let fetch_limit = limit
        .checked_add(1)
        .expect("validated page limit plus lookahead must fit usize");
    let fetch_limit_sql =
        i64::try_from(fetch_limit).expect("page limit plus lookahead must fit SQLite INTEGER");

    let mut statement = connection
        .prepare(LIST_PROJECTED_UNITS_SQL)
        .map_err(classify_runtime_error)?;
    let mut rows = statement
        .query(params![
            workflow_id,
            species,
            phase,
            status,
            after,
            fetch_limit_sql,
        ])
        .map_err(classify_runtime_error)?;

    let mut projected = Vec::with_capacity(fetch_limit);
    let mut previous_id = command.after().map(|cursor| cursor.to_string());
    while let Some(row) = rows.next().map_err(classify_runtime_error)? {
        let unit = decode_projected_unit(row, UNIT_PROJECTION_OFFSET)?;
        validate_projected_filters(&unit, command)?;
        validate_projected_coordinate(unit.last_coordinate(), checkpoint)?;
        let id = unit.intent_unit().id().to_string();
        if previous_id
            .as_ref()
            .is_some_and(|previous| previous.as_bytes() >= id.as_bytes())
        {
            return Err(BackendError::ProjectionMismatch);
        }
        previous_id = Some(id);
        projected.push(ProjectedUnitSummary::from_projected_unit(&unit));
    }

    let has_more = projected.len() > limit;
    projected.truncate(limit);
    let next_cursor = if has_more {
        projected
            .last()
            .map(|summary| ListCursor::from_id(summary.id()))
    } else {
        None
    };
    Ok(ProjectedUnitPage::new(
        projected,
        next_cursor,
        checkpoint.clone(),
    ))
}

pub(crate) fn project_lifecycle_verified(
    connection: &Connection,
    query: ProjectionQueryV1,
    checkpoint: &ProjectionCheckpoint,
) -> Result<ProjectedProjectionPage, BackendError> {
    if query.predicate().is_some() {
        return Err(BackendError::ProjectionMismatch);
    }
    let command = ListIntentUnits::new(query.filters().clone(), query.limit(), query.after());
    let page = list_projected_units(connection, &command, checkpoint)?;
    Ok(ProjectedProjectionPage::new(
        query,
        page.items().to_vec(),
        page.next_cursor(),
        checkpoint.clone(),
    ))
}

pub(crate) fn validate_projected_coordinate(
    coordinate: &LedgerCoordinate,
    checkpoint: &ProjectionCheckpoint,
) -> Result<(), BackendError> {
    let Some(last_global_sequence) = checkpoint.last_global_sequence() else {
        return Err(BackendError::ProjectionMismatch);
    };
    if coordinate.block_number() > checkpoint.block_number()
        || coordinate.global_sequence() > last_global_sequence
        || (coordinate.block_number() == checkpoint.block_number()
            && coordinate.block_hash() != checkpoint.block_hash())
    {
        return Err(BackendError::ProjectionMismatch);
    }
    Ok(())
}

fn decode_projected_unit(row: &Row<'_>, offset: usize) -> Result<ProjectedUnit, BackendError> {
    let id = decode_text(row, offset)?;
    let envelope_version = decode_integer(row, offset + 1)?;
    let envelope = decode_text(row, offset + 2)?;
    let origin_namespace = decode_text(row, offset + 3)?;
    let origin_scope = decode_text(row, offset + 4)?;
    let origin_value = decode_text(row, offset + 5)?;
    let workflow_id = decode_text(row, offset + 6)?;
    let species = decode_text(row, offset + 7)?;
    let phase = decode_text(row, offset + 8)?;
    let status = decode_text(row, offset + 9)?;
    let revision = decode_blob(row, offset + 10)?;
    let last_global_sequence = decode_blob(row, offset + 11)?;
    let coordinate = decode_ledger_coordinate(row, offset + COORDINATE_OFFSET)?;
    let event_kind = decode_text(row, offset + EVENT_KIND_OFFSET)?;
    let accepted_global_sequence = coordinate.global_sequence().get();

    let projection = UnitProjection {
        id: &id,
        envelope_version,
        origin_namespace: &origin_namespace,
        origin_scope: &origin_scope,
        origin_value: &origin_value,
        workflow_id: &workflow_id,
        species: &species,
        phase: &phase,
        status: &status,
        revision: &revision,
        last_global_sequence: &last_global_sequence,
        accepted_global_sequence,
    };
    let unit = stored::decode_projected_envelope(envelope.as_bytes(), &projection)?;
    let expected_event_kind = match unit.history().last() {
        None => "unit_created",
        Some(LifecycleRecord::Transition(_)) => "unit_transitioned",
        Some(LifecycleRecord::Completion(_)) => "unit_completed",
    };
    if event_kind != expected_event_kind {
        return Err(BackendError::ProjectionMismatch);
    }

    Ok(ProjectedUnit::new(
        crate::IntentUnitView::from_intent_unit(&unit),
        coordinate,
    ))
}

/// Decodes the exact eight-column ledger-coordinate selection shared by all
/// verified projection queries.
pub(crate) fn decode_ledger_coordinate(
    row: &Row<'_>,
    offset: usize,
) -> Result<LedgerCoordinate, BackendError> {
    let parachain_genesis_hash = decode_hash(row, offset)?;
    let deployment_id = decode_hash(row, offset + 1)?;
    let block_number = stored::decode_u64_blob(&decode_blob(row, offset + 2)?)
        .map_err(|_| BackendError::ProjectionMismatch)?;
    let block_hash = decode_hash(row, offset + 3)?;
    let extrinsic_index = decode_u32(row, offset + 4)?;
    let extrinsic_hash = decode_hash(row, offset + 5)?;
    let system_event_index = decode_u32(row, offset + 6)?;
    let global_sequence = stored::decode_u64_blob(&decode_blob(row, offset + 7)?)
        .map_err(|_| BackendError::ProjectionMismatch)?;
    let global_sequence =
        NonZeroU64::new(global_sequence).ok_or(BackendError::ProjectionMismatch)?;
    Ok(LedgerCoordinate::from_parts(LedgerCoordinateParts {
        parachain_genesis_hash,
        deployment_id,
        block_number,
        block_hash,
        extrinsic_index,
        extrinsic_hash,
        system_event_index,
        global_sequence,
    }))
}

pub(crate) fn validate_projected_event_binding(
    row: &Row<'_>,
    offset: usize,
    expected_kind: &str,
) -> Result<(), BackendError> {
    if !matches!(
        row.get_ref(offset).map_err(classify_runtime_error)?,
        ValueRef::Integer(1)
    ) || !matches!(
        row.get_ref(offset + 1).map_err(classify_runtime_error)?,
        ValueRef::Text(kind) if kind == expected_kind.as_bytes()
    ) {
        return Err(BackendError::ProjectionMismatch);
    }
    Ok(())
}

fn validate_projected_filters(
    unit: &ProjectedUnit,
    command: &ListIntentUnits,
) -> Result<(), BackendError> {
    let view = unit.intent_unit();
    let filters = command.filters();
    if filters
        .workflow_id()
        .is_some_and(|expected| expected != view.workflow_id())
        || filters
            .species()
            .is_some_and(|expected| expected != view.species())
        || filters
            .phase()
            .is_some_and(|expected| expected != view.phase())
        || filters
            .status()
            .is_some_and(|expected| expected != view.status())
    {
        return Err(BackendError::ProjectionMismatch);
    }
    Ok(())
}

fn decode_text(row: &Row<'_>, index: usize) -> Result<String, BackendError> {
    match row.get_ref(index).map_err(classify_runtime_error)? {
        ValueRef::Text(bytes) => str::from_utf8(bytes)
            .map(|value| value.to_owned())
            .map_err(|_| BackendError::ProjectionMismatch),
        _ => Err(BackendError::ProjectionMismatch),
    }
}

fn decode_blob(row: &Row<'_>, index: usize) -> Result<Vec<u8>, BackendError> {
    match row.get_ref(index).map_err(classify_runtime_error)? {
        ValueRef::Blob(bytes) => Ok(bytes.to_vec()),
        _ => Err(BackendError::ProjectionMismatch),
    }
}

fn decode_integer(row: &Row<'_>, index: usize) -> Result<i64, BackendError> {
    match row.get_ref(index).map_err(classify_runtime_error)? {
        ValueRef::Integer(value) => Ok(value),
        _ => Err(BackendError::ProjectionMismatch),
    }
}

fn decode_u32(row: &Row<'_>, index: usize) -> Result<u32, BackendError> {
    u32::try_from(decode_integer(row, index)?).map_err(|_| BackendError::ProjectionMismatch)
}

fn decode_hash(row: &Row<'_>, index: usize) -> Result<[u8; 32], BackendError> {
    decode_blob(row, index)?
        .try_into()
        .map_err(|_| BackendError::ProjectionMismatch)
}

const SELECT_ROW: &str = "SELECT id, envelope_version, envelope, workflow_id, species, phase, status, revision \
     FROM intent_units";
const WORKFLOW_FILTER: &str = "workflow_id COLLATE BINARY = ?";
const SPECIES_FILTER: &str = "species COLLATE BINARY = ?";
const PHASE_FILTER: &str = "phase COLLATE BINARY = ?";
const STATUS_FILTER: &str = "status COLLATE BINARY = ?";
const CURSOR_FILTER: &str = "id COLLATE BINARY > ?";
const ORDER_AND_LIMIT: &str = " ORDER BY id COLLATE BINARY ASC LIMIT ?";

pub(crate) fn list(
    connection: &Connection,
    command: &ListIntentUnits,
) -> Result<IntentUnitPage, BackendError> {
    let mut predicates = Vec::with_capacity(5);
    let mut values = Vec::with_capacity(6);
    let filters = command.filters();

    if let Some(workflow_id) = filters.workflow_id() {
        predicates.push(WORKFLOW_FILTER);
        values.push(Value::Text(workflow_id.as_str().to_owned()));
    }
    if let Some(species) = filters.species() {
        predicates.push(SPECIES_FILTER);
        values.push(Value::Text(species.as_str().to_owned()));
    }
    if let Some(phase) = filters.phase() {
        predicates.push(PHASE_FILTER);
        values.push(Value::Text(phase.as_str().to_owned()));
    }
    if let Some(status) = filters.status() {
        predicates.push(STATUS_FILTER);
        values.push(Value::Text(status_projection(status).to_owned()));
    }
    if let Some(after) = command.after() {
        predicates.push(CURSOR_FILTER);
        values.push(Value::Text(after.to_string()));
    }

    let mut sql = String::from(SELECT_ROW);
    if !predicates.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&predicates.join(" AND "));
    }
    sql.push_str(ORDER_AND_LIMIT);

    let limit = command.limit().value();
    let fetch_limit = limit
        .checked_add(1)
        .expect("validated page limit plus lookahead must fit usize");
    values.push(Value::Integer(
        i64::try_from(fetch_limit).expect("page limit plus lookahead must fit SQLite INTEGER"),
    ));

    let mut statement = connection.prepare(&sql).map_err(classify_runtime_error)?;
    let candidates = statement
        .query_map(params_from_iter(values.iter()), StoredRow::from_row)
        .map_err(classify_runtime_error)?;
    let mut summaries = Vec::with_capacity(fetch_limit);
    for candidate in candidates {
        let unit = candidate
            .map_err(classify_runtime_error)?
            .into_validated_unit()?;
        summaries.push(IntentUnitSummary::from_intent_unit(&unit));
    }

    let has_more = summaries.len() > limit;
    summaries.truncate(limit);
    let next_cursor = if has_more {
        summaries
            .last()
            .map(|summary| ListCursor::from_id(summary.id()))
    } else {
        None
    };
    Ok(IntentUnitPage::new(summaries, next_cursor))
}

/// Evaluates projection v1 when no relationship membership predicate is
/// present. Reusing the lifecycle query keeps its filtering, replay, ordering,
/// cursor, and live-page semantics identical while retaining the versioned
/// projection query in the result.
pub(crate) fn project_lifecycle(
    connection: &Connection,
    query: ProjectionQueryV1,
) -> Result<ProjectionPage, BackendError> {
    let command = ListIntentUnits::new(query.filters().clone(), query.limit(), query.after());
    let page = list(connection, &command)?;
    Ok(ProjectionPage::new(
        query,
        page.items().to_vec(),
        page.next_cursor(),
    ))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::{
        ffi::{OsStr, OsString},
        fs,
        num::NonZeroU64,
        os::unix::fs::DirBuilderExt,
        path::{Path, PathBuf},
        str::FromStr,
        sync::{
            atomic::{AtomicU64, Ordering},
            mpsc,
        },
        thread,
        time::Duration,
    };

    use cubikan_core::{
        ExternalReference, IntentSpecies, IntentUnit, IntentUnitId, PhaseId, ReferenceNamespace,
        ReferenceText, Workflow, WorkflowId,
    };

    use super::*;
    use crate::{
        ListFilters, PageLimit,
        projection_store::{
            self, ProjectedBlock, ProjectedEvent, ProjectedEventKind, ProjectionAnchor,
            ProjectionCheckpoint as StoredCheckpoint,
        },
        sqlite::{ProjectionWriterConnection, create_fresh_projection, open_projection_writer},
        verified_read::issue_test_snapshot,
    };

    const BASENAME: &str = "projection.sqlite3";
    const BLOCK_C: u64 = 7;
    const BLOCK_C_PLUS_ONE: u64 = 8;
    const RUNTIME_SPEC_VERSION: u32 = 11;
    const RELAY_GENESIS_HASH: [u8; 32] = [0x10; 32];
    const PARACHAIN_GENESIS_HASH: [u8; 32] = [0x11; 32];
    const DEPLOYMENT_ID: [u8; 32] = [0x12; 32];
    const RUNTIME_CODE_HASH: [u8; 32] = [0x13; 32];
    const BLOCK_HASH_C: [u8; 32] = [0x14; 32];
    const BLOCK_HASH_C_PLUS_ONE: [u8; 32] = [0x15; 32];
    const BLOCK_PARENT_HASH: [u8; 32] = [0x16; 32];
    const SIGNER: [u8; 32] = [0x17; 32];
    const FIXTURE_IDS: [&str; 3] = [
        "00000000-0000-0000-0000-000000000010",
        "00000000-0000-0000-0000-000000000001",
        "00000000-0000-0000-0000-000000000002",
    ];
    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        directory: PathBuf,
        basename: OsString,
        checkpoint: ProjectionCheckpoint,
    }

    impl Fixture {
        fn create(event_kinds: &[ProjectedEventKind], checkpoint_sequence: u64) -> Option<Self> {
            let root = std::env::var_os("CUBIKAN_TEST_SUPPORTED_ROOT").map(PathBuf::from)?;
            let unique = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let directory = root.join(format!(
                "cubikan-t1109-query-{}-{unique}",
                std::process::id()
            ));
            let mut builder = fs::DirBuilder::new();
            builder.mode(0o700);
            builder
                .create(&directory)
                .expect("create owner-only query fixture directory");
            let basename = OsString::from(BASENAME);
            let mut writer = create_fresh_projection(&directory, &basename)
                .expect("create hardened query projection");
            writer.begin_projection().expect("begin fixture projection");
            projection_store::insert_anchor(
                &mut writer,
                ProjectionAnchor {
                    relay_genesis_hash: &RELAY_GENESIS_HASH,
                    parachain_genesis_hash: &PARACHAIN_GENESIS_HASH,
                    deployment_id: &DEPLOYMENT_ID,
                    initial_runtime_spec_version: RUNTIME_SPEC_VERSION,
                    initial_runtime_code_hash: &RUNTIME_CODE_HASH,
                },
            )
            .expect("insert fixture anchor");
            let event_count = u32::try_from(event_kinds.len()).expect("bounded fixture events");
            projection_store::insert_block(
                &mut writer,
                ProjectedBlock {
                    block_number: BLOCK_C,
                    block_hash: &BLOCK_HASH_C,
                    parent_hash: &BLOCK_PARENT_HASH,
                    runtime_spec_version: RUNTIME_SPEC_VERSION,
                    runtime_code_hash: &RUNTIME_CODE_HASH,
                    event_count,
                    first_global_sequence: Some(1),
                    last_global_sequence: Some(u64::from(event_count)),
                },
            )
            .expect("insert fixture block");
            for (offset, kind) in event_kinds.iter().copied().enumerate() {
                let sequence = u64::try_from(offset + 1).expect("fixture sequence fits u64");
                let extrinsic_index = u32::try_from(offset).expect("fixture index fits u32");
                let extrinsic_hash = [0x20_u8
                    .checked_add(u8::try_from(sequence).expect("small fixture sequence"))
                    .expect("small fixture hash marker"); 32];
                projection_store::insert_event(
                    &mut writer,
                    ProjectedEvent {
                        block_number: BLOCK_C,
                        extrinsic_index,
                        system_event_index: extrinsic_index,
                        global_sequence: sequence,
                        deployment_id: &DEPLOYMENT_ID,
                        kind,
                        scale_payload: &[1],
                        signer: &SIGNER,
                        extrinsic_hash: &extrinsic_hash,
                    },
                )
                .expect("insert fixture event");
                projection_store::insert_intent_unit(
                    &mut writer,
                    &unit(FIXTURE_IDS[offset]),
                    sequence,
                )
                .expect("insert fixture unit");
            }
            projection_store::insert_checkpoint(
                &mut writer,
                StoredCheckpoint {
                    block_number: BLOCK_C,
                    block_hash: &BLOCK_HASH_C,
                    last_global_sequence: Some(checkpoint_sequence),
                    runtime_spec_version: RUNTIME_SPEC_VERSION,
                    runtime_code_hash: &RUNTIME_CODE_HASH,
                },
            )
            .expect("insert fixture checkpoint");
            writer
                .commit_projection()
                .expect("commit fixture projection");
            drop(writer);

            Some(Self {
                directory,
                basename,
                checkpoint: checkpoint(BLOCK_C, BLOCK_HASH_C, checkpoint_sequence),
            })
        }

        fn snapshot(&self, candidate: &ProjectionCheckpoint) -> VerifiedReadSnapshot {
            issue_test_snapshot(&self.directory, &self.basename, candidate)
                .expect("issue private test snapshot")
        }
    }

    impl Drop for Fixture {
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

    #[test]
    fn test_private_snapshot_queries_are_ordered_bounded_and_fail_closed() {
        let Some(fixture) = Fixture::create(
            &[
                ProjectedEventKind::UnitCreated,
                ProjectedEventKind::UnitCreated,
                ProjectedEventKind::UnitCreated,
            ],
            3,
        ) else {
            return;
        };
        assert_eq!(PageLimit::new(0).expect_err("zero rejects").value(), 0);
        assert_eq!(PageLimit::new(1).expect("one accepts").value(), 1);
        assert_eq!(PageLimit::new(100).expect("100 accepts").value(), 100);
        assert_eq!(PageLimit::new(101).expect_err("101 rejects").value(), 101);

        let one = fixture
            .snapshot(&fixture.checkpoint)
            .list_intent_units(list_command(1, None))
            .expect("read one-item page with decoded lookahead");
        assert_eq!(one.items().len(), 1);
        assert_eq!(one.items()[0].id().to_string(), FIXTURE_IDS[1]);
        assert_eq!(
            one.next_cursor().expect("one-item page has lookahead").id(),
            id(FIXTURE_IDS[1])
        );

        let first = fixture
            .snapshot(&fixture.checkpoint)
            .list_intent_units(list_command(2, None))
            .expect("read first ordered page");
        assert_eq!(
            first
                .items()
                .iter()
                .map(|item| item.id().to_string())
                .collect::<Vec<_>>(),
            vec![FIXTURE_IDS[1].to_owned(), FIXTURE_IDS[2].to_owned()]
        );
        assert_eq!(first.checkpoint(), &fixture.checkpoint);
        let cursor = first.next_cursor().expect("lookahead produces cursor");
        assert_eq!(cursor.id().to_string(), FIXTURE_IDS[2]);
        assert_eq!(first.items()[0].last_coordinate().block_number(), BLOCK_C);
        assert_eq!(
            first.items()[0].last_coordinate().block_hash(),
            &BLOCK_HASH_C
        );
        assert_eq!(
            first.items()[0].last_coordinate().parachain_genesis_hash(),
            &PARACHAIN_GENESIS_HASH
        );
        assert_eq!(
            first.items()[0].last_coordinate().deployment_id(),
            &DEPLOYMENT_ID
        );

        let second = fixture
            .snapshot(&fixture.checkpoint)
            .list_intent_units(list_command(100, Some(cursor)))
            .expect("read exclusive second page");
        assert_eq!(second.items().len(), 1);
        assert_eq!(second.items()[0].id().to_string(), FIXTURE_IDS[0]);
        assert_eq!(second.next_cursor(), None);

        let selected = fixture
            .snapshot(&fixture.checkpoint)
            .get_intent_unit(GetIntentUnit::new(id(FIXTURE_IDS[2])))
            .expect("get projected unit");
        assert_eq!(
            selected.intent_unit().intent_unit().id(),
            id(FIXTURE_IDS[2])
        );
        assert_eq!(selected.checkpoint(), &fixture.checkpoint);

        let projected = fixture
            .snapshot(&fixture.checkpoint)
            .project_intent_units_v1(ProjectionQueryV1::new(
                ListFilters::default(),
                None,
                PageLimit::new(2).expect("valid projection limit"),
                None,
            ))
            .expect("read lifecycle projection");
        assert_eq!(projected.query().version(), ProjectionQueryV1::VERSION);
        assert_eq!(projected.items(), first.items());
        assert_eq!(projected.next_cursor(), first.next_cursor());
        assert_eq!(projected.checkpoint(), &fixture.checkpoint);

        let missing = id("00000000-0000-0000-0000-000000000099");
        assert!(matches!(
            fixture
                .snapshot(&fixture.checkpoint)
                .get_intent_unit(GetIntentUnit::new(missing)),
            Err(ReadError::Backend(BackendError::IntentUnitNotFound { id })) if id == missing
        ));

        let Some(corrupt_lookahead) = Fixture::create(
            &[
                ProjectedEventKind::UnitCreated,
                ProjectedEventKind::UnitCreated,
                ProjectedEventKind::RelationshipCreated,
            ],
            3,
        ) else {
            return;
        };
        assert!(matches!(
            corrupt_lookahead
                .snapshot(&corrupt_lookahead.checkpoint)
                .list_intent_units(list_command(2, None)),
            Err(ReadError::Backend(BackendError::ProjectionMismatch))
        ));

        let Some(beyond_checkpoint) = Fixture::create(
            &[
                ProjectedEventKind::UnitCreated,
                ProjectedEventKind::UnitCreated,
                ProjectedEventKind::UnitCreated,
            ],
            2,
        ) else {
            return;
        };
        assert!(matches!(
            beyond_checkpoint
                .snapshot(&beyond_checkpoint.checkpoint)
                .list_intent_units(list_command(2, None)),
            Err(ReadError::Backend(BackendError::ProjectionMismatch))
        ));

        crate::provenance::exercise_supported_snapshot_query_matrix();
    }

    #[test]
    fn test_delete_snapshot_pins_c_blocks_c_plus_one_then_refreshes() {
        let Some(fixture) = Fixture::create(&[ProjectedEventKind::UnitCreated], 1) else {
            return;
        };
        let checkpoint_c_plus_one = checkpoint(BLOCK_C_PLUS_ONE, BLOCK_HASH_C_PLUS_ONE, 2);
        assert_eq!(
            issue_test_snapshot(
                &fixture.directory,
                &fixture.basename,
                &checkpoint_c_plus_one,
            )
            .expect_err("pre-pin checkpoint mismatch must refresh"),
            ReadError::RefreshRequired
        );

        let pending_writer = open_projection_writer(&fixture.directory, &fixture.basename)
            .expect("open C+1 writer before pinning C");
        let snapshot_c = fixture.snapshot(&fixture.checkpoint);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let (result_sender, result_receiver) = mpsc::sync_channel(1);
        let writer = thread::spawn(move || {
            let mut pending_writer = pending_writer;
            let result =
                advance_open_writer_to_c_plus_one(&mut pending_writer, Some(&ready_sender));
            result_sender
                .send(result)
                .expect("send bounded writer result");
        });
        ready_receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("writer reaches commit while C is pinned");
        let busy = result_receiver
            .recv_timeout(Duration::from_millis(7_500))
            .expect("SQLite busy handler remains inside outer timeout");
        assert!(matches!(busy, Err(BackendError::StorageBusy(_))));
        writer.join().expect("join bounded writer");

        let page_c = snapshot_c
            .list_intent_units(list_command(100, None))
            .expect("pinned page finishes entirely at C");
        assert_eq!(page_c.items().len(), 1);
        assert_eq!(page_c.checkpoint(), &fixture.checkpoint);

        advance_to_c_plus_one(&fixture.directory, &fixture.basename, None)
            .expect("writer advances after snapshot drop");
        let page_c_plus_one = fixture
            .snapshot(&checkpoint_c_plus_one)
            .list_intent_units(list_command(100, None))
            .expect("new snapshot sees C+1 membership");
        assert_eq!(page_c_plus_one.items().len(), 2);
        assert_eq!(page_c_plus_one.checkpoint(), &checkpoint_c_plus_one);
    }

    fn advance_to_c_plus_one(
        directory: &Path,
        basename: &OsStr,
        ready: Option<&mpsc::SyncSender<()>>,
    ) -> Result<(), BackendError> {
        let mut writer = open_projection_writer(directory, basename)?;
        advance_open_writer_to_c_plus_one(&mut writer, ready)
    }

    fn advance_open_writer_to_c_plus_one(
        writer: &mut ProjectionWriterConnection,
        ready: Option<&mpsc::SyncSender<()>>,
    ) -> Result<(), BackendError> {
        writer.begin_projection()?;
        projection_store::insert_block(
            writer,
            ProjectedBlock {
                block_number: BLOCK_C_PLUS_ONE,
                block_hash: &BLOCK_HASH_C_PLUS_ONE,
                parent_hash: &BLOCK_HASH_C,
                runtime_spec_version: RUNTIME_SPEC_VERSION,
                runtime_code_hash: &RUNTIME_CODE_HASH,
                event_count: 1,
                first_global_sequence: Some(2),
                last_global_sequence: Some(2),
            },
        )?;
        let extrinsic_hash = [0x30; 32];
        projection_store::insert_event(
            writer,
            ProjectedEvent {
                block_number: BLOCK_C_PLUS_ONE,
                extrinsic_index: 0,
                system_event_index: 0,
                global_sequence: 2,
                deployment_id: &DEPLOYMENT_ID,
                kind: ProjectedEventKind::UnitCreated,
                scale_payload: &[2],
                signer: &SIGNER,
                extrinsic_hash: &extrinsic_hash,
            },
        )?;
        projection_store::insert_intent_unit(writer, &unit(FIXTURE_IDS[1]), 2)?;
        projection_store::update_checkpoint(
            writer,
            StoredCheckpoint {
                block_number: BLOCK_C_PLUS_ONE,
                block_hash: &BLOCK_HASH_C_PLUS_ONE,
                last_global_sequence: Some(2),
                runtime_spec_version: RUNTIME_SPEC_VERSION,
                runtime_code_hash: &RUNTIME_CODE_HASH,
            },
            BLOCK_C,
            &BLOCK_HASH_C,
        )?;
        if let Some(ready) = ready {
            ready.send(()).expect("signal writer before commit");
        }
        writer.commit_projection()
    }

    fn list_command(limit: usize, after: Option<ListCursor>) -> ListIntentUnits {
        ListIntentUnits::new(
            ListFilters::default(),
            PageLimit::new(limit).expect("fixture page limit"),
            after,
        )
    }

    fn checkpoint(
        block_number: u64,
        block_hash: [u8; 32],
        last_global_sequence: u64,
    ) -> ProjectionCheckpoint {
        ProjectionCheckpoint::new(
            block_number,
            block_hash,
            NonZeroU64::new(last_global_sequence),
            RUNTIME_SPEC_VERSION,
            RUNTIME_CODE_HASH,
        )
    }

    fn id(value: &str) -> IntentUnitId {
        IntentUnitId::from_str(value).expect("canonical fixture ID")
    }

    fn unit(value: &str) -> IntentUnit {
        let phase = PhaseId::new("queued").expect("fixture phase");
        IntentUnit::new(
            id(value),
            ExternalReference::new(
                ReferenceNamespace::new("book.intent").expect("fixture namespace"),
                ReferenceText::new("sprint-11").expect("fixture scope"),
                ReferenceText::new(value).expect("fixture reference value"),
            ),
            IntentSpecies::new("feature").expect("fixture species"),
            Workflow::new(
                WorkflowId::new("delivery").expect("fixture workflow"),
                [phase.clone()],
                phase,
                [],
                [],
            )
            .expect("fixture workflow topology"),
        )
    }
}
