use std::str;

use cubikan_core::IntentSpecies;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params, types::ValueRef};

use crate::{
    BackendError, CreateRelationshipDefinition, RelationshipDefinitionId,
    RelationshipDefinitionKey, RelationshipDefinitionVersion, RelationshipDefinitionView,
    RelationshipDirection, RelationshipError, RelationshipPolicy, sqlite::classify_runtime_error,
};

const SELECT_DEFINITION_SQL: &str = "SELECT
    definition_id,
    definition_version,
    directed,
    source_species,
    target_species,
    self_policy,
    cycle_policy
 FROM relationship_definitions
 WHERE definition_id = ?1 COLLATE BINARY AND definition_version = ?2";

pub(crate) fn create_definition(
    connection: &mut Connection,
    command: CreateRelationshipDefinition,
) -> Result<RelationshipDefinitionView, RelationshipError> {
    let key = command.key().clone();
    let encoded_version = encode_version(key.version());
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(classify_runtime_error)?;

    if let Some(stored) = select_definition(&transaction, &key, &encoded_version)? {
        // A collision is not a valid duplicate until the selected durable value
        // has decoded as the exact typed definition named by the caller.
        stored.into_view(&key)?;
        return Err(RelationshipError::DefinitionAlreadyExists { definition: key });
    }

    let view = definition_view(&command);
    let changed = transaction
        .execute(
            "INSERT INTO relationship_definitions (
                definition_id,
                definition_version,
                directed,
                source_species,
                target_species,
                self_policy,
                cycle_policy
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                key.id().as_str(),
                encoded_version,
                encode_direction(command.direction()),
                command.source_species().map(IntentSpecies::as_str),
                command.target_species().map(IntentSpecies::as_str),
                encode_policy(command.self_policy()),
                encode_policy(command.cycle_policy()),
            ],
        )
        .map_err(classify_runtime_error)?;
    if changed != 1 {
        return Err(RelationshipError::Backend(
            BackendError::ConcurrentStorageChange,
        ));
    }

    transaction.commit().map_err(classify_runtime_error)?;
    Ok(view)
}

pub(crate) fn get_definition(
    connection: &Connection,
    key: RelationshipDefinitionKey,
) -> Result<RelationshipDefinitionView, RelationshipError> {
    let encoded_version = encode_version(key.version());
    let stored = select_definition(connection, &key, &encoded_version)?.ok_or_else(|| {
        RelationshipError::DefinitionNotFound {
            definition: key.clone(),
        }
    })?;
    stored.into_view(&key)
}

fn select_definition(
    connection: &Connection,
    key: &RelationshipDefinitionKey,
    encoded_version: &[u8; 8],
) -> Result<Option<StoredDefinition>, RelationshipError> {
    connection
        .query_row(
            SELECT_DEFINITION_SQL,
            params![key.id().as_str(), encoded_version],
            StoredDefinition::from_row,
        )
        .optional()
        .map_err(classify_runtime_error)
        .map_err(RelationshipError::from)
}

fn definition_view(command: &CreateRelationshipDefinition) -> RelationshipDefinitionView {
    RelationshipDefinitionView::new(
        command.key().clone(),
        command.direction(),
        command.source_species().cloned(),
        command.target_species().cloned(),
        command.self_policy(),
        command.cycle_policy(),
    )
}

const fn encode_version(version: RelationshipDefinitionVersion) -> [u8; 8] {
    version.value().to_be_bytes()
}

const fn encode_direction(direction: RelationshipDirection) -> i64 {
    match direction {
        RelationshipDirection::Directed => 1,
    }
}

const fn encode_policy(policy: RelationshipPolicy) -> &'static str {
    match policy {
        RelationshipPolicy::Allow => "allow",
        RelationshipPolicy::Reject => "reject",
    }
}

#[derive(Debug)]
struct StoredDefinition {
    definition_id: StoredValue,
    definition_version: StoredValue,
    directed: StoredValue,
    source_species: StoredValue,
    target_species: StoredValue,
    self_policy: StoredValue,
    cycle_policy: StoredValue,
}

impl StoredDefinition {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            definition_id: StoredValue::from_ref(row.get_ref(0)?),
            definition_version: StoredValue::from_ref(row.get_ref(1)?),
            directed: StoredValue::from_ref(row.get_ref(2)?),
            source_species: StoredValue::from_ref(row.get_ref(3)?),
            target_species: StoredValue::from_ref(row.get_ref(4)?),
            self_policy: StoredValue::from_ref(row.get_ref(5)?),
            cycle_policy: StoredValue::from_ref(row.get_ref(6)?),
        })
    }

    fn into_view(
        self,
        expected: &RelationshipDefinitionKey,
    ) -> Result<RelationshipDefinitionView, RelationshipError> {
        let result = self.decode();
        match result {
            Some(view) if view.key() == expected => Ok(view),
            Some(_) | None => Err(RelationshipError::CorruptDefinition {
                definition: expected.clone(),
            }),
        }
    }

    fn decode(self) -> Option<RelationshipDefinitionView> {
        let definition_id = decode_text(self.definition_id)?;
        let definition_id = RelationshipDefinitionId::new(definition_id).ok()?;
        let definition_version = decode_blob(self.definition_version)?;
        let definition_version = <[u8; 8]>::try_from(definition_version).ok()?;
        let definition_version =
            RelationshipDefinitionVersion::new(u64::from_be_bytes(definition_version)).ok()?;
        let key = RelationshipDefinitionKey::new(definition_id, definition_version);

        let direction = match self.directed {
            StoredValue::Integer(1) => RelationshipDirection::Directed,
            _ => return None,
        };
        let source_species = decode_species(self.source_species)?;
        let target_species = decode_species(self.target_species)?;
        let self_policy = decode_policy(self.self_policy)?;
        let cycle_policy = decode_policy(self.cycle_policy)?;

        Some(RelationshipDefinitionView::new(
            key,
            direction,
            source_species,
            target_species,
            self_policy,
            cycle_policy,
        ))
    }
}

#[derive(Debug)]
enum StoredValue {
    Null,
    Integer(i64),
    Real,
    Text(Vec<u8>),
    Blob(Vec<u8>),
}

impl StoredValue {
    fn from_ref(value: ValueRef<'_>) -> Self {
        match value {
            ValueRef::Null => Self::Null,
            ValueRef::Integer(value) => Self::Integer(value),
            ValueRef::Real(_) => Self::Real,
            ValueRef::Text(value) => Self::Text(value.to_vec()),
            ValueRef::Blob(value) => Self::Blob(value.to_vec()),
        }
    }
}

fn decode_text(value: StoredValue) -> Option<String> {
    let StoredValue::Text(bytes) = value else {
        return None;
    };
    str::from_utf8(&bytes).ok().map(str::to_owned)
}

fn decode_blob(value: StoredValue) -> Option<Vec<u8>> {
    let StoredValue::Blob(bytes) = value else {
        return None;
    };
    Some(bytes)
}

fn decode_species(value: StoredValue) -> Option<Option<IntentSpecies>> {
    match value {
        StoredValue::Null => Some(None),
        value => IntentSpecies::new(decode_text(value)?).ok().map(Some),
    }
}

fn decode_policy(value: StoredValue) -> Option<RelationshipPolicy> {
    match value {
        StoredValue::Text(value) if value == b"allow" => Some(RelationshipPolicy::Allow),
        StoredValue::Text(value) if value == b"reject" => Some(RelationshipPolicy::Reject),
        _ => None,
    }
}
