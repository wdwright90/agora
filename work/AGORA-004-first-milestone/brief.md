# AGORA-004 — First usable milestone

Status: active
Owner: maintainer

## Goal

Implement the SADD's [first usable milestone](../../docs/architecture/sadd.md#first-usable-milestone): an empty 10 × 10 grid where two connected agents submit moves, receive empty observations, and appear in the Bevy viewer.

## Scope and exclusions

The work is delivered as a sequence of small PRs against `develop`, following the [features and specs workflow](../../docs/workflows/ai-development.md#features-and-specs). Scope for each PR is agreed before implementation.

1. `agora-sim` core: grid, setup spawning, Start, submission, readiness, advancement, viewer state ([SPEC-001](../../docs/components/simulation/specs/lifecycle-and-movement.md)).
2. Minimal protocol contract and `agora-protocol`: JSON envelope, versioning, errors, and the messages the milestone needs, with fixtures.
3. `agora-server`, part 1: WebSocket host; create, join, and spawn; creator-only Start.
4. `agora-server`, part 2: submission and advancement, observation routing, viewer snapshots, and basic pacing.
5. `agora-client`: Rust library and a demo random-mover client.
6. `agora-viewer`: Bevy rendering of the grid and agents, with run creation, Start, and pause controls.

Out of scope for this task:

- deadlines and fallback control
- session recovery and expiry
- post-Start membership changes
- recording
- the Python client
- parallel-run acceptance

## Acceptance criteria

- Each SADD milestone criterion is demonstrated with the Rust demo client and viewer.
- Every PR's specs, tests, and documentation are updated together, and its checks pass.

## Dependencies and open questions

- [AGORA-003 handoff](../AGORA-003-server-client-connections/handoff.md): architecture decisions this work builds on, and the details to settle as features need them.
- Protocol details (ID and counter representations, the version-compatibility policy, and bootstrap correlation) were settled in PR 2. The [handoff](handoff.md) records them.
