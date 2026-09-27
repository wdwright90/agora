//! Drawing the run: the grid, one entity per agent synced to each complete view, movement
//! eased between cells, and a camera that fits the grid beside the control panel. Rendering is
//! driven by the view's data, not by knowledge of simulation types (ADR-002).

use std::collections::HashMap;
use std::time::Duration;

use agora_client::protocol::{AgentId, PacingMode, View};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::state::{ViewChanged, ViewerState};

/// World units per grid cell.
pub const CELL: f32 = 32.0;
/// Logical pixels on the left kept clear for the control panel.
pub const PANEL_WIDTH: f32 = 280.0;
/// The longest a move takes to animate. Shorter step intervals shorten it.
const MAX_MOVE: Duration = Duration::from_millis(300);

/// Grid size in cells, once known.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize(pub Option<(u32, u32)>);

/// Agent entities by agent ID.
#[derive(Resource, Default)]
pub struct AgentEntities(pub HashMap<AgentId, Entity>);

/// An agent's sprite.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentMarker(pub AgentId);

/// A grid cell's sprite.
#[derive(Component)]
pub struct CellMarker;

/// An agent's movement between cells, eased over `duration`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub from: Vec2,
    pub to: Vec2,
    pub elapsed: Duration,
    pub duration: Duration,
}

/// The world position of the centre of cell `(x, y)` in a `width` × `height` grid, with the
/// grid centred on the origin. `y` grows north, as in the protocol.
pub fn cell_center(x: u32, y: u32, (width, height): (u32, u32)) -> Vec2 {
    Vec2::new(
        (x as f32 - (width as f32 - 1.0) / 2.0) * CELL,
        (y as f32 - (height as f32 - 1.0) / 2.0) * CELL,
    )
}

/// A stable colour for an agent, spread around the hue circle by its ID.
pub fn agent_color(agent: AgentId) -> Color {
    let hue = (agent.get() as f32 * 0.618_034).fract() * 360.0;
    Color::hsl(hue, 0.7, 0.55)
}

/// How long a move takes to animate under `pacing`: at most [`MAX_MOVE`], and less than the
/// interval so a move finishes before the next one starts. Unlimited pacing snaps.
fn move_duration(state: &ViewerState) -> Duration {
    match state.pacing.map(|p| p.mode) {
        Some(PacingMode::Interval { ms }) => MAX_MOVE.min(ms.as_duration().mul_f32(0.8)),
        Some(PacingMode::Paused) => MAX_MOVE,
        Some(PacingMode::Unlimited) | None => Duration::ZERO,
    }
}

/// Bring the grid and agent entities in line with the newest view.
pub fn sync_view(
    mut commands: Commands,
    mut changed: MessageReader<ViewChanged>,
    state: Res<ViewerState>,
    mut grid: ResMut<GridSize>,
    mut agents: ResMut<AgentEntities>,
    mut motions: Query<(&Transform, &mut Motion)>,
) {
    if changed.read().count() == 0 {
        return;
    }
    let Some(view) = &state.view else {
        return;
    };
    let size = (view.width, view.height);
    if grid.0 != Some(size) {
        spawn_grid(&mut commands, size);
        grid.0 = Some(size);
    }
    sync_agents(
        &mut commands,
        view,
        &mut agents,
        &mut motions,
        move_duration(&state),
    );
}

fn spawn_grid(commands: &mut Commands, size: (u32, u32)) {
    let (width, height) = size;
    for y in 0..height {
        for x in 0..width {
            let shade = if (x + y) % 2 == 0 { 0.16 } else { 0.19 };
            commands.spawn((
                CellMarker,
                Sprite::from_color(Color::srgb(shade, shade, shade), Vec2::splat(CELL - 1.0)),
                Transform::from_translation(cell_center(x, y, size).extend(0.0)),
            ));
        }
    }
}

fn sync_agents(
    commands: &mut Commands,
    view: &View,
    agents: &mut AgentEntities,
    motions: &mut Query<(&Transform, &mut Motion)>,
    duration: Duration,
) {
    let size = (view.width, view.height);
    let present: Vec<AgentId> = view.agents.iter().map(|a| a.agent_id).collect();
    agents.0.retain(|id, entity| {
        let keep = present.contains(id);
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });
    for agent in &view.agents {
        let target = cell_center(agent.x, agent.y, size);
        match agents.0.get(&agent.agent_id) {
            Some(&entity) => {
                if let Ok((transform, mut motion)) = motions.get_mut(entity)
                    && motion.to != target
                {
                    *motion = Motion {
                        from: transform.translation.truncate(),
                        to: target,
                        elapsed: Duration::ZERO,
                        duration,
                    };
                }
            }
            None => {
                let entity = commands
                    .spawn((
                        AgentMarker(agent.agent_id),
                        Sprite::from_color(agent_color(agent.agent_id), Vec2::splat(CELL * 0.7)),
                        Transform::from_translation(target.extend(1.0)),
                        Motion {
                            from: target,
                            to: target,
                            elapsed: Duration::ZERO,
                            duration: Duration::ZERO,
                        },
                        children![(
                            Text2d::new(agent.agent_id.get().to_string()),
                            TextFont::from_font_size(12.0),
                            TextColor(Color::BLACK),
                            Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                        )],
                    ))
                    .id();
                agents.0.insert(agent.agent_id, entity);
            }
        }
    }
}

/// Ease each agent from its previous cell to its current one.
pub fn animate(time: Res<Time>, mut query: Query<(&mut Transform, &mut Motion)>) {
    for (mut transform, mut motion) in &mut query {
        motion.elapsed += time.delta();
        let t = if motion.duration.is_zero() {
            1.0
        } else {
            (motion.elapsed.as_secs_f32() / motion.duration.as_secs_f32()).min(1.0)
        };
        let eased = t * t * (3.0 - 2.0 * t);
        let position = motion.from.lerp(motion.to, eased);
        transform.translation.x = position.x;
        transform.translation.y = position.y;
    }
}

/// Scale and offset the camera so the whole grid fits in the area right of the panel.
pub fn fit_camera(
    grid: Res<GridSize>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&mut Projection, &mut Transform), With<Camera2d>>,
) {
    let Some((width, height)) = grid.0 else {
        return;
    };
    let (mut projection, mut transform) = camera.into_inner();
    let Projection::Orthographic(ortho) = projection.as_mut() else {
        return;
    };
    let area = Vec2::new(
        (window.width() - PANEL_WIDTH).max(1.0),
        window.height().max(1.0),
    );
    let extent = Vec2::new(width as f32, height as f32) * CELL * 1.1;
    let scale = (extent.x / area.x).max(extent.y / area.y);
    ortho.scale = scale;
    // Shift the view left so the grid is centred in the area beside the panel.
    transform.translation.x = -PANEL_WIDTH / 2.0 * scale;
}
