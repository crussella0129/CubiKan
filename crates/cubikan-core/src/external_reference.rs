use std::{error::Error, fmt, str::FromStr};

/// Maximum encoded byte length of an external-reference namespace.
pub const MAX_NAMESPACE_BYTES: usize = 64;
/// Maximum encoded byte length of caller-owned reference text.
pub const MAX_TEXT_BYTES: usize = 256;

/// Exact provider-neutral namespace with grammar `[a-z][a-z0-9._-]{0,63}`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReferenceNamespace(String);

impl ReferenceNamespace {
    /// Validates raw bytes, reporting malformed UTF-8 before domain grammar.
    pub fn from_bytes(value: &[u8]) -> Result<Self, ReferenceNamespaceError> {
        if value.len() > MAX_NAMESPACE_BYTES {
            return Err(ReferenceNamespaceError::TooLong {
                length: value.len(),
                maximum: MAX_NAMESPACE_BYTES,
            });
        }
        let value =
            std::str::from_utf8(value).map_err(|error| ReferenceNamespaceError::InvalidUtf8 {
                index: error.valid_up_to(),
                error_length: error.error_len(),
            })?;
        Self::new(value)
    }

    /// Validates and preserves a namespace without normalization.
    pub fn new(value: impl Into<String>) -> Result<Self, ReferenceNamespaceError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.is_empty() {
            return Err(ReferenceNamespaceError::Empty);
        }
        if bytes.len() > MAX_NAMESPACE_BYTES {
            return Err(ReferenceNamespaceError::TooLong {
                length: bytes.len(),
                maximum: MAX_NAMESPACE_BYTES,
            });
        }
        if let Some(index) = bytes.iter().position(|byte| *byte == 0) {
            return Err(ReferenceNamespaceError::Nul { index });
        }
        if !bytes[0].is_ascii_lowercase() {
            return Err(ReferenceNamespaceError::InvalidStart {
                index: 0,
                byte: bytes[0],
            });
        }
        if let Some((index, byte)) = bytes.iter().copied().enumerate().skip(1).find(|(_, byte)| {
            !(byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-'))
        }) {
            return Err(ReferenceNamespaceError::InvalidByte { index, byte });
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ReferenceNamespace {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ReferenceNamespace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ReferenceNamespace {
    type Err = ReferenceNamespaceError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<String> for ReferenceNamespace {
    type Error = ReferenceNamespaceError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ReferenceNamespace {
    type Error = ReferenceNamespaceError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl serde::Serialize for ReferenceNamespace {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ReferenceNamespace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Exact nonblank, NUL-free UTF-8 reference scope or value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReferenceText(String);

impl ReferenceText {
    /// Validates raw bytes with precise malformed-UTF-8 diagnostics.
    pub fn from_bytes(value: &[u8]) -> Result<Self, ReferenceTextError> {
        if value.len() > MAX_TEXT_BYTES {
            return Err(ReferenceTextError::TooLong {
                length: value.len(),
                maximum: MAX_TEXT_BYTES,
            });
        }
        let value =
            std::str::from_utf8(value).map_err(|error| ReferenceTextError::InvalidUtf8 {
                index: error.valid_up_to(),
                error_length: error.error_len(),
            })?;
        Self::new(value)
    }

    /// Validates and preserves caller-owned text byte-for-byte.
    pub fn new(value: impl Into<String>) -> Result<Self, ReferenceTextError> {
        let value = value.into();
        validate_text(&value)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ReferenceText {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ReferenceText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ReferenceText {
    type Err = ReferenceTextError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<String> for ReferenceText {
    type Error = ReferenceTextError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ReferenceText {
    type Error = ReferenceTextError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl serde::Serialize for ReferenceText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ReferenceText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Complete immutable identity of an externally owned value.
#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct ExternalReference {
    namespace: ReferenceNamespace,
    scope: ReferenceText,
    value: ReferenceText,
}

/// Positive caller-owned version of an immutable relationship definition.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Serialize)]
pub struct RelationshipDefinitionVersion(u64);

impl RelationshipDefinitionVersion {
    pub const fn new(value: u64) -> Result<Self, RelationshipDefinitionVersionError> {
        if value == 0 {
            Err(RelationshipDefinitionVersionError::Zero)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl<'de> serde::Deserialize<'de> for RelationshipDefinitionVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <u64 as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationshipDefinitionVersionError {
    Zero,
}

impl fmt::Display for RelationshipDefinitionVersionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("relationship definition version must be positive")
    }
}

impl Error for RelationshipDefinitionVersionError {}

/// Complete caller-owned identity of one relationship-definition version.
#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct RelationshipDefinitionKey {
    id: ReferenceNamespace,
    version: RelationshipDefinitionVersion,
}

impl RelationshipDefinitionKey {
    #[must_use]
    pub const fn new(id: ReferenceNamespace, version: RelationshipDefinitionVersion) -> Self {
        Self { id, version }
    }

    #[must_use]
    pub const fn id(&self) -> &ReferenceNamespace {
        &self.id
    }

    #[must_use]
    pub const fn version(&self) -> RelationshipDefinitionVersion {
        self.version
    }
}

/// Explicit allow/reject relationship policy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipPolicy {
    Allow,
    Reject,
}

impl FromStr for RelationshipPolicy {
    type Err = RelationshipPolicyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "allow" => Ok(Self::Allow),
            "reject" => Ok(Self::Reject),
            _ => Err(RelationshipPolicyError),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelationshipPolicyError;

impl fmt::Display for RelationshipPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("relationship policy must be `allow` or `reject`")
    }
}

impl Error for RelationshipPolicyError {}

/// Immutable provider-neutral relationship definition semantics.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RelationshipDefinition {
    key: RelationshipDefinitionKey,
    source_species: Option<crate::IntentSpecies>,
    target_species: Option<crate::IntentSpecies>,
    self_policy: RelationshipPolicy,
    cycle_policy: RelationshipPolicy,
}

impl RelationshipDefinition {
    #[must_use]
    pub const fn new(
        key: RelationshipDefinitionKey,
        source_species: Option<crate::IntentSpecies>,
        target_species: Option<crate::IntentSpecies>,
        self_policy: RelationshipPolicy,
        cycle_policy: RelationshipPolicy,
    ) -> Self {
        Self {
            key,
            source_species,
            target_species,
            self_policy,
            cycle_policy,
        }
    }

    #[must_use]
    pub const fn key(&self) -> &RelationshipDefinitionKey {
        &self.key
    }

    #[must_use]
    pub const fn source_species(&self) -> Option<&crate::IntentSpecies> {
        self.source_species.as_ref()
    }

    #[must_use]
    pub const fn target_species(&self) -> Option<&crate::IntentSpecies> {
        self.target_species.as_ref()
    }

    #[must_use]
    pub const fn self_policy(&self) -> RelationshipPolicy {
        self.self_policy
    }

    #[must_use]
    pub const fn cycle_policy(&self) -> RelationshipPolicy {
        self.cycle_policy
    }
}

/// Complete directed relationship identity, independent of lifecycle state.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RelationshipIdentity {
    definition: RelationshipDefinitionKey,
    source: crate::IntentUnitId,
    target: crate::IntentUnitId,
}

impl RelationshipIdentity {
    #[must_use]
    pub const fn new(
        definition: RelationshipDefinitionKey,
        source: crate::IntentUnitId,
        target: crate::IntentUnitId,
    ) -> Self {
        Self {
            definition,
            source,
            target,
        }
    }

    #[must_use]
    pub const fn definition(&self) -> &RelationshipDefinitionKey {
        &self.definition
    }

    #[must_use]
    pub const fn source(&self) -> crate::IntentUnitId {
        self.source
    }

    #[must_use]
    pub const fn target(&self) -> crate::IntentUnitId {
        self.target
    }
}

/// Provenance subject: the whole aggregate or one exact revision, including zero.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "revision")]
pub enum AssociationSubject {
    WholeUnit,
    Revision(u64),
}

impl AssociationSubject {
    pub fn from_parts(kind: &str, revision: Option<u64>) -> Result<Self, AssociationSubjectError> {
        match (kind, revision) {
            ("whole_unit", None) => Ok(Self::WholeUnit),
            ("revision", Some(revision)) => Ok(Self::Revision(revision)),
            ("whole_unit" | "revision", _) => Err(AssociationSubjectError::InvalidShape),
            _ => Err(AssociationSubjectError::InvalidKind),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssociationSubjectError {
    InvalidKind,
    InvalidShape,
}

impl fmt::Display for AssociationSubjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid association subject")
    }
}

impl Error for AssociationSubjectError {}

/// Complete provider-neutral recorded-association identity.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RecordedAssociation {
    unit_id: crate::IntentUnitId,
    subject: AssociationSubject,
    reference: ExternalReference,
}

impl RecordedAssociation {
    #[must_use]
    pub const fn new(
        unit_id: crate::IntentUnitId,
        subject: AssociationSubject,
        reference: ExternalReference,
    ) -> Self {
        Self {
            unit_id,
            subject,
            reference,
        }
    }

    #[must_use]
    pub const fn unit_id(&self) -> crate::IntentUnitId {
        self.unit_id
    }

    #[must_use]
    pub const fn subject(&self) -> AssociationSubject {
        self.subject
    }

    #[must_use]
    pub const fn reference(&self) -> &ExternalReference {
        &self.reference
    }
}

impl ExternalReference {
    #[must_use]
    pub const fn new(
        namespace: ReferenceNamespace,
        scope: ReferenceText,
        value: ReferenceText,
    ) -> Self {
        Self {
            namespace,
            scope,
            value,
        }
    }

    #[must_use]
    pub const fn namespace(&self) -> &ReferenceNamespace {
        &self.namespace
    }

    #[must_use]
    pub const fn scope(&self) -> &ReferenceText {
        &self.scope
    }

    #[must_use]
    pub const fn value(&self) -> &ReferenceText {
        &self.value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceNamespaceError {
    Empty,
    TooLong {
        length: usize,
        maximum: usize,
    },
    InvalidUtf8 {
        index: usize,
        error_length: Option<usize>,
    },
    Nul {
        index: usize,
    },
    InvalidStart {
        index: usize,
        byte: u8,
    },
    InvalidByte {
        index: usize,
        byte: u8,
    },
}

impl fmt::Display for ReferenceNamespaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("reference namespace is empty"),
            Self::TooLong { length, maximum } => write!(
                formatter,
                "reference namespace is {length} bytes; maximum is {maximum}"
            ),
            Self::InvalidUtf8 {
                index,
                error_length,
            } => write!(
                formatter,
                "reference namespace has invalid UTF-8 at byte index {index} with error length {error_length:?}"
            ),
            Self::Nul { index } => write!(
                formatter,
                "reference namespace contains NUL at byte index {index}"
            ),
            Self::InvalidStart { index, byte } => write!(
                formatter,
                "reference namespace has invalid first byte {byte} at index {index}"
            ),
            Self::InvalidByte { index, byte } => write!(
                formatter,
                "reference namespace has invalid byte {byte} at index {index}"
            ),
        }
    }
}

impl Error for ReferenceNamespaceError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceTextError {
    Empty,
    Blank,
    TooLong {
        length: usize,
        maximum: usize,
    },
    InvalidUtf8 {
        index: usize,
        error_length: Option<usize>,
    },
    Nul {
        index: usize,
    },
}

impl fmt::Display for ReferenceTextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("reference text is empty"),
            Self::Blank => formatter.write_str("reference text must not be blank"),
            Self::TooLong { length, maximum } => write!(
                formatter,
                "reference text is {length} bytes; maximum is {maximum}"
            ),
            Self::InvalidUtf8 {
                index,
                error_length,
            } => write!(
                formatter,
                "reference text has invalid UTF-8 at byte index {index} with error length {error_length:?}"
            ),
            Self::Nul { index } => write!(
                formatter,
                "reference text contains NUL at byte index {index}"
            ),
        }
    }
}

impl Error for ReferenceTextError {}

fn validate_text(value: &str) -> Result<(), ReferenceTextError> {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return Err(ReferenceTextError::Empty);
    }
    if bytes.len() > MAX_TEXT_BYTES {
        return Err(ReferenceTextError::TooLong {
            length: bytes.len(),
            maximum: MAX_TEXT_BYTES,
        });
    }
    if let Some(index) = bytes.iter().position(|byte| *byte == 0) {
        return Err(ReferenceTextError::Nul { index });
    }
    if value.trim().is_empty() {
        return Err(ReferenceTextError::Blank);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_preserves_exact_valid_bytes() {
        let value = ReferenceNamespace::new("git.commit.sha256")
            .expect("canonical namespace should validate");
        assert_eq!(value.as_str(), "git.commit.sha256");
    }

    #[test]
    fn namespace_reports_exact_bound_and_byte_index() {
        assert_eq!(
            ReferenceNamespace::new("a".repeat(65)),
            Err(ReferenceNamespaceError::TooLong {
                length: 65,
                maximum: 64,
            })
        );
        assert_eq!(
            ReferenceNamespace::new("ab/C"),
            Err(ReferenceNamespaceError::InvalidByte {
                index: 2,
                byte: b'/',
            })
        );
    }

    #[test]
    fn text_preserves_whitespace_without_normalization() {
        let value =
            ReferenceText::new("  caller scope  ").expect("nonblank caller text should validate");
        assert_eq!(value.as_str(), "  caller scope  ");
    }

    #[test]
    fn text_reports_nul_and_utf8_byte_length() {
        assert_eq!(
            ReferenceText::new("é".repeat(129)),
            Err(ReferenceTextError::TooLong {
                length: 258,
                maximum: 256,
            })
        );
        assert_eq!(
            ReferenceText::new("é\0x"),
            Err(ReferenceTextError::Nul { index: 2 })
        );
    }
}
