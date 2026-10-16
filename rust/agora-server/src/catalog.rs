//! The bundled catalogs (SPEC-003-R01): environments built from `agora-env`'s bundled
//! environment files, and agent definitions from its bundled agent files.

use std::sync::{Arc, LazyLock};

use agora_env::{AgentDefinition, Environment, builtin, bundled};
use agora_protocol::{AgentDefinitionId, CatalogEntryId};

/// An open 10 x 10 floor.
pub const EMPTY_GRID_10X10: &str = "empty-grid-10x10";
/// A 10 x 10 floor divided by a wall with a gap.
pub const DIVIDED_10X10: &str = "divided-10x10";
/// An agent with only a metabolism.
pub const FORAGER: &str = "forager";

/// Every bundled environment, loaded once with the built-in kinds.
static CATALOG: LazyLock<Vec<(&str, Environment)>> = LazyLock::new(|| {
    bundled::environments(Arc::new(builtin::registry())).unwrap_or_else(|error| panic!("{error}"))
});

/// Every bundled agent definition, loaded once.
static AGENTS: LazyLock<Vec<(&str, AgentDefinition)>> =
    LazyLock::new(|| bundled::agents().unwrap_or_else(|error| panic!("{error}")));

/// Look up a catalog entry by ID.
pub fn lookup(id: &CatalogEntryId) -> Option<Environment> {
    CATALOG
        .iter()
        .find(|(entry, _)| *entry == id.as_str())
        .map(|(_, environment)| environment.clone())
}

/// Look up an agent catalog entry by ID.
pub fn agent(id: &AgentDefinitionId) -> Option<AgentDefinition> {
    AGENTS
        .iter()
        .find(|(entry, _)| *entry == id.as_str())
        .map(|&(_, definition)| definition)
}
