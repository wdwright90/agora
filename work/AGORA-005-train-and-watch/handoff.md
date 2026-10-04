# Handoff

Checkpoint: 2026-10-03. Chunks 1 to 3 and 4.1 are merged (#17 to #20). Chunk 4.2, modular definitions, is open as #21 on branch `feature/modular-definitions`.

## Resume here

Check #21's review comments with `gh pr view 21 --comments` and `gh api repos/wdwright90/agora/pulls/21/comments`, and agree any changes before making them. Until #21 merges, this handoff is newer on `feature/modular-definitions` than on `develop`. Once it merges, agree the details of 4.3 (metabolism): the energy component and its range, the exertion component, the default metabolism's numbers, the Metabolism and Removal stage systems, and the `agent_removed {reason}` push.

## Chunk 4.2: modular definitions

Agreed scope (2026-10-03, [context](context.md#chunk-4-decisions)): layout files in `environments/layouts/`, environment files naming their layout by ID, two-step loading with an error for an unknown layout ID, no ecology field until 4.4, and the agent kind removed from definitions.

Choices made in 4.2, open to review:

- **The registry must declare `agent` as a creature.** `Environment::new` checks it in place of the old supplied agent kind, so a built environment can still be trusted ([SPEC-007-R02](../../docs/components/simulation/specs/environment-definitions.md#layouts-and-environments)).
- **Environments share their layout** as `Arc<Layout>`, so one loaded layout serves every environment that names it.
- **`bundled::environments(kinds)`** loads every bundled file and names the one that fails (`BundledError`); the server's catalog calls it instead of reading the files itself.
- **Requirement IDs stay stable:** R03 is now the layout file format, R04 and R05 are unchanged, and the environment file format is the new R07.
- An empty layout ID is rejected as an unknown layout, not with a separate error.

Contents:

- `agora-env`: `Layout::from_toml`, `Environment::from_toml(source, kinds, &layouts)`, `Environment::new(kinds, layout)` without the agent kind, `bundled::{LAYOUTS, ENVIRONMENTS, environments, BundledError}`, `DefinitionError::UnknownLayout`, and `EnvironmentError::MissingAgentKind`; the layout files moved to `environments/layouts/`, and environment files reduced to `layout = "<id>"`. Doc comments now cite SPEC-007 instead of SPEC-006.
- `agora-sim` spawns agents with `builtin::agent_kind()`; `agora-server`'s catalog uses `bundled::environments`.
- Docs: SPEC-007 (R02, R03, R06 rewritten, R07 new, terminology, open questions); SPEC-001-R01; SPEC-003-R01 wording; CDD-001's environment definitions; the server CDD's catalog line; the SADD package table; `rust/README.md`.
- Records: context (4.2 details), plan, work index, and this handoff.

Verification, in `rust/`:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --no-deps --workspace` with `RUSTDOCFLAGS=-D warnings`: all passed.
- `cargo test --workspace`: 174 passed, one full run.
- Not done: a manual run with the viewer and demo clients. The catalog's environments are unchanged, and the server's catalog and run tests pass.

## State

- M1 is complete and in `main` ([AGORA-004](../AGORA-004-first-milestone/handoff.md) has its follow-ups, several of which are now scheduled in the roadmap).
- Chunk 4.1's choices are recorded in #20 and ADR-003; chunk 3's in #19 and SPEC-007; chunk 2's in #18 and SPEC-006; chunk 1's in #17 and SPEC-003.
- Design decisions are recorded in the SADD's [second milestone](../../docs/architecture/sadd.md#second-milestone-train-and-watch) section and CDD-001, with the reasoning and wire sketches in [context.md](context.md).
