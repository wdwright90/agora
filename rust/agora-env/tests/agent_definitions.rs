//! Tests for SPEC-008 (`docs/components/simulation/specs/agent-definitions.md`).

use std::collections::BTreeSet;

use agora_env::{AgentDefinition, AgentDefinitionError, Metabolism, bundled};

fn metabolism(decay: f64, rate: f64) -> AgentDefinition {
    AgentDefinition {
        metabolism: Some(Metabolism { decay, rate }),
    }
}

#[test]
fn r01_an_agent_has_only_the_capabilities_its_file_lists() {
    let source = "[metabolism]\ndecay = 0.25\nrate = 2.0\n";
    assert_eq!(
        AgentDefinition::from_toml(source).unwrap(),
        metabolism(0.25, 2.0)
    );
    // An empty file is an agent with no capabilities.
    let bare = AgentDefinition::from_toml("").unwrap();
    assert_eq!(bare, AgentDefinition::default());
    assert_eq!(bare.metabolism, None);
}

#[test]
fn r01_a_definition_round_trips_through_its_file() {
    for definition in [AgentDefinition::default(), metabolism(0.005, 1.0)] {
        assert_eq!(
            AgentDefinition::from_toml(&definition.to_toml()).unwrap(),
            definition
        );
    }
}

/// The `(capability, field)` rows of SPEC-008's capability reference table.
fn documented_fields() -> BTreeSet<(String, String)> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/components/simulation/specs/agent-definitions.md"
    );
    let spec = std::fs::read_to_string(path).expect("SPEC-008 is readable");
    spec.lines()
        .map(str::trim_start)
        .skip_while(|line| !line.starts_with("### Capability reference"))
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .skip(2)
        .map(|row| {
            let cells: Vec<String> = row
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_owned())
                .collect();
            (cells[1].clone(), cells[2].clone())
        })
        .collect()
}

#[test]
fn r02_the_capability_reference_lists_every_supported_field() {
    // A struct literal names every capability, so adding one fails to compile here until it
    // is added, and the comparison then requires it to be documented.
    let every = AgentDefinition {
        metabolism: Some(Metabolism {
            decay: 0.0,
            rate: 0.0,
        }),
    };
    let file: toml::Table = toml::from_str(&every.to_toml()).unwrap();
    let supported: BTreeSet<(String, String)> = file
        .iter()
        .flat_map(|(capability, table)| {
            table
                .as_table()
                .expect("each capability is a table")
                .keys()
                .map(move |field| (capability.clone(), field.clone()))
        })
        .collect();
    assert_eq!(documented_fields(), supported);
}

#[test]
fn r03_unknown_capabilities_and_fields_name_the_supported_ones() {
    let unknown_capability = AgentDefinition::from_toml("[sight]\nradius = 5\n").unwrap_err();
    let message = unknown_capability.to_string();
    assert!(matches!(unknown_capability, AgentDefinitionError::Toml(_)));
    assert!(
        message.contains("sight") && message.contains("metabolism"),
        "{message}"
    );

    let unknown_field =
        AgentDefinition::from_toml("[metabolism]\ndecy = 0.1\nrate = 1.0\n").unwrap_err();
    let message = unknown_field.to_string();
    assert!(
        message.contains("decy") && message.contains("decay") && message.contains("rate"),
        "{message}"
    );
}

#[test]
fn r03_missing_and_mistyped_fields_are_rejected() {
    for source in [
        "[metabolism]\ndecay = 0.1\n",
        "[metabolism]\nrate = 1.0\n",
        "[metabolism]\ndecay = \"slow\"\nrate = 1.0\n",
        "metabolism = 1\n",
        "not toml at all [",
    ] {
        assert!(
            matches!(
                AgentDefinition::from_toml(source),
                Err(AgentDefinitionError::Toml(_))
            ),
            "{source:?}"
        );
    }
}

#[test]
fn r04_values_must_be_finite_and_at_least_zero() {
    let load = |decay: &str, rate: &str| {
        AgentDefinition::from_toml(&format!("[metabolism]\ndecay = {decay}\nrate = {rate}\n"))
    };
    assert_eq!(load("0", "0").unwrap(), metabolism(0.0, 0.0));
    assert_eq!(load("0.0", "5.5").unwrap(), metabolism(0.0, 5.5));
    // TOML integers are accepted as numbers.
    assert_eq!(load("1", "2").unwrap(), metabolism(1.0, 2.0));
    for (decay, rate, field) in [
        ("-0.1", "1.0", "decay"),
        ("0.1", "-1", "rate"),
        ("nan", "1.0", "decay"),
        ("0.1", "inf", "rate"),
        ("-inf", "1.0", "decay"),
    ] {
        let error = load(decay, rate).unwrap_err();
        assert!(
            matches!(
                &error,
                AgentDefinitionError::OutOfRange { capability: "metabolism", field: f, .. } if *f == field
            ),
            "{decay}, {rate}: {error}"
        );
    }
    assert!(metabolism(-1.0, 1.0).check().is_err());
    assert!(metabolism(0.5, 1.0).check().is_ok());
}

#[test]
fn r05_the_forager_is_bundled_with_a_metabolism() {
    let ids: Vec<&str> = bundled::AGENTS.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, ["forager"]);
    let agents = bundled::agents().unwrap();
    assert_eq!(agents, [("forager", metabolism(0.005, 1.0))]);
}

#[test]
fn r05_every_file_in_the_agents_directory_is_bundled() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/agents");
    let files: BTreeSet<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap())
        .inspect(|entry| assert!(entry.file_type().unwrap().is_file()))
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect();
    let named: BTreeSet<String> = bundled::AGENTS
        .iter()
        .map(|(id, _)| format!("{id}.toml"))
        .collect();
    assert_eq!(files, named);
}
