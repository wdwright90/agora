# Handoff

Checkpoint: 2026-09-27, chunk 1 (run lifecycle) on branch `feature/run-lifecycle`, ready for review. The planning PR merged as #16.

## Resume here

Check chunk 1's review comments with `gh pr view <number> --comments` and the inline comments through the GitHub API, and agree any changes before making them. Once it has merged, agree the scope of chunk 2 (registry and appearance) with the maintainer.

## Chunk 1

Agreed scope (2026-09-27): `leave_run` and `close_run`; the `run_closed` push; closing a setup run when its creator expires (the SADD rule deferred since AGORA-004); sequential sessions per connection on the server; the Rust client keeps one session per connection (`leave` and `close` close it); the viewer's Close run button and closed state; the demo leaving cleanly. `agent_removed` moved to chunk 4.

Choices made in chunk 1, open to review:

- A creator who **leaves** during setup also closes the run (reason `creator_left`), for the same reason as expiry: the run could never start.
- `run_closed` reasons are `closed_by_creator`, `creator_expired`, and `creator_left`; unknown reasons are read as a generic `Other`.
- Before `left` or `closed`, the server sends whatever the run already queued for the session (observations, pacing updates, and a pending view), so nothing from the old run arrives after the session ends.
- `Session::leave` and `Session::close` take `&self`, like the other requests, and end the session for every clone.

Contents:

- `agora-protocol` 0.5.0: `leave_run`, `close_run`, `left`, `closed`, `run_closed`, and `CloseReason`; fixtures.
- `agora-server`: leave and close in the run task with immediate release, closure on creator expiry or leave in setup, and the connection returning to no session.
- `agora-client`: `Session::leave`, `Session::close`, `Session::closed_reason`, and `ClientError::RunClosed`; the demo leaves at its step limit.
- `agora-viewer`: the Close run button and closure messages.
- Specs: SPEC-002-R14 and R15; SPEC-003-R06 (no longer interim), R15, and R16; SPEC-004-R09 and R10; SPEC-005-R07. CDD-002, CDD-003, and CDD-004 updated.

Verification, in `rust/`:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --no-deps --workspace` with `-D warnings`: all passed.
- `cargo test --workspace`: 135 passed (35 agora-sim, 13 agora-protocol, 66 agora-server, 11 agora-client, 10 agora-viewer), in 3 consecutive full runs; the server, client, and viewer tests also passed 8 consecutive runs.
- A viewer test (`r01_creating_watches_the_new_run_and_claims_pacing`) failed once under full-workspace load: the pacing update from the bridge's claim can arrive just after the bridge reports it is watching. The viewer corrects itself on the next frame; the test now waits for the update. The race predates this chunk.
- Manual run: two demo clients with a step limit both left, and the server released the run at once.

## State

- M1 is complete and in `main` ([AGORA-004](../AGORA-004-first-milestone/handoff.md) has its follow-ups, several of which are now scheduled in the roadmap).
- Design decisions are recorded in the SADD's [second milestone](../../docs/architecture/sadd.md#second-milestone-train-and-watch) section and CDD-001, with the reasoning and wire sketches in [context.md](context.md).
- `Agent` and `Position` in `agora-sim` still need `Reflect` (ADR-002); chunk 2 does it.
- The planning PR merged as #16; M1 is in `main`.
