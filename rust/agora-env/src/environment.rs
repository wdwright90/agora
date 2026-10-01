//! A validated environment: its kinds, layout, and agent kind (SPEC-006).

use std::sync::Arc;

use thiserror::Error;

use crate::kinds::{KindId, KindRegistry, Look};
use crate::layout::Layout;

/// Why an environment is inconsistent with its kinds.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EnvironmentError {
    #[error("layout kind {0} is not in the kind registry")]
    UnknownTerrainKind(KindId),
    #[error("layout kind {0} is not terrain")]
    NotTerrain(KindId),
    #[error("agent kind {0} is not in the kind registry")]
    UnknownAgentKind(KindId),
    #[error("agent kind {0} is not a creature")]
    AgentKindNotCreature(KindId),
}

/// Everything a run is built from: every kind in the run, the terrain layout, and the kind
/// given to every agent. Construction checks that they agree, so a simulation can trust them.
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    kinds: Arc<KindRegistry>,
    layout: Layout,
    agent_kind: KindId,
}

impl Environment {
    /// Checks that every layout cell is a terrain kind in `kinds` and that `agent_kind` is a
    /// creature in `kinds`.
    pub fn new(
        kinds: Arc<KindRegistry>,
        layout: Layout,
        agent_kind: KindId,
    ) -> Result<Self, EnvironmentError> {
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
        match kinds.get(&agent_kind).map(|k| k.look) {
            None => return Err(EnvironmentError::UnknownAgentKind(agent_kind)),
            Some(Look::Creature(_)) => {}
            Some(_) => return Err(EnvironmentError::AgentKindNotCreature(agent_kind)),
        }
        Ok(Self {
            kinds,
            layout,
            agent_kind,
        })
    }

    pub fn kinds(&self) -> &Arc<KindRegistry> {
        &self.kinds
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn agent_kind(&self) -> &KindId {
        &self.agent_kind
    }
}
