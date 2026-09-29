//! Tests for SPEC-006 (`docs/components/simulation/specs/kinds-and-appearance.md`).

use agora_env::{
    Appearance, EmptyKindId, Kind, KindId, KindRegistry, Look, RegistryError, Shape, Terrain,
    TerrainClass, Unit, UnitOutOfRange, builtin,
};

fn id(id: &str) -> KindId {
    KindId::new(id).unwrap()
}

fn floor(name: &str) -> Kind {
    Kind {
        id: id(name),
        look: Look::Terrain(Terrain {
            class: TerrainClass::Floor,
            blocks_movement: false,
            blocks_sight: false,
        }),
    }
}

#[test]
fn r01_registry_keeps_declaration_order_and_finds_kinds() {
    let registry = KindRegistry::new(vec![floor("b"), floor("a"), floor("c")]).unwrap();
    let ids: Vec<&str> = registry.iter().map(|k| k.id.as_str()).collect();
    assert_eq!(ids, ["b", "a", "c"]);
    assert_eq!(registry.len(), 3);
    assert_eq!(registry.get(&id("a")), Some(&floor("a")));
    assert_eq!(registry.get(&id("missing")), None);
}

#[test]
fn r01_duplicate_kind_ids_are_rejected() {
    assert_eq!(
        KindRegistry::new(vec![floor("a"), floor("b"), floor("a")]),
        Err(RegistryError::DuplicateKind(id("a")))
    );
}

#[test]
fn r01_empty_kind_ids_are_rejected() {
    assert_eq!(KindId::new(""), Err(EmptyKindId));
    assert_eq!(id("wall").as_str(), "wall");
}

#[test]
fn r03_units_accept_zero_to_one_inclusive() {
    for value in [0.0, 0.5, 1.0] {
        assert_eq!(Unit::new(value).unwrap().get(), value);
    }
    for value in [-0.01, 1.01, f64::NAN, f64::INFINITY] {
        assert!(Unit::new(value).is_err(), "{value} should be rejected");
    }
    assert_eq!(Unit::new(2.0), Err(UnitOutOfRange(2.0)));
}

#[test]
fn r04_builtin_registry_declares_floor_wall_berry_and_agent() {
    let registry = builtin::registry();
    let unit = |v| Unit::new(v).unwrap();
    let expected = [
        Kind {
            id: id("floor"),
            look: Look::Terrain(Terrain {
                class: TerrainClass::Floor,
                blocks_movement: false,
                blocks_sight: false,
            }),
        },
        Kind {
            id: id("wall"),
            look: Look::Terrain(Terrain {
                class: TerrainClass::Wall,
                blocks_movement: true,
                blocks_sight: true,
            }),
        },
        Kind {
            id: id("berry"),
            look: Look::Item(Appearance {
                hue: unit(0.02),
                size: unit(0.3),
                shape: Shape::Round,
            }),
        },
        Kind {
            id: id("agent"),
            look: Look::Creature(Appearance {
                hue: unit(0.6),
                size: unit(0.5),
                shape: Shape::Agent,
            }),
        },
    ];
    assert_eq!(registry.iter().cloned().collect::<Vec<_>>(), expected);
    assert_eq!(builtin::agent_kind(), id("agent"));
}
