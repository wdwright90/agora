//! Acceptance tests for SPEC-003 (server sessions and runs). Test names start with the
//! requirement ID they verify. Timer tests use short timeouts and generous waits.

mod common;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use agora_protocol::{
    CloseReason, Direction, ErrorCode, IntervalMs, Pacing, PacingMode, Placement, RunPhase,
    ServerMessage,
};
use agora_server::ServerConfig;
use common::{
    Client, cell, expect_code, expect_spawned, expect_submitted, start_default_server,
    start_server, state, stay, step,
};

/// A timeout short enough for tests to wait out.
const SHORT: Duration = Duration::from_millis(20);
/// A timeout no test waits out.
const LONG: Duration = Duration::from_secs(60);
/// Long enough for the server to notice a disconnect and for every short timeout to fire.
const SETTLE: Duration = Duration::from_millis(400);

#[tokio::test]
async fn r01_the_bundled_entry_is_an_empty_10_by_10_grid() {
    let address = start_default_server().await;
    let (mut client, _) = Client::with_new_run(address).await;

    for placement in [cell(0, 0), cell(9, 0), cell(0, 9), cell(9, 9)] {
        let reply = client.spawn(placement).await;
        assert!(matches!(reply, ServerMessage::Spawned { .. }), "{reply:?}");
    }
    for placement in [cell(10, 0), cell(0, 10)] {
        expect_code(client.spawn(placement).await, ErrorCode::CellOutOfBounds);
    }
}

#[tokio::test]
async fn r02_run_and_session_ids_are_unique() {
    let address = start_default_server().await;
    let mut runs = HashSet::new();
    let mut sessions = HashSet::new();

    for _ in 0..3 {
        let mut creator = Client::connect_with_hello(address).await;
        let request_id = creator.next_id();
        let ServerMessage::RunCreated {
            run_id, session_id, ..
        } = creator
            .exchange(&agora_protocol::ClientMessage::CreateRun {
                request_id,
                catalog_entry: common::empty_grid(),
            })
            .await
        else {
            panic!("expected run_created");
        };
        let mut joiner = Client::connect_with_hello(address).await;
        let ServerMessage::RunJoined {
            session_id: joined, ..
        } = joiner.join_run(&run_id).await
        else {
            panic!("expected run_joined");
        };
        assert!(runs.insert(run_id));
        assert!(sessions.insert(session_id));
        assert!(sessions.insert(joined));
    }
}

#[tokio::test]
async fn r03_only_the_creator_can_start_the_run() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut joiner = Client::joined(address, &run_id).await;
    joiner.spawn(Placement::Random).await;

    expect_code(joiner.start().await, ErrorCode::NotCreator);
    let reply = creator.start().await;

    assert!(matches!(reply, ServerMessage::Started { .. }), "{reply:?}");
}

#[tokio::test]
async fn r04_start_without_agents_is_rejected_and_the_run_stays_in_setup() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;

    expect_code(creator.start().await, ErrorCode::StartNotEligible);

    let mut observer = Client::connect_with_hello(address).await;
    let reply = observer.join_run(&run_id).await;
    assert!(
        matches!(
            reply,
            ServerMessage::RunJoined {
                phase: RunPhase::Setup,
                ..
            }
        ),
        "{reply:?}"
    );
    creator.spawn(Placement::Random).await;
    let reply = creator.start().await;
    assert!(matches!(reply, ServerMessage::Started { .. }), "{reply:?}");
}

#[tokio::test]
async fn r04_a_run_starts_only_once() {
    let address = start_default_server().await;
    let (mut creator, _) = Client::with_new_run(address).await;
    creator.spawn(Placement::Random).await;
    creator.start().await;

    expect_code(creator.start().await, ErrorCode::RunAlreadyStarted);
}

#[tokio::test]
async fn r05_spawning_after_start_is_rejected() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    creator.spawn(Placement::Random).await;
    creator.start().await;
    let mut joiner = Client::joined(address, &run_id).await;

    expect_code(
        joiner.spawn(Placement::Random).await,
        ErrorCode::RunAlreadyStarted,
    );
}

#[tokio::test]
async fn r06_a_disconnected_session_keeps_its_run_until_it_expires() {
    let address = start_server(ServerConfig {
        session_expiry: LONG,
        run_release: SHORT,
        ..ServerConfig::default()
    })
    .await;
    let (creator, run_id) = Client::with_new_run(address).await;

    creator.close().await;
    tokio::time::sleep(SETTLE).await;

    // The creator's session has not expired, so the run's release timer never started.
    let mut joiner = Client::connect_with_hello(address).await;
    let reply = joiner.join_run(&run_id).await;
    assert!(
        matches!(reply, ServerMessage::RunJoined { .. }),
        "{reply:?}"
    );
}

#[tokio::test]
async fn r07_a_run_whose_sessions_have_all_expired_is_released() {
    let address = start_server(ServerConfig {
        session_expiry: SHORT,
        run_release: SHORT,
        ..ServerConfig::default()
    })
    .await;
    let (creator, run_id) = Client::with_new_run(address).await;

    creator.close().await;
    tokio::time::sleep(SETTLE).await;

    let mut late = Client::connect_with_hello(address).await;
    expect_code(late.join_run(&run_id).await, ErrorCode::UnknownRun);
}

#[tokio::test]
async fn r07_joining_before_release_keeps_the_run() {
    let address = start_server(ServerConfig {
        session_expiry: SHORT,
        run_release: LONG,
        ..ServerConfig::default()
    })
    .await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    creator.spawn(cell(4, 4)).await;
    // Started, so the creator's expiry does not close the run (R06).
    creator.start().await;

    creator.close().await;
    tokio::time::sleep(SETTLE).await;

    // The creator's session has expired and the release timer is running.
    let mut rescuer = Client::connect_with_hello(address).await;
    let reply = rescuer.join_run(&run_id).await;
    assert!(
        matches!(reply, ServerMessage::RunJoined { .. }),
        "{reply:?}"
    );
    // The expired session's agent was removed.
    assert!(rescuer.watch().await.agents.is_empty());
}

#[tokio::test]
async fn r07_a_connected_session_keeps_its_run() {
    let address = start_server(ServerConfig {
        session_expiry: SHORT,
        run_release: SHORT,
        ..ServerConfig::default()
    })
    .await;
    let (_creator, run_id) = Client::with_new_run(address).await;

    tokio::time::sleep(SETTLE).await;

    let mut joiner = Client::connect_with_hello(address).await;
    let reply = joiner.join_run(&run_id).await;
    assert!(
        matches!(reply, ServerMessage::RunJoined { .. }),
        "{reply:?}"
    );
}

#[tokio::test]
async fn r03_only_an_agents_owner_can_submit_for_it() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut joiner = Client::joined(address, &run_id).await;
    let mine = expect_spawned(creator.spawn(Placement::Random).await);
    creator.start().await;

    let results = expect_submitted(joiner.submit(state(0), vec![stay(mine)]).await);

    let error = results[0].error.as_ref().expect("the entry is rejected");
    assert_eq!(error.code, ErrorCode::AgentNotOwned);
    // The owner's action is still accepted.
    let results = expect_submitted(creator.submit(state(0), vec![stay(mine)]).await);
    assert_eq!(results[0].error, None);
}

#[tokio::test]
async fn r08_r09_a_step_executes_once_every_agent_has_an_action() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut joiner = Client::joined(address, &run_id).await;
    let mut observer = Client::joined(address, &run_id).await;
    let first = expect_spawned(creator.spawn(cell(0, 0)).await);
    let second = expect_spawned(joiner.spawn(cell(5, 5)).await);
    creator.start().await;
    assert_eq!(creator.observations().await, (state(0), vec![first]));
    assert_eq!(joiner.observations().await, (state(0), vec![second]));

    creator
        .submit(state(0), vec![step(first, Direction::North, 1)])
        .await;
    // A rejected entry does not count as an action.
    joiner
        .submit(state(0), vec![step(second, Direction::East, 2)])
        .await;
    creator.expect_silence(SETTLE).await;

    joiner
        .submit(state(0), vec![step(second, Direction::East, 1)])
        .await;

    // Each session receives only its own agents' observations.
    assert_eq!(creator.observations().await, (state(1), vec![first]));
    assert_eq!(joiner.observations().await, (state(1), vec![second]));
    observer.expect_silence(SETTLE).await;
    // Collection now targets state 1.
    let results = expect_submitted(creator.submit(state(1), vec![stay(first)]).await);
    assert_eq!(results[0].error, None);
}

#[tokio::test]
async fn r06_an_expired_sessions_agents_are_removed_and_the_run_continues() {
    let address = start_server(ServerConfig {
        session_expiry: SHORT,
        run_release: LONG,
        ..ServerConfig::default()
    })
    .await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut joiner = Client::joined(address, &run_id).await;
    expect_spawned(creator.spawn(cell(0, 0)).await);
    let survivor = expect_spawned(joiner.spawn(cell(5, 5)).await);
    creator.start().await;
    joiner.observations().await;
    joiner.submit(state(0), vec![stay(survivor)]).await;

    // The creator's agent never acts. Once its session expires, the agent is removed and the
    // step executes without it.
    creator.close().await;

    assert_eq!(joiner.observations().await, (state(1), vec![survivor]));
}

#[tokio::test]
async fn r07_a_run_waiting_for_actions_is_released() {
    let address = start_server(ServerConfig {
        session_expiry: SHORT,
        run_release: SHORT,
        ..ServerConfig::default()
    })
    .await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let acted = expect_spawned(creator.spawn(Placement::Random).await);
    expect_spawned(creator.spawn(Placement::Random).await);
    creator.start().await;
    creator.submit(state(0), vec![stay(acted)]).await;

    creator.close().await;
    tokio::time::sleep(SETTLE).await;

    let mut late = Client::connect_with_hello(address).await;
    expect_code(late.join_run(&run_id).await, ErrorCode::UnknownRun);
}

#[tokio::test]
async fn r04_a_viewer_makes_a_run_without_agents_eligible() {
    let address = start_default_server().await;
    let (mut creator, _) = Client::with_new_run(address).await;
    creator.watch().await;

    let reply = creator.start().await;

    assert!(matches!(reply, ServerMessage::Started { .. }), "{reply:?}");
}

#[tokio::test]
async fn r04_a_disconnected_viewer_does_not_count() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut viewer = Client::joined(address, &run_id).await;
    viewer.watch().await;

    viewer.close().await;
    tokio::time::sleep(SETTLE).await;

    expect_code(creator.start().await, ErrorCode::StartNotEligible);
}

/// Start a run with one agent owned by `player`, execute the first step, and return the agent.
async fn first_step(player: &mut Client) -> agora_protocol::AgentId {
    let agent = expect_spawned(player.spawn(Placement::Random).await);
    player.start().await;
    player.observations().await;
    player.submit(state(0), vec![stay(agent)]).await;
    assert_eq!(player.observations().await, (state(1), vec![agent]));
    agent
}

#[tokio::test]
async fn r11_a_viewer_holds_steps_to_the_step_interval() {
    let address = start_server(ServerConfig {
        step_interval: LONG,
        ..ServerConfig::default()
    })
    .await;
    let (mut player, run_id) = Client::with_new_run(address).await;
    let mut viewer = Client::joined(address, &run_id).await;
    viewer.watch().await;
    let agent = first_step(&mut player).await;

    player.submit(state(1), vec![stay(agent)]).await;
    player.expect_silence(SETTLE).await;

    // The last viewer leaving lifts the interval, and the waiting step executes.
    viewer.close().await;
    assert_eq!(player.observations().await, (state(2), vec![agent]));
}

#[tokio::test]
async fn r11_a_held_step_executes_when_the_interval_passes() {
    let address = start_server(ServerConfig {
        step_interval: Duration::from_millis(100),
        ..ServerConfig::default()
    })
    .await;
    let (mut player, run_id) = Client::with_new_run(address).await;
    let mut viewer = Client::joined(address, &run_id).await;
    viewer.watch().await;
    let agent = first_step(&mut player).await;
    let after_first = Instant::now();

    player.submit(state(1), vec![stay(agent)]).await;

    assert_eq!(player.observations().await, (state(2), vec![agent]));
    assert!(after_first.elapsed() >= Duration::from_millis(50));
}

#[tokio::test]
async fn r11_without_viewers_steps_are_not_held() {
    let address = start_server(ServerConfig {
        step_interval: LONG,
        ..ServerConfig::default()
    })
    .await;
    let (mut player, _) = Client::with_new_run(address).await;
    let agent = first_step(&mut player).await;

    player.submit(state(1), vec![stay(agent)]).await;

    assert_eq!(player.observations().await, (state(2), vec![agent]));
}

fn interval(ms: u64) -> PacingMode {
    PacingMode::Interval {
        ms: IntervalMs::new(ms).unwrap(),
    }
}

#[tokio::test]
async fn r12_the_first_viewer_resets_pacing_to_the_default_interval() {
    let address = start_server(ServerConfig {
        step_interval: Duration::from_millis(250),
        ..ServerConfig::default()
    })
    .await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut first = Client::joined(address, &run_id).await;
    first.watch_and_control().await;
    first.set_pacing(PacingMode::Unlimited).await;

    // The last viewer leaves, and pacing becomes unlimited until another viewer arrives.
    first.close().await;
    tokio::time::sleep(SETTLE).await;
    let (_, pacing) = creator.watch_with_pacing().await;

    assert_eq!(pacing.mode, interval(250));
    assert!(!pacing.you_control);
}

#[tokio::test]
async fn r13_control_passes_to_the_oldest_remaining_viewer() {
    let address = start_default_server().await;
    let (mut controller, run_id) = Client::with_new_run(address).await;
    // `older` connects before `newer`, but watches after it.
    let mut older = Client::joined(address, &run_id).await;
    let mut newer = Client::joined(address, &run_id).await;
    controller.watch_and_control().await;
    newer.watch().await;
    older.watch().await;
    controller.set_pacing(PacingMode::Paused).await;
    older.pacing_update().await;
    newer.pacing_update().await;

    controller.close().await;

    let pacing = older.pacing_update().await;
    assert_eq!(
        pacing,
        Pacing {
            mode: PacingMode::Paused,
            you_control: true
        }
    );
    let pacing = newer.pacing_update().await;
    assert!(!pacing.you_control);
    expect_code(newer.claim_pacing().await, ErrorCode::PacingControlHeld);
}

#[tokio::test]
async fn r14_a_paused_run_steps_only_when_a_step_is_granted() {
    let address = start_default_server().await;
    let (mut viewer, run_id) = Client::with_new_run(address).await;
    let mut player = Client::joined(address, &run_id).await;
    viewer.watch_and_control().await;
    let agent = expect_spawned(player.spawn(Placement::Random).await);
    // Pausing during setup carries through Start.
    viewer.set_pacing(PacingMode::Paused).await;
    viewer.start().await;
    assert_eq!(player.observations().await, (state(0), vec![agent]));

    player.submit(state(0), vec![stay(agent)]).await;
    player.expect_silence(SETTLE).await;

    viewer.step_once().await;
    assert_eq!(player.observations().await, (state(1), vec![agent]));

    // The run stays paused after the granted step.
    player.submit(state(1), vec![stay(agent)]).await;
    player.expect_silence(SETTLE).await;

    viewer.set_pacing(PacingMode::Unlimited).await;
    assert_eq!(player.observations().await, (state(2), vec![agent]));
}

#[tokio::test]
async fn r14_granted_steps_do_not_accumulate() {
    let address = start_default_server().await;
    let (mut viewer, run_id) = Client::with_new_run(address).await;
    let mut player = Client::joined(address, &run_id).await;
    viewer.watch_and_control().await;
    let agent = expect_spawned(player.spawn(Placement::Random).await);
    viewer.set_pacing(PacingMode::Paused).await;
    viewer.start().await;
    player.observations().await;

    // Two grants before the agent is ready allow only one step.
    viewer.step_once().await;
    viewer.step_once().await;
    player.submit(state(0), vec![stay(agent)]).await;
    assert_eq!(player.observations().await, (state(1), vec![agent]));

    player.submit(state(1), vec![stay(agent)]).await;
    player.expect_silence(SETTLE).await;
}

#[tokio::test]
async fn r14_an_interval_change_applies_to_a_held_step() {
    let address = start_default_server().await;
    let (mut player, run_id) = Client::with_new_run(address).await;
    let mut viewer = Client::joined(address, &run_id).await;
    viewer.watch_and_control().await;
    viewer.set_pacing(interval(3_600_000)).await;
    let agent = first_step(&mut player).await;

    player.submit(state(1), vec![stay(agent)]).await;
    player.expect_silence(SETTLE).await;

    // The previous step started long enough ago for the new interval.
    viewer.set_pacing(interval(1)).await;
    assert_eq!(player.observations().await, (state(2), vec![agent]));
}

#[tokio::test]
async fn r15_leaving_removes_agents_and_a_waiting_step_executes() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut leaver = Client::joined(address, &run_id).await;
    let staying = expect_spawned(creator.spawn(cell(0, 0)).await);
    expect_spawned(leaver.spawn(cell(5, 5)).await);
    creator.start().await;
    creator.observations().await;
    creator.submit(state(0), vec![stay(staying)]).await;

    leaver.leave_run().await;

    assert_eq!(creator.observations().await, (state(1), vec![staying]));
}

#[tokio::test]
async fn r15_the_last_session_leaving_releases_the_run_at_once() {
    let address = start_server(ServerConfig {
        run_release: LONG,
        ..ServerConfig::default()
    })
    .await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    creator.spawn(Placement::Random).await;
    creator.start().await;

    creator.leave_run().await;

    let mut late = Client::connect_with_hello(address).await;
    expect_code(late.join_run(&run_id).await, ErrorCode::UnknownRun);
}

#[tokio::test]
async fn r15_a_creator_leaving_setup_closes_the_run() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut joiner = Client::joined(address, &run_id).await;

    creator.leave_run().await;

    assert_eq!(
        joiner.receive().await,
        ServerMessage::RunClosed {
            reason: CloseReason::CreatorLeft
        }
    );
    let mut late = Client::connect_with_hello(address).await;
    expect_code(late.join_run(&run_id).await, ErrorCode::UnknownRun);
}

#[tokio::test]
async fn r15_a_controller_leaving_hands_over_pacing_control() {
    let address = start_default_server().await;
    let (_creator, run_id) = Client::with_new_run(address).await;
    let mut controller = Client::joined(address, &run_id).await;
    let mut successor = Client::joined(address, &run_id).await;
    controller.watch_and_control().await;
    successor.watch().await;

    controller.leave_run().await;

    assert!(successor.pacing_update().await.you_control);
}

#[tokio::test]
async fn r16_closing_notifies_every_session_and_releases_the_run() {
    let address = start_default_server().await;
    let (mut creator, run_id) = Client::with_new_run(address).await;
    let mut first = Client::joined(address, &run_id).await;
    let mut second = Client::joined(address, &run_id).await;
    creator.spawn(Placement::Random).await;
    creator.start().await;

    creator.close_run().await;

    for client in [&mut first, &mut second] {
        assert_eq!(
            client.receive().await,
            ServerMessage::RunClosed {
                reason: CloseReason::ClosedByCreator
            }
        );
    }
    let mut late = Client::connect_with_hello(address).await;
    expect_code(late.join_run(&run_id).await, ErrorCode::UnknownRun);
}

#[tokio::test]
async fn r06_a_creator_expiring_in_setup_closes_the_run() {
    let address = start_server(ServerConfig {
        session_expiry: SHORT,
        run_release: LONG,
        ..ServerConfig::default()
    })
    .await;
    let (creator, run_id) = Client::with_new_run(address).await;
    let mut joiner = Client::joined(address, &run_id).await;

    creator.close().await;

    assert_eq!(
        joiner.receive().await,
        ServerMessage::RunClosed {
            reason: CloseReason::CreatorExpired
        }
    );
    let mut late = Client::connect_with_hello(address).await;
    expect_code(late.join_run(&run_id).await, ErrorCode::UnknownRun);
}
