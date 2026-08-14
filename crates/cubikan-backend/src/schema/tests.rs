use rusqlite::{Connection, TransactionBehavior};

use super::*;

fn initialized_connection() -> Connection {
    let mut connection = Connection::open_in_memory().expect("open in-memory projection");
    connection
        .pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("begin creator transaction");
    initialize_v3(&transaction).expect("initialize exact schema v3");
    transaction.commit().expect("commit exact schema v3");
    connection
}

fn validation_statement_order() -> Vec<SchemaStatement> {
    let mut statements = vec![
        SchemaStatement::ReadUserVersion,
        SchemaStatement::SchemaObjects,
        SchemaStatement::TableList,
    ];
    for table in SchemaTable::ALL {
        statements.extend([
            SchemaStatement::TableInfo(table),
            SchemaStatement::IndexList(table),
            SchemaStatement::ForeignKeyList(table),
        ]);
    }
    for index in SchemaIndex::ALL {
        statements.extend([
            SchemaStatement::IndexInfo(index),
            SchemaStatement::IndexXinfo(index),
        ]);
    }
    statements.extend([
        SchemaStatement::IntegrityCheck,
        SchemaStatement::ForeignKeyCheck,
    ]);
    statements
}

#[test]
fn schema_v3_static_inventory_is_closed_and_comment_free() {
    assert_eq!(APPLICATION_TABLES.len(), 8);
    assert_eq!(NAMED_INDEXES.len(), 11);
    assert_eq!(AUTO_INDEXES.len(), 6);
    assert_eq!(
        CREATE_TABLE_STATEMENTS.map(|entry| entry.0),
        APPLICATION_TABLES
    );
    assert_eq!(CREATE_INDEX_STATEMENTS.map(|entry| entry.0), NAMED_INDEXES);

    let names = APPLICATION_TABLES
        .into_iter()
        .chain(NAMED_INDEXES)
        .chain(AUTO_INDEXES)
        .collect::<BTreeSet<_>>();
    assert_eq!(names.len(), 25);
    for (_, statement) in CREATE_TABLE_STATEMENTS
        .into_iter()
        .chain(CREATE_INDEX_STATEMENTS)
    {
        assert!(!statement.contains("--"));
        assert!(!statement.contains("/*"));
        assert!(!statement.contains("*/"));
        assert!(!statement.contains('?'));
    }
}

#[test]
fn schema_v3_statement_callbacks_are_complete_ordered_and_fail_before_boundary() {
    assert_eq!(SchemaTable::ALL.map(SchemaTable::name), APPLICATION_TABLES);
    assert_eq!(
        NamedSchemaIndex::ALL.map(NamedSchemaIndex::name),
        NAMED_INDEXES
    );
    let expected_indexes = AUTO_INDEXES
        .into_iter()
        .chain(NAMED_INDEXES)
        .collect::<Vec<_>>();
    assert_eq!(
        SchemaIndex::ALL.map(SchemaIndex::name).as_slice(),
        expected_indexes.as_slice()
    );

    let mut connection = Connection::open_in_memory().expect("open scoped fixture");
    connection
        .pragma_update(None, "foreign_keys", true)
        .expect("enable foreign keys");
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("begin scoped creator transaction");
    let mut initialization = Vec::new();
    initialize_v3_scoped(&transaction, |statement| {
        initialization.push(statement);
        Ok(())
    })
    .expect("initialize through exact statement scopes");
    transaction.commit().expect("commit scoped schema");

    let expected_initialization = SchemaTable::ALL
        .into_iter()
        .map(SchemaStatement::CreateTable)
        .chain(
            NamedSchemaIndex::ALL
                .into_iter()
                .map(SchemaStatement::CreateIndex),
        )
        .chain([SchemaStatement::SetUserVersion])
        .chain(validation_statement_order())
        .collect::<Vec<_>>();
    assert_eq!(initialization, expected_initialization);
    assert_eq!(initialization.len(), 83);

    let mut validation = Vec::new();
    validate_v3_scoped(&connection, |statement| {
        validation.push(statement);
        Ok(())
    })
    .expect("validate through exact statement scopes");
    assert_eq!(validation, validation_statement_order());
    assert_eq!(validation.len(), 63);

    let rejected = SchemaStatement::TableInfo(SchemaTable::IntentUnits);
    let mut attempted = Vec::new();
    assert_eq!(
        validate_v3_scoped(&connection, |statement| {
            attempted.push(statement);
            if statement == rejected {
                Err(BackendError::CorruptSchema)
            } else {
                Ok(())
            }
        }),
        Err(BackendError::CorruptSchema)
    );
    assert_eq!(attempted.last(), Some(&rejected));
    assert_eq!(
        attempted,
        validation_statement_order()
            .into_iter()
            .take_while(|statement| *statement != rejected)
            .chain([rejected])
            .collect::<Vec<_>>()
    );
}

#[test]
fn schema_v3_initialization_is_schema_only_and_validates_exhaustively() {
    let connection = initialized_connection();
    assert_eq!(user_version(&connection), Ok(SCHEMA_VERSION));
    assert_eq!(validate_v3(&connection), Ok(()));

    for query in [
        "SELECT 1 FROM projection_anchor LIMIT 1",
        "SELECT 1 FROM projected_blocks LIMIT 1",
        "SELECT 1 FROM projected_events LIMIT 1",
        "SELECT 1 FROM projection_checkpoint LIMIT 1",
        "SELECT 1 FROM intent_units LIMIT 1",
        "SELECT 1 FROM relationship_definitions LIMIT 1",
        "SELECT 1 FROM intent_unit_relationships LIMIT 1",
        "SELECT 1 FROM recorded_associations LIMIT 1",
    ] {
        let mut statement = connection.prepare(query).expect("prepare empty query");
        assert!(!statement.exists([]).expect("query empty table"));
    }
}

#[test]
fn schema_v3_validation_rejects_extra_or_edited_objects_and_versions() {
    let connection = initialized_connection();
    connection
        .execute("CREATE TABLE intruder(value TEXT) STRICT", [])
        .expect("create extra object");
    assert_eq!(validate_v3(&connection), Err(BackendError::CorruptSchema));

    let connection = initialized_connection();
    connection
        .execute("DROP INDEX intent_units_by_status", [])
        .expect("drop exact index");
    connection
        .execute(
            "CREATE INDEX intent_units_by_status ON intent_units(id,status)",
            [],
        )
        .expect("replace with wrong order");
    assert_eq!(validate_v3(&connection), Err(BackendError::CorruptSchema));

    let connection = initialized_connection();
    connection
        .pragma_update(None, "user_version", 2_i64)
        .expect("edit schema version");
    assert_eq!(
        validate_v3(&connection),
        Err(BackendError::UnsupportedSchemaVersion { found: 2 })
    );
}

#[test]
fn schema_v3_enforces_exact_blob_text_and_envelope_bounds() {
    let connection = initialized_connection();
    let oversized = "x".repeat(crate::stored::MAX_ENVELOPE_BYTES + 1);
    let result = connection.execute(
        "INSERT INTO intent_units(id,envelope_version,envelope,origin_namespace,origin_scope,origin_value,workflow_id,species,phase,status,revision,last_global_sequence) VALUES(?1,2,?2,'fixture','scope','value','flow','intent','ready','active',?3,?4)",
        rusqlite::params![
            "00000000-0000-4000-8000-000000000001",
            oversized,
            [0_u8; 8].as_slice(),
            1_u64.to_be_bytes().as_slice(),
        ],
    );
    assert!(result.is_err(), "envelope ceiling must be enforced by DDL");

    assert!(
        CREATE_INTENT_UNITS_SQL.contains("length(CAST(envelope AS BLOB)) BETWEEN 1 AND 2097152")
    );
    assert!(CREATE_PROJECTED_EVENTS_SQL.contains("length(scale_payload) BETWEEN 1 AND 1048576"));
}
