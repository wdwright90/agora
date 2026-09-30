//! The bundled environment catalog (SPEC-003-R01).

use std::sync::Arc;

use agora_env::{KindId, KindRegistry, builtin};
use agora_protocol::CatalogEntryId;

/// ID of the only bundled entry: an empty 10 × 10 grid.
pub const EMPTY_GRID_10X10: &str = "empty-grid-10x10";

/// What a catalog entry builds.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogEntry {
    pub width: u32,
    pub height: u32,
    /// Every kind in the run.
    pub kinds: Arc<KindRegistry>,
    /// The kind given to every agent.
    pub agent_kind: KindId,
}

/// Look up a catalog entry by ID.
pub fn lookup(id: &CatalogEntryId) -> Option<CatalogEntry> {
    (id.as_str() == EMPTY_GRID_10X10).then(|| CatalogEntry {
        width: 10,
        height: 10,
        kinds: Arc::new(builtin::registry()),
        agent_kind: builtin::agent_kind(),
    })
}
