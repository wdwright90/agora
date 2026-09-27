//! Identifier and counter types.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Largest integer every JSON implementation represents exactly (2^53 − 1).
pub const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;

/// An integer outside the range allowed by the protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{value} is outside the allowed range {min}..={max}")]
pub struct OutOfRange {
    pub value: u64,
    pub min: u64,
    pub max: u64,
}

/// Defines a JSON-safe integer newtype that is range-checked on construction and deserialization.
macro_rules! safe_integer {
    ($(#[$doc:meta])* $name:ident, min = $min:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "u64", into = "u64")]
        pub struct $name(u64);

        impl $name {
            pub const MIN: u64 = $min;

            /// Returns `None` outside `MIN..=MAX_SAFE_INTEGER`.
            pub const fn new(value: u64) -> Option<Self> {
                if value >= Self::MIN && value <= MAX_SAFE_INTEGER {
                    Some(Self(value))
                } else {
                    None
                }
            }

            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl TryFrom<u64> for $name {
            type Error = OutOfRange;

            fn try_from(value: u64) -> Result<Self, Self::Error> {
                Self::new(value).ok_or(OutOfRange {
                    value,
                    min: Self::MIN,
                    max: MAX_SAFE_INTEGER,
                })
            }
        }

        impl From<$name> for u64 {
            fn from(id: $name) -> u64 {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

safe_integer!(
    /// Per-session request number, increasing from 1. Echoed in the matching response.
    RequestId,
    min = 1
);

safe_integer!(
    /// Run-unique agent identifier.
    AgentId,
    min = 1
);

safe_integer!(
    /// World-state identifier. A run starts at state 0.
    StateId,
    min = 0
);

/// Defines an opaque, non-empty string identifier.
macro_rules! opaque_string {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// Returns `None` for an empty string.
            pub fn new(value: impl Into<String>) -> Option<Self> {
                let value = value.into();
                (!value.is_empty()).then_some(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = EmptyId;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value).ok_or(EmptyId)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> String {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

/// An opaque string identifier that was empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("identifier must not be empty")]
pub struct EmptyId;

opaque_string!(
    /// Server-assigned run identifier, shared with joining clients.
    RunId
);

opaque_string!(
    /// Server-assigned logical session identifier.
    SessionId
);

opaque_string!(
    /// Environment catalog entry identifier.
    CatalogEntryId
);
