//! The layout and environment files bundled with Agora, compiled into the program (SPEC-007).

use std::collections::BTreeMap;
use std::sync::Arc;

use thiserror::Error;

use crate::definition::DefinitionError;
use crate::environment::Environment;
use crate::kinds::KindRegistry;
use crate::layout::Layout;

/// Each bundled layout's ID and TOML source. The files are in this package's
/// `environments/layouts` directory, named after their IDs, and use the
/// [built-in kinds](crate::builtin).
pub const LAYOUTS: &[(&str, &str)] = &[
    (
        "empty-grid-10x10",
        include_str!("../environments/layouts/empty-grid-10x10.toml"),
    ),
    (
        "divided-10x10",
        include_str!("../environments/layouts/divided-10x10.toml"),
    ),
];

/// Each bundled environment's catalog entry ID and TOML source, in catalog order. The files
/// are in this package's `environments` directory, named after their IDs, and name layouts in
/// [`LAYOUTS`].
pub const ENVIRONMENTS: &[(&str, &str)] = &[
    (
        "empty-grid-10x10",
        include_str!("../environments/empty-grid-10x10.toml"),
    ),
    (
        "divided-10x10",
        include_str!("../environments/divided-10x10.toml"),
    ),
];

/// Why the bundled files could not be loaded, naming the file that failed.
#[derive(Debug, Clone, Error)]
pub enum BundledError {
    #[error("bundled layout {id} is invalid: {error}")]
    Layout {
        id: &'static str,
        error: DefinitionError,
    },
    #[error("bundled environment {id} is invalid: {error}")]
    Environment {
        id: &'static str,
        error: DefinitionError,
    },
}

/// Load every bundled environment with `kinds`, in catalog order.
///
/// Every bundled layout is loaded first, including any that no environment uses, and then
/// each environment looks its layout up by ID.
pub fn environments(
    kinds: Arc<KindRegistry>,
) -> Result<Vec<(&'static str, Environment)>, BundledError> {
    let layouts = LAYOUTS
        .iter()
        .map(|&(id, source)| {
            Layout::from_toml(source, &kinds)
                .map(|layout| (id.to_owned(), Arc::new(layout)))
                .map_err(|error| BundledError::Layout { id, error })
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    ENVIRONMENTS
        .iter()
        .map(|&(id, source)| {
            Environment::from_toml(source, Arc::clone(&kinds), &layouts)
                .map(|environment| (id, environment))
                .map_err(|error| BundledError::Environment { id, error })
        })
        .collect()
}
