//! Crash-recoverable, per-signer submission journal.
//!
//! This module deliberately owns its filesystem classifier and secure path
//! handling. It does not rely on the backend crate, SQLite, or a caller-chosen
//! journal path. The advisory lock coordinates only cooperating CubiKan
//! processes using the same projection directory and signer.
//! External signer users and alternate projection directories are not coordinated.
//! Same-user deletion of an unresolved record is undetectable.
//! This lane makes no exactly-once delivery claim.

// The integration test includes this private module directly so it can drive
// process-crash hooks without exporting them from the library. The library's
// separate `cfg(test)` build therefore sees those hooks but does not call them.
#![cfg_attr(test, allow(dead_code))]

use std::{fmt, io, path::Path};

use sha2::{Digest, Sha256};

const LANE_DOMAIN: &[u8] = b"CubiKan signer lane v1\0";
const JOURNAL_DOMAIN: &[u8] = b"CubiKan submission-journal-v1\0";
const JOURNAL_MAGIC: &[u8; 8] = b"CUBKJNL1";
const JOURNAL_VERSION: u16 = 1;
const JOURNAL_LENGTH: usize = 256;
const JOURNAL_PREFIX_LENGTH: usize = 224;
const MORTAL_PERIOD: u64 = 64;

/// Original mutation identity persisted independently of later requests.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub(crate) enum MutationOperation {
    CreateUnit = 0,
    TransitionUnit = 1,
    CompleteUnit = 2,
    CreateDefinition = 3,
    CreateRelationship = 4,
    DeleteRelationship = 5,
    RecordAssociation = 6,
    RevokeAssociation = 7,
}

impl MutationOperation {
    const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::CreateUnit),
            1 => Some(Self::TransitionUnit),
            2 => Some(Self::CompleteUnit),
            3 => Some(Self::CreateDefinition),
            4 => Some(Self::CreateRelationship),
            5 => Some(Self::DeleteRelationship),
            6 => Some(Self::RecordAssociation),
            7 => Some(Self::RevokeAssociation),
            _ => None,
        }
    }

    pub(crate) const fn tag(self) -> u8 {
        self as u8
    }
}

/// Durable state of one submission lane.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub(crate) enum JournalState {
    Prepared = 0,
    FinalizedAccepted = 1,
    FinalizedDispatchRejected = 2,
    FinalizedInvariantFailed = 3,
    ExpiredNotIncluded = 4,
}

impl JournalState {
    const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::Prepared),
            1 => Some(Self::FinalizedAccepted),
            2 => Some(Self::FinalizedDispatchRejected),
            3 => Some(Self::FinalizedInvariantFailed),
            4 => Some(Self::ExpiredNotIncluded),
            _ => None,
        }
    }

    pub(crate) const fn is_resolved(self) -> bool {
        !matches!(self, Self::Prepared)
    }
}

/// Exact semantic content of one 256-byte `submission-journal-v1` record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JournalRecord {
    state: JournalState,
    deployment_id: [u8; 32],
    signer: [u8; 32],
    nonce: u64,
    extrinsic_hash: [u8; 32],
    signing_block_number: u64,
    signing_block_hash: [u8; 32],
    birth: u64,
    death: u64,
    resolution_block_number: u64,
    resolution_block_hash: [u8; 32],
    operation: MutationOperation,
}

impl JournalRecord {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepared(
        deployment_id: [u8; 32],
        signer: [u8; 32],
        nonce: u64,
        extrinsic_hash: [u8; 32],
        signing_block_number: u64,
        signing_block_hash: [u8; 32],
        operation: MutationOperation,
    ) -> Result<Self, JournalError> {
        let death = signing_block_number
            .checked_add(MORTAL_PERIOD - 1)
            .ok_or(JournalError::InvalidRecord)?;
        let record = Self {
            state: JournalState::Prepared,
            deployment_id,
            signer,
            nonce,
            extrinsic_hash,
            signing_block_number,
            signing_block_hash,
            birth: signing_block_number,
            death,
            resolution_block_number: 0,
            resolution_block_hash: [0; 32],
            operation,
        };
        record.validate()?;
        Ok(record)
    }

    pub(crate) fn resolved(
        &self,
        state: JournalState,
        resolution_block_number: u64,
        resolution_block_hash: [u8; 32],
    ) -> Result<Self, JournalError> {
        if self.state != JournalState::Prepared || !state.is_resolved() {
            return Err(JournalError::InvalidTransition);
        }
        let mut resolved = self.clone();
        resolved.state = state;
        resolved.resolution_block_number = resolution_block_number;
        resolved.resolution_block_hash = resolution_block_hash;
        resolved.validate()?;
        Ok(resolved)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, JournalError> {
        if bytes.len() != JOURNAL_LENGTH
            || &bytes[0..8] != JOURNAL_MAGIC
            || read_u16(bytes, 8)? != JOURNAL_VERSION
            || bytes[11] != 0
            || usize::from(read_u16(bytes, 12)?) != JOURNAL_LENGTH
            || bytes[14..16] != [0, 0]
            || bytes[217..224] != [0; 7]
        {
            return Err(JournalError::CorruptJournal);
        }
        let expected_checksum = checksum(&bytes[..JOURNAL_PREFIX_LENGTH]);
        if bytes[JOURNAL_PREFIX_LENGTH..] != expected_checksum {
            return Err(JournalError::CorruptJournal);
        }
        let state = JournalState::from_tag(bytes[10]).ok_or(JournalError::CorruptJournal)?;
        let operation =
            MutationOperation::from_tag(bytes[216]).ok_or(JournalError::CorruptJournal)?;
        let record = Self {
            state,
            deployment_id: read_array(bytes, 16)?,
            signer: read_array(bytes, 48)?,
            nonce: read_u64(bytes, 80)?,
            extrinsic_hash: read_array(bytes, 88)?,
            signing_block_number: read_u64(bytes, 120)?,
            signing_block_hash: read_array(bytes, 128)?,
            birth: read_u64(bytes, 160)?,
            death: read_u64(bytes, 168)?,
            resolution_block_number: read_u64(bytes, 176)?,
            resolution_block_hash: read_array(bytes, 184)?,
            operation,
        };
        record
            .validate()
            .map_err(|_| JournalError::CorruptJournal)?;
        Ok(record)
    }

    pub(crate) fn encode(&self) -> Result<[u8; JOURNAL_LENGTH], JournalError> {
        self.validate()?;
        let mut bytes = [0_u8; JOURNAL_LENGTH];
        bytes[0..8].copy_from_slice(JOURNAL_MAGIC);
        bytes[8..10].copy_from_slice(&JOURNAL_VERSION.to_be_bytes());
        bytes[10] = self.state as u8;
        bytes[12..14].copy_from_slice(&(JOURNAL_LENGTH as u16).to_be_bytes());
        bytes[16..48].copy_from_slice(&self.deployment_id);
        bytes[48..80].copy_from_slice(&self.signer);
        bytes[80..88].copy_from_slice(&self.nonce.to_be_bytes());
        bytes[88..120].copy_from_slice(&self.extrinsic_hash);
        bytes[120..128].copy_from_slice(&self.signing_block_number.to_be_bytes());
        bytes[128..160].copy_from_slice(&self.signing_block_hash);
        bytes[160..168].copy_from_slice(&self.birth.to_be_bytes());
        bytes[168..176].copy_from_slice(&self.death.to_be_bytes());
        bytes[176..184].copy_from_slice(&self.resolution_block_number.to_be_bytes());
        bytes[184..216].copy_from_slice(&self.resolution_block_hash);
        bytes[216] = self.operation.tag();
        let digest = checksum(&bytes[..JOURNAL_PREFIX_LENGTH]);
        bytes[JOURNAL_PREFIX_LENGTH..].copy_from_slice(&digest);
        Ok(bytes)
    }

    fn validate(&self) -> Result<(), JournalError> {
        if self.birth != self.signing_block_number
            || self
                .birth
                .checked_add(MORTAL_PERIOD - 1)
                .is_none_or(|death| death != self.death)
            || self.extrinsic_hash == [0; 32]
            || self.signing_block_hash == [0; 32]
        {
            return Err(JournalError::InvalidRecord);
        }
        match self.state {
            JournalState::Prepared => {
                if self.resolution_block_number != 0 || self.resolution_block_hash != [0; 32] {
                    return Err(JournalError::InvalidRecord);
                }
            }
            JournalState::FinalizedAccepted
            | JournalState::FinalizedDispatchRejected
            | JournalState::FinalizedInvariantFailed => {
                if !(self.birth..=self.death).contains(&self.resolution_block_number)
                    || self.resolution_block_hash == [0; 32]
                {
                    return Err(JournalError::InvalidRecord);
                }
            }
            JournalState::ExpiredNotIncluded => {
                if self.resolution_block_number <= self.death
                    || self.resolution_block_hash == [0; 32]
                {
                    return Err(JournalError::InvalidRecord);
                }
            }
        }
        Ok(())
    }

    pub(crate) const fn state(&self) -> JournalState {
        self.state
    }

    pub(crate) const fn deployment_id(&self) -> &[u8; 32] {
        &self.deployment_id
    }

    pub(crate) const fn signer(&self) -> &[u8; 32] {
        &self.signer
    }

    pub(crate) const fn nonce(&self) -> u64 {
        self.nonce
    }

    pub(crate) const fn extrinsic_hash(&self) -> &[u8; 32] {
        &self.extrinsic_hash
    }

    pub(crate) const fn signing_block_number(&self) -> u64 {
        self.signing_block_number
    }

    pub(crate) const fn signing_block_hash(&self) -> &[u8; 32] {
        &self.signing_block_hash
    }

    pub(crate) const fn birth(&self) -> u64 {
        self.birth
    }

    pub(crate) const fn death(&self) -> u64 {
        self.death
    }

    pub(crate) const fn resolution_block_number(&self) -> u64 {
        self.resolution_block_number
    }

    pub(crate) const fn resolution_block_hash(&self) -> &[u8; 32] {
        &self.resolution_block_hash
    }

    pub(crate) const fn operation(&self) -> MutationOperation {
        self.operation
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, JournalError> {
    Ok(u16::from_be_bytes(read_array(bytes, offset)?))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, JournalError> {
    Ok(u64::from_be_bytes(read_array(bytes, offset)?))
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], JournalError> {
    bytes
        .get(offset..offset.checked_add(N).ok_or(JournalError::CorruptJournal)?)
        .ok_or(JournalError::CorruptJournal)?
        .try_into()
        .map_err(|_| JournalError::CorruptJournal)
}

fn checksum(prefix: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(JOURNAL_DOMAIN);
    hasher.update(prefix);
    hasher.finalize().into()
}

/// Three fixed basenames derived for one signer lane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LaneNames {
    lock: String,
    journal: String,
    temporary: String,
}

impl LaneNames {
    pub(crate) fn derive(
        canonical_directory: &Path,
        deployment_id: &[u8; 32],
        signer: &[u8; 32],
    ) -> Result<Self, JournalError> {
        let path_bytes = unix_path_bytes(canonical_directory)?;
        if !is_lexically_canonical_absolute_path(path_bytes) {
            return Err(JournalError::InsecurePath);
        }
        let path_length =
            u32::try_from(path_bytes.len()).map_err(|_| JournalError::InsecurePath)?;
        let mut hasher = Sha256::new();
        hasher.update(LANE_DOMAIN);
        hasher.update(path_length.to_be_bytes());
        hasher.update(path_bytes);
        hasher.update(deployment_id);
        hasher.update(signer);
        let digest: [u8; 32] = hasher.finalize().into();
        let hex = lower_hex(&digest);
        let names = Self {
            lock: format!("cubikan-submission-{hex}.lock"),
            journal: format!("cubikan-submission-{hex}.journal"),
            temporary: format!("cubikan-submission-{hex}.tmp"),
        };
        names.validate()?;
        Ok(names)
    }

    fn validate(&self) -> Result<(), JournalError> {
        for (name, suffix) in [
            (&self.lock, ".lock"),
            (&self.journal, ".journal"),
            (&self.temporary, ".tmp"),
        ] {
            if name.len() != "cubikan-submission-".len() + 64 + suffix.len()
                || !name.starts_with("cubikan-submission-")
                || !name.ends_with(suffix)
                || !name["cubikan-submission-".len().."cubikan-submission-".len() + 64]
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                || !is_direct_child(name)
            {
                return Err(JournalError::InsecurePath);
            }
        }
        let lock_digest = &self.lock["cubikan-submission-".len().."cubikan-submission-".len() + 64];
        let journal_digest =
            &self.journal["cubikan-submission-".len().."cubikan-submission-".len() + 64];
        let temp_digest =
            &self.temporary["cubikan-submission-".len().."cubikan-submission-".len() + 64];
        if lock_digest != journal_digest || lock_digest != temp_digest {
            return Err(JournalError::InsecurePath);
        }
        Ok(())
    }

    pub(crate) fn lock(&self) -> &str {
        &self.lock
    }

    pub(crate) fn journal(&self) -> &str {
        &self.journal
    }

    pub(crate) fn temporary(&self) -> &str {
        &self.temporary
    }

    #[cfg(test)]
    pub(crate) fn from_parts_for_test(
        lock: String,
        journal: String,
        temporary: String,
    ) -> Result<Self, JournalError> {
        let names = Self {
            lock,
            journal,
            temporary,
        };
        names.validate()?;
        Ok(names)
    }
}

fn lower_hex(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn is_direct_child(name: &str) -> bool {
    !name.is_empty()
        && !name.as_bytes().contains(&0)
        && !matches!(name, "." | "..")
        && !name.as_bytes().contains(&b'/')
}

fn is_lexically_canonical_absolute_path(bytes: &[u8]) -> bool {
    if bytes.first() != Some(&b'/') || bytes.contains(&0) {
        return false;
    }
    if bytes == b"/" {
        return true;
    }
    if bytes.last() == Some(&b'/') || bytes.windows(2).any(|pair| pair == b"//") {
        return false;
    }
    bytes[1..]
        .split(|byte| *byte == b'/')
        .all(|component| !component.is_empty() && component != b"." && component != b"..")
}

#[cfg(test)]
pub(crate) fn validate_virtual_lane_path_for_test(
    path_bytes: &[u8],
    virtual_length: u64,
) -> Result<(), JournalError> {
    if !is_lexically_canonical_absolute_path(path_bytes)
        || u32::try_from(virtual_length).is_err()
        || usize::try_from(virtual_length).ok() != Some(path_bytes.len())
    {
        return Err(JournalError::InsecurePath);
    }
    Ok(())
}

fn validate_regular_observation(
    is_regular: bool,
    owner: u32,
    effective_owner: u32,
    permissions: u32,
    size: i64,
    expected_size: Option<i64>,
) -> Result<(), JournalError> {
    if !is_regular
        || owner != effective_owner
        || permissions != 0o600
        || expected_size.is_some_and(|expected| size != expected)
    {
        return Err(JournalError::InsecurePath);
    }
    Ok(())
}

fn validate_directory_observation(
    is_directory: bool,
    owner: u32,
    effective_owner: u32,
    permissions: u32,
) -> Result<(), JournalError> {
    if !is_directory || owner != effective_owner || permissions != 0o700 {
        return Err(JournalError::InsecurePath);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn validate_regular_observation_for_test(
    is_regular: bool,
    owner: u32,
    effective_owner: u32,
    permissions: u32,
    size: i64,
    expected_size: Option<i64>,
) -> Result<(), JournalError> {
    validate_regular_observation(
        is_regular,
        owner,
        effective_owner,
        permissions,
        size,
        expected_size,
    )
}

#[cfg(test)]
pub(crate) fn validate_directory_observation_for_test(
    is_directory: bool,
    owner: u32,
    effective_owner: u32,
    permissions: u32,
) -> Result<(), JournalError> {
    validate_directory_observation(is_directory, owner, effective_owner, permissions)
}

#[cfg(unix)]
fn unix_path_bytes(path: &Path) -> Result<&[u8], JournalError> {
    use std::os::unix::ffi::OsStrExt;

    if !path.is_absolute() {
        return Err(JournalError::InsecurePath);
    }
    Ok(path.as_os_str().as_bytes())
}

#[cfg(not(unix))]
fn unix_path_bytes(_path: &Path) -> Result<&[u8], JournalError> {
    Err(JournalError::UnsupportedPlatform)
}

/// Typed failure for lane derivation, validation, locking, or durability.
#[derive(Debug)]
pub(crate) enum JournalError {
    UnsupportedPlatform,
    UnsupportedFilesystem,
    InsecurePath,
    CorruptJournal,
    InvalidRecord,
    InvalidTransition,
    Io {
        operation: &'static str,
        source: io::Error,
    },
    #[cfg(test)]
    InjectedFault(PublicationPoint),
}

impl JournalError {
    fn io(operation: &'static str, source: impl Into<io::Error>) -> Self {
        Self::Io {
            operation,
            source: source.into(),
        }
    }
}

impl fmt::Display for JournalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => formatter.write_str("submission journal requires Linux"),
            Self::UnsupportedFilesystem => {
                formatter.write_str("submission journal filesystem is unsupported")
            }
            Self::InsecurePath => formatter.write_str("submission journal path is insecure"),
            Self::CorruptJournal => formatter.write_str("submission journal is corrupt"),
            Self::InvalidRecord => formatter.write_str("submission journal record is invalid"),
            Self::InvalidTransition => {
                formatter.write_str("submission journal transition is invalid")
            }
            Self::Io { operation, source } => write!(formatter, "{operation}: {source}"),
            #[cfg(test)]
            Self::InjectedFault(point) => write!(formatter, "injected journal fault at {point:?}"),
        }
    }
}

impl std::error::Error for JournalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// A held, process-wide advisory signer lane.
///
/// The Linux implementation keeps the persistent lock inode open and locked
/// for this value's whole lifetime. A terminal record is removed only by the
/// consuming [`SignerLane::acknowledge_resolved`] boundary.
pub(crate) struct SignerLane {
    inner: platform::LaneInner,
    names: LaneNames,
    deployment_id: [u8; 32],
    signer: [u8; 32],
    record: Option<JournalRecord>,
}

impl SignerLane {
    pub(crate) fn open(
        projection_directory: &Path,
        deployment_id: [u8; 32],
        signer: [u8; 32],
    ) -> Result<Self, JournalError> {
        let names = LaneNames::derive(projection_directory, &deployment_id, &signer)?;
        let mut inner = platform::LaneInner::open(projection_directory, &names)?;
        inner.cleanup_temporary(&names)?;
        let record = inner.load_journal(&names)?;
        if record
            .as_ref()
            .is_some_and(|record| record.deployment_id != deployment_id || record.signer != signer)
        {
            return Err(JournalError::CorruptJournal);
        }
        Ok(Self {
            inner,
            names,
            deployment_id,
            signer,
            record,
        })
    }

    pub(crate) const fn record(&self) -> Option<&JournalRecord> {
        self.record.as_ref()
    }

    #[cfg(test)]
    pub(crate) const fn names(&self) -> &LaneNames {
        &self.names
    }

    pub(crate) fn publish_prepared(&mut self, record: JournalRecord) -> Result<(), JournalError> {
        self.publish_prepared_inner(record, &mut |_| Ok(()))
    }

    fn publish_prepared_inner(
        &mut self,
        record: JournalRecord,
        hook: &mut dyn FnMut(PublicationPoint) -> Result<(), JournalError>,
    ) -> Result<(), JournalError> {
        if self.record.is_some()
            || record.state != JournalState::Prepared
            || record.deployment_id != self.deployment_id
            || record.signer != self.signer
        {
            return Err(JournalError::InvalidTransition);
        }
        self.inner
            .publish(&self.names, None, &record, hook)
            .inspect(|()| self.record = Some(record))
    }

    pub(crate) fn publish_resolved(&mut self, record: JournalRecord) -> Result<(), JournalError> {
        self.publish_resolved_inner(record, &mut |_| Ok(()))
    }

    fn publish_resolved_inner(
        &mut self,
        record: JournalRecord,
        hook: &mut dyn FnMut(PublicationPoint) -> Result<(), JournalError>,
    ) -> Result<(), JournalError> {
        let Some(prepared) = self.record.as_ref() else {
            return Err(JournalError::InvalidTransition);
        };
        if prepared.state != JournalState::Prepared
            || !record.state.is_resolved()
            || !same_submission(prepared, &record)
        {
            return Err(JournalError::InvalidTransition);
        }
        self.inner
            .publish(&self.names, Some(prepared), &record, hook)
            .inspect(|()| self.record = Some(record))
    }

    pub(crate) fn acknowledge_resolved(self) -> Result<(), JournalError> {
        self.acknowledge_resolved_inner(&mut |_| Ok(()))
    }

    fn acknowledge_resolved_inner(
        mut self,
        hook: &mut dyn FnMut(PublicationPoint) -> Result<(), JournalError>,
    ) -> Result<(), JournalError> {
        let Some(record) = self.record.as_ref() else {
            return Err(JournalError::InvalidTransition);
        };
        if !record.state.is_resolved() {
            return Err(JournalError::InvalidTransition);
        }
        self.inner.remove_resolved(&self.names, record, hook)?;
        self.record = None;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn acknowledge_resolved_with_hook(
        self,
        hook: &mut dyn FnMut(PublicationPoint) -> Result<(), JournalError>,
    ) -> Result<(), JournalError> {
        self.acknowledge_resolved_inner(hook)
    }

    #[cfg(test)]
    pub(crate) fn publish_prepared_with_fault(
        &mut self,
        record: JournalRecord,
        fault: PublicationPoint,
    ) -> Result<(), JournalError> {
        let mut hook = |point| {
            if point == fault {
                Err(JournalError::InjectedFault(point))
            } else {
                Ok(())
            }
        };
        self.publish_prepared_inner(record, &mut hook)
    }

    #[cfg(test)]
    pub(crate) fn publish_prepared_with_hook(
        &mut self,
        record: JournalRecord,
        hook: &mut dyn FnMut(PublicationPoint) -> Result<(), JournalError>,
    ) -> Result<(), JournalError> {
        self.publish_prepared_inner(record, hook)
    }

    #[cfg(test)]
    pub(crate) fn publish_resolved_with_fault(
        &mut self,
        record: JournalRecord,
        fault: PublicationPoint,
    ) -> Result<(), JournalError> {
        let mut hook = |point| {
            if point == fault {
                Err(JournalError::InjectedFault(point))
            } else {
                Ok(())
            }
        };
        self.publish_resolved_inner(record, &mut hook)
    }

    #[cfg(test)]
    pub(crate) fn publish_resolved_with_hook(
        &mut self,
        record: JournalRecord,
        hook: &mut dyn FnMut(PublicationPoint) -> Result<(), JournalError>,
    ) -> Result<(), JournalError> {
        self.publish_resolved_inner(record, hook)
    }
}

fn same_submission(left: &JournalRecord, right: &JournalRecord) -> bool {
    left.deployment_id == right.deployment_id
        && left.signer == right.signer
        && left.nonce == right.nonce
        && left.extrinsic_hash == right.extrinsic_hash
        && left.signing_block_number == right.signing_block_number
        && left.signing_block_hash == right.signing_block_hash
        && left.birth == right.birth
        && left.death == right.death
        && left.operation == right.operation
}

/// Journal publication boundaries available only to test builds.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublicationPoint {
    BeforeChecksum,
    AfterChecksum,
    BeforeTemporaryCreate,
    AfterTemporaryCreate,
    AfterPartialWrite,
    AfterCompleteWrite,
    BeforeFileSync,
    AfterFileSync,
    BeforeRename,
    AfterRename,
    BeforeDirectorySync,
    AfterDirectorySync,
    BeforeRemoval,
    AfterRemoval,
    BeforeRemovalDirectorySync,
    AfterRemovalDirectorySync,
}

#[cfg(not(test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublicationPoint {
    BeforeChecksum,
    AfterChecksum,
    BeforeTemporaryCreate,
    AfterTemporaryCreate,
    AfterPartialWrite,
    AfterCompleteWrite,
    BeforeFileSync,
    AfterFileSync,
    BeforeRename,
    AfterRename,
    BeforeDirectorySync,
    AfterDirectorySync,
    BeforeRemoval,
    AfterRemoval,
    BeforeRemovalDirectorySync,
    AfterRemovalDirectorySync,
}

#[cfg(target_os = "linux")]
mod platform {
    use std::{
        os::fd::OwnedFd,
        path::{Path, PathBuf},
    };

    use rustix::{
        fs::{self, AtFlags, CWD, FileType, FlockOperation, Mode, OFlags},
        io::{self as rustix_io, Errno},
        process,
    };

    use super::{
        JOURNAL_LENGTH, JournalError, JournalRecord, LaneNames, PublicationPoint,
        classify_mount_observation, validate_directory_observation, validate_regular_observation,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct FileIdentity {
        device: u64,
        inode: u64,
    }

    pub(super) struct LaneInner {
        directory: OwnedFd,
        directory_identity: FileIdentity,
        canonical_directory: PathBuf,
        lock: OwnedFd,
        lock_identity: FileIdentity,
    }

    impl LaneInner {
        pub(super) fn open(directory: &Path, names: &LaneNames) -> Result<Self, JournalError> {
            if !directory.is_absolute() {
                return Err(JournalError::InsecurePath);
            }
            let before = fs::statat(CWD, directory, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| JournalError::InsecurePath)?;
            validate_directory(&before)?;
            let canonical_directory =
                std::fs::canonicalize(directory).map_err(|_| JournalError::InsecurePath)?;
            if canonical_directory != directory {
                return Err(JournalError::InsecurePath);
            }
            let directory_fd = fs::openat(
                CWD,
                &canonical_directory,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|error| JournalError::io("open submission directory", error))?;
            let opened = fs::fstat(&directory_fd)
                .map_err(|error| JournalError::io("inspect submission directory", error))?;
            validate_directory(&opened)?;
            let directory_identity = identity(&opened);
            if directory_identity != identity(&before) {
                return Err(JournalError::InsecurePath);
            }
            let mountinfo = std::fs::read_to_string("/proc/self/mountinfo")
                .map_err(|error| JournalError::io("read Linux mountinfo", error))?;
            let statfs = fs::fstatfs(&directory_fd)
                .map_err(|error| JournalError::io("inspect submission filesystem", error))?;
            let mount_identity = classify_mount_observation(
                "linux",
                &canonical_directory,
                &mountinfo,
                (statfs.f_type as u64) & 0xffff_ffff,
            )?;
            if canonical_directory != mount_identity.mount_point
                && !canonical_directory.starts_with(&mount_identity.mount_point)
            {
                return Err(JournalError::UnsupportedFilesystem);
            }
            let (lock, created) = open_lock(&directory_fd, names.lock())?;
            let lock_stat =
                fs::fstat(&lock).map_err(|error| JournalError::io("inspect signer lock", error))?;
            validate_regular(&lock_stat, Some(0))?;
            let lock_identity = identity(&lock_stat);
            validate_linked_identity(&directory_fd, names.lock(), lock_identity)?;
            if created {
                fs::fsync(&lock).map_err(|error| JournalError::io("sync signer lock", error))?;
                fs::fsync(&directory_fd)
                    .map_err(|error| JournalError::io("sync signer lock directory", error))?;
            }
            fs::flock(&lock, FlockOperation::LockExclusive)
                .map_err(|error| JournalError::io("lock signer lane", error))?;
            validate_linked_identity(&directory_fd, names.lock(), lock_identity)?;
            let inner = Self {
                directory: directory_fd,
                directory_identity,
                canonical_directory,
                lock,
                lock_identity,
            };
            inner.validate_stable_directory()?;
            inner.validate_lock(names)?;
            Ok(inner)
        }

        pub(super) fn cleanup_temporary(&mut self, names: &LaneNames) -> Result<(), JournalError> {
            self.validate_stable_directory()?;
            self.validate_lock(names)?;
            match fs::statat(
                &self.directory,
                names.temporary(),
                AtFlags::SYMLINK_NOFOLLOW,
            ) {
                Err(Errno::NOENT) => Ok(()),
                Err(error) => Err(JournalError::io("inspect journal temporary file", error)),
                Ok(stat) => {
                    validate_regular(&stat, None)?;
                    if stat.st_size < 0 || stat.st_size > JOURNAL_LENGTH as i64 {
                        return Err(JournalError::InsecurePath);
                    }
                    let expected = identity(&stat);
                    validate_linked_identity(&self.directory, names.temporary(), expected)?;
                    fs::unlinkat(&self.directory, names.temporary(), AtFlags::empty()).map_err(
                        |error| JournalError::io("remove journal temporary file", error),
                    )?;
                    fs::fsync(&self.directory).map_err(|error| {
                        JournalError::io("sync temporary cleanup directory", error)
                    })?;
                    self.validate_stable_directory()?;
                    Ok(())
                }
            }
        }

        pub(super) fn load_journal(
            &self,
            names: &LaneNames,
        ) -> Result<Option<JournalRecord>, JournalError> {
            self.validate_stable_directory()?;
            self.validate_lock(names)?;
            let linked =
                match fs::statat(&self.directory, names.journal(), AtFlags::SYMLINK_NOFOLLOW) {
                    Err(Errno::NOENT) => return Ok(None),
                    Err(error) => {
                        return Err(JournalError::io("inspect submission journal", error));
                    }
                    Ok(stat) => stat,
                };
            validate_regular(&linked, Some(JOURNAL_LENGTH as i64))?;
            let expected = identity(&linked);
            let file = fs::openat(
                &self.directory,
                names.journal(),
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|error| JournalError::io("open submission journal", error))?;
            let opened = fs::fstat(&file)
                .map_err(|error| JournalError::io("inspect open submission journal", error))?;
            validate_regular(&opened, Some(JOURNAL_LENGTH as i64))?;
            if identity(&opened) != expected {
                return Err(JournalError::InsecurePath);
            }
            let mut bytes = [0_u8; JOURNAL_LENGTH + 1];
            let count = rustix_io::pread(&file, &mut bytes[..], 0)
                .map_err(|error| JournalError::io("read submission journal", error))?;
            if count != JOURNAL_LENGTH {
                return Err(JournalError::CorruptJournal);
            }
            validate_linked_identity(&self.directory, names.journal(), expected)?;
            self.validate_stable_directory()?;
            JournalRecord::decode(&bytes[..JOURNAL_LENGTH]).map(Some)
        }

        pub(super) fn publish(
            &mut self,
            names: &LaneNames,
            old: Option<&JournalRecord>,
            new: &JournalRecord,
            hook: &mut dyn FnMut(PublicationPoint) -> Result<(), JournalError>,
        ) -> Result<(), JournalError> {
            self.validate_stable_directory()?;
            self.validate_lock(names)?;
            self.require_temporary_absent(names)?;
            match old {
                None => self.require_journal_absent(names)?,
                Some(expected) => {
                    let actual = self
                        .load_journal(names)?
                        .ok_or(JournalError::InvalidTransition)?;
                    if &actual != expected {
                        return Err(JournalError::InvalidTransition);
                    }
                }
            }
            hook(PublicationPoint::BeforeChecksum)?;
            let bytes = new.encode()?;
            hook(PublicationPoint::AfterChecksum)?;
            hook(PublicationPoint::BeforeTemporaryCreate)?;
            let temporary = fs::openat(
                &self.directory,
                names.temporary(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|error| JournalError::io("create journal temporary file", error))?;
            let temp_stat = fs::fstat(&temporary)
                .map_err(|error| JournalError::io("inspect journal temporary file", error))?;
            validate_regular(&temp_stat, Some(0))?;
            let temp_identity = identity(&temp_stat);
            validate_linked_identity(&self.directory, names.temporary(), temp_identity)?;
            hook(PublicationPoint::AfterTemporaryCreate)?;
            write_all(&temporary, &bytes[..113])?;
            hook(PublicationPoint::AfterPartialWrite)?;
            write_all(&temporary, &bytes[113..])?;
            hook(PublicationPoint::AfterCompleteWrite)?;
            let complete = fs::fstat(&temporary)
                .map_err(|error| JournalError::io("inspect complete journal temporary", error))?;
            validate_regular(&complete, Some(JOURNAL_LENGTH as i64))?;
            if identity(&complete) != temp_identity {
                return Err(JournalError::InsecurePath);
            }
            hook(PublicationPoint::BeforeFileSync)?;
            fs::fsync(&temporary)
                .map_err(|error| JournalError::io("sync journal temporary file", error))?;
            hook(PublicationPoint::AfterFileSync)?;
            self.validate_stable_directory()?;
            self.validate_lock(names)?;
            validate_linked_identity(&self.directory, names.temporary(), temp_identity)?;
            hook(PublicationPoint::BeforeRename)?;
            fs::renameat(
                &self.directory,
                names.temporary(),
                &self.directory,
                names.journal(),
            )
            .map_err(|error| JournalError::io("publish submission journal", error))?;
            hook(PublicationPoint::AfterRename)?;
            let published = self
                .load_journal(names)?
                .ok_or(JournalError::CorruptJournal)?;
            if published != *new {
                return Err(JournalError::CorruptJournal);
            }
            hook(PublicationPoint::BeforeDirectorySync)?;
            fs::fsync(&self.directory)
                .map_err(|error| JournalError::io("sync journal directory", error))?;
            hook(PublicationPoint::AfterDirectorySync)?;
            self.validate_stable_directory()?;
            self.validate_lock(names)?;
            Ok(())
        }

        pub(super) fn remove_resolved(
            &mut self,
            names: &LaneNames,
            expected: &JournalRecord,
            hook: &mut dyn FnMut(PublicationPoint) -> Result<(), JournalError>,
        ) -> Result<(), JournalError> {
            self.validate_stable_directory()?;
            self.validate_lock(names)?;
            self.require_temporary_absent(names)?;
            let actual = self
                .load_journal(names)?
                .ok_or(JournalError::InvalidTransition)?;
            if actual != *expected || !actual.state().is_resolved() {
                return Err(JournalError::InvalidTransition);
            }
            hook(PublicationPoint::BeforeRemoval)?;
            fs::unlinkat(&self.directory, names.journal(), AtFlags::empty())
                .map_err(|error| JournalError::io("acknowledge submission journal", error))?;
            hook(PublicationPoint::AfterRemoval)?;
            hook(PublicationPoint::BeforeRemovalDirectorySync)?;
            fs::fsync(&self.directory)
                .map_err(|error| JournalError::io("sync journal acknowledgement", error))?;
            hook(PublicationPoint::AfterRemovalDirectorySync)?;
            self.require_journal_absent(names)?;
            self.validate_stable_directory()?;
            self.validate_lock(names)
        }

        fn validate_stable_directory(&self) -> Result<(), JournalError> {
            let current = fs::statat(CWD, &self.canonical_directory, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| JournalError::InsecurePath)?;
            validate_directory(&current)?;
            if identity(&current) != self.directory_identity {
                return Err(JournalError::InsecurePath);
            }
            Ok(())
        }

        fn validate_lock(&self, names: &LaneNames) -> Result<(), JournalError> {
            let stat = fs::fstat(&self.lock)
                .map_err(|error| JournalError::io("inspect held signer lock", error))?;
            validate_regular(&stat, Some(0))?;
            if identity(&stat) != self.lock_identity {
                return Err(JournalError::InsecurePath);
            }
            validate_linked_identity(&self.directory, names.lock(), self.lock_identity)
        }

        fn require_temporary_absent(&self, names: &LaneNames) -> Result<(), JournalError> {
            match fs::statat(
                &self.directory,
                names.temporary(),
                AtFlags::SYMLINK_NOFOLLOW,
            ) {
                Err(Errno::NOENT) => Ok(()),
                _ => Err(JournalError::InsecurePath),
            }
        }

        fn require_journal_absent(&self, names: &LaneNames) -> Result<(), JournalError> {
            match fs::statat(&self.directory, names.journal(), AtFlags::SYMLINK_NOFOLLOW) {
                Err(Errno::NOENT) => Ok(()),
                _ => Err(JournalError::InvalidTransition),
            }
        }
    }

    fn open_lock(directory: &OwnedFd, name: &str) -> Result<(OwnedFd, bool), JournalError> {
        let create = || {
            fs::openat(
                directory,
                name,
                OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
            )
        };
        match create() {
            Ok(file) => Ok((file, true)),
            Err(Errno::EXIST) => fs::openat(
                directory,
                name,
                OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map(|file| (file, false))
            .map_err(|error| JournalError::io("open signer lock", error)),
            Err(error) => Err(JournalError::io("create signer lock", error)),
        }
    }

    fn validate_directory(stat: &fs::Stat) -> Result<(), JournalError> {
        validate_directory_observation(
            FileType::from_raw_mode(stat.st_mode) == FileType::Directory,
            stat.st_uid,
            process::geteuid().as_raw(),
            Mode::from_raw_mode(stat.st_mode).as_raw_mode(),
        )
    }

    fn validate_regular(stat: &fs::Stat, size: Option<i64>) -> Result<(), JournalError> {
        validate_regular_observation(
            FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile,
            stat.st_uid,
            process::geteuid().as_raw(),
            Mode::from_raw_mode(stat.st_mode).as_raw_mode(),
            stat.st_size,
            size,
        )
    }

    const fn identity(stat: &fs::Stat) -> FileIdentity {
        FileIdentity {
            device: stat.st_dev,
            inode: stat.st_ino,
        }
    }

    fn validate_linked_identity(
        directory: &OwnedFd,
        name: &str,
        expected: FileIdentity,
    ) -> Result<(), JournalError> {
        let linked = fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|_| JournalError::InsecurePath)?;
        if identity(&linked) != expected {
            return Err(JournalError::InsecurePath);
        }
        Ok(())
    }

    fn write_all(file: &OwnedFd, mut bytes: &[u8]) -> Result<(), JournalError> {
        while !bytes.is_empty() {
            match rustix_io::write(file, bytes) {
                Ok(0) => {
                    return Err(JournalError::io(
                        "write submission journal",
                        std::io::Error::new(std::io::ErrorKind::WriteZero, "zero-byte write"),
                    ));
                }
                Ok(written) => bytes = &bytes[written..],
                Err(Errno::INTR) => {}
                Err(error) => return Err(JournalError::io("write submission journal", error)),
            }
        }
        Ok(())
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use std::path::Path;

    use super::{JournalError, JournalRecord, LaneNames, PublicationPoint};

    pub(super) struct LaneInner;

    impl LaneInner {
        pub(super) fn open(_directory: &Path, _names: &LaneNames) -> Result<Self, JournalError> {
            Err(JournalError::UnsupportedPlatform)
        }

        pub(super) fn cleanup_temporary(&mut self, _names: &LaneNames) -> Result<(), JournalError> {
            Err(JournalError::UnsupportedPlatform)
        }

        pub(super) fn load_journal(
            &self,
            _names: &LaneNames,
        ) -> Result<Option<JournalRecord>, JournalError> {
            Err(JournalError::UnsupportedPlatform)
        }

        pub(super) fn publish(
            &mut self,
            _names: &LaneNames,
            _old: Option<&JournalRecord>,
            _new: &JournalRecord,
            _hook: &mut dyn FnMut(PublicationPoint) -> Result<(), JournalError>,
        ) -> Result<(), JournalError> {
            Err(JournalError::UnsupportedPlatform)
        }

        pub(super) fn remove_resolved(
            &mut self,
            _names: &LaneNames,
            _expected: &JournalRecord,
            _hook: &mut dyn FnMut(PublicationPoint) -> Result<(), JournalError>,
        ) -> Result<(), JournalError> {
            Err(JournalError::UnsupportedPlatform)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MountIdentity {
    mount_point: std::path::PathBuf,
    filesystem_type: String,
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(crate) struct VfsObservation<'a> {
    pub(crate) requested_name: &'a str,
    pub(crate) registered: bool,
    pub(crate) built_in: bool,
}

#[cfg(test)]
pub(crate) fn classify_filesystem_fixture_case(
    platform: &str,
    canonical_directory: &Path,
    mountinfo: &str,
    statfs_magic: u64,
    vfs: VfsObservation<'_>,
) -> Result<(std::path::PathBuf, String), JournalError> {
    if vfs.requested_name != "unix" || !vfs.registered || !vfs.built_in {
        return Err(JournalError::UnsupportedFilesystem);
    }
    let identity =
        classify_mount_observation(platform, canonical_directory, mountinfo, statfs_magic)?;
    Ok((identity.mount_point, identity.filesystem_type))
}

fn classify_mount_observation(
    platform: &str,
    canonical_directory: &Path,
    mountinfo: &str,
    statfs_magic: u64,
) -> Result<MountIdentity, JournalError> {
    if platform != "linux" {
        return Err(JournalError::UnsupportedPlatform);
    }
    if !canonical_directory.is_absolute() {
        return Err(JournalError::InsecurePath);
    }
    let mut selected: Option<(usize, MountIdentity)> = None;
    let mut ambiguous = false;
    let mut saw_line = false;
    for line in mountinfo.lines() {
        saw_line = true;
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        let Some(separator) = fields.iter().position(|field| *field == "-") else {
            return Err(JournalError::UnsupportedFilesystem);
        };
        if separator < 6 || fields.len() < separator + 4 {
            return Err(JournalError::UnsupportedFilesystem);
        }
        let mount_point = decode_mount_path(fields[4])?;
        if canonical_directory != mount_point && !canonical_directory.starts_with(&mount_point) {
            continue;
        }
        let length = unix_path_bytes(&mount_point)?.len();
        let identity = MountIdentity {
            mount_point,
            filesystem_type: fields[separator + 1].to_owned(),
        };
        match selected.as_ref() {
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
        return Err(JournalError::UnsupportedFilesystem);
    }
    let Some((_, identity)) = selected else {
        return Err(JournalError::UnsupportedFilesystem);
    };
    let expected_magic = match identity.filesystem_type.as_str() {
        "ext2" | "ext3" | "ext4" => 0x0000_0000_0000_ef53,
        "xfs" => 0x0000_0000_5846_5342,
        "btrfs" => 0x0000_0000_9123_683e,
        _ => return Err(JournalError::UnsupportedFilesystem),
    };
    if statfs_magic != expected_magic {
        return Err(JournalError::UnsupportedFilesystem);
    }
    Ok(identity)
}

fn decode_mount_path(field: &str) -> Result<std::path::PathBuf, JournalError> {
    #[cfg(unix)]
    {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};

        let input = field.as_bytes();
        let mut decoded = Vec::with_capacity(input.len());
        let mut index = 0;
        while index < input.len() {
            if input[index] != b'\\' {
                decoded.push(input[index]);
                index += 1;
                continue;
            }
            let escape = input
                .get(index + 1..index + 4)
                .ok_or(JournalError::UnsupportedFilesystem)?;
            decoded.push(match escape {
                b"040" => b' ',
                b"011" => b'\t',
                b"012" => b'\n',
                b"134" => b'\\',
                _ => return Err(JournalError::UnsupportedFilesystem),
            });
            index += 4;
        }
        Ok(std::path::PathBuf::from(OsString::from_vec(decoded)))
    }
    #[cfg(not(unix))]
    {
        let _ = field;
        Err(JournalError::UnsupportedPlatform)
    }
}
