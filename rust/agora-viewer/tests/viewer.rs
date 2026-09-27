//! Acceptance tests for SPEC-005 (viewer). Test names start with the requirement ID they
//! verify. The network bridge and a headless viewer app run against an in-process server; the
//! window and drawing are checked by hand.

use std::time::{Duration, Instant};

use agora_client::protocol::{
    Action, ActionEntry, Direction, ErrorCode, PacingMode, Placement, RunId, RunPhase, StateId,
};
use agora_client::{Client, Session};
use agora_server::{ServerConfig, serve};
use agora_viewer::ViewerCorePlugin;
use agora_viewer::network::{self, Command, Network, NetworkEvent, Target};
use agora_viewer::render::{AgentEntities, AgentMarker, Motion, cell_center};
use agora_viewer::state::{Connection, NetworkLink, ViewerState};
use bevy::prelude::*;
use tokio::net::TcpListener;

const TIMEOUT: Duration = Duration::from_secs(10);

async fn start_server(config: ServerConfig) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    tokio::spawn(serve(listener, config));
    url
}

async fn next_event(network: &mut Network) -> NetworkEvent {
    tokio::time::timeout(TIMEOUT, network.events.recv())
        .await
        .expect("timed out waiting for a network event")
        .expect("the network thread stopped")
}

/// What a bridge reported when it started watching.
struct Watching {
    run_id: RunId,
    creator: bool,
    mode: PacingMode,
    you_control: bool,
    network: Network,
}

/// Start a bridge and wait until it is watching.
async fn watching(url: &str, target: Target) -> Watching {
    let mut network = network::start(url.to_owned(), target);
    match next_event(&mut network).await {
        NetworkEvent::Watching {
            info,
            creator,
            view,
            pacing,
        } => {
            assert!(view.borrow().is_some(), "watching fills the view");
            let pacing = pacing.borrow().expect("watching fills the pacing state");
            Watching {
                run_id: info.run_id,
                creator,
                mode: pacing.mode,
                you_control: pacing.you_control,
                network,
            }
        }
        NetworkEvent::Disconnected { reason } => panic!("disconnected: {reason}"),
        NetworkEvent::CommandFailed { error, .. } => panic!("command failed: {error}"),
    }
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

/// Update `app` until `done` holds, or fail after the timeout.
async fn update_until(app: &mut App, mut done: impl FnMut(&mut App) -> bool) {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        app.update();
        if done(app) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for the viewer"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// A headless viewer app with its own bridge, updated until it is watching. Returns the app
/// and its run ID.
async fn headless(url: &str, target: Target) -> (App, RunId) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        ViewerCorePlugin::new(network::start(url.to_owned(), target)),
    ));
    update_until(&mut app, |app| {
        matches!(
            app.world().resource::<ViewerState>().connection,
            Connection::Watching(_)
        )
    })
    .await;
    let Connection::Watching(info) = &app.world().resource::<ViewerState>().connection else {
        unreachable!("the app is watching");
    };
    let run_id = info.run_id.clone();
    (app, run_id)
}

fn send(app: &App, command: Command) {
    app.world()
        .resource::<NetworkLink>()
        .0
        .commands
        .send(command)
        .unwrap();
}

fn phase(app: &App) -> Option<RunPhase> {
    let state = app.world().resource::<ViewerState>();
    state.view.as_ref().map(|view| view.phase)
}

fn agent_entities(app: &mut App) -> Vec<(AgentMarker, Motion)> {
    let mut query = app.world_mut().query::<(&AgentMarker, &Motion)>();
    let mut agents: Vec<_> = query.iter(app.world()).map(|(a, m)| (*a, *m)).collect();
    agents.sort_by_key(|(agent, _)| agent.0);
    agents
}

fn east(agent: agora_client::protocol::AgentId) -> ActionEntry {
    ActionEntry {
        agent_id: agent,
        action: Action::Move {
            direction: Direction::East,
            distance: 1,
        },
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn r01_creating_watches_the_new_run_and_claims_pacing() {
    let url = start_server(ServerConfig::default()).await;

    let created = watching(&url, Target::Create).await;

    assert!(created.creator);
    assert!(matches!(created.mode, PacingMode::Interval { .. }));
    assert!(created.you_control);
}

#[tokio::test(flavor = "multi_thread")]
async fn r01_joining_watches_an_existing_run_without_taking_held_control() {
    let url = start_server(ServerConfig::default()).await;
    let created = watching(&url, Target::Create).await;

    let joined = watching(&url, Target::Join(created.run_id.clone())).await;

    assert_eq!(joined.run_id, created.run_id);
    assert!(!joined.creator);
    assert!(!joined.you_control);
}

#[tokio::test(flavor = "multi_thread")]
async fn r01_an_unreachable_server_is_reported() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    drop(listener);
    let mut network = network::start(url, Target::Create);

    assert!(matches!(
        next_event(&mut network).await,
        NetworkEvent::Disconnected { .. }
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn r02_commands_reach_the_run_and_failures_are_reported() {
    let url = start_server(ServerConfig::default()).await;
    let (mut app, run_id) = headless(&url, Target::Create).await;
    let mut joiner = watching(&url, Target::Join(run_id.clone())).await.network;
    let player = join(&url, &run_id).await;
    player.spawn(Placement::Random).await.unwrap();

    joiner.commands.send(Command::Start).unwrap();
    match next_event(&mut joiner).await {
        NetworkEvent::CommandFailed { command, error } => {
            assert_eq!(command, Command::Start);
            assert!(error.contains(ErrorCode::NotCreator.as_str()), "{error}");
        }
        _ => panic!("expected the joiner's Start to fail"),
    }

    send(&app, Command::Start);
    update_until(&mut app, |app| phase(app) == Some(RunPhase::Started)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn r03_r04_agent_entities_follow_the_views() {
    let url = start_server(ServerConfig {
        session_expiry: Duration::from_millis(20),
        ..ServerConfig::default()
    })
    .await;
    let (mut app, run_id) = headless(&url, Target::Create).await;
    let stayer = join(&url, &run_id).await;
    let leaver = join(&url, &run_id).await;
    let staying = stayer.spawn(Placement::Cell { x: 2, y: 3 }).await.unwrap();
    let leaving = leaver.spawn(Placement::Cell { x: 7, y: 7 }).await.unwrap();

    // One entity per agent, placed at its cell.
    update_until(&mut app, |app| agent_entities(app).len() == 2).await;
    let agents = agent_entities(&mut app);
    assert_eq!(agents[0].0, AgentMarker(staying));
    assert_eq!(agents[0].1.to, cell_center(2, 3, (10, 10)));

    // A move retargets the agent's motion.
    send(&app, Command::SetPacing(PacingMode::Unlimited));
    send(&app, Command::Start);
    update_until(&mut app, |app| phase(app) == Some(RunPhase::Started)).await;
    let state = StateId::new(0).unwrap();
    stayer.submit(state, vec![east(staying)]).await.unwrap();
    leaver.submit(state, vec![east(leaving)]).await.unwrap();
    update_until(&mut app, |app| {
        agent_entities(app)
            .first()
            .is_some_and(|(_, motion)| motion.to == cell_center(3, 3, (10, 10)))
    })
    .await;

    // An agent removed from the run loses its entity.
    drop(leaver);
    update_until(&mut app, |app| agent_entities(app).len() == 1).await;
    assert_eq!(agent_entities(&mut app)[0].0, AgentMarker(staying));
    assert_eq!(app.world().resource::<AgentEntities>().0.len(), 1);
}

#[test]
fn r03_cells_are_centred_with_y_growing_north() {
    let size = (10, 10);
    let south_west = cell_center(0, 0, size);
    let north_east = cell_center(9, 9, size);
    assert_eq!(south_west, -north_east);
    assert!(north_east.x > 0.0 && north_east.y > 0.0);
    assert!(cell_center(0, 1, size).y > south_west.y);
}
