//! Rust types for the Agora client protocol.
//!
//! The canonical definition is SPEC-002 (`docs/contracts/client-protocol.md`); these types
//! implement it and are checked against its shared JSON fixtures. This crate has no networking.

mod error;
mod ids;
mod message;
mod pacing;
mod version;

pub use error::{ErrorCode, ErrorResponse};
pub use ids::{
    AgentId, CatalogEntryId, EmptyId, MAX_SAFE_INTEGER, OutOfRange, RequestId, RunId, SessionId,
    StateId,
};
pub use message::{
    Action, ActionEntry, AgentObservation, AgentView, ClientMessage, Direction, EntryError,
    EntryResult, Observation, Placement, RunPhase, ServerMessage, View,
};
pub use pacing::{IntervalMs, Pacing, PacingMode};
pub use version::{InvalidVersion, PROTOCOL_VERSION, ProtocolVersion};
