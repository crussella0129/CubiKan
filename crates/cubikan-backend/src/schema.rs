use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension};

use crate::{BackendError, BackendSchemaVersion};

pub(crate) const SCHEMA_VERSION_V1: i64 = 1;
pub(crate) const SCHEMA_VERSION_V2: i64 = 2;

pub(crate) const CREATE_INTENT_UNITS_SQL: &str = r#"CREATE TABLE intent_units (
    id TEXT NOT NULL PRIMARY KEY COLLATE BINARY,
    envelope_version INTEGER NOT NULL CHECK(envelope_version = 1),
    envelope TEXT NOT NULL,
    workflow_id TEXT NOT NULL COLLATE BINARY,
    species TEXT NOT NULL COLLATE BINARY,
    phase TEXT NOT NULL COLLATE BINARY,
    status TEXT NOT NULL COLLATE BINARY CHECK(status IN ('active','completed')),
    revision BLOB NOT NULL CHECK(length(revision) = 8)
) STRICT"#;

pub(crate) const CREATE_INTENT_UNIT_INDEX_SQL: [&str; 4] = [
    "CREATE INDEX intent_units_by_workflow ON intent_units(workflow_id,id)",
    "CREATE INDEX intent_units_by_species ON intent_units(species,id)",
    "CREATE INDEX intent_units_by_phase ON intent_units(phase,id)",
    "CREATE INDEX intent_units_by_status ON intent_units(status,id)",
];

pub(crate) const CREATE_RELATIONSHIP_DEFINITIONS_SQL: &str = r#"CREATE TABLE relationship_definitions (
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

pub(crate) const CREATE_INTENT_UNIT_RELATIONSHIPS_SQL: &str = r#"CREATE TABLE intent_unit_relationships (
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

pub(crate) const CREATE_RELATIONSHIP_INDEX_SQL: [&str; 2] = [
    "CREATE INDEX relationship_edges_by_source ON intent_unit_relationships(definition_id,definition_version,source_id,target_id)",
    "CREATE INDEX relationship_edges_by_target ON intent_unit_relationships(definition_id,definition_version,target_id,source_id)",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Ownership {
    Empty,
    OwnedV1,
    OwnedV2,
}

impl Ownership {
    pub(crate) const fn capability(self) -> Option<BackendSchemaVersion> {
        match self {
            Self::Empty => None,
            Self::OwnedV1 => Some(BackendSchemaVersion::V1),
            Self::OwnedV2 => Some(BackendSchemaVersion::V2),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct SchemaObject {
    object_type: String,
    name: String,
    table_name: String,
    sql: Option<String>,
}

pub(crate) fn inspect(connection: &Connection) -> Result<Ownership, BackendError> {
    let version = user_version(connection)?;
    if !matches!(version, 0 | SCHEMA_VERSION_V1 | SCHEMA_VERSION_V2) {
        return Err(BackendError::UnsupportedSchemaVersion { found: version });
    }
    let objects = schema_objects(connection)?;

    match version {
        0 if objects.is_empty() => {
            verify_integrity(connection, false)?;
            Ok(Ownership::Empty)
        }
        0 => Err(BackendError::UnownedDatabase),
        SCHEMA_VERSION_V1 if objects == expected_v1_schema_objects() => {
            verify_v1_metadata(connection)?;
            verify_integrity(connection, false)?;
            Ok(Ownership::OwnedV1)
        }
        SCHEMA_VERSION_V1 => Err(BackendError::CorruptSchema),
        SCHEMA_VERSION_V2 if objects == expected_v2_schema_objects() => {
            verify_v2_metadata(connection)?;
            verify_integrity(connection, true)?;
            Ok(Ownership::OwnedV2)
        }
        SCHEMA_VERSION_V2 => Err(BackendError::CorruptSchema),
        _ => unreachable!("schema versions were classified before object inspection"),
    }
}

pub(crate) fn user_version(connection: &Connection) -> Result<i64, BackendError> {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(classify_inspection_error)
}

pub(crate) fn initialize_v2(connection: &Connection) -> Result<(), BackendError> {
    initialize_v1_objects(connection)?;
    add_v2_objects(connection)?;
    set_user_version(connection, SCHEMA_VERSION_V2)
}

fn initialize_v1_objects(connection: &Connection) -> Result<(), BackendError> {
    execute_ddl(connection, CREATE_INTENT_UNITS_SQL)?;
    for statement in CREATE_INTENT_UNIT_INDEX_SQL {
        execute_ddl(connection, statement)?;
    }
    Ok(())
}

pub(crate) fn add_v2_objects(connection: &Connection) -> Result<(), BackendError> {
    execute_ddl(connection, CREATE_RELATIONSHIP_DEFINITIONS_SQL)?;
    execute_ddl(connection, CREATE_INTENT_UNIT_RELATIONSHIPS_SQL)?;
    for statement in CREATE_RELATIONSHIP_INDEX_SQL {
        execute_ddl(connection, statement)?;
    }
    Ok(())
}

pub(crate) fn set_user_version(connection: &Connection, version: i64) -> Result<(), BackendError> {
    connection
        .pragma_update(None, "user_version", version)
        .map_err(crate::sqlite::classify_runtime_error)
}

fn execute_ddl(connection: &Connection, sql: &str) -> Result<(), BackendError> {
    connection
        .execute(sql, [])
        .map_err(crate::sqlite::classify_runtime_error)?;
    Ok(())
}

fn verify_integrity(connection: &Connection, check_foreign_keys: bool) -> Result<(), BackendError> {
    let mut statement = connection
        .prepare("PRAGMA main.integrity_check")
        .map_err(classify_inspection_error)?;
    let diagnostics = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(classify_inspection_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_inspection_error)?;
    if diagnostics != ["ok"] {
        return Err(BackendError::CorruptSchema);
    }

    if check_foreign_keys {
        let violation = connection
            .query_row("PRAGMA main.foreign_key_check", [], |_| Ok(()))
            .optional()
            .map_err(classify_inspection_error)?;
        if violation.is_some() {
            return Err(BackendError::CorruptSchema);
        }
    }
    Ok(())
}

fn verify_v1_metadata(connection: &Connection) -> Result<(), BackendError> {
    verify_table(
        connection,
        "intent_units",
        &[
            ("id", "TEXT", 1, 1),
            ("envelope_version", "INTEGER", 1, 0),
            ("envelope", "TEXT", 1, 0),
            ("workflow_id", "TEXT", 1, 0),
            ("species", "TEXT", 1, 0),
            ("phase", "TEXT", 1, 0),
            ("status", "TEXT", 1, 0),
            ("revision", "BLOB", 1, 0),
        ],
    )?;
    verify_index_set(
        connection,
        "intent_units",
        &[
            ("sqlite_autoindex_intent_units_1", 1, "pk", 0),
            ("intent_units_by_workflow", 0, "c", 0),
            ("intent_units_by_species", 0, "c", 0),
            ("intent_units_by_phase", 0, "c", 0),
            ("intent_units_by_status", 0, "c", 0),
        ],
    )?;
    verify_index_columns(connection, "sqlite_autoindex_intent_units_1", &["id"])?;
    for (name, first) in [
        ("intent_units_by_workflow", "workflow_id"),
        ("intent_units_by_species", "species"),
        ("intent_units_by_phase", "phase"),
        ("intent_units_by_status", "status"),
    ] {
        verify_index_columns(connection, name, &[first, "id"])?;
    }
    Ok(())
}

fn verify_v2_metadata(connection: &Connection) -> Result<(), BackendError> {
    verify_v1_metadata(connection)?;
    verify_table(
        connection,
        "relationship_definitions",
        &[
            ("definition_id", "TEXT", 1, 1),
            ("definition_version", "BLOB", 1, 2),
            ("directed", "INTEGER", 1, 0),
            ("source_species", "TEXT", 0, 0),
            ("target_species", "TEXT", 0, 0),
            ("self_policy", "TEXT", 1, 0),
            ("cycle_policy", "TEXT", 1, 0),
        ],
    )?;
    verify_table(
        connection,
        "intent_unit_relationships",
        &[
            ("definition_id", "TEXT", 1, 1),
            ("definition_version", "BLOB", 1, 2),
            ("source_id", "TEXT", 1, 3),
            ("target_id", "TEXT", 1, 4),
        ],
    )?;
    verify_index_set(
        connection,
        "relationship_definitions",
        &[("sqlite_autoindex_relationship_definitions_1", 1, "pk", 0)],
    )?;
    verify_index_set(
        connection,
        "intent_unit_relationships",
        &[
            ("sqlite_autoindex_intent_unit_relationships_1", 1, "pk", 0),
            ("relationship_edges_by_source", 0, "c", 0),
            ("relationship_edges_by_target", 0, "c", 0),
        ],
    )?;
    verify_index_columns(
        connection,
        "sqlite_autoindex_relationship_definitions_1",
        &["definition_id", "definition_version"],
    )?;
    verify_index_columns(
        connection,
        "sqlite_autoindex_intent_unit_relationships_1",
        &[
            "definition_id",
            "definition_version",
            "source_id",
            "target_id",
        ],
    )?;
    verify_index_columns(
        connection,
        "relationship_edges_by_source",
        &[
            "definition_id",
            "definition_version",
            "source_id",
            "target_id",
        ],
    )?;
    verify_index_columns(
        connection,
        "relationship_edges_by_target",
        &[
            "definition_id",
            "definition_version",
            "target_id",
            "source_id",
        ],
    )?;
    verify_relationship_foreign_keys(connection)
}

fn verify_table(
    connection: &Connection,
    table: &str,
    expected: &[(&str, &str, i64, i64)],
) -> Result<(), BackendError> {
    let flags = connection
        .query_row(
            "SELECT wr, strict FROM pragma_table_list WHERE schema='main' AND name=?1",
            [table],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(classify_inspection_error)?;
    if flags != Some((0, 1)) {
        return Err(BackendError::CorruptSchema);
    }

    let sql = format!("PRAGMA main.table_xinfo('{table}')");
    let mut statement = connection
        .prepare(&sql)
        .map_err(classify_inspection_error)?;
    let actual = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(classify_inspection_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_inspection_error)?;
    let expected = expected
        .iter()
        .map(|(name, kind, not_null, pk)| {
            (
                (*name).to_owned(),
                (*kind).to_owned(),
                *not_null,
                None,
                *pk,
                0,
            )
        })
        .collect::<Vec<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

fn verify_index_set(
    connection: &Connection,
    table: &str,
    expected: &[(&str, i64, &str, i64)],
) -> Result<(), BackendError> {
    let sql = format!("PRAGMA main.index_list('{table}')");
    let mut statement = connection
        .prepare(&sql)
        .map_err(classify_inspection_error)?;
    let actual = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                (
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ),
            ))
        })
        .map_err(classify_inspection_error)?
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(classify_inspection_error)?;
    let expected = expected
        .iter()
        .map(|(name, unique, origin, partial)| {
            (
                (*name).to_owned(),
                (*unique, (*origin).to_owned(), *partial),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if actual == expected {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

fn verify_index_columns(
    connection: &Connection,
    index: &str,
    expected: &[&str],
) -> Result<(), BackendError> {
    let sql = format!("PRAGMA main.index_xinfo('{index}')");
    let mut statement = connection
        .prepare(&sql)
        .map_err(classify_inspection_error)?;
    let actual = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(classify_inspection_error)?
        .filter_map(|row| match row {
            Ok(value) if value.4 == 1 => Some(Ok(value)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_inspection_error)?;
    let expected = expected
        .iter()
        .enumerate()
        .map(|(ordinal, name)| {
            (
                ordinal as i64,
                Some((*name).to_owned()),
                0,
                "BINARY".to_owned(),
                1,
            )
        })
        .collect::<Vec<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

fn verify_relationship_foreign_keys(connection: &Connection) -> Result<(), BackendError> {
    let mut statement = connection
        .prepare("PRAGMA main.foreign_key_list('intent_unit_relationships')")
        .map_err(classify_inspection_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .map_err(classify_inspection_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_inspection_error)?;
    if rows.len() != 4 {
        return Err(BackendError::CorruptSchema);
    }
    let normalized = rows
        .iter()
        .map(|(_, _, table, from, to, update, delete, match_kind)| {
            (
                table.as_str(),
                from.as_str(),
                to.as_str(),
                update.as_str(),
                delete.as_str(),
                match_kind.as_str(),
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    let expected = std::collections::BTreeSet::from([
        (
            "relationship_definitions",
            "definition_id",
            "definition_id",
            "RESTRICT",
            "RESTRICT",
            "NONE",
        ),
        (
            "relationship_definitions",
            "definition_version",
            "definition_version",
            "RESTRICT",
            "RESTRICT",
            "NONE",
        ),
        (
            "intent_units",
            "source_id",
            "id",
            "RESTRICT",
            "RESTRICT",
            "NONE",
        ),
        (
            "intent_units",
            "target_id",
            "id",
            "RESTRICT",
            "RESTRICT",
            "NONE",
        ),
    ]);
    if normalized != expected {
        return Err(BackendError::CorruptSchema);
    }
    let definition_rows = rows
        .iter()
        .filter(|row| row.2 == "relationship_definitions")
        .collect::<Vec<_>>();
    if definition_rows.len() != 2
        || definition_rows[0].0 != definition_rows[1].0
        || definition_rows.iter().map(|row| row.1).collect::<Vec<_>>() != [0, 1]
    {
        return Err(BackendError::CorruptSchema);
    }
    Ok(())
}

fn schema_objects(connection: &Connection) -> Result<Vec<SchemaObject>, BackendError> {
    let mut statement = connection
        .prepare(
            "SELECT type, name, tbl_name, sql
             FROM main.sqlite_schema
             ORDER BY name",
        )
        .map_err(classify_inspection_error)?;
    statement
        .query_map([], |row| {
            Ok(SchemaObject {
                object_type: row.get(0)?,
                name: row.get(1)?,
                table_name: row.get(2)?,
                sql: row.get(3)?,
            })
        })
        .map_err(classify_inspection_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_inspection_error)
}

fn expected_v1_schema_objects() -> Vec<SchemaObject> {
    object_map(false)
}

fn expected_v2_schema_objects() -> Vec<SchemaObject> {
    object_map(true)
}

fn object_map(include_relationships: bool) -> Vec<SchemaObject> {
    let mut definitions = BTreeMap::from([
        (
            "intent_units",
            ("table", "intent_units", Some(CREATE_INTENT_UNITS_SQL)),
        ),
        (
            "intent_units_by_workflow",
            (
                "index",
                "intent_units",
                Some(CREATE_INTENT_UNIT_INDEX_SQL[0]),
            ),
        ),
        (
            "intent_units_by_species",
            (
                "index",
                "intent_units",
                Some(CREATE_INTENT_UNIT_INDEX_SQL[1]),
            ),
        ),
        (
            "intent_units_by_phase",
            (
                "index",
                "intent_units",
                Some(CREATE_INTENT_UNIT_INDEX_SQL[2]),
            ),
        ),
        (
            "intent_units_by_status",
            (
                "index",
                "intent_units",
                Some(CREATE_INTENT_UNIT_INDEX_SQL[3]),
            ),
        ),
        (
            "sqlite_autoindex_intent_units_1",
            ("index", "intent_units", None),
        ),
    ]);
    if include_relationships {
        definitions.extend([
            (
                "relationship_definitions",
                (
                    "table",
                    "relationship_definitions",
                    Some(CREATE_RELATIONSHIP_DEFINITIONS_SQL),
                ),
            ),
            (
                "intent_unit_relationships",
                (
                    "table",
                    "intent_unit_relationships",
                    Some(CREATE_INTENT_UNIT_RELATIONSHIPS_SQL),
                ),
            ),
            (
                "sqlite_autoindex_relationship_definitions_1",
                ("index", "relationship_definitions", None),
            ),
            (
                "sqlite_autoindex_intent_unit_relationships_1",
                ("index", "intent_unit_relationships", None),
            ),
            (
                "relationship_edges_by_source",
                (
                    "index",
                    "intent_unit_relationships",
                    Some(CREATE_RELATIONSHIP_INDEX_SQL[0]),
                ),
            ),
            (
                "relationship_edges_by_target",
                (
                    "index",
                    "intent_unit_relationships",
                    Some(CREATE_RELATIONSHIP_INDEX_SQL[1]),
                ),
            ),
        ]);
    }
    definitions
        .into_iter()
        .map(|(name, (object_type, table_name, sql))| SchemaObject {
            object_type: object_type.to_owned(),
            name: name.to_owned(),
            table_name: table_name.to_owned(),
            sql: sql.map(str::to_owned),
        })
        .collect()
}

fn classify_inspection_error(error: rusqlite::Error) -> BackendError {
    if crate::sqlite::is_corrupt_database_error(&error)
        || matches!(
            error,
            rusqlite::Error::FromSqlConversionFailure(..)
                | rusqlite::Error::IntegralValueOutOfRange(..)
                | rusqlite::Error::Utf8Error(..)
                | rusqlite::Error::InvalidColumnType(..)
                | rusqlite::Error::QueryReturnedNoRows
        )
    {
        BackendError::CorruptSchema
    } else {
        crate::sqlite::classify_runtime_error(error)
    }
}
