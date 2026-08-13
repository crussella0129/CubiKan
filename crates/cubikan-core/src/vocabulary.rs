use std::{error::Error, fmt, str::FromStr};

/// Validation failure shared by CubiKan's caller-defined text values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VocabularyError {
    /// The supplied text was empty or contained only whitespace.
    Blank,
}

impl fmt::Display for VocabularyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blank => formatter.write_str("value must not be blank"),
        }
    }
}

impl Error for VocabularyError {}

/// Precise error for the additive bounded vocabulary conversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VocabularyValidationError {
    Empty,
    TooLong { length: usize, maximum: usize },
    InvalidUtf8 { index: usize, length: usize },
    Nul { index: usize },
    Blank,
}

impl fmt::Display for VocabularyValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("value is empty"),
            Self::TooLong { length, maximum } => {
                write!(formatter, "value is {length} bytes; maximum is {maximum}")
            }
            Self::InvalidUtf8 { index, length } => write!(
                formatter,
                "value has {length} invalid UTF-8 byte(s) at byte index {index}"
            ),
            Self::Nul { index } => write!(formatter, "value contains NUL at byte index {index}"),
            Self::Blank => formatter.write_str("value must not be blank"),
        }
    }
}

impl Error for VocabularyValidationError {}

macro_rules! define_text_value {
    ($(#[$attribute:meta])* $name:ident) => {
        $(#[$attribute])*
        #[derive(Clone, Debug, Eq, Hash, PartialEq)]
        pub struct $name(String);

        impl $name {
            /// Validates raw bytes and reports malformed UTF-8 precisely.
            pub fn from_bytes(value: &[u8]) -> Result<Self, VocabularyValidationError> {
                if value.is_empty() {
                    return Err(VocabularyValidationError::Empty);
                }
                if value.len() > crate::MAX_TEXT_BYTES {
                    return Err(VocabularyValidationError::TooLong {
                        length: value.len(),
                        maximum: crate::MAX_TEXT_BYTES,
                    });
                }
                let text = std::str::from_utf8(value).map_err(|error| {
                    let index = error.valid_up_to();
                    VocabularyValidationError::InvalidUtf8 {
                        index,
                        length: error.error_len().unwrap_or(value.len() - index),
                    }
                })?;
                if let Some(index) = value.iter().position(|byte| *byte == 0) {
                    return Err(VocabularyValidationError::Nul { index });
                }
                if text.trim().is_empty() {
                    return Err(VocabularyValidationError::Blank);
                }
                Ok(Self(text.to_owned()))
            }

            /// Validates and preserves caller-supplied text exactly.
            pub fn new(value: impl Into<String>) -> Result<Self, VocabularyError> {
                let value = value.into();
                if value.trim().is_empty() {
                    Err(VocabularyError::Blank)
                } else {
                    Ok(Self(value))
                }
            }

            /// Borrows the original caller-supplied text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }

        impl FromStr for $name {
            type Err = VocabularyError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = VocabularyError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = VocabularyError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

define_text_value!(
    /// Identity of a caller-declared workflow definition.
    WorkflowId
);

define_text_value!(
    /// Identity of an arbitrary phase within a workflow.
    PhaseId
);

define_text_value!(
    /// Caller-defined species provenance for an Intent Unit.
    IntentSpecies
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_values_preserve_non_blank_text() {
        let workflow = WorkflowId::new("  support-flow  ").expect("workflow ID should be valid");
        let phase = PhaseId::new("待機中").expect("Unicode phase should be valid");
        let species = IntentSpecies::new("kpi:cycle-time").expect("custom species should be valid");

        assert_eq!(workflow.as_str(), "  support-flow  ");
        assert_eq!(phase.as_str(), "待機中");
        assert_eq!(species.as_str(), "kpi:cycle-time");
    }

    #[test]
    fn test_domain_values_reject_empty_text() {
        assert_eq!(WorkflowId::new(""), Err(VocabularyError::Blank));
        assert_eq!(PhaseId::new(""), Err(VocabularyError::Blank));
        assert_eq!(IntentSpecies::new(""), Err(VocabularyError::Blank));
    }

    #[test]
    fn test_domain_values_reject_whitespace_only_text() {
        assert_eq!(WorkflowId::new(" \t\n"), Err(VocabularyError::Blank));
        assert_eq!(PhaseId::new("\u{2003}"), Err(VocabularyError::Blank));
        assert_eq!(IntentSpecies::new("   "), Err(VocabularyError::Blank));
    }

    #[test]
    fn test_serialization_rejects_blank_vocabulary() {
        assert!(serde_json::from_str::<WorkflowId>(r#""""#).is_err());
        assert!(serde_json::from_str::<PhaseId>(r#""   ""#).is_err());
        assert!(serde_json::from_str::<IntentSpecies>(r#""\t""#).is_err());
    }

    #[test]
    fn test_domain_values_enforce_shared_byte_and_nul_bounds() {
        assert_eq!(
            WorkflowId::from_bytes("é".repeat(129).as_bytes()),
            Err(VocabularyValidationError::TooLong {
                length: 258,
                maximum: 256,
            })
        );
        assert_eq!(
            PhaseId::from_bytes("é\0x".as_bytes()),
            Err(VocabularyValidationError::Nul { index: 2 })
        );
    }
}
