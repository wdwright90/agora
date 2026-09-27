//! Agora viewer: a Bevy window that watches one run, draws its grid and agents, and offers
//! Start and pacing controls.
//!
//! The viewer is an ordinary client of the server (CDD-004, `docs/components/viewer/cdd.md`);
//! it never runs the simulation. Its behavior is specified by SPEC-005
//! (`docs/components/viewer/specs/viewer.md`).

pub mod controls;
pub mod network;
pub mod render;
pub mod state;
mod ui;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

use crate::network::Network;
use crate::render::{AgentEntities, GridSize};
use crate::state::{NetworkLink, ViewChanged, ViewerState};

/// The viewer's systems and resources, without window or rendering setup. Headless tests use
/// this with `MinimalPlugins`.
pub struct ViewerCorePlugin {
    network: std::sync::Mutex<Option<Network>>,
}

impl ViewerCorePlugin {
    pub fn new(network: Network) -> Self {
        Self {
            network: std::sync::Mutex::new(Some(network)),
        }
    }
}

impl Plugin for ViewerCorePlugin {
    fn build(&self, app: &mut App) {
        let network = self
            .network
            .lock()
            .expect("the plugin is built once")
            .take()
            .expect("the plugin is built once");
        app.insert_resource(NetworkLink(network))
            .init_resource::<ViewerState>()
            .init_resource::<GridSize>()
            .init_resource::<AgentEntities>()
            .add_message::<ViewChanged>()
            .add_systems(
                Update,
                (state::poll_network, render::sync_view, render::animate).chain(),
            );
    }
}

/// The full viewer: the core plus the window, camera, and control panel.
pub fn run(network: Network, title: String) -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title, ..default() }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(ViewerCorePlugin::new(network))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.1)))
        .init_resource::<ui::IntervalSetting>()
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Update, render::fit_camera)
        .add_systems(EguiPrimaryContextPass, ui::panel)
        .run()
}
