//! The built-in kinds used by the bundled catalog (SPEC-006-R04).

use crate::kinds::{
    Appearance, Kind, KindId, KindRegistry, Look, Shape, Terrain, TerrainClass, Unit,
};

pub const FLOOR: &str = "floor";
pub const WALL: &str = "wall";
pub const BERRY: &str = "berry";
/// The kind every agent has until species exist.
pub const AGENT: &str = "agent";

/// The built-in registry: floor, wall, berry, and agent, in that order.
pub fn registry() -> KindRegistry {
    let terrain = |id, class, blocks| Kind {
        id: id_of(id),
        look: Look::Terrain(Terrain {
            class,
            blocks_movement: blocks,
            blocks_sight: blocks,
        }),
    };
    let look = |hue, size, shape| Appearance {
        hue: Unit::new(hue).expect("built-in hues are in range"),
        size: Unit::new(size).expect("built-in sizes are in range"),
        shape,
    };
    KindRegistry::new(vec![
        terrain(FLOOR, TerrainClass::Floor, false),
        terrain(WALL, TerrainClass::Wall, true),
        Kind {
            id: id_of(BERRY),
            look: Look::Item(look(0.02, 0.3, Shape::Round)),
        },
        Kind {
            id: id_of(AGENT),
            look: Look::Creature(look(0.6, 0.5, Shape::Agent)),
        },
    ])
    .expect("built-in kind IDs are unique")
}

/// The built-in agent kind's ID.
pub fn agent_kind() -> KindId {
    id_of(AGENT)
}

fn id_of(id: &str) -> KindId {
    KindId::new(id).expect("built-in kind IDs are non-empty")
}
