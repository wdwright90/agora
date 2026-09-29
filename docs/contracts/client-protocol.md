---
id: SPEC-002
status: draft
owner: maintainer
parents: [SADD-001]
packages: [agora-protocol, agora-server, agora-client, agora-viewer]
---

# Client protocol

## Purpose and scope

This contract is the canonical definition of the messages exchanged between the Agora server and its clients: the Rust client, the viewer, and later the Python client. It implements the SADD's [communication](../architecture/sadd.md#simulation-step-coordination), [error-response](../architecture/sadd.md#error-responses), and [run lifecycle](../architecture/sadd.md#run-lifecycle) rules.

The contract grows feature by feature. Protocol version 0.5.0 currently covers:

- message framing
- the version handshake
- request correlation
- errors
- run setup: create, join, spawn, and Start
- action submission and agent observations
- watching a run's view
- pacing controls
- leaving and closing runs

Session recovery and credentials are not yet defined. They are added as the features that use them are built. Version 0.2.0 added submission and observations, 0.3.0 added watching, 0.4.0 added pacing, and 0.5.0 added leaving and closing runs.

## Terminology and preconditions

- **Connection:** one WebSocket connection. **Session:** the server's logical client session. A connection carries at most one session at a time; after a session ends, the connection may establish another.
- **Request:** a client message that receives exactly one final response.
- **Response:** the server's success message for a request, or an `error` message.
- **Push:** a server message that is not a response. It has no `request_id`.
- "Client" and "server" name the sender of a message. `agora-protocol` types are `ClientMessage` and `ServerMessage`.

## Requirements

### Framing

- **SPEC-002-R01:** Messages are WebSocket text frames. Each frame carries exactly one JSON object. The object's string field `type` selects the message, and message types, field names, and enumerated values use snake_case. A client message with an unknown `type` is rejected with `unknown_message_type`. A frame that is not a JSON object, or that has missing or invalid fields, is rejected with `malformed_message`.
- **SPEC-002-R02:** Receivers ignore object fields they don't recognize. This lets later minor versions add optional fields.
- **SPEC-002-R03:** Identifiers and counters have these forms:

  | Field | JSON form | Allowed values |
  | --- | --- | --- |
  | `request_id`, `agent_id` | integer | 1 to 2⁵³ − 1 |
  | `state_id` | integer | 0 to 2⁵³ − 1 |
  | `run_id`, `session_id`, `catalog_entry` | string | non-empty; opaque to clients, compared exactly |
  | cell coordinates `x`, `y` | integer | 0 to 2³² − 1 |

  The upper bound of 2⁵³ − 1 keeps integers exact in every JSON implementation. Values outside the allowed ranges are malformed.

### Versioning and handshake

- **SPEC-002-R04:** The first client message on a connection is `hello`. The server replies in one of two ways:
  - with `welcome`, giving its protocol version;
  - with an `error` whose code is `unsupported_protocol_version` and whose `request_id` is null, after which it closes the connection.

  Any other message before a successful handshake is rejected with `handshake_required`. A second `hello` on the same connection is rejected with `malformed_message`.
- **SPEC-002-R05:** Protocol versions are semantic versions written as `"MAJOR.MINOR.PATCH"`. Each component is a decimal integer with no sign and no leading zeros, and there are no pre-release or build suffixes. Versions follow semver:
  - Incompatible changes increase the major version. While the major version is 0, they increase the minor version.
  - Backward-compatible additions increase the minor version.
  - Clarifications and fixes that don't change messages increase the patch version.

  The current version is **0.5.0**. Protocol versions are independent of capability versions.
- **SPEC-002-R06:** *(MVP.)* The server accepts only a client version exactly equal to its own. Semver-based backward compatibility is intended from 1.0.0 onward: the same major version, with the server's minor version at least the client's. Defining that rule will require a change to this spec.

### Requests and sessions

- **SPEC-002-R07:** Every client message except `hello` is a request carrying a `request_id`, and its final response echoes that ID. Request IDs are increasing numbers within a session, following the SADD's [request-correlation rules](../architecture/sadd.md#simulation-step-coordination):
  - A retry with the same ID and contents receives the original result.
  - Reusing an ID with different contents returns `request_id_conflict`.
  - An ID at or below the highest admitted number with no retained record returns `stale_request_id`.
  - A retry after its stored result expires returns `result_expired`.

  The `create_run` or `join_run` request that establishes a session supplies that session's first request number.

  *(MVP.)* The server keeps the results of each session's 5 most recent admitted requests, including error results. When an ID is at or below the highest admitted number and has no retained result:
  - if it is older than every retained result, the server returns `result_expired`, whether the number was used or skipped;
  - otherwise the number was skipped, and the server returns `stale_request_id`.

  Requests rejected by these ID checks are not admitted or recorded. The SADD's time-based retention window replaces this rule once session recovery is built.
- **SPEC-002-R08:** Setup messages:

  | Request | Success response | Notes |
  | --- | --- | --- |
  | `create_run {request_id, catalog_entry}` | `run_created {request_id, run_id, session_id, catalog_entry, state_id, phase}` | Creates a run in `setup` at state 0 and establishes this connection's session with creator authority. |
  | `join_run {request_id, run_id}` | `run_joined {request_id, run_id, session_id, catalog_entry, state_id, phase}` | Establishes a new non-creator session in an existing run. `phase` is `setup` or `started`. |
  | `spawn {request_id, placement}` | `spawned {request_id, agent_id}` | Sent once the agent has been placed. The session owns the agent. Coordinates are not returned. |
  | `start {request_id}` | `started {request_id}` | Confirms that the run has started. Initial observations for state 0 follow as a push ([R11](#steps-and-observations)). |

  - `placement` is either `{"kind": "random"}`, meaning uniformly among unoccupied cells, or `{"kind": "cell", "x", "y"}`. On the wire, `(0, 0)` is the grid's south-west corner, `x` grows east, and `y` grows north.
  - A `create_run` or `join_run` on a connection that already has a session returns `session_already_established`.
  - A run-scoped request before a session exists, or after it has ended, returns `no_session`.

### Steps and observations

- **SPEC-002-R10:** `submit {request_id, state_id, actions}` submits actions for agents the session owns. `state_id` is the state every action targets, and `actions` is a list of `{agent_id, action}` entries. The only action is `{"kind": "move", "direction", "distance"}`, where `direction` is `north`, `east`, `south`, or `west`, and `distance` is an integer from 0 to 2³² − 1. Distance 0 stays in place.
  - The whole request fails with `run_not_started` before Start, and with `wrong_state` when `state_id` is not the run's current state.
  - Otherwise the response is `submitted {request_id, state_id, results}`, with one result per entry in the order submitted. A result is `{agent_id}` when the action was accepted, or `{agent_id, error: {code, message, details?}}` when it was rejected. Entry errors have the same fields as error responses, without `request_id`.
  - Entries are checked in order. An entry is rejected with `agent_not_owned` when the session does not own the agent, `already_submitted` when the agent already has an accepted action for this state, and `agent_limit.distance_budget_exceeded` when the distance is over the movement budget of 1. A rejected entry leaves the agent free to submit again. An accepted action is fixed; a later entry for the same agent, in the same request or another, is rejected.
- **SPEC-002-R11:** `observations {state_id, observations}` is a push. It carries one `{agent_id, observation}` entry for each agent the session owns, in ascending agent ID order, all labeled with `state_id`. In this version `observation` is an empty object.
  - The server pushes observations for state 0 after Start, and for state N + 1 after each step, to every connected session that owns agents.
  - A push caused by a request is sent after that request's response. `started` precedes the state-0 observations, and the `submitted` that completes a step precedes that step's observations.
  - Clients use the observations' `state_id` as the next submission's target.

### Viewing

- **SPEC-002-R12:** `watch {request_id}` makes the session a viewer until its connection closes. There is no request to stop watching. The response is `watching {request_id, view, pacing}`, where `view` and `pacing` ([R13](#pacing)) are the run's current state. Watching again returns the current state and changes nothing else. Afterwards the server pushes `view_update {view}` when the view changes, after the response to any request that caused the change. A viewer may skip intermediate views, so each view is complete.

  A view is `{state_id, phase, width, height, agents}`:
  - `phase` is `setup` or `started`, and `width` and `height` are the grid size in cells.
  - `agents` lists every agent in the run as `{agent_id, x, y}`, in ascending ID order, using the coordinates in R08.

  Views are separate from agent observations and are not limited to the session's own agents.

### Pacing

- **SPEC-002-R13:** Viewers control how fast a run steps. The server's rules are in [SPEC-003](../components/server/specs/sessions-and-runs.md#pacing).
  - A pacing mode is `{"kind": "paused"}`, `{"kind": "unlimited"}`, or `{"kind": "interval", "ms"}`, where `ms` is an integer from 1 to 3,600,000 (one hour). Other values are malformed.
  - A pacing state is `{mode, you_control}`, where `you_control` says whether the receiving session holds pacing control. It appears in `watching`, and the server pushes `pacing_update {pacing}` to every viewer when the mode or the controller changes, after the response to any request that caused the change.

  | Request | Success response | Errors |
  | --- | --- | --- |
  | `claim_pacing {request_id}` | `pacing_claimed {request_id}` | `not_viewing`, `pacing_control_held` |
  | `set_pacing {request_id, mode}` | `pacing_set {request_id}` | `not_pacing_controller` |
  | `step_once {request_id}` | `step_granted {request_id}` | `not_pacing_controller`, `run_not_paused` |

  `step_granted` confirms that one step may start; it does not wait for the step. The step's result arrives as a `view_update` once every agent is ready.

### Leaving and closing

- **SPEC-002-R14:** `leave_run {request_id}` ends the session; the response is `left {request_id}`. `close_run {request_id}` closes the run for every session; the response is `closed {request_id}`, and only the creator may send it (`not_creator` otherwise). After either response the connection has no session and may send `create_run` or `join_run` again, and a new session numbers its requests independently. Neither request can be retried once answered, because the session has ended; a retry returns `no_session`. Pushes the run queued for the session before it ended are sent before `left` or `closed`, and nothing from that run follows.
- **SPEC-002-R15:** `run_closed {reason}` is a push telling a session that its run was closed and the session has ended; the connection may then create or join again. `reason` is `closed_by_creator`, `creator_expired`, or `creator_left`. Receivers treat an unknown reason as a generic closure.

### Errors

- **SPEC-002-R09:** Every failure is reported as `error {request_id, code, message, details?}`:
  - `request_id` echoes the failing request's ID, or is null when no valid request ID could be read.
  - `code` is a stable snake_case string.
  - `message` is for people, and clients must not parse it.
  - `details` is an optional object whose fields depend on the code.

  Clients must handle codes they don't recognize as generic failures. Current codes:

  | Code | Meaning | `details` |
  | --- | --- | --- |
  | `malformed_message` | Invalid JSON, not an object, or missing or invalid fields | — |
  | `unknown_message_type` | `type` names no known message | `type` |
  | `handshake_required` | Request before a successful `hello` | — |
  | `unsupported_protocol_version` | Version not accepted | `requested` (string), `supported` (array of strings) |
  | `session_already_established` | `create_run`/`join_run` when the connection already has a session | — |
  | `no_session` | Run-scoped request before a session exists | — |
  | `stale_request_id` | ID at or below the highest admitted, with no record | `highest_admitted` |
  | `request_id_conflict` | ID reused with different contents | — |
  | `result_expired` | Retry after the stored result expired | — |
  | `unknown_catalog_entry` | No such catalog entry | `catalog_entry` |
  | `unknown_run` | No such run | `run_id` |
  | `not_creator` | Operation requires creator authority | — |
  | `run_already_started` | Operation only available during setup | — |
  | `start_not_eligible` | Start with no agents and no connected viewers | — |
  | `cell_out_of_bounds` | Spawn cell outside the grid | `x`, `y` |
  | `cell_occupied` | Spawn cell occupied | `x`, `y` |
  | `no_free_cell` | Random placement found no unoccupied cell | — |
  | `run_not_started` | Operation only available after Start | — |
  | `wrong_state` | Submission for a state other than the current one | `expected`, `supplied` |
  | `agent_not_owned` | *(Entry.)* The session does not own the agent | — |
  | `already_submitted` | *(Entry.)* The agent already has an accepted action for this state | — |
  | `agent_limit.distance_budget_exceeded` | *(Entry.)* Move distance over the movement budget | `distance`, `budget` |
  | `not_viewing` | Operation requires the session to be a viewer | — |
  | `pacing_control_held` | Another viewer holds pacing control | — |
  | `not_pacing_controller` | Operation requires pacing control | — |
  | `run_not_paused` | `step_once` while the run is not paused | — |

  Codes marked *(Entry)* appear in `submitted` results rather than in `error` messages. Codes for actions that exceed an agent's limits start with `agent_limit.`.

## Interfaces and data

`agora-protocol` implements this contract as serde types (`ClientMessage`, `ServerMessage`, `ErrorResponse`, `ErrorCode`, and the identifier newtypes). Other languages implement the same JSON forms and verify them against the shared fixtures.

Fixtures live in [`fixtures/client-protocol/`](fixtures/client-protocol/):

- `valid/client/`, `valid/server/`: every file must parse. Re-serializing it must produce the same JSON value.
- `invalid/client/`, `invalid/server/`: every file must be rejected.

Each message type has at least one valid fixture. Add fixtures with every message change.

## Acceptance criteria and verification

Rust tests are named by requirement ID. Message types are tested in `rust/agora-protocol/tests/client_protocol.rs`. Server behavior is tested in `rust/agora-server/tests/client_protocol.rs`, which drives a running server over WebSocket; the table marks those tests "server".

| Requirement | Check and expected outcome | Test / fixture |
| --- | --- | --- |
| R01, R08 | Valid fixtures round-trip, and every message type is covered | `r01_r08_*`, `valid/` |
| R02 | Unknown fields are ignored | `r02_*` |
| R03 | Out-of-range, zero, string-typed, negative, and empty values are rejected | `r03_*`, `invalid/` |
| R05 | Version strings parse strictly | `r05_*`, `invalid/client/version_*` |
| R06 | Only an exact version match is compatible | `r06_*` |
| R07 | Only `hello` lacks a request ID | `r07_*` |
| R09 | Known codes map to their wire names, including the `agent_limit.` prefix, and unknown codes are preserved | `r09_*` |
| R10 | An accepted entry has no `error` field | `r10_*` |
| R12 | View fixtures round-trip; negative coordinates and a missing phase are rejected | `valid/`, `invalid/server/view_*` |
| R14, R15 | Leave and close fixtures round-trip; `run_closed` without a reason is rejected; an unknown reason is read as a generic closure | `valid/`, `invalid/server/run_closed_missing_reason.json`, `r15_*` |
| R13 | Pacing fixtures round-trip; intervals of 0 and over an hour, unknown modes, and `watching` without pacing are rejected | `valid/`, `invalid/client/set_pacing_*`, `invalid/server/watching_missing_pacing.json` |
| R01 (server) | Unknown types report their type; invalid JSON, non-objects, missing fields, and binary frames are malformed, echoing any readable request ID | server `r01_*` |
| R02 (server) | A request with an extra field succeeds | server `r02_*` |
| R04, R06 (server) | `hello` gets `welcome`; an unsupported version is rejected and the connection closed; requests before `hello` and a second `hello` are rejected | server `r04_*` |
| R07 (server) | Retries replay results, including errors, without re-executing; conflicts, skipped IDs, and IDs older than the retained results are rejected; sessions number requests independently | server `r07_*` |
| R08 (server) | Create, join, spawn, and Start succeed with the documented fields; a second session and requests without a session are rejected | server `r08_*` |
| R09 (server) | Catalog, run, and spawn failures return their codes and details | server `r09_*` |
| R10 (server) | Entry results are in order with their codes and details; submitting before Start or for another state is rejected, and a rejected submission leaves the slot open | server `r10_*` |
| R11 (server) | `started` precedes the state-0 observations; the step-completing `submitted` precedes the state-1 observations | server `r11_*` |
| R12 (server) | `watching` carries the current view; a viewer receives a view after a spawn, Start, and a step, with the moved position | server `r12_*` |
| R13 (server) | `watching` carries the default pacing state; each pacing request gets its response and the changes are pushed; each error code is returned | server `r13_*` |
| R14 (server) | `left` ends the session, later requests and a retried leave return `no_session`, and the connection can join again; a non-creator's close returns `not_creator`, and the creator's returns `closed` and ends its session | server `r14_*` |
| R15 (server) | Other sessions receive `run_closed` with the reason, lose their session, and can create a new run | server `r15_*` |

## Open questions

- **Retrying `create_run` or `join_run`:** if the response is lost, there is no session to retry against, so a retry creates another run or session. Deferred with session recovery and credentials.
- Session recovery messages and credentials. These arrive with a later feature.
- Large environments may need partial or incremental views instead of complete ones (see the SADD's large-environment viewing scenario).
- Pushes have no delivery guarantee across a lost connection. Session recovery will define how a client catches up.
- **Action kinds:** each `kind` defines its own parameters, placed beside `kind`. Names that every kind may share later, such as `kind` and a capability `version`, are reserved and not used as parameters. An unknown `kind` currently makes the request malformed; once clients declare capabilities, a well-formed action the agent does not support should probably be rejected per entry instead. Parameter values may also depend on the environment, such as a hex grid's six directions. The fixed `direction` values stand in until capability definitions supply them per run (see the [simulation CDD](../components/simulation/cdd.md#open-questions)).
- The concrete post-1.0.0 compatibility rule and how the server advertises multiple supported versions.
