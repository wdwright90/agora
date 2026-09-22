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

### Operation summary

This table consolidates the discussed behavior. Operations are conceptual responsibilities, not finalized Rust signatures or wire messages. Each operation addresses one simulation run; how the server selects that run is outside this interface. Unspecified fields and phase behavior are marked open rather than implied requirements.

Phases are **setup** (initial state 0 before Start), **collection** (a started run waiting for actions or pacing permission), and **execution** (advancement through actions, membership changes, and observations). A started run may also be empty. Pause is a server pacing setting, not a return to setup. Requests arriving during action execution wait until all agent actions finish, then are handled before membership changes and observations. At that boundary, capture a fixed batch of already-waiting requests; later arrivals wait until the batch and remaining membership/observation work finish. Outside action execution and completion of that boundary, process requests promptly under their operation-specific rules: populated-run membership changes remain queued, while empty-run admission can proceed without stepping, even while paused. Pause does not block request processing.

| Operation | Conceptual inputs | Output or effect | Lifecycle availability and constraints |
| --- | --- | --- | --- |
| Create simulation | Environment/run configuration; exact fields open | A new simulation in setup at state 0 | Creates a run; server retains creator identity and authority. |
| Request spawn | Explicit cell or random unoccupied placement; other agent-creation fields and request correlation format open | Placement success/failure; successful setup admission is confirmed after application | Setup: apply at state 0, defer agent observations until Start. Started and populated: queue for post-execution admission. Started and empty: admit at current state N and generate initial observations without stepping. |
| Start | Request on the run; creator authorization and eligibility checked by server | Close setup, generate state-0 observations keyed by agent ID, open collection | Setup only; occurs once. Server rejects with an interface error if there are no agents and no connected viewers when processed, leaving setup unchanged. Preserves pacing and executes no step. Concrete error representations remain open. |
| Submit action(s) | Target state N and ordered agent-ID/action entries; individual submissions carry the same information | Acceptance or error per entry; accepted actions remain stored | Collection: target must equal current state. First valid submission per agent is fixed. Process the whole batch before advancement; server checks authority. Whether the simulation exposes a batch call is open. |
| Query readiness | Run context | Current target state ID and IDs of agents missing valid actions | Defined for collection, independent of pacing. Setup/execution query behavior and the empty-run representation remain open; empty runs use spawn admission rather than empty steps. |
| Advance one step | Request on the run; server has checked pacing | Resulting state N+1, observations keyed by agent ID, and request-associated spawn/removal results | Ready collection. Execute actions, apply removals then spawns, increment state, generate observations. Missing actions cause rejection with no state or queue changes. No per-action execution outcomes. |
| Request explicit removal | Agent ID; request correlation format open | Queue removal; report its result with advancement | Available after Start, not during setup. Collection: replace pending action with zero-distance move. Execution: leave current execution unchanged. Remove after execution and before spawns/observations. |
| Clear pending action for reconnection | Agent ID; server has established replacement authority | Discard the former controller's pending action so the replacement can submit | Collection; does not reset the server's deadline. During execution, finish the step before handover. Exact acknowledgement shape is open. |
| Set observation generation | Agent ID and enabled/suppressed setting | Suppression skips expensive perception and yields empty observations without changing physical senses | Used for server-managed fallback and recovery. Application timing relative to execution remains open. |
| Obtain viewer state | Request on the run | Complete MVP view: current state ID, grid dimensions, agent IDs and positions | Available during setup and after completed steps, including while paused and on observer attachment. Updated after empty-run admissions. Reads during execution and delivery/buffering remain open. |
| Change capabilities / external live edit | Target and change description; schema open | Update simulation state; result shape open | Internal execution effects apply immediately. Admission and scheduling of external edits remain future design work, not a finalized MVP operation. |

Observations are outputs of Start, advancement, and admission into a started empty run; no separate observation-polling operation has been agreed. The server groups them by current client ownership under the [canonical routing rule](../../architecture/sadd.md#server-coordination-and-recovery). Viewer state is obtained separately. Connection handling, creator authority, deadlines, pacing controls, and cleanup are server responsibilities, not additional simulation operations.

### Submission, advancement, and state identification

The server decides when a ready step may execute, using the [server coordination rules](../../architecture/sadd.md#server-coordination-and-recovery). The simulation owns participation and action readiness. Reconnection, client ownership, and clocks do not belong in simulation execution.

Each action submission contains an agent ID, target state ID, and action. The target must match the current world state identifier; past and future targets are rejected. Action collection enforces the [fixed-submission rule](../../architecture/sadd.md#server-coordination-and-recovery): ordinary submissions cannot revise an accepted action. The server-directed reconnection reset and explicit-removal behavior are exceptions; invalid submissions do not fill the pending action slot. Concrete representations and errors remain to be specified.

Multi-agent batches follow the [per-agent acceptance and duplicate-entry rules](../../architecture/sadd.md#server-coordination-and-recovery). A shared target state applies to the batch's agent/action entries; authorized entries are validated in supplied order, and accepted actions remain stored when another entry is rejected. Once a valid submission is stored, further entries for that agent and target state are rejected, including in the same batch. Batch processing completes before advancement is considered. Whether this uses a simulation batch operation or server-orchestrated individual calls remains an implementation choice. Concrete error representations remain to be specified.

The readiness interface exposes the current step identifier and the identifiers of participating agents still missing a valid action. The simulation is the source of this submission state; the server uses it to manage deadlines and supply fallback actions. For a participating run, readiness means all required actions are present, independently of the server's pacing gate. A paused run may remain ready. Empty-run spawn admission follows the separate lifecycle rule below.

Advancing one ready step completes execution, queued removals and spawns, and observation generation. It returns the completed step identifier, agent observations keyed by agent ID (including initial observations for newly spawned agents), and results of queued spawns and removals associated with their requests. It does not return per-action execution outcomes. Viewer state is retrieved separately. An advance request before all required actions are present returns an error without changing the world or pending submissions and queues. The server is responsible for checking its pacing gate before requesting advancement. Concrete result and error representations remain to be specified.

Step identifiers follow world-state numbering: submissions target the current state N. After execution and queued membership changes, increment the state identifier to N+1 before generating observations. Agent observations and viewer state for the resulting world identify N+1; the completed step identifier in the advancement result is this resulting state identifier. Readiness then reports N+1 as the target for the next collection phase. A new run starts at state 0; initialization below does not increment it. Admission into a started run that has become empty also preserves its current state identifier.

The simulation returns observations keyed by agent ID. The server groups and routes them as one batch per client per completed state according to the [observation delivery rule](../../architecture/sadd.md#server-coordination-and-recovery); client ownership and connection grouping remain outside the simulation.

Viewer state is supplied separately from the step-advancement result. For the MVP, the viewing interface supplies a complete view of the current state: grid dimensions, the current world-state identifier, and each agent's identifier and position. The server can obtain this view after advancement or when an observer connects, including while paused. The Bevy integration and view delivery mechanism remain to be designed.

Full-state views are the MVP starting point. The [longer-term viewing requirement](../../architecture/sadd.md#core-workflows) includes very large environments with pan and zoom; viewport-limited delivery, incremental updates, and other scaling mechanisms remain open and are not required for the MVP.

Live viewing gates simulation advancement using the server's tunable [pacing policy](../../architecture/sadd.md#server-coordination-and-recovery), including its provisional default interval. The simulation does not own this wall-clock timer or use rendering frame rate as its step rate.

## Data flow and lifecycle

### Collection and execution

1. Collect one valid action from every participating agent. Validate whether the action and parameters are legal for that agent at submission time, independently of destination occupancy. Invalid submissions return interface errors and do not satisfy participation requirements.
2. After all submissions are ready and the server permits advancement, randomly shuffle agent execution order. Arrival order does not determine priority. Attribute-based ordering is deferred.
3. Execute actions sequentially against the current world. Recheck capabilities and world conditions at execution time, applying each action's effects immediately. An earlier action may immobilize a later agent, causing its already-accepted move to fail. A failed action consumes the turn. If B vacates a cell before A moves into it, A may enter; the reverse order causes A's move to fail.
4. Capture and process a fixed batch of requests already waiting at the post-action boundary, then apply explicit removals followed by spawns, including requests admitted from that batch. Later arrivals wait for the next processing opportunity after this step's membership and observation work. Cells vacated by removals are available to spawns.
5. Increment the world-state identifier from N to N+1, then generate observations from the completed state and resulting senses, including initial observations for newly spawned agents. Skip expensive perception work for agents whose observation generation is suppressed.
6. Begin the next collection phase with the updated participant set.

An empty run must process spawns without waiting for action execution. It need not execute empty steps to admit its first participant.

If a started run becomes empty, it does not reenter setup or require another Start. With no actions executing, process requests promptly, apply empty-run spawns, generate initial observations for admitted agents, and expose updated viewer state when needed. This admission is permitted while paused and does not require an execution cycle. Preserve state N: only real execution increments the counter. New agents target N, and the next real execution produces N+1. Once populated, the run follows the normal queued-membership rules. The server continues enforcing pacing and run-cleanup rules.

### Initialization

A new run begins in explicit setup at state 0, following the [run lifecycle](../../architecture/sadd.md#run-lifecycle). Clients can connect and initial spawns are applied without executing steps or incrementing the state identifier. Supply viewer state for state 0 when needed during setup. An explicit start operation closes setup, generates initial observations for the assembled state 0, and opens action collection. The first action collection targets state 0; the first real step produces state 1. The server groups initial observations by client using the same ownership-based routing as completed-step observations. Setup is distinct from pausing an already-started run; pause retains ordinary collection and queued-membership behavior. Concrete initialization operations remain to be specified.

The server's processing order determines admission at the setup/start boundary: spawns processed before Start are applied at state 0; those processed afterward follow normal post-execution admission. A setup spawn confirmation reports successful application, allowing the coordinator to wait for all intended initial participants before starting, as required by the SADD's run lifecycle rules.

The server enforces the run creator's sole start authority under the SADD's run lifecycle rules. The simulation's initialization interface does not own creator identity or connection authorization.

Start preserves server pacing: initialization and action collection can complete while paused, but advancement still requires the server's pacing gate. Start itself does not execute a step or implicitly resume a paused run.

Setup does not require agent removal; use normal removal after Start. The server permits viewer-only starts but rejects Start with an interface error when neither agents nor connected viewers exist, following the SADD's lifecycle rules. Rejection leaves the run in setup without start-triggered timers or observations. Normal timer policies apply after successful Start. Future environment-only execution without agents remains open; allowing a viewer-only start does not by itself define empty-step advancement.

### MVP actions and environment

Each agent submits one move action per step, with a cardinal direction and distance 0 or 1. The movement budget is 1. Distance 0 means staying still, satisfies the submission requirement, and ignores direction. The server uses this action for fallback control. Larger movement budgets and independent or composable actions are deferred.

Boundary behavior belongs to environment design. The MVP supports only a bounded grid: an outward move fails during execution, leaves the body in place, and consumes the turn. Agents cannot share a cell. Grid dimensions remain undecided.

### Spawning and explicit removal

Spawns request either an explicit cell or random unoccupied placement. Placement is checked when applied. In a started, populated run, apply queued spawns after execution and removals; a new agent receives an observation at that boundary and first acts in the following step. Setup and started-empty-run admission follow the lifecycle rules above. An occupied or out-of-bounds explicit cell produces an error; no available cell prevents random placement. Spawn coordinates are placement inputs, not automatic agent perception.

Explicit removal is a general operation, separate from connection loss. During collection, a removal request replaces the agent's pending action with a zero-distance move, satisfying its submission requirement. During execution, a removal request leaves that execution unchanged. In both cases the body remains until post-execution removal, before observations. Under the SADD's [removal response rules](../../architecture/sadd.md#run-lifecycle), a repeated request for an agent already queued for removal returns already-pending without adding or reordering a queue entry; a request for an agent that no longer exists returns agent-not-found.

Follow the SADD's [membership ordering rule](../../architecture/sadd.md#run-lifecycle): process each queue in server acceptance order, with all removals before spawns. Placement is validated against the world as each spawn is applied, so the first valid request for an available cell occupies it and later requests for that cell fail with an occupied-cell error. This ordering is separate from randomized agent-action execution.

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
- Random placement policy, agent-creation fields, and request correlation representations.
- Concrete errors for operations outside their allowed phase or an ineligible Start; future environment-only advancement with no agents.
- Readiness query behavior outside collection and operation-specific observation-suppression timing.
- Concrete initialization and empty-run admission result shapes; state IDs identify step progression, not every setup or empty-run membership change.
- Movement representation and validation/error schema, including capability changes between submission and execution.
- Observation and live view representations; baseline perception schemas are later work.
- How movement budgets above 1, multiple actions, and composable actions should work after the MVP.
