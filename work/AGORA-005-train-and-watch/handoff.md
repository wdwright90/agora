# Handoff

Checkpoint: 2026-09-30. Chunks 1 to 3 are merged (#17, #18, #19). Chunk 4 is split into four PRs ([plan](plan.md), decisions in [context](context.md#chunk-4-decisions)); the first, 4.1 step stages, is open as #20 on branch `feature/sim-schedule`.

## Resume here

Check #20's review comments with `gh pr view 20 --comments` and `gh api repos/wdwright90/agora/pulls/20/comments`, and agree any changes before making them. Until #20 merges, this handoff is newer on `feature/sim-schedule` than on `develop`. Once it merges, agree the details of 4.2 (modular definitions): file layout under `agora-env/environments/` (`layouts/`, `ecologies/`, and environment files), the environment file's fields, whether an environment with no ecology is allowed (proposed: yes), and how the server and SPEC-001-R01 change when the agent kind leaves the definition.

## Chunk 4.1: step stages

Agreed scope (2026-09-30): a step runs as a fixed sequence of stages, each system in one stage, with the order set once and checked against the CDD; conflicting systems within a stage must be ordered; one thread per run for now. No behavior change.

Choices made in 4.1, open to review:

- **"Stage", not "phase",** because CDD-001 already uses *phases* for setup, collection, and execution.
- **All agreed stages are declared now,** including those with no systems yet (Interactions, Metabolism, Removal, Ecology, Membership), so the full order is visible. `Stage::ALL` is the single list that sets the order.
- **Commit is a stage** that increments the state ID, so the step's whole sequence is in the schedule.
- **Perception is a separate schedule,** because it also runs without a step (at Start, and later empty-run admission and recovery). Its system writes a `Perceived` resource that the operation takes.
- **Schedules are built in `Simulation::new`,** so an ambiguous pair of systems fails at creation, not on the first step.
- **ADR-003 is marked accepted,** based on the maintainer's agreement in discussion; review of the PR confirms it.

Contents:

- `agora-sim`: `schedule.rs` (`Stage`, the `Step` and `Perceive` labels, `new_schedule` with a single-threaded executor and ambiguity detection as an error, `step_schedule`); `advance` runs the step schedule; moves, the state increment, and observations are now systems.
- Docs: ADR-003 (new); CDD-001 step stages table, metabolism and exertion, direction for actions, modular definitions (agreed, not built), determinism, and open questions; SPEC-001-R21 (new); the SADD's environment definition paragraph (separate layouts and ecologies; agent settings belong to the agent); SPEC-007's open question; the decisions index and the documentation index.
- Records: context ([chunk 4 decisions](context.md#chunk-4-decisions)), plan, work index, and this handoff.

Verification, in `rust/`:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --no-deps --workspace` with `RUSTDOCFLAGS=-D warnings`: all passed.
- `cargo test --workspace`: 168 passed (43 in agora-sim, including three new R21 unit tests), one full run.
- Relative links and anchors in all Markdown files resolve (a local script).
- Not done: a manual run with the viewer and demo clients. Behavior is unchanged, and the existing SPEC-001 tests, including R17's reproducibility test, pass.

## State

- M1 is complete and in `main` ([AGORA-004](../AGORA-004-first-milestone/handoff.md) has its follow-ups, several of which are now scheduled in the roadmap).
- Chunk 3's choices are recorded in #19 and in SPEC-007; chunk 2's in #18 and SPEC-006; chunk 1's in #17 and SPEC-003.
- Design decisions are recorded in the SADD's [second milestone](../../docs/architecture/sadd.md#second-milestone-train-and-watch) section and CDD-001, with the reasoning and wire sketches in [context.md](context.md).
