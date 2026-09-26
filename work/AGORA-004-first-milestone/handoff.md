# Handoff

Checkpoint: 2026-09-25, PR 3 (`agora-server`, part 1), open as #5 on branch `feature/server-setup`. PR 1 (`agora-sim` core) merged as #3, and PR 2 (client protocol) merged as #4.

## Resume here

The maintainer is reviewing PR #5. First, check its review comments with `gh pr view 5 --comments` and the inline comments through the GitHub API. Answer questions, and agree any changes before making them on `feature/server-setup`.

Once #5 has merged, agree the scope of PR 4 (`agora-server`, part 2: step submission and advancement, observation routing, viewer snapshots, and basic pacing) with the maintainer before implementing it.

## Decisions

- **PR 1:** the maintainer accepted ADR-001. Rust 1.98.1 is pinned, and Bevy 0.19 is used through `bevy_ecs` only.
  - Randomness uses per-run ChaCha8 streams: stream 0 is reserved for generation, stream 1 is for spawning, and stream 2 is for shuffling.
  - Agent IDs are `u64`, assigned from 1.
  - Specs are written alongside features ([workflow](../../docs/workflows/ai-development.md#features-and-specs)).
  - From review: agent-limit errors (`AgentLimitError`) are separate from submission errors. The grid occupancy index stays because grids will grow.
- **PR 2:**
  - Run, session, and catalog IDs are opaque strings. Request, agent, and state IDs are JSON integers capped at 2⁵³ − 1.
  - Protocol versions are semver strings, starting at 0.1.0. The MVP accepts only an exact match, and semver backward compatibility applies later.
  - `hello` belongs to the connection. `create_run` and `join_run` establish the session and supply its first request number.
  - Start returns a `started` confirmation.
  - `agora-sim` does not depend on `agora-protocol` for the MVP; the server maps between them. The ADR-001 revision records this.
  - Error codes are stable snake_case strings. Agent-limit codes will use an `agent_limit.` prefix when the step messages are added in PR 4.
  - From review: the shared fixtures stay under `docs/contracts/fixtures/`, and `hello` stays a separate handshake.
- **PR 3** (agreed 2026-09-25):
  - Stack: tokio, `tokio-tungstenite`, `tracing`, and `clap`. Each run is a task that owns its `Simulation`; connections send it commands.
  - Start is rejected with `start_not_eligible` when there are no agents and no viewers. Viewers don't exist yet, so Start needs an agent.
  - Lifetimes: a disconnected session expires after the session timeout (default 2 minutes). A run with no sessions is released after the run timeout (default 5 minutes), and a join cancels that. The SADD's release rule was updated from "no controlled agents or observers" to "no sessions".
  - Each session keeps its 5 most recent results for retries (SPEC-002-R07).
  - The SADD's session recovery window default changes from 5 minutes to 2 minutes, to match.
  - Deferred: removing an expired session's agents (needs removal in the sim), and closing a setup run whose creator expires (needs a closure message).
  - The `/agora-resume` Claude Code skill wrapper is included in this PR.

## PR 3 contents

- `rust/agora-server`: library and executable ([CDD-002](../../docs/components/server/cdd.md), [SPEC-003](../../docs/components/server/specs/sessions-and-runs.md)).
- SPEC-002: the retention rule in R07, and server-side verification rows.
- `agora-protocol`: `ClientMessage::TYPES`, and a clearer `Hello` doc comment.
- SADD: the run release rule, the 2-minute recovery window default, the MVP result retention, and the package status.
- `.claude/skills/agora-resume/SKILL.md`, which defers to `.agents/skills/agora-resume/SKILL.md`.

## Verification

In `rust/`:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --no-deps --workspace` with `-D warnings`: all passed.
- `cargo test --workspace`: 75 passed (30 in agora-sim, 10 in agora-protocol, and 35 in agora-server: 2 unit, 23 SPEC-002, and 10 SPEC-003).
- The SPEC-003 timer tests passed in 8 consecutive runs.
- `agora-server --help` and startup were checked by hand.

No CI exists.

## Open follow-ups

- Retrying a lost `create_run` or `join_run` response is unresolved until session recovery.
- PR 4 adds step, observation, viewer, and pacing messages. Observation batches will be lists, because JSON object keys must be strings.
- SPEC-003-R03 agent ownership is recorded but not yet observable. Test it in PR 4, when observations are routed by owner.
- PR 4 will need agent removal in `agora-sim` before expired sessions' agents can be cleaned up.
- The request log moves from the connection into shared session state when session recovery is built.
