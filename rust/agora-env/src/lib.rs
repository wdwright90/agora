//! Data-only Agora environment definitions.
//!
//! A [`KindRegistry`] declares every kind in a run once: terrain with its class and flags, and
//! items and creatures with their [`Appearance`]. It is the single source of truth for how
//! things look, read by the simulation and sent to viewers. [`builtin`] holds the kinds the
//! bundled environments use.
//!
//! An [`Environment`] is what a run is built from: its kinds, a [`Layout`] giving every cell's
//! terrain, and the kind given to agents. Environments load from TOML definitions, and
//! [`bundled`] holds the definitions compiled into the program. Behavior is specified by
//! SPEC-006 (`docs/components/simulation/specs/kinds-and-appearance.md`) and SPEC-007
//! (`docs/components/simulation/specs/environment-definitions.md`).
//!
//! This package holds no Bevy runtime types, so tools can use it without the simulation. The
//! `reflect` feature derives `bevy_reflect::Reflect` on the types simulation components hold.

pub mod builtin;
pub mod bundled;
mod definition;
mod environment;
mod kinds;
mod layout;

pub use definition::DefinitionError;
pub use environment::{Environment, EnvironmentError};
pub use kinds::{
    Appearance, EmptyKindId, Kind, KindId, KindRegistry, Look, RegistryError, Shape, Terrain,
    TerrainClass, Unit, UnitOutOfRange,
};
pub use layout::{Layout, LayoutError};
