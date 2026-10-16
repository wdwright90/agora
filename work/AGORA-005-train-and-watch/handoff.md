# Handoff

Checkpoint: 2026-10-15. Chunks 1 to 3, 4.1, and 4.2 are merged (#17 to #21). Chunk 4.3, metabolism, is implemented on branch `feature/metabolism` and not yet committed or opened as a PR.

## Resume here

Review the 4.3 changes with the maintainer, then commit and open the PR against `develop` once they agree. After it merges, agree the details of 4.4 (food): the ecology file format and the environment file's `ecology` field, food items and their placement, diets on agent definitions (a new capability or part of `metabolism`), automatic eating in the Interactions stage, and the maintain rule.

## Chunk 4.3: metabolism

Agreed scope (2026-10-12, [context](context.md#chunk-43-details)): agents have almost nothing by default; a catalog of premade agent definitions, starting with a `forager` that has only a metabolism, named by a required `agent` field on `spawn`; energy, effort, decay, and starvation with removal; energy in observations; the `agent_removed {reason}` push between `submitted` and `observations`; placeholder numbers (decay 0.005, move effort 0.005, rate 1).

Choices made in 4.3, open to review:

- **The client library's observation stream is now an agent event stream** (`AgentEvents`, yielding `AgentEvent::Observations` or `AgentEvent::Removed`), because removals must arrive in order with observations and a session whose last agent starves gets no more observations.
- **An unknown agent definition is checked before the run's phase**, so it returns `unknown_agent_definition` even after Start.
- **Energy uses the protocol's `Unit` type** (0 to 1 inclusive), so out-of-range energy is malformed; an absent sense is left out rather than sent as null.
- **Every agent records effort; only agents with a metabolism spend it as energy** (maintainer, 2026-10-15), so later readers such as a fatigue sense work for any agent.
- **Agent definitions get their own spec, SPEC-008**, separate from environments; its capability reference is checked against the loader by serializing a definition with every capability.
- **The demo spawns `--agent forager` by default and leaves once all its agents are removed.** With the viewer's default 500 ms interval, a demo forager starves after roughly one to two minutes.

Contents:

- `agora-env`: `AgentDefinition`, `Metabolism`, `AgentDefinitionError`, `agents/forager.toml`, `bundled::{AGENTS, agents}`, `BundledError::Agent`.
- `agora-sim`: `spawn(placement, &AgentDefinition)`; `Energy`, `Exertion`, and `Metabolism` components; the Metabolism and Removal stage systems; `advance` returns `Advanced { removed, observations }`; `Observation::energy`; `MOVE_EFFORT`, `Removal`, `RemovalReason`.
- `agora-protocol` 0.8.0: `spawn.agent` (`AgentDefinitionId`), `Observation::energy`, `ServerMessage::AgentRemoved`, `RemovalReason`, `unknown_agent_definition`; new and updated fixtures.
- `agora-server`: the agent catalog (`FORAGER`), spawn lookup, removals dropped from sessions and pushed before observations, energy in observations.
- `agora-client`: `spawn(placement, agent)`, the agent event stream, the demo's `--agent` and leaving on removal. Viewer tests spawn foragers.
- Docs: SPEC-008 (new); SPEC-001 (R03, R07, R15, R21 updated; R22 to R26 new); SPEC-002 (0.8.0, R03, R08, R09, R11, R16 new); SPEC-003 (R01, R17 new); SPEC-004 (R02, R04, R05, R07 to R10, R12 new); CDD-001, CDD-002, CDD-003; the SADD's second milestone and package table; the roadmap's agent catalog direction; `rust/README.md`; the components index.
- Records: context (4.3 details), plan, work index, and this handoff.

Verification, in `rust/`:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --no-deps --workspace` with `RUSTDOCFLAGS=-D warnings`: all passed.
- `cargo test --workspace`: 205 passed, one full run (2026-10-15, after the effort change).
- Not done: a manual run with the viewer and demo clients.

## State

- M1 is complete and in `main` ([AGORA-004](../AGORA-004-first-milestone/handoff.md) has its follow-ups, several of which are now scheduled in the roadmap).
- Chunk 4.2's choices are recorded in #21 and SPEC-007; 4.1's in #20 and ADR-003; chunk 3's in #19 and SPEC-007; chunk 2's in #18 and SPEC-006; chunk 1's in #17 and SPEC-003.
- Design decisions are recorded in the SADD's [second milestone](../../docs/architecture/sadd.md#second-milestone-train-and-watch) section and CDD-001, with the reasoning and wire sketches in [context.md](context.md).
- The maintainer wants help tuning the energy numbers if they get in the way of training (chunk 11).
