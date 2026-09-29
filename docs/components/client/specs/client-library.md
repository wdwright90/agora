---
id: SPEC-004
status: draft
owner: maintainer
parents: [CDD-003]
packages: [agora-client]
---

# Rust client library and demo

## Purpose and scope

This spec defines the observable behavior of the [Rust client](../cdd.md): connecting, establishing a session, making requests, receiving pushed messages, losing the connection, and the `agora-demo` executable. Message formats and server behavior belong to the [client protocol](../../../contracts/client-protocol.md) (SPEC-002) and the [server spec](../../server/specs/sessions-and-runs.md) (SPEC-003).

Session recovery and retries are not covered; the server does not support recovery yet.

## Terminology and preconditions

- **Stream:** a queue that delivers every item in order.
- **Latest-value handle:** a value that holds only the newest item; readers may skip intermediate items.

## Requirements

### Connection and session

- **SPEC-004-R01:** `Client::connect` opens a WebSocket connection to a `ws://` URL and completes the version handshake with the library's protocol version. A server rejection of the handshake is returned as a rejection carrying the server's error code.
- **SPEC-004-R02:** `create_run` and `join_run` consume the client and return a session, carrying the run ID, session ID, catalog entry, state ID, and phase the server reported, together with the session's observation stream. Clones of a session make requests on the same session.

### Requests

- **SPEC-004-R03:** The library assigns request IDs. The request that establishes a session is 1, and each later request takes the next number. Requests reach the server in number order, including when several tasks send requests through clones of one session at once.
- **SPEC-004-R04:** Each request method waits for its response and returns its result: an agent ID for `spawn`, per-entry results for `submit`, the current view for `watch`, and nothing for the others. A server `error` response is returned as a rejection carrying the server's error response.

### Pushed messages

- **SPEC-004-R05:** Observation batches are delivered through the observation stream in the order the server sent them, each with its state ID and the session's agents' observations. The stream can be read in one task while other tasks make requests through clones of the session.
- **SPEC-004-R06:** The view and pacing state are held in latest-value handles. Both are empty until the session watches. The `watching` response fills them before `watch` returns, and each `view_update` and `pacing_update` replaces them.
- **SPEC-004-R11:** `Session::kinds` returns the run's kinds once the session has watched, and nothing before. The `watching` response sets them before `watch` returns, for every clone of the session. They do not change afterwards.

### Connection loss

- **SPEC-004-R07:** When the connection closes, requests still waiting for a response, and requests made afterwards, fail with a closed-connection error. The observation stream ends after delivering any batches already received.

### Leaving and closing

- **SPEC-004-R09:** `Session::leave` ends the session and returns once the server confirms. The connection then closes: every clone's later requests fail with a closed-connection error, and the observation stream ends.
- **SPEC-004-R10:** `Session::close` closes the run for every session; the server's rejection (`not_creator`) is returned as a rejection. When the server closes the run, `Session::closed_reason` reports the reason, the observation stream ends, and requests fail with a run-closed error carrying the reason.

### Demo client

- **SPEC-004-R08:** `agora-demo` takes the server URL, a number of agents to spawn (default 1), an optional seed, and an optional step limit, followed by a command:
  - `create [--start-at M] [--unlimited]` creates a run on `empty-grid-10x10`, prints the run ID as the first line of standard output, and spawns its agents. It watches the run and starts it once the view shows M agents in total (default: its own agent count). With `--unlimited`, it claims pacing control and sets unlimited pacing before starting.
  - `join <RUN_ID>` joins the run and spawns its agents.

  The executable is built with the `demo` feature, which is on by default. After that, for each observation batch it submits one random cardinal step of distance 1 for every agent in the batch. It leaves the run and exits successfully once it observes the step limit's state, or when interrupted, and with a failure status on any client error. Logs go to standard error.

## Acceptance criteria and verification

Library tests are in `rust/agora-client/tests/client_library.rs`. Most run against an in-process server; R01's rejection and R07 use a scripted fake server. The demo test, in `rust/agora-client/tests/demo.rs`, runs the built executable twice against an in-process server; it requires the `demo` feature.

| Requirement | Check and expected outcome | Test |
| --- | --- | --- |
| R01 | Connecting to a server succeeds; a handshake rejection returns its code. | `r01_*` |
| R02 | Create and join report the same run, distinct sessions, the catalog entry, state 0, and setup. | `r02_*` |
| R03 | Twenty spawns from concurrent tasks, each with a clone of the session, all succeed; out-of-order numbers would be rejected as stale. Reintroducing out-of-order sending made this test fail. | `r03_*` |
| R04 | Start without agents and a spawn on an occupied cell return their codes; spawn, Start, and submit return their results. | `r04_*` |
| R05 | In a task of its own, the stream delivers five steps' observations for states 0 to 4, each with the session's agent. | `r05_*` |
| R06 | Handles are empty before watching, hold the view and pacing state after it, and follow later spawns and a pacing change. | `r06_*` |
| R11 | Kinds are absent before watching; after `watch`, a clone of the session returns the built-in kinds in order. | `r11_*` |
| R07 | After the fake server drops the connection, the waiting request, the stream, and a later request all report the closure. | `r07_*` |
| R09 | A session that leaves fails later requests as closed, the other session carries on, and the stream ends after its own leave. | `r09_*` |
| R10 | A joiner's close is rejected with `not_creator`; after the creator closes, the joiner's stream ends, its closed reason is `closed_by_creator`, and its requests fail with that reason. | `r10_*` |
| R08 | A `create --start-at 2 --unlimited` process and a `join` process, each with one agent and a 3-step limit, both exit successfully. | `r08_*` |
