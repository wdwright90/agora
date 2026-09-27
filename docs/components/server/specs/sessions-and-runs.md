---
id: SPEC-003
status: draft
owner: maintainer
parents: [CDD-002]
packages: [agora-server]
---

# Server sessions and runs

## Purpose and scope

This spec defines the [server's](../cdd.md) behavior that is not part of the wire contract. It covers:

- the bundled catalog
- run and session identifiers
- creator authority and agent ownership
- Start eligibility
- step advancement and observation routing
- viewers, view delivery, and the step interval while viewers watch
- session expiry, including removing an expired session's agents, and run release

It is extended feature by feature. Messages, framing, the handshake, request IDs, and error codes belong to the [client protocol](../../../contracts/client-protocol.md) (SPEC-002).

The SADD and CDD describe further behavior that is not yet covered here and is not implemented:

- session recovery
- closing a run in setup when its creator expires
- pacing controls (pause, single-step, rate changes, and controller claims), deadlines, and fallback control

## Terminology and preconditions

- **Session:** created by `create_run` or `join_run`. A session is *connected* while the connection that created it is open.
- **Session expiry timeout** and **run release timeout:** server settings. They default to 2 minutes and 5 minutes, and the executable sets them with `--session-expiry-secs` and `--run-release-secs`.
- **Viewer:** a connected session that has sent `watch`. It stays a viewer until its connection closes.
- **Step interval:** a server setting, the minimum time between step starts while a run has a viewer. It defaults to 500 ms, which is the SADD's provisional live-viewing default, and the executable sets it with `--step-interval-ms`.

## Requirements

### Catalog and identifiers

- **SPEC-003-R01:** The catalog has one entry, `empty-grid-10x10`: an empty, bounded grid 10 cells wide and 10 cells high. Each run's simulation seed comes from OS randomness and is written to the server log.
- **SPEC-003-R02:** Run IDs and session IDs are assigned by the server and are unique. Clients treat them as opaque strings. Each successful `join_run` creates a new session.

### Authority and ownership

- **SPEC-003-R03:** The session created by `create_run` holds creator authority, which gives it sole permission to Start. Start from any other session returns `not_creator`. The session that spawns an agent owns it. Only the owner can submit actions for an agent; other sessions' entries for it are rejected with `agent_not_owned`, and the owner can still submit.

### Start

- **SPEC-003-R04:** Start from the creator is rejected with `start_not_eligible` when the run has no agents and no connected viewers. The run stays in setup. A viewer whose connection has closed does not count. A run started by viewers alone has no agents, so it waits started-empty; stepping without agents is not defined yet. Once the run has started, a further Start returns `run_already_started`.
- **SPEC-003-R05:** *(Interim.)* Spawning after Start returns `run_already_started`, following [SPEC-001-R06](../../simulation/specs/lifecycle-and-movement.md#spawning). Post-Start admission will replace this rule.

### Steps and observations

- **SPEC-003-R08:** A started run executes a step once every agent in it has an accepted action for the current state, subject to the step interval (R11). Rejected entries do not count. The step is triggered by the submission that supplies the last missing action, or by the removal of the last agents without one (R06). There is no deadline yet.
- **SPEC-003-R09:** Observations are routed by ownership. After Start and after each step, each connected session that owns agents receives one `observations` push with exactly its own agents. Sessions without agents, and sessions without a connection, receive nothing; a disconnected session's observations are not kept.

### Viewers

- **SPEC-003-R10:** Any session can become a viewer with `watch`, including one that owns agents. The response carries the current view. Afterwards the viewer receives a `view_update` after every change to the view: a spawn, Start, a step, and the removal of an expired session's agents. Each viewer holds only the newest view it has not yet been sent, so a slow viewer skips intermediate views rather than queueing them or delaying the run.
- **SPEC-003-R11:** *(Interim, until pacing controls.)* While a run has a viewer, a step starts no sooner than the step interval after the previous step started. A step that is ready earlier waits and then executes without further input. Without viewers there is no interval, which is the SADD's unlimited pacing. When the last viewer disconnects, a waiting step executes at once.

### Lifetimes

- **SPEC-003-R06:** When a session's connection closes, the session stays in its run without a connection. It expires once the session expiry timeout has passed, and is then removed from the run. Its agents are removed from the world immediately ([SPEC-001-R18](../../simulation/specs/lifecycle-and-movement.md#removal)), in any phase, and a step that was waiting only for them then executes (R08). *(Interim.)* A creator expiring during setup does not close the run.
- **SPEC-003-R07:** A run is released once it has had no sessions for the run release timeout. The timer starts when the run's last session expires, and a `join_run` before it fires cancels it. After release, `join_run` for that run returns `unknown_run`. A run with a connected session, or with a disconnected session that has not yet expired, is never released. Agents and pending actions do not keep a run: a run waiting for actions is released like any other.

## Interfaces and data

The library's `agora_server::serve` accepts connections on a `TcpListener` with a `ServerConfig` that holds the two timeouts and the step interval. The `agora-server` executable wraps it and takes `--listen` (default `127.0.0.1:7878`), the two timeout flags, and `--step-interval-ms`.

## Acceptance criteria and verification

Tests are in `rust/agora-server/tests/sessions_and_runs.rs` and start a server on an unused local port. Each test name starts with the requirement ID it checks. Timer tests use timeouts of 20 ms and wait 400 ms.

| Requirement | Check and expected outcome | Test |
| --- | --- | --- |
| R01 | Corner cells of the bundled grid spawn; cells at x = 10 or y = 10 are out of bounds. | `r01_*` |
| R02 | Run IDs and session IDs from several creates and joins are all distinct. | `r02_*` |
| R03 | A joining session's Start returns `not_creator`; the creator's succeeds. A non-owner's entry returns `agent_not_owned`; the owner's is accepted. | `r03_*` |
| R04 | Start with no agents returns `start_not_eligible` and leaves setup; a connected viewer makes it eligible and a disconnected one doesn't; a second Start returns `run_already_started`. | `r04_*` |
| R05 | Spawning after Start returns `run_already_started`. | `r05_*` |
| R06 | A disconnected session that has not expired keeps its run. An expired session's agent is removed, freeing its cell, and a step waiting for it executes. | `r06_*`, `r07_joining_before_release_keeps_the_run` |
| R07 | A run is released after its sessions expire, including while waiting for actions; a join during the release timeout keeps it; a connected session keeps it. | `r07_*` |
| R08, R09 | No step executes until every agent has an accepted action; then each session receives only its own agents' observations, a session without agents receives none, and collection targets the next state. | `r08_r09_*` |
| R10 | Viewers receive the current view and one after each change. Covered by the SPEC-002 server tests `r12_*`. Holding only the newest view comes from the channel type (a Tokio `watch` channel) and is not tested directly. | server `r12_*` in `client_protocol.rs` |
| R11 | With a viewer and a long interval, a ready step waits, then executes once the viewer disconnects; with a short interval it executes after the interval; without viewers it is not held. | `r11_*` |

## Open questions

- Timeout values are provisional until tested with real clients.
