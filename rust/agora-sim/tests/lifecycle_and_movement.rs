//! Acceptance tests for SPEC-001 (simulation lifecycle and movement).
//! Test names start with the requirement ID they verify.

use std::collections::BTreeSet;
use std::sync::Arc;

use agora_env::{AgentDefinition, Environment, KindId, Layout, Metabolism, builtin};
use agora_sim::{
    AdvanceError, Advanced, AgentId, AgentLimitError, Direction, GridPos, MOVE_EFFORT, Move,
    Observation, Placement, Removal, RemovalReason, RemoveError, SimConfig, Simulation, SpawnError,
    StartError, StateId, Status, Submission, SubmitError, ViewState,
};

/// An agent with no capabilities.
const BARE: AgentDefinition = AgentDefinition { metabolism: None };

/// A simulation on a layout drawn as rows, north first: `#` is a wall, anything else floor.
fn sim_on(rows: &[&str], seed: u64) -> Simulation {
    let kind = |id: &str| KindId::new(id).unwrap();
    let width = rows[0].len() as u32;
    let cells = rows
        .iter()
        .rev()
        .flat_map(|row| row.chars())
        .map(|c| {
            kind(if c == '#' {
                builtin::WALL
            } else {
                builtin::FLOOR
            })
        })
        .collect();
    let layout = Layout::new(width, rows.len() as u32, cells).unwrap();
    let environment = Environment::new(Arc::new(builtin::registry()), Arc::new(layout)).unwrap();
    Simulation::new(SimConfig { environment, seed })
}

/// A simulation on an open floor.
fn sim(width: u32, height: u32, seed: u64) -> Simulation {
    let row = ".".repeat(width as usize);
    sim_on(&vec![row.as_str(); height as usize], seed)
}

fn spawn_at(sim: &mut Simulation, x: u32, y: u32) -> AgentId {
    sim.spawn(Placement::Cell(GridPos::new(x, y)), &BARE)
        .expect("spawn succeeds")
}

fn submit(sim: &mut Simulation, agent: AgentId, action: Move) -> Result<(), SubmitError> {
    let target_state = sim.readiness().state;
    sim.submit(Submission {
        agent,
        target_state,
        action,
    })
}

fn position(sim: &Simulation, agent: AgentId) -> GridPos {
    sim.view()
        .agents
        .iter()
        .find(|a| a.id == agent)
        .expect("agent exists")
        .position
}

fn assert_no_shared_cells(view: &ViewState) {
    let cells: BTreeSet<GridPos> = view.agents.iter().map(|a| a.position).collect();
    assert_eq!(
        cells.len(),
        view.agents.len(),
        "agents share a cell: {view:?}"
    );
}

#[test]
fn r01_new_simulation_is_in_setup_at_state_zero() {
    let sim = sim(10, 10, 0);
    let readiness = sim.readiness();
    assert_eq!(readiness.state, StateId(0));
    assert_eq!(readiness.status, Status::Setup);
    assert_eq!(sim.agent_count(), 0);
}

#[test]
fn r01_r20_grid_and_terrain_come_from_the_layout() {
    // Built-in order: floor is kind 0 and wall is kind 1.
    let sim = sim_on(&["#..", "..#"], 0);
    let view = sim.view();
    assert_eq!((view.width, view.height), (3, 2));
    // Row-major from the south-west corner: the southern row, then the northern one.
    assert_eq!(sim.terrain(), [0, 0, 1, 1, 0, 0]);
    let wall = sim.terrain()[2];
    assert_eq!(
        sim.kinds().iter().nth(wall).unwrap().id.as_str(),
        builtin::WALL
    );
}

#[test]
fn r01_agents_get_the_built_in_agent_kind() {
    let mut sim = sim(3, 3, 0);
    spawn_at(&mut sim, 0, 0);
    assert_eq!(sim.view().agents[0].kind, builtin::agent_kind());
}

#[test]
fn r02_directions_follow_documented_axes() {
    let cases = [
        (Direction::North, GridPos::new(5, 6)),
        (Direction::East, GridPos::new(6, 5)),
        (Direction::South, GridPos::new(5, 4)),
        (Direction::West, GridPos::new(4, 5)),
    ];
    for (direction, expected) in cases {
        let mut sim = sim(10, 10, 0);
        let agent = spawn_at(&mut sim, 5, 5);
        sim.start().unwrap();
        submit(&mut sim, agent, Move::step(direction)).unwrap();
        sim.advance().unwrap();
        assert_eq!(position(&sim, agent), expected, "{direction:?}");
    }
}

#[test]
fn r03_explicit_spawn_places_agent_without_changing_state() {
    let mut sim = sim(10, 10, 0);
    let agent = spawn_at(&mut sim, 9, 0);
    assert_eq!(position(&sim, agent), GridPos::new(9, 0));
    assert_eq!(sim.readiness().state, StateId(0));
}

#[test]
fn r03_out_of_bounds_and_occupied_cells_are_rejected() {
    let mut sim = sim(10, 10, 0);
    spawn_at(&mut sim, 3, 3);
    let before = sim.view();

    for pos in [GridPos::new(10, 0), GridPos::new(0, 10)] {
        assert_eq!(
            sim.spawn(Placement::Cell(pos), &BARE),
            Err(SpawnError::OutOfBounds(pos))
        );
    }
    let occupied = GridPos::new(3, 3);
    assert_eq!(
        sim.spawn(Placement::Cell(occupied), &BARE),
        Err(SpawnError::Occupied(occupied))
    );
    assert_eq!(sim.view(), before);
}

#[test]
fn r03_cells_whose_terrain_blocks_movement_are_rejected() {
    let mut sim = sim_on(&[".#", ".."], 0);
    let wall = GridPos::new(1, 1);
    assert_eq!(
        sim.spawn(Placement::Cell(wall), &BARE),
        Err(SpawnError::Blocked(wall))
    );
    assert_eq!(sim.agent_count(), 0);
    assert_eq!(spawn_at(&mut sim, 0, 1), AgentId(1));
}

#[test]
fn r04_random_spawns_fill_every_free_cell_then_fail() {
    let mut sim = sim(3, 3, 7);
    spawn_at(&mut sim, 1, 1);
    for _ in 0..8 {
        sim.spawn(Placement::Random, &BARE)
            .expect("a free cell exists");
    }
    let view = sim.view();
    assert_eq!(view.agents.len(), 9);
    assert_no_shared_cells(&view);
    assert_eq!(
        sim.spawn(Placement::Random, &BARE),
        Err(SpawnError::NoFreeCell)
    );
    assert_eq!(sim.view(), view);
}

#[test]
fn r04_random_spawn_picks_cover_the_free_cells() {
    // With one cell occupied on a 2x2 grid, the first random pick lands on each of the
    // three free cells for some seed, and never on the occupied one.
    let mut seen = BTreeSet::new();
    for seed in 0..200 {
        let mut sim = sim(2, 2, seed);
        spawn_at(&mut sim, 0, 0);
        let agent = sim.spawn(Placement::Random, &BARE).unwrap();
        seen.insert(position(&sim, agent));
    }
    let expected: BTreeSet<_> = [GridPos::new(1, 0), GridPos::new(0, 1), GridPos::new(1, 1)]
        .into_iter()
        .collect();
    assert_eq!(seen, expected);
}

#[test]
fn r04_random_spawns_skip_blocking_terrain() {
    // Two floor cells among walls: two random spawns fill them, and a third finds no cell.
    for seed in 0..20 {
        let mut sim = sim_on(&["#.#", "##.", "###"], seed);
        let a = sim.spawn(Placement::Random, &BARE).unwrap();
        let b = sim.spawn(Placement::Random, &BARE).unwrap();
        let cells: BTreeSet<_> = [position(&sim, a), position(&sim, b)].into_iter().collect();
        let floors: BTreeSet<_> = [GridPos::new(1, 2), GridPos::new(2, 1)]
            .into_iter()
            .collect();
        assert_eq!(cells, floors);
        assert_eq!(
            sim.spawn(Placement::Random, &BARE),
            Err(SpawnError::NoFreeCell)
        );
    }
}

#[test]
fn r05_ids_are_sequential_and_failed_spawns_do_not_consume_ids() {
    let mut sim = sim(10, 10, 0);
    assert_eq!(spawn_at(&mut sim, 0, 0), AgentId(1));
    assert!(
        sim.spawn(Placement::Cell(GridPos::new(0, 0)), &BARE)
            .is_err()
    );
    assert_eq!(spawn_at(&mut sim, 1, 0), AgentId(2));
    assert_eq!(sim.spawn(Placement::Random, &BARE), Ok(AgentId(3)));
}

#[test]
fn r06_spawn_after_start_is_rejected() {
    let mut sim = sim(10, 10, 0);
    spawn_at(&mut sim, 0, 0);
    sim.start().unwrap();
    assert_eq!(
        sim.spawn(Placement::Random, &BARE),
        Err(SpawnError::NotInSetup)
    );
    assert_eq!(sim.agent_count(), 1);
}

#[test]
fn r07_start_returns_state_zero_observations_for_every_agent() {
    let mut sim = sim(10, 10, 0);
    let a = spawn_at(&mut sim, 0, 0);
    let b = spawn_at(&mut sim, 1, 0);
    let observations = sim.start().unwrap();
    assert_eq!(observations.state, StateId(0));
    assert_eq!(
        observations.by_agent.keys().copied().collect::<Vec<_>>(),
        [a, b]
    );
    assert_eq!(sim.readiness().state, StateId(0));
    assert_eq!(
        sim.readiness().status,
        Status::Collecting {
            missing: vec![a, b]
        }
    );
}

#[test]
fn r07_start_with_zero_agents_is_allowed() {
    let mut sim = sim(10, 10, 0);
    let observations = sim.start().unwrap();
    assert!(observations.by_agent.is_empty());
    assert_eq!(sim.readiness().status, Status::StartedEmpty);
}

#[test]
fn r07_start_can_only_happen_once() {
    let mut sim = sim(10, 10, 0);
    sim.start().unwrap();
    assert_eq!(sim.start(), Err(StartError::AlreadyStarted));
}

#[test]
fn r08_submission_errors_in_check_order() {
    let mut sim = sim(10, 10, 0);
    let agent = spawn_at(&mut sim, 0, 0);
    let valid = Submission {
        agent,
        target_state: StateId(0),
        action: Move::stay(),
    };

    assert_eq!(sim.submit(valid), Err(SubmitError::NotStarted));
    sim.start().unwrap();

    let unknown = AgentId(99);
    assert_eq!(
        sim.submit(Submission {
            agent: unknown,
            target_state: StateId(5),
            ..valid
        }),
        Err(SubmitError::UnknownAgent(unknown))
    );
    assert_eq!(
        sim.submit(Submission {
            target_state: StateId(1),
            ..valid
        }),
        Err(SubmitError::WrongTargetState {
            expected: StateId(0),
            supplied: StateId(1),
        })
    );
    let too_far = Move {
        direction: Direction::East,
        distance: 2,
    };
    assert_eq!(
        sim.submit(Submission {
            action: too_far,
            ..valid
        }),
        Err(SubmitError::AgentLimit(
            AgentLimitError::DistanceBudgetExceeded {
                distance: 2,
                budget: 1,
            }
        ))
    );

    // None of the rejections filled the slot, so a valid submission is still accepted.
    assert_eq!(
        sim.readiness().status,
        Status::Collecting {
            missing: vec![agent]
        }
    );
    assert_eq!(sim.submit(valid), Ok(()));
}

#[test]
fn r08_past_state_targets_are_rejected() {
    let mut sim = sim(10, 10, 0);
    let agent = spawn_at(&mut sim, 0, 0);
    sim.start().unwrap();
    submit(&mut sim, agent, Move::stay()).unwrap();
    sim.advance().unwrap();
    assert_eq!(
        sim.submit(Submission {
            agent,
            target_state: StateId(0),
            action: Move::stay(),
        }),
        Err(SubmitError::WrongTargetState {
            expected: StateId(1),
            supplied: StateId(0),
        })
    );
}

#[test]
fn r09_first_accepted_submission_is_fixed() {
    let mut sim = sim(10, 10, 0);
    let agent = spawn_at(&mut sim, 5, 5);
    sim.start().unwrap();
    submit(&mut sim, agent, Move::step(Direction::North)).unwrap();
    assert_eq!(
        submit(&mut sim, agent, Move::step(Direction::South)),
        Err(SubmitError::AlreadySubmitted(agent))
    );
    sim.advance().unwrap();
    assert_eq!(position(&sim, agent), GridPos::new(5, 6));
}

#[test]
fn r10_missing_agents_shrink_as_actions_are_accepted() {
    let mut sim = sim(10, 10, 0);
    let a = spawn_at(&mut sim, 0, 0);
    let b = spawn_at(&mut sim, 1, 0);
    sim.start().unwrap();

    submit(&mut sim, b, Move::stay()).unwrap();
    let readiness = sim.readiness();
    assert_eq!(readiness.status, Status::Collecting { missing: vec![a] });
    assert!(!readiness.is_ready());

    submit(&mut sim, a, Move::stay()).unwrap();
    assert!(sim.readiness().is_ready());
}

#[test]
fn r10_setup_and_started_empty_are_not_ready() {
    let mut sim = sim(10, 10, 0);
    assert!(!sim.readiness().is_ready());
    sim.start().unwrap();
    assert_eq!(sim.readiness().status, Status::StartedEmpty);
    assert!(!sim.readiness().is_ready());
}

#[test]
fn r11_advance_before_ready_fails_without_changes() {
    let mut sim = sim(10, 10, 0);
    assert_eq!(sim.advance(), Err(AdvanceError::NotStarted));

    let a = spawn_at(&mut sim, 0, 0);
    let b = spawn_at(&mut sim, 5, 5);
    sim.start().unwrap();
    submit(&mut sim, a, Move::step(Direction::North)).unwrap();
    let view = sim.view();
    let readiness = sim.readiness();

    assert_eq!(
        sim.advance(),
        Err(AdvanceError::MissingActions { missing: vec![b] })
    );
    assert_eq!(sim.view(), view);
    assert_eq!(sim.readiness(), readiness);

    // The accepted action survives the failed advance.
    submit(&mut sim, b, Move::stay()).unwrap();
    sim.advance().unwrap();
    assert_eq!(position(&sim, a), GridPos::new(0, 1));
}

#[test]
fn r11_advance_with_no_agents_fails() {
    let mut sim = sim(10, 10, 0);
    sim.start().unwrap();
    assert_eq!(sim.advance(), Err(AdvanceError::NoAgents));
    assert_eq!(sim.readiness().state, StateId(0));
}

#[test]
fn r13_distance_zero_stays_and_ignores_direction() {
    let mut sim = sim(10, 10, 0);
    let agent = spawn_at(&mut sim, 0, 0);
    sim.start().unwrap();
    let stay_west = Move {
        direction: Direction::West,
        distance: 0,
    };
    submit(&mut sim, agent, stay_west).unwrap();
    sim.advance().unwrap();
    assert_eq!(position(&sim, agent), GridPos::new(0, 0));
}

#[test]
fn r13_moves_off_the_grid_fail_and_consume_the_turn() {
    let edges = [
        (GridPos::new(0, 0), Direction::West),
        (GridPos::new(0, 0), Direction::South),
        (GridPos::new(9, 9), Direction::East),
        (GridPos::new(9, 9), Direction::North),
    ];
    for (start, direction) in edges {
        let mut sim = sim(10, 10, 0);
        let agent = spawn_at(&mut sim, start.x, start.y);
        sim.start().unwrap();
        submit(&mut sim, agent, Move::step(direction)).unwrap();
        let observations = sim.advance().unwrap().observations;
        assert_eq!(position(&sim, agent), start, "{direction:?}");
        assert_eq!(observations.state, StateId(1));
    }
}

#[test]
fn r13_moves_into_blocking_terrain_fail_and_consume_the_turn() {
    // A wall to the east of (0, 0); the cell to the north is floor.
    let mut sim = sim_on(&["..", ".#"], 0);
    let agent = spawn_at(&mut sim, 0, 0);
    sim.start().unwrap();
    submit(&mut sim, agent, Move::step(Direction::East)).unwrap();
    let observations = sim.advance().unwrap().observations;
    assert_eq!(position(&sim, agent), GridPos::new(0, 0));
    assert_eq!(observations.state, StateId(1));
    submit(&mut sim, agent, Move::step(Direction::North)).unwrap();
    sim.advance().unwrap();
    assert_eq!(position(&sim, agent), GridPos::new(0, 1));
}

#[test]
fn r13_moves_into_a_stationary_agent_fail() {
    let mut sim = sim(10, 10, 0);
    let a = spawn_at(&mut sim, 0, 0);
    let b = spawn_at(&mut sim, 1, 0);
    sim.start().unwrap();
    submit(&mut sim, a, Move::step(Direction::East)).unwrap();
    submit(&mut sim, b, Move::stay()).unwrap();
    sim.advance().unwrap();
    assert_eq!(position(&sim, a), GridPos::new(0, 0));
    assert_eq!(position(&sim, b), GridPos::new(1, 0));
}

#[test]
fn r12_r14_contested_cell_goes_to_either_agent_by_shuffle_order() {
    // A at (0,0) and B at (2,0) both move into (1,0). The first in execution order wins.
    let mut winners = BTreeSet::new();
    for seed in 0..100 {
        let mut sim = sim(10, 10, seed);
        let a = spawn_at(&mut sim, 0, 0);
        let b = spawn_at(&mut sim, 2, 0);
        sim.start().unwrap();
        submit(&mut sim, a, Move::step(Direction::East)).unwrap();
        submit(&mut sim, b, Move::step(Direction::West)).unwrap();
        sim.advance().unwrap();

        let view = sim.view();
        assert_no_shared_cells(&view);
        let (pa, pb) = (position(&sim, a), position(&sim, b));
        let winner = match (pa, pb) {
            (p, q) if p == GridPos::new(1, 0) && q == GridPos::new(2, 0) => a,
            (p, q) if p == GridPos::new(0, 0) && q == GridPos::new(1, 0) => b,
            other => panic!("unexpected positions {other:?}"),
        };
        winners.insert(winner);
    }
    assert_eq!(winners, [AgentId(1), AgentId(2)].into_iter().collect());
}

#[test]
fn r12_r13_agent_can_follow_into_a_cell_vacated_earlier_in_the_step() {
    // A at (0,0) and B at (1,0) both move east. If B executes first, both move;
    // if A executes first, A is blocked and only B moves.
    let mut outcomes = BTreeSet::new();
    for seed in 0..100 {
        let mut sim = sim(10, 10, seed);
        let a = spawn_at(&mut sim, 0, 0);
        let b = spawn_at(&mut sim, 1, 0);
        sim.start().unwrap();
        submit(&mut sim, a, Move::step(Direction::East)).unwrap();
        submit(&mut sim, b, Move::step(Direction::East)).unwrap();
        sim.advance().unwrap();

        assert_eq!(position(&sim, b), GridPos::new(2, 0));
        let a_pos = position(&sim, a);
        assert!(a_pos == GridPos::new(0, 0) || a_pos == GridPos::new(1, 0));
        outcomes.insert(a_pos);
    }
    assert_eq!(outcomes.len(), 2, "both execution orders should occur");
}

#[test]
fn r12_submission_order_does_not_affect_execution_order() {
    // Same seed, opposite submission order: identical outcome.
    let run = |a_first: bool| {
        let mut sim = sim(10, 10, 3);
        let a = spawn_at(&mut sim, 0, 0);
        let b = spawn_at(&mut sim, 2, 0);
        sim.start().unwrap();
        let (first, second) = if a_first { (a, b) } else { (b, a) };
        let direction = |agent| {
            if agent == a {
                Direction::East
            } else {
                Direction::West
            }
        };
        submit(&mut sim, first, Move::step(direction(first))).unwrap();
        submit(&mut sim, second, Move::step(direction(second))).unwrap();
        sim.advance().unwrap();
        sim.view()
    };
    assert_eq!(run(true), run(false));
}

#[test]
fn r14_random_walks_never_share_cells() {
    let directions = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
    let mut sim = sim(4, 4, 11);
    for _ in 0..6 {
        sim.spawn(Placement::Random, &BARE).unwrap();
    }
    sim.start().unwrap();
    for step in 0..200usize {
        let agents: Vec<_> = sim.view().agents.iter().map(|a| a.id).collect();
        for (i, agent) in agents.into_iter().enumerate() {
            let direction = directions[(step + i * 3) % directions.len()];
            submit(&mut sim, agent, Move::step(direction)).unwrap();
        }
        sim.advance().unwrap();
        assert_no_shared_cells(&sim.view());
    }
}

#[test]
fn r15_advance_increments_state_and_clears_actions() {
    let mut sim = sim(10, 10, 0);
    let a = spawn_at(&mut sim, 0, 0);
    let b = spawn_at(&mut sim, 5, 5);
    sim.start().unwrap();
    for expected in 1..=3 {
        submit(&mut sim, a, Move::stay()).unwrap();
        submit(&mut sim, b, Move::stay()).unwrap();
        let observations = sim.advance().unwrap().observations;
        assert_eq!(observations.state, StateId(expected));
        assert_eq!(
            observations.by_agent.keys().copied().collect::<Vec<_>>(),
            [a, b]
        );
        let readiness = sim.readiness();
        assert_eq!(readiness.state, StateId(expected));
        assert_eq!(
            readiness.status,
            Status::Collecting {
                missing: vec![a, b]
            }
        );
    }
}

#[test]
fn r16_view_reports_state_dimensions_and_agents_in_id_order() {
    let mut sim = sim(10, 8, 0);
    let a = spawn_at(&mut sim, 4, 4);
    let b = spawn_at(&mut sim, 1, 7);
    let view = sim.view();
    assert_eq!((view.state, view.width, view.height), (StateId(0), 10, 8));
    assert_eq!(
        view.agents
            .iter()
            .map(|agent| (agent.id, agent.kind.as_str(), agent.position))
            .collect::<Vec<_>>(),
        [
            (a, builtin::AGENT, GridPos::new(4, 4)),
            (b, builtin::AGENT, GridPos::new(1, 7))
        ]
    );

    sim.start().unwrap();
    submit(&mut sim, a, Move::step(Direction::North)).unwrap();
    submit(&mut sim, b, Move::step(Direction::East)).unwrap();
    sim.advance().unwrap();
    let view = sim.view();
    assert_eq!(view.state, StateId(1));
    assert_eq!(view.agents[0].position, GridPos::new(4, 5));
    assert_eq!(view.agents[1].position, GridPos::new(2, 7));
}

/// Run a fixed script of random spawns and contested moves; return every intermediate view.
fn scripted_history(seed: u64) -> Vec<ViewState> {
    let mut sim = sim(5, 5, seed);
    for _ in 0..8 {
        sim.spawn(Placement::Random, &BARE).unwrap();
    }
    let mut history = vec![sim.view()];
    sim.start().unwrap();
    let directions = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
    for step in 0..50usize {
        for agent in 1..=8u64 {
            let direction = directions[(step + agent as usize) % 4];
            submit(&mut sim, AgentId(agent), Move::step(direction)).unwrap();
        }
        sim.advance().unwrap();
        history.push(sim.view());
    }
    history
}

#[test]
fn r17_same_seed_and_inputs_reproduce_identical_histories() {
    assert_eq!(scripted_history(42), scripted_history(42));
}

#[test]
fn r17_different_seeds_can_produce_different_histories() {
    assert_ne!(scripted_history(1), scripted_history(2));
}

#[test]
fn r18_removal_frees_the_cell_without_changing_the_state() {
    let mut sim = sim(3, 3, 0);
    let a = spawn_at(&mut sim, 1, 1);
    let b = spawn_at(&mut sim, 2, 2);

    assert_eq!(sim.remove(a), Ok(()));

    assert_eq!(sim.agent_count(), 1);
    assert_eq!(sim.readiness().state, StateId(0));
    assert_eq!(
        sim.view().agents.iter().map(|v| v.id).collect::<Vec<_>>(),
        [b]
    );
    // The cell is free, and the removed agent's ID is not reused.
    assert_eq!(
        sim.spawn(Placement::Cell(GridPos::new(1, 1)), &BARE),
        Ok(AgentId(3))
    );
}

#[test]
fn r18_removing_an_unknown_agent_fails() {
    let mut sim = sim(3, 3, 0);
    let a = spawn_at(&mut sim, 0, 0);
    sim.remove(a).unwrap();

    assert_eq!(sim.remove(a), Err(RemoveError::UnknownAgent(a)));
    assert_eq!(
        sim.remove(AgentId(99)),
        Err(RemoveError::UnknownAgent(AgentId(99)))
    );
}

#[test]
fn r18_removing_the_only_missing_agent_makes_the_run_ready() {
    let mut sim = sim(3, 3, 0);
    let a = spawn_at(&mut sim, 0, 0);
    let b = spawn_at(&mut sim, 2, 2);
    sim.start().unwrap();
    submit(&mut sim, a, Move::step(Direction::North)).unwrap();

    sim.remove(b).unwrap();

    assert!(sim.readiness().is_ready());
    let observations = sim.advance().unwrap().observations;
    assert_eq!(observations.state, StateId(1));
    assert_eq!(
        observations.by_agent.keys().copied().collect::<Vec<_>>(),
        [a]
    );
    assert_eq!(position(&sim, a), GridPos::new(0, 1));
}

#[test]
fn r18_removal_discards_an_accepted_action() {
    let mut sim = sim(3, 3, 0);
    let a = spawn_at(&mut sim, 0, 0);
    let b = spawn_at(&mut sim, 2, 2);
    sim.start().unwrap();
    submit(&mut sim, b, Move::step(Direction::West)).unwrap();

    sim.remove(b).unwrap();

    assert_eq!(
        sim.readiness().status,
        Status::Collecting { missing: vec![a] }
    );
}

#[test]
fn r18_removing_every_agent_leaves_a_started_empty_run() {
    let mut sim = sim(3, 3, 0);
    let a = spawn_at(&mut sim, 0, 0);
    sim.start().unwrap();

    sim.remove(a).unwrap();

    assert_eq!(sim.readiness().status, Status::StartedEmpty);
    assert_eq!(sim.readiness().state, StateId(0));
}

/// An agent with a metabolism.
fn with_metabolism(decay: f64, rate: f64) -> AgentDefinition {
    AgentDefinition {
        metabolism: Some(Metabolism { decay, rate }),
    }
}

fn spawn_with(sim: &mut Simulation, x: u32, y: u32, agent: &AgentDefinition) -> AgentId {
    sim.spawn(Placement::Cell(GridPos::new(x, y)), agent)
        .expect("spawn succeeds")
}

/// Submit each action, advance, and return the step's result.
fn step_all(sim: &mut Simulation, actions: &[(AgentId, Move)]) -> Advanced {
    for &(agent, action) in actions {
        submit(sim, agent, action).unwrap();
    }
    sim.advance().unwrap()
}

fn energy(advanced: &Advanced, agent: AgentId) -> Option<f64> {
    advanced.observations.by_agent[&agent].energy
}

#[test]
fn r22_an_agent_with_a_metabolism_starts_with_full_energy() {
    let mut sim = sim(3, 3, 0);
    let forager = spawn_with(&mut sim, 0, 0, &with_metabolism(0.25, 1.0));
    let bare = spawn_at(&mut sim, 2, 2);
    let observations = sim.start().unwrap();
    assert_eq!(observations.by_agent[&forager].energy, Some(1.0));
    assert_eq!(observations.by_agent[&bare], Observation::default());
    assert_eq!(observations.by_agent[&bare].energy, None);
}

#[test]
fn r22_an_agent_without_a_metabolism_never_starves() {
    let mut sim = sim(3, 3, 0);
    let bare = spawn_at(&mut sim, 1, 1);
    sim.start().unwrap();
    for _ in 0..300 {
        let advanced = step_all(&mut sim, &[(bare, Move::step(Direction::North))]);
        assert!(advanced.removed.is_empty());
        assert_eq!(energy(&advanced, bare), None);
    }
    assert_eq!(sim.agent_count(), 1);
}

#[test]
fn r23_only_a_successful_move_spends_effort() {
    // No decay, so energy falls only by effort times rate.
    let rate = 2.0;
    let mut sim = sim_on(&["..", ".#"], 0);
    let agent = spawn_with(&mut sim, 0, 0, &with_metabolism(0.0, rate));
    sim.start().unwrap();

    // Staying spends nothing.
    let advanced = step_all(&mut sim, &[(agent, Move::stay())]);
    assert_eq!(energy(&advanced, agent), Some(1.0));
    // A move into the wall to the east fails and spends nothing.
    let advanced = step_all(&mut sim, &[(agent, Move::step(Direction::East))]);
    assert_eq!(energy(&advanced, agent), Some(1.0));
    // A move off the grid fails and spends nothing.
    let advanced = step_all(&mut sim, &[(agent, Move::step(Direction::West))]);
    assert_eq!(energy(&advanced, agent), Some(1.0));
    // A successful move spends its effort for the cell moved.
    let advanced = step_all(&mut sim, &[(agent, Move::step(Direction::North))]);
    assert_eq!(energy(&advanced, agent), Some(1.0 - MOVE_EFFORT * rate));
    assert_eq!(position(&sim, agent), GridPos::new(0, 1));
}

#[test]
fn r23_a_move_blocked_by_another_agent_spends_nothing() {
    let mut sim = sim(2, 1, 0);
    let mover = spawn_with(&mut sim, 0, 0, &with_metabolism(0.0, 1.0));
    let blocker = spawn_at(&mut sim, 1, 0);
    sim.start().unwrap();
    let advanced = step_all(
        &mut sim,
        &[
            (mover, Move::step(Direction::East)),
            (blocker, Move::stay()),
        ],
    );
    assert_eq!(energy(&advanced, mover), Some(1.0));
}

#[test]
fn r24_energy_falls_by_decay_and_the_effort_of_the_step() {
    let (decay, rate) = (0.125, 4.0);
    let mut sim = sim(5, 5, 0);
    let agent = spawn_with(&mut sim, 0, 0, &with_metabolism(decay, rate));
    sim.start().unwrap();

    let moved = step_all(&mut sim, &[(agent, Move::step(Direction::North))]);
    let after_move = 1.0 - decay - MOVE_EFFORT * rate;
    assert_eq!(energy(&moved, agent), Some(after_move));
    // The exertion was cleared, so staying costs only the decay.
    let stayed = step_all(&mut sim, &[(agent, Move::stay())]);
    assert_eq!(energy(&stayed, agent), Some(after_move - decay));
}

#[test]
fn r24_r25_energy_stops_at_zero_and_the_agent_starves() {
    // 0.25 is exact in binary, so energy reaches exactly 0 on the fourth step.
    let mut sim = sim(3, 3, 0);
    let agent = spawn_with(&mut sim, 1, 1, &with_metabolism(0.25, 1.0));
    sim.start().unwrap();
    for expected in [0.75, 0.5, 0.25] {
        let advanced = step_all(&mut sim, &[(agent, Move::stay())]);
        assert_eq!(energy(&advanced, agent), Some(expected));
        assert!(advanced.removed.is_empty());
    }
    let advanced = step_all(&mut sim, &[(agent, Move::stay())]);
    assert_eq!(
        advanced.removed,
        [Removal {
            agent,
            reason: RemovalReason::Starved
        }]
    );
    assert!(advanced.observations.by_agent.is_empty());
    assert_eq!(advanced.observations.state, StateId(4));
    assert_eq!(sim.agent_count(), 0);
    assert_eq!(sim.readiness().status, Status::StartedEmpty);
    assert!(sim.view().agents.is_empty());
}

#[test]
fn r24_a_cost_larger_than_the_energy_left_stops_at_zero() {
    let mut sim = sim(3, 3, 0);
    let agent = spawn_with(&mut sim, 1, 1, &with_metabolism(0.75, 1.0));
    sim.start().unwrap();
    let advanced = step_all(&mut sim, &[(agent, Move::stay())]);
    assert_eq!(energy(&advanced, agent), Some(0.25));
    let advanced = step_all(&mut sim, &[(agent, Move::stay())]);
    assert_eq!(advanced.removed.len(), 1);
    assert_eq!(advanced.removed[0].agent, agent);
}

#[test]
fn r25_starved_agents_are_removed_in_id_order_and_free_their_cells() {
    let mut sim = sim(3, 1, 0);
    let starving = with_metabolism(1.0, 0.0);
    let a = spawn_with(&mut sim, 0, 0, &starving);
    let survivor = spawn_with(&mut sim, 1, 0, &with_metabolism(0.0, 0.0));
    let c = spawn_with(&mut sim, 2, 0, &starving);
    sim.start().unwrap();

    // The survivor's move west is blocked when it runs, because removal comes after actions.
    let advanced = step_all(
        &mut sim,
        &[
            (c, Move::stay()),
            (survivor, Move::step(Direction::West)),
            (a, Move::step(Direction::North)),
        ],
    );
    let removed: Vec<AgentId> = advanced.removed.iter().map(|r| r.agent).collect();
    assert_eq!(removed, [a, c]);
    assert!(
        advanced
            .removed
            .iter()
            .all(|r| r.reason == RemovalReason::Starved)
    );
    assert_eq!(
        advanced
            .observations
            .by_agent
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        [survivor]
    );
    assert_eq!(position(&sim, survivor), GridPos::new(1, 0));
    assert_eq!(
        sim.readiness().status,
        Status::Collecting {
            missing: vec![survivor]
        }
    );

    // Both freed cells can be entered.
    step_all(&mut sim, &[(survivor, Move::step(Direction::West))]);
    assert_eq!(position(&sim, survivor), GridPos::new(0, 0));
    step_all(&mut sim, &[(survivor, Move::step(Direction::East))]);
    step_all(&mut sim, &[(survivor, Move::step(Direction::East))]);
    assert_eq!(position(&sim, survivor), GridPos::new(2, 0));
}

#[test]
fn r25_a_starved_agent_no_longer_exists() {
    let mut sim = sim(3, 3, 0);
    let agent = spawn_with(&mut sim, 1, 1, &with_metabolism(1.0, 0.0));
    sim.start().unwrap();
    step_all(&mut sim, &[(agent, Move::stay())]);
    assert_eq!(
        submit(&mut sim, agent, Move::stay()),
        Err(SubmitError::UnknownAgent(agent))
    );
    assert_eq!(sim.remove(agent), Err(RemoveError::UnknownAgent(agent)));
    assert_eq!(sim.advance(), Err(AdvanceError::NoAgents));
}

#[test]
fn r26_energy_is_perceived_at_full_precision() {
    // 0.1 is not exact in binary; the observation carries the value the agent has.
    let mut sim = sim(3, 3, 0);
    let agent = spawn_with(&mut sim, 1, 1, &with_metabolism(0.1, 1.0));
    sim.start().unwrap();
    let mut expected = 1.0_f64;
    for _ in 0..3 {
        expected -= 0.1;
        let advanced = step_all(&mut sim, &[(agent, Move::stay())]);
        assert_eq!(energy(&advanced, agent), Some(expected));
    }
}

#[test]
fn r24_the_forager_starves_after_about_two_hundred_idle_steps_or_half_that_moving() {
    let forager = agora_env::bundled::agents().unwrap()[0].1;

    let mut idle = sim(3, 3, 0);
    let agent = spawn_with(&mut idle, 1, 1, &forager);
    idle.start().unwrap();
    let mut steps = 0;
    while idle.agent_count() > 0 {
        step_all(&mut idle, &[(agent, Move::stay())]);
        steps += 1;
    }
    // Rounding in the repeated subtraction can take one step more than the exact 200.
    assert!((200..=201).contains(&steps), "{steps}");

    let mut moving = sim(2, 1, 0);
    let agent = spawn_with(&mut moving, 0, 0, &forager);
    moving.start().unwrap();
    let mut steps = 0;
    let mut direction = Direction::East;
    while moving.agent_count() > 0 {
        step_all(&mut moving, &[(agent, Move::step(direction))]);
        direction = match direction {
            Direction::East => Direction::West,
            _ => Direction::East,
        };
        steps += 1;
    }
    assert!((100..=101).contains(&steps), "{steps}");
}
