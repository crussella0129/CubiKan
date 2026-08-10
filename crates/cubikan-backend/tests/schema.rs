mod common;

use std::{error::Error as _, fs, process::Command};

use common::TestDatabase;
use cubikan_backend::{BackendError, SqliteBackend};
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;

const EXPECTED_TABLE_SQL: &str = r#"CREATE TABLE intent_units (
    id TEXT NOT NULL PRIMARY KEY COLLATE BINARY,
    envelope_version INTEGER NOT NULL CHECK(envelope_version = 1),
    envelope TEXT NOT NULL,
    workflow_id TEXT NOT NULL COLLATE BINARY,
    species TEXT NOT NULL COLLATE BINARY,
    phase TEXT NOT NULL COLLATE BINARY,
    status TEXT NOT NULL COLLATE BINARY CHECK(status IN ('active','completed')),
    revision BLOB NOT NULL CHECK(length(revision) = 8)
) STRICT"#;

const EXPECTED_DEFINITIONS_SQL: &str = r#"CREATE TABLE relationship_definitions (
    definition_id TEXT NOT NULL COLLATE BINARY
        CHECK(
            length(CAST(definition_id AS BLOB)) BETWEEN 1 AND 64
            AND instr(definition_id, char(0)) = 0
            AND definition_id GLOB '[a-z]*'
            AND definition_id NOT GLOB '*[^a-z0-9._-]*'
        ),
    definition_version BLOB NOT NULL
        CHECK(
            length(definition_version) = 8
            AND definition_version <> X'0000000000000000'
        ),
    directed INTEGER NOT NULL CHECK(directed = 1),
    source_species TEXT COLLATE BINARY,
    target_species TEXT COLLATE BINARY,
    self_policy TEXT NOT NULL COLLATE BINARY
        CHECK(self_policy IN ('allow','reject')),
    cycle_policy TEXT NOT NULL COLLATE BINARY
        CHECK(cycle_policy IN ('allow','reject')),
    PRIMARY KEY(definition_id,definition_version)
) STRICT"#;

const EXPECTED_RELATIONSHIPS_SQL: &str = r#"CREATE TABLE intent_unit_relationships (
    definition_id TEXT NOT NULL COLLATE BINARY,
    definition_version BLOB NOT NULL
        CHECK(
            length(definition_version) = 8
            AND definition_version <> X'0000000000000000'
        ),
    source_id TEXT NOT NULL COLLATE BINARY,
    target_id TEXT NOT NULL COLLATE BINARY,
    PRIMARY KEY(definition_id,definition_version,source_id,target_id),
    FOREIGN KEY(definition_id,definition_version)
        REFERENCES relationship_definitions(definition_id,definition_version)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(source_id) REFERENCES intent_units(id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(target_id) REFERENCES intent_units(id)
        ON UPDATE RESTRICT ON DELETE RESTRICT
) STRICT"#;

const EXPECTED_INDEXES: [(&str, &str, &str); 4] = [
    (
        "intent_units_by_phase",
        "phase",
        "CREATE INDEX intent_units_by_phase ON intent_units(phase,id)",
    ),
    (
        "intent_units_by_species",
        "species",
        "CREATE INDEX intent_units_by_species ON intent_units(species,id)",
    ),
    (
        "intent_units_by_status",
        "status",
        "CREATE INDEX intent_units_by_status ON intent_units(status,id)",
    ),
    (
        "intent_units_by_workflow",
        "workflow_id",
        "CREATE INDEX intent_units_by_workflow ON intent_units(workflow_id,id)",
    ),
];

const EXPECTED_CORE_MANIFEST: &str = r#"[package]
name = "cubikan-core"
version = "0.1.0"
description = "Chain-agnostic domain core for CubiKan intent workflows"
edition.workspace = true
license.workspace = true
repository.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
serde.workspace = true
uuid.workspace = true

[dev-dependencies]
serde_json.workspace = true
"#;

const EXPECTED_CLI_MANIFEST: &str = r#"[package]
name = "cubikan-cli"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
publish = false

[dependencies]
cubikan-core = { path = "../cubikan-core" }
serde.workspace = true
serde_json.workspace = true

[[bin]]
name = "cubikan"
path = "src/main.rs"

[lints]
workspace = true
"#;

#[derive(Debug, Eq, PartialEq)]
struct LogicalSnapshot {
    user_version: i64,
    schema_version: i64,
    journal_mode: String,
    objects: Vec<(String, String, String, Option<String>)>,
    foreign_values: Vec<String>,
    versioned_values: Vec<String>,
    unexpected_values: Vec<String>,
    intent_units: Vec<IntentUnitRow>,
    definitions: Vec<DefinitionRow>,
    relationships: Vec<RelationshipRow>,
}

#[derive(Debug, Eq, PartialEq)]
struct IntentUnitRow {
    id: String,
    envelope_version: i64,
    envelope: String,
    workflow_id: String,
    species: String,
    phase: String,
    status: String,
    revision: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
struct DefinitionRow {
    id: String,
    version: Vec<u8>,
    directed: i64,
    source_species: Option<String>,
    target_species: Option<String>,
    self_policy: String,
    cycle_policy: String,
}

#[derive(Debug, Eq, PartialEq)]
struct RelationshipRow {
    definition_id: String,
    definition_version: Vec<u8>,
    source_id: String,
    target_id: String,
}

#[derive(Debug, Eq, PartialEq)]
struct ForeignKeyRow {
    id: i64,
    sequence: i64,
    table: String,
    from: String,
    to: String,
    on_update: String,
    on_delete: String,
    match_kind: String,
}

fn pragma_i64(connection: &Connection, name: &str) -> i64 {
    connection
        .pragma_query_value(None, name, |row| row.get(0))
        .expect("fixture PRAGMA should be readable")
}

fn logical_snapshot(connection: &Connection) -> LogicalSnapshot {
    let journal_mode = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .expect("journal mode should be readable");
    let mut statement = connection
        .prepare(
            "SELECT type, name, tbl_name, sql
             FROM main.sqlite_schema
             ORDER BY name",
        )
        .expect("schema should be readable");
    let objects = statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("schema query should execute")
        .collect::<Result<Vec<_>, _>>()
        .expect("schema rows should decode");
    let foreign_values = text_values_if_present(connection, &objects, "foreign_data", "value");
    let versioned_values = text_values_if_present(connection, &objects, "versioned_data", "value");
    let unexpected_values = text_values_if_present(connection, &objects, "unexpected", "value");
    let intent_units = if has_table(&objects, "intent_units") {
        let mut statement = connection
            .prepare(
                "SELECT id, envelope_version, envelope, workflow_id, species, phase, status, revision
                 FROM intent_units ORDER BY id",
            )
            .expect("Intent Unit rows should be readable");
        statement
            .query_map([], |row| {
                Ok(IntentUnitRow {
                    id: row.get(0)?,
                    envelope_version: row.get(1)?,
                    envelope: row.get(2)?,
                    workflow_id: row.get(3)?,
                    species: row.get(4)?,
                    phase: row.get(5)?,
                    status: row.get(6)?,
                    revision: row.get(7)?,
                })
            })
            .expect("Intent Unit snapshot query should execute")
            .collect::<Result<Vec<_>, _>>()
            .expect("Intent Unit snapshot rows should decode")
    } else {
        Vec::new()
    };
    let definitions = if has_table(&objects, "relationship_definitions") {
        connection
            .prepare(
                "SELECT definition_id,definition_version,directed,source_species,target_species,
                        self_policy,cycle_policy
                 FROM relationship_definitions ORDER BY definition_id,definition_version",
            )
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| {
                        Ok(DefinitionRow {
                            id: row.get(0)?,
                            version: row.get(1)?,
                            directed: row.get(2)?,
                            source_species: row.get(3)?,
                            target_species: row.get(4)?,
                            self_policy: row.get(5)?,
                            cycle_policy: row.get(6)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()
            })
            .expect("relationship definitions should be readable")
    } else {
        Vec::new()
    };
    let relationships = if has_table(&objects, "intent_unit_relationships") {
        connection
            .prepare(
                "SELECT definition_id,definition_version,source_id,target_id
                 FROM intent_unit_relationships
                 ORDER BY definition_id,definition_version,source_id,target_id",
            )
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| {
                        Ok(RelationshipRow {
                            definition_id: row.get(0)?,
                            definition_version: row.get(1)?,
                            source_id: row.get(2)?,
                            target_id: row.get(3)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()
            })
            .expect("relationships should be readable")
    } else {
        Vec::new()
    };
    LogicalSnapshot {
        user_version: pragma_i64(connection, "user_version"),
        schema_version: pragma_i64(connection, "schema_version"),
        journal_mode,
        objects,
        foreign_values,
        versioned_values,
        unexpected_values,
        intent_units,
        definitions,
        relationships,
    }
}

fn has_table(objects: &[(String, String, String, Option<String>)], name: &str) -> bool {
    objects
        .iter()
        .any(|(object_type, object_name, _, _)| object_type == "table" && object_name == name)
}

fn text_values_if_present(
    connection: &Connection,
    objects: &[(String, String, String, Option<String>)],
    table: &str,
    column: &str,
) -> Vec<String> {
    if !has_table(objects, table) {
        return Vec::new();
    }
    let query = format!("SELECT {column} FROM {table} ORDER BY {column}");
    let mut statement = connection
        .prepare(&query)
        .expect("sentinel table should be readable");
    statement
        .query_map([], |row| row.get(0))
        .expect("sentinel snapshot query should execute")
        .collect::<Result<Vec<_>, _>>()
        .expect("sentinel snapshot rows should decode")
}

fn set_wal(connection: &Connection) {
    let mode: String = connection
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .expect("fixture should select WAL mode");
    assert_eq!(mode.to_ascii_lowercase(), "wal");
}

fn assert_exact_owned_schema(connection: &Connection) {
    assert_eq!(pragma_i64(connection, "user_version"), 2);
    let snapshot = logical_snapshot(connection);
    assert_eq!(snapshot.objects.len(), 12);
    assert!(snapshot.objects.contains(&(
        "table".to_owned(),
        "intent_units".to_owned(),
        "intent_units".to_owned(),
        Some(EXPECTED_TABLE_SQL.to_owned()),
    )));
    for (name, table, sql) in [
        (
            "relationship_definitions",
            "relationship_definitions",
            EXPECTED_DEFINITIONS_SQL,
        ),
        (
            "intent_unit_relationships",
            "intent_unit_relationships",
            EXPECTED_RELATIONSHIPS_SQL,
        ),
        (
            "relationship_edges_by_source",
            "intent_unit_relationships",
            "CREATE INDEX relationship_edges_by_source ON intent_unit_relationships(definition_id,definition_version,source_id,target_id)",
        ),
        (
            "relationship_edges_by_target",
            "intent_unit_relationships",
            "CREATE INDEX relationship_edges_by_target ON intent_unit_relationships(definition_id,definition_version,target_id,source_id)",
        ),
    ] {
        assert!(snapshot.objects.contains(&(
            if name.starts_with("relationship_edges") {
                "index".to_owned()
            } else {
                "table".to_owned()
            },
            name.to_owned(),
            table.to_owned(),
            Some(sql.to_owned()),
        )));
    }
    for (name, table) in [
        (
            "sqlite_autoindex_relationship_definitions_1",
            "relationship_definitions",
        ),
        (
            "sqlite_autoindex_intent_unit_relationships_1",
            "intent_unit_relationships",
        ),
    ] {
        assert!(snapshot.objects.contains(&(
            "index".to_owned(),
            name.to_owned(),
            table.to_owned(),
            None,
        )));
    }
    for (name, _, sql) in EXPECTED_INDEXES {
        assert!(snapshot.objects.contains(&(
            "index".to_owned(),
            name.to_owned(),
            "intent_units".to_owned(),
            Some(sql.to_owned()),
        )));
    }
    assert!(snapshot.objects.contains(&(
        "index".to_owned(),
        "sqlite_autoindex_intent_units_1".to_owned(),
        "intent_units".to_owned(),
        None,
    )));

    let columns = connection
        .prepare("PRAGMA table_xinfo('intent_units')")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .expect("table columns should be inspectable");
    assert_eq!(
        columns,
        [
            (0, "id".to_owned(), "TEXT".to_owned(), 1, None, 1, 0),
            (
                1,
                "envelope_version".to_owned(),
                "INTEGER".to_owned(),
                1,
                None,
                0,
                0,
            ),
            (2, "envelope".to_owned(), "TEXT".to_owned(), 1, None, 0, 0,),
            (
                3,
                "workflow_id".to_owned(),
                "TEXT".to_owned(),
                1,
                None,
                0,
                0,
            ),
            (4, "species".to_owned(), "TEXT".to_owned(), 1, None, 0, 0,),
            (5, "phase".to_owned(), "TEXT".to_owned(), 1, None, 0, 0,),
            (6, "status".to_owned(), "TEXT".to_owned(), 1, None, 0, 0,),
            (7, "revision".to_owned(), "BLOB".to_owned(), 1, None, 0, 0,),
        ]
    );

    let (without_rowid, strict): (i64, i64) = connection
        .query_row(
            "SELECT wr, strict FROM pragma_table_list WHERE schema='main' AND name='intent_units'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("table flags should be inspectable");
    assert_eq!((without_rowid, strict), (0, 1));

    for table in ["relationship_definitions", "intent_unit_relationships"] {
        let flags: (i64, i64) = connection
            .query_row(
                "SELECT wr, strict FROM pragma_table_list WHERE schema='main' AND name=?1",
                [table],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("relationship table flags should be inspectable");
        assert_eq!(flags, (0, 1));
    }

    let definition_columns = table_columns(connection, "relationship_definitions");
    assert_eq!(
        definition_columns,
        [
            ("definition_id".to_owned(), "TEXT".to_owned(), 1, 1, 0),
            ("definition_version".to_owned(), "BLOB".to_owned(), 1, 2, 0),
            ("directed".to_owned(), "INTEGER".to_owned(), 1, 0, 0),
            ("source_species".to_owned(), "TEXT".to_owned(), 0, 0, 0),
            ("target_species".to_owned(), "TEXT".to_owned(), 0, 0, 0),
            ("self_policy".to_owned(), "TEXT".to_owned(), 1, 0, 0),
            ("cycle_policy".to_owned(), "TEXT".to_owned(), 1, 0, 0),
        ]
    );
    let relationship_columns = table_columns(connection, "intent_unit_relationships");
    assert_eq!(
        relationship_columns,
        [
            ("definition_id".to_owned(), "TEXT".to_owned(), 1, 1, 0),
            ("definition_version".to_owned(), "BLOB".to_owned(), 1, 2, 0),
            ("source_id".to_owned(), "TEXT".to_owned(), 1, 3, 0),
            ("target_id".to_owned(), "TEXT".to_owned(), 1, 4, 0),
        ]
    );

    for (index, expected) in [
        (
            "sqlite_autoindex_relationship_definitions_1",
            vec!["definition_id", "definition_version"],
        ),
        (
            "sqlite_autoindex_intent_unit_relationships_1",
            vec![
                "definition_id",
                "definition_version",
                "source_id",
                "target_id",
            ],
        ),
        (
            "relationship_edges_by_source",
            vec![
                "definition_id",
                "definition_version",
                "source_id",
                "target_id",
            ],
        ),
        (
            "relationship_edges_by_target",
            vec![
                "definition_id",
                "definition_version",
                "target_id",
                "source_id",
            ],
        ),
    ] {
        assert_eq!(index_columns(connection, index), expected);
    }

    let foreign_keys = connection
        .prepare("PRAGMA foreign_key_list('intent_unit_relationships')")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok(ForeignKeyRow {
                        id: row.get(0)?,
                        sequence: row.get(1)?,
                        table: row.get(2)?,
                        from: row.get(3)?,
                        to: row.get(4)?,
                        on_update: row.get(5)?,
                        on_delete: row.get(6)?,
                        match_kind: row.get(7)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .expect("foreign keys should be inspectable");
    assert_eq!(foreign_keys.len(), 4);
    assert!(foreign_keys.iter().all(|row| row.on_update == "RESTRICT"
        && row.on_delete == "RESTRICT"
        && row.match_kind == "NONE"));
    let mappings = foreign_keys
        .iter()
        .map(|row| (row.table.as_str(), row.from.as_str(), row.to.as_str()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        mappings,
        std::collections::BTreeSet::from([
            ("relationship_definitions", "definition_id", "definition_id"),
            (
                "relationship_definitions",
                "definition_version",
                "definition_version",
            ),
            ("intent_units", "source_id", "id"),
            ("intent_units", "target_id", "id"),
        ])
    );
    let definition_fk = foreign_keys
        .iter()
        .filter(|row| row.table == "relationship_definitions")
        .collect::<Vec<_>>();
    assert_eq!(definition_fk.len(), 2);
    assert_eq!(definition_fk[0].id, definition_fk[1].id);
    assert_eq!(
        definition_fk
            .iter()
            .map(|row| row.sequence)
            .collect::<Vec<_>>(),
        [0, 1]
    );

    let integrity = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
        .expect("integrity should be inspectable");
    assert_eq!(integrity, "ok");
    let foreign_key_violation = connection
        .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
        .optional()
        .expect("foreign key check should execute");
    assert!(foreign_key_violation.is_none());

    for (name, first_column, _) in EXPECTED_INDEXES {
        let key_columns = connection
            .prepare(&format!("PRAGMA index_xinfo('{name}')"))
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, i64>(5)?,
                        ))
                    })?
                    .filter_map(|row| match row {
                        Ok(value) if value.3 == 1 => Some(Ok(value)),
                        Ok(_) => None,
                        Err(error) => Some(Err(error)),
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .expect("index columns should be inspectable");
        assert_eq!(
            key_columns,
            [
                (0, Some(first_column.to_owned()), "BINARY".to_owned(), 1),
                (1, Some("id".to_owned()), "BINARY".to_owned(), 1),
            ]
        );
    }

    let journal_mode: String = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .expect("journal mode should be readable");
    let locking_mode: String = connection
        .pragma_query_value(None, "locking_mode", |row| row.get(0))
        .expect("locking mode should be readable");
    assert_eq!(journal_mode.to_ascii_lowercase(), "delete");
    assert_eq!(locking_mode.to_ascii_lowercase(), "normal");
}

fn table_columns(connection: &Connection, table: &str) -> Vec<(String, String, i64, i64, i64)> {
    connection
        .prepare(&format!("PRAGMA table_xinfo('{table}')"))
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok((
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .expect("table metadata should be readable")
}

fn index_columns(connection: &Connection, index: &str) -> Vec<String> {
    connection
        .prepare(&format!("PRAGMA index_xinfo('{index}')"))
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                })?
                .filter_map(|row| match row {
                    Ok((Some(name), collation, 1)) => {
                        assert_eq!(collation, "BINARY");
                        Some(Ok(name))
                    }
                    Ok((_, _, 0)) => None,
                    Ok(other) => panic!("unexpected index metadata: {other:?}"),
                    Err(error) => Some(Err(error)),
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .expect("index metadata should be readable")
}

fn initialize_and_drop(database: &TestDatabase) {
    drop(SqliteBackend::open(database.path()).expect("fixture database should initialize"));
}

fn insert_sentinel_intent_unit(connection: &Connection) {
    connection
        .execute(
            "INSERT INTO intent_units (
                id, envelope_version, envelope, workflow_id, species, phase, status, revision
             ) VALUES (?1, 1, ?2, ?3, ?4, ?5, 'active', ?6)",
            (
                "00000000-0000-0000-0000-000000000000",
                r#"{"sentinel":true}"#,
                "sentinel-workflow",
                "sentinel-species",
                "sentinel-phase",
                [0_u8; 8],
            ),
        )
        .expect("sentinel Intent Unit row should insert");
}

fn assert_rejected_without_logical_change(
    database: &TestDatabase,
    expected: impl FnOnce(&BackendError) -> bool,
) {
    let before = {
        let connection = database.connect();
        logical_snapshot(&connection)
    };
    assert!(
        !before.foreign_values.is_empty()
            || !before.versioned_values.is_empty()
            || !before.unexpected_values.is_empty()
            || !before.intent_units.is_empty()
            || !before.definitions.is_empty()
            || !before.relationships.is_empty(),
        "rejection fixture must contain sentinel row content"
    );
    let error = SqliteBackend::open(database.path()).expect_err("fixture should be rejected");
    assert!(expected(&error), "unexpected error: {error:?}");
    let after = {
        let connection = database.connect();
        logical_snapshot(&connection)
    };
    assert_eq!(after, before);
}

#[test]
fn test_new_and_existing_v2_databases_have_exact_schema_and_pragmas() {
    for precreate_empty_file in [false, true] {
        let database = TestDatabase::new("initialize");
        if precreate_empty_file {
            fs::File::create(database.path()).expect("empty fixture file should be created");
        }

        initialize_and_drop(&database);

        let connection = database.connect();
        assert_exact_owned_schema(&connection);
        insert_sentinel_intent_unit(&connection);
        let before_reopen = logical_snapshot(&connection);
        drop(connection);
        initialize_and_drop(&database);
        let connection = database.connect();
        assert_exact_owned_schema(&connection);
        assert_eq!(logical_snapshot(&connection), before_reopen);
    }
}

#[test]
fn test_open_rejects_corrupt_v2_without_repair() {
    for path in ["", ":memory:"] {
        let error = SqliteBackend::open(path).expect_err("special path should be rejected");
        assert!(matches!(error, BackendError::Storage(_)));
        let category_source = error
            .source()
            .expect("storage category should retain an opaque source");
        assert!(
            category_source.source().is_some(),
            "opaque category should retain the original rusqlite diagnostic"
        );
        assert_eq!(error.to_string(), "CubiKan storage operation failed");
    }

    let unowned = TestDatabase::new("unowned-v0");
    {
        let connection = unowned.connect();
        connection
            .execute_batch(
                "CREATE TABLE foreign_data(value TEXT); INSERT INTO foreign_data VALUES ('keep');",
            )
            .expect("unowned fixture should initialize");
        set_wal(&connection);
    }
    assert_rejected_without_logical_change(&unowned, |error| {
        matches!(error, BackendError::UnownedDatabase)
    });

    let unsupported = TestDatabase::new("version-3");
    {
        let connection = unsupported.connect();
        connection
            .execute_batch(
                "CREATE TABLE versioned_data(value TEXT);
                 INSERT INTO versioned_data VALUES ('version-3-keep');",
            )
            .expect("unsupported-version sentinel should initialize");
        connection
            .pragma_update(None, "user_version", 3_i64)
            .expect("fixture version should be assigned");
        set_wal(&connection);
    }
    assert_rejected_without_logical_change(&unsupported, |error| {
        matches!(error, BackendError::UnsupportedSchemaVersion { found: 3 })
    });

    let missing = TestDatabase::new("missing-v1");
    initialize_and_drop(&missing);
    {
        let connection = missing.connect();
        insert_sentinel_intent_unit(&connection);
        connection
            .execute("DROP INDEX intent_units_by_status", [])
            .expect("fixture index should drop");
        set_wal(&connection);
    }
    assert_rejected_without_logical_change(&missing, |error| {
        matches!(error, BackendError::CorruptSchema)
    });

    let wrong = TestDatabase::new("wrong-v1");
    initialize_and_drop(&wrong);
    {
        let connection = wrong.connect();
        insert_sentinel_intent_unit(&connection);
        connection
            .execute("DROP INDEX intent_units_by_phase", [])
            .expect("fixture index should drop");
        connection
            .execute(
                "CREATE INDEX intent_units_by_phase ON intent_units(status,id)",
                [],
            )
            .expect("wrong fixture index should create");
        set_wal(&connection);
    }
    assert_rejected_without_logical_change(&wrong, |error| {
        matches!(error, BackendError::CorruptSchema)
    });

    let extra = TestDatabase::new("extra-v1");
    initialize_and_drop(&extra);
    {
        let connection = extra.connect();
        insert_sentinel_intent_unit(&connection);
        connection
            .execute("CREATE TABLE unexpected(value TEXT) STRICT", [])
            .expect("extra fixture table should create");
        connection
            .execute("INSERT INTO unexpected VALUES ('extra-keep')", [])
            .expect("extra fixture sentinel should insert");
        set_wal(&connection);
    }
    assert_rejected_without_logical_change(&extra, |error| {
        matches!(error, BackendError::CorruptSchema)
    });

    let invalid_checks = TestDatabase::new("invalid-v2-checks");
    initialize_and_drop(&invalid_checks);
    {
        let connection = invalid_checks.connect();
        connection
            .pragma_update(None, "ignore_check_constraints", 1_i64)
            .expect("fixture should bypass CHECK constraints");
        connection
            .execute(
                "INSERT INTO relationship_definitions VALUES
                    ('Bad',?1,1,NULL,NULL,'allow','allow'),
                    ('valid',?2,1,NULL,NULL,'allow','allow')",
                (1_u64.to_be_bytes(), 0_u64.to_be_bytes()),
            )
            .expect("CHECK-invalid definitions should insert");
        connection
            .pragma_update(None, "ignore_check_constraints", 0_i64)
            .expect("fixture should restore CHECK enforcement");
        set_wal(&connection);
    }
    assert_rejected_without_logical_change(&invalid_checks, |error| {
        matches!(error, BackendError::CorruptSchema)
    });

    let invalid_foreign_keys = TestDatabase::new("invalid-v2-foreign-keys");
    initialize_and_drop(&invalid_foreign_keys);
    {
        let connection = invalid_foreign_keys.connect();
        connection
            .pragma_update(None, "foreign_keys", 0_i64)
            .expect("fixture should bypass foreign keys");
        insert_sentinel_intent_unit(&connection);
        connection
            .execute(
                "INSERT INTO relationship_definitions VALUES
                    ('valid',?1,1,NULL,NULL,'allow','allow')",
                [1_u64.to_be_bytes()],
            )
            .expect("valid definition should insert");
        connection
            .execute(
                "INSERT INTO intent_unit_relationships VALUES
                    ('missing',?1,?2,?2),
                    ('valid',?1,?3,?2),
                    ('valid',?1,?2,?4)",
                rusqlite::params![
                    1_u64.to_be_bytes(),
                    "00000000-0000-0000-0000-000000000000",
                    "00000000-0000-0000-0000-000000000001",
                    "00000000-0000-0000-0000-000000000002",
                ],
            )
            .expect("FK-invalid edges should insert");
        set_wal(&connection);
    }
    assert_rejected_without_logical_change(&invalid_foreign_keys, |error| {
        matches!(error, BackendError::CorruptSchema)
    });

    let invalid_schema_text = TestDatabase::new("invalid-v2-schema-text");
    initialize_and_drop(&invalid_schema_text);
    let injected_sql = "CREATE INDEX intent_units_by_status ON intent_units(";
    {
        let connection = invalid_schema_text.connect();
        set_wal(&connection);
        connection
            .pragma_update(None, "writable_schema", 1_i64)
            .expect("fixture should enable writable_schema");
        connection
            .execute(
                "UPDATE sqlite_schema SET sql=?1 WHERE name='intent_units_by_status'",
                [injected_sql],
            )
            .expect("fixture should inject invalid schema text");
        let next_schema_version = pragma_i64(&connection, "schema_version") + 1;
        connection
            .pragma_update(None, "schema_version", next_schema_version)
            .expect("fixture schema cache should be invalidated");
    }
    let error = SqliteBackend::open(invalid_schema_text.path())
        .expect_err("invalid sqlite_schema SQL should be rejected");
    assert!(matches!(error, BackendError::CorruptSchema));
    {
        let connection = invalid_schema_text.connect();
        connection
            .pragma_update(None, "writable_schema", 1_i64)
            .expect("tampered schema should be inspectable");
        let retained: String = connection
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE name='intent_units_by_status'",
                [],
                |row| row.get(0),
            )
            .expect("injected schema text should remain readable");
        assert_eq!(retained, injected_sql);
        assert_eq!(pragma_i64(&connection, "user_version"), 2);
    }

    let reserved = TestDatabase::new("reserved-v1");
    initialize_and_drop(&reserved);
    {
        let connection = reserved.connect();
        set_wal(&connection);
        connection
            .execute("CREATE TABLE evil_payload(value TEXT) STRICT", [])
            .expect("adversarial source table should create");
        connection
            .pragma_update(None, "writable_schema", 1_i64)
            .expect("fixture should enable writable_schema");
        connection
            .execute(
                "UPDATE sqlite_schema
                 SET name='sqlite_evil',
                     tbl_name='sqlite_evil',
                     sql='CREATE TABLE sqlite_evil(value TEXT) STRICT'
                 WHERE type='table' AND name='evil_payload'",
                [],
            )
            .expect("fixture should inject a reserved-name object");
        let next_schema_version = pragma_i64(&connection, "schema_version") + 1;
        connection
            .pragma_update(None, "schema_version", next_schema_version)
            .expect("fixture schema cache should be invalidated");
        connection
            .pragma_update(None, "writable_schema", 0_i64)
            .expect("fixture should disable writable_schema");
    }
    let error = SqliteBackend::open(reserved.path())
        .expect_err("unexpected sqlite_* object should be rejected");
    assert!(matches!(error, BackendError::CorruptSchema));
    {
        let connection = reserved.connect();
        connection
            .pragma_update(None, "writable_schema", 1_i64)
            .expect("fixture should reopen under writable_schema");
        let evil_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name='sqlite_evil'",
                [],
                |row| row.get(0),
            )
            .expect("injected object should remain inspectable");
        let journal_mode: String = connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("rejected fixture journal mode should remain inspectable");
        assert_eq!(evil_count, 1, "open must not repair or adopt sqlite_evil");
        assert_eq!(pragma_i64(&connection, "user_version"), 2);
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    }

    let aliased_rootpage = TestDatabase::new("aliased-rootpage-v1");
    initialize_and_drop(&aliased_rootpage);
    {
        let connection = aliased_rootpage.connect();
        set_wal(&connection);
        let table_rootpage: i64 = connection
            .query_row(
                "SELECT rootpage FROM sqlite_schema WHERE name='intent_units'",
                [],
                |row| row.get(0),
            )
            .expect("owned table rootpage should be readable");
        let index_rootpage: i64 = connection
            .query_row(
                "SELECT rootpage FROM sqlite_schema
                 WHERE name='sqlite_autoindex_intent_units_1'",
                [],
                |row| row.get(0),
            )
            .expect("owned autoindex rootpage should be readable");
        assert_ne!(table_rootpage, index_rootpage);
        connection
            .pragma_update(None, "writable_schema", 1_i64)
            .expect("fixture should enable writable_schema");
        connection
            .execute(
                "UPDATE sqlite_schema SET rootpage=?1 WHERE name='intent_units'",
                [index_rootpage],
            )
            .expect("fixture should alias physical rootpages");
        let next_schema_version = pragma_i64(&connection, "schema_version") + 1;
        connection
            .pragma_update(None, "schema_version", next_schema_version)
            .expect("fixture schema cache should be invalidated");
        connection
            .pragma_update(None, "writable_schema", 0_i64)
            .expect("fixture should disable writable_schema");
    }
    let error = SqliteBackend::open(aliased_rootpage.path())
        .expect_err("physically aliased rootpages should be rejected");
    assert!(matches!(error, BackendError::CorruptSchema));
    {
        let connection = aliased_rootpage.connect();
        connection
            .pragma_update(None, "writable_schema", 1_i64)
            .expect("fixture should reopen under writable_schema");
        let rootpages = connection
            .prepare(
                "SELECT rootpage FROM sqlite_schema
                 WHERE name IN ('intent_units','sqlite_autoindex_intent_units_1')
                 ORDER BY name",
            )
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, i64>(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .expect("tampered rootpages should remain inspectable");
        let journal_mode: String = connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("rejected fixture journal mode should remain inspectable");
        assert_eq!(rootpages.len(), 2);
        assert_eq!(rootpages[0], rootpages[1], "open must not repair rootpages");
        assert_eq!(pragma_i64(&connection, "user_version"), 2);
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    }

    let corrupt = TestDatabase::new("not-sqlite");
    let original = b"this is deliberately not a SQLite database";
    fs::write(corrupt.path(), original).expect("corrupt fixture should be written");
    let error = SqliteBackend::open(corrupt.path()).expect_err("non-SQLite input should fail");
    assert!(matches!(error, BackendError::CorruptSchema));
    assert_eq!(
        fs::read(corrupt.path()).expect("corrupt fixture should remain readable"),
        original
    );
}

#[test]
fn test_sqlite_dependency_is_bundled_and_adapter_only() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .expect("backend crate should live below the workspace root");
    assert_eq!(
        fs::read_to_string(workspace.join("crates/cubikan-core/Cargo.toml"))
            .expect("core manifest should be readable"),
        EXPECTED_CORE_MANIFEST,
        "T-803 must not alter the core manifest"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("crates/cubikan-cli/Cargo.toml"))
            .expect("CLI manifest should be readable"),
        EXPECTED_CLI_MANIFEST,
        "T-803 must not alter the existing CLI manifest"
    );

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(&cargo)
        .args([
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--no-deps",
        ])
        .current_dir(workspace)
        .output()
        .expect("Cargo metadata should execute");
    assert!(output.status.success(), "Cargo metadata failed");
    let metadata: Value =
        serde_json::from_slice(&output.stdout).expect("Cargo metadata should be JSON");
    let packages = metadata["packages"]
        .as_array()
        .expect("metadata packages should be an array");
    let backend = packages
        .iter()
        .find(|package| package["name"] == "cubikan-backend")
        .expect("backend package should be present");
    let rusqlite = backend["dependencies"]
        .as_array()
        .expect("dependencies should be an array")
        .iter()
        .find(|dependency| dependency["name"] == "rusqlite")
        .expect("backend should depend on rusqlite");
    assert_eq!(rusqlite["req"], "=0.40.2");
    assert_eq!(rusqlite["uses_default_features"], false);
    assert_eq!(rusqlite["features"], serde_json::json!(["bundled"]));

    for package_name in ["cubikan-core", "cubikan-cli"] {
        let package = packages
            .iter()
            .find(|package| package["name"] == package_name)
            .expect("workspace package should be present");
        assert!(
            package["dependencies"]
                .as_array()
                .expect("dependencies should be an array")
                .iter()
                .all(|dependency| dependency["name"] != "rusqlite"),
            "{package_name} must remain free of rusqlite"
        );
    }

    for (package_name, should_contain_rusqlite) in [
        ("cubikan-backend", true),
        ("cubikan-core", false),
        ("cubikan-cli", false),
    ] {
        let output = Command::new(&cargo)
            .args(["tree", "--offline", "--depth", "1", "-p", package_name])
            .current_dir(workspace)
            .output()
            .expect("Cargo tree should execute");
        assert!(
            output.status.success(),
            "Cargo tree failed for {package_name}"
        );
        let tree = String::from_utf8(output.stdout).expect("Cargo tree should be UTF-8");
        assert_eq!(
            tree.contains("rusqlite v0.40.2"),
            should_contain_rusqlite,
            "unexpected rusqlite dependency boundary for {package_name}:\n{tree}"
        );
    }
}
