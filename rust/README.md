# Agora Rust workspace

This directory is the Cargo workspace for the Rust simulation framework. The [SADD](../docs/architecture/sadd.md) owns system responsibilities, and the [package mapping](../docs/architecture/sadd.md#rust-package-mapping) (accepted in [ADR-001](../docs/decisions/0001-rust-package-boundaries.md)) defines the packages. Packages are added to `members` in `Cargo.toml` as features need them. [Component design descriptions](../docs/components/README.md) map components to packages, and shared message contracts belong in [docs/contracts](../docs/contracts/README.md).

## Packages

| Package | Purpose | Specs |
| --- | --- | --- |
| [`agora-protocol`](agora-protocol/) | Serde types for the client protocol. No networking. Tested against the shared JSON fixtures. | [SPEC-002](../docs/contracts/client-protocol.md) |
| [`agora-sim`](agora-sim/) | Headless simulation: one run's grid, agents, action collection, and step execution. Uses `bevy_ecs`, with no rendering or networking. | [SPEC-001](../docs/components/simulation/specs/lifecycle-and-movement.md) |

## Toolchain

`rust-toolchain.toml` pins Rust 1.98.1 with rustfmt and clippy, and rustup installs it automatically. Bevy 0.19 requires at least Rust 1.95. Shared dependency versions live in `[workspace.dependencies]`.

## Checks

Run these from this directory before opening a PR:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

No CI runs these checks yet.
