---
id: CDD-003
status: draft
owner: maintainer
parent: SADD-001
packages: [agora-client]
---

# Rust client — Component Design Description

## Responsibility and boundaries

The Rust client connects Rust programs to an Agora server: agent clients that submit actions and receive observations, and viewers that watch a run and control its pacing. It implements the [client protocol](../../contracts/client-protocol.md) (SPEC-002) and follows the SADD's [communication rules](../../architecture/sadd.md#simulation-step-coordination), including that client libraries generate request IDs for application code.

It adds no simulation behavior and does not depend on `agora-sim`. Model loading and inference belong to the programs that use it. The Python client is a separate project implementing the same contract.

## Package mapping

The `agora-client` package (`rust/agora-client`) implements this component as a library and the `agora-demo` executable. It depends on `agora-protocol` and uses tokio and `tokio-tungstenite`, like the server. The planned `agora-viewer` package will depend on it, as the [package mapping](../../architecture/sadd.md#rust-package-mapping) describes.

The demo sits behind the `demo` feature, which is on by default. It enables the demo's own dependencies (`clap`, `rand`, and `tracing-subscriber`); library users such as the viewer can turn it off to skip them. The maintainer chose the feature over a separate package on 2026-09-27.

## Internal structure

- **`Client`:** a connection that has completed the `hello` handshake but has no session. Creating or joining a run consumes it and returns a `Session` with its `Observations`, so a connection carries at most one session, as the protocol requires. A program that takes part in several runs opens a connection for each.
- **`Session`:** a cheap, cloneable handle for requests (`spawn`, `start`, `submit`, `watch`, `claim_pacing`, `set_pacing`, and `step_once`) and for the latest-value handles. Clones share one session, so several tasks can make requests at once: for example, a viewer's UI task sending pacing changes while another task renders, or one task per agent submitting its action.
- **`Observations`:** the session's observation stream. It is returned once, alongside the session, so exactly one task owns it and handles steps, while other tasks keep using clones of the session.
- **Request numbering:** a session numbers its requests from 1, the request that established it. Allocating a number and queueing the request happen under one lock, so requests reach the server in number order even when several tasks send at once.
- **Connection task:** one background task owns the socket. It writes queued requests, matches each response to its waiting request by request ID, and routes pushed messages.

## Interfaces and dependencies

Pushed messages reach the application in two forms:

- **Observations** arrive through a stream (`Observations::next`), one batch per state, in the order the server sent them. Agent code must see every state, so none are dropped.
- **Views and pacing state** are held in latest-value handles (`Session::view` and `Session::pacing`, Tokio `watch` receivers). Only the newest value matters for display, so a reader that falls behind sees the current state and skips stale ones, matching the server's newest-only delivery. A Bevy viewer can read them once per frame.

Callbacks were rejected because handlers would run inside the library's task, which fits poorly with async code and with Bevy. A single stream for all pushes was rejected because views would queue up behind a slow reader.

Failures are `ClientError` values: a transport failure, a server rejection carrying the server's `ErrorResponse`, a closed connection, a run closed by the server (with its reason), an unreadable message, or a response that does not fit the request.

`Session::leave` and `Session::close` end the session; `Session::closed_reason` reports why the server closed the run, if it did. The server allows a connection to hold several sessions in turn, but this library keeps one session per connection: ending a session closes its connection, and the next episode connects again. Handing a connection back for reuse is awkward because sessions are cloned across tasks, and reconnecting locally takes milliseconds.

## Data flow and lifecycle

- **Connect:** open the WebSocket, send `hello`, and require `welcome`.
- **Establish:** start the connection task, send `create_run` or `join_run` as request 1, and return the session, with the server's session information, and its observation stream.
- **Request:** queue the request with the next number and a reply slot, then wait. The connection task writes it and, when the response arrives, completes the reply slot. A server `error` becomes `ClientError::Rejected`.
- **Push:** `observations` go to the stream. `view_update` and `pacing_update` replace the latest values. A `watching` response fills both handles before the caller sees it.
- **Close:** when the session leaves or closes its run, or the server sends `run_closed`, the connection task closes the socket. When the socket closes for any reason, waiting and later requests fail, with `ClientError::RunClosed` if the server closed the run and `ClientError::Closed` otherwise, and the observation stream ends after any batches already received. Dropping every clone of the session also closes the connection.

## The demo client

`agora-demo` moves every agent it owns one random cardinal step each state. `create` creates a run on the bundled catalog entry, prints the run ID on standard output, watches the run, and starts it once the view shows enough agents. `join` joins a run by ID. Both stop after a set number of steps or when interrupted. Watching makes the creating demo a viewer, so its run is paced at the server's default interval unless `--unlimited` claims pacing control and removes it.

## Design constraints and rationale

- **Async on tokio.** The viewer will run the library on a background runtime, and agent clients usually wait on the network. The maintainer chose async on 2026-09-27.
- **The observation stream is separate from the session.** Requests need shared access from several tasks, but reading a stream needs one owner. Keeping both in one value meant a shared session could not read its observations; the first draft had this flaw, found in review.
- **No retries.** Without session recovery, a lost connection ends the session, so the library does not retry requests. It still sends every request with a fresh number, as the protocol requires.

## Detailed specifications

- [SPEC-004 — Rust client library and demo](specs/client-library.md) (draft).
- [SPEC-002 — Client protocol](../../contracts/client-protocol.md) (draft): the wire contract the client implements.

## Open questions

- Session recovery, and retrying requests across a reconnect, once the server supports recovery.
- A blocking wrapper for scripts, if one is wanted.
