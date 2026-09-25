# Handoff

Checkpoint: 2026-09-24, PR 1 (`agora-sim` core), branch `feature/sim-core`.

## Resume here

When PR 1 has been reviewed and merged, agree the scope of PR 2 (minimal protocol contract and `agora-protocol`) with the maintainer before implementing it.

## Decisions this session

- The maintainer accepted ADR-001's package mapping. The SADD, the simulation CDD, and the workspace README were updated to match.
- Rust 1.98.1 is pinned in `rust/rust-toolchain.toml`, and Bevy 0.19 is used (it needs Rust 1.95 or later). `agora-sim` depends on `bevy_ecs`, not full Bevy.
- Per-run randomness uses ChaCha8 streams derived from the run seed: stream 0 is reserved for environment generation, stream 1 is for spawning, and stream 2 is for shuffling.
- Agent IDs are `u64`, assigned sequentially from 1.
- Specs are written alongside features and scoped to durable behavior areas (see the [workflow](../../docs/workflows/ai-development.md#features-and-specs)).

## PR 1 contents

- `rust/agora-sim`: a `Simulation` wrapping a `bevy_ecs::World`, with the operations `new`, `spawn`, `start`, `submit`, `readiness`, `advance`, `view`, and `agent_count`.
- SPEC-001 requirements R01–R17 were added. Tests are in `rust/agora-sim/tests/lifecycle_and_movement.rs`, named by requirement ID.
- Choices for review:
  - The coordinate convention: `(0, 0)` is the south-west corner and north is `+y`, matching Bevy's y-up world.
  - The submission check order.
  - Interim rule R06: spawning after Start is rejected until post-Start admission is implemented.
  - The simulation allows Start with zero agents; the server enforces the agents-or-viewers rule.

## Verification

In `rust/`, on Rust 1.98.1:

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p agora-sim --no-deps`: passed.
- `cargo test --workspace`: 30 passed.

No CI exists.

## Open follow-ups

- Whether `agora-sim` should use identifier types from `agora-protocol` or keep its own and map them in the server. Decide in PR 2.
- The ECS is used minimally so far: resources and components, with no schedules. Schedules can be introduced when perception or other per-agent systems arrive.
