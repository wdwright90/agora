//! Value types exchanged with the simulation.

use std::collections::BTreeMap;
use std::fmt;

/// Maximum total movement distance an MVP agent may request in one step.
pub const MOVEMENT_BUDGET: u32 = 1;

/// Run-unique agent identifier, assigned in spawn order starting at 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentId(pub u64);

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "agent {}", self.0)
    }
}

/// World-state identifier. A new run starts at state 0; each real step increments it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct StateId(pub u64);

impl fmt::Display for StateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "state {}", self.0)
    }
}

/// Grid cell coordinates. `(0, 0)` is the south-west corner; `x` grows east and `y` grows north.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GridPos {
    pub x: u32,
    pub y: u32,
}

impl GridPos {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    /// The adjacent cell in `direction`, or `None` if it would leave the unsigned coordinate range.
    pub(crate) fn neighbor(self, direction: Direction) -> Option<Self> {
        let Self { x, y } = self;
        match direction {
            Direction::North => y.checked_add(1).map(|y| Self { x, y }),
            Direction::East => x.checked_add(1).map(|x| Self { x, y }),
            Direction::South => y.checked_sub(1).map(|y| Self { x, y }),
            Direction::West => x.checked_sub(1).map(|x| Self { x, y }),
        }
    }
}

impl fmt::Display for GridPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

/// Cardinal direction on the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

/// The MVP move action. Distance 0 stays still and ignores `direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    pub direction: Direction,
    pub distance: u32,
}

impl Move {
    /// Move one cell in `direction`.
    pub const fn step(direction: Direction) -> Self {
        Self {
            direction,
            distance: 1,
        }
    }

    /// Stay in place (distance 0).
    pub const fn stay() -> Self {
        Self {
            direction: Direction::North,
            distance: 0,
        }
    }
}

/// An action submission for one agent, targeting the current world state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Submission {
    pub agent: AgentId,
    pub target_state: StateId,
    pub action: Move,
}

/// Where a spawned agent is placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// A specific cell, which must be in bounds and unoccupied.
    Cell(GridPos),
    /// A cell chosen uniformly at random among all unoccupied cells.
    Random,
}

/// An agent observation. Empty for the MVP; richer perception is later work.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Observation {}

/// Observations for a completed state, keyed by agent ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observations {
    pub state: StateId,
    pub by_agent: BTreeMap<AgentId, Observation>,
}

/// Status reported by a readiness query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Before Start. Not ready to advance.
    Setup,
    /// Started with participating agents. Ready to advance when `missing` is empty.
    Collecting { missing: Vec<AgentId> },
    /// Started with no agents. Not ready to advance.
    StartedEmpty,
}

/// Result of a readiness query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Readiness {
    pub state: StateId,
    pub status: Status,
}

impl Readiness {
    /// Whether an advance request would currently succeed.
    pub fn is_ready(&self) -> bool {
        matches!(&self.status, Status::Collecting { missing } if missing.is_empty())
    }
}

/// One agent in a viewer state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentView {
    pub id: AgentId,
    pub position: GridPos,
}

/// Complete MVP view of the current world, separate from agent observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewState {
    pub state: StateId,
    pub width: u32,
    pub height: u32,
    /// Agents in ascending ID order.
    pub agents: Vec<AgentView>,
}
