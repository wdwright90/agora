# Handoff

Checkpoint: 2026-09-26, PR 4a (`agora-server` steps and observations) on branch `feature/server-steps`, ready for review. PR 1 (`agora-sim` core) merged as #3, PR 2 (client protocol) as #4, and PR 3 (`agora-server` part 1) as #5.

## Resume here

Check PR 4a's review comments with `gh pr view <number> --comments` and the inline comments through the GitHub API. Answer questions, and agree any changes before making them on `feature/server-steps`.

Once 4a has merged, agree the scope of PR 4b (viewer subscriptions and snapshots, viewer-aware Start eligibility, and pacing controls) with the maintainer before implementing it.

## Decisions

- **PR 1:** the maintainer accepted ADR-001. Rust 1.98.1 is pinned, and Bevy 0.19 is used through `bevy_ecs` only.
  - Randomness uses per-run ChaCha8 streams: stream 0 is reserved for generation, stream 1 is for spawning, and stream 2 is for shuffling.
  - Agent IDs are `u64`, assigned from 1.
  - Specs are written alongside features ([workflow](../../docs/workflows/ai-development.md#features-and-specs)).
  - From review: agent-limit errors (`AgentLimitError`) are separate from submission errors. The grid occupancy index stays because grids will grow.
- **PR 2:**
  - Run, session, and catalog IDs are opaque strings. Request, agent, and state IDs are JSON integers capped at 2⁵³ − 1.
  - Protocol versions are semver strings. The MVP accepts only an exact match, and semver backward compatibility applies later.
  - `hello` belongs to the connection. `create_run` and `join_run` establish the session and supply its first request number.
  - `agora-sim` does not depend on `agora-protocol` for the MVP; the server maps between them.
  - Error codes are stable snake_case strings, and agent-limit codes use an `agent_limit.` prefix.
  - From review: the shared fixtures stay under `docs/contracts/fixtures/`, and `hello` stays a separate handshake.
- **PR 3:**
  - Stack: tokio, `tokio-tungstenite`, `tracing`, and `clap`. Each run is a task that owns its `Simulation`; connections send it commands.
  - Lifetimes: a disconnected session expires after 2 minutes by default. A run with no sessions is released after 5 minutes, and a join cancels that. Release depends on sessions, not agents.
  - Each session keeps its 5 most recent results for retries (SPEC-002-R07).
  - From review: environment code is expected to move into its own package later, once a real definition format or an outside consumer exists. The SADD package section records this.
- **PR 4** (agreed 2026-09-26):
  - PR 4 is split: 4a is steps and observations, and 4b is viewers and pacing.
  - An expired session's agents are removed (option (a)), so a run with other connected clients does not stall. The maintainer confirmed that session expiry must still let a run release itself, and that agents must never hold a world open.
  - Choices made in 4a, agreed by the maintainer on 2026-09-26:
    - Protocol 0.2.0. `submit` carries one `state_id` and a list of `{agent_id, action}`. Actions are tagged by `kind` (`move`), and each kind has its own parameters beside `kind`. The maintainer agreed flat parameters, with shared names such as `version` reserved (SPEC-002 open questions).
    - Wrong phase and wrong state fail the whole request. Ownership, duplicate, and agent-limit failures are per entry, with an optional `error` object on each result.
    - `observations` is a push without a request ID, one per session with only its agents. It follows the response to the request that caused it.
    - Without viewers, the run advances as soon as every agent has an action (the SADD's unlimited pacing).
    - `Simulation::remove` is immediate and works in any phase. Client-requested queued removal is separate and not built.

## PR 4a contents

- `agora-sim`: `Simulation::remove` and `RemoveError` (SPEC-001-R18).
- `agora-protocol`: version 0.2.0; `submit`, `submitted`, and `observations`; `Action`, `Direction`, `ActionEntry`, `EntryResult`, `EntryError`, `AgentObservation`, and `Observation`; five new error codes. New fixtures, and version fixtures bumped.
- `agora-server`: per-connection outboxes for pushes, `submit` handling, advancement on readiness, observation routing by owner, and agent removal on session expiry.
- Specs: SPEC-001-R18; SPEC-002-R10 and R11 and the new codes; SPEC-003-R03, R06, and R07 updated, and R08 and R09 added.
- CDD-002 and CDD-001 updated, the SADD package section notes the environment-package direction, and `rust/README.md` is updated.

## Verification

In `rust/`:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --no-deps --workspace` with `-D warnings`: all passed.
- `cargo test --workspace`: 90 passed (35 in agora-sim, 12 in agora-protocol, and 43 in agora-server: 2 unit, 27 SPEC-002, and 14 SPEC-003).
- The SPEC-003 timer tests passed in 8 consecutive runs.

No manual run against a live client was done; the demo client arrives in PR 5. No CI exists.

## Open follow-ups

- A connected client that stops submitting holds its run until deadlines and fallback control are built.
- Closing a setup run whose creator expires needs a closure message.
- Retrying a lost `create_run` or `join_run` response is unresolved until session recovery. Pushes lost with a connection are also left to recovery.
- Spawning after Start is still rejected (SPEC-001-R06, SPEC-003-R05).
- The request log moves from the connection into shared session state when session recovery is built.
