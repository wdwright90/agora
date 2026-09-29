//! The background task that owns a connection's socket. It writes requests, matches responses
//! to them by request ID, and routes pushed messages to the session's stream and latest-value
//! handles.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use agora_protocol::{
    AgentObservation, ClientMessage, CloseReason, Pacing, RequestId, ServerMessage, StateId, View,
};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, watch};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, warn};

use crate::error::ClientError;

pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Observations of one state for the agents this session owns, in ascending agent ID order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationBatch {
    pub state_id: StateId,
    pub observations: Vec<AgentObservation>,
}

/// A request for the connection task, with where to send its response.
pub struct Outgoing {
    pub message: ClientMessage,
    pub reply: oneshot::Sender<Result<ServerMessage, ClientError>>,
}

/// Where the connection task delivers pushed messages.
pub struct Pushes {
    pub observations: mpsc::UnboundedSender<ObservationBatch>,
    pub view: watch::Sender<Option<View>>,
    pub pacing: watch::Sender<Option<Pacing>>,
    /// Set when the server closes the run, so later requests can report why.
    pub closed: ClosedReason,
}

/// Why the server closed the session's run, once it has.
pub type ClosedReason = Arc<Mutex<Option<CloseReason>>>;

/// Run the connection until the socket closes or every session handle is dropped.
pub async fn run(
    mut socket: Socket,
    mut requests: mpsc::UnboundedReceiver<Outgoing>,
    pushes: Pushes,
) {
    let mut pending: HashMap<RequestId, oneshot::Sender<Result<ServerMessage, ClientError>>> =
        HashMap::new();
    loop {
        tokio::select! {
            request = requests.recv() => {
                let Some(Outgoing { message, reply }) = request else {
                    // Every handle is gone: close politely.
                    let _ = socket.close(None).await;
                    break;
                };
                let id = message.request_id().expect("sessions send only requests");
                let text = serde_json::to_string(&message).expect("client messages serialize");
                match socket.send(Message::text(text)).await {
                    Ok(()) => {
                        pending.insert(id, reply);
                    }
                    Err(e) => {
                        let _ = reply.send(Err(ClientError::Transport(e)));
                        break;
                    }
                }
            }
            frame = socket.next() => match frame {
                Some(Ok(Message::Text(text))) => match serde_json::from_str(text.as_str()) {
                    Ok(message) => {
                        if dispatch(message, &mut pending, &pushes) == Session::Ended {
                            // The session is over; this library uses one session per connection.
                            let _ = socket.close(None).await;
                            break;
                        }
                    }
                    Err(e) => warn!(error = %e, "ignoring an unreadable server message"),
                },
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(e)) => {
                    debug!(error = %e, "connection failed");
                    break;
                }
            },
        }
    }
    // Dropping `pending` fails every waiting request with `Closed`, and dropping `pushes` ends
    // the observation stream.
}

/// Whether the session continues after a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Session {
    Continues,
    /// The session left, closed its run, or had its run closed.
    Ended,
}

fn dispatch(
    message: ServerMessage,
    pending: &mut HashMap<RequestId, oneshot::Sender<Result<ServerMessage, ClientError>>>,
    pushes: &Pushes,
) -> Session {
    let ends = matches!(
        message,
        ServerMessage::Left { .. } | ServerMessage::Closed { .. }
    );
    let request_id = match &message {
        ServerMessage::Observations {
            state_id,
            observations,
        } => {
            // The session may have stopped reading observations; that is its choice.
            let _ = pushes.observations.send(ObservationBatch {
                state_id: *state_id,
                observations: observations.clone(),
            });
            return Session::Continues;
        }
        ServerMessage::ViewUpdate { view } => {
            pushes.view.send_replace(Some(view.clone()));
            return Session::Continues;
        }
        ServerMessage::PacingUpdate { pacing } => {
            pushes.pacing.send_replace(Some(*pacing));
            return Session::Continues;
        }
        ServerMessage::RunClosed { reason } => {
            *pushes.closed.lock().unwrap_or_else(PoisonError::into_inner) = Some(*reason);
            return Session::Ended;
        }
        ServerMessage::Watching { view, pacing, .. } => {
            // Fill the handles before the caller sees the response.
            pushes.view.send_replace(Some(view.clone()));
            pushes.pacing.send_replace(Some(*pacing));
            message.request_id()
        }
        _ => message.request_id(),
    };
    let Some(id) = request_id else {
        warn!(?message, "ignoring a server message without a request ID");
        return Session::Continues;
    };
    let Some(reply) = pending.remove(&id) else {
        warn!(%id, "ignoring a response to no pending request");
        return Session::Continues;
    };
    let result = match message {
        ServerMessage::Error(error) => Err(ClientError::Rejected(error)),
        message => Ok(message),
    };
    let _ = reply.send(result);
    if ends {
        Session::Ended
    } else {
        Session::Continues
    }
}
