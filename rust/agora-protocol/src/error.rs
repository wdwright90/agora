//! The common error response.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ids::RequestId;

/// A request error. `request_id` is `None` when the failing message had no usable request ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub request_id: Option<RequestId>,
    pub code: ErrorCode,
    /// Human-readable description. Clients must not parse it.
    pub message: String,
    /// Code-specific structured details, documented per code in the contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Defines [`ErrorCode`] with its stable wire strings.
macro_rules! error_codes {
    ($($(#[$doc:meta])* $variant:ident => $wire:literal,)*) => {
        /// Stable, programmatic error code.
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum ErrorCode {
            $($(#[$doc])* $variant,)*
            /// A code this version does not know, preserved as received.
            Unrecognized(String),
        }

        impl ErrorCode {
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $wire,)*
                    Self::Unrecognized(code) => code,
                }
            }

            fn from_wire(code: String) -> Self {
                match code.as_str() {
                    $($wire => Self::$variant,)*
                    _ => Self::Unrecognized(code),
                }
            }
        }
    };
}

error_codes! {
    /// Not valid JSON, not an object, or missing or invalid fields.
    MalformedMessage => "malformed_message",
    /// The `type` field names no message this version knows.
    UnknownMessageType => "unknown_message_type",
    /// A message other than `hello` arrived before the handshake completed.
    HandshakeRequired => "handshake_required",
    /// The client's protocol version is not supported.
    UnsupportedProtocolVersion => "unsupported_protocol_version",
    /// `create_run` or `join_run` on a connection that already has a session.
    SessionAlreadyEstablished => "session_already_established",
    /// A run-scoped request before `create_run` or `join_run` succeeded.
    NoSession => "no_session",
    /// A request number at or below the highest admitted number with no retained record.
    StaleRequestId => "stale_request_id",
    /// A request number reused with different contents.
    RequestIdConflict => "request_id_conflict",
    /// A retry of a request whose stored result has expired.
    ResultExpired => "result_expired",
    /// The catalog has no entry with the requested ID.
    UnknownCatalogEntry => "unknown_catalog_entry",
    /// No run exists with the requested ID.
    UnknownRun => "unknown_run",
    /// The operation requires creator authority.
    NotCreator => "not_creator",
    /// The operation is only available before Start.
    RunAlreadyStarted => "run_already_started",
    /// Start was requested with no agents and no connected viewers.
    StartNotEligible => "start_not_eligible",
    /// The requested spawn cell is outside the grid.
    CellOutOfBounds => "cell_out_of_bounds",
    /// The requested spawn cell is occupied.
    CellOccupied => "cell_occupied",
    /// Random placement found no unoccupied cell.
    NoFreeCell => "no_free_cell",
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ErrorCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from_wire)
    }
}
