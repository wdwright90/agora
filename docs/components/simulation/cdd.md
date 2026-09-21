---
id: CDD-001
status: draft
owner: maintainer
parent: SADD-001
packages: [agora-sim]
---

# Simulation — Component Design Description

## Responsibility and boundaries

The simulation owns a run's world state, participating agents, action collection and validation, sequential execution, environment effects, and perception generation. It follows the [SADD](../../architecture/sadd.md). Networking, connection authority, elapsed-time deadlines, pacing, and run cleanup belong to the server; rendering belongs to the viewer.

This draft records lifecycle decisions discussed with the maintainer. It does not accept the complete package proposal or authorize implementation. Rich capabilities and perception remain future design work beyond the empty-observation MVP.

## Package mapping

The proposed `agora-sim` package implements this component using Bevy, without rendering or networking. No package exists yet. The proposed dependency on `agora-protocol` follows the [package mapping](../../architecture/sadd.md#proposed-rust-package-mapping); message representations remain to be specified in canonical shared contracts.

## Internal structure

Proposed internal responsibilities are world and occupancy state, participant/action collection, action execution and effects, queued membership changes, and agent perception. Bevy ECS components and systems should support live changes to capabilities and senses. Concrete components, resources, schedules, and extension interfaces remain undecided.

Simulation effects update the world and agent capabilities immediately during execution. Submission validation is not a promise that an action will remain executable.

## Interfaces and dependencies

Conceptual operations include creating a run, submitting an action, querying readiness, advancing one ready step, queuing a spawn or removal, changing capabilities, enabling or suppressing observation generation, and retrieving agent observations and separate observer state. These are responsibilities, not finalized Rust signatures.

The server decides when a ready step may execute, using the [server coordination rules](../../architecture/sadd.md#server-coordination-and-recovery). The simulation owns participation and action readiness. Reconnection, client ownership, and clocks do not belong in simulation execution.

## Data flow and lifecycle

### Collection and execution

1. Collect one valid action from every participating agent. Validate whether the action and parameters are legal for that agent at submission time, independently of destination occupancy. Invalid submissions return interface errors and do not satisfy participation requirements.
2. After all submissions are ready and the server permits advancement, randomly shuffle agent execution order. Arrival order does not determine priority. Attribute-based ordering is deferred.
3. Execute actions sequentially against the current world. Recheck capabilities and world conditions at execution time, applying each action's effects immediately. An earlier action may immobilize a later agent, causing its already-accepted move to fail. A failed action consumes the turn. If B vacates a cell before A moves into it, A may enter; the reverse order causes A's move to fail.
4. Apply queued explicit removals, then queued spawns. Cells vacated by removals are available to spawns at this boundary.
5. Generate observations from the completed state and resulting senses, including initial observations for newly spawned agents. Skip expensive perception work for agents whose observation generation is suppressed.
6. Begin the next collection phase with the updated participant set.

An empty run must process spawns without waiting for action execution. It need not execute empty steps to admit its first participant.

### MVP actions and environment

Each agent submits one move action per step, with a cardinal direction and distance 0 or 1. The movement budget is 1. Distance 0 means staying still, satisfies the submission requirement, and ignores direction. The server uses this action for fallback control. Larger movement budgets and independent or composable actions are deferred.

Boundary behavior belongs to environment design. The MVP supports only a bounded grid: an outward move fails during execution, leaves the body in place, and consumes the turn. Agents cannot share a cell. Grid dimensions remain undecided.

### Spawning and explicit removal

Spawns request either an explicit cell or random unoccupied placement. Placement is checked when applied, after execution and removals. An occupied or out-of-bounds explicit cell produces an error; no available cell prevents random placement. A new agent receives an observation at that boundary and first acts in the following step. Spawn coordinates are placement inputs, not automatic agent perception.

Explicit removal is a general operation, separate from connection loss. During collection, a removal request replaces the agent's pending action with a zero-distance move, satisfying its submission requirement. During execution, a removal request leaves that execution unchanged. In both cases the body remains until post-execution removal, before observations. Multiple membership requests and their ordering within each queue remain to be specified.

### Control loss and recovery

Control loss leaves the body in the environment. The server supplies zero-distance actions and requests observation suppression, avoiding sense computation with no recipient. Suppression yields empty observations and does not make the body physically blind or remove its capabilities. Control loss is reversible; recovery restores observation generation while preserving actual world changes and impairments. Authority replacement and pending submissions follow the SADD's server coordination rules.

## Perception direction beyond the MVP

Agents infer execution success and failure from observations; no separate execution outcome or failure reason is returned. Invalid submissions still produce interface errors. The MVP's empty observations intentionally do not yet allow inference of movement outcomes.

Future observations combine external senses and internal feelings, exposing only information meaningful from the agent's perspective. Hunger, fatigue, restraint, and impaired vision may be perceived; hidden causes and raw simulation bookkeeping are not automatically exposed. Absolute world coordinates are not supplied by default.

Conditions affecting an agent's ability to act must have associated perceptible feedback. Each condition and its perception require intentional joint design. Feedback may convey restraint without revealing a trap or the rule that movement is disabled. Observations contain only end-of-step state: transient experiences that begin and end within a step are deferred and may leave no observable trace.

Distinguish configured normal capabilities, active effects, effective capabilities, and perceived condition. A naturally slow agent and a slowed agent with the same effective speed have different internal experiences: the latter perceives impairment relative to its baseline. Initially internal signals should be accurate but deliberately limited, translated rather than copied from simulation attributes. Concrete scales and schemas are not settled.

Sense absence differs from an existing sense becoming impaired. Partial impairment also matters. Smoke can affect spatial visibility rather than simply reduce a global sight statistic. The sight model and internal feedback must be designed together; exact modeling remains open. Future noisy or misleading internal perception is deferred.

## Design constraints and rationale

End-of-step observations put all agents at the same simulation-time boundary while allowing distinct perceptions. Sequential effects make action outcomes depend on the evolving world. Stable step execution does not freeze agent attributes: effects from earlier actions can change later actions' feasibility.

External live edits are required eventually, but their admission and scheduling relative to execution still need definition. No transient experience log, advanced action composition, or rich sense system is required for the MVP.

## Detailed specifications

No detailed simulation specs or message contracts have been written. Future specs should link canonical [shared contracts](../../contracts/README.md), rather than duplicate them.

## Open questions

- Grid dimensions, random seeds, and reproducibility requirements.
- Concrete Bevy state, schedules, and interfaces; external edit timing.
- Spawn queue ordering, random selection policy, and initial admission details.
- Movement representation and validation/error schema, including capability changes between submission and execution.
- Observation and live view representations; baseline perception schemas are later work.
- How movement budgets above 1, multiple actions, and composable actions should work after the MVP.
