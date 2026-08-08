use std::{error::Error, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Stable identity for an Intent Unit.
///
/// Generated values use UUID v4. Parsed and fixed values preserve any
/// syntactically valid UUID so external adapters can map existing identifiers.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IntentUnitId(Uuid);

impl IntentUnitId {
    /// Generates a new non-nil UUID v4 identity.
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wraps a fixed UUID value, primarily for deterministic construction.
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Borrows the underlying immutable UUID value.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for IntentUnitId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<Uuid> for IntentUnitId {
    fn from(value: Uuid) -> Self {
        Self::from_uuid(value)
    }
}

impl From<IntentUnitId> for Uuid {
    fn from(value: IntentUnitId) -> Self {
        value.0
    }
}

impl FromStr for IntentUnitId {
    type Err = ParseIntentUnitIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value)
            .map(Self::from_uuid)
            .map_err(ParseIntentUnitIdError)
    }
}

impl Serialize for IntentUnitId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for IntentUnitId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// Error returned when textual Intent Unit identity is not a valid UUID.
#[derive(Debug)]
pub struct ParseIntentUnitIdError(uuid::Error);

impl fmt::Display for ParseIntentUnitIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Intent Unit ID: {}", self.0)
    }
}

impl Error for ParseIntentUnitIdError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IntentSpecies, PhaseId, WorkflowId};
    use uuid::Version;

    #[test]
    fn test_generated_intent_unit_id_is_non_nil_v4() {
        let id = IntentUnitId::generate();

        assert!(!id.as_uuid().is_nil());
        assert_eq!(id.as_uuid().get_version(), Some(Version::Random));
    }

    #[test]
    fn test_generated_intent_unit_ids_differ() {
        assert_ne!(IntentUnitId::generate(), IntentUnitId::generate());
    }

    #[test]
    fn test_intent_unit_id_parse_display_round_trip() {
        let text = "67e55044-10b1-426f-9247-bb680e5fe0c8";
        let id: IntentUnitId = text.parse().expect("fixed UUID should parse");

        assert_eq!(id.to_string(), text);
    }

    #[test]
    fn test_intent_unit_id_rejects_malformed_text() {
        let error = "not-a-uuid"
            .parse::<IntentUnitId>()
            .expect_err("malformed UUID should fail");

        assert!(error.to_string().starts_with("invalid Intent Unit ID:"));
    }

    #[test]
    fn test_identifier_and_vocabulary_semantic_round_trip() {
        let values = (
            IntentUnitId::from_str("67e55044-10b1-426f-9247-bb680e5fe0c8")
                .expect("fixed ID should parse"),
            WorkflowId::new("flow").expect("workflow ID should be valid"),
            PhaseId::new("in progress").expect("phase should be valid"),
            IntentSpecies::new("feature").expect("species should be valid"),
        );
        let json = serde_json::to_string(&values).expect("values should serialize");
        let restored = serde_json::from_str(&json).expect("values should deserialize");

        assert_eq!(values, restored);
    }

    #[test]
    fn test_serialization_rejects_malformed_identifier() {
        assert!(serde_json::from_str::<IntentUnitId>(r#""not-a-uuid""#).is_err());
    }
}
