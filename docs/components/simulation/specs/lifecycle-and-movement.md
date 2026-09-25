---
id: SPEC-001
status: draft
owner: maintainer
parents: [CDD-001]
packages: [agora-sim]
---

# Simulation lifecycle and movement

## Purpose and scope

This spec defines the observable behavior of the [simulation component](../cdd.md) that is currently implemented. It covers:

- creating a bounded grid
- spawning agents during setup
- Start
- action submission
- readiness
- advancing a step with MVP move actions
- viewer state

It is extended feature by feature. Each change updates these requirements together with the implementation and its tests.

The CDD and [SADD](../../../architecture/sadd.md) define further behavior that is not yet covered here and is not implemented:

- spawning or removing agents after Start, including queued membership changes and admission into a started empty run
- batch submission
- observation suppression and restoration
- clearing a pending action on reconnection
- session-expiry cleanup
- live edits

Server responsibilities are out of scope: connections, authority, pacing, deadlines, request correlation, and wire formats. Wire representations belong to shared contracts.

## Terminology and preconditions

- **State N:** the world-state identifier. A new simulation starts at state 0. Only a real step increments it.
- **Setup:** the phase before Start. **Started:** the phase after Start. Collection and started-empty are statuses of a started run.
- **Operations are synchronous.** Each call completes before the next is processed, so callers never observe the execution phase. The server serializes calls into one simulation.

## Requirements

### Creation and coordinates

- **SPEC-001-R01:** Creating a simulation takes a grid width, a height, and a run seed. The new simulation is in setup at state 0 with no agents. A zero width or height is rejected with a configuration error. So are dimensions whose cell count cannot be represented on the platform.
- **SPEC-001-R02:** Cells use unsigned `(x, y)` coordinates. `(0, 0)` is the south-west corner; `x` grows east and `y` grows north. North is `y + 1`, east is `x + 1`, south is `y − 1`, and west is `x − 1`. A cell is in bounds when `x < width` and `y < height`.

### Spawning

- **SPEC-001-R03:** During setup, spawning at an explicit cell succeeds when the cell is in bounds and unoccupied, and returns the new agent's ID. A cell outside the grid returns an out-of-bounds error, and an occupied cell returns an occupied error. Neither error changes the world. Spawning does not change the state ID.
- **SPEC-001-R04:** A random spawn chooses uniformly among all cells that are unoccupied when the spawn is applied. It draws from the run's spawn stream. If no cell is free, it returns a no-free-cell error and changes nothing.
- **SPEC-001-R05:** Agent IDs are unsigned 64-bit integers, assigned sequentially from 1 in successful spawn order and unique within the run. Failed spawns do not consume an ID.
- **SPEC-001-R06:** *(Interim.)* Spawning after Start returns a not-in-setup error. Post-Start admission, as described in the CDD, will replace this rule.

### Start

- **SPEC-001-R07:** Start succeeds only in setup, and only once. It opens collection targeting state 0 without executing a step or changing the state ID. It returns an empty observation for every agent, keyed by agent ID and labeled state 0. A second Start returns an already-started error. The simulation allows Start with zero agents. Rejecting a run with neither agents nor connected viewers is the server's job, and the simulation exposes its agent count for that check.

### Submission

- **SPEC-001-R08:** A submission contains an agent ID, a target state ID, and a move. The checks run in this order, and the first failure returns its error:
  1. The run must be started (*not-started*).
  2. The agent must exist (*unknown-agent*).
  3. The target must equal the current state (*wrong-target-state*, reporting the expected and supplied values).
  4. The agent must not already have an accepted action for this state (*already-submitted*).
  5. The action must be within the agent's limits. A distance above the movement budget of 1 is rejected with *distance-budget-exceeded*.

  Checks 1–4 are *submission errors*: the submission is malformed or doesn't fit the run's current state. Check 5 is an *agent-limit error*: a well-formed action that exceeds what this agent can do. The two categories are kept distinct so that limit checks can grow with agent properties. A rejected submission does not fill the agent's action slot.
- **SPEC-001-R09:** The first accepted submission for an agent in a state is fixed. Later submissions for that agent and state are rejected, and the accepted action is kept.

### Readiness

- **SPEC-001-R10:** A readiness query returns the current state ID and one of three statuses:
  - *setup*
  - *started-empty* (started with no agents)
  - *collecting*, which lists the agents still missing an accepted action in ascending ID order

  The run is ready to advance only when it is collecting and that list is empty.

### Advancement

- **SPEC-001-R11:** Advancing a run that isn't ready returns an error: not-started in setup, no-agents when started-empty, or missing-actions (listing the agents) while collecting. The error leaves the world, the state ID, and the accepted actions unchanged.
- **SPEC-001-R12:** Each step executes the accepted actions one at a time, in an order drawn from the run's shuffle stream. The shuffle is a uniform random permutation of the agents in ascending ID order. The order in which submissions arrived does not affect it.
- **SPEC-001-R13:** Each move runs against the world as earlier moves in the same step have left it:
  - Distance 0 stays in place and ignores direction.
  - Distance 1 moves to the adjacent cell in the given direction. It fails if that cell is out of bounds or occupied at that point in the step.
  - A failed move leaves the agent in place and uses up its action.
  - An agent can enter a cell that another agent vacated earlier in the same step.
  - No per-action outcome is returned.
- **SPEC-001-R14:** At most one agent occupies any cell at all times.
- **SPEC-001-R15:** After all moves execute, the state ID goes from N to N+1 and the accepted actions are cleared. Advancing returns an empty observation for every agent, keyed by agent ID and labeled N+1. Collection then targets N+1.

### Viewer state

- **SPEC-001-R16:** In any phase, the viewer state returns the current state ID, the grid dimensions, and every agent's ID and position in ascending ID order. It is separate from agent observations.

### Reproducibility

- **SPEC-001-R17:** Within the same build and platform, two simulations with the same configuration, including the seed, that receive the same ordered sequence of operations produce identical results and viewer states. Each run has its own random streams, derived from its seed with ChaCha8: stream 0 is reserved for environment generation, stream 1 is for spawning, and stream 2 is for shuffling execution order. Drawing from one stream does not affect another.

## Interfaces and data

The Rust API is `agora_sim::Simulation`. Operation names map to `new`, `spawn`, `start`, `submit`, `readiness`, `advance`, `view`, and `agent_count`. Error variants correspond to the error names above. These are internal package interfaces, not wire contracts.

## Acceptance criteria and verification

Tests are in `rust/agora-sim/tests/lifecycle_and_movement.rs`. Each test name starts with the requirement ID it checks.

| Requirement | Check and expected outcome | Test |
| --- | --- | --- |
| R01 | New simulation is in setup at state 0 with no agents; zero dimensions are rejected. | `r01_*` |
| R02 | Each direction moves along the documented axis. | `r02_*` |
| R03 | Explicit spawn succeeds; out-of-bounds and occupied cells fail without changes. | `r03_*` |
| R04 | Random spawns fill every free cell and then report no free cell; picks cover the free cells. | `r04_*` |
| R05 | IDs are 1, 2, … and failed spawns don't consume an ID. | `r05_*` |
| R06 | Spawning after Start fails. | `r06_*` |
| R07 | Start returns state-0 observations for all agents, works with zero agents, and can't be repeated. | `r07_*` |
| R08 | Each rejection returns its error, and a rejected submission leaves the slot open. | `r08_*` |
| R09 | A second submission for the same agent and state is rejected; the first action executes. | `r09_*` |
| R10 | Each status is reported, and missing agents shrink as actions are accepted. | `r10_*` |
| R11 | Advancing before the run is ready fails without changes. | `r11_*` |
| R12, R13, R14 | Blocked moves and edges fail. Contested cells and follow-the-leader moves produce both possible outcomes depending on shuffle order, and cells are never shared. | `r12_*`, `r13_*`, `r14_*` |
| R15 | The state ID increments, observations carry N+1, and actions clear. | `r15_*` |
| R16 | View contents in setup and after steps. | `r16_*` |
| R17 | Identical seeds and inputs give identical histories; different seeds can differ. | `r17_*` |

## Open questions

- Post-Start membership, removal, suppression, reconnection reset, and batch submission. See the CDD's open questions.
- Sharing types with `agora-protocol` is deferred. For the MVP the simulation keeps its own types, and the server maps them.
