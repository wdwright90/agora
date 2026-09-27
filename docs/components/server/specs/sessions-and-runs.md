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
- session expiry and run release

It is extended feature by feature. Messages, framing, the handshake, request IDs, and error codes belong to the [client protocol](../../../contracts/client-protocol.md) (SPEC-002).

The SADD and CDD describe further behavior that is not yet covered here and is not implemented:

- session recovery
- cleaning up an expired session's agents
- closing a run in setup when its creator expires
- viewers, pacing, deadlines, and observation routing

## Terminology and preconditions

- **Session:** created by `create_run` or `join_run`. A session is *connected* while the connection that created it is open.
- **Session expiry timeout** and **run release timeout:** server settings. They default to 2 minutes and 5 minutes, and the executable sets them with `--session-expiry-secs` and `--run-release-secs`.

## Requirements

### Catalog and identifiers

- **SPEC-003-R01:** The catalog has one entry, `empty-grid-10x10`: an empty, bounded grid 10 cells wide and 10 cells high. Each run's simulation seed comes from OS randomness and is written to the server log.
- **SPEC-003-R02:** Run IDs and session IDs are assigned by the server and are unique. Clients treat them as opaque strings. Each successful `join_run` creates a new session.

### Authority and ownership

- **SPEC-003-R03:** The session created by `create_run` holds creator authority, which gives it sole permission to Start. Start from any other session returns `not_creator`. The session that spawns an agent owns it.

### Start

- **SPEC-003-R04:** Start from the creator is rejected with `start_not_eligible` when the run has no agents and no connected viewers. The run stays in setup. Viewers do not exist yet, so in practice the run needs an agent. Once the run has started, a further Start returns `run_already_started`.
- **SPEC-003-R05:** *(Interim.)* Spawning after Start returns `run_already_started`, following [SPEC-001-R06](../../simulation/specs/lifecycle-and-movement.md#spawning). Post-Start admission will replace this rule.

### Lifetimes

- **SPEC-003-R06:** When a session's connection closes, the session stays in its run without a connection. It expires once the session expiry timeout has passed, and is then removed from the run. *(Interim.)* An expired session's agents stay in the world, and a creator expiring during setup does not close the run.
- **SPEC-003-R07:** A run is released once it has had no sessions for the run release timeout. The timer starts when the run's last session expires, and a `join_run` before it fires cancels it. After release, `join_run` for that run returns `unknown_run`. A run with a connected session, or with a disconnected session that has not yet expired, is never released.

## Interfaces and data

The library's `agora_server::serve` accepts connections on a `TcpListener` with a `ServerConfig` that holds the two timeouts. The `agora-server` executable wraps it and takes `--listen` (default `127.0.0.1:7878`) and the two timeout flags.

## Acceptance criteria and verification

Tests are in `rust/agora-server/tests/sessions_and_runs.rs` and start a server on an unused local port. Each test name starts with the requirement ID it checks. Timer tests use timeouts of 20 ms and wait 400 ms.

| Requirement | Check and expected outcome | Test |
| --- | --- | --- |
| R01 | Corner cells of the bundled grid spawn; cells at x = 10 or y = 10 are out of bounds. | `r01_*` |
| R02 | Run IDs and session IDs from several creates and joins are all distinct. | `r02_*` |
| R03 | A joining session's Start returns `not_creator`; the creator's succeeds. | `r03_*` |
| R04 | Start with no agents returns `start_not_eligible` and leaves setup; a second Start returns `run_already_started`. | `r04_*` |
| R05 | Spawning after Start returns `run_already_started`. | `r05_*` |
| R06 | A disconnected session that has not expired keeps its run. | `r06_*` |
| R07 | A run is released after its sessions expire; a join during the release timeout keeps it; a connected session keeps it. | `r07_*` |

Agent ownership (R03) is recorded but not yet observable by clients. It is verified when observation routing uses it.

## Open questions

- Timeout values are provisional until tested with real clients.
