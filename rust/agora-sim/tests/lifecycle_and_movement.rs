//! Acceptance tests for SPEC-001 (simulation lifecycle and movement).
//! Test names start with the requirement ID they verify.

use std::collections::BTreeSet;

use agora_sim::{
    AdvanceError, AgentId, ConfigError, Direction, GridPos, Move, Placement, SimConfig, Simulation,
    SpawnError, StartError, StateId, Status, Submission, SubmitError, ViewState,
};

fn sim(width: u32, height: u32, seed: u64) -> Simulation {
    Simulation::new(SimConfig {
        width,
        height,
        seed,
    })
    .expect("valid config")
}

fn spawn_at(sim: &mut Simulation, x: u32, y: u32) -> AgentId {
    sim.spawn(Placement::Cell(GridPos::new(x, y)))
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
fn r01_zero_dimensions_are_rejected() {
    for (width, height) in [(0, 10), (10, 0), (0, 0)] {
        let result = Simulation::new(SimConfig {
            width,
            height,
            seed: 0,
        });
        assert!(matches!(result, Err(ConfigError::EmptyGrid { .. })));
    }
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
            sim.spawn(Placement::Cell(pos)),
            Err(SpawnError::OutOfBounds(pos))
        );
    }
    let occupied = GridPos::new(3, 3);
    assert_eq!(
        sim.spawn(Placement::Cell(occupied)),
        Err(SpawnError::Occupied(occupied))
    );
    assert_eq!(sim.view(), before);
}

#[test]
fn r04_random_spawns_fill_every_free_cell_then_fail() {
    let mut sim = sim(3, 3, 7);
    spawn_at(&mut sim, 1, 1);
    for _ in 0..8 {
        sim.spawn(Placement::Random).expect("a free cell exists");
    }
    let view = sim.view();
    assert_eq!(view.agents.len(), 9);
    assert_no_shared_cells(&view);
    assert_eq!(sim.spawn(Placement::Random), Err(SpawnError::NoFreeCell));
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
        let agent = sim.spawn(Placement::Random).unwrap();
        seen.insert(position(&sim, agent));
    }
    let expected: BTreeSet<_> = [GridPos::new(1, 0), GridPos::new(0, 1), GridPos::new(1, 1)]
        .into_iter()
        .collect();
    assert_eq!(seen, expected);
}

#[test]
fn r05_ids_are_sequential_and_failed_spawns_do_not_consume_ids() {
    let mut sim = sim(10, 10, 0);
    assert_eq!(spawn_at(&mut sim, 0, 0), AgentId(1));
    assert!(sim.spawn(Placement::Cell(GridPos::new(0, 0))).is_err());
    assert_eq!(spawn_at(&mut sim, 1, 0), AgentId(2));
    assert_eq!(sim.spawn(Placement::Random), Ok(AgentId(3)));
}

#[test]
fn r06_spawn_after_start_is_rejected() {
    let mut sim = sim(10, 10, 0);
    spawn_at(&mut sim, 0, 0);
    sim.start().unwrap();
    assert_eq!(sim.spawn(Placement::Random), Err(SpawnError::NotInSetup));
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
        Err(SubmitError::DistanceExceedsBudget {
            distance: 2,
            budget: 1,
        })
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
        let observations = sim.advance().unwrap();
        assert_eq!(position(&sim, agent), start, "{direction:?}");
        assert_eq!(observations.state, StateId(1));
    }
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
        sim.spawn(Placement::Random).unwrap();
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
        let observations = sim.advance().unwrap();
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
            .map(|agent| (agent.id, agent.position))
            .collect::<Vec<_>>(),
        [(a, GridPos::new(4, 4)), (b, GridPos::new(1, 7))]
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
        sim.spawn(Placement::Random).unwrap();
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
