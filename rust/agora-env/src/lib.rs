//! Data-only Agora environment definitions.
//!
//! A [`KindRegistry`] declares every kind in a run once: terrain with its class and flags, and
//! items and creatures with their [`Appearance`]. It is the single source of truth for how
//! things look, read by the simulation and sent to viewers. [`builtin`] holds the kinds the
//! bundled catalog uses. Behavior is specified by SPEC-006
//! (`docs/components/simulation/specs/kinds-and-appearance.md`).
//!
//! This package holds no Bevy runtime types, so tools can use it without the simulation. The
//! `reflect` feature derives `bevy_reflect::Reflect` on the types simulation components hold.

pub mod builtin;
mod kinds;

pub use kinds::{
    Appearance, EmptyKindId, Kind, KindId, KindRegistry, Look, RegistryError, Shape, Terrain,
    TerrainClass, Unit, UnitOutOfRange,
};
