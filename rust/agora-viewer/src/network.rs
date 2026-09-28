//! The bridge between Bevy and the server. The client library runs on a Tokio runtime in a
//! background thread, so Bevy never waits on the network. Bevy sends [`Command`]s and receives
//! [`NetworkEvent`]s over channels; the view and pacing latest-value handles arrive once the
//! session is established and are read directly each frame.

use agora_client::protocol::{CatalogEntryId, CloseReason, Pacing, PacingMode, RunId, View};
use agora_client::{Client, ClientError, Session, SessionInfo};
use tokio::sync::{mpsc, watch};
use tracing::{info, warn};

/// The catalog entry the viewer creates runs from.
pub const CATALOG_ENTRY: &str = "empty-grid-10x10";

/// Which run the viewer shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Create a run on the bundled catalog entry. The viewer holds creator authority.
    Create,
    /// Watch an existing run.
    Join(RunId),
}

/// A request from the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Start,
    ClaimPacing,
    SetPacing(PacingMode),
    StepOnce,
    /// Close the run for everyone. Creator only.
    Close,
}

/// What the network thread reports to Bevy.
pub enum NetworkEvent {
    /// The session is established and watching. Carries the latest-value handles.
    Watching {
        info: SessionInfo,
        creator: bool,
        view: watch::Receiver<Option<View>>,
        pacing: watch::Receiver<Option<Pacing>>,
    },
    /// A command failed; the viewer keeps running.
    CommandFailed { command: Command, error: String },
    /// The connection could not be established or has closed. Nothing more will arrive.
    Disconnected { reason: String },
}

/// Bevy's end of the bridge.
pub struct Network {
    pub commands: mpsc::UnboundedSender<Command>,
    pub events: mpsc::UnboundedReceiver<NetworkEvent>,
}

/// Start the network thread for `target` on the server at `url`.
pub fn start(url: String, target: Target) -> Network {
    let (commands, command_receiver) = mpsc::unbounded_channel();
    let (event_sender, events) = mpsc::unbounded_channel();
    std::thread::Builder::new()
        .name("agora-network".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .expect("the network runtime starts");
            runtime.block_on(run(url, target, command_receiver, event_sender));
        })
        .expect("the network thread starts");
    Network { commands, events }
}

async fn run(
    url: String,
    target: Target,
    mut commands: mpsc::UnboundedReceiver<Command>,
    events: mpsc::UnboundedSender<NetworkEvent>,
) {
    let session = match establish(&url, &target).await {
        Ok(session) => session,
        Err(error) => {
            let _ = events.send(NetworkEvent::Disconnected {
                reason: error.to_string(),
            });
            return;
        }
    };
    let creator = target == Target::Create;
    info!(run = %session.info().run_id, creator, "watching");
    // Take pacing control if nobody holds it. Another viewer may already have it.
    if let Err(error) = session.claim_pacing().await
        && error.code().is_none()
    {
        let _ = events.send(NetworkEvent::Disconnected {
            reason: error.to_string(),
        });
        return;
    }
    let mut closed = session.view();
    let _ = events.send(NetworkEvent::Watching {
        info: session.info().clone(),
        creator,
        view: session.view(),
        pacing: session.pacing(),
    });

    loop {
        tokio::select! {
            command = commands.recv() => {
                // Bevy has exited.
                let Some(command) = command else { return };
                match execute(&session, command).await {
                    Ok(()) if command == Command::Close => {
                        let _ = events.send(NetworkEvent::Disconnected {
                            reason: "you closed the run".into(),
                        });
                        return;
                    }
                    Ok(()) => {}
                    Err(error) => {
                        warn!(?command, %error, "command failed");
                        let _ = events.send(NetworkEvent::CommandFailed {
                            command,
                            error: error.to_string(),
                        });
                    }
                }
            }
            // The view handle closes when the connection does.
            changed = closed.changed() => if changed.is_err() {
                let reason = match session.closed_reason() {
                    Some(reason) => format!("the run was closed ({})", describe_close(reason)),
                    None => "the connection closed".into(),
                };
                let _ = events.send(NetworkEvent::Disconnected { reason });
                return;
            },
        }
    }
}

async fn establish(url: &str, target: &Target) -> Result<Session, ClientError> {
    let client = Client::connect(url).await?;
    // The viewer spawns no agents, so its observation stream is not needed.
    let (session, _observations) = match target {
        Target::Create => {
            let entry = CatalogEntryId::new(CATALOG_ENTRY).expect("the entry ID is non-empty");
            client.create_run(entry).await?
        }
        Target::Join(run_id) => client.join_run(run_id.clone()).await?,
    };
    session.watch().await?;
    Ok(session)
}

async fn execute(session: &Session, command: Command) -> Result<(), ClientError> {
    match command {
        Command::Start => session.start().await,
        Command::ClaimPacing => session.claim_pacing().await,
        Command::SetPacing(mode) => session.set_pacing(mode).await,
        Command::StepOnce => session.step_once().await,
        Command::Close => session.close().await,
    }
}

fn describe_close(reason: CloseReason) -> &'static str {
    match reason {
        CloseReason::ClosedByCreator => "closed by its creator",
        CloseReason::CreatorExpired => "its creator disconnected before starting it",
        CloseReason::CreatorLeft => "its creator left before starting it",
        CloseReason::Other => "for a reason this viewer does not know",
    }
}
