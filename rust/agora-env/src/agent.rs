//! Agent definitions: the capabilities an agent is spawned with (SPEC-008).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Why an agent file could not be loaded.
#[derive(Debug, Clone, Error)]
pub enum AgentDefinitionError {
    /// Not TOML, or not the file's shape: an unknown capability or field, or a missing or
    /// mistyped one. The message names the supported capabilities or fields.
    #[error("invalid file: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("{capability}.{field} must be a finite number of at least 0, but is {value}")]
    OutOfRange {
        capability: &'static str,
        field: &'static str,
        value: f64,
    },
}

/// The capabilities an agent is spawned with. Each one is optional, and an agent without a
/// capability does not have it: nothing is filled in by default.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AgentDefinition {
    /// Energy that decays each step and is spent by effort; the agent starves at 0.
    pub metabolism: Option<Metabolism>,
}

/// An agent's energy accounting. Energy runs from 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metabolism {
    /// Energy lost every step.
    pub decay: f64,
    /// Energy lost per unit of effort spent by actions.
    pub rate: f64,
}

/// An agent file. Each table is one capability; serializing a value with every capability
/// present lists every supported table and field.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    metabolism: Option<Metabolism>,
}

impl AgentDefinition {
    /// Load a definition from a TOML agent file and check its values.
    pub fn from_toml(source: &str) -> Result<Self, AgentDefinitionError> {
        let file: AgentFile = toml::from_str(source)?;
        let definition = Self {
            metabolism: file.metabolism,
        };
        definition.check()?;
        Ok(definition)
    }

    /// The definition as a TOML agent file.
    pub fn to_toml(&self) -> String {
        toml::to_string(&AgentFile {
            metabolism: self.metabolism,
        })
        .expect("agent definitions serialize")
    }

    /// Check that every value is in range. [`from_toml`](Self::from_toml) checks the values it
    /// loads; a definition built in code should be checked before it is used.
    pub fn check(&self) -> Result<(), AgentDefinitionError> {
        if let Some(Metabolism { decay, rate }) = self.metabolism {
            non_negative("metabolism", "decay", decay)?;
            non_negative("metabolism", "rate", rate)?;
        }
        Ok(())
    }
}

fn non_negative(
    capability: &'static str,
    field: &'static str,
    value: f64,
) -> Result<(), AgentDefinitionError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(AgentDefinitionError::OutOfRange {
            capability,
            field,
            value,
        })
    }
}
