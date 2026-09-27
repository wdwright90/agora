---
id: CDD-002
status: draft
owner: maintainer
parent: SADD-001
packages: [agora-server]
---

# Server — Component Design Description

## Responsibility and boundaries

The server hosts simulation runs and connects clients to them. It owns connections, sessions, creator authority, agent ownership, request correlation, run and session lifetimes, pacing, observation routing, viewer delivery, and later deadlines. It follows the [SADD](../../architecture/sadd.md), in particular [runs, sessions, and connections](../../architecture/sadd.md#runs-sessions-and-connections), the [run lifecycle](../../architecture/sadd.md#run-lifecycle), and [server coordination and recovery](../../architecture/sadd.md#server-coordination-and-recovery).

The simulation owns world state, action validation and execution, and perception ([CDD-001](../simulation/cdd.md)). The server calls it but adds no action-specific behavior.

This draft covers what is built so far: the WebSocket host, the handshake, run creation and joining, spawning, Start, action submission, step advancement, observation routing, viewers and pacing, and session and run lifetimes.

## Package mapping

The `agora-server` package (`rust/agora-server`) implements this component as a library and an executable of the same name. It depends on `agora-sim` and `agora-protocol` and maps between their types, as the [package mapping](../../architecture/sadd.md#rust-package-mapping) describes. It uses tokio for asynchronous I/O and `tokio-tungstenite` for WebSockets.

## Internal structure

- **Listener:** accepts TCP connections and starts one connection task for each.
- **Connection task:** performs the WebSocket upgrade, then reads frames one at a time. It holds the handshake state and, once established, the connection's session: its ID, its run's handle, and its request log. Each request is answered before the next frame is read, so responses are in request order. It also owns an outbox channel, which it gives the run when its session is established. Between frames it writes whatever the run has pushed there. A request is answered before the outbox is read again, so a push caused by a request follows its response. On `watch`, it also gives the run a view slot, which holds only the newest view; it sends each new view from the slot the same way.
- **Request log:** tracks a session's highest admitted request ID and its most recent results, and decides whether a request is new, a retry to replay, or a rejected ID ([SPEC-002-R07](../../contracts/client-protocol.md#requests-and-sessions)).
- **Run task:** one per run. It owns the run's `Simulation`, its sessions (creator flag, owned agents, outbox and view slot while connected, and expiry deadline), the run's release deadline, and the time of its last step for the step interval. It handles commands from connection tasks one at a time through a channel, which serializes every call into the simulation as [SPEC-001](../simulation/specs/lifecycle-and-movement.md) requires. Each run has its own task, so runs never share an action barrier.
- **Registry:** the table of live runs by ID, shared by all connections. A run task removes itself when the run is released.
- **Catalog:** the bundled environment entries. There is one, `empty-grid-10x10`.

## Interfaces and dependencies

Clients use the [client protocol](../../contracts/client-protocol.md) (SPEC-002) over WebSocket. The executable takes the listen address and the two timeouts on its command line.

Connection tasks talk to run tasks through a handle with asynchronous `join`, `spawn`, `start`, `submit`, `watch`, and `disconnect` operations. Run tasks push messages to sessions through their outboxes and publish views to viewers' slots. A handle to a released run fails, and the connection reports `unknown_run`.

## Data flow and lifecycle

- **Create:** the connection looks up the catalog entry, then creates the simulation with a seed from OS randomness. It registers the run and starts its task with the creator's session. The seed is logged so a run can be reproduced.
- **Join:** the connection finds the run in the registry and asks its task for a new session.
- **Spawn and Start:** the run task applies them to the simulation, records ownership, and checks creator authority and Start eligibility. Start delivers the initial observations.
- **Submit:** the run task checks the phase and target state for the whole request, then checks ownership and submits each entry to the simulation in order. It replies with the per-entry results, then advances if the run is ready.
- **Advance and deliver:** when every agent has an accepted action, the run task asks the pacer whether the step may start. If so, it executes the step and pushes each connected session its own agents' observations. If an interval holds the step, it sets a deadline and checks again when the deadline fires; if the run is paused, it waits for a pacing request.
- **Pacing:** the pacer holds the mode, the controller, any granted single step, and when the last step started. Claims, mode changes, and grants from connections update it, and each change is pushed to every viewer as that viewer sees it.
- **Watch and publish:** a watching session receives the current view in its response. After every spawn, Start, step, and expiry removal, the run task publishes the new view to each viewer's slot, replacing any view not yet sent.
- **Disconnect:** when a connection closes, its session stays in the run without a connection, and its outbox and view slot are dropped. The session's expiry timer starts. If it held pacing control, control passes to the remaining viewer with the oldest connection. If it was the last viewer, pacing becomes unlimited and a waiting step executes.
- **Session expiry:** when the timer fires, the session is removed and its agents are removed from the simulation. If the run was waiting only for those agents, the step executes.
- **Run release:** when the run has no sessions left, its release timer starts. A join cancels it. When it fires, the run is removed from the registry and its task ends.

Session recovery will let a client reattach a connection to its session before expiry, cancelling the timer. Until then, a disconnected session cannot be resumed.

## Design constraints and rationale

- **Mapping between protocol and simulation types.** The server converts between `agora-protocol` and `agora-sim` types, so wire concerns stay out of simulation code, as the [package mapping](../../architecture/sadd.md#rust-package-mapping) requires. The mapping is intended, not temporary. The identifier conversions and error-code mappings stay, because clients' error codes are the server's decision. The hand-written action conversion is expected to be replaced by a generic path when capability definitions arrive, which will decode an action's parameters into a simulation action (raised in review of PR #7, 2026-09-26).
- **One task per run.** This gives each run its own world and execution schedule, as the SADD requires, without locks around the simulation. It leaves room for pacing and parallel runs later.
- **Release after sessions, not agents.** The SADD's release rule is "neither client-controlled agents nor observers". Tying release to sessions instead means a connected session keeps its run alive even before it spawns anything, such as a coordinator setting up. A disconnected session keeps the run until the session expires. The maintainer chose this model on 2026-09-25, and the SADD rule is updated to match.
- **Advance on readiness, gated by pacing.** The run checks readiness after each submission, expiry, viewer disconnect, and pacing change, and when a held step's deadline fires.
- **Pacing in its own module.** The pacer (`pacing.rs`) is a small state machine with no I/O, so the run task only asks it whether a step may start and tells it what happened.
- **Connection numbers for handover.** Connections are numbered as they are accepted, and each viewer records its connection's number. The SADD orders handover by connection age, not session or watch age.
- **Newest-only views.** Views are published through a Tokio `watch` channel per connection, which keeps only the latest value. This implements the SADD's rule that a slow viewer skips states instead of building a backlog or delaying the run, with no extra bookkeeping.
- **Unbounded outboxes.** The SADD rejects an application-level bounded queue for agent clients. A client that stops reading stalls its own agents' submissions, which holds the step, so its backlog stays small. Viewer views use a newest-only slot instead.
- **Request log in the connection.** A session cannot yet outlive its connection's usefulness, because there is no recovery, so the log lives with the connection task. It moves into shared session state when session recovery is built.
- **Provisional timeouts.** Sessions expire 2 minutes after disconnecting, which is the SADD's recovery window, and runs are released 5 minutes after their last session ends. Both are configurable.

## Detailed specifications

- [SPEC-003 — Server sessions and runs](specs/sessions-and-runs.md) (draft): the catalog, identifiers, authority, Start eligibility, session expiry, and run release.
- [SPEC-002 — Client protocol](../../contracts/client-protocol.md) (draft): the wire contract the server implements.

## Open questions

- **Closing a setup run whose creator expires:** deferred until a closure message exists. Such a run can no longer start, and it is released once its remaining sessions expire.
- **A connected client that stops submitting** holds its run's steps until deadlines and fallback control are built. Disconnecting ends the stall once the session expires.
- **Retries of `create_run` and `join_run` after a lost response** need session recovery (see SPEC-002).
- **Deadlines and fallback control** are designed in the SADD but not built.
- **Viewer-only runs** can start but not step, because stepping without agents is not defined.
