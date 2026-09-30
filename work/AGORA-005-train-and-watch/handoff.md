# Handoff

Checkpoint: 2026-09-29, chunks 1 and 2 merged as #17 and #18. Chunk 3 (environment definitions) is implemented and verified on branch `feature/env-definitions`, not yet committed or opened as a PR.

## Resume here

Review chunk 3 with the maintainer, commit it, and open the PR against `develop` once they agree. Then record the PR number here and in the plan, and agree the scope of chunk 4 (metabolism and food), which also settles how a catalog entry combines a layout with ecology rules ([SPEC-007 open questions](../../docs/components/simulation/specs/environment-definitions.md#open-questions)).

## Chunk 3

Agreed scope (2026-09-29):

1. Definitions are TOML, parsed in `agora-env` with serde. The layout is an ASCII map with a legend from single characters (any Unicode character) to terrain kinds. The map is the file form; `agora-env` parses it into a layout of kind IDs, which code can also build directly. Generating a layout per run would later be a Rust generator configured by the definition, not generated TOML.
2. Bundled definitions live in `agora-env/environments/*.toml` and are compiled into the program; the catalog is built from them, with a test that every bundled definition loads.
3. Definitions use only the built-in kinds. Unknown kinds, non-terrain kinds in the legend, and ragged rows are rejected. Declaring kinds in definitions waits until an environment needs it.
4. The simulation gets a terrain layer: blocking terrain stops moves as the edge does, and random spawns never choose a blocked cell. `blocks_sight` waits for sight (chunk 5).
5. `watching` carries `terrain`, a grid of indices into `kinds`, as a snapshot at the time of watching; the order of `kinds` is fixed for a run (append-only), so later terrain changes can arrive as a push of changed cells. The simulation changes terrain in one place. Agents never receive this grid; the viewer ignores it until chunk 8.
6. `empty-grid-10x10` becomes a definition, plus one small walled layout. The tuned environments are chunk 11.
7. No ecology rules yet (chunk 4), but the format leaves room for them.

Choices made in chunk 3, open to review:

- **A new spec, SPEC-007 (environment definitions),** rather than growing SPEC-006, since definitions change for different reasons than kinds.
- **`Environment` is a checked type in `agora-env`** (kinds, layout, and agent kind). `Simulation::new` takes it and can no longer fail, so `ConfigError` is gone and its checks moved to SPEC-007-R01 and R02. This replaces chunk 2's validation in `SimConfig`.
- **Definition shape:** `agent_kind`, `[layout] map`, and `[layout.legend]`, with unknown fields rejected so typos fail loudly. The catalog entry ID is the file name, not a field. Every legend entry is checked even when unused. Errors give 1-based line and column from the top-left.
- **Wire cell order:** `terrain` is row-major from the south-west corner (`y * width + x`), matching positions and the simulation, not the map's north-first reading order.
- **`watching` is validated on parse:** `terrain` must have `width × height` entries, each indexing a terrain kind. This made `ServerMessage::Watching` a newtype variant around a `Watching` struct. The `kind_*` fixtures now include terrain, so they fail only for their kind defect.
- **Spawning on a wall** returns the new `cell_blocked` error (details `x`, `y`), checked after bounds and before occupancy. Protocol version 0.7.0.
- **The walled layout is `divided-10x10`:** a wall along x = 5 with a gap at y = 4 and 5.
- **Client:** `Session::terrain()` returns `Option<&[u32]>`, set with the kinds by the first `watching`.

Contents:

- `agora-env`: `Layout`, `Environment`, `Environment::from_toml`, `bundled::DEFINITIONS`, `KindRegistry::index_of`, and `environments/empty-grid-10x10.toml` and `divided-10x10.toml`. New dependencies: `serde` and `toml` 1.1.
- `agora-sim`: `SimConfig { environment, seed }`; a terrain layer with a derived blocked mask; `SpawnError::Blocked`; `Simulation::terrain`.
- `agora-protocol` 0.7.0: `Watching` with `terrain`, `InvalidTerrain`, and `ErrorCode::CellBlocked`; fixtures.
- `agora-server`: the catalog loads the bundled definitions once; `watching` carries the current terrain; `cell_blocked`; `DIVIDED_10X10`.
- `agora-client`: `Session::terrain`.
- `agora-viewer`: no change.
- Specs: SPEC-007 (new) R01 to R06; SPEC-001 R01, R03, R04, and R13 updated, R20 added; SPEC-006 R01 and R02 updated; SPEC-002 R08, R09, and R12 updated, version 0.7.0; SPEC-003 R01 and R10 updated; SPEC-004-R11 updated. CDD-001, CDD-002, CDD-003, the SADD's package mapping, the components index, `rust/README.md`, and the architecture diagrams updated.

Verification, in `rust/`:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --no-deps --workspace` with `RUSTDOCFLAGS=-D warnings`: all passed.
- `cargo test --workspace`: 165 passed (18 agora-env, 40 agora-sim, 15 agora-protocol, 69 agora-server, 13 agora-client, 10 agora-viewer), one full run.
- Relative links and anchors in all Markdown files resolve (a local script).
- Not done: a manual run with the viewer and demo clients, and rendering the edited dependency diagram (one node label changed). The viewer ignores terrain, and the server tests cover the new wire field.

## State

- M1 is complete and in `main` ([AGORA-004](../AGORA-004-first-milestone/handoff.md) has its follow-ups, several of which are now scheduled in the roadmap).
- Chunk 2's choices are recorded in #18 and in SPEC-006; chunk 1's in #17 and SPEC-003.
- Design decisions are recorded in the SADD's [second milestone](../../docs/architecture/sadd.md#second-milestone-train-and-watch) section and CDD-001, with the reasoning and wire sketches in [context.md](context.md).
