//! A test server and a minimal WebSocket client for the acceptance tests.

#![allow(dead_code, reason = "each test file uses a different subset")]

use std::net::SocketAddr;
use std::time::Duration;

use agora_protocol::{
    CatalogEntryId, ClientMessage, ErrorCode, ErrorResponse, PROTOCOL_VERSION, Placement,
    RequestId, RunId, ServerMessage,
};
use agora_server::{EMPTY_GRID_10X10, ServerConfig, serve};
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

pub fn cell(x: u32, y: u32) -> Placement {
    Placement::Cell { x, y }
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
}

impl Client {
    /// Connect without a handshake.
    pub async fn connect(address: SocketAddr) -> Self {
        let (socket, _) = connect_async(format!("ws://{address}")).await.unwrap();
        Self {
            socket,
            next_request: 1,
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

    /// Connect, complete the handshake, and create a run on the bundled catalog entry.
    pub async fn with_new_run(address: SocketAddr) -> (Self, RunId) {
        let mut client = Self::connect_with_hello(address).await;
        let run_id = client.create_run().await;
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

    pub async fn receive(&mut self) -> ServerMessage {
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
        let request_id = self.next_id();
        let reply = self
            .exchange(&ClientMessage::CreateRun {
                request_id,
                catalog_entry: empty_grid(),
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
