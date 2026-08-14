use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use cubikan_core::{IntentUnit, IntentUnitId, IntentUnitStatus};
use rusqlite::{
    Connection, ErrorCode, OpenFlags, Params, Row,
    config::DbConfig,
    hooks::{AuthAction, AuthContext, Authorization, TransactionOperation},
    limits::Limit,
};

use crate::{
    BackendError, BackendSchemaVersion, CompleteIntentUnit, CreateIntentUnit, CreateRelationship,
    CreateRelationshipDefinition, DeleteRelationship, IntentUnitPage, IntentUnitView,
    ListIntentUnits, ListRelationships, MigrationError, MutationResult, ProjectionPage,
    ProjectionQueryV1, RelationshipDefinitionKey, RelationshipDefinitionView, RelationshipError,
    RelationshipPage, RelationshipView, StorageFailure, TransitionIntentUnit, migration,
    projection_store::{ProjectionStatement, ProjectionWriter as ProjectionWriterBoundary},
    query, relationship_store, schema, stored,
};

const RETIRED_SCHEMA_VERSION: i64 = 2;
const SQLITE_VERSION: &str = "3.53.2";
const SQLITE_VERSION_NUMBER: i32 = 3_053_002;
const SQLITE_VFS: &str = "unix";
const DATABASE_HEADER_LENGTH: usize = 100;
const DATABASE_PAGE_SIZE: i64 = 4_096;
const MAX_DATABASE_PAGES: i64 = 262_144;
const MAX_DATABASE_BYTES: i64 = DATABASE_PAGE_SIZE * MAX_DATABASE_PAGES;
const BUSY_TIMEOUT_MILLISECONDS: i64 = 5_000;

const NUMERIC_LIMITS: [(Limit, i32); 12] = [
    (Limit::SQLITE_LIMIT_LENGTH, 4_194_304),
    (Limit::SQLITE_LIMIT_SQL_LENGTH, 65_536),
    (Limit::SQLITE_LIMIT_COLUMN, 64),
    (Limit::SQLITE_LIMIT_EXPR_DEPTH, 64),
    (Limit::SQLITE_LIMIT_COMPOUND_SELECT, 8),
    (Limit::SQLITE_LIMIT_VDBE_OP, 100_000),
    (Limit::SQLITE_LIMIT_FUNCTION_ARG, 32),
    (Limit::SQLITE_LIMIT_ATTACHED, 0),
    (Limit::SQLITE_LIMIT_LIKE_PATTERN_LENGTH, 256),
    (Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 128),
    (Limit::SQLITE_LIMIT_TRIGGER_DEPTH, 0),
    (Limit::SQLITE_LIMIT_WORKER_THREADS, 0),
];

const EXPECTED_COMPILE_OPTIONS_X86_64_LINUX_GNU: [&str; 51] = [
    "ATOMIC_INTRINSICS=1",
    "COMPILER=gcc-15.2.0",
    "DEFAULT_AUTOVACUUM",
    "DEFAULT_CACHE_SIZE=-2000",
    "DEFAULT_FILE_FORMAT=4",
    "DEFAULT_FOREIGN_KEYS",
    "DEFAULT_JOURNAL_SIZE_LIMIT=-1",
    "DEFAULT_MMAP_SIZE=0",
    "DEFAULT_PAGE_SIZE=4096",
    "DEFAULT_PCACHE_INITSZ=20",
    "DEFAULT_RECURSIVE_TRIGGERS",
    "DEFAULT_SECTOR_SIZE=4096",
    "DEFAULT_SYNCHRONOUS=2",
    "DEFAULT_WAL_AUTOCHECKPOINT=1000",
    "DEFAULT_WAL_SYNCHRONOUS=2",
    "DEFAULT_WORKER_THREADS=0",
    "DIRECT_OVERFLOW_READ",
    "ENABLE_API_ARMOR",
    "ENABLE_COLUMN_METADATA",
    "ENABLE_DBSTAT_VTAB",
    "ENABLE_FTS3",
    "ENABLE_FTS3_PARENTHESIS",
    "ENABLE_FTS5",
    "ENABLE_LOAD_EXTENSION",
    "ENABLE_MEMORY_MANAGEMENT",
    "ENABLE_RTREE",
    "ENABLE_STAT4",
    "HAVE_ISNAN",
    "MALLOC_SOFT_LIMIT=1024",
    "MAX_ATTACHED=10",
    "MAX_COLUMN=2000",
    "MAX_COMPOUND_SELECT=500",
    "MAX_DEFAULT_PAGE_SIZE=8192",
    "MAX_EXPR_DEPTH=1000",
    "MAX_FUNCTION_ARG=1000",
    "MAX_LENGTH=1000000000",
    "MAX_LIKE_PATTERN_LENGTH=50000",
    "MAX_MMAP_SIZE=0x7fff0000",
    "MAX_PAGE_COUNT=0xfffffffe",
    "MAX_PAGE_SIZE=65536",
    "MAX_SQL_LENGTH=1000000000",
    "MAX_TRIGGER_DEPTH=1000",
    "MAX_VARIABLE_NUMBER=32766",
    "MAX_VDBE_OP=250000000",
    "MAX_WORKER_THREADS=8",
    "MUTEX_PTHREADS",
    "SOUNDEX",
    "SYSTEM_MALLOC",
    "TEMP_STORE=1",
    "THREADSAFE=1",
    "USE_URI",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConnectionRole {
    Creator,
    PreflightReader,
    ProjectorWriter,
    PublicReader,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConfigurationStatement {
    UserVersionRead,
    EncodingRead,
    EncodingSetUtf8,
    PageSizeRead,
    PageSizeSet4096,
    PageCountRead,
    JournalModeRead,
    JournalModeSetDelete,
    SynchronousRead,
    SynchronousSetExtra,
    ForeignKeysRead,
    ForeignKeysSetOn,
    TrustedSchemaRead,
    TrustedSchemaSetOff,
    QueryOnlyRead,
    QueryOnlySetOn,
    QueryOnlySetOff,
    CellSizeCheckRead,
    CellSizeCheckSetOn,
    MmapSizeRead,
    MmapSizeSetZero,
    TempStoreRead,
    TempStoreSetMemory,
    BusyTimeoutRead,
    BusyTimeoutSet5000,
    MaxPageCountRead,
    MaxPageCountSet262144,
    #[cfg(test)]
    MaxPageCountSet128,
    DataVersionRead,
}

impl ConfigurationStatement {
    const fn tuple(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::UserVersionRead => ("user_version", None),
            Self::EncodingRead => ("encoding", None),
            Self::EncodingSetUtf8 => ("encoding", Some("UTF-8")),
            Self::PageSizeRead => ("page_size", None),
            Self::PageSizeSet4096 => ("page_size", Some("4096")),
            Self::PageCountRead => ("page_count", None),
            Self::JournalModeRead => ("journal_mode", None),
            Self::JournalModeSetDelete => ("journal_mode", Some("DELETE")),
            Self::SynchronousRead => ("synchronous", None),
            Self::SynchronousSetExtra => ("synchronous", Some("EXTRA")),
            Self::ForeignKeysRead => ("foreign_keys", None),
            Self::ForeignKeysSetOn => ("foreign_keys", Some("ON")),
            Self::TrustedSchemaRead => ("trusted_schema", None),
            Self::TrustedSchemaSetOff => ("trusted_schema", Some("OFF")),
            Self::QueryOnlyRead => ("query_only", None),
            Self::QueryOnlySetOn => ("query_only", Some("ON")),
            Self::QueryOnlySetOff => ("query_only", Some("OFF")),
            Self::CellSizeCheckRead => ("cell_size_check", None),
            Self::CellSizeCheckSetOn => ("cell_size_check", Some("ON")),
            Self::MmapSizeRead => ("mmap_size", None),
            Self::MmapSizeSetZero => ("mmap_size", Some("0")),
            Self::TempStoreRead => ("temp_store", None),
            Self::TempStoreSetMemory => ("temp_store", Some("MEMORY")),
            Self::BusyTimeoutRead => ("busy_timeout", None),
            Self::BusyTimeoutSet5000 => ("busy_timeout", Some("5000")),
            Self::MaxPageCountRead => ("max_page_count", None),
            Self::MaxPageCountSet262144 => ("max_page_count", Some("262144")),
            #[cfg(test)]
            Self::MaxPageCountSet128 => ("max_page_count", Some("128")),
            Self::DataVersionRead => ("data_version", None),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransactionStatement {
    Begin,
    Commit,
    Rollback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VerifiedQueryStatement {
    OneBoundedRead,
    FullProjectionCompare,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuthorizationScope {
    Deny,
    Configuration(ConfigurationStatement),
    Schema(schema::SchemaStatement),
    Transaction(TransactionStatement),
    Projection(ProjectionStatement),
    VerifiedQuery(VerifiedQueryStatement),
}

#[derive(Clone, Copy, Debug)]
struct AuthorizationState {
    role: ConnectionRole,
    scope: AuthorizationScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MountIdentity {
    mount_point: PathBuf,
    filesystem_type: String,
}

#[cfg(test)]
#[derive(Clone, Copy)]
struct VfsObservation<'a> {
    requested_name: &'a str,
    registered: bool,
    built_in: bool,
}

pub(crate) struct ProjectionReaderConnection {
    connection: Connection,
    authorization: Arc<Mutex<AuthorizationState>>,
    one_bounded_read_used: bool,
    #[cfg(target_os = "linux")]
    _path: secure_fs::SecurePath,
}

pub(crate) struct ProjectionWriterConnection {
    connection: Connection,
    authorization: Arc<Mutex<AuthorizationState>>,
    #[cfg(target_os = "linux")]
    _path: secure_fs::SecurePath,
}

impl ProjectionReaderConnection {
    pub(crate) fn data_version(&self) -> Result<i64, BackendError> {
        self.require_public_reader()?;
        if !self.connection.is_autocommit() {
            return Err(BackendError::CorruptSchema);
        }
        query_configuration_i64(
            &self.connection,
            &self.authorization,
            ConnectionRole::PublicReader,
            ConfigurationStatement::DataVersionRead,
            "PRAGMA main.data_version",
        )
    }

    pub(crate) fn begin_verified_read(&mut self) -> Result<(), BackendError> {
        self.require_public_reader()?;
        if !self.connection.is_autocommit() {
            return Err(BackendError::CorruptSchema);
        }
        execute_transaction(
            &self.connection,
            &self.authorization,
            TransactionStatement::Begin,
            "BEGIN",
        )?;
        self.one_bounded_read_used = false;
        Ok(())
    }

    pub(crate) fn rollback_verified_read(&mut self) -> Result<(), BackendError> {
        self.require_public_reader()?;
        if self.connection.is_autocommit() {
            self.one_bounded_read_used = false;
            return set_authorization_scope(&self.authorization, AuthorizationScope::Deny);
        }
        execute_transaction(
            &self.connection,
            &self.authorization,
            TransactionStatement::Rollback,
            "ROLLBACK",
        )?;
        self.one_bounded_read_used = false;
        Ok(())
    }

    pub(crate) fn with_verified_query<R, F>(
        &mut self,
        statement: VerifiedQueryStatement,
        operation: F,
    ) -> Result<R, BackendError>
    where
        F: for<'connection> FnOnce(&'connection Connection) -> Result<R, BackendError>,
    {
        self.require_public_reader()?;
        if self.connection.is_autocommit() {
            return Err(BackendError::CorruptSchema);
        }
        if statement == VerifiedQueryStatement::OneBoundedRead {
            if self.one_bounded_read_used {
                return Err(BackendError::CorruptSchema);
            }
            self.one_bounded_read_used = true;
        }
        set_authorization_scope(
            &self.authorization,
            AuthorizationScope::VerifiedQuery(statement),
        )?;
        let result = operation(&self.connection);
        let reset = set_authorization_scope(&self.authorization, AuthorizationScope::Deny);
        match result {
            Ok(value) => {
                reset?;
                Ok(value)
            }
            Err(error) => {
                let _ = reset;
                Err(error)
            }
        }
    }

    fn require_public_reader(&self) -> Result<(), BackendError> {
        if self
            .authorization
            .lock()
            .map_err(|_| BackendError::CorruptSchema)?
            .role
            == ConnectionRole::PublicReader
        {
            Ok(())
        } else {
            Err(BackendError::CorruptSchema)
        }
    }
}

impl Drop for ProjectionReaderConnection {
    fn drop(&mut self) {
        if !self.connection.is_autocommit() {
            let _ = execute_transaction(
                &self.connection,
                &self.authorization,
                TransactionStatement::Rollback,
                "ROLLBACK",
            );
        }
        let _ = set_authorization_scope(&self.authorization, AuthorizationScope::Deny);
    }
}

impl ProjectionWriterConnection {
    pub(crate) fn begin_projection(&mut self) -> Result<(), BackendError> {
        if !self.connection.is_autocommit() {
            return Err(BackendError::CorruptSchema);
        }
        let begin = execute_transaction(
            &self.connection,
            &self.authorization,
            TransactionStatement::Begin,
            "BEGIN IMMEDIATE",
        );
        if let Err(error) = begin {
            let _ = set_authorization_scope(&self.authorization, AuthorizationScope::Deny);
            return Err(error);
        }
        if let Err(error) = validate_schema_scoped(
            &self.connection,
            &self.authorization,
            ConnectionRole::ProjectorWriter,
        ) {
            let _ = self.rollback_projection();
            return Err(error);
        }
        set_authorization_scope(&self.authorization, AuthorizationScope::Deny)
    }

    pub(crate) fn commit_projection(&mut self) -> Result<(), BackendError> {
        if self.connection.is_autocommit() {
            return Err(BackendError::CorruptSchema);
        }
        execute_transaction(
            &self.connection,
            &self.authorization,
            TransactionStatement::Commit,
            "COMMIT",
        )
    }

    pub(crate) fn rollback_projection(&mut self) -> Result<(), BackendError> {
        if self.connection.is_autocommit() {
            return set_authorization_scope(&self.authorization, AuthorizationScope::Deny);
        }
        execute_transaction(
            &self.connection,
            &self.authorization,
            TransactionStatement::Rollback,
            "ROLLBACK",
        )
    }

    #[cfg(test)]
    fn connection(&self) -> &Connection {
        &self.connection
    }
}

impl ProjectionWriterBoundary for ProjectionWriterConnection {
    fn execute<P: Params>(
        &mut self,
        statement: ProjectionStatement,
        parameters: P,
    ) -> Result<usize, BackendError> {
        if self.connection.is_autocommit() || !static_sql_is_safe(statement.sql()) {
            return Err(BackendError::CorruptSchema);
        }
        set_authorization_scope(
            &self.authorization,
            AuthorizationScope::Projection(statement),
        )?;
        let result = self
            .connection
            .execute(statement.sql(), parameters)
            .map_err(classify_runtime_error);
        let reset = set_authorization_scope(&self.authorization, AuthorizationScope::Deny);
        match result {
            Ok(changed) => {
                reset?;
                Ok(changed)
            }
            Err(error) => {
                let _ = reset;
                Err(error)
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn create_fresh_projection(
    directory: &Path,
    basename: &OsStr,
) -> Result<ProjectionWriterConnection, BackendError> {
    let mut path = secure_fs::SecurePath::for_creation(directory, basename)?;
    validate_runtime_boundary()?;
    let identity = path.create_exclusive_file()?;
    create_fresh_projection_at(path, identity, CreationFault::None)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn create_fresh_projection(
    _directory: &Path,
    _basename: &OsStr,
) -> Result<ProjectionWriterConnection, BackendError> {
    Err(BackendError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CreationFault {
    None,
    #[cfg(test)]
    AfterSchema,
}

#[cfg(target_os = "linux")]
fn create_fresh_projection_at(
    mut path: secure_fs::SecurePath,
    identity: secure_fs::FileIdentity,
    fault: CreationFault,
) -> Result<ProjectionWriterConnection, BackendError> {
    let _ = fault;
    path.arm_creation_cleanup(identity);
    path.validate_existing_file(Some(identity), false)?;
    let (connection, authorization) = open_role_connection(
        path.sqlite_path(),
        ConnectionRole::Creator,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    let configured = configure_sql_connection(&connection, &authorization, ConnectionRole::Creator);
    path.capture_creator_sidecars()?;
    configured?;
    let begun = execute_transaction(
        &connection,
        &authorization,
        TransactionStatement::Begin,
        "BEGIN IMMEDIATE",
    );
    path.capture_creator_sidecars()?;
    begun?;
    let initialized = initialize_schema_scoped(&connection, &authorization);
    if let Err(error) = initialized {
        let _ = execute_transaction(
            &connection,
            &authorization,
            TransactionStatement::Rollback,
            "ROLLBACK",
        );
        let _ = path.capture_creator_sidecars();
        return Err(error);
    }
    #[cfg(test)]
    if fault == CreationFault::AfterSchema {
        let _ = execute_transaction(
            &connection,
            &authorization,
            TransactionStatement::Rollback,
            "ROLLBACK",
        );
        let _ = path.capture_creator_sidecars();
        return Err(BackendError::CorruptSchema);
    }
    let committed = execute_transaction(
        &connection,
        &authorization,
        TransactionStatement::Commit,
        "COMMIT",
    );
    path.capture_creator_sidecars()?;
    committed?;
    drop(connection);
    path.sync_directory()?;
    path.validate_existing_file(Some(identity), true)?;
    let reader = open_preflight_at(&path)?;
    drop(reader);
    let mut writer = open_projector_at(path)?;
    writer._path.disarm_creation_cleanup();
    Ok(writer)
}

#[cfg(target_os = "linux")]
pub(crate) fn open_projection_writer(
    directory: &Path,
    basename: &OsStr,
) -> Result<ProjectionWriterConnection, BackendError> {
    let path = secure_fs::SecurePath::for_existing(directory, basename)?;
    validate_runtime_boundary()?;
    let reader = open_preflight_at(&path)?;
    drop(reader);
    open_projector_at(path)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn open_projection_writer(
    _directory: &Path,
    _basename: &OsStr,
) -> Result<ProjectionWriterConnection, BackendError> {
    Err(BackendError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
pub(crate) fn open_projection_reader(
    directory: &Path,
    basename: &OsStr,
) -> Result<ProjectionReaderConnection, BackendError> {
    let path = secure_fs::SecurePath::for_existing(directory, basename)?;
    validate_runtime_boundary()?;
    let preflight = open_preflight_at(&path)?;
    drop(preflight);
    path.validate_existing_file(None, true)?;
    let (connection, authorization) = open_role_connection(
        path.sqlite_path(),
        ConnectionRole::PublicReader,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    configure_sql_connection(&connection, &authorization, ConnectionRole::PublicReader)?;
    validate_existing_connection(
        &connection,
        &authorization,
        ConnectionRole::PublicReader,
        path.file_size()?,
    )?;
    set_authorization_scope(&authorization, AuthorizationScope::Deny)?;
    Ok(ProjectionReaderConnection {
        connection,
        authorization,
        one_bounded_read_used: false,
        _path: path,
    })
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn open_projection_reader(
    _directory: &Path,
    _basename: &OsStr,
) -> Result<ProjectionReaderConnection, BackendError> {
    Err(BackendError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
fn open_preflight_at(
    path: &secure_fs::SecurePath,
) -> Result<ProjectionReaderConnection, BackendError> {
    let file_size = path.validate_existing_file(None, true)?;
    let (connection, authorization) = open_role_connection(
        path.sqlite_path(),
        ConnectionRole::PreflightReader,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    configure_sql_connection(&connection, &authorization, ConnectionRole::PreflightReader)?;
    validate_existing_connection(
        &connection,
        &authorization,
        ConnectionRole::PreflightReader,
        file_size,
    )?;
    set_authorization_scope(&authorization, AuthorizationScope::Deny)?;
    Ok(ProjectionReaderConnection {
        connection,
        authorization,
        one_bounded_read_used: false,
        _path: path.duplicate()?,
    })
}

#[cfg(target_os = "linux")]
fn open_projector_at(
    path: secure_fs::SecurePath,
) -> Result<ProjectionWriterConnection, BackendError> {
    path.validate_existing_file(None, true)?;
    let (connection, authorization) = open_role_connection(
        path.sqlite_path(),
        ConnectionRole::ProjectorWriter,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    configure_sql_connection(&connection, &authorization, ConnectionRole::ProjectorWriter)?;
    let mut writer = ProjectionWriterConnection {
        connection,
        authorization,
        _path: path,
    };
    writer.begin_projection()?;
    writer.commit_projection()?;
    Ok(writer)
}

fn open_role_connection(
    path: &Path,
    role: ConnectionRole,
    flags: OpenFlags,
) -> Result<(Connection, Arc<Mutex<AuthorizationState>>), BackendError> {
    let connection = Connection::open_with_flags_and_vfs(path, flags, SQLITE_VFS)
        .map_err(classify_runtime_error)?;
    apply_non_sql_defenses(&connection)?;
    let authorization = Arc::new(Mutex::new(AuthorizationState {
        role,
        scope: AuthorizationScope::Deny,
    }));
    let callback_state = Arc::clone(&authorization);
    connection
        .authorizer(Some(move |context: AuthContext<'_>| -> Authorization {
            let Ok(state) = callback_state.lock() else {
                return Authorization::Deny;
            };
            authorize(context, *state)
        }))
        .map_err(classify_runtime_error)?;
    Ok((connection, authorization))
}

fn set_authorization_scope(
    authorization: &Arc<Mutex<AuthorizationState>>,
    scope: AuthorizationScope,
) -> Result<(), BackendError> {
    let mut state = authorization
        .lock()
        .map_err(|_| BackendError::CorruptSchema)?;
    state.scope = scope;
    Ok(())
}

fn validate_runtime_boundary() -> Result<(), BackendError> {
    if !cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        return Err(BackendError::UnsupportedPlatform);
    }
    if rusqlite::version() != SQLITE_VERSION || rusqlite::version_number() != SQLITE_VERSION_NUMBER
    {
        return Err(BackendError::CorruptSchema);
    }
    let mut options = rusqlite::compile_options().map_err(|_| BackendError::CorruptSchema)?;
    options.sort();
    if options
        .iter()
        .map(String::as_str)
        .ne(EXPECTED_COMPILE_OPTIONS_X86_64_LINUX_GNU.iter().copied())
        || EXPECTED_COMPILE_OPTIONS_X86_64_LINUX_GNU
            .iter()
            .any(|option| !rusqlite::compile_option_used(option))
    {
        return Err(BackendError::CorruptSchema);
    }
    let identities = rusqlite::registered_vfses().map_err(|_| BackendError::CorruptSchema)?;
    if identities.len() != 5
        || identities
            .iter()
            .map(|identity| identity.name.as_str())
            .ne(["unix", "memdb", "unix-excl", "unix-dotfile", "unix-none"])
        || identities.iter().enumerate().any(|(index, identity)| {
            let (expected_version, expected_maximum_pathname) =
                if index == 1 { (2, 1_024) } else { (3, 512) };
            identity.version != expected_version
                || identity.maximum_pathname != expected_maximum_pathname
                || identity.os_file_size <= 0
                || identity.is_default != (index == 0)
        })
        || identities
            .iter()
            .skip(1)
            .any(|identity| identity.os_file_size != identities[0].os_file_size)
    {
        return Err(BackendError::UnsupportedPlatform);
    }
    Ok(())
}

fn apply_non_sql_defenses(connection: &Connection) -> Result<(), BackendError> {
    for (limit, value) in NUMERIC_LIMITS {
        connection
            .set_limit(limit, value)
            .map_err(classify_runtime_error)?;
        if connection.limit(limit).map_err(classify_runtime_error)? != value {
            return Err(BackendError::CorruptSchema);
        }
    }
    for (config, value) in [
        (DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true),
        (DbConfig::SQLITE_DBCONFIG_WRITABLE_SCHEMA, false),
        (DbConfig::SQLITE_DBCONFIG_DQS_DML, false),
        (DbConfig::SQLITE_DBCONFIG_DQS_DDL, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_VIEW, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_FTS3_TOKENIZER, false),
        (DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_ATTACH_CREATE, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_ATTACH_WRITE, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_COMMENTS, false),
    ] {
        if connection
            .set_db_config(config, value)
            .map_err(classify_runtime_error)?
            != value
            || connection
                .db_config(config)
                .map_err(classify_runtime_error)?
                != value
        {
            return Err(BackendError::CorruptSchema);
        }
    }
    connection
        .busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MILLISECONDS as u64))
        .map_err(classify_runtime_error)?;
    connection
        .load_extension_disable()
        .map_err(classify_runtime_error)
}

fn configure_sql_connection(
    connection: &Connection,
    authorization: &Arc<Mutex<AuthorizationState>>,
    role: ConnectionRole,
) -> Result<(), BackendError> {
    for (identity, statement) in [
        (
            ConfigurationStatement::ForeignKeysSetOn,
            "PRAGMA main.foreign_keys=ON",
        ),
        (
            ConfigurationStatement::TrustedSchemaSetOff,
            "PRAGMA main.trusted_schema=OFF",
        ),
        (
            ConfigurationStatement::CellSizeCheckSetOn,
            "PRAGMA main.cell_size_check=ON",
        ),
        (
            ConfigurationStatement::MmapSizeSetZero,
            "PRAGMA main.mmap_size=0",
        ),
        (
            ConfigurationStatement::TempStoreSetMemory,
            "PRAGMA temp_store=MEMORY",
        ),
        (
            ConfigurationStatement::BusyTimeoutSet5000,
            "PRAGMA busy_timeout=5000",
        ),
    ] {
        execute_configuration(connection, authorization, role, identity, statement)?;
    }
    match role {
        ConnectionRole::Creator => {
            for (identity, statement) in [
                (
                    ConfigurationStatement::EncodingSetUtf8,
                    "PRAGMA main.encoding='UTF-8'",
                ),
                (
                    ConfigurationStatement::PageSizeSet4096,
                    "PRAGMA main.page_size=4096",
                ),
                (
                    ConfigurationStatement::JournalModeSetDelete,
                    "PRAGMA main.journal_mode=DELETE",
                ),
                (
                    ConfigurationStatement::SynchronousSetExtra,
                    "PRAGMA main.synchronous=EXTRA",
                ),
                (
                    ConfigurationStatement::QueryOnlySetOff,
                    "PRAGMA main.query_only=OFF",
                ),
                (
                    ConfigurationStatement::MaxPageCountSet262144,
                    "PRAGMA main.max_page_count=262144",
                ),
            ] {
                execute_configuration(connection, authorization, role, identity, statement)?;
            }
        }
        ConnectionRole::ProjectorWriter => {
            for (identity, statement) in [
                (
                    ConfigurationStatement::JournalModeSetDelete,
                    "PRAGMA main.journal_mode=DELETE",
                ),
                (
                    ConfigurationStatement::SynchronousSetExtra,
                    "PRAGMA main.synchronous=EXTRA",
                ),
                (
                    ConfigurationStatement::QueryOnlySetOff,
                    "PRAGMA main.query_only=OFF",
                ),
                (
                    ConfigurationStatement::MaxPageCountSet262144,
                    "PRAGMA main.max_page_count=262144",
                ),
            ] {
                execute_configuration(connection, authorization, role, identity, statement)?;
            }
        }
        ConnectionRole::PreflightReader | ConnectionRole::PublicReader => {
            execute_configuration(
                connection,
                authorization,
                role,
                ConfigurationStatement::QueryOnlySetOn,
                "PRAGMA main.query_only=ON",
            )?;
        }
    }
    validate_common_settings(connection, authorization, role)
}

fn validate_common_settings(
    connection: &Connection,
    authorization: &Arc<Mutex<AuthorizationState>>,
    role: ConnectionRole,
) -> Result<(), BackendError> {
    expect_configuration_i64(
        connection,
        authorization,
        role,
        ConfigurationStatement::ForeignKeysRead,
        "PRAGMA main.foreign_keys",
        1,
    )?;
    expect_configuration_i64(
        connection,
        authorization,
        role,
        ConfigurationStatement::TrustedSchemaRead,
        "PRAGMA main.trusted_schema",
        0,
    )?;
    expect_configuration_i64(
        connection,
        authorization,
        role,
        ConfigurationStatement::CellSizeCheckRead,
        "PRAGMA main.cell_size_check",
        1,
    )?;
    expect_configuration_i64(
        connection,
        authorization,
        role,
        ConfigurationStatement::MmapSizeRead,
        "PRAGMA main.mmap_size",
        0,
    )?;
    expect_configuration_i64(
        connection,
        authorization,
        role,
        ConfigurationStatement::TempStoreRead,
        "PRAGMA temp_store",
        2,
    )?;
    expect_configuration_i64(
        connection,
        authorization,
        role,
        ConfigurationStatement::BusyTimeoutRead,
        "PRAGMA busy_timeout",
        BUSY_TIMEOUT_MILLISECONDS,
    )?;
    match role {
        ConnectionRole::Creator | ConnectionRole::ProjectorWriter => {
            expect_configuration_i64(
                connection,
                authorization,
                role,
                ConfigurationStatement::QueryOnlyRead,
                "PRAGMA main.query_only",
                0,
            )?;
            expect_configuration_i64(
                connection,
                authorization,
                role,
                ConfigurationStatement::MaxPageCountRead,
                "PRAGMA main.max_page_count",
                MAX_DATABASE_PAGES,
            )?;
            expect_configuration_i64(
                connection,
                authorization,
                role,
                ConfigurationStatement::SynchronousRead,
                "PRAGMA main.synchronous",
                3,
            )?;
            expect_configuration_text(
                connection,
                authorization,
                role,
                ConfigurationStatement::JournalModeRead,
                "PRAGMA main.journal_mode",
                "delete",
            )?;
        }
        ConnectionRole::PreflightReader | ConnectionRole::PublicReader => {
            expect_configuration_i64(
                connection,
                authorization,
                role,
                ConfigurationStatement::QueryOnlyRead,
                "PRAGMA main.query_only",
                1,
            )?;
            let _ = query_configuration_i64(
                connection,
                authorization,
                role,
                ConfigurationStatement::MaxPageCountRead,
                "PRAGMA main.max_page_count",
            )?;
        }
    }
    Ok(())
}

fn validate_existing_connection(
    connection: &Connection,
    authorization: &Arc<Mutex<AuthorizationState>>,
    role: ConnectionRole,
    file_size: i64,
) -> Result<(), BackendError> {
    let encoding = query_configuration_text(
        connection,
        authorization,
        role,
        ConfigurationStatement::EncodingRead,
        "PRAGMA main.encoding",
    )?;
    if encoding != "UTF-8" {
        return Err(BackendError::CorruptSchema);
    }
    expect_configuration_i64(
        connection,
        authorization,
        role,
        ConfigurationStatement::PageSizeRead,
        "PRAGMA main.page_size",
        DATABASE_PAGE_SIZE,
    )?;
    expect_configuration_text(
        connection,
        authorization,
        role,
        ConfigurationStatement::JournalModeRead,
        "PRAGMA main.journal_mode",
        "delete",
    )?;
    if matches!(
        role,
        ConnectionRole::PreflightReader | ConnectionRole::PublicReader
    ) {
        expect_configuration_i64(
            connection,
            authorization,
            role,
            ConfigurationStatement::SynchronousRead,
            "PRAGMA main.synchronous",
            2,
        )?;
    }
    let page_count = query_configuration_i64(
        connection,
        authorization,
        role,
        ConfigurationStatement::PageCountRead,
        "PRAGMA main.page_count",
    )?;
    if !(1..=MAX_DATABASE_PAGES).contains(&page_count)
        || page_count.checked_mul(DATABASE_PAGE_SIZE) != Some(file_size)
    {
        return Err(BackendError::CorruptSchema);
    }
    validate_schema_scoped(connection, authorization, role)
}

fn initialize_schema_scoped(
    connection: &Connection,
    authorization: &Arc<Mutex<AuthorizationState>>,
) -> Result<(), BackendError> {
    let result = schema::initialize_v3_scoped(connection, |statement| {
        set_authorization_scope(authorization, AuthorizationScope::Schema(statement))
    });
    let reset = set_authorization_scope(authorization, AuthorizationScope::Deny);
    result.and(reset)
}

fn validate_schema_scoped(
    connection: &Connection,
    authorization: &Arc<Mutex<AuthorizationState>>,
    role: ConnectionRole,
) -> Result<(), BackendError> {
    if authorization
        .lock()
        .map_err(|_| BackendError::CorruptSchema)?
        .role
        != role
    {
        return Err(BackendError::CorruptSchema);
    }
    let result = schema::validate_v3_scoped(connection, |statement| {
        set_authorization_scope(authorization, AuthorizationScope::Schema(statement))
    });
    let reset = set_authorization_scope(authorization, AuthorizationScope::Deny);
    result.and(reset)
}

fn configuration_allowed(role: ConnectionRole, statement: ConfigurationStatement) -> bool {
    match statement {
        ConfigurationStatement::DataVersionRead => role == ConnectionRole::PublicReader,
        ConfigurationStatement::PageCountRead => role != ConnectionRole::Creator,
        ConfigurationStatement::EncodingSetUtf8 | ConfigurationStatement::PageSizeSet4096 => {
            role == ConnectionRole::Creator
        }
        ConfigurationStatement::JournalModeSetDelete
        | ConfigurationStatement::SynchronousSetExtra
        | ConfigurationStatement::QueryOnlySetOff
        | ConfigurationStatement::MaxPageCountSet262144 => matches!(
            role,
            ConnectionRole::Creator | ConnectionRole::ProjectorWriter
        ),
        #[cfg(test)]
        ConfigurationStatement::MaxPageCountSet128 => matches!(
            role,
            ConnectionRole::Creator | ConnectionRole::ProjectorWriter
        ),
        ConfigurationStatement::QueryOnlySetOn => matches!(
            role,
            ConnectionRole::PreflightReader | ConnectionRole::PublicReader
        ),
        _ => true,
    }
}

fn execute_configuration(
    connection: &Connection,
    authorization: &Arc<Mutex<AuthorizationState>>,
    role: ConnectionRole,
    identity: ConfigurationStatement,
    statement: &'static str,
) -> Result<(), BackendError> {
    if !configuration_allowed(role, identity) {
        return Err(BackendError::CorruptSchema);
    }
    set_authorization_scope(authorization, AuthorizationScope::Configuration(identity))?;
    let result = execute_static(connection, statement);
    let reset = set_authorization_scope(authorization, AuthorizationScope::Deny);
    result.and(reset)
}

fn query_configuration_i64(
    connection: &Connection,
    authorization: &Arc<Mutex<AuthorizationState>>,
    role: ConnectionRole,
    identity: ConfigurationStatement,
    statement: &'static str,
) -> Result<i64, BackendError> {
    if !configuration_allowed(role, identity) {
        return Err(BackendError::CorruptSchema);
    }
    set_authorization_scope(authorization, AuthorizationScope::Configuration(identity))?;
    let result = query_pragma_i64(connection, statement);
    let reset = set_authorization_scope(authorization, AuthorizationScope::Deny);
    match result {
        Ok(value) => {
            reset?;
            Ok(value)
        }
        Err(error) => {
            let _ = reset;
            Err(error)
        }
    }
}

fn query_configuration_text(
    connection: &Connection,
    authorization: &Arc<Mutex<AuthorizationState>>,
    role: ConnectionRole,
    identity: ConfigurationStatement,
    statement: &'static str,
) -> Result<String, BackendError> {
    if !configuration_allowed(role, identity) {
        return Err(BackendError::CorruptSchema);
    }
    set_authorization_scope(authorization, AuthorizationScope::Configuration(identity))?;
    let result = query_pragma_text(connection, statement);
    let reset = set_authorization_scope(authorization, AuthorizationScope::Deny);
    match result {
        Ok(value) => {
            reset?;
            Ok(value)
        }
        Err(error) => {
            let _ = reset;
            Err(error)
        }
    }
}

fn expect_configuration_i64(
    connection: &Connection,
    authorization: &Arc<Mutex<AuthorizationState>>,
    role: ConnectionRole,
    identity: ConfigurationStatement,
    statement: &'static str,
    expected: i64,
) -> Result<(), BackendError> {
    if query_configuration_i64(connection, authorization, role, identity, statement)? == expected {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

fn expect_configuration_text(
    connection: &Connection,
    authorization: &Arc<Mutex<AuthorizationState>>,
    role: ConnectionRole,
    identity: ConfigurationStatement,
    statement: &'static str,
    expected: &str,
) -> Result<(), BackendError> {
    if query_configuration_text(connection, authorization, role, identity, statement)? == expected {
        Ok(())
    } else {
        Err(BackendError::CorruptSchema)
    }
}

fn execute_transaction(
    connection: &Connection,
    authorization: &Arc<Mutex<AuthorizationState>>,
    identity: TransactionStatement,
    statement: &'static str,
) -> Result<(), BackendError> {
    set_authorization_scope(authorization, AuthorizationScope::Transaction(identity))?;
    let result = execute_static(connection, statement);
    let reset = set_authorization_scope(authorization, AuthorizationScope::Deny);
    result.and(reset)
}

fn execute_static(connection: &Connection, statement: &'static str) -> Result<(), BackendError> {
    if !static_sql_is_safe(statement) {
        return Err(BackendError::CorruptSchema);
    }
    connection
        .execute_batch(statement)
        .map_err(classify_runtime_error)
}

fn query_pragma_i64(connection: &Connection, statement: &'static str) -> Result<i64, BackendError> {
    if !static_sql_is_safe(statement) {
        return Err(BackendError::CorruptSchema);
    }
    connection
        .query_row(statement, [], |row| row.get(0))
        .map_err(classify_runtime_error)
}

fn query_pragma_text(
    connection: &Connection,
    statement: &'static str,
) -> Result<String, BackendError> {
    if !static_sql_is_safe(statement) {
        return Err(BackendError::CorruptSchema);
    }
    connection
        .query_row(statement, [], |row| row.get(0))
        .map_err(classify_runtime_error)
}

const fn static_sql_is_safe(statement: &str) -> bool {
    let bytes = statement.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == 0 || bytes[index] == b';' {
            return false;
        }
        if index + 1 < bytes.len()
            && ((bytes[index] == b'-' && bytes[index + 1] == b'-')
                || (bytes[index] == b'/' && bytes[index + 1] == b'*')
                || (bytes[index] == b'*' && bytes[index + 1] == b'/'))
        {
            return false;
        }
        index += 1;
    }
    !bytes.is_empty()
}

fn authorize(context: AuthContext<'_>, state: AuthorizationState) -> Authorization {
    if context.accessor.is_some() {
        return Authorization::Deny;
    }
    let allowed = match state.scope {
        AuthorizationScope::Deny => false,
        AuthorizationScope::Configuration(statement) => {
            allow_configuration(context.action, context.database_name, state.role, statement)
        }
        AuthorizationScope::Schema(statement) => {
            allow_schema_statement(context.action, context.database_name, state.role, statement)
        }
        AuthorizationScope::Transaction(statement) => {
            allow_transaction(context.action, context.database_name, statement)
        }
        AuthorizationScope::Projection(statement)
            if state.role == ConnectionRole::ProjectorWriter =>
        {
            allow_projection_statement(context.action, context.database_name, statement)
        }
        AuthorizationScope::VerifiedQuery(statement)
            if state.role == ConnectionRole::PublicReader =>
        {
            allow_verified_query(context.action, context.database_name, statement)
        }
        _ => false,
    };
    if allowed {
        Authorization::Allow
    } else {
        Authorization::Deny
    }
}

fn allow_verified_query(
    action: AuthAction<'_>,
    database: Option<&str>,
    statement: VerifiedQueryStatement,
) -> bool {
    match action {
        AuthAction::Select => database.is_none(),
        AuthAction::Function { function_name } => {
            database.is_none()
                && statement == VerifiedQueryStatement::FullProjectionCompare
                && matches!(
                    function_name,
                    "typeof" | "length" | "instr" | "char" | "hex" | "coalesce" | "glob"
                )
        }
        AuthAction::Read {
            table_name,
            column_name,
        } => database == Some("main") && is_application_column(table_name, column_name),
        _ => false,
    }
}

fn allow_transaction(
    action: AuthAction<'_>,
    database: Option<&str>,
    statement: TransactionStatement,
) -> bool {
    let operation = match statement {
        TransactionStatement::Begin => TransactionOperation::Begin,
        TransactionStatement::Commit => TransactionOperation::Commit,
        TransactionStatement::Rollback => TransactionOperation::Rollback,
    };
    database.is_none()
        && matches!(action, AuthAction::Transaction { operation: actual } if actual == operation)
}

fn allow_configuration(
    action: AuthAction<'_>,
    database: Option<&str>,
    role: ConnectionRole,
    statement: ConfigurationStatement,
) -> bool {
    if database.is_some() || !configuration_allowed(role, statement) {
        return false;
    }
    let AuthAction::Pragma {
        pragma_name,
        pragma_value,
    } = action
    else {
        return false;
    };
    (pragma_name, pragma_value) == statement.tuple()
}

fn allow_schema_statement(
    action: AuthAction<'_>,
    database: Option<&str>,
    role: ConnectionRole,
    statement: schema::SchemaStatement,
) -> bool {
    if matches!(
        statement,
        schema::SchemaStatement::CreateTable(_)
            | schema::SchemaStatement::CreateIndex(_)
            | schema::SchemaStatement::SetUserVersion
    ) && role != ConnectionRole::Creator
    {
        return false;
    }
    match statement {
        schema::SchemaStatement::CreateTable(table) => {
            allow_create_table_statement(action, database, table)
        }
        schema::SchemaStatement::CreateIndex(index) => {
            allow_create_index_statement(action, database, index)
        }
        schema::SchemaStatement::SetUserVersion => allow_exact_pragma(
            action,
            database,
            ConfigurationStatement::UserVersionRead.tuple().0,
            Some("3"),
        ),
        schema::SchemaStatement::ReadUserVersion => allow_exact_pragma(
            action,
            database,
            ConfigurationStatement::UserVersionRead.tuple().0,
            None,
        ),
        schema::SchemaStatement::SchemaObjects => match action {
            AuthAction::Select => database.is_none(),
            AuthAction::Read {
                table_name,
                column_name,
            } => {
                database == Some("main")
                    && table_name == "sqlite_master"
                    && matches!(
                        column_name,
                        "type" | "name" | "tbl_name" | "rootpage" | "sql"
                    )
            }
            _ => false,
        },
        schema::SchemaStatement::TableList => allow_pragma_query(
            action,
            database,
            "table_list",
            None,
            "pragma_table_list",
            &["schema", "name", "type", "ncol", "wr", "strict"],
        ),
        schema::SchemaStatement::TableInfo(table) => allow_pragma_query(
            action,
            database,
            "table_info",
            Some(table.name()),
            "pragma_table_info",
            &["cid", "name", "type", "notnull", "dflt_value", "pk"],
        ),
        schema::SchemaStatement::IndexList(table) => allow_pragma_query(
            action,
            database,
            "index_list",
            Some(table.name()),
            "pragma_index_list",
            &["seq", "name", "unique", "origin", "partial"],
        ),
        schema::SchemaStatement::ForeignKeyList(table) => allow_pragma_query(
            action,
            database,
            "foreign_key_list",
            Some(table.name()),
            "pragma_foreign_key_list",
            &[
                "id",
                "seq",
                "table",
                "from",
                "to",
                "on_update",
                "on_delete",
                "match",
            ],
        ),
        schema::SchemaStatement::IndexInfo(index) => allow_pragma_query(
            action,
            database,
            "index_info",
            Some(index.name()),
            "pragma_index_info",
            &["seqno", "cid", "name"],
        ),
        schema::SchemaStatement::IndexXinfo(index) => allow_pragma_query(
            action,
            database,
            "index_xinfo",
            Some(index.name()),
            "pragma_index_xinfo",
            &["seqno", "cid", "name", "desc", "coll", "key"],
        ),
        schema::SchemaStatement::IntegrityCheck => allow_pragma_query(
            action,
            database,
            "integrity_check",
            None,
            "pragma_integrity_check",
            &["integrity_check"],
        ),
        schema::SchemaStatement::ForeignKeyCheck => allow_pragma_query(
            action,
            database,
            "foreign_key_check",
            None,
            "pragma_foreign_key_check",
            &["table", "rowid", "parent", "fkid"],
        ),
    }
}

fn allow_exact_pragma(
    action: AuthAction<'_>,
    database: Option<&str>,
    expected_name: &str,
    expected_value: Option<&str>,
) -> bool {
    database.is_none()
        && matches!(
            action,
            AuthAction::Pragma {
                pragma_name,
                pragma_value,
            } if pragma_name == expected_name && pragma_value == expected_value
        )
}

fn allow_pragma_query(
    action: AuthAction<'_>,
    database: Option<&str>,
    pragma_name: &str,
    pragma_value: Option<&str>,
    virtual_table: &str,
    columns: &[&str],
) -> bool {
    match action {
        AuthAction::Select => database.is_none(),
        AuthAction::Pragma {
            pragma_name: actual_name,
            pragma_value: actual_value,
        } => database.is_none() && actual_name == pragma_name && actual_value == pragma_value,
        AuthAction::Read {
            table_name,
            column_name,
        } => {
            database == Some("main")
                && table_name == virtual_table
                && columns.contains(&column_name)
        }
        AuthAction::Function { function_name } => {
            database.is_none() && is_common_scalar_function(function_name)
        }
        _ => false,
    }
}

fn allow_create_table_statement(
    action: AuthAction<'_>,
    database: Option<&str>,
    table: schema::SchemaTable,
) -> bool {
    let table_name = table.name();
    match action {
        AuthAction::CreateTable { table_name: actual } => {
            database == Some("main") && actual == table_name
        }
        AuthAction::CreateIndex {
            index_name,
            table_name: actual,
        } => {
            database == Some("main")
                && actual == table_name
                && auto_index_for_table(table) == Some(index_name)
        }
        AuthAction::Insert {
            table_name: "sqlite_master",
        } => database == Some("main"),
        AuthAction::Update {
            table_name: "sqlite_master",
            column_name,
        } => {
            database == Some("main")
                && matches!(
                    column_name,
                    "type" | "name" | "tbl_name" | "rootpage" | "sql"
                )
        }
        AuthAction::Read {
            table_name: "sqlite_master",
            column_name: "ROWID",
        } => database == Some("main"),
        AuthAction::Read {
            table_name: actual,
            column_name,
        } => {
            database == Some("main")
                && actual == table_name
                && is_application_column(actual, column_name)
        }
        AuthAction::Select => database.is_none(),
        AuthAction::Function { function_name } => {
            database.is_none() && is_common_scalar_function(function_name)
        }
        _ => false,
    }
}

fn allow_create_index_statement(
    action: AuthAction<'_>,
    database: Option<&str>,
    index: schema::NamedSchemaIndex,
) -> bool {
    let index_name = index.name();
    let Some(table_name) = expected_index_table(index_name) else {
        return false;
    };
    match action {
        AuthAction::CreateIndex {
            index_name: actual_index,
            table_name: actual_table,
        } => database == Some("main") && actual_index == index_name && actual_table == table_name,
        AuthAction::Reindex { index_name: actual } => {
            database == Some("main") && actual == index_name
        }
        AuthAction::Insert {
            table_name: "sqlite_master",
        } => database == Some("main"),
        AuthAction::Update {
            table_name: "sqlite_master",
            column_name,
        } => {
            database == Some("main")
                && matches!(
                    column_name,
                    "type" | "name" | "tbl_name" | "rootpage" | "sql"
                )
        }
        AuthAction::Read {
            table_name: "sqlite_master",
            column_name: "ROWID",
        } => database == Some("main"),
        AuthAction::Read {
            table_name: actual_table,
            column_name,
        } => {
            database == Some("main")
                && actual_table == table_name
                && index_key_columns(index_name).contains(&column_name)
        }
        AuthAction::Select => database.is_none(),
        AuthAction::Function { function_name } => {
            database.is_none() && is_common_scalar_function(function_name)
        }
        _ => false,
    }
}

fn allow_projection_statement(
    action: AuthAction<'_>,
    database: Option<&str>,
    statement: ProjectionStatement,
) -> bool {
    match action {
        AuthAction::Select => database.is_none(),
        AuthAction::Function { function_name } => {
            database.is_none() && is_common_scalar_function(function_name)
        }
        AuthAction::Read {
            table_name,
            column_name,
        } => {
            database == Some("main")
                && is_projection_statement_read(statement, table_name, column_name)
        }
        AuthAction::Insert { table_name } => {
            database == Some("main") && projection_insert_table(statement) == Some(table_name)
        }
        AuthAction::Delete { table_name } => {
            database == Some("main") && projection_delete_table(statement) == Some(table_name)
        }
        AuthAction::Update {
            table_name,
            column_name,
        } => {
            database == Some("main")
                && projection_update_columns(statement).is_some_and(|(table, columns)| {
                    table == table_name && columns.contains(&column_name)
                })
        }
        _ => false,
    }
}

fn is_projection_statement_read(statement: ProjectionStatement, table: &str, column: &str) -> bool {
    let own_table = match statement {
        ProjectionStatement::InsertAnchor => "projection_anchor",
        ProjectionStatement::InsertBlock => "projected_blocks",
        ProjectionStatement::InsertEvent => "projected_events",
        ProjectionStatement::InsertCheckpoint | ProjectionStatement::UpdateCheckpoint => {
            "projection_checkpoint"
        }
        ProjectionStatement::InsertIntentUnit | ProjectionStatement::UpdateIntentUnit => {
            "intent_units"
        }
        ProjectionStatement::InsertRelationshipDefinition => "relationship_definitions",
        ProjectionStatement::InsertRelationship | ProjectionStatement::DeleteRelationship => {
            "intent_unit_relationships"
        }
        ProjectionStatement::InsertAssociation | ProjectionStatement::DeleteAssociation => {
            "recorded_associations"
        }
    };
    if table == own_table {
        return match statement {
            ProjectionStatement::DeleteRelationship => matches!(
                column,
                "definition_id" | "definition_version" | "source_id" | "target_id"
            ),
            ProjectionStatement::DeleteAssociation => matches!(
                column,
                "unit_id"
                    | "subject_kind"
                    | "subject_revision_key"
                    | "namespace"
                    | "scope"
                    | "value"
            ),
            _ => is_application_column(table, column),
        };
    }
    match statement {
        ProjectionStatement::InsertBlock => table == "projection_anchor" && column == "singleton",
        ProjectionStatement::InsertEvent => table == "projected_blocks" && column == "block_number",
        ProjectionStatement::InsertCheckpoint | ProjectionStatement::UpdateCheckpoint => {
            (table == "projected_blocks" && matches!(column, "block_number" | "block_hash"))
                || (table == "projected_events" && column == "global_sequence")
        }
        ProjectionStatement::InsertIntentUnit | ProjectionStatement::UpdateIntentUnit => {
            table == "projected_events" && column == "global_sequence"
        }
        ProjectionStatement::InsertRelationshipDefinition => {
            table == "projected_events" && column == "global_sequence"
        }
        ProjectionStatement::InsertRelationship => {
            (table == "relationship_definitions"
                && matches!(column, "definition_id" | "definition_version"))
                || (table == "intent_units" && column == "id")
                || (table == "projected_events" && column == "global_sequence")
        }
        ProjectionStatement::InsertAssociation => {
            (table == "intent_units" && column == "id")
                || (table == "projected_events" && column == "global_sequence")
        }
        ProjectionStatement::InsertAnchor
        | ProjectionStatement::DeleteRelationship
        | ProjectionStatement::DeleteAssociation => false,
    }
}

const fn projection_insert_table(statement: ProjectionStatement) -> Option<&'static str> {
    match statement {
        ProjectionStatement::InsertAnchor => Some("projection_anchor"),
        ProjectionStatement::InsertBlock => Some("projected_blocks"),
        ProjectionStatement::InsertEvent => Some("projected_events"),
        ProjectionStatement::InsertCheckpoint => Some("projection_checkpoint"),
        ProjectionStatement::InsertIntentUnit => Some("intent_units"),
        ProjectionStatement::InsertRelationshipDefinition => Some("relationship_definitions"),
        ProjectionStatement::InsertRelationship => Some("intent_unit_relationships"),
        ProjectionStatement::InsertAssociation => Some("recorded_associations"),
        ProjectionStatement::UpdateCheckpoint
        | ProjectionStatement::UpdateIntentUnit
        | ProjectionStatement::DeleteRelationship
        | ProjectionStatement::DeleteAssociation => None,
    }
}

const fn projection_delete_table(statement: ProjectionStatement) -> Option<&'static str> {
    match statement {
        ProjectionStatement::DeleteRelationship => Some("intent_unit_relationships"),
        ProjectionStatement::DeleteAssociation => Some("recorded_associations"),
        _ => None,
    }
}

const fn projection_update_columns(
    statement: ProjectionStatement,
) -> Option<(&'static str, &'static [&'static str])> {
    match statement {
        ProjectionStatement::UpdateCheckpoint => Some((
            "projection_checkpoint",
            &[
                "block_number",
                "block_hash",
                "last_global_sequence",
                "runtime_spec_version",
                "runtime_code_hash",
            ],
        )),
        ProjectionStatement::UpdateIntentUnit => Some((
            "intent_units",
            &[
                "envelope",
                "phase",
                "status",
                "revision",
                "last_global_sequence",
            ],
        )),
        _ => None,
    }
}

fn is_common_scalar_function(name: &str) -> bool {
    matches!(
        name,
        "typeof" | "length" | "instr" | "char" | "hex" | "coalesce" | "glob"
    )
}

const fn auto_index_for_table(table: schema::SchemaTable) -> Option<&'static str> {
    match table {
        schema::SchemaTable::ProjectedBlocks => Some("sqlite_autoindex_projected_blocks_1"),
        schema::SchemaTable::ProjectedEvents => Some("sqlite_autoindex_projected_events_1"),
        schema::SchemaTable::IntentUnits => Some("sqlite_autoindex_intent_units_1"),
        schema::SchemaTable::RelationshipDefinitions => {
            Some("sqlite_autoindex_relationship_definitions_1")
        }
        schema::SchemaTable::IntentUnitRelationships => {
            Some("sqlite_autoindex_intent_unit_relationships_1")
        }
        schema::SchemaTable::RecordedAssociations => {
            Some("sqlite_autoindex_recorded_associations_1")
        }
        schema::SchemaTable::ProjectionAnchor | schema::SchemaTable::ProjectionCheckpoint => None,
    }
}

fn expected_index_table(index: &str) -> Option<&'static str> {
    match index {
        "projected_blocks_by_hash"
        | "projected_blocks_by_number_hash"
        | "sqlite_autoindex_projected_blocks_1" => Some("projected_blocks"),
        "projected_events_by_sequence" | "sqlite_autoindex_projected_events_1" => {
            Some("projected_events")
        }
        "intent_units_by_workflow"
        | "intent_units_by_species"
        | "intent_units_by_phase"
        | "intent_units_by_status"
        | "sqlite_autoindex_intent_units_1" => Some("intent_units"),
        "sqlite_autoindex_relationship_definitions_1" => Some("relationship_definitions"),
        "relationship_edges_by_source"
        | "relationship_edges_by_target"
        | "sqlite_autoindex_intent_unit_relationships_1" => Some("intent_unit_relationships"),
        "recorded_associations_by_unit"
        | "recorded_associations_by_reference"
        | "sqlite_autoindex_recorded_associations_1" => Some("recorded_associations"),
        _ => None,
    }
}

fn index_key_columns(index: &str) -> &'static [&'static str] {
    match index {
        "projected_blocks_by_hash" => &["block_hash"],
        "projected_blocks_by_number_hash" => &["block_number", "block_hash"],
        "projected_events_by_sequence" => &["global_sequence"],
        "intent_units_by_workflow" => &["workflow_id", "id"],
        "intent_units_by_species" => &["species", "id"],
        "intent_units_by_phase" => &["phase", "id"],
        "intent_units_by_status" => &["status", "id"],
        "relationship_edges_by_source" => &[
            "definition_id",
            "definition_version",
            "source_id",
            "target_id",
        ],
        "relationship_edges_by_target" => &[
            "definition_id",
            "definition_version",
            "target_id",
            "source_id",
        ],
        "recorded_associations_by_unit" => &[
            "unit_id",
            "subject_kind",
            "subject_revision_key",
            "namespace",
            "scope",
            "value",
        ],
        "recorded_associations_by_reference" => &[
            "namespace",
            "scope",
            "value",
            "unit_id",
            "subject_kind",
            "subject_revision_key",
        ],
        _ => &[],
    }
}

fn is_application_column(table: &str, column: &str) -> bool {
    match table {
        "projection_anchor" => matches!(
            column,
            "singleton"
                | "namespace"
                | "relay_genesis_hash"
                | "parachain_genesis_hash"
                | "para_id"
                | "deployment_id"
                | "pallet_storage_version"
                | "event_schema_version"
                | "initial_runtime_spec_version"
                | "initial_runtime_code_hash"
        ),
        "projected_blocks" => matches!(
            column,
            "anchor_singleton"
                | "block_number"
                | "block_hash"
                | "parent_hash"
                | "runtime_spec_version"
                | "runtime_code_hash"
                | "cubikan_event_count"
                | "first_global_sequence"
                | "last_global_sequence"
        ),
        "projected_events" => matches!(
            column,
            "block_number"
                | "extrinsic_index"
                | "system_event_index"
                | "global_sequence"
                | "deployment_id"
                | "event_schema_version"
                | "event_kind"
                | "scale_payload"
                | "signer"
                | "extrinsic_hash"
        ),
        "projection_checkpoint" => matches!(
            column,
            "singleton"
                | "block_number"
                | "block_hash"
                | "last_global_sequence"
                | "runtime_spec_version"
                | "runtime_code_hash"
        ),
        "intent_units" => matches!(
            column,
            "id" | "envelope_version"
                | "envelope"
                | "origin_namespace"
                | "origin_scope"
                | "origin_value"
                | "workflow_id"
                | "species"
                | "phase"
                | "status"
                | "revision"
                | "last_global_sequence"
        ),
        "relationship_definitions" => matches!(
            column,
            "definition_id"
                | "definition_version"
                | "directed"
                | "source_species"
                | "target_species"
                | "self_policy"
                | "cycle_policy"
                | "created_global_sequence"
        ),
        "intent_unit_relationships" => matches!(
            column,
            "definition_id"
                | "definition_version"
                | "source_id"
                | "target_id"
                | "created_global_sequence"
        ),
        "recorded_associations" => matches!(
            column,
            "unit_id"
                | "subject_kind"
                | "subject_revision_key"
                | "namespace"
                | "scope"
                | "value"
                | "created_global_sequence"
        ),
        _ => false,
    }
}

#[cfg(test)]
fn classify_filesystem(
    platform: &str,
    canonical_directory: &Path,
    mountinfo: &str,
    statfs_magic: u64,
    vfs: VfsObservation<'_>,
) -> Result<MountIdentity, BackendError> {
    if platform != "linux" {
        return Err(BackendError::UnsupportedPlatform);
    }
    if !canonical_directory.is_absolute()
        || vfs.requested_name != SQLITE_VFS
        || !vfs.registered
        || !vfs.built_in
    {
        return Err(BackendError::InsecureProjectionPath);
    }
    classify_mount_observation(platform, canonical_directory, mountinfo, statfs_magic)
}

fn classify_mount_observation(
    platform: &str,
    canonical_directory: &Path,
    mountinfo: &str,
    statfs_magic: u64,
) -> Result<MountIdentity, BackendError> {
    if platform != "linux" {
        return Err(BackendError::UnsupportedPlatform);
    }
    if !canonical_directory.is_absolute() {
        return Err(BackendError::InsecureProjectionPath);
    }
    let mut selected: Option<(usize, MountIdentity)> = None;
    let mut ambiguous = false;
    let mut saw_line = false;
    for line in mountinfo.lines() {
        saw_line = true;
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        let Some(separator) = fields.iter().position(|field| *field == "-") else {
            return Err(BackendError::InsecureProjectionPath);
        };
        if separator < 6 || fields.len() < separator + 4 {
            return Err(BackendError::InsecureProjectionPath);
        }
        let mount_point = decode_mount_path(fields[4])?;
        if canonical_directory != mount_point && !canonical_directory.starts_with(&mount_point) {
            continue;
        }
        let length = mount_point.as_os_str().len();
        let identity = MountIdentity {
            mount_point,
            filesystem_type: fields[separator + 1].to_owned(),
        };
        match &selected {
            None => {
                selected = Some((length, identity));
                ambiguous = false;
            }
            Some((best, _)) if length > *best => {
                selected = Some((length, identity));
                ambiguous = false;
            }
            Some((best, _)) if length == *best => ambiguous = true,
            Some(_) => {}
        }
    }
    if !saw_line || ambiguous {
        return Err(BackendError::InsecureProjectionPath);
    }
    let Some((_, identity)) = selected else {
        return Err(BackendError::InsecureProjectionPath);
    };
    let expected_magic = match identity.filesystem_type.as_str() {
        "ext2" | "ext3" | "ext4" => 0x0000_0000_0000_ef53,
        "xfs" => 0x0000_0000_5846_5342,
        "btrfs" => 0x0000_0000_9123_683e,
        _ => return Err(BackendError::InsecureProjectionPath),
    };
    if statfs_magic != expected_magic {
        return Err(BackendError::InsecureProjectionPath);
    }
    Ok(identity)
}

fn decode_mount_path(field: &str) -> Result<PathBuf, BackendError> {
    let input = field.as_bytes();
    let mut decoded = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        if input[index] != b'\\' {
            decoded.push(input[index]);
            index += 1;
            continue;
        }
        if index + 3 >= input.len() {
            return Err(BackendError::InsecureProjectionPath);
        }
        let escape = &input[index + 1..=index + 3];
        decoded.push(match escape {
            b"040" => b' ',
            b"011" => b'\t',
            b"012" => b'\n',
            b"134" => b'\\',
            _ => return Err(BackendError::InsecureProjectionPath),
        });
        index += 4;
    }
    String::from_utf8(decoded)
        .map(PathBuf::from)
        .map_err(|_| BackendError::InsecureProjectionPath)
}

fn validate_direct_child(
    canonical_directory: &Path,
    basename: &OsStr,
) -> Result<PathBuf, BackendError> {
    let child = Path::new(basename);
    let mut components = child.components();
    if basename.is_empty()
        || basename.as_encoded_bytes().contains(&0)
        || !matches!(components.next(), Some(std::path::Component::Normal(name)) if name == basename)
        || components.next().is_some()
        || !canonical_directory.is_absolute()
    {
        return Err(BackendError::InsecureProjectionPath);
    }
    Ok(canonical_directory.join(child))
}

#[cfg(target_os = "linux")]
mod secure_fs {
    use std::{
        ffi::{OsStr, OsString},
        os::fd::OwnedFd,
        path::{Path, PathBuf},
    };

    use rustix::{
        fs::{self, AtFlags, CWD, FileType, Mode, OFlags},
        io::{self, Errno},
        process,
    };

    use super::{
        BackendError, DATABASE_HEADER_LENGTH, DATABASE_PAGE_SIZE, MAX_DATABASE_BYTES,
        validate_direct_child,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(super) struct FileIdentity {
        device: u64,
        inode: u64,
    }

    pub(super) struct SecurePath {
        directory: OwnedFd,
        directory_identity: FileIdentity,
        canonical_directory: PathBuf,
        basename: OsString,
        sqlite_path: PathBuf,
        cleanup_identity: Option<FileIdentity>,
        cleanup_sidecar_identities: [Option<FileIdentity>; 3],
    }

    impl SecurePath {
        pub(super) fn for_creation(
            directory: &Path,
            basename: &OsStr,
        ) -> Result<Self, BackendError> {
            Self::validate(directory, basename)
        }

        pub(super) fn for_existing(
            directory: &Path,
            basename: &OsStr,
        ) -> Result<Self, BackendError> {
            Self::validate(directory, basename)
        }

        fn validate(directory: &Path, basename: &OsStr) -> Result<Self, BackendError> {
            if !directory.is_absolute() {
                return Err(BackendError::InsecureProjectionPath);
            }
            let before = fs::statat(CWD, directory, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| BackendError::InsecureProjectionPath)?;
            validate_directory_stat(&before)?;
            let canonical_directory = std::fs::canonicalize(directory)
                .map_err(|_| BackendError::InsecureProjectionPath)?;
            if canonical_directory != directory {
                return Err(BackendError::InsecureProjectionPath);
            }
            let sqlite_path = validate_direct_child(&canonical_directory, basename)?;
            let directory_fd = fs::openat(
                CWD,
                &canonical_directory,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| BackendError::InsecureProjectionPath)?;
            let opened =
                fs::fstat(&directory_fd).map_err(|_| BackendError::InsecureProjectionPath)?;
            validate_directory_stat(&opened)?;
            let directory_identity = identity(&opened);
            if directory_identity != identity(&before) {
                return Err(BackendError::InsecureProjectionPath);
            }
            let mountinfo = std::fs::read_to_string("/proc/self/mountinfo")
                .map_err(|_| BackendError::UnsupportedPlatform)?;
            let statfs =
                fs::fstatfs(&directory_fd).map_err(|_| BackendError::InsecureProjectionPath)?;
            super::classify_mount_observation(
                "linux",
                &canonical_directory,
                &mountinfo,
                (statfs.f_type as u64) & 0xffff_ffff,
            )?;
            let path = Self {
                directory: directory_fd,
                directory_identity,
                canonical_directory,
                basename: basename.to_owned(),
                sqlite_path,
                cleanup_identity: None,
                cleanup_sidecar_identities: [None; 3],
            };
            path.validate_stable_directory()?;
            path.require_sidecars_absent()?;
            Ok(path)
        }

        pub(super) fn sqlite_path(&self) -> &Path {
            &self.sqlite_path
        }

        pub(super) fn duplicate(&self) -> Result<Self, BackendError> {
            Ok(Self {
                directory: io::dup(&self.directory)
                    .map_err(|_| BackendError::InsecureProjectionPath)?,
                directory_identity: self.directory_identity,
                canonical_directory: self.canonical_directory.clone(),
                basename: self.basename.clone(),
                sqlite_path: self.sqlite_path.clone(),
                cleanup_identity: None,
                cleanup_sidecar_identities: [None; 3],
            })
        }

        pub(super) fn create_exclusive_file(&mut self) -> Result<FileIdentity, BackendError> {
            self.validate_stable_directory()?;
            self.require_sidecars_absent()?;
            match fs::statat(&self.directory, &self.basename, AtFlags::SYMLINK_NOFOLLOW) {
                Err(Errno::NOENT) => {}
                _ => return Err(BackendError::InsecureProjectionPath),
            }
            let file = fs::openat(
                &self.directory,
                &self.basename,
                OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|_| BackendError::InsecureProjectionPath)?;
            let stat = fs::fstat(&file).map_err(|_| BackendError::InsecureProjectionPath)?;
            validate_file_stat(&stat)?;
            let file_identity = identity(&stat);
            self.cleanup_identity = Some(file_identity);
            let linked = fs::statat(&self.directory, &self.basename, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| BackendError::InsecureProjectionPath)?;
            if identity(&linked) != file_identity {
                return Err(BackendError::InsecureProjectionPath);
            }
            fs::fsync(&file).map_err(|_| BackendError::InsecureProjectionPath)?;
            fs::fsync(&self.directory).map_err(|_| BackendError::InsecureProjectionPath)?;
            Ok(file_identity)
        }

        pub(super) fn arm_creation_cleanup(&mut self, identity: FileIdentity) {
            self.cleanup_identity = Some(identity);
        }

        pub(super) fn disarm_creation_cleanup(&mut self) {
            self.cleanup_identity = None;
            self.cleanup_sidecar_identities = [None; 3];
        }

        pub(super) fn capture_creator_sidecars(&mut self) -> Result<(), BackendError> {
            self.validate_stable_directory()?;
            for (index, suffix) in ["-journal", "-wal", "-shm"].into_iter().enumerate() {
                let mut sidecar = self.basename.clone();
                sidecar.push(suffix);
                match fs::statat(&self.directory, &sidecar, AtFlags::SYMLINK_NOFOLLOW) {
                    Ok(current) => {
                        validate_file_stat(&current)?;
                        let observed = identity(&current);
                        if self.cleanup_sidecar_identities[index]
                            .is_some_and(|expected| expected != observed)
                        {
                            return Err(BackendError::InsecureProjectionPath);
                        }
                        self.cleanup_sidecar_identities[index] = Some(observed);
                    }
                    Err(Errno::NOENT) => {}
                    Err(_) => return Err(BackendError::InsecureProjectionPath),
                }
            }
            Ok(())
        }

        pub(super) fn validate_existing_file(
            &self,
            expected: Option<FileIdentity>,
            require_header: bool,
        ) -> Result<i64, BackendError> {
            self.validate_stable_directory()?;
            self.require_sidecars_absent()?;
            let file = fs::openat(
                &self.directory,
                &self.basename,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| BackendError::InsecureProjectionPath)?;
            let stat = fs::fstat(&file).map_err(|_| BackendError::InsecureProjectionPath)?;
            validate_file_stat(&stat)?;
            let file_identity = identity(&stat);
            if expected.is_some_and(|expected| expected != file_identity) {
                return Err(BackendError::InsecureProjectionPath);
            }
            let linked = fs::statat(&self.directory, &self.basename, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| BackendError::InsecureProjectionPath)?;
            if identity(&linked) != file_identity {
                return Err(BackendError::InsecureProjectionPath);
            }
            if require_header {
                validate_header(&file, stat.st_size)?;
            }
            self.validate_stable_directory()?;
            Ok(stat.st_size)
        }

        pub(super) fn file_size(&self) -> Result<i64, BackendError> {
            self.validate_existing_file(None, true)
        }

        pub(super) fn sync_directory(&self) -> Result<(), BackendError> {
            fs::fsync(&self.directory).map_err(|_| BackendError::InsecureProjectionPath)
        }

        fn validate_stable_directory(&self) -> Result<(), BackendError> {
            let current = fs::statat(CWD, &self.canonical_directory, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| BackendError::InsecureProjectionPath)?;
            validate_directory_stat(&current)?;
            if identity(&current) != self.directory_identity {
                return Err(BackendError::InsecureProjectionPath);
            }
            Ok(())
        }

        fn require_sidecars_absent(&self) -> Result<(), BackendError> {
            for suffix in ["-journal", "-wal", "-shm"] {
                let mut sidecar = self.basename.clone();
                sidecar.push(suffix);
                match fs::statat(&self.directory, &sidecar, AtFlags::SYMLINK_NOFOLLOW) {
                    Err(Errno::NOENT) => {}
                    _ => return Err(BackendError::InsecureProjectionPath),
                }
            }
            Ok(())
        }

        fn cleanup_created_file(&mut self) {
            let Some(expected) = self.cleanup_identity.take() else {
                return;
            };
            let mut removed = false;
            for (suffix, expected_sidecar) in ["-journal", "-wal", "-shm"]
                .into_iter()
                .zip(self.cleanup_sidecar_identities)
            {
                let Some(expected_sidecar) = expected_sidecar else {
                    continue;
                };
                let mut sidecar = self.basename.clone();
                sidecar.push(suffix);
                if let Ok(current) =
                    fs::statat(&self.directory, &sidecar, AtFlags::SYMLINK_NOFOLLOW)
                    && identity(&current) == expected_sidecar
                    && FileType::from_raw_mode(current.st_mode) == FileType::RegularFile
                    && fs::unlinkat(&self.directory, &sidecar, AtFlags::empty()).is_ok()
                {
                    removed = true;
                }
            }
            if let Ok(current) =
                fs::statat(&self.directory, &self.basename, AtFlags::SYMLINK_NOFOLLOW)
                && identity(&current) == expected
                && FileType::from_raw_mode(current.st_mode) == FileType::RegularFile
                && fs::unlinkat(&self.directory, &self.basename, AtFlags::empty()).is_ok()
            {
                removed = true;
            }
            if removed {
                let _ = fs::fsync(&self.directory);
            }
        }
    }

    impl Drop for SecurePath {
        fn drop(&mut self) {
            self.cleanup_created_file();
        }
    }

    fn identity(stat: &fs::Stat) -> FileIdentity {
        FileIdentity {
            device: stat.st_dev,
            inode: stat.st_ino,
        }
    }

    fn validate_directory_stat(stat: &fs::Stat) -> Result<(), BackendError> {
        if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
            || stat.st_uid != process::geteuid().as_raw()
            || Mode::from_raw_mode(stat.st_mode) != Mode::RWXU
        {
            return Err(BackendError::InsecureProjectionPath);
        }
        Ok(())
    }

    fn validate_file_stat(stat: &fs::Stat) -> Result<(), BackendError> {
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
            || stat.st_uid != process::geteuid().as_raw()
            || Mode::from_raw_mode(stat.st_mode) != Mode::from_raw_mode(0o600)
        {
            return Err(BackendError::InsecureProjectionPath);
        }
        Ok(())
    }

    fn validate_header(file: &OwnedFd, file_size: i64) -> Result<(), BackendError> {
        if !(DATABASE_PAGE_SIZE..=MAX_DATABASE_BYTES).contains(&file_size)
            || file_size % DATABASE_PAGE_SIZE != 0
        {
            return Err(BackendError::CorruptSchema);
        }
        let mut header = [0_u8; DATABASE_HEADER_LENGTH];
        let read = io::pread(file, &mut header[..], 0)
            .map_err(|_| BackendError::InsecureProjectionPath)?;
        if read != DATABASE_HEADER_LENGTH
            || &header[..16] != b"SQLite format 3\0"
            || header[16..18] != [0x10, 0x00]
            || header[18] != 1
            || header[19] != 1
            || header[20] != 0
            || header[56..60] != [0, 0, 0, 1]
        {
            return Err(BackendError::CorruptSchema);
        }
        Ok(())
    }
}

/// Temporary fail-closed bridge for the retired schema-v1/v2 backend.
///
/// This type is deliberately unconstructible outside this crate. T-1108
/// replaces it with the fresh-only schema-v3 projection reader; until then no
/// root process can regain the removed SQLite write authority.
#[derive(Debug)]
pub struct SqliteBackend {
    connection: Connection,
    _private: (),
}

impl SqliteBackend {
    /// Rejects the retired backend generation before inspecting or creating a path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, BackendError> {
        reject_retired_schema()?;
        let _ = path.as_ref();
        retain_retired_implementation_symbols();
        Err(retired_schema())
    }

    /// Returns the last historical schema identity represented by this bridge.
    #[must_use]
    pub const fn schema_version(&self) -> BackendSchemaVersion {
        BackendSchemaVersion::V2
    }

    /// Rejects the retired originless migration before inspecting its path.
    pub fn migrate_v1_to_v2(path: impl AsRef<Path>) -> Result<(), MigrationError> {
        migration::migrate_v1_to_v2(path.as_ref())
    }

    pub(crate) fn require_relationship_schema(&self) -> Result<(), RelationshipError> {
        Err(RelationshipError::Backend(retired_schema()))
    }

    pub fn create_relationship_definition(
        &mut self,
        _command: CreateRelationshipDefinition,
    ) -> Result<RelationshipDefinitionView, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::create_definition(&mut self.connection, _command)
    }

    pub fn get_relationship_definition(
        &self,
        _key: RelationshipDefinitionKey,
    ) -> Result<RelationshipDefinitionView, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::get_definition(&self.connection, _key)
    }

    pub fn create_relationship(
        &mut self,
        _command: CreateRelationship,
    ) -> Result<RelationshipView, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::create_relationship(&mut self.connection, _command)
    }

    pub fn delete_relationship(
        &mut self,
        _command: DeleteRelationship,
    ) -> Result<RelationshipView, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::delete_relationship(&mut self.connection, _command)
    }

    pub fn list_relationships(
        &self,
        _query: ListRelationships,
    ) -> Result<RelationshipPage, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::list_relationships(&self.connection, _query)
    }

    pub fn project(&self, _query: ProjectionQueryV1) -> Result<ProjectionPage, RelationshipError> {
        self.require_relationship_schema()?;
        relationship_store::project(&self.connection, _query)
    }

    /// Rejects the originless historical create command before ID generation or SQL.
    pub fn create(&mut self, command: CreateIntentUnit) -> Result<IntentUnitView, BackendError> {
        reject_retired_schema()?;
        let _ = command.into_parts();
        retain_retired_envelope_symbols();
        Err(retired_schema())
    }

    pub fn get(&self, id: IntentUnitId) -> Result<IntentUnitView, BackendError> {
        reject_retired_schema()?;
        let unit = load_validated_unit(&self.connection, id)?;
        Ok(IntentUnitView::from_intent_unit(&unit))
    }

    pub fn list(&self, command: ListIntentUnits) -> Result<IntentUnitPage, BackendError> {
        reject_retired_schema()?;
        query::list(&self.connection, &command)
    }

    /// Rejects the historical canonical mutation surface before SQLite access.
    pub fn transition(
        &mut self,
        _command: TransitionIntentUnit,
    ) -> Result<MutationResult, BackendError> {
        reject_retired_schema()?;
        Err(retired_schema())
    }

    /// Rejects the historical canonical mutation surface before SQLite access.
    pub fn complete(
        &mut self,
        _command: CompleteIntentUnit,
    ) -> Result<MutationResult, BackendError> {
        reject_retired_schema()?;
        Err(retired_schema())
    }
}

const fn retired_schema() -> BackendError {
    BackendError::UnsupportedSchemaVersion {
        found: RETIRED_SCHEMA_VERSION,
    }
}

fn reject_retired_schema() -> Result<(), BackendError> {
    Err(retired_schema())
}

fn retain_retired_implementation_symbols() {
    let _ = (
        schema::SCHEMA_VERSION,
        schema::APPLICATION_TABLES,
        schema::NAMED_INDEXES,
        schema::user_version,
        schema::initialize_v3,
        schema::validate_v3,
    );
    let _: fn(&Path, &OsStr) -> Result<ProjectionWriterConnection, BackendError> =
        create_fresh_projection;
    let _: fn(&Path, &OsStr) -> Result<ProjectionWriterConnection, BackendError> =
        open_projection_writer;
    let _: fn(&Path, &OsStr) -> Result<ProjectionReaderConnection, BackendError> =
        open_projection_reader;
    let _: fn(&ProjectionReaderConnection) -> Result<i64, BackendError> =
        ProjectionReaderConnection::data_version;
    let _: fn(&mut ProjectionReaderConnection) -> Result<(), BackendError> =
        ProjectionReaderConnection::begin_verified_read;
    let _: fn(&mut ProjectionReaderConnection) -> Result<(), BackendError> =
        ProjectionReaderConnection::rollback_verified_read;
    let _ = (
        VerifiedQueryStatement::OneBoundedRead,
        VerifiedQueryStatement::FullProjectionCompare,
    );
    retain_verified_query_bridge();
    crate::projection_store::retain_projection_store_symbols::<ProjectionWriterConnection>();
}

fn retain_verified_query_bridge() {
    fn operation(_: &Connection) -> Result<(), BackendError> {
        Ok(())
    }
    let _ = ProjectionReaderConnection::with_verified_query::<
        (),
        fn(&Connection) -> Result<(), BackendError>,
    >;
    let _ = operation as fn(&Connection) -> Result<(), BackendError>;
}

fn retain_retired_envelope_symbols() {
    let _ = (
        stored::ENVELOPE_VERSION,
        stored::encode_envelope,
        stored::decode_envelope,
        stored::encode_revision_text,
        stored::decode_revision_text,
        stored::encode_revision_blob,
        stored::decode_revision_blob,
    );
}

/// Retained row decoder used only to classify historical query results as
/// unsupported. It never replays an originless envelope.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StoredRow {
    id: String,
    envelope_version: i64,
    envelope: String,
    workflow_id: String,
    species: String,
    phase: String,
    status: String,
    revision: Vec<u8>,
}

impl StoredRow {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Self::from_row_at(row, 0)
    }

    pub(crate) fn from_row_at(row: &Row<'_>, offset: usize) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(offset)?,
            envelope_version: row.get(offset + 1)?,
            envelope: row.get(offset + 2)?,
            workflow_id: row.get(offset + 3)?,
            species: row.get(offset + 4)?,
            phase: row.get(offset + 5)?,
            status: row.get(offset + 6)?,
            revision: row.get(offset + 7)?,
        })
    }

    pub(crate) fn optional_from_row_at(
        row: &Row<'_>,
        offset: usize,
    ) -> rusqlite::Result<Option<Self>> {
        let Some(id) = row.get::<_, Option<String>>(offset)? else {
            return Ok(None);
        };
        Ok(Some(Self {
            id,
            envelope_version: row.get(offset + 1)?,
            envelope: row.get(offset + 2)?,
            workflow_id: row.get(offset + 3)?,
            species: row.get(offset + 4)?,
            phase: row.get(offset + 5)?,
            status: row.get(offset + 6)?,
            revision: row.get(offset + 7)?,
        }))
    }

    pub(crate) fn into_validated_unit(self) -> Result<IntentUnit, BackendError> {
        let Self {
            id,
            envelope_version,
            envelope,
            workflow_id,
            species,
            phase,
            status,
            revision,
        } = self;
        let _ = (
            id,
            envelope_version,
            envelope,
            workflow_id,
            species,
            phase,
            status,
            revision,
        );
        Err(BackendError::UnsupportedEnvelopeVersion { found: 1 })
    }
}

pub(crate) fn load_validated_unit(
    _connection: &rusqlite::Connection,
    _id: IntentUnitId,
) -> Result<IntentUnit, BackendError> {
    Err(BackendError::UnsupportedEnvelopeVersion { found: 1 })
}

pub(crate) const fn status_projection(status: IntentUnitStatus) -> &'static str {
    match status {
        IntentUnitStatus::Active => "active",
        IntentUnitStatus::Completed => "completed",
    }
}

pub(crate) fn classify_runtime_error(error: rusqlite::Error) -> BackendError {
    if is_busy_error(&error) {
        BackendError::StorageBusy(StorageFailure::new(error))
    } else if matches!(
        &error,
        rusqlite::Error::SqliteFailure(failure, _) if failure.code == ErrorCode::DiskFull
    ) {
        BackendError::StorageFull(StorageFailure::new(error))
    } else {
        BackendError::storage(error)
    }
}

fn is_busy_error(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(failure.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
    )
}

pub(crate) fn is_corrupt_database_error(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(failure.code, ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase)
    )
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::{OsStr, OsString},
        fs::{self, DirBuilder, OpenOptions},
        io::Write,
        os::unix::fs::{DirBuilderExt, FileExt, OpenOptionsExt, PermissionsExt},
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant},
    };

    use rusqlite::{OptionalExtension, params};
    use serde_json::Value;

    use crate::projection_store::{
        self, ProjectedBlock, ProjectedEvent, ProjectedEventKind, ProjectionAnchor,
    };

    use super::*;

    const FILESYSTEM_FIXTURE: &str =
        include_str!("../../../tests/fixtures/filesystem-boundary-v1.json");
    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    struct SupportedDirectory {
        path: PathBuf,
    }

    impl SupportedDirectory {
        fn new(label: &str) -> Option<Self> {
            let Some(root) = std::env::var_os("CUBIKAN_TEST_SUPPORTED_ROOT") else {
                eprintln!(
                    "CUBIKAN_TEST_SUPPORTED_ROOT is not set; the real ext4/xfs/btrfs branch is pending the declared supported-root run"
                );
                return None;
            };
            let root = fs::canonicalize(root).expect("supported test root must canonicalize");
            let path = root.join(format!(
                "cubikan-t1108-{label}-{}-{}",
                std::process::id(),
                TEST_ID.fetch_add(1, Ordering::Relaxed)
            ));
            let mut builder = DirBuilder::new();
            builder.mode(0o700);
            builder
                .create(&path)
                .expect("create test-owned supported directory");
            assert_eq!(
                fs::metadata(&path)
                    .expect("supported directory metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            Some(Self { path })
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for SupportedDirectory {
        fn drop(&mut self) {
            if let Ok(entries) = fs::read_dir(&self.path) {
                for entry in entries.flatten() {
                    let _ = fs::remove_file(entry.path());
                }
            }
            let _ = fs::remove_dir(&self.path);
        }
    }

    fn fixture() -> Value {
        serde_json::from_str(FILESYSTEM_FIXTURE).expect("filesystem fixture must be valid JSON")
    }

    fn hex_u64(value: &Value) -> u64 {
        u64::from_str_radix(
            value
                .as_str()
                .expect("hex string")
                .strip_prefix("0x")
                .expect("0x prefix"),
            16,
        )
        .expect("valid u64 hex")
    }

    fn directory_names(path: &Path) -> Vec<OsString> {
        let mut names = fs::read_dir(path)
            .expect("read test directory")
            .map(|entry| entry.expect("directory entry").file_name())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    fn create_mode_0600(path: &Path, bytes: &[u8]) {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .expect("create test-owned mode-0600 file");
        file.write_all(bytes).expect("write test-owned bytes");
        file.sync_all().expect("sync test-owned bytes");
    }

    fn assert_projection_empty(reader: &mut ProjectionReaderConnection) {
        reader.begin_verified_read().expect("begin verified read");
        assert!(matches!(
            reader.data_version(),
            Err(BackendError::CorruptSchema)
        ));
        reader
            .with_verified_query(
                VerifiedQueryStatement::FullProjectionCompare,
                |connection| {
                    for statement in [
                        "SELECT singleton FROM projection_anchor LIMIT 1",
                        "SELECT block_number FROM projected_blocks LIMIT 1",
                        "SELECT global_sequence FROM projected_events LIMIT 1",
                        "SELECT singleton FROM projection_checkpoint LIMIT 1",
                        "SELECT id FROM intent_units LIMIT 1",
                        "SELECT definition_id FROM relationship_definitions LIMIT 1",
                        "SELECT definition_id FROM intent_unit_relationships LIMIT 1",
                        "SELECT unit_id FROM recorded_associations LIMIT 1",
                    ] {
                        let row = connection
                            .query_row(statement, [], |row| row.get::<_, rusqlite::types::Value>(0))
                            .optional()
                            .map_err(classify_runtime_error)?;
                        if row.is_some() {
                            return Err(BackendError::ProjectionMismatch);
                        }
                    }
                    Ok(())
                },
            )
            .expect("all application tables start empty");
        reader
            .rollback_verified_read()
            .expect("rollback verified read");
    }

    fn assert_no_sidecars(directory: &Path, basename: &OsStr) {
        for suffix in ["-journal", "-wal", "-shm"] {
            let mut sidecar = basename.to_os_string();
            sidecar.push(suffix);
            assert!(!directory.join(sidecar).exists());
        }
    }

    #[test]
    fn test_fresh_linux_schema_v3_is_exact_and_empty() {
        let fixture = fixture();
        let cases = fixture["mount_classifier_cases"]
            .as_array()
            .expect("mount classifier cases");
        assert_eq!(cases.len(), 34);
        for case in cases {
            let mountinfo = case["mountinfo_lines"]
                .as_array()
                .expect("mountinfo lines")
                .iter()
                .map(|line| line.as_str().expect("mountinfo line"))
                .collect::<Vec<_>>()
                .join("\n");
            let vfs = &case["sqlite_vfs"];
            let result = classify_filesystem(
                case["platform"].as_str().expect("platform"),
                Path::new(
                    case["canonical_directory"]
                        .as_str()
                        .expect("canonical directory"),
                ),
                &mountinfo,
                hex_u64(&case["statfs_magic"]),
                VfsObservation {
                    requested_name: vfs["requested_name"].as_str().expect("VFS name"),
                    registered: vfs["registered"].as_bool().expect("registered flag"),
                    built_in: vfs["built_in"].as_bool().expect("built-in flag"),
                },
            );
            match case["expected"]["decision"]
                .as_str()
                .expect("expected decision")
            {
                "accept" => {
                    let identity = result.unwrap_or_else(|error| {
                        panic!("{} unexpectedly rejected: {error:?}", case["id"])
                    });
                    assert_eq!(
                        identity.mount_point,
                        PathBuf::from(
                            case["expected"]["selected_mount_point"]
                                .as_str()
                                .expect("selected mount")
                        )
                    );
                    assert_eq!(
                        identity.filesystem_type,
                        case["expected"]["filesystem_type"]
                            .as_str()
                            .expect("filesystem type")
                    );
                }
                "reject_before_access" => assert!(result.is_err(), "{} accepted", case["id"]),
                other => panic!("unknown fixture decision {other}"),
            }
        }

        let Some(directory) = SupportedDirectory::new("fresh") else {
            return;
        };
        let basename = OsStr::new("projection.sqlite3");
        let writer = create_fresh_projection(directory.path(), basename)
            .expect("create exact fresh schema-v3 projection");
        assert_eq!(
            writer
                .connection()
                .query_row("PRAGMA main.user_version", [], |row| row.get::<_, i64>(0))
                .expect_err("deny scope must block unscoped test SQL")
                .sqlite_error_code(),
            Some(ErrorCode::AuthorizationForStatementDenied)
        );
        drop(writer);

        let metadata = fs::metadata(directory.path().join(basename)).expect("projection metadata");
        assert!(metadata.is_file());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        assert_no_sidecars(directory.path(), basename);

        let mut reader = open_projection_reader(directory.path(), basename)
            .expect("reopen exact fresh projection read-only");
        let _ = reader
            .data_version()
            .expect("data_version is available before the snapshot");
        validate_schema_scoped(
            &reader.connection,
            &reader.authorization,
            ConnectionRole::PublicReader,
        )
        .expect("exact schema-v3 validation");
        assert_projection_empty(&mut reader);
        let _ = reader
            .data_version()
            .expect("data_version is available after snapshot rollback");
        drop(reader);

        let fault_basename = OsStr::new("fault.sqlite3");
        let mut fault_path = secure_fs::SecurePath::for_creation(directory.path(), fault_basename)
            .expect("validate fault-injection path");
        let identity = fault_path
            .create_exclusive_file()
            .expect("create fault-injection inode");
        assert!(matches!(
            create_fresh_projection_at(fault_path, identity, CreationFault::AfterSchema),
            Err(BackendError::CorruptSchema)
        ));
        assert!(!directory.path().join(fault_basename).exists());
        assert_no_sidecars(directory.path(), fault_basename);
    }

    #[test]
    fn test_existing_projection_preflight_order_is_read_only_and_fail_closed() {
        let wrong_configuration = authorize(
            AuthContext {
                action: AuthAction::Pragma {
                    pragma_name: "page_count",
                    pragma_value: None,
                },
                database_name: None,
                accessor: None,
            },
            AuthorizationState {
                role: ConnectionRole::PreflightReader,
                scope: AuthorizationScope::Configuration(ConfigurationStatement::PageSizeRead),
            },
        );
        assert_eq!(wrong_configuration, Authorization::Deny);
        let wrong_schema_column = authorize(
            AuthContext {
                action: AuthAction::Read {
                    table_name: "pragma_table_info",
                    column_name: "bogus",
                },
                database_name: Some("main"),
                accessor: None,
            },
            AuthorizationState {
                role: ConnectionRole::ProjectorWriter,
                scope: AuthorizationScope::Schema(schema::SchemaStatement::TableInfo(
                    schema::SchemaTable::IntentUnits,
                )),
            },
        );
        assert_eq!(wrong_schema_column, Authorization::Deny);
        let wrong_schema_value = authorize(
            AuthContext {
                action: AuthAction::Pragma {
                    pragma_name: "table_info",
                    pragma_value: Some("projected_blocks"),
                },
                database_name: None,
                accessor: None,
            },
            AuthorizationState {
                role: ConnectionRole::ProjectorWriter,
                scope: AuthorizationScope::Schema(schema::SchemaStatement::TableInfo(
                    schema::SchemaTable::IntentUnits,
                )),
            },
        );
        assert_eq!(wrong_schema_value, Authorization::Deny);
        let wrong_transaction = authorize(
            AuthContext {
                action: AuthAction::Transaction {
                    operation: TransactionOperation::Commit,
                },
                database_name: None,
                accessor: None,
            },
            AuthorizationState {
                role: ConnectionRole::ProjectorWriter,
                scope: AuthorizationScope::Transaction(TransactionStatement::Begin),
            },
        );
        assert_eq!(wrong_transaction, Authorization::Deny);
        let wrong_projection = authorize(
            AuthContext {
                action: AuthAction::Read {
                    table_name: "intent_units",
                    column_name: "id",
                },
                database_name: Some("main"),
                accessor: None,
            },
            AuthorizationState {
                role: ConnectionRole::ProjectorWriter,
                scope: AuthorizationScope::Projection(ProjectionStatement::InsertAnchor),
            },
        );
        assert_eq!(wrong_projection, Authorization::Deny);

        let Some(directory) = SupportedDirectory::new("preflight") else {
            return;
        };
        let basename = OsStr::new("projection.sqlite3");
        drop(
            create_fresh_projection(directory.path(), basename).expect("create preflight fixture"),
        );

        let path = directory.path().join(basename);
        let file = OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("open header fixture");
        file.write_all_at(&[1], 20).expect("corrupt reserved byte");
        file.sync_all().expect("sync header corruption");
        let bytes = fs::read(&path).expect("snapshot corrupt bytes");
        let names = directory_names(directory.path());
        assert!(matches!(
            open_projection_reader(directory.path(), basename),
            Err(BackendError::CorruptSchema)
        ));
        assert_eq!(fs::read(&path).expect("unchanged corrupt bytes"), bytes);
        assert_eq!(directory_names(directory.path()), names);
        assert_no_sidecars(directory.path(), basename);

        fs::remove_file(&path).expect("remove corrupt fixture");
        drop(create_fresh_projection(directory.path(), basename).expect("recreate schema fixture"));
        let mutation = Connection::open_with_flags_and_vfs(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
            SQLITE_VFS,
        )
        .expect("open test-owned schema fixture");
        mutation
            .execute_batch("CREATE TABLE unexpected(value INTEGER) STRICT")
            .expect("inject extra schema object");
        drop(mutation);
        let bytes = fs::read(&path).expect("snapshot extra-object bytes");
        let names = directory_names(directory.path());
        assert!(open_projection_reader(directory.path(), basename).is_err());
        assert_eq!(
            fs::read(&path).expect("unchanged extra-object bytes"),
            bytes
        );
        assert_eq!(directory_names(directory.path()), names);
        assert_no_sidecars(directory.path(), basename);
    }

    #[test]
    fn test_projection_paths_sidecars_features_and_uri_surface_fail_closed() {
        validate_runtime_boundary().expect("exact bundled SQLite runtime and built-in unix VFS");
        let fixture = fixture();
        for case in fixture["path_cases"].as_array().expect("path cases") {
            let result = match (
                case["canonical_directory"].as_str(),
                case["stable_parent"].as_bool().expect("stable-parent flag"),
            ) {
                (Some(directory), true) => validate_direct_child(
                    Path::new(directory),
                    OsStr::new(case["basename"].as_str().expect("basename")),
                ),
                _ => Err(BackendError::InsecureProjectionPath),
            };
            match case["expected"]["decision"]
                .as_str()
                .expect("path decision")
            {
                "accept" => assert_eq!(
                    result.expect("accepted path"),
                    PathBuf::from(
                        case["expected"]["sqlite_path"]
                            .as_str()
                            .expect("SQLite path")
                    )
                ),
                "reject_before_access" => assert!(result.is_err()),
                other => panic!("unknown path decision {other}"),
            }
        }

        let Some(directory) = SupportedDirectory::new("literal-uri") else {
            return;
        };
        let literal = OsStr::new("file:projection.sqlite3?mode=memory");
        drop(
            create_fresh_projection(directory.path(), literal)
                .expect("file:-shaped basename is a literal child"),
        );
        assert!(directory.path().join(literal).is_file());
        assert_eq!(directory_names(directory.path()), [literal.to_os_string()]);
        assert_no_sidecars(directory.path(), literal);

        let mut journal = literal.to_os_string();
        journal.push("-journal");
        let journal_path = directory.path().join(&journal);
        create_mode_0600(&journal_path, b"adversarial sidecar");
        let database_bytes = fs::read(directory.path().join(literal)).expect("database snapshot");
        let sidecar_bytes = fs::read(&journal_path).expect("sidecar snapshot");
        let names = directory_names(directory.path());
        assert!(matches!(
            open_projection_reader(directory.path(), literal),
            Err(BackendError::InsecureProjectionPath)
        ));
        assert_eq!(
            fs::read(directory.path().join(literal)).expect("unchanged database"),
            database_bytes
        );
        assert_eq!(
            fs::read(&journal_path).expect("unchanged sidecar"),
            sidecar_bytes
        );
        assert_eq!(directory_names(directory.path()), names);
    }

    #[test]
    fn test_projection_page_budget_and_busy_timeout_are_exact() {
        let full = classify_runtime_error(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_FULL),
            None,
        ));
        assert!(matches!(full, BackendError::StorageFull(_)));
        let busy = classify_runtime_error(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            None,
        ));
        assert!(matches!(busy, BackendError::StorageBusy(_)));

        let Some(directory) = SupportedDirectory::new("page-budget") else {
            return;
        };
        let basename = OsStr::new("projection.sqlite3");
        drop(
            create_fresh_projection(directory.path(), basename)
                .expect("create page-budget projection"),
        );
        let mut writer = open_projection_writer(directory.path(), basename)
            .expect("reopen writer before page-budget readback and growth");
        expect_configuration_i64(
            writer.connection(),
            &writer.authorization,
            ConnectionRole::ProjectorWriter,
            ConfigurationStatement::MaxPageCountRead,
            "PRAGMA main.max_page_count",
            MAX_DATABASE_PAGES,
        )
        .expect("production max_page_count readback");
        execute_configuration(
            writer.connection(),
            &writer.authorization,
            ConnectionRole::ProjectorWriter,
            ConfigurationStatement::MaxPageCountSet128,
            "PRAGMA main.max_page_count=128",
        )
        .expect("set test-only lower page ceiling through exact configuration path");
        expect_configuration_i64(
            writer.connection(),
            &writer.authorization,
            ConnectionRole::ProjectorWriter,
            ConfigurationStatement::MaxPageCountRead,
            "PRAGMA main.max_page_count",
            128,
        )
        .expect("test page ceiling readback");

        let hash = [7_u8; 32];
        writer
            .begin_projection()
            .expect("begin in-budget transaction");
        projection_store::insert_anchor(
            &mut writer,
            ProjectionAnchor {
                relay_genesis_hash: &hash,
                parachain_genesis_hash: &hash,
                deployment_id: &hash,
                initial_runtime_spec_version: 1,
                initial_runtime_code_hash: &hash,
            },
        )
        .expect("insert in-budget anchor");
        projection_store::insert_block(
            &mut writer,
            ProjectedBlock {
                block_number: 0,
                block_hash: &hash,
                parent_hash: &hash,
                runtime_spec_version: 1,
                runtime_code_hash: &hash,
                event_count: 0,
                first_global_sequence: None,
                last_global_sequence: None,
            },
        )
        .expect("insert in-budget block");
        writer
            .commit_projection()
            .expect("commit final in-budget work");

        writer
            .begin_projection()
            .expect("begin over-budget transaction");
        let payload = vec![0_u8; 1_048_576];
        let error = projection_store::insert_event(
            &mut writer,
            ProjectedEvent {
                block_number: 0,
                extrinsic_index: 0,
                system_event_index: 0,
                global_sequence: 1,
                deployment_id: &hash,
                kind: ProjectedEventKind::UnitCreated,
                scale_payload: &payload,
                signer: &hash,
                extrinsic_hash: &hash,
            },
        )
        .expect_err("growth beyond the test ceiling must fail");
        assert!(matches!(error, BackendError::StorageFull(_)));
        writer
            .rollback_projection()
            .expect("rollback over-budget transaction");
        drop(writer);

        let mut reader =
            open_projection_reader(directory.path(), basename).expect("open after full rollback");
        reader.begin_verified_read().expect("begin rollback check");
        let event = reader
            .with_verified_query(VerifiedQueryStatement::OneBoundedRead, |connection| {
                connection
                    .query_row(
                        "SELECT global_sequence FROM projected_events LIMIT 1",
                        params![],
                        |row| row.get::<_, Vec<u8>>(0),
                    )
                    .optional()
                    .map_err(classify_runtime_error)
            })
            .expect("query rolled-back event");
        assert!(event.is_none());
        assert!(matches!(
            reader
                .with_verified_query(VerifiedQueryStatement::OneBoundedRead, |_connection| Ok(())),
            Err(BackendError::CorruptSchema)
        ));
        assert!(matches!(
            reader.data_version(),
            Err(BackendError::CorruptSchema)
        ));
        reader
            .rollback_verified_read()
            .expect("close rollback check");
        drop(reader);

        let busy_basename = OsStr::new("busy.sqlite3");
        let mut first =
            create_fresh_projection(directory.path(), busy_basename).expect("create busy fixture");
        first
            .begin_projection()
            .expect("hold DELETE-mode writer lock");
        let started = Instant::now();
        let contender = match open_projection_writer(directory.path(), busy_basename) {
            Err(error) => error,
            Ok(_) => panic!("contending writer unexpectedly opened"),
        };
        let elapsed = started.elapsed();
        assert!(matches!(contender, BackendError::StorageBusy(_)));
        assert!(
            elapsed >= Duration::from_millis(4_500),
            "elapsed={elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_millis(7_500),
            "elapsed={elapsed:?}"
        );
        first.rollback_projection().expect("release writer lock");
    }

    #[test]
    fn test_open_rejects_before_creating_a_database_or_parent() {
        let path = PathBuf::from(format!(
            "{}/cubikan-retired-open-{}-missing/parent/database.sqlite3",
            std::env::temp_dir().display(),
            std::process::id()
        ));
        assert!(!path.exists());

        assert_eq!(
            SqliteBackend::open(&path).expect_err("retired schema must reject"),
            BackendError::UnsupportedSchemaVersion { found: 2 }
        );
        assert!(!path.exists());
        assert!(!path.parent().expect("fixture has a parent").exists());
    }
}
