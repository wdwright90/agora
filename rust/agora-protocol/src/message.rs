//! Client and server messages. Each WebSocket text message carries one JSON object whose
//! `type` field selects the variant.

use serde::{Deserialize, Serialize};

use crate::error::{ErrorCode, ErrorResponse};
use crate::ids::{AgentId, CatalogEntryId, RequestId, RunId, SessionId, StateId};
use crate::pacing::{Pacing, PacingMode};
use crate::version::ProtocolVersion;

/// Messages sent by clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// First message on every connection: the protocol version handshake. It belongs to the
    /// connection, does not create a session, and is not a request, so it has no request ID.
    Hello { protocol_version: ProtocolVersion },
    /// Create a run from a catalog entry and establish this connection's session as its creator.
    CreateRun {
        request_id: RequestId,
        catalog_entry: CatalogEntryId,
    },
    /// Join an existing run, establishing this connection's session.
    JoinRun {
        request_id: RequestId,
        run_id: RunId,
    },
    /// Spawn an agent owned by this session.
    Spawn {
        request_id: RequestId,
        placement: Placement,
    },
    /// Close setup and start the run. Creator only.
    Start { request_id: RequestId },
    /// Submit actions for agents this session owns, all targeting `state_id`.
    Submit {
        request_id: RequestId,
        state_id: StateId,
        actions: Vec<ActionEntry>,
    },
    /// Become a viewer of the run until the connection closes.
    Watch { request_id: RequestId },
    /// Take pacing control while no other viewer holds it. Viewers only.
    ClaimPacing { request_id: RequestId },
    /// Change the pacing mode. Pacing controller only.
    SetPacing {
        request_id: RequestId,
        mode: PacingMode,
    },
    /// Allow one step while paused. Pacing controller only.
    StepOnce { request_id: RequestId },
}

impl ClientMessage {
    /// The `type` wire name of every client message. A frame whose `type` is not listed here
    /// is an unknown message type rather than a malformed message.
    pub const TYPES: &[&str] = &[
        "hello",
        "create_run",
        "join_run",
        "spawn",
        "start",
        "submit",
        "watch",
        "claim_pacing",
        "set_pacing",
        "step_once",
    ];

    /// The request ID, for every message except `hello`.
    pub fn request_id(&self) -> Option<RequestId> {
        match self {
            Self::Hello { .. } => None,
            Self::CreateRun { request_id, .. }
            | Self::JoinRun { request_id, .. }
            | Self::Spawn { request_id, .. }
            | Self::Start { request_id }
            | Self::Submit { request_id, .. }
            | Self::Watch { request_id }
            | Self::ClaimPacing { request_id }
            | Self::SetPacing { request_id, .. }
            | Self::StepOnce { request_id } => Some(*request_id),
        }
    }
}

/// Messages sent by the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Successful handshake.
    Welcome { protocol_version: ProtocolVersion },
    /// Successful `create_run`.
    RunCreated {
        request_id: RequestId,
        run_id: RunId,
        session_id: SessionId,
        catalog_entry: CatalogEntryId,
        state_id: StateId,
        phase: RunPhase,
    },
    /// Successful `join_run`.
    RunJoined {
        request_id: RequestId,
        run_id: RunId,
        session_id: SessionId,
        catalog_entry: CatalogEntryId,
        state_id: StateId,
        phase: RunPhase,
    },
    /// Successful `spawn`: the agent has been placed.
    Spawned {
        request_id: RequestId,
        agent_id: AgentId,
    },
    /// Successful `start`: setup is closed and the run has started.
    Started { request_id: RequestId },
    /// Response to `submit`: one result per action entry, in the order submitted.
    Submitted {
        request_id: RequestId,
        state_id: StateId,
        results: Vec<EntryResult>,
    },
    /// Pushed by the server, not a response: this session's agents' observations of a state.
    Observations {
        state_id: StateId,
        observations: Vec<AgentObservation>,
    },
    /// Successful `watch`: the session is a viewer. `view` and `pacing` are the run's current
    /// state.
    Watching {
        request_id: RequestId,
        view: View,
        pacing: Pacing,
    },
    /// Pushed by the server, not a response: the run's newest state, for viewers.
    ViewUpdate { view: View },
    /// Successful `claim_pacing`: this session holds pacing control.
    PacingClaimed { request_id: RequestId },
    /// Successful `set_pacing`: the new mode is in effect.
    PacingSet { request_id: RequestId },
    /// Successful `step_once`: one step may start once every agent is ready. The step's result
    /// arrives as a `view_update`.
    StepGranted { request_id: RequestId },
    /// Pushed by the server, not a response: the pacing mode or controller changed, for viewers.
    PacingUpdate { pacing: Pacing },
    /// Any failed request or rejected message.
    Error(ErrorResponse),
}

/// Spawn placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Placement {
    /// A specific cell. `(0, 0)` is the south-west corner; `x` grows east and `y` grows north.
    Cell { x: u32, y: u32 },
    /// A cell chosen uniformly among unoccupied cells.
    Random,
}

/// One agent's action in a `submit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionEntry {
    pub agent_id: AgentId,
    pub action: Action,
}

/// An agent action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// Move `distance` cells in `direction`. Distance 0 stays in place.
    Move { direction: Direction, distance: u32 },
}

/// Cardinal direction. North is `y + 1` and east is `x + 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North,
    East,
    South,
    West,
}

/// The outcome of one action entry. The action was accepted when `error` is absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryResult {
    pub agent_id: AgentId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<EntryError>,
}

/// Why one action entry was rejected. The same fields as an error response, without the
/// request ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryError {
    pub code: ErrorCode,
    /// Human-readable description. Clients must not parse it.
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Map<String, serde_json::Value>>,
}

/// One agent's observation of a state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentObservation {
    pub agent_id: AgentId,
    pub observation: Observation,
}

/// An agent observation. Empty in this version; later versions add fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {}

/// A complete view of a run's world, separate from agent observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    pub state_id: StateId,
    pub phase: RunPhase,
    /// Grid width in cells.
    pub width: u32,
    /// Grid height in cells.
    pub height: u32,
    /// Every agent, in ascending ID order.
    pub agents: Vec<AgentView>,
}

/// One agent's position in a view. `(0, 0)` is the south-west corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentView {
    pub agent_id: AgentId,
    pub x: u32,
    pub y: u32,
}

/// Run lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunPhase {
    Setup,
    Started,
}
