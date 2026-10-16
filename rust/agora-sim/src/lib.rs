//! Headless Agora simulation.
//!
//! One [`Simulation`] holds one run's world: a bounded grid, agents with the capabilities of
//! their agent definitions, action collection, and step execution, including metabolism and
//! starvation. Behavior is specified by SPEC-001
//! (`docs/components/simulation/specs/lifecycle-and-movement.md`) under the simulation CDD.
//! Networking, authority, pacing, and rendering live in other packages.

mod error;
mod schedule;
mod simulation;
mod types;

pub use error::{AdvanceError, AgentLimitError, RemoveError, SpawnError, StartError, SubmitError};
pub use simulation::{SimConfig, Simulation};
pub use types::{
    Advanced, AgentId, AgentView, Direction, GridPos, MOVE_EFFORT, MOVEMENT_BUDGET, Move,
    Observation, Observations, Placement, Readiness, Removal, RemovalReason, StateId, Status,
    Submission, ViewState,
};
