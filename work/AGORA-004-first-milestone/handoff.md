# Handoff

Checkpoint: 2026-09-24, PR 2 (client protocol, part 1), branch `feature/protocol-setup`. PR 1 (`agora-sim` core) was merged as #3.

## Resume here

When PR 2 has been reviewed and merged, agree the scope of PR 3 (`agora-server`, part 1: WebSocket host, handshake, create/join, spawn, and Start) with the maintainer before implementing it.

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

## PR 2 contents

- [SPEC-002](../../docs/contracts/client-protocol.md) (draft), with fixtures in `docs/contracts/fixtures/client-protocol/`.
- `rust/agora-protocol`: serde types, plus tests named by requirement ID.
- Choices for review:
  - A second `hello` returns `malformed_message`.
  - `unsupported_protocol_version` lists the supported versions as an array.
  - The bundled catalog entry ID appears in the fixtures as `empty-grid-10x10`, but the entry itself is defined in PR 3.

## Verification

In `rust/`:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --no-deps` with `-D warnings`: all passed.
- `cargo test --workspace`: 40 passed (30 in agora-sim, 10 in agora-protocol).
- Each invalid fixture was confirmed to fail for its intended reason.

No CI exists.

## Open follow-ups

- Retrying a lost `create_run` or `join_run` response is unresolved until session recovery.
- PR 4 adds step, observation, viewer, and pacing messages. Observation batches will be lists, because JSON object keys must be strings.
