//! Client errors.

use agora_protocol::{ErrorCode, ErrorResponse, ServerMessage};
use thiserror::Error;
use tokio_tungstenite::tungstenite;

/// Why a client operation failed.
#[derive(Debug, Error)]
pub enum ClientError {
    /// The WebSocket connection could not be opened or failed.
    #[error("connection failed: {0}")]
    Transport(#[from] tungstenite::Error),
    /// The server rejected the request, or the handshake.
    #[error("server rejected the request: {} ({})", .0.message, .0.code)]
    Rejected(ErrorResponse),
    /// The connection closed before the response arrived.
    #[error("the connection is closed")]
    Closed,
    /// The server sent a message this client cannot read.
    #[error("unreadable server message: {0}")]
    Unreadable(#[from] serde_json::Error),
    /// The server answered with a message that does not fit the request. This indicates a
    /// protocol mismatch or a server bug.
    #[error("unexpected response from the server: {0:?}")]
    Unexpected(Box<ServerMessage>),
}

impl ClientError {
    /// The server's error code, if the server rejected the request.
    pub fn code(&self) -> Option<&ErrorCode> {
        match self {
            Self::Rejected(error) => Some(&error.code),
            _ => None,
        }
    }
}
