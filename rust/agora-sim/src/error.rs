//! Simulation operation errors. The server maps these to client-facing error codes.

use thiserror::Error;

use crate::types::{AgentId, GridPos, StateId};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SpawnError {
    #[error("spawning after Start is not supported yet")]
    NotInSetup,
    #[error("cell {0} is outside the grid")]
    OutOfBounds(GridPos),
    #[error("cell {0} has terrain that blocks movement")]
    Blocked(GridPos),
    #[error("cell {0} is occupied")]
    Occupied(GridPos),
    #[error("no unoccupied cell is available")]
    NoFreeCell,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StartError {
    #[error("the run has already started")]
    AlreadyStarted,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SubmitError {
    #[error("the run has not started")]
    NotStarted,
    #[error("{0} does not exist")]
    UnknownAgent(AgentId),
    #[error("submission targets {supplied}, but the current state is {expected}")]
    WrongTargetState {
        expected: StateId,
        supplied: StateId,
    },
    #[error("{0} already has an accepted action for this state")]
    AlreadySubmitted(AgentId),
    /// The submission is well formed, but the action exceeds the agent's limits.
    #[error(transparent)]
    AgentLimit(#[from] AgentLimitError),
}

/// A well-formed action whose parameters exceed what the agent can do, such as its
/// movement budget. Kept separate from submission errors (wrong phase, agent, or state).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AgentLimitError {
    #[error("move distance {distance} exceeds the movement budget of {budget}")]
    DistanceBudgetExceeded { distance: u32, budget: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RemoveError {
    #[error("{0} does not exist")]
    UnknownAgent(AgentId),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AdvanceError {
    #[error("the run has not started")]
    NotStarted,
    #[error("the run has no agents to advance")]
    NoAgents,
    #[error("actions are missing for {} agent(s)", missing.len())]
    MissingActions { missing: Vec<AgentId> },
}
