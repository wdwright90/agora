//! Acceptance tests for SPEC-003 (server sessions and runs). Test names start with the
//! requirement ID they verify. Timer tests use short timeouts and generous waits.

mod common;

use std::collections::HashSet;
use std::time::Duration;

use agora_protocol::{ErrorCode, Placement, RunPhase, ServerMessage};
use agora_server::ServerConfig;
use common::{Client, cell, expect_code, start_default_server, start_server};

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
    // The expired session's agent is still in the world.
    expect_code(rescuer.spawn(cell(4, 4)).await, ErrorCode::CellOccupied);
}

#[tokio::test]
async fn r07_a_connected_session_keeps_its_run() {
    let address = start_server(ServerConfig {
        session_expiry: SHORT,
        run_release: SHORT,
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
