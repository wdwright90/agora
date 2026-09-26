//! Protocol versioning.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The protocol version implemented by this crate.
pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(0, 1, 0);

/// Semantic version of the client protocol, written as `"MAJOR.MINOR.PATCH"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProtocolVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl ProtocolVersion {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Whether a peer using `other` may communicate with this version.
    ///
    /// The MVP accepts only an exact match. Semver-based compatibility applies after 1.0.0.
    pub fn is_compatible_with(self, other: Self) -> bool {
        self == other
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A string that is not `MAJOR.MINOR.PATCH` with decimal components.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid protocol version {0:?}; expected MAJOR.MINOR.PATCH")]
pub struct InvalidVersion(pub String);

impl FromStr for ProtocolVersion {
    type Err = InvalidVersion;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || InvalidVersion(s.to_owned());
        let component = |part: &str| {
            // Semver forbids leading zeros and signs; `u64::from_str` would accept "+1" and "01".
            let canonical = !part.is_empty()
                && part.bytes().all(|b| b.is_ascii_digit())
                && (part == "0" || !part.starts_with('0'));
            canonical.then(|| part.parse::<u64>().ok()).flatten()
        };
        let mut parts = s.split('.');
        let (Some(major), Some(minor), Some(patch), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid());
        };
        Ok(Self::new(
            component(major).ok_or_else(invalid)?,
            component(minor).ok_or_else(invalid)?,
            component(patch).ok_or_else(invalid)?,
        ))
    }
}

impl TryFrom<String> for ProtocolVersion {
    type Error = InvalidVersion;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<ProtocolVersion> for String {
    fn from(version: ProtocolVersion) -> String {
        version.to_string()
    }
}
