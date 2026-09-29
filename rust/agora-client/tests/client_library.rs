//! Acceptance tests for SPEC-004 (Rust client library). Test names start with the requirement
//! ID they verify. Most tests use an in-process `agora-server`; tests of failures the real
//! server does not produce use a scripted fake server.

use std::net::SocketAddr;
use std::time::Duration;

use agora_client::protocol::{
    Action, ActionEntry, AgentId, CatalogEntryId, CloseReason, Direction, ErrorCode, ErrorResponse,
    PROTOCOL_VERSION, PacingMode, Placement, RunId, RunPhase, ServerMessage, SessionId, StateId,
};
use agora_client::{Client, ClientError, Observations, Session};
use agora_server::{EMPTY_GRID_10X10, ServerConfig, serve};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

/// Longest wait for anything a test expects.
const TIMEOUT: Duration = Duration::from_secs(5);

async fn start_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(serve(listener, ServerConfig::default()));
    format!("ws://{address}")
}

fn empty_grid() -> CatalogEntryId {
    CatalogEntryId::new(EMPTY_GRID_10X10).unwrap()
}

fn state(n: u64) -> StateId {
    StateId::new(n).unwrap()
}

fn stay(agent: AgentId) -> ActionEntry {
    ActionEntry {
        agent_id: agent,
        action: Action::Move {
            direction: Direction::North,
            distance: 0,
        },
    }
}

async fn new_run(url: &str) -> (Session, Observations) {
    Client::connect(url)
        .await
        .unwrap()
        .create_run(empty_grid())
        .await
        .unwrap()
}

async fn join(url: &str, run_id: &RunId) -> Session {
    let (session, _) = Client::connect(url)
        .await
        .unwrap()
        .join_run(run_id.clone())
        .await
        .unwrap();
    session
}

async fn next_observations(observations: &mut Observations) -> agora_client::ObservationBatch {
    tokio::time::timeout(TIMEOUT, observations.next())
        .await
        .expect("timed out waiting for observations")
        .expect("the observation stream ended")
}

/// A fake server on an unused port that answers `hello` with `welcome_reply`. If
/// `run_created` is set, it answers the next request with it and then closes the connection.
async fn fake_server(welcome_reply: ServerMessage, run_created: Option<ServerMessage>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        socket.next().await;
        let text = serde_json::to_string(&welcome_reply).unwrap();
        socket.send(Message::text(text)).await.unwrap();
        if let Some(reply) = run_created {
            socket.next().await;
            let text = serde_json::to_string(&reply).unwrap();
            socket.send(Message::text(text)).await.unwrap();
            // Wait for one more request, then drop the connection without answering it.
            socket.next().await;
        }
    });
    format!("ws://{address}")
}

#[tokio::test]
async fn r01_connecting_completes_the_handshake() {
    let url = start_server().await;

    let client = Client::connect(&url).await;

    assert!(client.is_ok());
}

#[tokio::test]
async fn r01_a_rejected_handshake_is_reported() {
    let url = fake_server(
        ServerMessage::Error(ErrorResponse {
            request_id: None,
            code: ErrorCode::UnsupportedProtocolVersion,
            message: "no".into(),
            details: None,
        }),
        None,
    )
    .await;

    let error = Client::connect(&url).await.err().unwrap();

    assert_eq!(error.code(), Some(&ErrorCode::UnsupportedProtocolVersion));
}

#[tokio::test]
async fn r02_creating_and_joining_report_the_session() {
    let url = start_server().await;
    let (creator, _) = new_run(&url).await;

    let joiner = join(&url, &creator.info().run_id).await;

    let (created, joined) = (creator.info(), joiner.info());
    assert_eq!(created.catalog_entry, empty_grid());
    assert_eq!(
        (created.state_id, created.phase),
        (state(0), RunPhase::Setup)
    );
    assert_eq!(joined.run_id, created.run_id);
    assert_ne!(joined.session_id, created.session_id);
}

#[tokio::test]
async fn r03_concurrent_requests_reach_the_server_in_number_order() {
    let url = start_server().await;
    let (session, _) = new_run(&url).await;

    // Spawns from many tasks at once, each with its own clone of the session. Out-of-order
    // request numbers would be rejected as stale.
    let tasks: Vec<_> = (0..20)
        .map(|_| {
            let session = session.clone();
            tokio::spawn(async move { session.spawn(Placement::Random).await })
        })
        .collect();

    for task in tasks {
        task.await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn r04_requests_return_results_and_rejections() {
    let url = start_server().await;
    let (session, _) = new_run(&url).await;

    let error = session.start().await.err().unwrap();
    assert_eq!(error.code(), Some(&ErrorCode::StartNotEligible));

    let agent = session.spawn(Placement::Cell { x: 3, y: 3 }).await.unwrap();
    let error = session
        .spawn(Placement::Cell { x: 3, y: 3 })
        .await
        .err()
        .unwrap();
    assert_eq!(error.code(), Some(&ErrorCode::CellOccupied));

    session.start().await.unwrap();
    let results = session.submit(state(0), vec![stay(agent)]).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].error, None);
}

#[tokio::test]
async fn r05_observations_arrive_in_order() {
    let url = start_server().await;
    let (session, mut observations) = new_run(&url).await;
    let agent = session.spawn(Placement::Random).await.unwrap();
    session.start().await.unwrap();

    // One task handles steps while the session is shared with another.
    let steps = tokio::spawn(async move {
        for n in 0..5 {
            let batch = next_observations(&mut observations).await;
            assert_eq!(batch.state_id, state(n));
            assert_eq!(batch.observations.len(), 1);
            assert_eq!(batch.observations[0].agent_id, agent);
            session.submit(state(n), vec![stay(agent)]).await.unwrap();
        }
    });
    steps.await.unwrap();
}

#[tokio::test]
async fn r06_view_and_pacing_handles_hold_the_newest_state() {
    let url = start_server().await;
    let (viewer, _) = new_run(&url).await;
    let player = join(&url, &viewer.info().run_id).await;
    let mut view = viewer.view();
    let pacing = viewer.pacing();
    assert!(view.borrow().is_none());
    assert!(pacing.borrow().is_none());

    viewer.watch().await.unwrap();
    assert_eq!(view.borrow_and_update().as_ref().unwrap().agents.len(), 0);
    assert!(!pacing.borrow().unwrap().you_control);

    player.spawn(Placement::Random).await.unwrap();
    player.spawn(Placement::Random).await.unwrap();
    tokio::time::timeout(TIMEOUT, async {
        while view.borrow_and_update().as_ref().unwrap().agents.len() < 2 {
            view.changed().await.unwrap();
        }
    })
    .await
    .expect("timed out waiting for the view");

    viewer.claim_pacing().await.unwrap();
    viewer.set_pacing(PacingMode::Paused).await.unwrap();
    let mut pacing = pacing;
    tokio::time::timeout(TIMEOUT, async {
        while pacing.borrow_and_update().unwrap().mode != PacingMode::Paused {
            pacing.changed().await.unwrap();
        }
    })
    .await
    .expect("timed out waiting for the pacing state");
    assert!(pacing.borrow().unwrap().you_control);
    viewer.step_once().await.unwrap();
}

#[tokio::test]
async fn r07_connection_loss_fails_requests_and_ends_observations() {
    let url = fake_server(
        ServerMessage::Welcome {
            protocol_version: PROTOCOL_VERSION,
        },
        Some(ServerMessage::RunCreated {
            request_id: agora_client::protocol::RequestId::new(1).unwrap(),
            run_id: RunId::new("run").unwrap(),
            session_id: SessionId::new("session").unwrap(),
            catalog_entry: empty_grid(),
            state_id: state(0),
            phase: RunPhase::Setup,
        }),
    )
    .await;
    let (session, mut observations) = new_run(&url).await;

    // The fake server closes the connection instead of answering.
    let error = session.spawn(Placement::Random).await.err().unwrap();
    assert!(matches!(error, ClientError::Closed), "{error:?}");

    let batch = tokio::time::timeout(TIMEOUT, observations.next())
        .await
        .expect("timed out waiting for the stream to end");
    assert_eq!(batch, None);
    let error = session.start().await.err().unwrap();
    assert!(matches!(error, ClientError::Closed), "{error:?}");
}

#[tokio::test]
async fn r09_leaving_ends_the_session_and_its_stream() {
    let url = start_server().await;
    let (session, mut observations) = new_run(&url).await;
    let other = join(&url, &session.info().run_id).await;

    other.leave().await.unwrap();

    let error = other.spawn(Placement::Random).await.err().unwrap();
    assert!(matches!(error, ClientError::Closed), "{error:?}");
    // The run is unaffected for the session that stayed.
    session.spawn(Placement::Random).await.unwrap();
    session.leave().await.unwrap();
    let ended = tokio::time::timeout(TIMEOUT, observations.next())
        .await
        .expect("timed out waiting for the stream to end");
    assert_eq!(ended, None);
}

#[tokio::test]
async fn r10_a_closed_run_reports_its_reason() {
    let url = start_server().await;
    let (creator, _) = new_run(&url).await;
    let (joiner, mut observations) = Client::connect(&url)
        .await
        .unwrap()
        .join_run(creator.info().run_id.clone())
        .await
        .unwrap();
    let error = joiner.close().await.err().unwrap();
    assert_eq!(error.code(), Some(&ErrorCode::NotCreator));

    creator.close().await.unwrap();

    let ended = tokio::time::timeout(TIMEOUT, observations.next())
        .await
        .expect("timed out waiting for the stream to end");
    assert_eq!(ended, None);
    assert_eq!(joiner.closed_reason(), Some(CloseReason::ClosedByCreator));
    let error = joiner.spawn(Placement::Random).await.err().unwrap();
    assert!(
        matches!(error, ClientError::RunClosed(CloseReason::ClosedByCreator)),
        "{error:?}"
    );
}
