//! Tests for SPEC-007 (`docs/components/simulation/specs/environment-definitions.md`).

use std::collections::BTreeSet;
use std::sync::Arc;

use agora_env::{
    DefinitionError, Environment, EnvironmentError, KindId, KindRegistry, Layout, LayoutError,
    builtin, bundled,
};

fn id(id: &str) -> KindId {
    KindId::new(id).unwrap()
}

fn kinds() -> Arc<KindRegistry> {
    Arc::new(builtin::registry())
}

/// A definition with the built-in agent kind and a floor-and-wall legend.
fn definition(map: &str) -> String {
    format!(
        "agent_kind = \"agent\"\n\n[layout]\nmap = \"\"\"\n{map}\"\"\"\n\n[layout.legend]\n\".\" = \"floor\"\n\"#\" = \"wall\"\n"
    )
}

fn load(source: &str) -> Result<Environment, DefinitionError> {
    Environment::from_toml(source, kinds())
}

#[test]
fn r01_layout_cells_are_row_major_from_the_south_west() {
    let cells = ["a", "b", "c", "d", "e", "f"].map(id).to_vec();
    let layout = Layout::new(3, 2, cells).unwrap();
    assert_eq!((layout.width(), layout.height()), (3, 2));
    assert_eq!(layout.get(0, 0), Some(&id("a")));
    assert_eq!(layout.get(2, 0), Some(&id("c")));
    assert_eq!(layout.get(0, 1), Some(&id("d")));
    assert_eq!(layout.get(3, 0), None);
    assert_eq!(layout.get(0, 2), None);
    assert_eq!(
        Layout::filled(2, 2, id("x")).unwrap().cells(),
        [id("x"), id("x"), id("x"), id("x")]
    );
}

#[test]
fn r01_empty_and_mismatched_layouts_are_rejected() {
    for (width, height) in [(0, 2), (2, 0), (0, 0)] {
        assert_eq!(
            Layout::filled(width, height, id("x")),
            Err(LayoutError::Empty { width, height })
        );
    }
    assert_eq!(
        Layout::new(2, 2, vec![id("x"); 3]),
        Err(LayoutError::WrongCellCount {
            width: 2,
            height: 2,
            expected: 4,
            actual: 3
        })
    );
}

#[test]
fn r02_environment_checks_layout_and_agent_kinds() {
    let floor = || Layout::filled(2, 2, id(builtin::FLOOR)).unwrap();
    let env = |layout, agent: &str| Environment::new(kinds(), layout, id(agent));

    let ok = env(floor(), builtin::AGENT).unwrap();
    assert_eq!(ok.agent_kind(), &id(builtin::AGENT));
    assert_eq!(ok.layout(), &floor());
    assert_eq!(**ok.kinds(), builtin::registry());

    assert_eq!(
        env(Layout::filled(2, 2, id("lava")).unwrap(), builtin::AGENT),
        Err(EnvironmentError::UnknownTerrainKind(id("lava")))
    );
    assert_eq!(
        env(
            Layout::filled(2, 2, id(builtin::BERRY)).unwrap(),
            builtin::AGENT
        ),
        Err(EnvironmentError::NotTerrain(id(builtin::BERRY)))
    );
    assert_eq!(
        env(floor(), "dragon"),
        Err(EnvironmentError::UnknownAgentKind(id("dragon")))
    );
    for not_creature in [builtin::FLOOR, builtin::BERRY] {
        assert_eq!(
            env(floor(), not_creature),
            Err(EnvironmentError::AgentKindNotCreature(id(not_creature)))
        );
    }
}

#[test]
fn r03_r05_a_definition_loads_with_the_first_line_north() {
    let env = load(&definition("#..\n..#\n")).unwrap();
    let layout = env.layout();
    assert_eq!((layout.width(), layout.height()), (3, 2));
    // The first map line is the northern row, y = 1.
    assert_eq!(layout.get(0, 1), Some(&id(builtin::WALL)));
    assert_eq!(layout.get(2, 0), Some(&id(builtin::WALL)));
    assert_eq!(layout.get(0, 0), Some(&id(builtin::FLOOR)));
    assert_eq!(env.agent_kind(), &id(builtin::AGENT));
}

#[test]
fn r03_invalid_toml_and_wrong_fields_are_rejected() {
    let unknown_field = definition("..\n").replace("[layout]\n", "[layout]\nwrap = true\n");
    let unknown_top = format!("colour = \"blue\"\n{}", definition("..\n"));
    let missing_agent = definition("..\n").replace("agent_kind = \"agent\"\n", "");
    let mistyped = definition("..\n").replace("\"agent\"", "3");
    for source in [
        "not toml at all [",
        &unknown_field,
        &unknown_top,
        &missing_agent,
        &mistyped,
    ] {
        assert!(
            matches!(load(source), Err(DefinitionError::Toml(_))),
            "{source}"
        );
    }
}

#[test]
fn r03_the_agent_kind_is_checked() {
    let dragon = definition("..\n").replace("\"agent\"", "\"dragon\"");
    assert!(matches!(
        load(&dragon),
        Err(DefinitionError::Environment(EnvironmentError::UnknownAgentKind(k))) if k == id("dragon")
    ));
    let empty = definition("..\n").replace("\"agent\"", "\"\"");
    assert!(matches!(load(&empty), Err(DefinitionError::EmptyKindId(_))));
}

#[test]
fn r04_legend_keys_are_single_characters_of_any_script() {
    let source = "agent_kind = \"agent\"\n[layout]\nmap = \"\"\"\n█·\n\"\"\"\n[layout.legend]\n\"█\" = \"wall\"\n\"·\" = \"floor\"\n";
    let env = load(source).unwrap();
    assert_eq!(env.layout().get(0, 0), Some(&id(builtin::WALL)));
    assert_eq!(env.layout().get(1, 0), Some(&id(builtin::FLOOR)));

    for key in ["", "..", "ab"] {
        let bad = definition("..\n").replace("\"#\" = ", &format!("\"{key}\" = "));
        assert!(
            matches!(load(&bad), Err(DefinitionError::LegendKey(k)) if k == key),
            "{key:?}"
        );
    }
}

#[test]
fn r04_every_legend_entry_must_be_registered_terrain() {
    // `#` is unused by the map, but its entry is still checked.
    let unknown = definition("..\n").replace("\"wall\"", "\"lava\"");
    assert!(matches!(
        load(&unknown),
        Err(DefinitionError::UnknownLegendKind { symbol: '#', kind }) if kind == id("lava")
    ));
    let item = definition("..\n").replace("\"wall\"", "\"berry\"");
    assert!(matches!(
        load(&item),
        Err(DefinitionError::LegendNotTerrain { symbol: '#', kind }) if kind == id("berry")
    ));
}

#[test]
fn r05_crlf_and_a_missing_final_line_ending_are_accepted() {
    let lf = load(&definition("#.\n..\n")).unwrap();
    let crlf = load(&definition("#.\r\n..\r\n")).unwrap();
    let unterminated = load(&definition("#.\n..")).unwrap();
    assert_eq!(lf, crlf);
    assert_eq!(lf, unterminated);
}

#[test]
fn r05_empty_ragged_and_unknown_maps_are_rejected() {
    assert!(matches!(
        load(&definition("")),
        Err(DefinitionError::EmptyMap)
    ));
    assert!(matches!(
        load(&definition("\n")),
        Err(DefinitionError::Layout(LayoutError::Empty { .. }))
    ));
    assert!(matches!(
        load(&definition("...\n..\n")),
        Err(DefinitionError::RaggedRow {
            row: 2,
            width: 2,
            expected: 3
        })
    ));
    assert!(matches!(
        load(&definition("..\n.x\n")),
        Err(DefinitionError::UnknownSymbol {
            row: 2,
            column: 2,
            symbol: 'x'
        })
    ));
}

#[test]
fn r06_bundled_definitions_load_with_the_built_in_kinds() {
    let ids: Vec<&str> = bundled::DEFINITIONS.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, ["empty-grid-10x10", "divided-10x10"]);
    for (entry, source) in bundled::DEFINITIONS {
        let env = Environment::from_toml(source, kinds())
            .unwrap_or_else(|e| panic!("{entry} does not load: {e}"));
        assert_eq!(env.agent_kind(), &builtin::agent_kind(), "{entry}");
    }
}

#[test]
fn r06_every_file_in_the_environments_directory_is_bundled() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/environments");
    let files: BTreeSet<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    let bundled: BTreeSet<String> = bundled::DEFINITIONS
        .iter()
        .map(|(id, _)| format!("{id}.toml"))
        .collect();
    assert_eq!(files, bundled);
}

#[test]
fn r06_bundled_layouts() {
    let load = |entry: &str| {
        let (_, source) = bundled::DEFINITIONS
            .iter()
            .find(|(id, _)| *id == entry)
            .unwrap();
        Environment::from_toml(source, kinds()).unwrap()
    };
    let wall = id(builtin::WALL);

    let empty = load("empty-grid-10x10");
    assert_eq!(
        empty.layout(),
        &Layout::filled(10, 10, id(builtin::FLOOR)).unwrap()
    );

    let divided = load("divided-10x10");
    let layout = divided.layout();
    assert_eq!((layout.width(), layout.height()), (10, 10));
    let walls: BTreeSet<(u32, u32)> = (0..10)
        .flat_map(|y| (0..10).map(move |x| (x, y)))
        .filter(|&(x, y)| layout.get(x, y) == Some(&wall))
        .collect();
    let expected: BTreeSet<(u32, u32)> = (0..10)
        .filter(|y| !(4..=5).contains(y))
        .map(|y| (5, y))
        .collect();
    assert_eq!(walls, expected);
}
