//! The bundled environment catalog (SPEC-003-R01).

use agora_protocol::CatalogEntryId;

/// ID of the only bundled entry: an empty 10 × 10 grid.
pub const EMPTY_GRID_10X10: &str = "empty-grid-10x10";

/// What a catalog entry builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogEntry {
    pub width: u32,
    pub height: u32,
}

/// Look up a catalog entry by ID.
pub fn lookup(id: &CatalogEntryId) -> Option<CatalogEntry> {
    (id.as_str() == EMPTY_GRID_10X10).then_some(CatalogEntry {
        width: 10,
        height: 10,
    })
}
