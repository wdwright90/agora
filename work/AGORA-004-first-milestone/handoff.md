# Handoff

Checkpoint: 2026-09-27, PR 6 (`agora-viewer`) open as #13 on branch `feature/viewer`, the last PR of the milestone. PR 1 (`agora-sim` core) merged as #3, PR 2 (client protocol) as #4, PR 3 (`agora-server` part 1) as #5, PR 4a (steps and observations) as #7, PR 4b (viewers) as #8, PR 4c (pacing) as #9, PR 5 (`agora-client`) as #10, and ADR-002 as #12.

## Resume here

Check PR 6's review comments with `gh pr view 13 --comments` and the inline comments through the GitHub API, and agree any changes before making them on `feature/viewer`.

The maintainer ran the milestone's manual check on 2026-09-27 (the server, `agora-viewer --create`, and two `agora-demo join` clients; SPEC-005, "Manual check") and confirmed it in the window. Once PR 6 has merged, mark AGORA-004 completed in its brief and the work index, and agree the next task with the maintainer.

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
  - Choices made in 4c, accepted without notes when #9 merged:
    - Protocol 0.4.0. Modes are tagged by `kind`: `paused`, `unlimited`, and `interval` with `ms`.
    - The first viewer resets the mode to the default interval with no controller; the last viewer leaving makes the run unlimited. Claiming control you already hold succeeds.
    - Grants do not accumulate, and any mode change cancels an unused grant.
    - A new interval is measured from the previous step's start.
    - Connections are numbered in acceptance order to pick the handover successor.
    - The pacing state machine lives in `agora-server/src/pacing.rs`.
- **PR 5** (agreed 2026-09-27):
  - The client library is async on tokio. Observations arrive through a stream; views and pacing are latest-value handles (Tokio `watch` receivers).
  - The demo has `create [--start-at M] [--unlimited]` and `join <RUN_ID>`, spawns `--agents` agents, and moves them at random until `--steps` or Ctrl-C.
  - The client gets its own CDD (CDD-003) and spec (SPEC-004).
  - The maintainer decided not to guard against or document clients reading views to "cheat": Agora is for training, and cheating only defeats the client's own training.
  - Settled in PR 5 review before opening (2026-09-27), and merged as #10:
    - `create_run` and `join_run` consume the `Client`, so a connection carries at most one session, as the protocol requires. The maintainer agreed: programs in several runs open several connections.
    - `Session` is a cloneable handle, and the observation stream is a separate `Observations` value returned with it. The first draft kept the stream inside the session, so a session shared between tasks could not read observations; the maintainer asked for the fix in this PR.
    - Request numbering and queueing happen under one lock, so concurrent tasks send in number order (a bug the first draft had, caught by a test).
    - `ServerMessage::request_id()` was added to `agora-protocol`, without a wire change.
    - The demo is behind a `demo` feature, on by default, at the maintainer's request.
    - The server's run log span no longer nests under the connection that created the run; a manual run showed the joiner's events logged under the creator's connection.
- **PR 6 viewer design** (agreed 2026-09-27):
  - `rust/agora-viewer`: a Bevy 0.19 binary with only the needed features, depending on `agora-client` (no default features) and `agora-protocol`, never `agora-sim`.
  - A Tokio runtime on a background thread runs the client. UI actions reach it over a channel, and results and errors come back over another. A Bevy system reads the view and pacing latest-value handles each frame.
  - Rendering: a 2D camera fitted to the grid, grid cells, and a shape per agent with a colour derived from its ID and its ID as a label. Entities are synced to each complete view, and movement is interpolated over a short time capped by the step interval. Rendering is data-driven (ADR-002). No pan or zoom yet.
  - UI: `bevy_egui` 0.42 (it targets Bevy 0.19). A side panel shows status (a copyable run ID, state, phase, agent count, pacing mode, and whether this viewer controls pacing) and controls. Start is enabled only in setup with at least one agent. Pause, Step, Resume, an interval slider, and Unlimited are enabled only with control. Claim control is offered when control is vacant.
  - Viewer startup is separate from run creation: `agora-viewer --create` or `agora-viewer --join <RUN_ID>`, one run per window. A future run browser (which needs a list-runs message) can open viewer windows by launching `--join`. After creating or joining, the viewer watches and claims control when vacant.
  - Tests cover the entity sync in a headless Bevy app, the network bridge against an in-process server, and the control-enabling rules. The visuals are checked by hand by the maintainer.
  - One PR, with CDD-004 and SPEC-005.
- **ADR-002** (accepted 2026-09-27): three layers of viewer data (the stable view; an opt-in, same-build debug channel with reflected component dumps, debug-drawing primitives emitted by systems, and mirrored observations; and ownership of view export, reflection, and debug drawing beside each component). Sharing simulation component types with the viewer, through a data-only crate for the debug layer only, waits for a trigger: a component that needs a typed inspector, or the generic tree proving too weak. From now on simulation components derive `Reflect`; `Agent` and `Position` in `agora-sim` do not yet, and gain it the next time the simulation changes.

## PR 6 contents

- `rust/agora-viewer`: a library (`network` bridge, `state`, `render`, `controls`, and the `egui` panel) and the `agora-viewer` executable, built on Bevy 0.19 (`2d` feature set) and `bevy_egui` 0.42.
- Workspace: `bevy`, `bevy_egui`, and `agora-client` (without default features) added to the workspace dependencies.
- Docs: CDD-004 and SPEC-005 (new), the components index, the SADD package status, `rust/README.md` (package table and how to run the milestone), the work index, and this handoff.
- Choices made in PR 6, accepted by the maintainer after the manual check:
  - The control panel is an anchored `egui` window rather than a side panel; `egui` 0.36 changed its panel API, and the camera keeps the grid clear of the window instead.
  - Claim control is offered whenever this viewer lacks control, because the protocol does not say whether control is vacant.
  - Move easing takes the shorter of 300 ms and 80% of the interval, 300 ms while paused, and snaps when unlimited.
  - The interval slider covers 20 ms to 5 s on a logarithmic scale.

## PR 5 contents

- `rust/agora-client`: the library (`Client`, `Session`, `Observations`, `SessionInfo`, `ObservationBatch`, and `ClientError`) and the `agora-demo` executable behind the default `demo` feature.
- `agora-protocol`: `ServerMessage::request_id()`.
- `agora-server`: the run's tracing span has no parent.
- Docs: CDD-003 and SPEC-004 (new), the components index, the SADD package status, `rust/README.md` (package table and demo instructions), the work index, and this handoff.

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
- `cargo test --workspace`: 121 passed (35 in agora-sim, 12 in agora-protocol, 57 in agora-server, 9 in agora-client, and 8 in agora-viewer: 2 unit and 6 integration).
- The viewer was launched against a server with two demo clients joined: it opened its window on the GPU, created and watched the run, claimed pacing control, and ran without errors until stopped. The maintainer then pressed Start and checked the drawing and controls in the window: the manual check passed.
- The agora-client and agora-server tests passed in 8 consecutive runs. `agora-client` also passes clippy and its tests with `--no-default-features`, and then has no demo dependencies. The R03 ordering test failed every time when out-of-order sending was reintroduced on purpose.
- Manual run: `agora-server` with a `create --start-at 2` demo and a `join` demo, each with one agent. Both saw states 0 to 5 in step, the steps after the first were about 500 ms apart because the creator watches, both exited successfully, and the server returned to unlimited pacing when the viewer left. A second run confirmed the run's log lines are no longer under the creator's connection.

The newest-only view rule is not tested directly; it follows from the channel type. No CI exists.

## Open follow-ups

- A connected client that stops submitting holds its run until deadlines and fallback control are built.
- Closing a setup run whose creator expires needs a closure message.
- Retrying a lost `create_run` or `join_run` response is unresolved until session recovery. Pushes lost with a connection are also left to recovery.
- Spawning after Start is still rejected (SPEC-001-R06, SPEC-003-R05).
- A run started by viewers alone cannot step, because stepping without agents is undefined, and spawning after Start is rejected. The milestone viewer should spawn or wait for agents before Start.
- The request log moves from the connection into shared session state when session recovery is built.
