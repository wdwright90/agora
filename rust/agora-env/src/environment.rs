//! A validated environment: its kinds and layout (SPEC-007).

use std::sync::Arc;

use thiserror::Error;

use crate::builtin;
use crate::kinds::{KindId, KindRegistry, Look};
use crate::layout::Layout;

/// Why an environment is inconsistent with its kinds.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EnvironmentError {
    #[error("layout kind {0} is not in the kind registry")]
    UnknownTerrainKind(KindId),
    #[error("layout kind {0} is not terrain")]
    NotTerrain(KindId),
    #[error("the kind registry has no {0} kind for agents")]
    MissingAgentKind(KindId),
    #[error("the agent kind {0} is not a creature")]
    AgentKindNotCreature(KindId),
}

/// Everything a run is built from: every kind in the run and the terrain layout. Construction
/// checks that they agree, so a simulation can trust them.
///
/// Every agent gets the [built-in agent kind](builtin::agent_kind), so the registry must
/// declare it as a creature.
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    kinds: Arc<KindRegistry>,
    layout: Arc<Layout>,
}

impl Environment {
    /// Checks that every layout cell is a terrain kind in `kinds` and that `kinds` declares the
    /// built-in agent kind as a creature.
    pub fn new(kinds: Arc<KindRegistry>, layout: Arc<Layout>) -> Result<Self, EnvironmentError> {
        // Layouts repeat a few kinds many times, so check each distinct kind once.
        let mut checked: Vec<&KindId> = Vec::new();
        for kind in layout.cells() {
            if checked.contains(&kind) {
                continue;
            }
            match kinds.get(kind).map(|k| k.look) {
                None => return Err(EnvironmentError::UnknownTerrainKind(kind.clone())),
                Some(Look::Terrain(_)) => checked.push(kind),
                Some(_) => return Err(EnvironmentError::NotTerrain(kind.clone())),
            }
        }
        let agent_kind = builtin::agent_kind();
        match kinds.get(&agent_kind).map(|k| k.look) {
            None => return Err(EnvironmentError::MissingAgentKind(agent_kind)),
            Some(Look::Creature(_)) => {}
            Some(_) => return Err(EnvironmentError::AgentKindNotCreature(agent_kind)),
        }
        Ok(Self { kinds, layout })
    }

    pub fn kinds(&self) -> &Arc<KindRegistry> {
        &self.kinds
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }
}
