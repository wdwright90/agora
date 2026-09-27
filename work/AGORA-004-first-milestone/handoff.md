# Handoff

Checkpoint: 2026-09-27, PR 4c (`agora-server` pacing) on branch `feature/server-pacing`. PR 1 (`agora-sim` core) merged as #3, PR 2 (client protocol) as #4, PR 3 (`agora-server` part 1) as #5, PR 4a (steps and observations) as #7, and PR 4b (viewers) as #8. With 4c, the server side of the milestone is complete.

## Resume here

Check PR 4c's review comments with `gh pr view <number> --comments` and the inline comments through the GitHub API. Answer questions, and agree any changes before making them on `feature/server-pacing`.

Once 4c has merged, agree the scope of PR 5 (`agora-client`: the Rust client library and a random-mover demo client) with the maintainer before implementing it. The viewer (PR 6) needs a viewer CDD first; the maintainer asked on #8 how views will be drawn, and the answer sketched Bevy sprites synced from complete views, a background network task, and interpolation between steps.

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
  - PR 4 is split: 4a is steps and observations, 4b is viewers with a fixed step interval, and 4c is pacing controls. The 4b/4c split was agreed on 2026-09-26.
  - An expired session's agents are removed (option (a)), so a run with other connected clients does not stall. The maintainer confirmed that session expiry must still let a run release itself, and that agents must never hold a world open.
  - Choices made in 4a, agreed by the maintainer on 2026-09-26:
    - Protocol 0.2.0. `submit` carries one `state_id` and a list of `{agent_id, action}`. Actions are tagged by `kind` (`move`), and each kind has its own parameters beside `kind`. The maintainer agreed flat parameters, with shared names such as `version` reserved (SPEC-002 open questions).
    - Wrong phase and wrong state fail the whole request. Ownership, duplicate, and agent-limit failures are per entry, with an optional `error` object on each result.
    - `observations` is a push without a request ID, one per session with only its agents. It follows the response to the request that caused it.
    - Without viewers, the run advances as soon as every agent has an action (the SADD's unlimited pacing).
    - `Simulation::remove` is immediate and works in any phase. Client-requested queued removal is separate and not built.
  - From review of 4a: direction values should eventually come from the environment's grid layout through capability definitions (simulation CDD open questions). The protocol-to-simulation mapping is intended; only the action conversion is expected to become generic (server CDD).
  - Agreed for 4b: `watch` is a separate request any session can make, and stopping watching is only by disconnecting.
  - Choices made in 4b, agreed by the maintainer on 2026-09-26 (the newest-only rule stays verified by design, not by a test):
    - Protocol 0.3.0. `watching {request_id, view}` answers `watch`, and `view_update {view}` is the push. A view is `{state_id, phase, width, height, agents: [{agent_id, x, y}]}`. `phase` is included so a viewer knows whether to offer Start.
    - Watching twice returns the current view again rather than an error.
    - Views are published after a spawn, Start, a step, and an expiry removal, through a Tokio `watch` channel per connection, which keeps only the newest view.
    - The step interval (`--step-interval-ms`, default 500 ms) applies only while a viewer is connected, measured between step starts. The first step after Start is not held. When the last viewer leaves, a held step executes at once.
  - Agreed for 4c (2026-09-27):
    - Pacing control is claimed explicitly with `claim_pacing`; the viewer app will claim right after `watch`.
    - `watching` carries a `pacing {mode, you_control}` state, and `pacing_update` pushes it to every viewer on any mode or controller change.
    - `step_once` is answered with `step_granted` at once; the viewer waits for the `view_update`.
    - Intervals range from 1 ms to one hour; `unlimited` is a separate mode.
  - Choices made in 4c, open to review:
    - Protocol 0.4.0. Modes are tagged by `kind`: `paused`, `unlimited`, and `interval` with `ms`.
    - The first viewer resets the mode to the default interval with no controller; the last viewer leaving makes the run unlimited. Claiming control you already hold succeeds.
    - Grants do not accumulate, and any mode change cancels an unused grant.
    - A new interval is measured from the previous step's start.
    - Connections are numbered in acceptance order to pick the handover successor.
    - The pacing state machine lives in `agora-server/src/pacing.rs`.

## PR 4c contents

- `agora-protocol`: version 0.4.0; `claim_pacing`, `set_pacing`, `step_once`, their responses, and `pacing_update`; `pacing` in `watching`; `IntervalMs`, `PacingMode`, and `Pacing`; four new error codes. New fixtures, and version fixtures bumped.
- `agora-server`: the pacer module, pacing commands in the run task, handover to the oldest connection, connection numbering, and a bounded `--step-interval-ms`.
- Specs: SPEC-002-R12 updated and R13 added; SPEC-003-R11 rewritten, and R12 to R14 added.
- CDD-002, the SADD's pacing-timing sentence, `rust/README.md`, and the work index updated.

## PR 4b contents

- `agora-protocol`: version 0.3.0; `watch`, `watching`, and `view_update`; `View` and `AgentView`. New fixtures, and version fixtures bumped.
- `agora-server`: `ServerConfig::step_interval` and `--step-interval-ms`; a newest-only view slot per connection; view publishing; viewers counting toward Start eligibility; and the step interval while viewers are connected.
- Specs: SPEC-002-R12; SPEC-003-R04 and R08 updated, and R10 and R11 added.
- CDD-002, `rust/README.md`, the brief, and the work index updated.

## Verification

In `rust/`:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --no-deps --workspace` with `-D warnings`: all passed.
- `cargo test --workspace`: 104 passed (35 in agora-sim, 12 in agora-protocol, and 57 in agora-server: 2 unit, 31 SPEC-002, and 24 SPEC-003).
- The agora-server tests passed in 8 consecutive runs.
- `agora-server --step-interval-ms 0` is rejected by the argument parser.

The newest-only view rule is not tested directly; it follows from the channel type. No manual run against a live client was done; the demo client arrives in PR 5. No CI exists.

## Open follow-ups

- A connected client that stops submitting holds its run until deadlines and fallback control are built.
- Closing a setup run whose creator expires needs a closure message.
- Retrying a lost `create_run` or `join_run` response is unresolved until session recovery. Pushes lost with a connection are also left to recovery.
- Spawning after Start is still rejected (SPEC-001-R06, SPEC-003-R05).
- A run started by viewers alone cannot step, because stepping without agents is undefined, and spawning after Start is rejected. The milestone viewer should spawn or wait for agents before Start.
- The request log moves from the connection into shared session state when session recovery is built.
