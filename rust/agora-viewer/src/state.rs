//! What the viewer knows about its run, refreshed from the network bridge each frame.

use agora_client::SessionInfo;
use agora_client::protocol::{Pacing, View};
use bevy::prelude::*;
use tokio::sync::watch;

use crate::network::{Command, Network, NetworkEvent};

/// The bridge, as a Bevy resource.
#[derive(Resource)]
pub struct NetworkLink(pub Network);

/// The viewer's knowledge of its run.
#[derive(Resource, Default)]
pub struct ViewerState {
    pub connection: Connection,
    /// Whether this viewer created the run, and so may start it.
    pub creator: bool,
    /// The newest view. `None` until watching.
    pub view: Option<View>,
    /// The newest pacing state. `None` until watching.
    pub pacing: Option<Pacing>,
    /// The most recent failure to show the user.
    pub message: Option<String>,
    handles: Option<Handles>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum Connection {
    #[default]
    Connecting,
    Watching(SessionInfo),
    Disconnected(String),
}

struct Handles {
    view: watch::Receiver<Option<View>>,
    pacing: watch::Receiver<Option<Pacing>>,
}

impl ViewerState {
    /// Take a network event into account.
    pub fn apply(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::Watching {
                info,
                creator,
                view,
                pacing,
            } => {
                self.connection = Connection::Watching(info);
                self.creator = creator;
                self.handles = Some(Handles { view, pacing });
            }
            NetworkEvent::CommandFailed { command, error } => {
                self.message = Some(format!("{} failed: {error}", describe(command)));
            }
            NetworkEvent::Disconnected { reason } => {
                self.connection = Connection::Disconnected(reason);
            }
        }
    }

    /// Copy the newest view and pacing state from the latest-value handles. Returns whether
    /// the view changed.
    pub fn refresh(&mut self) -> bool {
        let Some(handles) = &mut self.handles else {
            return false;
        };
        if handles.pacing.has_changed().unwrap_or(false) {
            self.pacing = *handles.pacing.borrow_and_update();
        }
        if handles.view.has_changed().unwrap_or(false) {
            self.view = handles.view.borrow_and_update().clone();
            return true;
        }
        false
    }
}

fn describe(command: Command) -> &'static str {
    match command {
        Command::Start => "Start",
        Command::ClaimPacing => "Claiming pacing control",
        Command::SetPacing(_) => "Changing pacing",
        Command::StepOnce => "Step",
    }
}

/// Signals that a new view arrived this frame.
#[derive(Message)]
pub struct ViewChanged;

/// Drain network events and refresh the view and pacing state.
pub fn poll_network(
    mut link: ResMut<NetworkLink>,
    mut state: ResMut<ViewerState>,
    mut changed: MessageWriter<ViewChanged>,
) {
    while let Ok(event) = link.0.events.try_recv() {
        state.apply(event);
    }
    if state.refresh() {
        changed.write(ViewChanged);
    }
}
