//! The run's kind registry as sent to viewers (SPEC-002-R12).

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::KindId;

/// One registered kind. `category` selects the variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "category", rename_all = "snake_case")]
pub enum Kind {
    /// Terrain: exactly one per cell.
    Terrain {
        kind: KindId,
        class: TerrainClass,
        blocks_movement: bool,
        blocks_sight: bool,
    },
    /// Items lie in cells, at most one per cell.
    Item {
        kind: KindId,
        appearance: Appearance,
    },
    /// Creatures, including agents.
    Creature {
        kind: KindId,
        appearance: Appearance,
    },
}

impl Kind {
    pub fn id(&self) -> &KindId {
        match self {
            Self::Terrain { kind, .. } | Self::Item { kind, .. } | Self::Creature { kind, .. } => {
                kind
            }
        }
    }
}

/// How an item or creature looks.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Appearance {
    /// Position on the colour wheel. 0 and 1 are the same hue.
    pub hue: Unit,
    pub size: Unit,
    pub shape: Shape,
}

/// Shape class of an item or creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    Agent,
    Round,
}

/// Terrain class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainClass {
    Floor,
    Wall,
}

/// A number from 0 to 1 inclusive.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct Unit(f64);

/// A number outside 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
#[error("{0} is not a number from 0 to 1")]
pub struct UnitOutOfRange(pub f64);

impl Unit {
    /// Returns `None` outside 0 to 1, or for NaN.
    pub fn new(value: f64) -> Option<Self> {
        (0.0..=1.0).contains(&value).then_some(Self(value))
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Unit {
    type Error = UnitOutOfRange;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(UnitOutOfRange(value))
    }
}

impl From<Unit> for f64 {
    fn from(unit: Unit) -> f64 {
        unit.0
    }
}
