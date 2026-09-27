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
- session expiry, including removing an expired session's agents, and run release

It is extended feature by feature. Messages, framing, the handshake, request IDs, and error codes belong to the [client protocol](../../../contracts/client-protocol.md) (SPEC-002).

The SADD and CDD describe further behavior that is not yet covered here and is not implemented:

- session recovery
- closing a run in setup when its creator expires
- viewers, pacing, deadlines, and fallback control

## Terminology and preconditions

- **Session:** created by `create_run` or `join_run`. A session is *connected* while the connection that created it is open.
- **Session expiry timeout** and **run release timeout:** server settings. They default to 2 minutes and 5 minutes, and the executable sets them with `--session-expiry-secs` and `--run-release-secs`.

## Requirements

### Catalog and identifiers

- **SPEC-003-R01:** The catalog has one entry, `empty-grid-10x10`: an empty, bounded grid 10 cells wide and 10 cells high. Each run's simulation seed comes from OS randomness and is written to the server log.
- **SPEC-003-R02:** Run IDs and session IDs are assigned by the server and are unique. Clients treat them as opaque strings. Each successful `join_run` creates a new session.

### Authority and ownership

- **SPEC-003-R03:** The session created by `create_run` holds creator authority, which gives it sole permission to Start. Start from any other session returns `not_creator`. The session that spawns an agent owns it. Only the owner can submit actions for an agent; other sessions' entries for it are rejected with `agent_not_owned`, and the owner can still submit.

### Start

- **SPEC-003-R04:** Start from the creator is rejected with `start_not_eligible` when the run has no agents and no connected viewers. The run stays in setup. Viewers do not exist yet, so in practice the run needs an agent. Once the run has started, a further Start returns `run_already_started`.
- **SPEC-003-R05:** *(Interim.)* Spawning after Start returns `run_already_started`, following [SPEC-001-R06](../../simulation/specs/lifecycle-and-movement.md#spawning). Post-Start admission will replace this rule.

### Steps and observations

- **SPEC-003-R08:** *(Interim, until pacing.)* A started run executes a step as soon as every agent in it has an accepted action for the current state. Rejected entries do not count. The step is triggered by the submission that supplies the last missing action, or by the removal of the last agents without one (R06). There is no pacing gate or deadline yet, so the run advances as fast as its agents submit, which is the SADD's unlimited pacing for a run without viewers.
- **SPEC-003-R09:** Observations are routed by ownership. After Start and after each step, each connected session that owns agents receives one `observations` push with exactly its own agents. Sessions without agents, and sessions without a connection, receive nothing; a disconnected session's observations are not kept.

### Lifetimes

- **SPEC-003-R06:** When a session's connection closes, the session stays in its run without a connection. It expires once the session expiry timeout has passed, and is then removed from the run. Its agents are removed from the world immediately ([SPEC-001-R18](../../simulation/specs/lifecycle-and-movement.md#removal)), in any phase, and a step that was waiting only for them then executes (R08). *(Interim.)* A creator expiring during setup does not close the run.
- **SPEC-003-R07:** A run is released once it has had no sessions for the run release timeout. The timer starts when the run's last session expires, and a `join_run` before it fires cancels it. After release, `join_run` for that run returns `unknown_run`. A run with a connected session, or with a disconnected session that has not yet expired, is never released. Agents and pending actions do not keep a run: a run waiting for actions is released like any other.

## Interfaces and data

The library's `agora_server::serve` accepts connections on a `TcpListener` with a `ServerConfig` that holds the two timeouts. The `agora-server` executable wraps it and takes `--listen` (default `127.0.0.1:7878`) and the two timeout flags.

## Acceptance criteria and verification

Tests are in `rust/agora-server/tests/sessions_and_runs.rs` and start a server on an unused local port. Each test name starts with the requirement ID it checks. Timer tests use timeouts of 20 ms and wait 400 ms.

| Requirement | Check and expected outcome | Test |
| --- | --- | --- |
| R01 | Corner cells of the bundled grid spawn; cells at x = 10 or y = 10 are out of bounds. | `r01_*` |
| R02 | Run IDs and session IDs from several creates and joins are all distinct. | `r02_*` |
| R03 | A joining session's Start returns `not_creator`; the creator's succeeds. A non-owner's entry returns `agent_not_owned`; the owner's is accepted. | `r03_*` |
| R04 | Start with no agents returns `start_not_eligible` and leaves setup; a second Start returns `run_already_started`. | `r04_*` |
| R05 | Spawning after Start returns `run_already_started`. | `r05_*` |
| R06 | A disconnected session that has not expired keeps its run. An expired session's agent is removed, freeing its cell, and a step waiting for it executes. | `r06_*`, `r07_joining_before_release_keeps_the_run` |
| R07 | A run is released after its sessions expire, including while waiting for actions; a join during the release timeout keeps it; a connected session keeps it. | `r07_*` |
| R08, R09 | No step executes until every agent has an accepted action; then each session receives only its own agents' observations, a session without agents receives none, and collection targets the next state. | `r08_r09_*` |

## Open questions

- Timeout values are provisional until tested with real clients.
