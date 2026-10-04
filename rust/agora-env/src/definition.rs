//! Layout and environment files in TOML (SPEC-007).

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Deserialize;
use thiserror::Error;

use crate::environment::{Environment, EnvironmentError};
use crate::kinds::{EmptyKindId, KindId, KindRegistry, Look};
use crate::layout::{Layout, LayoutError};

/// Why a layout or environment file could not be loaded.
#[derive(Debug, Clone, Error)]
pub enum DefinitionError {
    /// Not TOML, or not the file's shape: a missing, unknown, or mistyped field.
    #[error("invalid file: {0}")]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    EmptyKindId(#[from] EmptyKindId),
    #[error("legend key {0:?} is not a single character")]
    LegendKey(String),
    #[error("legend maps {symbol:?} to {kind}, which is not in the kind registry")]
    UnknownLegendKind { symbol: char, kind: KindId },
    #[error("legend maps {symbol:?} to {kind}, which is not terrain")]
    LegendNotTerrain { symbol: char, kind: KindId },
    #[error("the map has no rows")]
    EmptyMap,
    #[error("map row {row} is {width} characters wide, but row 1 is {expected}")]
    RaggedRow {
        row: usize,
        width: usize,
        expected: usize,
    },
    #[error("map row {row}, column {column}: {symbol:?} is not in the legend")]
    UnknownSymbol {
        row: usize,
        column: usize,
        symbol: char,
    },
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error("layout {0:?} does not exist")]
    UnknownLayout(String),
    #[error(transparent)]
    Environment(#[from] EnvironmentError),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutFile {
    map: String,
    legend: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvironmentFile {
    layout: String,
}

impl Layout {
    /// Load a layout from a TOML layout file whose legend names terrain kinds in `kinds`.
    ///
    /// The map's first line is the northern row, and each character is one cell, west to east.
    pub fn from_toml(source: &str, kinds: &KindRegistry) -> Result<Self, DefinitionError> {
        let file: LayoutFile = toml::from_str(source)?;
        let legend = legend(file.legend, kinds)?;
        layout(&file.map, &legend)
    }
}

impl Environment {
    /// Load an environment from a TOML environment file. Its layout is looked up by ID in
    /// `layouts`, which were loaded with the same `kinds`.
    pub fn from_toml(
        source: &str,
        kinds: Arc<KindRegistry>,
        layouts: &BTreeMap<String, Arc<Layout>>,
    ) -> Result<Self, DefinitionError> {
        let file: EnvironmentFile = toml::from_str(source)?;
        let layout = layouts
            .get(&file.layout)
            .ok_or(DefinitionError::UnknownLayout(file.layout))?;
        Ok(Environment::new(kinds, Arc::clone(layout))?)
    }
}

/// Every legend entry must name a terrain kind, whether or not the map uses it.
fn legend(
    entries: BTreeMap<String, String>,
    kinds: &KindRegistry,
) -> Result<BTreeMap<char, KindId>, DefinitionError> {
    entries
        .into_iter()
        .map(|(key, kind)| {
            let mut chars = key.chars();
            let (Some(symbol), None) = (chars.next(), chars.next()) else {
                return Err(DefinitionError::LegendKey(key));
            };
            let kind = KindId::new(kind)?;
            match kinds.get(&kind).map(|k| k.look) {
                None => Err(DefinitionError::UnknownLegendKind { symbol, kind }),
                Some(Look::Terrain(_)) => Ok((symbol, kind)),
                Some(_) => Err(DefinitionError::LegendNotTerrain { symbol, kind }),
            }
        })
        .collect()
}

fn layout(map: &str, legend: &BTreeMap<char, KindId>) -> Result<Layout, DefinitionError> {
    // `lines` drops the final line ending and accepts CRLF, so checkouts on any platform agree.
    let rows: Vec<Vec<char>> = map.lines().map(|row| row.chars().collect()).collect();
    let Some(expected) = rows.first().map(Vec::len) else {
        return Err(DefinitionError::EmptyMap);
    };
    for (i, row) in rows.iter().enumerate() {
        if row.len() != expected {
            return Err(DefinitionError::RaggedRow {
                row: i + 1,
                width: row.len(),
                expected,
            });
        }
    }
    let mut grid = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let kinds = row
            .iter()
            .enumerate()
            .map(|(j, &symbol)| {
                legend
                    .get(&symbol)
                    .cloned()
                    .ok_or(DefinitionError::UnknownSymbol {
                        row: i + 1,
                        column: j + 1,
                        symbol,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        grid.push(kinds);
    }
    // A map too large for `u32` fails `Layout::new`'s cell count check.
    let width = u32::try_from(expected).unwrap_or(u32::MAX);
    let height = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    // Cells run from the south-west corner, so the last map row comes first.
    let cells = grid.into_iter().rev().flatten().collect();
    Ok(Layout::new(width, height, cells)?)
}
