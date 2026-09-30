//! Connecting, establishing a session, and making requests.

use std::sync::{Arc, Mutex, PoisonError};

use agora_protocol::{
    ActionEntry, AgentId, CatalogEntryId, ClientMessage, CloseReason, EntryResult, Kind,
    PROTOCOL_VERSION, Pacing, PacingMode, Placement, RequestId, RunId, RunPhase, ServerMessage,
    SessionId, StateId, View, Watching,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::connection::{
    self, ClosedReason, ObservationBatch, Outgoing, Pushes, Socket, WatchedRun,
};
use crate::error::ClientError;

/// A connection that has completed the version handshake but has no session yet.
pub struct Client {
    socket: Socket,
}

impl Client {
    /// Connect to a server at a `ws://` URL and complete the version handshake.
    pub async fn connect(url: &str) -> Result<Self, ClientError> {
        let (mut socket, _) = connect_async(url).await?;
        let hello = ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
        };
        socket
            .send(Message::text(
                serde_json::to_string(&hello).expect("client messages serialize"),
            ))
            .await?;
        match next_message(&mut socket).await? {
            ServerMessage::Welcome { .. } => Ok(Self { socket }),
            ServerMessage::Error(error) => Err(ClientError::Rejected(error)),
            other => Err(ClientError::Unexpected(Box::new(other))),
        }
    }

    /// Create a run from a catalog entry. The new session holds creator authority. Returns the
    /// session and its observation stream.
    pub async fn create_run(
        self,
        catalog_entry: CatalogEntryId,
    ) -> Result<(Session, Observations), ClientError> {
        self.establish(|request_id| ClientMessage::CreateRun {
            request_id,
            catalog_entry,
        })
        .await
    }

    /// Join an existing run. Returns the session and its observation stream.
    pub async fn join_run(self, run_id: RunId) -> Result<(Session, Observations), ClientError> {
        self.establish(|request_id| ClientMessage::JoinRun { request_id, run_id })
            .await
    }

    async fn establish(
        self,
        request: impl FnOnce(RequestId) -> ClientMessage,
    ) -> Result<(Session, Observations), ClientError> {
        let (requests, receiver) = mpsc::unbounded_channel();
        let (observations_sender, observations) = mpsc::unbounded_channel();
        let (view_sender, view) = watch::channel(None);
        let (pacing_sender, pacing) = watch::channel(None);
        let watched = WatchedRun::default();
        let closed = ClosedReason::default();
        tokio::spawn(connection::run(
            self.socket,
            receiver,
            Pushes {
                observations: observations_sender,
                view: view_sender,
                pacing: pacing_sender,
                watched: watched.clone(),
                closed: closed.clone(),
            },
        ));
        let requests = Requests {
            next: Mutex::new(1),
            sender: requests,
            closed,
        };
        let info = match requests.call(request).await? {
            ServerMessage::RunCreated {
                run_id,
                session_id,
                catalog_entry,
                state_id,
                phase,
                ..
            }
            | ServerMessage::RunJoined {
                run_id,
                session_id,
                catalog_entry,
                state_id,
                phase,
                ..
            } => SessionInfo {
                run_id,
                session_id,
                catalog_entry,
                state_id,
                phase,
            },
            other => return Err(ClientError::Unexpected(Box::new(other))),
        };
        let session = Session {
            info: Arc::new(info),
            requests: Arc::new(requests),
            view,
            pacing,
            watched,
        };
        Ok((
            session,
            Observations {
                receiver: observations,
            },
        ))
    }
}

/// What the server reported when the session was established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub run_id: RunId,
    pub session_id: SessionId,
    pub catalog_entry: CatalogEntryId,
    /// The run's state when the session was established.
    pub state_id: StateId,
    /// The run's phase when the session was established.
    pub phase: RunPhase,
}

/// A handle to a session in a run. It is cheap to clone, and every clone sends requests on
/// the same session, so several tasks can make requests at once. The newest view and pacing
/// state are available through latest-value handles. The connection closes when every clone
/// has been dropped.
#[derive(Clone)]
pub struct Session {
    info: Arc<SessionInfo>,
    requests: Arc<Requests>,
    view: watch::Receiver<Option<View>>,
    pacing: watch::Receiver<Option<Pacing>>,
    watched: WatchedRun,
}

impl Session {
    pub fn info(&self) -> &SessionInfo {
        &self.info
    }

    /// Spawn an agent owned by this session and return its ID.
    pub async fn spawn(&self, placement: Placement) -> Result<AgentId, ClientError> {
        match self
            .call(|request_id| ClientMessage::Spawn {
                request_id,
                placement,
            })
            .await?
        {
            ServerMessage::Spawned { agent_id, .. } => Ok(agent_id),
            other => Err(ClientError::Unexpected(Box::new(other))),
        }
    }

    /// Start the run. Creator only.
    pub async fn start(&self) -> Result<(), ClientError> {
        match self
            .call(|request_id| ClientMessage::Start { request_id })
            .await?
        {
            ServerMessage::Started { .. } => Ok(()),
            other => Err(ClientError::Unexpected(Box::new(other))),
        }
    }

    /// Submit actions targeting `state_id`. Returns one result per entry, in order; an entry
    /// was accepted when its `error` is `None`.
    pub async fn submit(
        &self,
        state_id: StateId,
        actions: Vec<ActionEntry>,
    ) -> Result<Vec<EntryResult>, ClientError> {
        match self
            .call(|request_id| ClientMessage::Submit {
                request_id,
                state_id,
                actions,
            })
            .await?
        {
            ServerMessage::Submitted { results, .. } => Ok(results),
            other => Err(ClientError::Unexpected(Box::new(other))),
        }
    }

    /// Become a viewer. Returns the current view; afterwards [`Session::view`] and
    /// [`Session::pacing`] hold the newest view and pacing state, and [`Session::kinds`] and
    /// [`Session::terrain`] hold the run's kinds and terrain.
    pub async fn watch(&self) -> Result<View, ClientError> {
        match self
            .call(|request_id| ClientMessage::Watch { request_id })
            .await?
        {
            ServerMessage::Watching(Watching { view, .. }) => Ok(view),
            other => Err(ClientError::Unexpected(Box::new(other))),
        }
    }

    /// Claim pacing control. Viewers only.
    pub async fn claim_pacing(&self) -> Result<(), ClientError> {
        self.expect_ok(
            |request_id| ClientMessage::ClaimPacing { request_id },
            |m| matches!(m, ServerMessage::PacingClaimed { .. }),
        )
        .await
    }

    /// Set the pacing mode. Pacing controller only.
    pub async fn set_pacing(&self, mode: PacingMode) -> Result<(), ClientError> {
        self.expect_ok(
            |request_id| ClientMessage::SetPacing { request_id, mode },
            |m| matches!(m, ServerMessage::PacingSet { .. }),
        )
        .await
    }

    /// Allow one step while paused. Pacing controller only. The step's result arrives as a
    /// new view.
    pub async fn step_once(&self) -> Result<(), ClientError> {
        self.expect_ok(
            |request_id| ClientMessage::StepOnce { request_id },
            |m| matches!(m, ServerMessage::StepGranted { .. }),
        )
        .await
    }

    /// Leave the run: the session ends and its agents are removed. The connection then
    /// closes, and every clone's later requests fail with [`ClientError::Closed`].
    pub async fn leave(&self) -> Result<(), ClientError> {
        self.expect_ok(
            |request_id| ClientMessage::LeaveRun { request_id },
            |m| matches!(m, ServerMessage::Left { .. }),
        )
        .await
    }

    /// Close the run for every session. Creator only. The connection then closes.
    pub async fn close(&self) -> Result<(), ClientError> {
        self.expect_ok(
            |request_id| ClientMessage::CloseRun { request_id },
            |m| matches!(m, ServerMessage::Closed { .. }),
        )
        .await
    }

    /// Why the server closed the run, if it has. Requests then fail with
    /// [`ClientError::RunClosed`], and the observation stream ends.
    pub fn closed_reason(&self) -> Option<CloseReason> {
        *self
            .requests
            .closed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// A handle to the newest view. It is `None` until the session watches.
    pub fn view(&self) -> watch::Receiver<Option<View>> {
        self.view.clone()
    }

    /// A handle to the newest pacing state. It is `None` until the session watches.
    pub fn pacing(&self) -> watch::Receiver<Option<Pacing>> {
        self.pacing.clone()
    }

    /// The run's kinds, which views refer to. `None` until the session watches; they do not
    /// change afterwards.
    pub fn kinds(&self) -> Option<&[Kind]> {
        self.watched.get().map(|run| run.kinds.as_slice())
    }

    /// Every cell's terrain as an index into [`kinds`](Self::kinds), in row-major order from the
    /// south-west corner (`y * width + x`). `None` until the session watches. It is the terrain
    /// when the session first watched; terrain does not change during a run yet.
    pub fn terrain(&self) -> Option<&[u32]> {
        self.watched.get().map(|run| run.terrain.as_slice())
    }

    async fn expect_ok(
        &self,
        request: impl FnOnce(RequestId) -> ClientMessage,
        success: impl FnOnce(&ServerMessage) -> bool,
    ) -> Result<(), ClientError> {
        let response = self.requests.call(request).await?;
        if success(&response) {
            Ok(())
        } else {
            Err(ClientError::Unexpected(Box::new(response)))
        }
    }

    async fn call(
        &self,
        request: impl FnOnce(RequestId) -> ClientMessage,
    ) -> Result<ServerMessage, ClientError> {
        self.requests.call(request).await
    }
}

/// A session's observations, in the order the server sent them. It is returned once, with
/// the session, so exactly one task owns it.
pub struct Observations {
    receiver: mpsc::UnboundedReceiver<ObservationBatch>,
}

impl Observations {
    /// The next observations for the session's agents. Returns `None` once the connection has
    /// closed and every batch already received has been returned.
    pub async fn next(&mut self) -> Option<ObservationBatch> {
        self.receiver.recv().await
    }
}

/// Numbers a session's requests and hands them to the connection task.
struct Requests {
    /// The next request ID. The request that establishes the session is 1. The lock is held
    /// while a request is queued, so requests reach the server in number order even when
    /// several tasks send at once.
    next: Mutex<u64>,
    sender: mpsc::UnboundedSender<Outgoing>,
    closed: ClosedReason,
}

impl Requests {
    /// Send a request with the next request ID and wait for its response. A rejection is
    /// returned as [`ClientError::Rejected`].
    async fn call(
        &self,
        request: impl FnOnce(RequestId) -> ClientMessage,
    ) -> Result<ServerMessage, ClientError> {
        let (reply, response) = oneshot::channel();
        {
            let mut next = self.next.lock().unwrap_or_else(PoisonError::into_inner);
            let id = RequestId::new(*next).expect("a session sends fewer than 2^53 requests");
            self.sender
                .send(Outgoing {
                    message: request(id),
                    reply,
                })
                .map_err(|_| self.closed_error())?;
            *next += 1;
        }
        response.await.map_err(|_| self.closed_error())?
    }

    /// The error for a request the connection could not answer.
    fn closed_error(&self) -> ClientError {
        match *self.closed.lock().unwrap_or_else(PoisonError::into_inner) {
            Some(reason) => ClientError::RunClosed(reason),
            None => ClientError::Closed,
        }
    }
}

/// Read the next server message during the handshake.
async fn next_message(socket: &mut Socket) -> Result<ServerMessage, ClientError> {
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) => return Ok(serde_json::from_str(text.as_str())?),
            Some(Ok(Message::Close(_))) | None => return Err(ClientError::Closed),
            Some(Ok(_)) => continue,
            Some(Err(e)) => return Err(e.into()),
        }
    }
}
