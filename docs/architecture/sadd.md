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

Running agents training in different environments in parallel is a stretch goal. The initial set of agent internal states and observation transformers will be simple, with more options added as the project matures. Broader simulation scenarios and project non-goals remain to be defined.

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

The MVP uses one action per step: a cardinal move with distance 0 or 1 and a movement budget of 1. Distance 0 stays still. Environments own boundary behavior; the MVP supports bounded edges, where outward moves fail during execution. Spawning supports explicit or random unoccupied cells. The draft [simulation CDD](../components/simulation/cdd.md) records execution, membership, and perception decisions. Grid dimensions, concrete messages, and implementation interfaces remain to be specified.

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
- **Recording and playback:** Training runs should be recorded for later examination. A user can open a previous run and watch and inspect it through the framework's visualization. Resuming or branching execution from a recording is not required. Recording remains outside the first milestone.
- **Local behavior inspection:** A user starts the visualizer and spawns one or more local agents to watch their behavior, or connects independently started clients. Live runs with connected agents support environment modification; playback is for inspection.
- **Testing through intervention:** While observing, a user can modify the simulation state to see how agents react to the changes.

### Run lifecycle

A simulation run should normally close automatically when it is no longer being used for training or visualization. Completion must preserve its recording for later examination once recording is supported. Inactivity and disconnected-client handling follow the server coordination and recovery rules below; timeout durations remain to be specified. This requirement concerns an individual run; the lifetime of the hosting application remains to be defined.

Connection mechanisms, run creation and joining rules, and control permissions remain to be defined. A client's request for a random environment may mean choosing a stored environment or generating a new one; this distinction remains open.

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

### Simulation step coordination

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
- Submission deadlines count only time when pacing permits advancement and submissions are missing. Intentional pacing delays are excluded; pause suspends the response window. Connection-health checks operate independently of pacing.
- Reconnection identifies the existing agent and proves control using an opaque control token issued at creation. It revokes the old connection even if it appears healthy. During collection, discard the old pending action and allow replacement without resetting the deadline. During execution, finish the step before handover. Restore observation generation while preserving physical state and impairments. Contract details and token lifecycle remain open.
- A separate server-maintained pacing gate permits a ready step to execute. Controls are pause, single-step, a selected maximum rate, and unlimited advancement. An executing step finishes before pausing. Client readiness may reduce the achieved rate.
- One observer holds pacing control at a time; others may view. If the controller disconnects while observers remain, preserve pacing and allow another to claim control. When the last observer leaves, use unlimited pacing if client-controlled agents remain.
- With neither client-controlled agents nor observers, pause advancement and start a resource-release timeout. Regaining a controlled agent or observer cancels it. Expiry releases the run, preventing reconnection to its former agents. Timeout values, control-claim arbitration, and hosting-application lifetime remain open.

## Cross-cutting concerns

Simulation execution must support operation without visualization and allow observers to connect to an ongoing run. Agent internal state representations and observation transformers should accommodate additional options as the project matures; the initial supported set remains to be defined.

To be defined: reproducibility, performance, compatibility, error handling, observability, and any security or deployment requirements relevant to the agreed scope.

## Decisions and open questions

- For the first usable milestone, what are the grid dimensions, concrete message contracts, and visualization controls?
- Which capabilities from the POC should be retained?
- How will simulations define agent actions and observations, including interactions between multiple agents?
- How will multiple actions from one agent be ordered within a step?
- How will simulations define conflict policies beyond the initial randomized sequential policy?
- How will future simulation-defined action rates allow faster agents to act more often independently of client hardware and connection quality?
- What connection-health checks, timeout values, and control-token lifecycle implement fallback control and reconnection?
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
