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

macro_rules! define_text_value {
    ($(#[$attribute:meta])* $name:ident) => {
        $(#[$attribute])*
        #[derive(Clone, Debug, Eq, Hash, PartialEq)]
        pub struct $name(String);

        impl $name {
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
}
