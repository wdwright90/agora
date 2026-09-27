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

The contract grows feature by feature. Protocol version 0.1.0 currently covers:

- message framing
- the version handshake
- request correlation
- errors
- run setup: create, join, spawn, and Start

Step submission, observations, viewer snapshots, pacing, session recovery, and credentials are not yet defined. They are added as the features that use them are built.

## Terminology and preconditions

- **Connection:** one WebSocket connection. **Session:** the server's logical client session. A connection carries at most one session.
- **Request:** a client message that receives exactly one final response.
- **Response:** the server's success message for a request, or an `error` message.
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

  The current version is **0.1.0**. Protocol versions are independent of capability versions.
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
  | `start {request_id}` | `started {request_id}` | Confirms that the run has started. Initial observations are delivered separately (defined in a later version). |

  - `placement` is either `{"kind": "random"}`, meaning uniformly among unoccupied cells, or `{"kind": "cell", "x", "y"}`. On the wire, `(0, 0)` is the grid's south-west corner, `x` grows east, and `y` grows north.
  - A `create_run` or `join_run` on a connection that already has a session returns `session_already_established`.
  - A `spawn` or `start` before a session exists returns `no_session`.

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
| R09 | Known codes map to their wire names, and unknown codes are preserved | `r09_*` |
| R01 (server) | Unknown types report their type; invalid JSON, non-objects, missing fields, and binary frames are malformed, echoing any readable request ID | server `r01_*` |
| R02 (server) | A request with an extra field succeeds | server `r02_*` |
| R04, R06 (server) | `hello` gets `welcome`; an unsupported version is rejected and the connection closed; requests before `hello` and a second `hello` are rejected | server `r04_*` |
| R07 (server) | Retries replay results, including errors, without re-executing; conflicts, skipped IDs, and IDs older than the retained results are rejected; sessions number requests independently | server `r07_*` |
| R08 (server) | Create, join, spawn, and Start succeed with the documented fields; a second session and requests without a session are rejected | server `r08_*` |
| R09 (server) | Catalog, run, and spawn failures return their codes and details | server `r09_*` |

## Open questions

- **Retrying `create_run` or `join_run`:** if the response is lost, there is no session to retry against, so a retry creates another run or session. Deferred with session recovery and credentials.
- Session recovery messages and credentials; step, observation, viewer, and pacing messages. These arrive with later features.
- The concrete post-1.0.0 compatibility rule and how the server advertises multiple supported versions.
