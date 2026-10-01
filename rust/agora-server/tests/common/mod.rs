//! A test server and a minimal WebSocket client for the acceptance tests.

#![allow(dead_code, reason = "each test file uses a different subset")]

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Duration;

use agora_protocol::{
    Action, ActionEntry, AgentId, CatalogEntryId, ClientMessage, Direction, EntryResult, ErrorCode,
    ErrorResponse, PROTOCOL_VERSION, Pacing, PacingMode, Placement, RequestId, RunId,
    ServerMessage, StateId, View, Watching,
};
use agora_server::{DIVIDED_10X10, EMPTY_GRID_10X10, ServerConfig, serve};
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

/// Longest wait for a server message before a test fails.
const RECEIVE_TIMEOUT: Duration = Duration::from_secs(5);

/// Start a server on an unused local port and return its address.
pub async fn start_server(config: ServerConfig) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(serve(listener, config));
    address
}

pub async fn start_default_server() -> SocketAddr {
    start_server(ServerConfig::default()).await
}

pub fn request_id(n: u64) -> RequestId {
    RequestId::new(n).unwrap()
}

pub fn empty_grid() -> CatalogEntryId {
    CatalogEntryId::new(EMPTY_GRID_10X10).unwrap()
}

/// A 10 × 10 grid with a wall at x = 5, except at y = 4 and y = 5.
pub fn divided() -> CatalogEntryId {
    CatalogEntryId::new(DIVIDED_10X10).unwrap()
}

pub fn cell(x: u32, y: u32) -> Placement {
    Placement::Cell { x, y }
}

pub fn agent(n: u64) -> AgentId {
    AgentId::new(n).unwrap()
}

pub fn state(n: u64) -> StateId {
    StateId::new(n).unwrap()
}

/// An action entry moving `agent` by `distance` in `direction`.
pub fn step(agent: AgentId, direction: Direction, distance: u32) -> ActionEntry {
    ActionEntry {
        agent_id: agent,
        action: Action::Move {
            direction,
            distance,
        },
    }
}

/// An action entry keeping `agent` in place.
pub fn stay(agent: AgentId) -> ActionEntry {
    step(agent, Direction::North, 0)
}

/// The entry results of a `submitted` response, failing the test on any other message.
pub fn expect_submitted(message: ServerMessage) -> Vec<EntryResult> {
    match message {
        ServerMessage::Submitted { results, .. } => results,
        other => panic!("expected submitted, got {other:?}"),
    }
}

/// The spawned agent's ID, failing the test on any other message.
pub fn expect_spawned(message: ServerMessage) -> AgentId {
    match message {
        ServerMessage::Spawned { agent_id, .. } => agent_id,
        other => panic!("expected spawned, got {other:?}"),
    }
}

/// Unwrap an error response, failing the test on any other message.
pub fn expect_error(message: ServerMessage) -> ErrorResponse {
    match message {
        ServerMessage::Error(error) => error,
        other => panic!("expected an error, got {other:?}"),
    }
}

pub fn expect_code(message: ServerMessage, code: ErrorCode) -> ErrorResponse {
    let error = expect_error(message);
    assert_eq!(error.code, code, "{error:?}");
    error
}

pub struct Client {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_request: u64,
    /// Pushed `observations` messages not yet taken by [`Client::observations`].
    observations: VecDeque<ServerMessage>,
    /// Pushed views not yet taken by [`Client::view_update`].
    views: VecDeque<View>,
    /// Pushed pacing states not yet taken by [`Client::pacing_update`].
    pacings: VecDeque<Pacing>,
}

impl Client {
    /// Connect without a handshake.
    pub async fn connect(address: SocketAddr) -> Self {
        let (socket, _) = connect_async(format!("ws://{address}")).await.unwrap();
        Self {
            socket,
            next_request: 1,
            observations: VecDeque::new(),
            views: VecDeque::new(),
            pacings: VecDeque::new(),
        }
    }

    /// Connect and complete the version handshake.
    pub async fn connect_with_hello(address: SocketAddr) -> Self {
        let mut client = Self::connect(address).await;
        let reply = client
            .exchange(&ClientMessage::Hello {
                protocol_version: PROTOCOL_VERSION,
            })
            .await;
        assert_eq!(
            reply,
            ServerMessage::Welcome {
                protocol_version: PROTOCOL_VERSION
            }
        );
        client
    }

    /// Connect, complete the handshake, and create a run on the empty-grid catalog entry.
    pub async fn with_new_run(address: SocketAddr) -> (Self, RunId) {
        Self::with_new_run_from(address, empty_grid()).await
    }

    /// Connect, complete the handshake, and create a run on `catalog_entry`.
    pub async fn with_new_run_from(
        address: SocketAddr,
        catalog_entry: CatalogEntryId,
    ) -> (Self, RunId) {
        let mut client = Self::connect_with_hello(address).await;
        let run_id = client.create_run_from(catalog_entry).await;
        (client, run_id)
    }

    /// Connect, complete the handshake, and join `run_id`.
    pub async fn joined(address: SocketAddr, run_id: &RunId) -> Self {
        let mut client = Self::connect_with_hello(address).await;
        let reply = client.join_run(run_id).await;
        assert!(
            matches!(reply, ServerMessage::RunJoined { .. }),
            "{reply:?}"
        );
        client
    }

    /// The next unused request ID.
    pub fn next_id(&mut self) -> RequestId {
        let id = request_id(self.next_request);
        self.next_request += 1;
        id
    }

    pub async fn send_text(&mut self, text: impl Into<String>) {
        self.socket.send(Message::text(text.into())).await.unwrap();
    }

    pub async fn send_binary(&mut self, bytes: Vec<u8>) {
        self.socket.send(Message::binary(bytes)).await.unwrap();
    }

    pub async fn send(&mut self, message: &ClientMessage) {
        self.send_text(serde_json::to_string(message).unwrap())
            .await;
    }

    /// Receive a message, setting pushes aside. Returns `None` for a push.
    async fn receive_or_set_aside(&mut self) -> Option<ServerMessage> {
        match self.receive_raw().await {
            message @ ServerMessage::Observations { .. } => self.observations.push_back(message),
            ServerMessage::ViewUpdate { view } => self.views.push_back(view),
            ServerMessage::PacingUpdate { pacing } => self.pacings.push_back(pacing),
            message => return Some(message),
        }
        None
    }

    /// The next message that is not a push. Pushes received meanwhile are kept for
    /// [`Client::observations`] and [`Client::view_update`].
    pub async fn receive(&mut self) -> ServerMessage {
        loop {
            if let Some(message) = self.receive_or_set_aside().await {
                return message;
            }
        }
    }

    /// The next pushed `observations` message, as `(state_id, agent IDs)`.
    pub async fn observations(&mut self) -> (StateId, Vec<AgentId>) {
        loop {
            if let Some(message) = self.observations.pop_front() {
                let ServerMessage::Observations {
                    state_id,
                    observations,
                } = message
                else {
                    unreachable!("only observations are kept here");
                };
                return (state_id, observations.iter().map(|o| o.agent_id).collect());
            }
            if let Some(message) = self.receive_or_set_aside().await {
                panic!("expected observations, got {message:?}");
            }
        }
    }

    /// The next pushed view.
    pub async fn view_update(&mut self) -> View {
        loop {
            if let Some(view) = self.views.pop_front() {
                return view;
            }
            if let Some(message) = self.receive_or_set_aside().await {
                panic!("expected a view update, got {message:?}");
            }
        }
    }

    /// The next pushed pacing state.
    pub async fn pacing_update(&mut self) -> Pacing {
        loop {
            if let Some(pacing) = self.pacings.pop_front() {
                return pacing;
            }
            if let Some(message) = self.receive_or_set_aside().await {
                panic!("expected a pacing update, got {message:?}");
            }
        }
    }

    /// Require that no message arrives within `wait`.
    pub async fn expect_silence(&mut self, wait: Duration) {
        assert!(self.observations.is_empty(), "{:?}", self.observations);
        assert!(self.views.is_empty(), "{:?}", self.views);
        assert!(self.pacings.is_empty(), "{:?}", self.pacings);
        if let Ok(frame) = tokio::time::timeout(wait, self.socket.next()).await {
            panic!("expected no message, got {frame:?}");
        }
    }

    /// The next message of any kind, in arrival order.
    pub async fn receive_raw(&mut self) -> ServerMessage {
        loop {
            let frame = tokio::time::timeout(RECEIVE_TIMEOUT, self.socket.next())
                .await
                .expect("timed out waiting for a server message")
                .expect("connection closed")
                .unwrap();
            match frame {
                Message::Text(text) => return serde_json::from_str(text.as_str()).unwrap(),
                Message::Ping(_) | Message::Pong(_) => continue,
                other => panic!("unexpected frame {other:?}"),
            }
        }
    }

    /// Send a message and receive the reply.
    pub async fn exchange(&mut self, message: &ClientMessage) -> ServerMessage {
        self.send(message).await;
        self.receive().await
    }

    /// Send raw text and receive the reply.
    pub async fn exchange_text(&mut self, text: &str) -> ServerMessage {
        self.send_text(text).await;
        self.receive().await
    }

    pub async fn create_run(&mut self) -> RunId {
        self.create_run_from(empty_grid()).await
    }

    pub async fn create_run_from(&mut self, catalog_entry: CatalogEntryId) -> RunId {
        let request_id = self.next_id();
        let reply = self
            .exchange(&ClientMessage::CreateRun {
                request_id,
                catalog_entry,
            })
            .await;
        match reply {
            ServerMessage::RunCreated { run_id, .. } => run_id,
            other => panic!("expected run_created, got {other:?}"),
        }
    }

    pub async fn join_run(&mut self, run_id: &RunId) -> ServerMessage {
        let request_id = self.next_id();
        self.exchange(&ClientMessage::JoinRun {
            request_id,
            run_id: run_id.clone(),
        })
        .await
    }

    pub async fn spawn(&mut self, placement: Placement) -> ServerMessage {
        let request_id = self.next_id();
        self.exchange(&ClientMessage::Spawn {
            request_id,
            placement,
        })
        .await
    }

    pub async fn start(&mut self) -> ServerMessage {
        let request_id = self.next_id();
        self.exchange(&ClientMessage::Start { request_id }).await
    }

    /// Watch the run, returning the view in the `watching` response.
    pub async fn watch(&mut self) -> View {
        self.watch_with_pacing().await.0
    }

    /// Watch the run, returning the view and pacing state in the `watching` response.
    pub async fn watch_with_pacing(&mut self) -> (View, Pacing) {
        let watching = self.watching().await;
        (watching.view, watching.pacing)
    }

    /// Watch the run, returning the whole `watching` response.
    pub async fn watching(&mut self) -> Watching {
        let request_id = self.next_id();
        match self.exchange(&ClientMessage::Watch { request_id }).await {
            ServerMessage::Watching(watching) => watching,
            other => panic!("expected watching, got {other:?}"),
        }
    }

    pub async fn claim_pacing(&mut self) -> ServerMessage {
        let request_id = self.next_id();
        self.exchange(&ClientMessage::ClaimPacing { request_id })
            .await
    }

    pub async fn set_pacing(&mut self, mode: PacingMode) -> ServerMessage {
        let request_id = self.next_id();
        self.exchange(&ClientMessage::SetPacing { request_id, mode })
            .await
    }

    pub async fn leave_run(&mut self) -> ServerMessage {
        let request_id = self.next_id();
        self.exchange(&ClientMessage::LeaveRun { request_id }).await
    }

    pub async fn close_run(&mut self) -> ServerMessage {
        let request_id = self.next_id();
        self.exchange(&ClientMessage::CloseRun { request_id }).await
    }

    pub async fn step_once(&mut self) -> ServerMessage {
        let request_id = self.next_id();
        self.exchange(&ClientMessage::StepOnce { request_id }).await
    }

    /// Watch, claim pacing control, and consume the resulting pacing update.
    pub async fn watch_and_control(&mut self) {
        self.watch().await;
        let reply = self.claim_pacing().await;
        assert!(
            matches!(reply, ServerMessage::PacingClaimed { .. }),
            "{reply:?}"
        );
        assert!(self.pacing_update().await.you_control);
    }

    pub async fn submit(&mut self, state_id: StateId, actions: Vec<ActionEntry>) -> ServerMessage {
        let request_id = self.next_id();
        self.exchange(&ClientMessage::Submit {
            request_id,
            state_id,
            actions,
        })
        .await
    }

    /// Require that the server closes the connection next.
    pub async fn expect_closed(&mut self) {
        let frame = tokio::time::timeout(RECEIVE_TIMEOUT, self.socket.next())
            .await
            .expect("timed out waiting for the connection to close");
        assert!(
            matches!(frame, None | Some(Ok(Message::Close(_))) | Some(Err(_))),
            "expected the connection to close, got {frame:?}"
        );
    }

    /// Close the connection from the client side.
    pub async fn close(mut self) {
        self.socket.close(None).await.unwrap();
    }
}
