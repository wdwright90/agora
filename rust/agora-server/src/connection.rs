//! One client connection: the version handshake, then requests handled one at a time
//! (SPEC-002).

use agora_protocol::{
    Action, AgentId, ClientMessage, Direction, EntryError, EntryResult, ErrorCode, ErrorResponse,
    PROTOCOL_VERSION, ProtocolVersion, RequestId, RunId, ServerMessage, SessionId,
};
use agora_sim::{AgentLimitError, GridPos, SpawnError, SubmitError};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info};

use crate::catalog;
use crate::config::ServerConfig;
use crate::pacing::PacingRejection;
use crate::registry::Registry;
use crate::requests::{Admission, RequestLog};
use crate::run::{
    self, EntryRejection, NotCreator, Outbox, RunGone, RunHandle, StartRejection, SubmitRejection,
    ViewSlot,
};
use crate::wire;

/// Serve one TCP connection until the client disconnects.
/// `number` is the connection's position in acceptance order.
pub async fn serve(stream: TcpStream, number: u64, registry: Registry, config: ServerConfig) {
    let mut socket = match tokio_tungstenite::accept_async(stream).await {
        Ok(socket) => socket,
        Err(e) => {
            debug!(error = %e, "WebSocket handshake failed");
            return;
        }
    };
    info!("connected");
    let (outbox, mut pushes) = mpsc::unbounded_channel();
    let (view_slot, mut views) = watch::channel(None);
    let mut connection = Connection {
        registry,
        config,
        outbox,
        view_slot,
        number,
        handshake_done: false,
        session: None,
    };
    loop {
        // A request is answered before anything else is read, so the run's pushes and views
        // produced while handling it are sent after its response.
        let reply = tokio::select! {
            frame = socket.next() => match frame {
                Some(Ok(Message::Text(text))) => connection.receive(text.as_str()).await,
                Some(Ok(Message::Binary(_))) => Reply::error(wire::malformed(
                    None,
                    "binary frames are not supported; send JSON text frames",
                )),
                Some(Ok(Message::Close(_))) | None => break,
                // Tungstenite answers pings itself.
                Some(Ok(_)) => continue,
                Some(Err(e)) => {
                    debug!(error = %e, "read failed");
                    break;
                }
            },
            // The connection holds a sender for each channel, so neither closes.
            Some(push) = pushes.recv() => {
                if matches!(push, ServerMessage::RunClosed { .. }) {
                    // The run has ended this session; the connection may create or join again.
                    connection.session = None;
                }
                Reply::send(push)
            }
            Ok(()) = views.changed() => match views.borrow_and_update().clone() {
                Some(view) => Reply::send(ServerMessage::ViewUpdate { view }),
                None => continue,
            },
        };
        if matches!(
            reply.message,
            ServerMessage::Left { .. } | ServerMessage::Closed { .. }
        ) {
            // Nothing more comes from the ended session's run, so send what it already queued
            // before the response that ends the session.
            let mut queued = Vec::new();
            while let Ok(push) = pushes.try_recv() {
                queued.push(push);
            }
            if views.has_changed().unwrap_or(false)
                && let Some(view) = views.borrow_and_update().clone()
            {
                queued.push(ServerMessage::ViewUpdate { view });
            }
            if send(&mut socket, &queued).await.is_err() {
                break;
            }
        }
        if send(&mut socket, std::slice::from_ref(&reply.message))
            .await
            .is_err()
        {
            break;
        }
        if reply.close {
            let _ = socket.close(None).await;
            break;
        }
    }
    if let Some(session) = connection.session {
        session.run.disconnect(session.id).await;
    }
    info!("disconnected");
}

/// Send messages in order. Fails if the socket does.
async fn send(
    socket: &mut tokio_tungstenite::WebSocketStream<TcpStream>,
    messages: &[ServerMessage],
) -> Result<(), ()> {
    for message in messages {
        let text = serde_json::to_string(message).expect("server messages serialize");
        if let Err(e) = socket.send(Message::text(text)).await {
            debug!(error = %e, "send failed");
            return Err(());
        }
    }
    Ok(())
}

struct Connection {
    registry: Registry,
    config: ServerConfig,
    /// Given to the run on create or join, for messages it pushes to this connection.
    outbox: Outbox,
    /// Given to the run on `watch`, for the views it publishes to this connection.
    view_slot: ViewSlot,
    /// This connection's position in acceptance order.
    number: u64,
    handshake_done: bool,
    session: Option<SessionLink>,
}

/// This connection's session.
struct SessionLink {
    id: SessionId,
    run_id: RunId,
    run: RunHandle,
    view_slot: ViewSlot,
    connection: u64,
    requests: RequestLog,
}

/// The response to one frame, and whether to close the connection after sending it.
struct Reply {
    message: ServerMessage,
    close: bool,
}

impl Reply {
    fn send(message: ServerMessage) -> Self {
        Self {
            message,
            close: false,
        }
    }

    fn error(error: ErrorResponse) -> Self {
        Self::send(ServerMessage::Error(error))
    }
}

impl Connection {
    async fn receive(&mut self, text: &str) -> Reply {
        let message = match wire::parse(text) {
            Ok(message) => message,
            Err(error) => return Reply::error(error),
        };
        match message {
            ClientMessage::Hello { protocol_version } => self.hello(protocol_version),
            request if !self.handshake_done => Reply::error(wire::error(
                request.request_id(),
                ErrorCode::HandshakeRequired,
                "send hello before any request",
            )),
            request => Reply::send(self.request(request).await),
        }
    }

    fn hello(&mut self, requested: ProtocolVersion) -> Reply {
        if self.handshake_done {
            return Reply::error(wire::malformed(
                None,
                "hello was already received on this connection",
            ));
        }
        if !PROTOCOL_VERSION.is_compatible_with(requested) {
            info!(%requested, "unsupported protocol version");
            return Reply {
                message: ServerMessage::Error(wire::with_details(
                    wire::error(
                        None,
                        ErrorCode::UnsupportedProtocolVersion,
                        format!("protocol version {requested} is not supported"),
                    ),
                    json!({ "requested": requested, "supported": [PROTOCOL_VERSION] }),
                )),
                close: true,
            };
        }
        self.handshake_done = true;
        Reply::send(ServerMessage::Welcome {
            protocol_version: PROTOCOL_VERSION,
        })
    }

    async fn request(&mut self, request: ClientMessage) -> ServerMessage {
        let id = request
            .request_id()
            .expect("every client message except hello is a request");
        let Some(session) = &mut self.session else {
            return self.establish(id, request).await;
        };
        match session.requests.admit(id, &request) {
            Admission::Replay(response) => {
                debug!(request_id = %id, "replaying a retained result");
                response.clone()
            }
            Admission::Rejected(error) => ServerMessage::Error(error),
            Admission::New => {
                let response = session.execute(id, &request).await;
                if matches!(
                    response,
                    ServerMessage::Left { .. } | ServerMessage::Closed { .. }
                ) {
                    // The session has ended, and its request log with it.
                    self.session = None;
                } else {
                    session.requests.record(id, request, response.clone());
                }
                response
            }
        }
    }

    /// Handle a request on a connection without a session.
    async fn establish(&mut self, id: RequestId, request: ClientMessage) -> ServerMessage {
        let (run_id, run, session, response) = match &request {
            ClientMessage::CreateRun { catalog_entry, .. } => {
                let Some(entry) = catalog::lookup(catalog_entry) else {
                    return ServerMessage::Error(wire::with_details(
                        wire::error(
                            Some(id),
                            ErrorCode::UnknownCatalogEntry,
                            format!("no catalog entry {catalog_entry:?}"),
                        ),
                        json!({ "catalog_entry": catalog_entry }),
                    ));
                };
                let (run_id, run, session) = run::create(
                    catalog_entry.clone(),
                    entry,
                    &self.registry,
                    self.config,
                    self.outbox.clone(),
                );
                let response = ServerMessage::RunCreated {
                    request_id: id,
                    run_id: run_id.clone(),
                    session_id: session.session_id.clone(),
                    catalog_entry: session.catalog_entry,
                    state_id: session.state_id,
                    phase: session.phase,
                };
                (run_id, run, session.session_id, response)
            }
            ClientMessage::JoinRun { run_id, .. } => {
                let joined = match self.registry.get(run_id) {
                    Some(run) => run
                        .join(self.outbox.clone())
                        .await
                        .ok()
                        .map(|session| (run, session)),
                    None => None,
                };
                let Some((run, session)) = joined else {
                    return unknown_run(id, run_id);
                };
                let response = ServerMessage::RunJoined {
                    request_id: id,
                    run_id: run_id.clone(),
                    session_id: session.session_id.clone(),
                    catalog_entry: session.catalog_entry,
                    state_id: session.state_id,
                    phase: session.phase,
                };
                (run_id.clone(), run, session.session_id, response)
            }
            _ => {
                return ServerMessage::Error(wire::error(
                    Some(id),
                    ErrorCode::NoSession,
                    "create or join a run first",
                ));
            }
        };
        self.session = Some(SessionLink {
            id: session,
            run_id,
            run,
            view_slot: self.view_slot.clone(),
            connection: self.number,
            requests: RequestLog::new(id, request, response.clone()),
        });
        response
    }
}

impl SessionLink {
    /// Execute a newly admitted request.
    async fn execute(&self, id: RequestId, request: &ClientMessage) -> ServerMessage {
        let result = match request {
            ClientMessage::CreateRun { .. } | ClientMessage::JoinRun { .. } => {
                return ServerMessage::Error(wire::error(
                    Some(id),
                    ErrorCode::SessionAlreadyEstablished,
                    "this connection already has a session",
                ));
            }
            ClientMessage::Spawn { placement, .. } => self
                .run
                .spawn(self.id.clone(), to_sim_placement(*placement))
                .await
                .map(|result| match result {
                    Ok(agent) => ServerMessage::Spawned {
                        request_id: id,
                        agent_id: AgentId::new(agent.0).expect("agent IDs stay below 2^53"),
                    },
                    Err(e) => ServerMessage::Error(spawn_error(id, e)),
                }),
            ClientMessage::Start { .. } => {
                self.run
                    .start(self.id.clone())
                    .await
                    .map(|result| match result {
                        Ok(()) => ServerMessage::Started { request_id: id },
                        Err(rejection) => ServerMessage::Error(start_error(id, rejection)),
                    })
            }
            ClientMessage::Submit {
                state_id, actions, ..
            } => {
                let actions = actions
                    .iter()
                    .map(|entry| (to_sim_agent(entry.agent_id), to_sim_move(entry.action)))
                    .collect();
                self.run
                    .submit(self.id.clone(), agora_sim::StateId(state_id.get()), actions)
                    .await
                    .map(|result| match result {
                        Ok(results) => ServerMessage::Submitted {
                            request_id: id,
                            state_id: *state_id,
                            results: results.into_iter().map(entry_result).collect(),
                        },
                        Err(rejection) => ServerMessage::Error(submit_error(id, rejection)),
                    })
            }
            ClientMessage::Watch { .. } => self
                .run
                .watch(self.id.clone(), self.connection, self.view_slot.clone())
                .await
                .map(|(view, pacing)| ServerMessage::Watching {
                    request_id: id,
                    view,
                    pacing,
                }),
            ClientMessage::ClaimPacing { .. } => {
                self.run.claim_pacing(self.id.clone()).await.map(|result| {
                    pacing_response(id, result, ServerMessage::PacingClaimed { request_id: id })
                })
            }
            ClientMessage::SetPacing { mode, .. } => self
                .run
                .set_pacing(self.id.clone(), *mode)
                .await
                .map(|result| {
                    pacing_response(id, result, ServerMessage::PacingSet { request_id: id })
                }),
            ClientMessage::StepOnce { .. } => {
                self.run.step_once(self.id.clone()).await.map(|result| {
                    pacing_response(id, result, ServerMessage::StepGranted { request_id: id })
                })
            }
            ClientMessage::LeaveRun { .. } => self
                .run
                .leave(self.id.clone())
                .await
                .map(|()| ServerMessage::Left { request_id: id }),
            ClientMessage::CloseRun { .. } => {
                self.run
                    .close(self.id.clone())
                    .await
                    .map(|result| match result {
                        Ok(()) => ServerMessage::Closed { request_id: id },
                        Err(NotCreator) => ServerMessage::Error(wire::error(
                            Some(id),
                            ErrorCode::NotCreator,
                            "only the run's creator can close it",
                        )),
                    })
            }
            ClientMessage::Hello { .. } => unreachable!("hello is not a request"),
        };
        // A run is not released while it has a connected session, so this is not expected.
        result.unwrap_or_else(|RunGone| unknown_run(id, &self.run_id))
    }
}

fn to_sim_placement(placement: agora_protocol::Placement) -> agora_sim::Placement {
    // The protocol and the simulation use the same coordinate convention.
    match placement {
        agora_protocol::Placement::Cell { x, y } => agora_sim::Placement::Cell(GridPos::new(x, y)),
        agora_protocol::Placement::Random => agora_sim::Placement::Random,
    }
}

fn to_sim_agent(agent: AgentId) -> agora_sim::AgentId {
    agora_sim::AgentId(agent.get())
}

fn to_sim_move(action: Action) -> agora_sim::Move {
    let Action::Move {
        direction,
        distance,
    } = action;
    let direction = match direction {
        Direction::North => agora_sim::Direction::North,
        Direction::East => agora_sim::Direction::East,
        Direction::South => agora_sim::Direction::South,
        Direction::West => agora_sim::Direction::West,
    };
    agora_sim::Move {
        direction,
        distance,
    }
}

fn submit_error(id: RequestId, rejection: SubmitRejection) -> ErrorResponse {
    match rejection {
        SubmitRejection::NotStarted => wire::error(
            Some(id),
            ErrorCode::RunNotStarted,
            "the run has not started",
        ),
        SubmitRejection::WrongState { expected, supplied } => wire::with_details(
            wire::error(
                Some(id),
                ErrorCode::WrongState,
                format!("submission targets {supplied}, but the current state is {expected}"),
            ),
            json!({ "expected": expected.0, "supplied": supplied.0 }),
        ),
    }
}

fn entry_result((agent, result): (agora_sim::AgentId, Result<(), EntryRejection>)) -> EntryResult {
    let error = result.err().map(|rejection| {
        let (code, message, details) = match rejection {
            // Owned agents stay in the world while their session exists.
            EntryRejection::NotOwned | EntryRejection::Sim(SubmitError::UnknownAgent(_)) => (
                ErrorCode::AgentNotOwned,
                format!("{agent} is not owned by this session"),
                None,
            ),
            EntryRejection::Sim(error @ SubmitError::AlreadySubmitted(_)) => {
                (ErrorCode::AlreadySubmitted, error.to_string(), None)
            }
            EntryRejection::Sim(
                error @ SubmitError::AgentLimit(AgentLimitError::DistanceBudgetExceeded {
                    distance,
                    budget,
                }),
            ) => (
                ErrorCode::DistanceBudgetExceeded,
                error.to_string(),
                Some(json!({ "distance": distance, "budget": budget })),
            ),
            // The run checks the phase and target state for the whole submission first.
            EntryRejection::Sim(
                error @ (SubmitError::NotStarted | SubmitError::WrongTargetState { .. }),
            ) => unreachable!("checked for the whole submission: {error}"),
        };
        EntryError {
            code,
            message,
            details: details.map(wire::object),
        }
    });
    EntryResult {
        agent_id: AgentId::new(agent.0).expect("agent IDs stay below 2^53"),
        error,
    }
}

fn pacing_response(
    id: RequestId,
    result: Result<(), PacingRejection>,
    success: ServerMessage,
) -> ServerMessage {
    let Err(rejection) = result else {
        return success;
    };
    let (code, message) = match rejection {
        PacingRejection::NotViewing => (
            ErrorCode::NotViewing,
            "only a viewer can claim pacing control; send watch first",
        ),
        PacingRejection::ControlHeld => (
            ErrorCode::PacingControlHeld,
            "another viewer holds pacing control",
        ),
        PacingRejection::NotController => (
            ErrorCode::NotPacingController,
            "this session does not hold pacing control",
        ),
        PacingRejection::NotPaused => (
            ErrorCode::RunNotPaused,
            "a single step can only be granted while paused",
        ),
    };
    ServerMessage::Error(wire::error(Some(id), code, message))
}

fn spawn_error(id: RequestId, error: SpawnError) -> ErrorResponse {
    let message = error.to_string();
    match error {
        SpawnError::NotInSetup => wire::error(Some(id), ErrorCode::RunAlreadyStarted, message),
        SpawnError::OutOfBounds(cell) => wire::with_details(
            wire::error(Some(id), ErrorCode::CellOutOfBounds, message),
            json!({ "x": cell.x, "y": cell.y }),
        ),
        SpawnError::Occupied(cell) => wire::with_details(
            wire::error(Some(id), ErrorCode::CellOccupied, message),
            json!({ "x": cell.x, "y": cell.y }),
        ),
        SpawnError::NoFreeCell => wire::error(Some(id), ErrorCode::NoFreeCell, message),
    }
}

fn start_error(id: RequestId, rejection: StartRejection) -> ErrorResponse {
    let (code, message) = match rejection {
        StartRejection::NotCreator => {
            (ErrorCode::NotCreator, "only the run's creator can start it")
        }
        StartRejection::AlreadyStarted => {
            (ErrorCode::RunAlreadyStarted, "the run has already started")
        }
        StartRejection::NotEligible => (
            ErrorCode::StartNotEligible,
            "the run has no agents and no connected viewers",
        ),
    };
    wire::error(Some(id), code, message)
}

fn unknown_run(id: RequestId, run_id: &RunId) -> ServerMessage {
    ServerMessage::Error(wire::with_details(
        wire::error(
            Some(id),
            ErrorCode::UnknownRun,
            format!("no run {run_id:?}"),
        ),
        json!({ "run_id": run_id }),
    ))
}
