---
id: SADD-001
status: draft
owner: maintainer
---

# Agora — SADD

This draft captures the project purpose and high-level requirements. Detailed architecture remains to be defined. Use the [SADD template](../templates/sadd.md) as the system design develops.

## Purpose and scope

Agora's purpose is to provide a simulation environment where clients can train and test AI agents, and observers can watch those agents inside the simulation.

The project has two main parts:

- A simulation framework written in Rust using the Bevy game engine. It will run the simulation, provide an API that clients can use to train and test AI agents inside it, and provide a way for observers to watch the agents.
- An example Python client that uses the simulation framework's API, with support tooling for training different model architectures and understanding their behavior. Python is provided because it is familiar to the maintainer for agent training; it is not a requirement for other clients.

Agora is the successor to `agora-poc`. The previous proof of concept is reference material for the migration; its behavior does not automatically define requirements for Agora.

The initial scope is a 2D, top-down simulation and visualization, supporting one or more agents in an environment. Different agent types can connect from different clients simultaneously, allowing agents to be observed or trained together. Visualization is optional: simulations can run without an observer, and observers can connect to an ongoing simulation or open a recorded run for playback.

The framework must support multiple independent simulation runs in parallel, beyond the first milestone's acceptance scope. The initial set of agent internal states and observation transformers will be simple, with more options added as the project matures. Broader simulation scenarios and project non-goals remain to be defined.

### First usable milestone

The first usable milestone is a small, empty grid simulation viewable through Bevy, with a simple client that connects and spawns an agent. The agent's only supported action is move. Two agents must be able to connect and run in the same simulation at once.

Each demo agent can move at most one unit per step. Future movement limits will be modifiable through agent properties. Two agents cannot occupy the same cell. Competing moves use the [initial conflict policy](#action-conflict-resolution): when both agents attempt to enter the same available cell, the first in that step's randomized execution order succeeds and the later conflicting move fails.

The framework returns an empty observation to each agent's client after each completed step. For this milestone, observations contain no agent internal state or perceived environment information; richer observations belong to later work. The empty observation's message representation remains to be specified.

The milestone is usable when:

- A user can open the simulation and see the small empty grid.
- The simple client can connect and spawn an agent, with two connected agents supported simultaneously in the same run.
- Both agents can submit move actions, each agent moves at most one unit per step, and the visualization shows their resulting positions.
- Agents never share a cell; competing moves into the same available cell are resolved by the initial conflict policy.
- Steps follow the [default coordination rules](#simulation-step-coordination) and the [initial conflict policy](#action-conflict-resolution).
- Each agent's client receives an empty observation after each completed step and can continue submitting moves over successive steps.

This milestone establishes the client-to-simulation action and observation loop. Model training, richer observations, environment generation and storage tooling, environment objects and machines, recording and playback, interactive state editing, and parallel environments are outside its acceptance scope. They remain part of the broader project scope described above.

The MVP uses one action per step: a cardinal move with distance 0 or 1 and a movement budget of 1. Distance 0 stays still. Environments own boundary behavior; the MVP supports bounded edges, where outward moves fail during execution. Spawning supports explicit or random unoccupied cells. The draft [simulation CDD](../components/simulation/cdd.md) records execution, membership, and perception decisions. Concrete messages and implementation interfaces remain to be specified.

### Core concepts

- **Environment:** The world in which agents act, built from the framework's environment components. Users can create, organize, and store environments for clients to use, either defining them themselves or using random generation tooling.
- **Agent:** A participant in a simulation whose actions are submitted by a connected client. An agent has internal state and properties that affect how it acts and what it can observe, such as speed and sight.
- **Simulation run:** An execution of an environment with one or more agents. The framework updates the simulation from agent actions and properties and can record the run for later playback.
- **Agent observation:** Information returned to a client for a particular agent, covering that agent's own internal state and its perception of the simulation. Observation transformers determine what the agent perceives according to its properties, such as its type of sight or smell.
- **Observer:** A user viewing an ongoing or recorded simulation through the framework's visualization. This view is distinct from the agent-specific observations sent to clients.

## System context

Agora is for people interested in training and observing AI agents interacting with different types of simulations and with each other. Users interact with the framework through clients and through its Bevy visualization.

Clients may use any language or client type, provided they fulfill the framework's message contracts. The included Python project is one example client.

Development will initially run the simulation and clients on the same computer, but client access must be independent of machine location. Longer term, users should be able to move simulation and training workloads to other computers to free their desktop. Future remote access should allow a user to give a friend client tooling and authentication credentials to train against Agora. Authentication and access-control details remain to be designed.

### Core workflows

- **Environment preparation:** A user creates an environment by editing a configuration file or through programmatic creation, including random generation, then organizes and stores it for clients to use. A visual environment editor is a stretch goal.
- **Training:** A client requests a specific or random training environment, spawns one or more agents, trains them, and disconnects when finished. The framework processes each agent's actions together with its properties to update the simulation, then returns agent-specific observations to the connected clients. Clients are responsible for updating their model weights, including when agents train together. Multiple clients training in different environments in parallel remain a desired capability beyond the first milestone.
- **Observation:** A user connects to an ongoing simulation and watches its state through a visualization provided by the framework using Bevy. The user can pause or run the simulation. Visualization is optional for simulation execution.
- **Large-environment viewing:** Eventually, support very large environments with pan and zoom, where only part of the environment is visible at a time. The MVP uses complete view-state data for its small grid. Whether larger environments require viewport-limited delivery or other data-transfer optimizations remains to be designed; camera visibility alone does not determine the delivery strategy.
- **Recording and playback:** Training runs should be recorded for later examination. A user can open a previous run and watch and inspect it through the framework's visualization. Resuming or branching execution from a recording is not required. Recording remains outside the first milestone.
- **Local behavior inspection:** A user starts the visualizer and spawns one or more local agents to watch their behavior, or connects independently started clients. Live runs with connected agents support environment modification; playback is for inspection.
- **Testing through intervention:** While observing, a user can modify the simulation state to see how agents react to the changes.

### Environment configuration and selection

The current design direction is an environment configuration API supporting programmatic environment authoring/configuration and requests for predesigned environments from a catalog. User-defined environments must be saved to the catalog and pass validation by Agora tooling before they can be used to create a run. Catalog admission provides the validation entry point and builds a reusable environment library. Uploading a new environment is a possible additional input path, not yet a committed requirement; if supported, it follows the same catalog admission requirement. Concrete API operations, environment representations, validation criteria, and catalog management remain to be designed. Grid dimensions are part of this broader configuration discussion; no specific configuration schema has been selected.

Simulation-provided random environment generation is a separate path and does not require the user to save a definition to the catalog first. Retain the generated initial environment definition with the run, together with all seeds and generation metadata needed to reproduce the environment. This includes generator identity/version, generation configuration, and any other inputs or dependency versions required for reproduction. The concrete metadata schema, storage format, and retention policy remain to be designed. By default, do not add generated environments to the catalog. Adding one is an explicit choice and follows the catalog's validation requirements. The catalog is intended for deliberately authored or selected environments, whether crafted by a person or a client; routine random generation must not flood it with entries. Recreating the initial environment does not by itself promise reproduction of the entire run's action execution.

The first milestone starts with a bundled, prevalidated catalog entry defining a bounded, empty 10 × 10 grid. Environment-authoring APIs, user catalog admission tooling, uploads, and random environment generation are deferred beyond that milestone. Random agent spawning within the supplied environment remains supported. A create-run request selects the environment by catalog entry ID. For the first milestone, the viewer automatically supplies the bundled entry's ID, without requiring a catalog-selection UI. The ID representation and concrete request schema remain to be specified.

### Runs, sessions, and connections

The server hosts multiple independent simulation runs in parallel. Each run supports multiple logical client sessions; each session belongs to one run. Independent clients can participate in the same run through separate sessions, without a mandatory coordinating super-client. A client participating in different runs uses a session for each run. Runs retain independent state, action readiness, and pacing rather than sharing an action barrier.

Each session has at most one active connection: one while connected, zero after connection loss. A client can establish a new connection and recover the same session with proof of ownership within the session recovery window. Recovery replaces the old connection rather than allowing both to remain active. Losing a connection does not immediately destroy the session or its request history. Agent-control handover continues to follow the step-boundary rules below; replacing a connection must not bypass them.

This hierarchy defines the required parallel-run capability without adding parallel execution to the first milestone. The entry flows below define run creation, joining, and session recovery. Concrete session establishment ordering, including correlation before a run-scoped session exists, remains to be designed.

Clients enter through three flows:

- **Create:** Select a catalog entry; the server creates the run and its first session, granting that session creator authority.
- **Join:** Supply an existing run ID; the server creates a new session in that run. Joining alone neither spawns an agent nor grants creator authority.
- **Resume:** Identify an existing session and attach a replacement connection to it under the recovery rules.

After creating or joining, clients can request agent spawning or subscribe as viewers. For the initial local milestone, a client connected to the server may join a run by its ID; join credentials, invitations, and access restrictions are deferred. This deferral applies to server-access permissions, not session-recovery credentials. Agents are owned by their logical session; no per-agent control tokens are issued. Recovering a session restores control of its owned agents under the handover rules. Session credentials are issued to clients permitted to establish sessions and prove ownership during recovery; they do not independently grant permission to access the server. Cross-session agent transfer is not required for the MVP. In the local MVP, server access is unrestricted as described above; a future access-permission layer is separate from session and agent recovery. Concrete bootstrap correlation and message ordering remain open.

### Run lifecycle

A successful create-run response contains the request correlation ID, assigned run ID, selected catalog entry ID, initial state ID (`0`), and phase (`setup`). The creator can share the run ID with joining clients. Creator-authority recovery remains to be designed separately; these result fields do not define a recovery credential.

A simulation run should normally close automatically when it is no longer being used for training or visualization. Completion must preserve its recording for later examination once recording is supported. Inactivity and disconnected-client handling follow the server coordination and recovery rules below; timeout durations remain to be specified. This requirement concerns an individual run; the lifetime of the hosting application remains to be defined.

New runs have an explicit setup phase at state 0. Clients can connect and agents can be spawned before execution is allowed; viewer data can show the assembled world during setup. An explicit start operation closes setup, delivers initial agent observations for the assembled state 0, and opens action collection. The run creator is the single holder of start authority, whether it is the viewer or a training coordinator. Joining clients cannot independently start the run. Connections alone do not start the run. Setup is distinct from pausing a started run, which retains ordinary step and membership rules. Start does not bypass the server's pacing gate. Representation and recovery of creator authority, and its interaction with observer pacing controls, remain to be specified.

Python client design must cover coordinating training across multiple clients: arranging participants in the same run, establishing that their preparation is complete, and explicitly starting the run so they begin together at state 0. The concrete coordination mechanism remains to be designed with the clients. Automatic start based on expected counts or participant slots is deferred. A single client managing multiple agents remains supported but is not required.

The server's processing order defines the setup/start boundary: spawns processed before Start are admitted during setup at state 0; spawns processed after Start follow normal post-execution admission rules. The coordinator must wait for successful spawn confirmations for all intended initial participants before requesting Start. Sending a spawn request before Start is not sufficient without confirmation that it was applied. How clients report these confirmations and preparation readiness to the Python coordinator remains client-design work.

Start preserves the selected pacing mode. While paused, Start delivers initial observations and opens action collection, but execution waits for Resume or Single-step. Timed viewing requires both action readiness and permission from the interval gate; unlimited mode permits advancement as soon as actions are ready. Start does not implicitly resume or execute a step.

Setup occurs only once for a run. If a started run becomes empty while still hosted, it remains started at its current state N. Admit new spawns without executing an empty step, deliver their initial observations for N, and make updated viewer state available when needed. Their actions target N and the next real step produces N+1. This does not reopen setup or require another Start; normal pacing and run-cleanup rules still apply.

For started runs, requests arriving during agent-action execution wait until all actions finish, then are handled before removals, spawns, and observations. Spawn/removal requests admitted at this boundary can affect the state about to be observed. Increment the state identifier after membership changes and before observations only when real execution occurred. Outside action execution, process requests promptly under each operation's rules: submissions enter collection, and populated-run membership changes remain queued for the post-action boundary. Started empty runs can apply spawns without waiting for execution, including while paused, and generate initial observations and viewer updates without incrementing the counter. Setup retains its initial admission rules. Pause gates action execution, not request processing. Operation-specific rules, including reconnection handover, still apply.

At the post-action boundary, capture the requests already waiting as a fixed batch. Process that batch before membership changes and observations; requests arriving after the cutoff wait for the next request-processing opportunity under their normal operation rules. Do not extend the batch with new arrivals or interrupt the remaining membership/observation work to process them. This prevents continuing arrivals from indefinitely delaying observations; the next processing opportunity need not require another action execution.

Apply membership requests in server acceptance order within each queue, processing all removals before spawns. Validate spawn placement against the world as each request is applied. If two valid spawn requests target the same available cell, the first accepted request occupies it and the later request fails with an occupied-cell error. Membership ordering does not change randomized agent-action execution order.

Spawn and removal requests use one client-facing completion rule across lifecycle phases: return the final success or failure when application is resolved, associated with the originating request. Queuing does not produce a separate acceptance acknowledgement. Setup and started-empty spawns return their result after immediate application; queued membership requests return their result after application at the step boundary. Clients use the same result semantics regardless of when application occurs. Requests rejected before application return their error directly.

A successful client-facing spawn result contains the request correlation ID and assigned agent ID. The creating session owns the agent; no agent-control token is returned. It omits spawn coordinates; placement inputs do not automatically become agent perception. Session ownership and control authority belong to the server. Concrete field representations remain to be specified in shared contracts.

A successful removal result contains the request correlation ID and removed agent ID, confirming that removal has completed. If an agent is already queued for removal, a repeated removal request returns an already-pending response without adding another queue entry or changing its position. A removal request for an agent that no longer exists returns an agent-not-found error. These responses apply to distinct requests; retries of the same request follow the request-correlation rules. Concrete response representations remain to be specified.

Agent removal during setup is not required; removal becomes available after Start under normal lifecycle rules. Start enables the normal timer policies, including their pause and pacing exclusions. The server rejects a Start command with an interface error if the run has no agents and no connected viewers when the command is processed. Rejection leaves the run in setup without initiating start-triggered timers or observations. A viewer-only run may start without agents; future non-agent environmental animation is a motivating use case, but environment-only stepping remains to be designed. If a started run has no use, the existing inactivity policy pauses advancement and eventually releases it. The server determines observer presence and enforces Start eligibility; the concrete error representation remains to be specified.

Connection mechanisms, concrete run creation and joining contracts, and control permissions remain to be defined. Catalog selection and random generation follow the [environment configuration and selection rules](#environment-configuration-and-selection). Whether random selection from the catalog is also supported, and how requests distinguish these modes, remain open.

### Agent profiles and launching

The starting design uses reusable agent profiles that identify a client program, model location, and settings. The visualizer launches clients on its own computer; they connect through the same client interface as independently started clients, including when the simulation is remote. A launched client owns model loading and inference, using Python, Rust, or another suitable runtime. The visualizer and simulation do not need to understand each model format.

Users should have explicit controls to stop launched clients or leave them running independently of the visualizer, allowing later reconnection to the run. Remote process launching is deferred. Profile format, process supervision, and shutdown interaction details remain to be specified. This approach can be revisited if it creates excessive friction.

### Extending agents and environments

Adding a new sense, agent property, or behavior may require writing Rust code and rebuilding Agora. Environment configuration and programmatic authoring can arrange and configure supported behaviors without adding a new implementation.

The framework must expose agent properties and senses through language-neutral descriptions and data contracts so non-Rust clients can discover and interpret them. Their types, configuration, and observation representations must not require access to Rust types or Bevy internals. The exact description format and compatibility rules remain to be specified in shared contracts.

## Components and responsibilities

The simulation framework is responsible for:

- Providing tools to create, organize, and store user-defined and randomly generated environments.
- Supplying simple environment building components, including walls, holes, food tokens, and machines such as doors, levers, and buttons.
- Supporting one or more agents, including different agent types connected from different clients simultaneously.
- Managing client-requested environments, agent spawning, and the lifecycle of runs, including automatic closure when no longer in use.
- Processing each agent's submitted action or actions together with its properties, such as speed, to update the simulation.
- Producing observations of each agent's own internal state and the simulation according to that agent's properties, such as sight, and returning them to its connected client.
- Providing all visualization infrastructure, live observer access, run/pause controls, and user modifications to simulation state.
- Recording simulation runs and providing playback.

Clients own model training, including weight updates. The Python project provides an example client and tooling for training different model architectures and understanding their behavior.

### Proposed Rust package mapping

The following is a proposal for maintainer review, not an accepted package split. No member packages exist yet. The rationale is recorded in [ADR-001](../decisions/0001-rust-package-boundaries.md). Each logical component will receive a [CDD](../components/README.md) as its design is developed.

| Planned package | Responsibility | First milestone |
| --- | --- | --- |
| `agora-protocol` | Language-neutral API representations: run and agent identifiers, requests, actions, observations, capability descriptions, live view data, and errors. Rust types implement canonical shared contracts. No simulation execution or model inference. | Minimal connection, spawn, move, empty observation, and live view messages. |
| `agora-sim` | Bevy simulation state and stepping, action validation, randomized conflict resolution, agent properties and senses, environment building components and generation. Produces agent observations and full state for viewing. No networking, process launching, or rendering. | Empty grid, spawning, bounded movement, exclusive cell occupancy, and step coordination. |
| `agora-server` | Host application and run management: client connections, run creation/joining, agent ownership, routing actions and observations, observer subscriptions and controls, cleanup, and later authentication and parallel runs. | One shared run with two agents from connected clients and live view access. |
| `agora-client` | Reusable Rust connection library and simple demo client executable. Supports the shared API without depending on simulation internals. Model-specific inference belongs in client implementations. | Spawn an agent and repeatedly send move actions and receive empty observations. |
| `agora-viewer` | Bevy visualization, live viewing and controls, playback UI, and later live editing and local client launching from profiles. | Render the live empty grid and both agents. |
| `agora-recording` | Recording format, writing and reading recorded state, and inspection/playback data access. No model execution or simulation resumption. | Deferred until recording work begins. |

Environment loading, generation, and built-in behaviors start as modules within `agora-sim`. Run management and environment storage orchestration start within `agora-server`. Agent profile launching starts within `agora-viewer`. These can become separate packages if their scope or reuse justifies it. The Python client remains a separate project implementing the same shared contracts; a Rust training package is not proposed.

Proposed internal dependencies: `agora-sim` and `agora-client` depend on `agora-protocol`; `agora-server` depends on `agora-sim` and `agora-protocol`; `agora-viewer` depends on `agora-client` and `agora-protocol`. When added, `agora-recording` depends on `agora-protocol`, and the server and viewer depend on it. Neither the viewer nor the recording package depends on simulation execution. A future recording schema may differ from live messages; that persistence contract must be designed explicitly.

The server owns live state changes, including changes requested through the viewer. The simulation owns per-run step coordination; the server supplies submissions from clients and handles connection lifecycle. Independent runs must not share an action barrier when parallel environments are added. View data is separate from agent observations, so the first milestone can render positions while returning empty observations to agents.

## Interactions and shared contracts

The client-facing API must be agnostic to client type and implementation language as long as the client fulfills its message contracts. The action and observation exchange described above will be defined in canonical [shared contracts](../contracts/README.md), used by the framework and its clients.

### Error responses

Request error responses use a common structure: the originating request's correlation ID, a stable error code for programmatic handling, a human-readable message, and structured details when relevant (for example, expected versus supplied state ID). Clients rely on the code and structured details rather than parsing message text. Concrete code names, detail schemas, and handling of malformed requests without a usable correlation ID remain to be specified in shared contracts.

### Simulation step coordination

Client requests that receive responses carry a client-generated correlation ID, which the server echoes in the corresponding success or error response. Generate the ID before sending and reuse it for retries, including when the original response was lost. Provided client libraries should manage request-ID generation for application code. Preserve this association through deferred processing, including queued spawn/removal application, so clients can match results to outstanding requests independently of response order. If a request has already completed, retrying it with the same correlation ID and the same request contents returns its stored result without executing it again. Reusing that ID for different request contents returns an error. If an identical retry arrives while the original request is still pending, it remains associated with the original operation and receives its final result when ready. Do not enqueue or execute another operation, change queue order, or send a separate queued acknowledgement. Request identity is the pair of logical client session and correlation ID. Different sessions may use the same correlation ID independently. A logical session survives connection loss so a recovering client can retrieve earlier results and resume pending requests. Session recovery requires proof of session ownership; a correlation ID alone does not authorize access. The server issues the logical session identity. Keep a logical session alive while connected. After disconnection, allow a configurable recovery window defaulting to five minutes. Recovery of an expired session returns an explicit session-expired error and requires establishing a new session. Session expiry triggers cleanup of all agents owned by that session; a new session cannot reclaim them. Session expiry remains separate from completed-result retention and whole-run cleanup. The session-establishment exchange and recovery credentials remain to be designed. Session expiry does not cancel or roll back already-admitted requests. Pending operations continue under normal lifecycle rules, preserving queue order and effects already applied; expiry does not force a paused run to advance. Cleanup uses normal removal boundaries for started runs and does not interrupt execution. Agents created by already-admitted spawns after session expiry must also be cleaned up. The exact interaction with queued spawns, setup (where explicit client removal is otherwise unavailable), and paused runs remains to be designed; these cases must not leave agents permanently orphaned. Completion does not restore an expired session or permit its recovery. Result disposal and any access from a new session remain to be specified. Retain completed results for a configurable retry window, defaulting to five minutes from request completion. Retrying a request does not extend its expiry; pending requests remain tracked until resolved and are not expired by the completed-result retention policy. Once a completed result expires, a retry returns an explicit result-expired error without executing the request again. The server must distinguish expired request IDs from new requests even after discarding result payloads. Correlation IDs are increasing request numbers within each logical session. Client libraries coordinate number allocation and send new requests in number order; responses may complete out of order. Preserve numbering across connection recovery rather than restarting it. The server tracks the highest admitted request number, pending requests, and retained completed results. Check pending and retained records first; a number at or below the highest admitted number with no remaining record must be rejected without execution. Expired retries receive the result-expired error. This permits discarding completed result payloads without retaining every old ID. Exact numeric representation, handling of skipped or out-of-order new numbers, recovery synchronization, and request-content comparison rules remain to be specified in shared contracts.

By default, each simulation step waits for every participating agent to submit its action or actions for that step before the framework advances the simulation and returns the resulting agent-specific observations. A client that responds sooner does not receive additional action opportunities while other agents are still preparing their submissions.

Client hardware performance and connection quality may affect how long a step takes in real time, but must not determine an agent's action rate within the simulation. The design must leave room for a future capability in which intentionally faster agents can take more actions than slower agents according to explicit simulation rules. Those rules, including how action rate relates to agent properties such as movement speed, remain to be defined.

A disconnected or stalled client enters reversible fallback control: the server supplies zero-distance actions and suppresses expensive observation generation, leaving its body in the world. Explicit removal is a separate operation. Failure detection and timeout values remain open; recovery follows the rules below.

Message formats, transport, detailed timing and recovery policies, and component interactions remain to be defined.

### Action conflict resolution

Each simulation can instantiate its own policy for resolving conflicting agent actions. Initially, the framework will supply only an ordering-based policy: after all agents have submitted their actions for a step, actions are processed in the simulation's execution order. When actions conflict, the first action succeeds and the later conflicting action fails. For example, if two agents try to collect the same food token, the first agent in execution order collects it and the second agent's collection action fails.

Under the initial policy, the framework randomly shuffles agent execution order for each simulation step. Execution order is independent of client message arrival order, so connection quality and hardware performance do not determine conflict priority. Execution failures are inferred from observations, without a separate result. The MVP permits one action per agent per step; multiple and composable actions are deferred.

### Server coordination and recovery

These decisions refine the earlier open lifecycle questions. The server owns connections, authority, clocks, pacing, and cleanup; the simulation owns participation, execution, and perception.

- Invalid submissions return interface errors, do not satisfy participation requirements, and do not reset deadlines. Execution failures produce no separate result: agents infer consequences from end-of-step observations. The MVP's observations remain empty.
- The first valid action submitted for an agent in a step is fixed: further ordinary submissions for that agent and step are rejected. Invalid submissions leave the agent free to submit a corrected action. Reconnection may discard the previous controller's pending action under the recovery rule below; explicit removal retains its defined replacement with a zero-distance action.
- Clients controlling multiple agents may submit batches with a shared target state identifier and an action entry for each agent. Acceptance is per agent: the server checks authority for each entry, and the simulation validates authorized submissions. Each entry receives acceptance or an error; valid submissions remain fixed even if other entries fail. Clients may submit all their agents together or use smaller batches, correcting and resubmitting rejected entries. Process the entire batch before considering advancement.
- Repeated entries for an agent follow the same fixed-submission rule, including within a batch: process entries in their supplied order, accept the first valid submission if none is already stored, and reject subsequent submissions for that agent and target state. An invalid entry does not prevent a later valid entry from being accepted. A duplicate does not undo an accepted action or reject unrelated agents' entries. Submission order does not determine randomized execution priority.
- For each completed state, the server groups agent observations by current client ownership and sends one observation batch per client, containing the resulting state identifier and observations keyed by agent ID for that client's agents. The simulation returns observations by agent and does not group them by connection. Clients may use the batch to compute their agents' next actions and submit those actions together or separately.
- A new run initializes at state 0 without executing a step: apply spawns during explicit setup and supply viewer data when needed. Explicit start finishes setup and generates initial agent observations for the assembled state 0. The server routes initial observations by client ownership. The first real step consumes actions targeting state 0 and produces state 1, as described in the simulation CDD.
- Submission deadlines count only time when pacing permits advancement and submissions are missing. Intentional pacing delays are excluded; pause suspends the response window. Connection-health checks operate independently of pacing.
- Reconnection proves ownership of the logical session using its recovery credential and restores control of the agents still owned by that session. It revokes the old connection even if it appears healthy. During collection, discard the old pending action and allow replacement without resetting the deadline. During execution, finish the step before handover. When handover completes, restore observation generation while preserving physical state and impairments. For a started run, generate and send the replacement client a fresh observation for the recovered agent at the current state ID, without advancing the simulation or incrementing the state ID. During setup, observations still wait until Start. This recovery delivery is additional to normal completed-state observation delivery. The session recovery result identifies the recovered agents and current run state/status with request correlation; its concrete aggregate representation remains to be specified. Return it after handover completes, confirming transferred control and completion of the pending-action reset where applicable. In a started run, the fresh current-state observation accompanies this result; during setup it is absent until Start. Concrete representations and session-credential lifecycle remain open.
- A separate server-maintained pacing gate permits a ready step to execute. Controls are pause, single-step, a selected maximum rate, and unlimited advancement. An executing step finishes before pausing. Client readiness may reduce the achieved rate.
- Live viewing defaults to a minimum interval of 500 ms between step starts (at most two steps per second). This interval gates simulation advancement, rather than only slowing viewer playback, and is tunable to allow faster or slower viewing. The default is provisional pending testing with a working viewer. Pause and single-step support closer inspection; rendering frames do not set the simulation step rate. Exact timing of initial observer attachment and pacing changes remains to be specified.
- Viewer-state reads requested during execution wait until the entire step completes, then return the completed state including membership changes and its resulting state ID. During setup or collection, including while paused, reads return the current state promptly. Newly attached viewers follow the same rule; no partially updated execution state is exposed.
- Viewer pacing uses the selected time interval without waiting for acknowledgements that a viewer has displayed a state. A slow viewer does not add a display barrier to simulation advancement. For each viewer, keep only the newest unsent full-state update, replacing older pending snapshots. Slow viewers may skip intermediate states but must not accumulate a snapshot backlog or delay simulation advancement. This replacement policy applies only to viewer snapshots, not command responses or agent observations; their delivery rules remain to be specified.
- One observer holds pacing control at a time; others may view. If the controller disconnects while observers remain, preserve pacing and allow another to claim control. When the last observer leaves, use unlimited pacing if client-controlled agents remain.
- With neither client-controlled agents nor observers, pause advancement and start a resource-release timeout. Regaining a controlled agent or observer cancels it. Expiry releases the run, preventing reconnection to its former agents. Timeout values, control-claim arbitration, and hosting-application lifetime remain open.

## Cross-cutting concerns

Simulation execution must support operation without visualization and allow observers to connect to an ongoing run. Agent internal state representations and observation transformers should accommodate additional options as the project matures; the initial supported set remains to be defined.

To be defined: reproducibility, performance, compatibility, error handling, observability, and any security or deployment requirements relevant to the agreed scope.

## Decisions and open questions

- For the first usable milestone, what are the concrete message contracts and visualization controls?
- Which capabilities from the POC should be retained?
- How will simulations define agent actions and observations, including interactions between multiple agents?
- How will multiple actions from one agent be ordered within a step?
- How will simulations define conflict policies beyond the initial randomized sequential policy?
- How will future simulation-defined action rates allow faster agents to act more often independently of client hardware and connection quality?
- What connection-health checks, timeout values, and session-credential lifecycle implement fallback control and reconnection?
- Which agent internal states, properties, and observation transformers will be supported initially?
- What configuration format and programmatic interfaces will support environment creation, generation, organization, and storage, and what behavior will the initial environment components provide?
- Does requesting a random environment mean selecting a stored environment, generating a new environment, or supporting both?
- What Rust extension interfaces and language-neutral capability descriptions will expose new properties, senses, and behaviors?
- What profile format and process lifecycle controls will support visualizer-launched clients?
- How will clients join shared runs, and who may reset, pause, modify, or close them?
- What authentication and access controls will future remote training require?
- Which state changes will users be able to make during observation, and how will those changes interact with simulation execution?
- What will recordings contain, and what playback controls will be supported?
- Which model architectures and behavior analysis tools will the Python project initially support?

Record consequential choices in [ADRs](../decisions/README.md). Track POC evaluation in the [migration inventory](../migration/inventory.md).
