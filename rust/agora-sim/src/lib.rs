//! Headless Agora simulation.
//!
//! One [`Simulation`] holds one run's world: a bounded grid, agents, action collection, and
//! step execution. Behavior is specified by SPEC-001
//! (`docs/components/simulation/specs/lifecycle-and-movement.md`) under the simulation CDD.
//! Networking, authority, pacing, and rendering live in other packages.

mod error;
mod simulation;
mod types;

pub use error::{AdvanceError, ConfigError, SpawnError, StartError, SubmitError};
pub use simulation::{SimConfig, Simulation};
pub use types::{
    AgentId, AgentView, Direction, GridPos, MOVEMENT_BUDGET, Move, Observation, Observations,
    Placement, Readiness, StateId, Status, Submission, ViewState,
};
