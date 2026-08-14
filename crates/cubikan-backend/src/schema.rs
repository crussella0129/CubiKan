use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;

use crate::BackendError;

pub(crate) const SCHEMA_VERSION: i64 = 3;

pub(crate) const APPLICATION_TABLES: [&str; 8] = [
    "projection_anchor",
    "projected_blocks",
    "projected_events",
    "projection_checkpoint",
    "intent_units",
    "relationship_definitions",
    "intent_unit_relationships",
    "recorded_associations",
];

pub(crate) const NAMED_INDEXES: [&str; 11] = [
    "projected_blocks_by_hash",
    "projected_blocks_by_number_hash",
    "projected_events_by_sequence",
    "intent_units_by_workflow",
    "intent_units_by_species",
    "intent_units_by_phase",
    "intent_units_by_status",
    "relationship_edges_by_source",
    "relationship_edges_by_target",
    "recorded_associations_by_unit",
    "recorded_associations_by_reference",
];

pub(crate) const AUTO_INDEXES: [&str; 6] = [
    "sqlite_autoindex_projected_blocks_1",
    "sqlite_autoindex_projected_events_1",
    "sqlite_autoindex_intent_units_1",
    "sqlite_autoindex_relationship_definitions_1",
    "sqlite_autoindex_intent_unit_relationships_1",
    "sqlite_autoindex_recorded_associations_1",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SchemaTable {
    ProjectionAnchor,
    ProjectedBlocks,
    ProjectedEvents,
    ProjectionCheckpoint,
    IntentUnits,
    RelationshipDefinitions,
    IntentUnitRelationships,
    RecordedAssociations,
}

impl SchemaTable {
    pub(crate) const ALL: [Self; 8] = [
        Self::ProjectionAnchor,
        Self::ProjectedBlocks,
        Self::ProjectedEvents,
        Self::ProjectionCheckpoint,
        Self::IntentUnits,
        Self::RelationshipDefinitions,
        Self::IntentUnitRelationships,
        Self::RecordedAssociations,
    ];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::ProjectionAnchor => "projection_anchor",
            Self::ProjectedBlocks => "projected_blocks",
            Self::ProjectedEvents => "projected_events",
            Self::ProjectionCheckpoint => "projection_checkpoint",
            Self::IntentUnits => "intent_units",
            Self::RelationshipDefinitions => "relationship_definitions",
            Self::IntentUnitRelationships => "intent_unit_relationships",
            Self::RecordedAssociations => "recorded_associations",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NamedSchemaIndex {
    ProjectedBlocksByHash,
    ProjectedBlocksByNumberHash,
    ProjectedEventsBySequence,
    IntentUnitsByWorkflow,
    IntentUnitsBySpecies,
    IntentUnitsByPhase,
    IntentUnitsByStatus,
    RelationshipEdgesBySource,
    RelationshipEdgesByTarget,
    RecordedAssociationsByUnit,
    RecordedAssociationsByReference,
}

impl NamedSchemaIndex {
    pub(crate) const ALL: [Self; 11] = [
        Self::ProjectedBlocksByHash,
        Self::ProjectedBlocksByNumberHash,
        Self::ProjectedEventsBySequence,
        Self::IntentUnitsByWorkflow,
        Self::IntentUnitsBySpecies,
        Self::IntentUnitsByPhase,
        Self::IntentUnitsByStatus,
        Self::RelationshipEdgesBySource,
        Self::RelationshipEdgesByTarget,
        Self::RecordedAssociationsByUnit,
        Self::RecordedAssociationsByReference,
    ];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::ProjectedBlocksByHash => "projected_blocks_by_hash",
            Self::ProjectedBlocksByNumberHash => "projected_blocks_by_number_hash",
            Self::ProjectedEventsBySequence => "projected_events_by_sequence",
            Self::IntentUnitsByWorkflow => "intent_units_by_workflow",
            Self::IntentUnitsBySpecies => "intent_units_by_species",
            Self::IntentUnitsByPhase => "intent_units_by_phase",
            Self::IntentUnitsByStatus => "intent_units_by_status",
            Self::RelationshipEdgesBySource => "relationship_edges_by_source",
            Self::RelationshipEdgesByTarget => "relationship_edges_by_target",
            Self::RecordedAssociationsByUnit => "recorded_associations_by_unit",
            Self::RecordedAssociationsByReference => "recorded_associations_by_reference",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SchemaIndex {
    ProjectedBlocksPrimaryKey,
    ProjectedEventsPrimaryKey,
    IntentUnitsPrimaryKey,
    RelationshipDefinitionsPrimaryKey,
    IntentUnitRelationshipsPrimaryKey,
    RecordedAssociationsPrimaryKey,
    ProjectedBlocksByHash,
    ProjectedBlocksByNumberHash,
    ProjectedEventsBySequence,
    IntentUnitsByWorkflow,
    IntentUnitsBySpecies,
    IntentUnitsByPhase,
    IntentUnitsByStatus,
    RelationshipEdgesBySource,
    RelationshipEdgesByTarget,
    RecordedAssociationsByUnit,
    RecordedAssociationsByReference,
}

impl SchemaIndex {
    pub(crate) const ALL: [Self; 17] = [
        Self::ProjectedBlocksPrimaryKey,
        Self::ProjectedEventsPrimaryKey,
        Self::IntentUnitsPrimaryKey,
        Self::RelationshipDefinitionsPrimaryKey,
        Self::IntentUnitRelationshipsPrimaryKey,
        Self::RecordedAssociationsPrimaryKey,
        Self::ProjectedBlocksByHash,
        Self::ProjectedBlocksByNumberHash,
        Self::ProjectedEventsBySequence,
        Self::IntentUnitsByWorkflow,
        Self::IntentUnitsBySpecies,
        Self::IntentUnitsByPhase,
        Self::IntentUnitsByStatus,
        Self::RelationshipEdgesBySource,
        Self::RelationshipEdgesByTarget,
        Self::RecordedAssociationsByUnit,
        Self::RecordedAssociationsByReference,
    ];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::ProjectedBlocksPrimaryKey => "sqlite_autoindex_projected_blocks_1",
            Self::ProjectedEventsPrimaryKey => "sqlite_autoindex_projected_events_1",
            Self::IntentUnitsPrimaryKey => "sqlite_autoindex_intent_units_1",
            Self::RelationshipDefinitionsPrimaryKey => {
                "sqlite_autoindex_relationship_definitions_1"
            }
            Self::IntentUnitRelationshipsPrimaryKey => {
                "sqlite_autoindex_intent_unit_relationships_1"
            }
            Self::RecordedAssociationsPrimaryKey => "sqlite_autoindex_recorded_associations_1",
            Self::ProjectedBlocksByHash => "projected_blocks_by_hash",
            Self::ProjectedBlocksByNumberHash => "projected_blocks_by_number_hash",
            Self::ProjectedEventsBySequence => "projected_events_by_sequence",
            Self::IntentUnitsByWorkflow => "intent_units_by_workflow",
            Self::IntentUnitsBySpecies => "intent_units_by_species",
            Self::IntentUnitsByPhase => "intent_units_by_phase",
            Self::IntentUnitsByStatus => "intent_units_by_status",
            Self::RelationshipEdgesBySource => "relationship_edges_by_source",
            Self::RelationshipEdgesByTarget => "relationship_edges_by_target",
            Self::RecordedAssociationsByUnit => "recorded_associations_by_unit",
            Self::RecordedAssociationsByReference => "recorded_associations_by_reference",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SchemaStatement {
    CreateTable(SchemaTable),
    CreateIndex(NamedSchemaIndex),
    SetUserVersion,
    ReadUserVersion,
    SchemaObjects,
    TableList,
    TableInfo(SchemaTable),
    IndexList(SchemaTable),
    ForeignKeyList(SchemaTable),
    IndexInfo(SchemaIndex),
    IndexXinfo(SchemaIndex),
    IntegrityCheck,
    ForeignKeyCheck,
}

pub(crate) const CREATE_PROJECTION_ANCHOR_SQL: &str = r#"CREATE TABLE projection_anchor (
    singleton INTEGER NOT NULL PRIMARY KEY CHECK(typeof(singleton)='integer' AND singleton=1),
    namespace TEXT NOT NULL CHECK(typeof(namespace)='text' AND namespace='polkadot-sdk-parachain'),
    relay_genesis_hash BLOB NOT NULL CHECK(typeof(relay_genesis_hash)='blob' AND length(relay_genesis_hash)=32),
    parachain_genesis_hash BLOB NOT NULL CHECK(typeof(parachain_genesis_hash)='blob' AND length(parachain_genesis_hash)=32),
    para_id INTEGER NOT NULL CHECK(typeof(para_id)='integer' AND para_id=1000),
    deployment_id BLOB NOT NULL CHECK(typeof(deployment_id)='blob' AND length(deployment_id)=32),
    pallet_storage_version INTEGER NOT NULL CHECK(typeof(pallet_storage_version)='integer' AND pallet_storage_version=1),
    event_schema_version INTEGER NOT NULL CHECK(typeof(event_schema_version)='integer' AND event_schema_version=1),
    initial_runtime_spec_version INTEGER NOT NULL CHECK(typeof(initial_runtime_spec_version)='integer' AND initial_runtime_spec_version BETWEEN 0 AND 4294967295),
    initial_runtime_code_hash BLOB NOT NULL CHECK(typeof(initial_runtime_code_hash)='blob' AND length(initial_runtime_code_hash)=32)
) STRICT"#;

pub(crate) const CREATE_PROJECTED_BLOCKS_SQL: &str = r#"CREATE TABLE projected_blocks (
    anchor_singleton INTEGER NOT NULL CHECK(typeof(anchor_singleton)='integer' AND anchor_singleton=1),
    block_number BLOB NOT NULL PRIMARY KEY CHECK(typeof(block_number)='blob' AND length(block_number)=8),
    block_hash BLOB NOT NULL CHECK(typeof(block_hash)='blob' AND length(block_hash)=32),
    parent_hash BLOB NOT NULL CHECK(typeof(parent_hash)='blob' AND length(parent_hash)=32),
    runtime_spec_version INTEGER NOT NULL CHECK(typeof(runtime_spec_version)='integer' AND runtime_spec_version BETWEEN 0 AND 4294967295),
    runtime_code_hash BLOB NOT NULL CHECK(typeof(runtime_code_hash)='blob' AND length(runtime_code_hash)=32),
    cubikan_event_count INTEGER NOT NULL CHECK(typeof(cubikan_event_count)='integer' AND cubikan_event_count BETWEEN 0 AND 4294967295),
    first_global_sequence BLOB,
    last_global_sequence BLOB,
    FOREIGN KEY(anchor_singleton) REFERENCES projection_anchor(singleton) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT,
    CHECK((cubikan_event_count=0 AND first_global_sequence IS NULL AND last_global_sequence IS NULL) OR (cubikan_event_count>0 AND typeof(first_global_sequence)='blob' AND length(first_global_sequence)=8 AND first_global_sequence<>X'0000000000000000' AND typeof(last_global_sequence)='blob' AND length(last_global_sequence)=8 AND last_global_sequence<>X'0000000000000000' AND first_global_sequence<=last_global_sequence))
) STRICT"#;

pub(crate) const CREATE_PROJECTED_EVENTS_SQL: &str = r#"CREATE TABLE projected_events (
    block_number BLOB NOT NULL CHECK(typeof(block_number)='blob' AND length(block_number)=8),
    extrinsic_index INTEGER NOT NULL CHECK(typeof(extrinsic_index)='integer' AND extrinsic_index BETWEEN 0 AND 4294967295),
    system_event_index INTEGER NOT NULL CHECK(typeof(system_event_index)='integer' AND system_event_index BETWEEN 0 AND 4294967295),
    global_sequence BLOB NOT NULL CHECK(typeof(global_sequence)='blob' AND length(global_sequence)=8 AND global_sequence<>X'0000000000000000'),
    deployment_id BLOB NOT NULL CHECK(typeof(deployment_id)='blob' AND length(deployment_id)=32),
    event_schema_version INTEGER NOT NULL CHECK(typeof(event_schema_version)='integer' AND event_schema_version=1),
    event_kind TEXT NOT NULL CHECK(event_kind IN ('unit_created','unit_transitioned','unit_completed','relationship_definition_created','relationship_created','relationship_deleted','association_recorded','association_revoked')),
    scale_payload BLOB NOT NULL CHECK(typeof(scale_payload)='blob' AND length(scale_payload) BETWEEN 1 AND 1048576),
    signer BLOB NOT NULL CHECK(typeof(signer)='blob' AND length(signer)=32),
    extrinsic_hash BLOB NOT NULL CHECK(typeof(extrinsic_hash)='blob' AND length(extrinsic_hash)=32),
    PRIMARY KEY(block_number,extrinsic_index,system_event_index),
    FOREIGN KEY(block_number) REFERENCES projected_blocks(block_number) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT
) STRICT"#;

pub(crate) const CREATE_PROJECTION_CHECKPOINT_SQL: &str = r#"CREATE TABLE projection_checkpoint (
    singleton INTEGER NOT NULL PRIMARY KEY CHECK(typeof(singleton)='integer' AND singleton=1),
    block_number BLOB NOT NULL CHECK(typeof(block_number)='blob' AND length(block_number)=8),
    block_hash BLOB NOT NULL CHECK(typeof(block_hash)='blob' AND length(block_hash)=32),
    last_global_sequence BLOB CHECK(last_global_sequence IS NULL OR (typeof(last_global_sequence)='blob' AND length(last_global_sequence)=8 AND last_global_sequence<>X'0000000000000000')),
    runtime_spec_version INTEGER NOT NULL CHECK(typeof(runtime_spec_version)='integer' AND runtime_spec_version BETWEEN 0 AND 4294967295),
    runtime_code_hash BLOB NOT NULL CHECK(typeof(runtime_code_hash)='blob' AND length(runtime_code_hash)=32),
    FOREIGN KEY(block_number,block_hash) REFERENCES projected_blocks(block_number,block_hash) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(last_global_sequence) REFERENCES projected_events(global_sequence) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT
) STRICT"#;

pub(crate) const CREATE_INTENT_UNITS_SQL: &str = r#"CREATE TABLE intent_units (
    id TEXT NOT NULL PRIMARY KEY CHECK(typeof(id)='text' AND length(CAST(id AS BLOB))=36 AND instr(id,char(0))=0),
    envelope_version INTEGER NOT NULL CHECK(typeof(envelope_version)='integer' AND envelope_version=2),
    envelope TEXT NOT NULL CHECK(typeof(envelope)='text' AND length(CAST(envelope AS BLOB)) BETWEEN 1 AND 2097152),
    origin_namespace TEXT NOT NULL CHECK(typeof(origin_namespace)='text' AND length(CAST(origin_namespace AS BLOB)) BETWEEN 1 AND 64 AND instr(origin_namespace,char(0))=0 AND origin_namespace GLOB '[a-z]*' AND origin_namespace NOT GLOB '*[^a-z0-9._-]*'),
    origin_scope TEXT NOT NULL CHECK(typeof(origin_scope)='text' AND length(CAST(origin_scope AS BLOB)) BETWEEN 1 AND 256 AND instr(origin_scope,char(0))=0),
    origin_value TEXT NOT NULL CHECK(typeof(origin_value)='text' AND length(CAST(origin_value AS BLOB)) BETWEEN 1 AND 256 AND instr(origin_value,char(0))=0),
    workflow_id TEXT NOT NULL CHECK(typeof(workflow_id)='text' AND length(CAST(workflow_id AS BLOB)) BETWEEN 1 AND 256 AND instr(workflow_id,char(0))=0),
    species TEXT NOT NULL CHECK(typeof(species)='text' AND length(CAST(species AS BLOB)) BETWEEN 1 AND 256 AND instr(species,char(0))=0),
    phase TEXT NOT NULL CHECK(typeof(phase)='text' AND length(CAST(phase AS BLOB)) BETWEEN 1 AND 256 AND instr(phase,char(0))=0),
    status TEXT NOT NULL CHECK(typeof(status)='text' AND status IN ('active','completed')),
    revision BLOB NOT NULL CHECK(typeof(revision)='blob' AND length(revision)=8),
    last_global_sequence BLOB NOT NULL CHECK(typeof(last_global_sequence)='blob' AND length(last_global_sequence)=8 AND last_global_sequence<>X'0000000000000000') REFERENCES projected_events(global_sequence) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT
) STRICT"#;

pub(crate) const CREATE_RELATIONSHIP_DEFINITIONS_SQL: &str = r#"CREATE TABLE relationship_definitions (
    definition_id TEXT NOT NULL CHECK(typeof(definition_id)='text' AND length(CAST(definition_id AS BLOB)) BETWEEN 1 AND 64 AND instr(definition_id,char(0))=0 AND definition_id GLOB '[a-z]*' AND definition_id NOT GLOB '*[^a-z0-9._-]*'),
    definition_version BLOB NOT NULL CHECK(typeof(definition_version)='blob' AND length(definition_version)=8 AND definition_version<>X'0000000000000000'),
    directed INTEGER NOT NULL CHECK(typeof(directed)='integer' AND directed=1),
    source_species TEXT CHECK(source_species IS NULL OR (typeof(source_species)='text' AND length(CAST(source_species AS BLOB)) BETWEEN 1 AND 256 AND instr(source_species,char(0))=0)),
    target_species TEXT CHECK(target_species IS NULL OR (typeof(target_species)='text' AND length(CAST(target_species AS BLOB)) BETWEEN 1 AND 256 AND instr(target_species,char(0))=0)),
    self_policy TEXT NOT NULL CHECK(self_policy IN ('allow','reject')),
    cycle_policy TEXT NOT NULL CHECK(cycle_policy IN ('allow','reject')),
    created_global_sequence BLOB NOT NULL CHECK(typeof(created_global_sequence)='blob' AND length(created_global_sequence)=8 AND created_global_sequence<>X'0000000000000000') REFERENCES projected_events(global_sequence) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT,
    PRIMARY KEY(definition_id,definition_version)
) STRICT"#;

pub(crate) const CREATE_INTENT_UNIT_RELATIONSHIPS_SQL: &str = r#"CREATE TABLE intent_unit_relationships (
    definition_id TEXT NOT NULL,
    definition_version BLOB NOT NULL CHECK(typeof(definition_version)='blob' AND length(definition_version)=8 AND definition_version<>X'0000000000000000'),
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    created_global_sequence BLOB NOT NULL CHECK(typeof(created_global_sequence)='blob' AND length(created_global_sequence)=8 AND created_global_sequence<>X'0000000000000000') REFERENCES projected_events(global_sequence) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT,
    PRIMARY KEY(definition_id,definition_version,source_id,target_id),
    FOREIGN KEY(definition_id,definition_version) REFERENCES relationship_definitions(definition_id,definition_version) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(source_id) REFERENCES intent_units(id) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(target_id) REFERENCES intent_units(id) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT
) STRICT"#;

pub(crate) const CREATE_RECORDED_ASSOCIATIONS_SQL: &str = r#"CREATE TABLE recorded_associations (
    unit_id TEXT NOT NULL,
    subject_kind TEXT NOT NULL CHECK(subject_kind IN ('whole_unit','revision')),
    subject_revision_key BLOB NOT NULL CHECK((subject_kind='whole_unit' AND typeof(subject_revision_key)='blob' AND length(subject_revision_key)=0) OR (subject_kind='revision' AND typeof(subject_revision_key)='blob' AND length(subject_revision_key)=8)),
    namespace TEXT NOT NULL CHECK(typeof(namespace)='text' AND length(CAST(namespace AS BLOB)) BETWEEN 1 AND 64 AND instr(namespace,char(0))=0 AND namespace GLOB '[a-z]*' AND namespace NOT GLOB '*[^a-z0-9._-]*'),
    scope TEXT NOT NULL CHECK(typeof(scope)='text' AND length(CAST(scope AS BLOB)) BETWEEN 1 AND 256 AND instr(scope,char(0))=0),
    value TEXT NOT NULL CHECK(typeof(value)='text' AND length(CAST(value AS BLOB)) BETWEEN 1 AND 256 AND instr(value,char(0))=0),
    created_global_sequence BLOB NOT NULL CHECK(typeof(created_global_sequence)='blob' AND length(created_global_sequence)=8 AND created_global_sequence<>X'0000000000000000') REFERENCES projected_events(global_sequence) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT,
    PRIMARY KEY(unit_id,subject_kind,subject_revision_key,namespace,scope,value),
    FOREIGN KEY(unit_id) REFERENCES intent_units(id) MATCH NONE ON UPDATE RESTRICT ON DELETE RESTRICT
) STRICT"#;

pub(crate) const CREATE_TABLE_STATEMENTS: [(&str, &str); 8] = [
    ("projection_anchor", CREATE_PROJECTION_ANCHOR_SQL),
    ("projected_blocks", CREATE_PROJECTED_BLOCKS_SQL),
    ("projected_events", CREATE_PROJECTED_EVENTS_SQL),
    ("projection_checkpoint", CREATE_PROJECTION_CHECKPOINT_SQL),
    ("intent_units", CREATE_INTENT_UNITS_SQL),
    (
        "relationship_definitions",
        CREATE_RELATIONSHIP_DEFINITIONS_SQL,
    ),
    (
        "intent_unit_relationships",
        CREATE_INTENT_UNIT_RELATIONSHIPS_SQL,
    ),
    ("recorded_associations", CREATE_RECORDED_ASSOCIATIONS_SQL),
];

pub(crate) const CREATE_INDEX_STATEMENTS: [(&str, &str); 11] = [
    (
        "projected_blocks_by_hash",
        "CREATE UNIQUE INDEX projected_blocks_by_hash ON projected_blocks(block_hash)",
    ),
    (
        "projected_blocks_by_number_hash",
        "CREATE UNIQUE INDEX projected_blocks_by_number_hash ON projected_blocks(block_number,block_hash)",
    ),
    (
        "projected_events_by_sequence",
        "CREATE UNIQUE INDEX projected_events_by_sequence ON projected_events(global_sequence)",
    ),
    (
        "intent_units_by_workflow",
        "CREATE INDEX intent_units_by_workflow ON intent_units(workflow_id,id)",
    ),
    (
        "intent_units_by_species",
        "CREATE INDEX intent_units_by_species ON intent_units(species,id)",
    ),
    (
        "intent_units_by_phase",
        "CREATE INDEX intent_units_by_phase ON intent_units(phase,id)",
    ),
    (
        "intent_units_by_status",
        "CREATE INDEX intent_units_by_status ON intent_units(status,id)",
    ),
    (
        "relationship_edges_by_source",
        "CREATE INDEX relationship_edges_by_source ON intent_unit_relationships(definition_id,definition_version,source_id,target_id)",
    ),
    (
        "relationship_edges_by_target",
        "CREATE INDEX relationship_edges_by_target ON intent_unit_relationships(definition_id,definition_version,target_id,source_id)",
    ),
    (
        "recorded_associations_by_unit",
        "CREATE INDEX recorded_associations_by_unit ON recorded_associations(unit_id,subject_kind,subject_revision_key,namespace,scope,value)",
    ),
    (
        "recorded_associations_by_reference",
        "CREATE INDEX recorded_associations_by_reference ON recorded_associations(namespace,scope,value,unit_id,subject_kind,subject_revision_key)",
    ),
];

const SCHEMA_OBJECTS_SQL: &str =
    "SELECT type,name,tbl_name,rootpage,sql FROM main.sqlite_schema ORDER BY type,name";
const TABLE_LIST_SQL: &str = "SELECT schema,name,type,ncol,wr,strict FROM pragma_table_list WHERE schema='main' ORDER BY name";
const INTEGRITY_CHECK_SQL: &str = "SELECT integrity_check FROM pragma_integrity_check";
const FOREIGN_KEY_CHECK_SQL: &str =
    "SELECT \"table\",rowid,parent,fkid FROM pragma_foreign_key_check";

#[derive(Clone, Copy)]
struct ColumnSpec {
    name: &'static str,
    declared_type: &'static str,
    not_null: i64,
    primary_key_order: i64,
}

#[derive(Clone, Copy)]
struct TableSpec {
    name: &'static str,
    info_sql: &'static str,
    index_list_sql: &'static str,
    foreign_key_sql: &'static str,
    columns: &'static [ColumnSpec],
    foreign_keys: &'static [ForeignKeySpec],
}

#[derive(Clone, Copy)]
struct ForeignKeySpec {
    parent: &'static str,
    columns: &'static [(&'static str, &'static str)],
}

#[derive(Clone, Copy)]
struct IndexSpec {
    name: &'static str,
    table: &'static str,
    unique: i64,
    origin: &'static str,
    columns: &'static [&'static str],
    info_sql: &'static str,
    xinfo_sql: &'static str,
}

const fn column(
    name: &'static str,
    declared_type: &'static str,
    not_null: i64,
    primary_key_order: i64,
) -> ColumnSpec {
    ColumnSpec {
        name,
        declared_type,
        not_null,
        primary_key_order,
    }
}

const NO_FOREIGN_KEYS: &[ForeignKeySpec] = &[];
const PROJECTION_ANCHOR_COLUMNS: &[ColumnSpec] = &[
    column("singleton", "INTEGER", 1, 1),
    column("namespace", "TEXT", 1, 0),
    column("relay_genesis_hash", "BLOB", 1, 0),
    column("parachain_genesis_hash", "BLOB", 1, 0),
    column("para_id", "INTEGER", 1, 0),
    column("deployment_id", "BLOB", 1, 0),
    column("pallet_storage_version", "INTEGER", 1, 0),
    column("event_schema_version", "INTEGER", 1, 0),
    column("initial_runtime_spec_version", "INTEGER", 1, 0),
    column("initial_runtime_code_hash", "BLOB", 1, 0),
];
const PROJECTED_BLOCKS_COLUMNS: &[ColumnSpec] = &[
    column("anchor_singleton", "INTEGER", 1, 0),
    column("block_number", "BLOB", 1, 1),
    column("block_hash", "BLOB", 1, 0),
    column("parent_hash", "BLOB", 1, 0),
    column("runtime_spec_version", "INTEGER", 1, 0),
    column("runtime_code_hash", "BLOB", 1, 0),
    column("cubikan_event_count", "INTEGER", 1, 0),
    column("first_global_sequence", "BLOB", 0, 0),
    column("last_global_sequence", "BLOB", 0, 0),
];
const PROJECTED_EVENTS_COLUMNS: &[ColumnSpec] = &[
    column("block_number", "BLOB", 1, 1),
    column("extrinsic_index", "INTEGER", 1, 2),
    column("system_event_index", "INTEGER", 1, 3),
    column("global_sequence", "BLOB", 1, 0),
    column("deployment_id", "BLOB", 1, 0),
    column("event_schema_version", "INTEGER", 1, 0),
    column("event_kind", "TEXT", 1, 0),
    column("scale_payload", "BLOB", 1, 0),
    column("signer", "BLOB", 1, 0),
    column("extrinsic_hash", "BLOB", 1, 0),
];
const PROJECTION_CHECKPOINT_COLUMNS: &[ColumnSpec] = &[
    column("singleton", "INTEGER", 1, 1),
    column("block_number", "BLOB", 1, 0),
    column("block_hash", "BLOB", 1, 0),
    column("last_global_sequence", "BLOB", 0, 0),
    column("runtime_spec_version", "INTEGER", 1, 0),
    column("runtime_code_hash", "BLOB", 1, 0),
];
const INTENT_UNITS_COLUMNS: &[ColumnSpec] = &[
    column("id", "TEXT", 1, 1),
    column("envelope_version", "INTEGER", 1, 0),
    column("envelope", "TEXT", 1, 0),
    column("origin_namespace", "TEXT", 1, 0),
    column("origin_scope", "TEXT", 1, 0),
    column("origin_value", "TEXT", 1, 0),
    column("workflow_id", "TEXT", 1, 0),
    column("species", "TEXT", 1, 0),
    column("phase", "TEXT", 1, 0),
    column("status", "TEXT", 1, 0),
    column("revision", "BLOB", 1, 0),
    column("last_global_sequence", "BLOB", 1, 0),
];
const RELATIONSHIP_DEFINITIONS_COLUMNS: &[ColumnSpec] = &[
    column("definition_id", "TEXT", 1, 1),
    column("definition_version", "BLOB", 1, 2),
    column("directed", "INTEGER", 1, 0),
    column("source_species", "TEXT", 0, 0),
    column("target_species", "TEXT", 0, 0),
    column("self_policy", "TEXT", 1, 0),
    column("cycle_policy", "TEXT", 1, 0),
    column("created_global_sequence", "BLOB", 1, 0),
];
const INTENT_UNIT_RELATIONSHIPS_COLUMNS: &[ColumnSpec] = &[
    column("definition_id", "TEXT", 1, 1),
    column("definition_version", "BLOB", 1, 2),
    column("source_id", "TEXT", 1, 3),
    column("target_id", "TEXT", 1, 4),
    column("created_global_sequence", "BLOB", 1, 0),
];
const RECORDED_ASSOCIATIONS_COLUMNS: &[ColumnSpec] = &[
    column("unit_id", "TEXT", 1, 1),
    column("subject_kind", "TEXT", 1, 2),
    column("subject_revision_key", "BLOB", 1, 3),
    column("namespace", "TEXT", 1, 4),
    column("scope", "TEXT", 1, 5),
    column("value", "TEXT", 1, 6),
    column("created_global_sequence", "BLOB", 1, 0),
];

const PROJECTED_BLOCKS_FOREIGN_KEYS: &[ForeignKeySpec] = &[ForeignKeySpec {
    parent: "projection_anchor",
    columns: &[("anchor_singleton", "singleton")],
}];
const PROJECTED_EVENTS_FOREIGN_KEYS: &[ForeignKeySpec] = &[ForeignKeySpec {
    parent: "projected_blocks",
    columns: &[("block_number", "block_number")],
}];
const PROJECTION_CHECKPOINT_FOREIGN_KEYS: &[ForeignKeySpec] = &[
    ForeignKeySpec {
        parent: "projected_blocks",
        columns: &[
            ("block_number", "block_number"),
            ("block_hash", "block_hash"),
        ],
    },
    ForeignKeySpec {
        parent: "projected_events",
        columns: &[("last_global_sequence", "global_sequence")],
    },
];
const INTENT_UNITS_FOREIGN_KEYS: &[ForeignKeySpec] = &[ForeignKeySpec {
    parent: "projected_events",
    columns: &[("last_global_sequence", "global_sequence")],
}];
const RELATIONSHIP_DEFINITIONS_FOREIGN_KEYS: &[ForeignKeySpec] = &[ForeignKeySpec {
    parent: "projected_events",
    columns: &[("created_global_sequence", "global_sequence")],
}];
const INTENT_UNIT_RELATIONSHIPS_FOREIGN_KEYS: &[ForeignKeySpec] = &[
    ForeignKeySpec {
        parent: "projected_events",
        columns: &[("created_global_sequence", "global_sequence")],
    },
    ForeignKeySpec {
        parent: "relationship_definitions",
        columns: &[
            ("definition_id", "definition_id"),
            ("definition_version", "definition_version"),
        ],
    },
    ForeignKeySpec {
        parent: "intent_units",
        columns: &[("source_id", "id")],
    },
    ForeignKeySpec {
        parent: "intent_units",
        columns: &[("target_id", "id")],
    },
];
const RECORDED_ASSOCIATIONS_FOREIGN_KEYS: &[ForeignKeySpec] = &[
    ForeignKeySpec {
        parent: "projected_events",
        columns: &[("created_global_sequence", "global_sequence")],
    },
    ForeignKeySpec {
        parent: "intent_units",
        columns: &[("unit_id", "id")],
    },
];

const TABLE_SPECS: &[TableSpec] = &[
    TableSpec {
        name: "projection_anchor",
        info_sql: "SELECT cid,name,type,\"notnull\",dflt_value,pk FROM pragma_table_info('projection_anchor') ORDER BY cid",
        index_list_sql: "SELECT seq,name,\"unique\",origin,partial FROM pragma_index_list('projection_anchor') ORDER BY name",
        foreign_key_sql: "SELECT id,seq,\"table\",\"from\",\"to\",on_update,on_delete,\"match\" FROM pragma_foreign_key_list('projection_anchor') ORDER BY id,seq",
        columns: PROJECTION_ANCHOR_COLUMNS,
        foreign_keys: NO_FOREIGN_KEYS,
    },
    TableSpec {
        name: "projected_blocks",
        info_sql: "SELECT cid,name,type,\"notnull\",dflt_value,pk FROM pragma_table_info('projected_blocks') ORDER BY cid",
        index_list_sql: "SELECT seq,name,\"unique\",origin,partial FROM pragma_index_list('projected_blocks') ORDER BY name",
        foreign_key_sql: "SELECT id,seq,\"table\",\"from\",\"to\",on_update,on_delete,\"match\" FROM pragma_foreign_key_list('projected_blocks') ORDER BY id,seq",
        columns: PROJECTED_BLOCKS_COLUMNS,
        foreign_keys: PROJECTED_BLOCKS_FOREIGN_KEYS,
    },
    TableSpec {
        name: "projected_events",
        info_sql: "SELECT cid,name,type,\"notnull\",dflt_value,pk FROM pragma_table_info('projected_events') ORDER BY cid",
        index_list_sql: "SELECT seq,name,\"unique\",origin,partial FROM pragma_index_list('projected_events') ORDER BY name",
        foreign_key_sql: "SELECT id,seq,\"table\",\"from\",\"to\",on_update,on_delete,\"match\" FROM pragma_foreign_key_list('projected_events') ORDER BY id,seq",
        columns: PROJECTED_EVENTS_COLUMNS,
        foreign_keys: PROJECTED_EVENTS_FOREIGN_KEYS,
    },
    TableSpec {
        name: "projection_checkpoint",
        info_sql: "SELECT cid,name,type,\"notnull\",dflt_value,pk FROM pragma_table_info('projection_checkpoint') ORDER BY cid",
        index_list_sql: "SELECT seq,name,\"unique\",origin,partial FROM pragma_index_list('projection_checkpoint') ORDER BY name",
        foreign_key_sql: "SELECT id,seq,\"table\",\"from\",\"to\",on_update,on_delete,\"match\" FROM pragma_foreign_key_list('projection_checkpoint') ORDER BY id,seq",
        columns: PROJECTION_CHECKPOINT_COLUMNS,
        foreign_keys: PROJECTION_CHECKPOINT_FOREIGN_KEYS,
    },
    TableSpec {
        name: "intent_units",
        info_sql: "SELECT cid,name,type,\"notnull\",dflt_value,pk FROM pragma_table_info('intent_units') ORDER BY cid",
        index_list_sql: "SELECT seq,name,\"unique\",origin,partial FROM pragma_index_list('intent_units') ORDER BY name",
        foreign_key_sql: "SELECT id,seq,\"table\",\"from\",\"to\",on_update,on_delete,\"match\" FROM pragma_foreign_key_list('intent_units') ORDER BY id,seq",
        columns: INTENT_UNITS_COLUMNS,
        foreign_keys: INTENT_UNITS_FOREIGN_KEYS,
    },
    TableSpec {
        name: "relationship_definitions",
        info_sql: "SELECT cid,name,type,\"notnull\",dflt_value,pk FROM pragma_table_info('relationship_definitions') ORDER BY cid",
        index_list_sql: "SELECT seq,name,\"unique\",origin,partial FROM pragma_index_list('relationship_definitions') ORDER BY name",
        foreign_key_sql: "SELECT id,seq,\"table\",\"from\",\"to\",on_update,on_delete,\"match\" FROM pragma_foreign_key_list('relationship_definitions') ORDER BY id,seq",
        columns: RELATIONSHIP_DEFINITIONS_COLUMNS,
        foreign_keys: RELATIONSHIP_DEFINITIONS_FOREIGN_KEYS,
    },
    TableSpec {
        name: "intent_unit_relationships",
        info_sql: "SELECT cid,name,type,\"notnull\",dflt_value,pk FROM pragma_table_info('intent_unit_relationships') ORDER BY cid",
        index_list_sql: "SELECT seq,name,\"unique\",origin,partial FROM pragma_index_list('intent_unit_relationships') ORDER BY name",
        foreign_key_sql: "SELECT id,seq,\"table\",\"from\",\"to\",on_update,on_delete,\"match\" FROM pragma_foreign_key_list('intent_unit_relationships') ORDER BY id,seq",
        columns: INTENT_UNIT_RELATIONSHIPS_COLUMNS,
        foreign_keys: INTENT_UNIT_RELATIONSHIPS_FOREIGN_KEYS,
    },
    TableSpec {
        name: "recorded_associations",
        info_sql: "SELECT cid,name,type,\"notnull\",dflt_value,pk FROM pragma_table_info('recorded_associations') ORDER BY cid",
        index_list_sql: "SELECT seq,name,\"unique\",origin,partial FROM pragma_index_list('recorded_associations') ORDER BY name",
        foreign_key_sql: "SELECT id,seq,\"table\",\"from\",\"to\",on_update,on_delete,\"match\" FROM pragma_foreign_key_list('recorded_associations') ORDER BY id,seq",
        columns: RECORDED_ASSOCIATIONS_COLUMNS,
        foreign_keys: RECORDED_ASSOCIATIONS_FOREIGN_KEYS,
    },
];

const INDEX_SPECS: &[IndexSpec] = &[
    IndexSpec {
        name: "sqlite_autoindex_projected_blocks_1",
        table: "projected_blocks",
        unique: 1,
        origin: "pk",
        columns: &["block_number"],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('sqlite_autoindex_projected_blocks_1') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('sqlite_autoindex_projected_blocks_1') ORDER BY seqno",
    },
    IndexSpec {
        name: "sqlite_autoindex_projected_events_1",
        table: "projected_events",
        unique: 1,
        origin: "pk",
        columns: &["block_number", "extrinsic_index", "system_event_index"],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('sqlite_autoindex_projected_events_1') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('sqlite_autoindex_projected_events_1') ORDER BY seqno",
    },
    IndexSpec {
        name: "sqlite_autoindex_intent_units_1",
        table: "intent_units",
        unique: 1,
        origin: "pk",
        columns: &["id"],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('sqlite_autoindex_intent_units_1') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('sqlite_autoindex_intent_units_1') ORDER BY seqno",
    },
    IndexSpec {
        name: "sqlite_autoindex_relationship_definitions_1",
        table: "relationship_definitions",
        unique: 1,
        origin: "pk",
        columns: &["definition_id", "definition_version"],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('sqlite_autoindex_relationship_definitions_1') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('sqlite_autoindex_relationship_definitions_1') ORDER BY seqno",
    },
    IndexSpec {
        name: "sqlite_autoindex_intent_unit_relationships_1",
        table: "intent_unit_relationships",
        unique: 1,
        origin: "pk",
        columns: &[
            "definition_id",
            "definition_version",
            "source_id",
            "target_id",
        ],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('sqlite_autoindex_intent_unit_relationships_1') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('sqlite_autoindex_intent_unit_relationships_1') ORDER BY seqno",
    },
    IndexSpec {
        name: "sqlite_autoindex_recorded_associations_1",
        table: "recorded_associations",
        unique: 1,
        origin: "pk",
        columns: &[
            "unit_id",
            "subject_kind",
            "subject_revision_key",
            "namespace",
            "scope",
            "value",
        ],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('sqlite_autoindex_recorded_associations_1') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('sqlite_autoindex_recorded_associations_1') ORDER BY seqno",
    },
    IndexSpec {
        name: "projected_blocks_by_hash",
        table: "projected_blocks",
        unique: 1,
        origin: "c",
        columns: &["block_hash"],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('projected_blocks_by_hash') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('projected_blocks_by_hash') ORDER BY seqno",
    },
    IndexSpec {
        name: "projected_blocks_by_number_hash",
        table: "projected_blocks",
        unique: 1,
        origin: "c",
        columns: &["block_number", "block_hash"],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('projected_blocks_by_number_hash') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('projected_blocks_by_number_hash') ORDER BY seqno",
    },
    IndexSpec {
        name: "projected_events_by_sequence",
        table: "projected_events",
        unique: 1,
        origin: "c",
        columns: &["global_sequence"],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('projected_events_by_sequence') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('projected_events_by_sequence') ORDER BY seqno",
    },
    IndexSpec {
        name: "intent_units_by_workflow",
        table: "intent_units",
        unique: 0,
        origin: "c",
        columns: &["workflow_id", "id"],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('intent_units_by_workflow') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('intent_units_by_workflow') ORDER BY seqno",
    },
    IndexSpec {
        name: "intent_units_by_species",
        table: "intent_units",
        unique: 0,
        origin: "c",
        columns: &["species", "id"],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('intent_units_by_species') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('intent_units_by_species') ORDER BY seqno",
    },
    IndexSpec {
        name: "intent_units_by_phase",
        table: "intent_units",
        unique: 0,
        origin: "c",
        columns: &["phase", "id"],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('intent_units_by_phase') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('intent_units_by_phase') ORDER BY seqno",
    },
    IndexSpec {
        name: "intent_units_by_status",
        table: "intent_units",
        unique: 0,
        origin: "c",
        columns: &["status", "id"],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('intent_units_by_status') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('intent_units_by_status') ORDER BY seqno",
    },
    IndexSpec {
        name: "relationship_edges_by_source",
        table: "intent_unit_relationships",
        unique: 0,
        origin: "c",
        columns: &[
            "definition_id",
            "definition_version",
            "source_id",
            "target_id",
        ],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('relationship_edges_by_source') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('relationship_edges_by_source') ORDER BY seqno",
    },
    IndexSpec {
        name: "relationship_edges_by_target",
        table: "intent_unit_relationships",
        unique: 0,
        origin: "c",
        columns: &[
            "definition_id",
            "definition_version",
            "target_id",
            "source_id",
        ],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('relationship_edges_by_target') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('relationship_edges_by_target') ORDER BY seqno",
    },
    IndexSpec {
        name: "recorded_associations_by_unit",
        table: "recorded_associations",
        unique: 0,
        origin: "c",
        columns: &[
            "unit_id",
            "subject_kind",
            "subject_revision_key",
            "namespace",
            "scope",
            "value",
        ],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('recorded_associations_by_unit') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('recorded_associations_by_unit') ORDER BY seqno",
    },
    IndexSpec {
        name: "recorded_associations_by_reference",
        table: "recorded_associations",
        unique: 0,
        origin: "c",
        columns: &[
            "namespace",
            "scope",
            "value",
            "unit_id",
            "subject_kind",
            "subject_revision_key",
        ],
        info_sql: "SELECT seqno,cid,name FROM pragma_index_info('recorded_associations_by_reference') ORDER BY seqno",
        xinfo_sql: "SELECT seqno,cid,name,desc,coll,key FROM pragma_index_xinfo('recorded_associations_by_reference') ORDER BY seqno",
    },
];

pub(crate) fn user_version(connection: &Connection) -> Result<i64, BackendError> {
    connection
        .pragma_query_value(Some("main"), "user_version", |row| row.get(0))
        .map_err(classify_validation_error)
}

pub(crate) fn initialize_v3(connection: &Connection) -> Result<(), BackendError> {
    initialize_v3_scoped(connection, |_| Ok(()))
}

pub(crate) fn initialize_v3_scoped(
    connection: &Connection,
    mut before: impl FnMut(SchemaStatement) -> Result<(), BackendError>,
) -> Result<(), BackendError> {
    for (identity, (name, statement)) in SchemaTable::ALL.into_iter().zip(CREATE_TABLE_STATEMENTS) {
        debug_assert_eq!(identity.name(), name);
        before(SchemaStatement::CreateTable(identity))?;
        connection
            .execute(statement, [])
            .map_err(crate::sqlite::classify_runtime_error)?;
    }
    for (identity, (name, statement)) in NamedSchemaIndex::ALL
        .into_iter()
        .zip(CREATE_INDEX_STATEMENTS)
    {
        debug_assert_eq!(identity.name(), name);
        before(SchemaStatement::CreateIndex(identity))?;
        connection
            .execute(statement, [])
            .map_err(crate::sqlite::classify_runtime_error)?;
    }
    before(SchemaStatement::SetUserVersion)?;
    connection
        .pragma_update(Some("main"), "user_version", SCHEMA_VERSION)
        .map_err(crate::sqlite::classify_runtime_error)?;
    validate_v3_with_callback(connection, &mut before)
}

pub(crate) fn validate_v3(connection: &Connection) -> Result<(), BackendError> {
    validate_v3_scoped(connection, |_| Ok(()))
}

pub(crate) fn validate_v3_scoped(
    connection: &Connection,
    mut before: impl FnMut(SchemaStatement) -> Result<(), BackendError>,
) -> Result<(), BackendError> {
    validate_v3_with_callback(connection, &mut before)
}

fn validate_v3_with_callback(
    connection: &Connection,
    before: &mut impl FnMut(SchemaStatement) -> Result<(), BackendError>,
) -> Result<(), BackendError> {
    before(SchemaStatement::ReadUserVersion)?;
    let found = user_version(connection)?;
    if found != SCHEMA_VERSION {
        return Err(BackendError::UnsupportedSchemaVersion { found });
    }
    validate_schema_objects(connection, before)?;
    validate_table_list(connection, before)?;
    for table in TABLE_SPECS {
        validate_columns(connection, table, before)?;
        validate_index_list(connection, table, before)?;
        validate_foreign_keys(connection, table, before)?;
    }
    for index in INDEX_SPECS {
        validate_index_columns(connection, index, before)?;
    }
    validate_integrity(connection, before)
}

fn validate_schema_objects(
    connection: &Connection,
    before: &mut impl FnMut(SchemaStatement) -> Result<(), BackendError>,
) -> Result<(), BackendError> {
    before(SchemaStatement::SchemaObjects)?;
    let mut statement = connection
        .prepare(SCHEMA_OBJECTS_SQL)
        .map_err(classify_validation_error)?;
    let actual = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(classify_validation_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_validation_error)?;
    let expected = expected_schema_objects();
    if actual.len() != expected.len() {
        return Err(BackendError::CorruptSchema);
    }
    for (object_type, name, table, rootpage, sql) in actual {
        if rootpage <= 0 {
            return Err(BackendError::CorruptSchema);
        }
        let Some((expected_type, expected_table, expected_sql)) = expected.get(name.as_str())
        else {
            return Err(BackendError::CorruptSchema);
        };
        if object_type != *expected_type
            || table != *expected_table
            || sql.as_deref() != *expected_sql
        {
            return Err(BackendError::CorruptSchema);
        }
    }
    Ok(())
}

fn expected_schema_objects()
-> BTreeMap<&'static str, (&'static str, &'static str, Option<&'static str>)> {
    let mut expected = BTreeMap::new();
    for (name, sql) in CREATE_TABLE_STATEMENTS {
        expected.insert(name, ("table", name, Some(sql)));
    }
    for (name, sql) in CREATE_INDEX_STATEMENTS {
        let table = INDEX_SPECS
            .iter()
            .find(|index| index.name == name)
            .expect("every named index has static metadata")
            .table;
        expected.insert(name, ("index", table, Some(sql)));
    }
    for name in AUTO_INDEXES {
        let table = INDEX_SPECS
            .iter()
            .find(|index| index.name == name)
            .expect("every autoindex has static metadata")
            .table;
        expected.insert(name, ("index", table, None));
    }
    expected
}

fn table_identity(name: &str) -> SchemaTable {
    SchemaTable::ALL
        .into_iter()
        .find(|identity| identity.name() == name)
        .expect("every table specification has a closed identity")
}

fn index_identity(name: &str) -> SchemaIndex {
    SchemaIndex::ALL
        .into_iter()
        .find(|identity| identity.name() == name)
        .expect("every index specification has a closed identity")
}

fn validate_table_list(
    connection: &Connection,
    before: &mut impl FnMut(SchemaStatement) -> Result<(), BackendError>,
) -> Result<(), BackendError> {
    before(SchemaStatement::TableList)?;
    let mut statement = connection
        .prepare(TABLE_LIST_SQL)
        .map_err(classify_validation_error)?;
    let actual = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(classify_validation_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_validation_error)?;
    let mut expected = TABLE_SPECS
        .iter()
        .map(|table| {
            (
                "main".to_owned(),
                table.name.to_owned(),
                "table".to_owned(),
                table.columns.len() as i64,
                0,
                1,
            )
        })
        .collect::<Vec<_>>();
    expected.push((
        "main".to_owned(),
        "sqlite_schema".to_owned(),
        "table".to_owned(),
        5,
        0,
        0,
    ));
    expected.sort_by(|left, right| left.1.cmp(&right.1));
    if actual == expected {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

fn validate_columns(
    connection: &Connection,
    table: &TableSpec,
    before: &mut impl FnMut(SchemaStatement) -> Result<(), BackendError>,
) -> Result<(), BackendError> {
    before(SchemaStatement::TableInfo(table_identity(table.name)))?;
    let mut statement = connection
        .prepare(table.info_sql)
        .map_err(classify_validation_error)?;
    let actual = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(classify_validation_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_validation_error)?;
    let expected = table
        .columns
        .iter()
        .enumerate()
        .map(|(cid, column)| {
            (
                cid as i64,
                column.name.to_owned(),
                column.declared_type.to_owned(),
                column.not_null,
                None,
                column.primary_key_order,
            )
        })
        .collect::<Vec<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

fn validate_index_list(
    connection: &Connection,
    table: &TableSpec,
    before: &mut impl FnMut(SchemaStatement) -> Result<(), BackendError>,
) -> Result<(), BackendError> {
    before(SchemaStatement::IndexList(table_identity(table.name)))?;
    let mut statement = connection
        .prepare(table.index_list_sql)
        .map_err(classify_validation_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(classify_validation_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_validation_error)?;
    let mut sequences = BTreeSet::new();
    let mut actual = BTreeMap::new();
    for (sequence, name, unique, origin, partial) in rows {
        if sequence < 0
            || !sequences.insert(sequence)
            || actual.insert(name, (unique, origin, partial)).is_some()
        {
            return Err(BackendError::CorruptSchema);
        }
    }
    let expected = INDEX_SPECS
        .iter()
        .filter(|index| index.table == table.name)
        .map(|index| {
            (
                index.name.to_owned(),
                (index.unique, index.origin.to_owned(), 0),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if actual == expected {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

fn validate_index_columns(
    connection: &Connection,
    index: &IndexSpec,
    before: &mut impl FnMut(SchemaStatement) -> Result<(), BackendError>,
) -> Result<(), BackendError> {
    let table = TABLE_SPECS
        .iter()
        .find(|table| table.name == index.table)
        .expect("every index belongs to one application table");
    let expected = index
        .columns
        .iter()
        .enumerate()
        .map(|(sequence, name)| {
            let cid = table
                .columns
                .iter()
                .position(|column| column.name == *name)
                .expect("every indexed column belongs to its table") as i64;
            (sequence as i64, cid, Some((*name).to_owned()))
        })
        .collect::<Vec<_>>();

    let identity = index_identity(index.name);
    before(SchemaStatement::IndexInfo(identity))?;
    let mut statement = connection
        .prepare(index.info_sql)
        .map_err(classify_validation_error)?;
    let actual = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(classify_validation_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_validation_error)?;
    if actual != expected {
        return Err(BackendError::CorruptSchema);
    }

    before(SchemaStatement::IndexXinfo(identity))?;
    let mut statement = connection
        .prepare(index.xinfo_sql)
        .map_err(classify_validation_error)?;
    let actual = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(classify_validation_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_validation_error)?;
    let mut expected_xinfo = expected
        .into_iter()
        .map(|(sequence, cid, name)| (sequence, cid, name, 0, "BINARY".to_owned(), 1))
        .collect::<Vec<_>>();
    expected_xinfo.push((
        expected_xinfo.len() as i64,
        -1,
        None,
        0,
        "BINARY".to_owned(),
        0,
    ));
    if actual == expected_xinfo {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ForeignKeyDefinition {
    parent: String,
    columns: Vec<(String, String)>,
    on_update: String,
    on_delete: String,
    match_kind: String,
}

fn validate_foreign_keys(
    connection: &Connection,
    table: &TableSpec,
    before: &mut impl FnMut(SchemaStatement) -> Result<(), BackendError>,
) -> Result<(), BackendError> {
    before(SchemaStatement::ForeignKeyList(table_identity(table.name)))?;
    let mut statement = connection
        .prepare(table.foreign_key_sql)
        .map_err(classify_validation_error)?;
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
        .map_err(classify_validation_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_validation_error)?;

    let mut grouped: BTreeMap<i64, ForeignKeyDefinition> = BTreeMap::new();
    for (id, sequence, parent, from, to, update, delete, match_kind) in rows {
        let entry = grouped.entry(id).or_insert_with(|| ForeignKeyDefinition {
            parent: parent.clone(),
            columns: Vec::new(),
            on_update: update.clone(),
            on_delete: delete.clone(),
            match_kind: match_kind.clone(),
        });
        if sequence != entry.columns.len() as i64
            || entry.parent != parent
            || entry.on_update != update
            || entry.on_delete != delete
            || entry.match_kind != match_kind
        {
            return Err(BackendError::CorruptSchema);
        }
        entry.columns.push((from, to));
    }
    let actual = grouped.into_values().collect::<BTreeSet<_>>();
    let expected = table
        .foreign_keys
        .iter()
        .map(|foreign_key| ForeignKeyDefinition {
            parent: foreign_key.parent.to_owned(),
            columns: foreign_key
                .columns
                .iter()
                .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
                .collect(),
            on_update: "RESTRICT".to_owned(),
            on_delete: "RESTRICT".to_owned(),
            match_kind: "NONE".to_owned(),
        })
        .collect::<BTreeSet<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

fn validate_integrity(
    connection: &Connection,
    before: &mut impl FnMut(SchemaStatement) -> Result<(), BackendError>,
) -> Result<(), BackendError> {
    before(SchemaStatement::IntegrityCheck)?;
    let mut statement = connection
        .prepare(INTEGRITY_CHECK_SQL)
        .map_err(classify_validation_error)?;
    let diagnostics = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(classify_validation_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_validation_error)?;
    if diagnostics != ["ok"] {
        return Err(BackendError::CorruptSchema);
    }

    before(SchemaStatement::ForeignKeyCheck)?;
    let mut statement = connection
        .prepare(FOREIGN_KEY_CHECK_SQL)
        .map_err(classify_validation_error)?;
    let violations = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(classify_validation_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_validation_error)?;
    if violations.is_empty() {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

fn classify_validation_error(error: rusqlite::Error) -> BackendError {
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

#[cfg(test)]
#[path = "schema/tests.rs"]
mod tests;
