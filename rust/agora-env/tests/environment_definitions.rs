//! Tests for SPEC-007 (`docs/components/simulation/specs/environment-definitions.md`).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use agora_env::{
    DefinitionError, Environment, EnvironmentError, Kind, KindId, KindRegistry, Layout,
    LayoutError, Look, builtin, bundled,
};

fn id(id: &str) -> KindId {
    KindId::new(id).unwrap()
}

fn kinds() -> Arc<KindRegistry> {
    Arc::new(builtin::registry())
}

/// The built-in kinds, with the agent kind given `look`, or left out if `look` is `None`.
fn kinds_with_agent(look: Option<Look>) -> Arc<KindRegistry> {
    let mut kinds: Vec<Kind> = builtin::registry()
        .iter()
        .filter(|kind| kind.id != builtin::agent_kind())
        .cloned()
        .collect();
    kinds.extend(look.map(|look| Kind {
        id: builtin::agent_kind(),
        look,
    }));
    Arc::new(KindRegistry::new(kinds).unwrap())
}

/// A layout file with a floor-and-wall legend.
fn definition(map: &str) -> String {
    format!("map = \"\"\"\n{map}\"\"\"\n\n[legend]\n\".\" = \"floor\"\n\"#\" = \"wall\"\n")
}

fn load(source: &str) -> Result<Layout, DefinitionError> {
    Layout::from_toml(source, &kinds())
}

fn floor() -> Arc<Layout> {
    Arc::new(Layout::filled(2, 2, id(builtin::FLOOR)).unwrap())
}

fn layouts() -> BTreeMap<String, Arc<Layout>> {
    BTreeMap::from([("floor".to_owned(), floor())])
}

fn load_environment(source: &str) -> Result<Environment, DefinitionError> {
    Environment::from_toml(source, kinds(), &layouts())
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
fn r02_environment_checks_layout_kinds() {
    let env = |layout: Layout| Environment::new(kinds(), Arc::new(layout));

    let ok = Environment::new(kinds(), floor()).unwrap();
    assert_eq!(ok.layout(), &*floor());
    assert_eq!(**ok.kinds(), builtin::registry());

    assert_eq!(
        env(Layout::filled(2, 2, id("lava")).unwrap()),
        Err(EnvironmentError::UnknownTerrainKind(id("lava")))
    );
    assert_eq!(
        env(Layout::filled(2, 2, id(builtin::BERRY)).unwrap()),
        Err(EnvironmentError::NotTerrain(id(builtin::BERRY)))
    );
}

#[test]
fn r02_the_registry_must_declare_the_agent_kind_as_a_creature() {
    assert_eq!(
        Environment::new(kinds_with_agent(None), floor()),
        Err(EnvironmentError::MissingAgentKind(builtin::agent_kind()))
    );
    let berry = builtin::registry().get(&id(builtin::BERRY)).unwrap().look;
    assert_eq!(
        Environment::new(kinds_with_agent(Some(berry)), floor()),
        Err(EnvironmentError::AgentKindNotCreature(builtin::agent_kind()))
    );
}

#[test]
fn r03_r05_a_layout_file_loads_with_the_first_line_north() {
    let layout = load(&definition("#..\n..#\n")).unwrap();
    assert_eq!((layout.width(), layout.height()), (3, 2));
    // The first map line is the northern row, y = 1.
    assert_eq!(layout.get(0, 1), Some(&id(builtin::WALL)));
    assert_eq!(layout.get(2, 0), Some(&id(builtin::WALL)));
    assert_eq!(layout.get(0, 0), Some(&id(builtin::FLOOR)));
}

#[test]
fn r03_invalid_toml_and_wrong_fields_are_rejected() {
    let unknown_field = format!("colour = \"blue\"\n{}", definition("..\n"));
    let missing_map = "[legend]\n\".\" = \"floor\"\n";
    let missing_legend = "map = \"..\"\n";
    let mistyped = definition("..\n").replace("\"wall\"", "3");
    // The format before layouts and environments were separate files.
    let combined =
        "agent_kind = \"agent\"\n[layout]\nmap = \"..\"\n[layout.legend]\n\".\" = \"floor\"\n";
    for source in [
        "not toml at all [",
        &unknown_field,
        missing_map,
        missing_legend,
        &mistyped,
        combined,
    ] {
        assert!(
            matches!(load(source), Err(DefinitionError::Toml(_))),
            "{source}"
        );
    }
}

#[test]
fn r03_an_empty_kind_id_is_rejected() {
    let empty = definition("..\n").replace("\"wall\"", "\"\"");
    assert!(matches!(load(&empty), Err(DefinitionError::EmptyKindId(_))));
}

#[test]
fn r04_legend_keys_are_single_characters_of_any_script() {
    let source = "map = \"\"\"\n█·\n\"\"\"\n[legend]\n\"█\" = \"wall\"\n\"·\" = \"floor\"\n";
    let layout = load(source).unwrap();
    assert_eq!(layout.get(0, 0), Some(&id(builtin::WALL)));
    assert_eq!(layout.get(1, 0), Some(&id(builtin::FLOOR)));

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
fn r06_bundled_files_load_with_the_built_in_kinds() {
    let ids = |files: &[(&'static str, &str)]| files.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    assert_eq!(ids(bundled::LAYOUTS), ["empty-grid-10x10", "divided-10x10"]);
    assert_eq!(
        ids(bundled::ENVIRONMENTS),
        ["empty-grid-10x10", "divided-10x10"]
    );
    let environments = bundled::environments(kinds()).unwrap();
    let loaded: Vec<&str> = environments.iter().map(|(id, _)| *id).collect();
    assert_eq!(loaded, ids(bundled::ENVIRONMENTS));
}

#[test]
fn r06_every_file_in_the_bundled_directories_is_bundled() {
    let entries = |dir: &str, files: bool| -> BTreeSet<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| entry.file_type().unwrap().is_file() == files)
            .map(|entry| entry.file_name().into_string().unwrap())
            .collect()
    };
    let named = |files: &[(&str, &str)]| -> BTreeSet<String> {
        files.iter().map(|(id, _)| format!("{id}.toml")).collect()
    };
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/environments");
    let layouts = format!("{dir}/layouts");
    assert_eq!(entries(dir, true), named(bundled::ENVIRONMENTS));
    assert_eq!(entries(&layouts, true), named(bundled::LAYOUTS));
    assert_eq!(entries(dir, false), BTreeSet::from(["layouts".to_owned()]));
    assert!(entries(&layouts, false).is_empty());
}

#[test]
fn r06_bundled_environments_use_their_layouts() {
    let environments = bundled::environments(kinds()).unwrap();
    let layout = |entry: &str| {
        let (_, environment) = environments.iter().find(|(id, _)| *id == entry).unwrap();
        environment.layout().clone()
    };
    let wall = id(builtin::WALL);

    assert_eq!(
        layout("empty-grid-10x10"),
        Layout::filled(10, 10, id(builtin::FLOOR)).unwrap()
    );

    let divided = layout("divided-10x10");
    assert_eq!((divided.width(), divided.height()), (10, 10));
    let walls: BTreeSet<(u32, u32)> = (0..10)
        .flat_map(|y| (0..10).map(move |x| (x, y)))
        .filter(|&(x, y)| divided.get(x, y) == Some(&wall))
        .collect();
    let expected: BTreeSet<(u32, u32)> = (0..10)
        .filter(|y| !(4..=5).contains(y))
        .map(|y| (5, y))
        .collect();
    assert_eq!(walls, expected);
}

#[test]
fn r06_a_bundled_failure_names_the_file() {
    // Without the agent kind, every layout loads but the first environment fails.
    let error = bundled::environments(kinds_with_agent(None)).unwrap_err();
    assert!(
        matches!(
            &error,
            bundled::BundledError::Environment {
                id: "empty-grid-10x10",
                ..
            }
        ),
        "{error}"
    );
    assert!(error.to_string().contains("empty-grid-10x10"), "{error}");
}

#[test]
fn r07_an_environment_file_names_its_layout() {
    let env = load_environment("layout = \"floor\"\n").unwrap();
    assert_eq!(env.layout(), &*floor());
    assert_eq!(**env.kinds(), builtin::registry());
}

#[test]
fn r07_unknown_and_empty_layout_ids_are_rejected() {
    for missing in ["walls", ""] {
        let source = format!("layout = \"{missing}\"\n");
        assert!(
            matches!(
                load_environment(&source),
                Err(DefinitionError::UnknownLayout(l)) if l == missing
            ),
            "{missing:?}"
        );
    }
}

#[test]
fn r07_invalid_toml_and_wrong_fields_are_rejected() {
    for source in [
        "not toml at all [",
        "",
        "layout = 3\n",
        "layout = \"floor\"\nagent_kind = \"agent\"\n",
        "layout = \"floor\"\necology = \"berries\"\n",
    ] {
        assert!(
            matches!(load_environment(source), Err(DefinitionError::Toml(_))),
            "{source:?}"
        );
    }
}

#[test]
fn r07_the_loaded_environment_is_checked() {
    let berries = Arc::new(Layout::filled(2, 2, id(builtin::BERRY)).unwrap());
    let layouts = BTreeMap::from([("berries".to_owned(), berries)]);
    assert!(matches!(
        Environment::from_toml("layout = \"berries\"\n", kinds(), &layouts),
        Err(DefinitionError::Environment(EnvironmentError::NotTerrain(k))) if k == id(builtin::BERRY)
    ));
}
