use std::collections::BTreeMap;

use rusqlite::Connection;

use crate::BackendError;

pub(crate) const SCHEMA_VERSION: i64 = 1;

pub(crate) const CREATE_TABLE_SQL: &str = r#"CREATE TABLE intent_units (
    id TEXT NOT NULL PRIMARY KEY COLLATE BINARY,
    envelope_version INTEGER NOT NULL CHECK(envelope_version = 1),
    envelope TEXT NOT NULL,
    workflow_id TEXT NOT NULL COLLATE BINARY,
    species TEXT NOT NULL COLLATE BINARY,
    phase TEXT NOT NULL COLLATE BINARY,
    status TEXT NOT NULL COLLATE BINARY CHECK(status IN ('active','completed')),
    revision BLOB NOT NULL CHECK(length(revision) = 8)
) STRICT"#;

pub(crate) const CREATE_INDEX_SQL: [&str; 4] = [
    "CREATE INDEX intent_units_by_workflow ON intent_units(workflow_id,id)",
    "CREATE INDEX intent_units_by_species ON intent_units(species,id)",
    "CREATE INDEX intent_units_by_phase ON intent_units(phase,id)",
    "CREATE INDEX intent_units_by_status ON intent_units(status,id)",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Ownership {
    Empty,
    OwnedV1,
}

#[derive(Debug, Eq, PartialEq)]
struct SchemaObject {
    object_type: String,
    name: String,
    table_name: String,
    sql: Option<String>,
}

pub(crate) fn inspect(connection: &Connection) -> Result<Ownership, BackendError> {
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(classify_inspection_error)?;
    if version != 0 && version != SCHEMA_VERSION {
        return Err(BackendError::UnsupportedSchemaVersion { found: version });
    }
    let objects = schema_objects(connection)?;

    match version {
        0 if objects.is_empty() => {
            verify_physical_integrity(connection)?;
            Ok(Ownership::Empty)
        }
        0 => Err(BackendError::UnownedDatabase),
        SCHEMA_VERSION if objects == expected_schema_objects() => {
            verify_physical_integrity(connection)?;
            Ok(Ownership::OwnedV1)
        }
        SCHEMA_VERSION => Err(BackendError::CorruptSchema),
        _ => unreachable!("schema versions were classified before object inspection"),
    }
}

fn verify_physical_integrity(connection: &Connection) -> Result<(), BackendError> {
    let mut statement = connection
        .prepare("PRAGMA main.integrity_check")
        .map_err(classify_inspection_error)?;
    let diagnostics = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(classify_inspection_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_inspection_error)?;
    if diagnostics == ["ok"] {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

pub(crate) fn initialize(connection: &Connection) -> Result<(), BackendError> {
    connection
        .execute(CREATE_TABLE_SQL, [])
        .map_err(crate::sqlite::classify_runtime_error)?;
    for statement in CREATE_INDEX_SQL {
        connection
            .execute(statement, [])
            .map_err(crate::sqlite::classify_runtime_error)?;
    }
    connection
        .pragma_update(None, "user_version", SCHEMA_VERSION)
        .map_err(crate::sqlite::classify_runtime_error)?;
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
    let rows = statement
        .query_map([], |row| {
            Ok(SchemaObject {
                object_type: row.get(0)?,
                name: row.get(1)?,
                table_name: row.get(2)?,
                sql: row.get(3)?,
            })
        })
        .map_err(classify_inspection_error)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(classify_inspection_error)
}

fn expected_schema_objects() -> Vec<SchemaObject> {
    let definitions = BTreeMap::from([
        (
            "intent_units",
            ("table", "intent_units", Some(CREATE_TABLE_SQL)),
        ),
        (
            "intent_units_by_workflow",
            ("index", "intent_units", Some(CREATE_INDEX_SQL[0])),
        ),
        (
            "intent_units_by_species",
            ("index", "intent_units", Some(CREATE_INDEX_SQL[1])),
        ),
        (
            "intent_units_by_phase",
            ("index", "intent_units", Some(CREATE_INDEX_SQL[2])),
        ),
        (
            "intent_units_by_status",
            ("index", "intent_units", Some(CREATE_INDEX_SQL[3])),
        ),
        (
            "sqlite_autoindex_intent_units_1",
            ("index", "intent_units", None),
        ),
    ]);

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
