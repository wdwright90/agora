//! A single run's simulation world and its operations.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::error::{
    AdvanceError, AgentLimitError, ConfigError, SpawnError, StartError, SubmitError,
};
use crate::types::{
    AgentId, AgentView, GridPos, MOVEMENT_BUDGET, Move, Observation, Observations, Placement,
    Readiness, StateId, Status, Submission, ViewState,
};

/// ChaCha stream reserved for future environment generation.
#[expect(dead_code, reason = "reserved so later streams keep their numbers")]
const GENERATION_STREAM: u64 = 0;
const SPAWN_STREAM: u64 = 1;
const SHUFFLE_STREAM: u64 = 2;

/// Configuration for creating a simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimConfig {
    /// Grid width in cells.
    pub width: u32,
    /// Grid height in cells.
    pub height: u32,
    /// Run seed from which all of the run's random streams are derived.
    pub seed: u64,
}

/// Identifies an agent entity; read by future ECS systems that iterate agents.
#[derive(Component)]
#[expect(dead_code, reason = "no system reads agent entities yet")]
struct Agent(AgentId);

#[derive(Component)]
struct Position(GridPos);

/// Bounded grid dimensions plus a cell-to-agent index.
#[derive(Resource)]
struct Grid {
    width: u32,
    height: u32,
    /// Which agent, if any, occupies each cell, in row-major order (`y * width + x`).
    /// Answers "is this cell free?" in constant time during spawns and moves, and lists
    /// free cells in a fixed order for random spawning. It mirrors the agents' `Position`
    /// components and is updated alongside them on every spawn and move.
    occupancy: Vec<Option<AgentId>>,
}

impl Grid {
    fn index(&self, pos: GridPos) -> Option<usize> {
        (pos.x < self.width && pos.y < self.height)
            .then(|| pos.y as usize * self.width as usize + pos.x as usize)
    }

    fn position(&self, index: usize) -> GridPos {
        let width = self.width as usize;
        GridPos::new((index % width) as u32, (index / width) as u32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Setup,
    Started,
}

#[derive(Resource)]
struct Lifecycle {
    phase: Phase,
    state: StateId,
    next_agent: u64,
}

/// Agent entities by ID. The ordered map gives a deterministic base order for shuffling.
#[derive(Resource, Default)]
struct Agents(BTreeMap<AgentId, Entity>);

/// Accepted actions for the current state.
#[derive(Resource, Default)]
struct PendingActions(BTreeMap<AgentId, Move>);

/// Per-run random streams, each derived from the run seed.
#[derive(Resource)]
struct RunRng {
    spawn: ChaCha8Rng,
    shuffle: ChaCha8Rng,
}

fn stream(seed: u64, stream: u64) -> ChaCha8Rng {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    rng.set_stream(stream);
    rng
}

/// The simulation of one run: world state, action collection, and step execution.
///
/// Networking, authority, pacing, and deadlines belong to the server.
pub struct Simulation {
    world: World,
}

impl Simulation {
    /// Create a simulation in setup at state 0.
    pub fn new(config: SimConfig) -> Result<Self, ConfigError> {
        let SimConfig {
            width,
            height,
            seed,
        } = config;
        if width == 0 || height == 0 {
            return Err(ConfigError::EmptyGrid { width, height });
        }
        let cells = (width as usize)
            .checked_mul(height as usize)
            .ok_or(ConfigError::GridTooLarge { width, height })?;

        let mut world = World::new();
        world.insert_resource(Grid {
            width,
            height,
            occupancy: vec![None; cells],
        });
        world.insert_resource(Lifecycle {
            phase: Phase::Setup,
            state: StateId(0),
            next_agent: 1,
        });
        world.insert_resource(Agents::default());
        world.insert_resource(PendingActions::default());
        world.insert_resource(RunRng {
            spawn: stream(seed, SPAWN_STREAM),
            shuffle: stream(seed, SHUFFLE_STREAM),
        });
        Ok(Self { world })
    }

    /// Spawn an agent during setup and return its assigned ID.
    pub fn spawn(&mut self, placement: Placement) -> Result<AgentId, SpawnError> {
        if self.world.resource::<Lifecycle>().phase != Phase::Setup {
            return Err(SpawnError::NotInSetup);
        }

        let index = {
            let grid = self.world.resource::<Grid>();
            match placement {
                Placement::Cell(pos) => {
                    let index = grid.index(pos).ok_or(SpawnError::OutOfBounds(pos))?;
                    if grid.occupancy[index].is_some() {
                        return Err(SpawnError::Occupied(pos));
                    }
                    index
                }
                Placement::Random => {
                    let free: Vec<usize> = (0..grid.occupancy.len())
                        .filter(|&i| grid.occupancy[i].is_none())
                        .collect();
                    if free.is_empty() {
                        return Err(SpawnError::NoFreeCell);
                    }
                    let pick = self.random_index(free.len());
                    free[pick]
                }
            }
        };

        let id = {
            let mut lifecycle = self.world.resource_mut::<Lifecycle>();
            let id = AgentId(lifecycle.next_agent);
            lifecycle.next_agent += 1;
            id
        };
        let pos = {
            let mut grid = self.world.resource_mut::<Grid>();
            grid.occupancy[index] = Some(id);
            grid.position(index)
        };
        let entity = self.world.spawn((Agent(id), Position(pos))).id();
        self.world.resource_mut::<Agents>().0.insert(id, entity);
        Ok(id)
    }

    /// Close setup, open action collection, and return initial observations for state 0.
    ///
    /// The server enforces creator authority and the no-agents-and-no-viewers rejection.
    pub fn start(&mut self) -> Result<Observations, StartError> {
        let mut lifecycle = self.world.resource_mut::<Lifecycle>();
        if lifecycle.phase != Phase::Setup {
            return Err(StartError::AlreadyStarted);
        }
        lifecycle.phase = Phase::Started;
        Ok(self.observations())
    }

    /// Submit one agent's action for the current state.
    ///
    /// The first accepted submission for an agent and state is fixed.
    pub fn submit(&mut self, submission: Submission) -> Result<(), SubmitError> {
        let Submission {
            agent,
            target_state,
            action,
        } = submission;
        let lifecycle = self.world.resource::<Lifecycle>();
        if lifecycle.phase != Phase::Started {
            return Err(SubmitError::NotStarted);
        }
        let current = lifecycle.state;
        if !self.world.resource::<Agents>().0.contains_key(&agent) {
            return Err(SubmitError::UnknownAgent(agent));
        }
        if target_state != current {
            return Err(SubmitError::WrongTargetState {
                expected: current,
                supplied: target_state,
            });
        }
        if self
            .world
            .resource::<PendingActions>()
            .0
            .contains_key(&agent)
        {
            return Err(SubmitError::AlreadySubmitted(agent));
        }
        if action.distance > MOVEMENT_BUDGET {
            return Err(AgentLimitError::DistanceBudgetExceeded {
                distance: action.distance,
                budget: MOVEMENT_BUDGET,
            }
            .into());
        }
        self.world
            .resource_mut::<PendingActions>()
            .0
            .insert(agent, action);
        Ok(())
    }

    /// Report the run's phase/status and current state ID.
    pub fn readiness(&self) -> Readiness {
        let lifecycle = self.world.resource::<Lifecycle>();
        let agents = &self.world.resource::<Agents>().0;
        let status = match lifecycle.phase {
            Phase::Setup => Status::Setup,
            Phase::Started if agents.is_empty() => Status::StartedEmpty,
            Phase::Started => {
                let pending = &self.world.resource::<PendingActions>().0;
                Status::Collecting {
                    missing: agents
                        .keys()
                        .filter(|id| !pending.contains_key(id))
                        .copied()
                        .collect(),
                }
            }
        };
        Readiness {
            state: lifecycle.state,
            status,
        }
    }

    /// Execute one ready step and return observations for the resulting state.
    ///
    /// Fails without changing the world or pending actions if the step is not ready.
    /// The server checks its pacing gate before calling this.
    pub fn advance(&mut self) -> Result<Observations, AdvanceError> {
        match self.readiness().status {
            Status::Setup => return Err(AdvanceError::NotStarted),
            Status::StartedEmpty => return Err(AdvanceError::NoAgents),
            Status::Collecting { missing } if !missing.is_empty() => {
                return Err(AdvanceError::MissingActions { missing });
            }
            Status::Collecting { .. } => {}
        }

        let actions = std::mem::take(&mut self.world.resource_mut::<PendingActions>().0);
        let mut order: Vec<AgentId> = actions.keys().copied().collect();
        self.shuffle(&mut order);
        for agent in order {
            self.execute_move(agent, actions[&agent]);
        }

        self.world.resource_mut::<Lifecycle>().state.0 += 1;
        Ok(self.observations())
    }

    /// Complete view of the current world for viewers.
    pub fn view(&self) -> ViewState {
        let grid = self.world.resource::<Grid>();
        let agents = self
            .world
            .resource::<Agents>()
            .0
            .iter()
            .map(|(&id, &entity)| AgentView {
                id,
                position: self.position(entity),
            })
            .collect();
        ViewState {
            state: self.world.resource::<Lifecycle>().state,
            width: grid.width,
            height: grid.height,
            agents,
        }
    }

    /// Number of agents currently in the world.
    pub fn agent_count(&self) -> usize {
        self.world.resource::<Agents>().0.len()
    }

    /// Execute one move against the current world. Failed moves leave the agent in place.
    fn execute_move(&mut self, agent: AgentId, action: Move) {
        if action.distance == 0 {
            return;
        }
        let entity = self.world.resource::<Agents>().0[&agent];
        let from = self.position(entity);
        let mut grid = self.world.resource_mut::<Grid>();
        let Some(to) = from.neighbor(action.direction) else {
            return;
        };
        let Some(to_index) = grid.index(to) else {
            return;
        };
        if grid.occupancy[to_index].is_some() {
            return;
        }
        let from_index = grid.index(from).expect("agent position is in bounds");
        grid.occupancy[from_index] = None;
        grid.occupancy[to_index] = Some(agent);
        self.world
            .get_mut::<Position>(entity)
            .expect("agent entity has a position")
            .0 = to;
    }

    fn observations(&self) -> Observations {
        Observations {
            state: self.world.resource::<Lifecycle>().state,
            by_agent: self
                .world
                .resource::<Agents>()
                .0
                .keys()
                .map(|&id| (id, Observation::default()))
                .collect(),
        }
    }

    fn position(&self, entity: Entity) -> GridPos {
        self.world
            .get::<Position>(entity)
            .expect("agent entity has a position")
            .0
    }

    fn random_index(&mut self, len: usize) -> usize {
        use rand::RngExt;
        self.world
            .resource_mut::<RunRng>()
            .spawn
            .random_range(0..len)
    }

    fn shuffle(&mut self, order: &mut [AgentId]) {
        use rand::seq::SliceRandom;
        order.shuffle(&mut self.world.resource_mut::<RunRng>().shuffle);
    }
}
