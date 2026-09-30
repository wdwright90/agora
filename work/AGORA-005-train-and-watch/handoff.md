# Handoff

Checkpoint: 2026-09-28, chunk 2 (registry and appearance) open as #18 on branch `feature/registry`. Chunk 1 merged as #17, with the Rust component diagrams ([docs/architecture/rust-components.md](../../docs/architecture/rust-components.md)).

## Resume here

Check #18's review comments with `gh pr view 18 --comments` and the inline comments through the GitHub API (`gh api repos/wdwright90/agora/pulls/18/comments`), and agree any changes before making them. Until #18 merges, this handoff is newer on `feature/registry` than on `develop`. Once it merges, update the plan's status column and the work index, then agree the scope of chunk 3 (environment definitions) using the notes below.

## Chunk 2

Agreed scope (2026-09-28):

1. Create `agora-env`, the data-only environment package, now, holding the kind registry; chunk 3 adds definitions to it.
2. Kinds are data (an ID, a category, an appearance, and flags); behavior stays in Rust. The built-in registry is built in code: floor, wall, berry, and agent. Loading from definitions is chunk 3.
3. Option B for the wire: the server sends the run's registry once, as `kinds` in `watching`, and each agent in a view carries a `kind` ID. Agents will perceive appearance, never kind IDs.
4. `Reflect` on `Agent` and `Position`.
5. The viewer keeps colouring agents by ID until chunk 8.

Choices made in chunk 2, open to review:

- **Terrain has no hue, size, or shape.** The planning notes ([context.md](context.md#appearance)) give terrain a class and flags only, so terrain kinds are `{category: terrain, kind, class, blocks_movement, blocks_sight}`; items and creatures carry `appearance: {hue, size, shape}`. The sketch shown while agreeing option B gave terrain an appearance; the notes won.
- **Field names:** `category` selects the entry's form and `kind` is the ID, both in `kinds` entries and in agent views. `kind` is also the tag for placements, actions, and pacing modes, on other objects.
- **Strict enums:** unknown categories, classes, and shapes are malformed, since versions must match exactly before 1.0.0. Hue and size are 0 to 1 inclusive.
- **Built-in values** follow the sight sketch: berry hue 0.02, size 0.3, `round`; agent hue 0.6, size 0.5, `agent`. Walls block movement and sight; floors block neither. The flags do nothing yet.
- **`agora-env`'s `reflect` feature** derives `Reflect` on `KindId`, because the simulation's new `Kind` component holds one and a derive cannot be added to another crate's type. It is off by default, so the package keeps no Bevy dependency for tools; `agora-sim` turns it on.
- **Reflection test:** it checks every component on a spawned agent (`Agent`, `Kind`, `Position`) is registered for reflection. A test over all registered components did not work: in `bevy_ecs` 0.19 resources are components, and the deprecated `get_valid_resource_id` reports every component as a resource.
- **Client:** `Session::kinds()` returns `Option<&[Kind]>`, set once by `watching`, rather than a latest-value handle, because kinds do not change during a run. A `kinds_updated` push could replace this if M7's live editing adds kinds.
- **Server:** the catalog entry carries the registry and the agent kind; the run keeps the wire form and clones it for each new viewer.

Contents:

- `agora-env` 0.1.0 (new): `KindId`, `Unit`, `Appearance`, `Terrain`, `Look`, `Kind`, `KindRegistry`, and `builtin`.
- `agora-sim`: `SimConfig` takes `kinds` and `agent_kind` and validates them; agents get a `Kind` component; `AgentView` has `kind`; `Simulation::kinds`; `Agent`, `Position`, and `Kind` derive `Reflect` and are registered in an `AppTypeRegistry`.
- `agora-protocol` 0.6.0: `KindId`, `Kind`, `Appearance`, `Shape`, `TerrainClass`, and `Unit`; `kinds` in `watching`; `kind` in `AgentView`; fixtures.
- `agora-server`: the catalog entry uses the built-in kinds; `watching` carries them; views carry each agent's kind.
- `agora-client`: `Session::kinds`.
- `agora-viewer`: compiles against the new view; no behavior change.
- Specs: SPEC-006 (new) R01 to R04; SPEC-001-R01 and R16 updated, R19 added; SPEC-002 R03 and R12 updated, version 0.6.0; SPEC-003 R01 and R10 updated; SPEC-004-R11 added. CDD-001 (now also mapping `agora-env`), CDD-002, CDD-003, the SADD's package mapping, the components index, `rust/README.md`, and the architecture diagrams updated.

Verification, in `rust/`:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --no-deps --workspace` with `RUSTDOCFLAGS=-D warnings`: all passed.
- `cargo test --workspace`: 145 passed (5 agora-env, 37 agora-sim, 14 agora-protocol, 67 agora-server, 12 agora-client, 10 agora-viewer), one full run.
- Relative links and anchors in all Markdown files resolve (a local script). The dependency diagram was rendered with `@mermaid-js/mermaid-cli`.
- Not done: a manual run with the viewer and demo clients. The viewer's drawing is unchanged, and the server tests cover the new wire fields.

## Chunk 3 notes

Not yet discussed with the maintainer:

- The definition format (probably RON or TOML with serde in `agora-env`) and where bundled definitions live.
- Declaring kinds in definitions, and whether a catalog entry can add kinds to the built-in ones.
- Terrain in the grid: views will need terrain, probably as a grid of kind references; whether to use indices into `kinds` to keep views small.

## State

- M1 is complete and in `main` ([AGORA-004](../AGORA-004-first-milestone/handoff.md) has its follow-ups, several of which are now scheduled in the roadmap).
- Design decisions are recorded in the SADD's [second milestone](../../docs/architecture/sadd.md#second-milestone-train-and-watch) section and CDD-001, with the reasoning and wire sketches in [context.md](context.md).
