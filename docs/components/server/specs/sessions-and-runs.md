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
- viewers, view delivery, and pacing
- session expiry, including removing an expired session's agents, and run release

It is extended feature by feature. Messages, framing, the handshake, request IDs, and error codes belong to the [client protocol](../../../contracts/client-protocol.md) (SPEC-002).

The SADD and CDD describe further behavior that is not yet covered here and is not implemented:

- session recovery
- closing a run in setup when its creator expires
- deadlines and fallback control

## Terminology and preconditions

- **Session:** created by `create_run` or `join_run`. A session is *connected* while the connection that created it is open.
- **Session expiry timeout** and **run release timeout:** server settings. They default to 2 minutes and 5 minutes, and the executable sets them with `--session-expiry-secs` and `--run-release-secs`.
- **Viewer:** a connected session that has sent `watch`. It stays a viewer until its connection closes.
- **Default interval:** a server setting, the pacing interval a run takes when its first viewer arrives. It defaults to 500 ms, which is the SADD's provisional live-viewing default, and the executable sets it with `--step-interval-ms`, from 1 ms to one hour.
- **Pacing controller:** the viewer that holds pacing control, if any.

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

- **SPEC-003-R08:** A started run executes a step once every agent in it has an accepted action for the current state, subject to the pacing gate (R11). Rejected entries do not count. The step is triggered by the submission that supplies the last missing action, or by the removal of the last agents without one (R06). There is no deadline yet.
- **SPEC-003-R09:** Observations are routed by ownership. After Start and after each step, each connected session that owns agents receives one `observations` push with exactly its own agents. Sessions without agents, and sessions without a connection, receive nothing; a disconnected session's observations are not kept.

### Viewers

- **SPEC-003-R10:** Any session can become a viewer with `watch`, including one that owns agents. The response carries the current view. Afterwards the viewer receives a `view_update` after every change to the view: a spawn, Start, a step, and the removal of an expired session's agents. Each viewer holds only the newest view it has not yet been sent, so a slow viewer skips intermediate views rather than queueing them or delaying the run.

### Pacing

- **SPEC-003-R11:** A run has a pacing mode, which gates when a ready step starts:
  - *unlimited:* at once;
  - *interval N:* no sooner than N ms after the previous step started; a step ready earlier waits, then executes without further input;
  - *paused:* not at all, unless the controller has granted a single step (R14).

  Pacing gates only step execution; requests are still processed.
- **SPEC-003-R12:** A run without viewers is unlimited. When a run gains its first viewer, its mode becomes the default interval and it has no controller. When its last viewer disconnects, it becomes unlimited again and has no controller, so a waiting step executes at once. The mode is kept across Start.
- **SPEC-003-R13:** Only a viewer can claim pacing control, and only while no other viewer holds it; creator authority gives no priority. Claiming control already held by the same session succeeds. When the controller disconnects and viewers remain, control passes to the remaining viewer whose connection the server accepted first, and the mode is unchanged. There is no recovery yet, so a reconnecting client is a new viewer and does not regain control.
- **SPEC-003-R14:** The controller can set any mode at any time, including during setup. A new interval is measured from the start of the previous step, so a held step executes at once if that much time has already passed. While paused, `step_once` grants one step: the next step that becomes ready executes, and the run stays paused. Grants do not accumulate, and changing the mode cancels an unused grant.

### Lifetimes

- **SPEC-003-R06:** When a session's connection closes, the session stays in its run without a connection. It expires once the session expiry timeout has passed, and is then removed from the run. Its agents are removed from the world immediately ([SPEC-001-R18](../../simulation/specs/lifecycle-and-movement.md#removal)), in any phase, and a step that was waiting only for them then executes (R08). *(Interim.)* A creator expiring during setup does not close the run.
- **SPEC-003-R07:** A run is released once it has had no sessions for the run release timeout. The timer starts when the run's last session expires, and a `join_run` before it fires cancels it. After release, `join_run` for that run returns `unknown_run`. A run with a connected session, or with a disconnected session that has not yet expired, is never released. Agents and pending actions do not keep a run: a run waiting for actions is released like any other.

## Interfaces and data

The library's `agora_server::serve` accepts connections on a `TcpListener` with a `ServerConfig` that holds the two timeouts and the default interval. The `agora-server` executable wraps it and takes `--listen` (default `127.0.0.1:7878`), the two timeout flags, and `--step-interval-ms`.

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
| R11, R12 | With a viewer and a long interval, a ready step waits, then executes once the last viewer disconnects; with a short interval it executes after the interval; without viewers it is not held; a new first viewer resets the mode to the default interval with no controller. | `r11_*`, `r12_*` |
| R13 | Control passes to the remaining viewer with the oldest connection, not the one that watched first, and the mode is kept; its control then rejects other claims. Claim errors are covered by SPEC-002 server `r13_*`. | `r13_*` |
| R14 | Paused during setup, the run does not step after Start until a step is granted, stays paused after it, and resumes when set unlimited; two grants allow one step; shortening a held interval releases the step. | `r14_*` |

## Open questions

- Timeout values are provisional until tested with real clients.
