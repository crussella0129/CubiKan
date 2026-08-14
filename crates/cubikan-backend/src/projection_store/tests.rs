use std::str::FromStr;

use cubikan_core::{
    AssociationSubject, ExternalReference, IntentSpecies, IntentUnit, IntentUnitId,
    IntentUnitRevision, PhaseId, RecordedAssociation, ReferenceNamespace, ReferenceText, Workflow,
    WorkflowId,
};
use rusqlite::{Connection, TransactionBehavior};

use super::*;

struct TestWriter<'connection> {
    connection: &'connection Connection,
    calls: usize,
}

impl ProjectionWriter for TestWriter<'_> {
    fn execute<P: Params>(
        &mut self,
        statement: ProjectionStatement,
        parameters: P,
    ) -> Result<usize, BackendError> {
        self.calls += 1;
        self.connection
            .execute(statement.sql(), parameters)
            .map_err(crate::sqlite::classify_runtime_error)
    }
}

fn initialized_connection() -> Connection {
    let mut connection = Connection::open_in_memory().expect("open fixture projection");
    connection
        .pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("begin creator transaction");
    crate::schema::initialize_v3(&transaction).expect("initialize schema v3");
    transaction.commit().expect("commit schema v3");
    connection
}

fn fixture_unit(scope: &str, value: &str, species: IntentSpecies) -> IntentUnit {
    let ready = PhaseId::new("ready").expect("phase");
    IntentUnit::new(
        IntentUnitId::from_str("00000000-0000-4000-8000-000000000001").expect("id"),
        ExternalReference::new(
            ReferenceNamespace::new("fixture").expect("namespace"),
            ReferenceText::new(scope).expect("scope"),
            ReferenceText::new(value).expect("value"),
        ),
        species,
        Workflow::new(
            WorkflowId::new("one-step").expect("workflow"),
            [ready.clone()],
            ready.clone(),
            [],
            [ready],
        )
        .expect("topology"),
    )
}

fn insert_chain_prefix(writer: &mut TestWriter<'_>) {
    let relay = [1_u8; 32];
    let parachain = [2_u8; 32];
    let deployment = [3_u8; 32];
    let runtime = [4_u8; 32];
    let parent = [5_u8; 32];
    let block_zero = [6_u8; 32];
    let block_one = [7_u8; 32];
    let signer = [8_u8; 32];
    let extrinsic_zero = [9_u8; 32];
    let extrinsic_one = [10_u8; 32];

    insert_anchor(
        writer,
        ProjectionAnchor {
            relay_genesis_hash: &relay,
            parachain_genesis_hash: &parachain,
            deployment_id: &deployment,
            initial_runtime_spec_version: 1,
            initial_runtime_code_hash: &runtime,
        },
    )
    .expect("insert anchor");
    insert_block(
        writer,
        ProjectedBlock {
            block_number: 0,
            block_hash: &block_zero,
            parent_hash: &parent,
            runtime_spec_version: 1,
            runtime_code_hash: &runtime,
            event_count: 1,
            first_global_sequence: Some(1),
            last_global_sequence: Some(1),
        },
    )
    .expect("insert block zero");
    insert_event(
        writer,
        ProjectedEvent {
            block_number: 0,
            extrinsic_index: 0,
            system_event_index: 0,
            global_sequence: 1,
            deployment_id: &deployment,
            kind: ProjectedEventKind::UnitCreated,
            scale_payload: &[1],
            signer: &signer,
            extrinsic_hash: &extrinsic_zero,
        },
    )
    .expect("insert unit event");
    insert_block(
        writer,
        ProjectedBlock {
            block_number: 1,
            block_hash: &block_one,
            parent_hash: &block_zero,
            runtime_spec_version: 1,
            runtime_code_hash: &runtime,
            event_count: 1,
            first_global_sequence: Some(2),
            last_global_sequence: Some(2),
        },
    )
    .expect("insert block one");
    insert_event(
        writer,
        ProjectedEvent {
            block_number: 1,
            extrinsic_index: 0,
            system_event_index: 0,
            global_sequence: 2,
            deployment_id: &deployment,
            kind: ProjectedEventKind::AssociationRecorded,
            scale_payload: &[2],
            signer: &signer,
            extrinsic_hash: &extrinsic_one,
        },
    )
    .expect("insert association event");
    insert_checkpoint(
        writer,
        ProjectionCheckpoint {
            block_number: 1,
            block_hash: &block_one,
            last_global_sequence: Some(2),
            runtime_spec_version: 1,
            runtime_code_hash: &runtime,
        },
    )
    .expect("insert checkpoint");
}

#[test]
fn test_sql_injection_shapes_are_bound_and_private_writers_stay_private() {
    let connection = initialized_connection();
    let origin_scope = "quote'\"; -- PRAGMA writable_schema=ON";
    let origin_value = "/* ATTACH */ load_extension('x'); \\ end";
    let association_scope = "'; DELETE FROM intent_units; --";
    let association_value = "PRAGMA trusted_schema=ON; ATTACH 'x' AS y";
    let unit = fixture_unit(
        origin_scope,
        origin_value,
        IntentSpecies::new("intent").expect("species"),
    );
    let association = RecordedAssociation::new(
        unit.id(),
        AssociationSubject::WholeUnit,
        ExternalReference::new(
            ReferenceNamespace::new("fixture.association").expect("namespace"),
            ReferenceText::new(association_scope).expect("scope"),
            ReferenceText::new(association_value).expect("value"),
        ),
    );

    let mut writer = TestWriter {
        connection: &connection,
        calls: 0,
    };
    insert_chain_prefix(&mut writer);
    insert_intent_unit(&mut writer, &unit, 1).expect("insert unit through binds");
    insert_association(&mut writer, &association, 2).expect("insert association through binds");

    let unit_text = connection
        .query_row(
            "SELECT origin_scope,origin_value FROM intent_units WHERE id=?1 COLLATE BINARY",
            [unit.id().to_string()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .expect("read exact unit origin");
    assert_eq!(
        unit_text,
        (origin_scope.to_owned(), origin_value.to_owned())
    );
    let association_text = connection
        .query_row(
            "SELECT scope,value FROM recorded_associations WHERE unit_id=?1 COLLATE BINARY",
            [unit.id().to_string()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .expect("read exact association reference");
    assert_eq!(
        association_text,
        (association_scope.to_owned(), association_value.to_owned())
    );
    crate::schema::validate_v3(&connection).expect("bound text cannot mutate schema");

    let calls_before_rejection = writer.calls;
    let overbound_species =
        IntentSpecies::new("s".repeat(257)).expect("legacy constructor remains permissive");
    let invalid_unit = fixture_unit("scope", "value", overbound_species);
    assert_eq!(
        insert_intent_unit(&mut writer, &invalid_unit, 2),
        Err(BackendError::CorruptEnvelope)
    );
    assert_eq!(writer.calls, calls_before_rejection);

    assert_eq!(
        update_intent_unit(&mut writer, &unit, IntentUnitRevision::new(u64::MAX), 2,),
        Err(BackendError::ProjectionMismatch)
    );
    assert_eq!(writer.calls, calls_before_rejection);

    for statement in ProjectionStatement::ALL {
        let sql = statement.sql();
        assert!(!sql.contains("--"));
        assert!(!sql.contains("/*"));
        assert!(!sql.contains("*/"));
        assert!(!sql.contains("PRAGMA"));
        assert!(!sql.contains("ATTACH"));
        assert!(!sql.contains("load_extension"));
        assert!(!sql.contains("count("));
    }
    let public_surface = include_str!("../lib.rs");
    assert!(!public_surface.contains("pub use projection_store"));
}

#[test]
fn projection_writers_reject_inconsistent_coordinates_before_sql() {
    let connection = initialized_connection();
    let mut writer = TestWriter {
        connection: &connection,
        calls: 0,
    };
    let hash = [1_u8; 32];
    assert_eq!(
        insert_block(
            &mut writer,
            ProjectedBlock {
                block_number: 0,
                block_hash: &hash,
                parent_hash: &hash,
                runtime_spec_version: 0,
                runtime_code_hash: &hash,
                event_count: 2,
                first_global_sequence: Some(1),
                last_global_sequence: Some(1),
            },
        ),
        Err(BackendError::ProjectionMismatch)
    );
    assert_eq!(writer.calls, 0);

    assert_eq!(
        insert_event(
            &mut writer,
            ProjectedEvent {
                block_number: 0,
                extrinsic_index: 0,
                system_event_index: 0,
                global_sequence: 0,
                deployment_id: &hash,
                kind: ProjectedEventKind::UnitCreated,
                scale_payload: &[1],
                signer: &hash,
                extrinsic_hash: &hash,
            },
        ),
        Err(BackendError::ProjectionMismatch)
    );
    assert_eq!(writer.calls, 0);
}
