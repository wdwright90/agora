//! Acceptance tests for SPEC-003 (server sessions and runs). Test names start with the
//! requirement ID they verify. Timer tests use short timeouts and generous waits.

mod common;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use agora_protocol::{Direction, ErrorCode, Placement, RunPhase, ServerMessage};
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

    creator.close().await;
    tokio::time::sleep(SETTLE).await;

    // The creator's session has expired and the release timer is running.
    let mut rescuer = Client::connect_with_hello(address).await;
    let reply = rescuer.join_run(&run_id).await;
    assert!(
        matches!(reply, ServerMessage::RunJoined { .. }),
        "{reply:?}"
    );
    // The expired session's agent was removed, freeing its cell.
    expect_spawned(rescuer.spawn(cell(4, 4)).await);
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
