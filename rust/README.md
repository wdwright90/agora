# Agora Rust workspace

This directory is the Cargo workspace for the Rust simulation framework. It starts as an empty virtual workspace; packages will be added after their responsibilities are defined.

The [SADD](../docs/architecture/sadd.md) owns system responsibilities. [Component design descriptions](../docs/components/README.md) will map logical components to implementing packages. Shared message contracts belong in [docs/contracts](../docs/contracts/README.md).

Package names, directory layout within this workspace, dependencies, and the Rust toolchain baseline remain to be decided. Add packages explicitly to `members` in `Cargo.toml` as they are introduced.

The SADD contains a [proposed package mapping](../docs/architecture/sadd.md#proposed-rust-package-mapping) for review. It does not yet create or finalize workspace members.

There are no buildable packages or tests yet. Cargo build and test commands will be documented with the packages once they exist.
