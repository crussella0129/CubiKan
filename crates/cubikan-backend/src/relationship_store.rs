use std::str;

use cubikan_core::{IntentSpecies, IntentUnit, IntentUnitId};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params, types::ValueRef};

use crate::{
    BackendError, CreateRelationship, CreateRelationshipDefinition, DeleteRelationship,
    RelationshipDefinitionId, RelationshipDefinitionKey, RelationshipDefinitionVersion,
    RelationshipDefinitionView, RelationshipDirection, RelationshipEndpoint, RelationshipError,
    RelationshipIdentity, RelationshipPolicy, RelationshipView,
    sqlite::{classify_runtime_error, load_validated_unit},
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

const SELECT_RELATIONSHIP_SQL: &str = "SELECT
    definition_id,
    definition_version,
    source_id,
    target_id
 FROM intent_unit_relationships
 WHERE definition_id = ?1 COLLATE BINARY
   AND definition_version = ?2
   AND source_id = ?3 COLLATE BINARY
   AND target_id = ?4 COLLATE BINARY";

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

pub(crate) fn create_relationship(
    connection: &mut Connection,
    command: CreateRelationship,
) -> Result<RelationshipView, RelationshipError> {
    let relationship = command.relationship().clone();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(classify_runtime_error)?;

    let definition = load_definition(&transaction, relationship.definition())?;
    let source = load_endpoint(
        &transaction,
        RelationshipEndpoint::Source,
        relationship.source(),
    )?;
    let target = load_endpoint(
        &transaction,
        RelationshipEndpoint::Target,
        relationship.target(),
    )?;
    validate_endpoint_species(&definition, RelationshipEndpoint::Source, &source)?;
    validate_endpoint_species(&definition, RelationshipEndpoint::Target, &target)?;

    let is_self = relationship.source() == relationship.target();
    if is_self && definition.self_policy() == RelationshipPolicy::Reject {
        return Err(RelationshipError::SelfEdgeRejected { relationship });
    }

    if let Some(stored) = select_relationship(&transaction, &relationship)? {
        stored.into_view(relationship.definition())?;
        return Err(RelationshipError::DuplicateRelationship { relationship });
    }

    if !is_self && definition.cycle_policy() == RelationshipPolicy::Reject {
        validate_reachability(&transaction, &relationship)?;
    }

    let encoded_version = encode_version(relationship.definition().version());
    let changed = transaction
        .execute(
            "INSERT INTO intent_unit_relationships (
                definition_id,
                definition_version,
                source_id,
                target_id
             ) VALUES (?1, ?2, ?3, ?4)",
            params![
                relationship.definition().id().as_str(),
                encoded_version,
                relationship.source().to_string(),
                relationship.target().to_string(),
            ],
        )
        .map_err(classify_runtime_error)?;
    if changed != 1 {
        return Err(RelationshipError::Backend(
            BackendError::ConcurrentStorageChange,
        ));
    }

    transaction.commit().map_err(classify_runtime_error)?;
    Ok(RelationshipView::new(relationship))
}

pub(crate) fn delete_relationship(
    connection: &mut Connection,
    command: DeleteRelationship,
) -> Result<RelationshipView, RelationshipError> {
    let relationship = command.relationship().clone();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(classify_runtime_error)?;

    let definition = load_definition(&transaction, relationship.definition())?;
    let source = load_endpoint(
        &transaction,
        RelationshipEndpoint::Source,
        relationship.source(),
    )?;
    let target = load_endpoint(
        &transaction,
        RelationshipEndpoint::Target,
        relationship.target(),
    )?;
    validate_endpoint_species(&definition, RelationshipEndpoint::Source, &source)?;
    validate_endpoint_species(&definition, RelationshipEndpoint::Target, &target)?;

    let Some(stored) = select_relationship(&transaction, &relationship)? else {
        return Err(RelationshipError::RelationshipNotFound { relationship });
    };
    stored.into_view(relationship.definition())?;

    let encoded_version = encode_version(relationship.definition().version());
    let changed = transaction
        .execute(
            "DELETE FROM intent_unit_relationships
             WHERE definition_id = ?1 COLLATE BINARY
               AND definition_version = ?2
               AND source_id = ?3 COLLATE BINARY
               AND target_id = ?4 COLLATE BINARY",
            params![
                relationship.definition().id().as_str(),
                encoded_version,
                relationship.source().to_string(),
                relationship.target().to_string(),
            ],
        )
        .map_err(classify_runtime_error)?;
    if changed != 1 {
        return Err(RelationshipError::Backend(
            BackendError::ConcurrentStorageChange,
        ));
    }

    transaction.commit().map_err(classify_runtime_error)?;
    Ok(RelationshipView::new(relationship))
}

fn load_definition(
    connection: &Connection,
    key: &RelationshipDefinitionKey,
) -> Result<RelationshipDefinitionView, RelationshipError> {
    let encoded_version = encode_version(key.version());
    let stored = select_definition(connection, key, &encoded_version)?.ok_or_else(|| {
        RelationshipError::DefinitionNotFound {
            definition: key.clone(),
        }
    })?;
    stored.into_view(key)
}

fn load_endpoint(
    connection: &Connection,
    endpoint: RelationshipEndpoint,
    id: IntentUnitId,
) -> Result<IntentUnit, RelationshipError> {
    match load_validated_unit(connection, id) {
        Ok(unit) => Ok(unit),
        Err(BackendError::IntentUnitNotFound { .. }) => {
            Err(RelationshipError::EndpointNotFound { endpoint, id })
        }
        Err(
            source @ (BackendError::UnsupportedEnvelopeVersion { .. }
            | BackendError::CorruptEnvelope
            | BackendError::ProjectionMismatch),
        ) => Err(RelationshipError::EndpointCorrupt {
            endpoint,
            id,
            source,
        }),
        Err(error) => Err(RelationshipError::Backend(error)),
    }
}

fn validate_endpoint_species(
    definition: &RelationshipDefinitionView,
    endpoint: RelationshipEndpoint,
    unit: &IntentUnit,
) -> Result<(), RelationshipError> {
    let expected = match endpoint {
        RelationshipEndpoint::Source => definition.source_species(),
        RelationshipEndpoint::Target => definition.target_species(),
    };
    if let Some(expected) = expected
        && expected != unit.species()
    {
        return Err(RelationshipError::EndpointSpeciesMismatch {
            endpoint,
            id: unit.id(),
            expected: expected.clone(),
            actual: unit.species().clone(),
        });
    }
    Ok(())
}

fn select_relationship(
    connection: &Connection,
    relationship: &RelationshipIdentity,
) -> Result<Option<StoredRelationship>, RelationshipError> {
    let encoded_version = encode_version(relationship.definition().version());
    connection
        .query_row(
            SELECT_RELATIONSHIP_SQL,
            params![
                relationship.definition().id().as_str(),
                encoded_version,
                relationship.source().to_string(),
                relationship.target().to_string(),
            ],
            StoredRelationship::from_row,
        )
        .optional()
        .map_err(classify_runtime_error)
        .map_err(RelationshipError::from)
}

fn validate_reachability(
    connection: &Connection,
    proposed: &RelationshipIdentity,
) -> Result<(), RelationshipError> {
    let encoded_version = encode_version(proposed.definition().version());
    let mut statement = connection
        .prepare(
            "WITH RECURSIVE reachable(node) AS (
                VALUES (?3)
                UNION
                SELECT edge.target_id
                FROM intent_unit_relationships AS edge
                JOIN reachable
                  ON edge.source_id = reachable.node COLLATE BINARY
                WHERE edge.definition_id = ?1 COLLATE BINARY
                  AND edge.definition_version = ?2
             )
             SELECT
                edge.definition_id,
                edge.definition_version,
                edge.source_id,
                edge.target_id
             FROM intent_unit_relationships AS edge
             JOIN reachable
               ON edge.source_id = reachable.node COLLATE BINARY
             WHERE edge.definition_id = ?1 COLLATE BINARY
               AND edge.definition_version = ?2
             ORDER BY edge.source_id COLLATE BINARY, edge.target_id COLLATE BINARY",
        )
        .map_err(classify_runtime_error)?;
    let rows = statement
        .query_map(
            params![
                proposed.definition().id().as_str(),
                encoded_version,
                proposed.target().to_string(),
            ],
            StoredRelationship::from_row,
        )
        .map_err(classify_runtime_error)?;

    let mut closes_cycle = false;
    for row in rows {
        let view = row
            .map_err(classify_runtime_error)?
            .into_view(proposed.definition())?;
        if view.relationship().target() == proposed.source() {
            closes_cycle = true;
        }
    }
    if closes_cycle {
        return Err(RelationshipError::CycleRejected {
            relationship: proposed.clone(),
        });
    }
    Ok(())
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
struct StoredRelationship {
    definition_id: StoredValue,
    definition_version: StoredValue,
    source_id: StoredValue,
    target_id: StoredValue,
}

impl StoredRelationship {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            definition_id: StoredValue::from_ref(row.get_ref(0)?),
            definition_version: StoredValue::from_ref(row.get_ref(1)?),
            source_id: StoredValue::from_ref(row.get_ref(2)?),
            target_id: StoredValue::from_ref(row.get_ref(3)?),
        })
    }

    fn into_view(
        self,
        expected_definition: &RelationshipDefinitionKey,
    ) -> Result<RelationshipView, RelationshipError> {
        let definition_id = decode_text(self.definition_id)
            .and_then(|value| RelationshipDefinitionId::new(value).ok());
        let definition_version = decode_blob(self.definition_version)
            .and_then(|value| <[u8; 8]>::try_from(value).ok())
            .and_then(|value| RelationshipDefinitionVersion::new(u64::from_be_bytes(value)).ok());
        let source = decode_unit_id(self.source_id);
        let target = decode_unit_id(self.target_id);
        let Some((definition_id, definition_version, source, target)) = definition_id
            .zip(definition_version)
            .zip(source)
            .zip(target)
            .map(|(((id, version), source), target)| (id, version, source, target))
        else {
            return Err(RelationshipError::CorruptRelationship {
                definition: expected_definition.clone(),
            });
        };
        let definition = RelationshipDefinitionKey::new(definition_id, definition_version);
        if &definition != expected_definition {
            return Err(RelationshipError::CorruptRelationship {
                definition: expected_definition.clone(),
            });
        }
        Ok(RelationshipView::new(RelationshipIdentity::new(
            definition, source, target,
        )))
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

fn decode_unit_id(value: StoredValue) -> Option<IntentUnitId> {
    let value = decode_text(value)?;
    let id = value.parse::<IntentUnitId>().ok()?;
    (id.to_string() == value).then_some(id)
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
