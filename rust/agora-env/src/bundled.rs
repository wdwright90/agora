//! The environment definitions bundled with Agora, compiled into the program (SPEC-006).

/// Each bundled definition's catalog entry ID and TOML source, in catalog order. The files are
/// in this package's `environments` directory, named after their IDs, and use the
/// [built-in kinds](crate::builtin).
pub const DEFINITIONS: &[(&str, &str)] = &[
    (
        "empty-grid-10x10",
        include_str!("../environments/empty-grid-10x10.toml"),
    ),
    (
        "divided-10x10",
        include_str!("../environments/divided-10x10.toml"),
    ),
];
