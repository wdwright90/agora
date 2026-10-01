# Agora Rust workspace

This directory is the Cargo workspace for the Rust simulation framework. [How the Rust components fit together](../docs/architecture/rust-components.md) diagrams the packages and how they interact at run time. The [SADD](../docs/architecture/sadd.md) owns system responsibilities, and the [package mapping](../docs/architecture/sadd.md#rust-package-mapping) (accepted in [ADR-001](../docs/decisions/0001-rust-package-boundaries.md)) defines the packages. Packages are added to `members` in `Cargo.toml` as features need them. [Component design descriptions](../docs/components/README.md) map components to packages, and shared message contracts belong in [docs/contracts](../docs/contracts/README.md).

## Packages

| Package | Purpose | Specs |
| --- | --- | --- |
| [`agora-client`](agora-client/) | Async client library (connection, sessions, requests, observation stream, and latest-value view and pacing handles) and the `agora-demo` random-mover executable, behind the default `demo` feature. | [SPEC-002](../docs/contracts/client-protocol.md), [SPEC-004](../docs/components/client/specs/client-library.md) |
| [`agora-viewer`](agora-viewer/) | Bevy viewer: watches one run (`--create` or `--join <RUN_ID>`), draws the grid and agents, and offers Start and pacing controls in an `egui` panel. | [SPEC-005](../docs/components/viewer/specs/viewer.md) |
| [`agora-protocol`](agora-protocol/) | Serde types for the client protocol. No networking. Tested against the shared JSON fixtures. | [SPEC-002](../docs/contracts/client-protocol.md) |
| [`agora-server`](agora-server/) | Host application: WebSocket connections, sessions, run creation and joining, spawning, Start, action submission, step advancement, observation routing, viewers, pacing, and session and run lifetimes. Runs each simulation in its own task. | [SPEC-002](../docs/contracts/client-protocol.md), [SPEC-003](../docs/components/server/specs/sessions-and-runs.md) |
| [`agora-env`](agora-env/) | Data-only environment definitions: the kind registry and the built-in kinds, layouts, the TOML definition format, and the bundled definitions in `environments/`. No Bevy runtime types; the `reflect` feature derives `Reflect` for simulation components. | [SPEC-006](../docs/components/simulation/specs/kinds-and-appearance.md), [SPEC-007](../docs/components/simulation/specs/environment-definitions.md) |
| [`agora-sim`](agora-sim/) | Headless simulation: one run's grid and terrain, agents, action collection, and step execution. Uses `bevy_ecs`, with no rendering or networking. | [SPEC-001](../docs/components/simulation/specs/lifecycle-and-movement.md), [SPEC-006](../docs/components/simulation/specs/kinds-and-appearance.md) |

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

## Running the server

```sh
cargo run -p agora-server -- --listen 127.0.0.1:7878
```

`--help` lists the timeout and step-interval flags. Set `RUST_LOG` (for example, `RUST_LOG=debug`) to change log detail.

## Running the first milestone

With the server running, start the viewer, which creates a run and shows its ID:

```sh
cargo run -p agora-viewer -- --create
```

Copy the run ID from the viewer's panel, then join the run with two demo clients:

```sh
cargo run -p agora-client --bin agora-demo -- join <RUN_ID>
cargo run -p agora-client --bin agora-demo -- join <RUN_ID>
```

Press Start in the viewer. The panel's controls pause, single-step, resume, change the interval, or run unlimited. `agora-viewer --join <RUN_ID>` watches an existing run instead. The first build of the viewer compiles Bevy and takes a while.

## Running the demo clients without the viewer

With the server running, start a run in one terminal and join it from another:

```sh
cargo run -p agora-client --bin agora-demo -- create --start-at 2
cargo run -p agora-client --bin agora-demo -- join <RUN_ID>
```

`create` prints the run ID, then starts the run once both agents have spawned. Each client moves its agent at random every step. `--steps N` stops after N steps, and `--help` lists the other options.
