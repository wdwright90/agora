//! Kinds, their appearance, and the registry that declares them (SPEC-006).

use std::collections::HashMap;
use std::fmt;

use thiserror::Error;

/// A kind's identifier, unique within its registry. Non-empty.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
pub struct KindId(String);

/// A kind ID must not be empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("kind IDs must not be empty")]
pub struct EmptyKindId;

impl KindId {
    pub fn new(id: impl Into<String>) -> Result<Self, EmptyKindId> {
        let id = id.into();
        if id.is_empty() {
            Err(EmptyKindId)
        } else {
            Ok(Self(id))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for KindId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A number from 0 to 1 inclusive, used for hue and size.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Unit(f64);

/// A value outside 0 to 1, or not a number.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
#[error("{0} is not a number from 0 to 1")]
pub struct UnitOutOfRange(pub f64);

impl Unit {
    pub fn new(value: f64) -> Result<Self, UnitOutOfRange> {
        if (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(UnitOutOfRange(value))
        }
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

/// A shape class for items and creatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    Agent,
    Round,
}

/// A terrain class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainClass {
    Floor,
    Wall,
}

/// How an item or creature looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Appearance {
    /// Position on the colour wheel. 0 and 1 are the same hue.
    pub hue: Unit,
    pub size: Unit,
    pub shape: Shape,
}

/// A terrain kind's class and flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Terrain {
    pub class: TerrainClass,
    pub blocks_movement: bool,
    pub blocks_sight: bool,
}

/// A kind's category and what describes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Look {
    Terrain(Terrain),
    Item(Appearance),
    Creature(Appearance),
}

/// One registered kind.
#[derive(Debug, Clone, PartialEq)]
pub struct Kind {
    pub id: KindId,
    pub look: Look,
}

/// Why a registry could not be built.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RegistryError {
    #[error("kind {0} is declared more than once")]
    DuplicateKind(KindId),
}

/// Every kind in a run, each declared once, in declaration order.
#[derive(Debug, Clone, PartialEq)]
pub struct KindRegistry {
    kinds: Vec<Kind>,
    index: HashMap<KindId, usize>,
}

impl KindRegistry {
    pub fn new(kinds: Vec<Kind>) -> Result<Self, RegistryError> {
        let mut index = HashMap::with_capacity(kinds.len());
        for (i, kind) in kinds.iter().enumerate() {
            if index.insert(kind.id.clone(), i).is_some() {
                return Err(RegistryError::DuplicateKind(kind.id.clone()));
            }
        }
        Ok(Self { kinds, index })
    }

    pub fn get(&self, id: &KindId) -> Option<&Kind> {
        self.index.get(id).map(|&i| &self.kinds[i])
    }

    /// A kind's position in declaration order.
    pub fn index_of(&self, id: &KindId) -> Option<usize> {
        self.index.get(id).copied()
    }

    /// The kinds in declaration order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Kind> {
        self.kinds.iter()
    }

    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}
