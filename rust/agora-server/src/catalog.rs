//! The bundled environment catalog (SPEC-003-R01), built from `agora-env`'s bundled
//! definitions.

use std::sync::{Arc, LazyLock};

use agora_env::{Environment, builtin, bundled};
use agora_protocol::CatalogEntryId;

/// An open 10 × 10 floor.
pub const EMPTY_GRID_10X10: &str = "empty-grid-10x10";
/// A 10 × 10 floor divided by a wall with a gap.
pub const DIVIDED_10X10: &str = "divided-10x10";

/// Every bundled environment, loaded once with the built-in kinds.
static CATALOG: LazyLock<Vec<(&str, Environment)>> = LazyLock::new(|| {
    let kinds = Arc::new(builtin::registry());
    bundled::DEFINITIONS
        .iter()
        .map(|&(id, source)| {
            let environment = Environment::from_toml(source, Arc::clone(&kinds))
                .unwrap_or_else(|error| panic!("bundled environment {id} is invalid: {error}"));
            (id, environment)
        })
        .collect()
});

/// Look up a catalog entry by ID.
pub fn lookup(id: &CatalogEntryId) -> Option<Environment> {
    CATALOG
        .iter()
        .find(|(entry, _)| *entry == id.as_str())
        .map(|(_, environment)| environment.clone())
}
