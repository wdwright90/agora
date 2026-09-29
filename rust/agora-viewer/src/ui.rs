//! The control panel.

use agora_client::protocol::{IntervalMs, PacingMode, RunPhase};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::controls::controls;
use crate::network::Command;
use crate::render::PANEL_WIDTH;
use crate::state::{Connection, NetworkLink, ViewerState};

/// The interval the slider shows, in milliseconds.
#[derive(Resource)]
pub struct IntervalSetting(pub u64);

impl Default for IntervalSetting {
    fn default() -> Self {
        Self(500)
    }
}

pub fn panel(
    mut contexts: EguiContexts,
    link: Res<NetworkLink>,
    mut state: ResMut<ViewerState>,
    mut interval: ResMut<IntervalSetting>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let send = |command| {
        // The network thread only stops after Bevy exits.
        let _ = link.0.commands.send(command);
    };
    egui::Window::new("Agora")
        .anchor(egui::Align2::LEFT_TOP, [8.0, 8.0])
        .fixed_size([PANEL_WIDTH - 32.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            match &state.connection {
                Connection::Connecting => {
                    ui.label("Connecting…");
                }
                Connection::Disconnected(reason) => {
                    ui.colored_label(egui::Color32::LIGHT_RED, format!("Disconnected: {reason}"));
                }
                Connection::Watching(info) => {
                    ui.horizontal(|ui| {
                        ui.label("Run");
                        ui.monospace(info.run_id.as_str());
                    });
                    if ui.button("Copy run ID").clicked() {
                        ui.ctx().copy_text(info.run_id.as_str().to_owned());
                    }
                }
            }
            ui.separator();

            if let Some(view) = &state.view {
                let phase = match view.phase {
                    RunPhase::Setup => "setup",
                    RunPhase::Started => "started",
                };
                ui.label(format!("State {} ({phase})", view.state_id.get()));
                ui.label(format!("{} agents", view.agents.len()));
            }
            if let Some(pacing) = &state.pacing {
                let mode = match pacing.mode {
                    PacingMode::Paused => "paused".to_owned(),
                    PacingMode::Unlimited => "unlimited".to_owned(),
                    PacingMode::Interval { ms } => format!("every {} ms", ms.get()),
                };
                ui.label(format!("Pacing: {mode}"));
                ui.label(if pacing.you_control {
                    "You control pacing"
                } else {
                    "Another viewer may control pacing"
                });
            }
            ui.separator();

            let enabled = controls(state.creator, state.view.as_ref(), state.pacing.as_ref());
            if ui
                .add_enabled(enabled.start, egui::Button::new("Start"))
                .clicked()
            {
                send(Command::Start);
            }
            if enabled.claim && ui.button("Claim pacing control").clicked() {
                send(Command::ClaimPacing);
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(enabled.pause, egui::Button::new("Pause"))
                    .clicked()
                {
                    send(Command::SetPacing(PacingMode::Paused));
                }
                if ui
                    .add_enabled(enabled.resume, egui::Button::new("Resume"))
                    .clicked()
                {
                    send(Command::SetPacing(interval_mode(interval.0)));
                }
                if ui
                    .add_enabled(enabled.step, egui::Button::new("Step"))
                    .clicked()
                {
                    send(Command::StepOnce);
                }
            });
            let slider = ui.add_enabled(
                enabled.pacing,
                egui::Slider::new(&mut interval.0, 20..=5000)
                    .logarithmic(true)
                    .suffix(" ms")
                    .text("interval"),
            );
            if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
                send(Command::SetPacing(interval_mode(interval.0)));
            }
            if ui
                .add_enabled(enabled.pacing, egui::Button::new("Unlimited"))
                .clicked()
            {
                send(Command::SetPacing(PacingMode::Unlimited));
            }

            if enabled.close {
                ui.separator();
                if ui.button("Close run").clicked() {
                    send(Command::Close);
                }
            }

            if let Some(message) = &state.message {
                ui.separator();
                ui.colored_label(egui::Color32::LIGHT_RED, message);
                if ui.small_button("Dismiss").clicked() {
                    state.message = None;
                }
            }
        });
    Ok(())
}

fn interval_mode(ms: u64) -> PacingMode {
    PacingMode::Interval {
        ms: IntervalMs::new(ms.clamp(IntervalMs::MIN, IntervalMs::MAX)).expect("clamped"),
    }
}
