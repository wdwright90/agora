# Handoff

Checkpoint: 2026-09-23. Broad architecture discussion is complete enough to begin first-feature planning. The maintainer requested this handoff, a commit, and a push before stopping for the day.

## Resume here

Start with the first empty-grid vertical feature: create a run, connect two agents, execute moves, deliver empty observations, and display resulting state in the viewer. Consolidate the server component design and define only the shared contracts and implementation steps this feature needs. Do not restart exhaustive response-field or state enumeration. The maintainer wants remaining details resolved alongside features.

Use the [SADD](../../docs/architecture/sadd.md) as the canonical source and the [simulation CDD](../../docs/components/simulation/cdd.md) for simulation boundaries. Both remain drafts. Individual discussion decisions do not automatically accept entire documents or the complete package proposal. No implementation was started. AGORA-002 and AGORA-003 remain active; no server CDD or detailed shared contracts exist yet.

## Decisions captured at this checkpoint

- Session expiry: finish an executing step in full, then clean up owned agents. Otherwise clean up promptly during setup, collection, or pause without advancing. Admitted spawns continue normally and any agents they create after expiry receive cleanup. Whole-run shutdown handling remains separate.
- Creator authority: follows session recovery; expiry leaves authority unassigned for started runs without forcing their closure. If the creator expires during setup, close the unstarted run and notify remaining connected clients.
- Pacing: first valid claim gets vacant control, without creator priority. On controller disconnection, preserve pacing and automatically transfer to the remaining viewer with the oldest active connection. Reconnection neither restores control nor preserves connection age.
- Recovery: one synchronized result after handover establishes current agents, run state/status, and pending requests before commands resume. Existing fresh-observation and pending-action-reset rules apply.
- Slow agents: use action deadlines and fallback observation suppression. The proposed additional application-level bounded outgoing queue and overflow-disconnection policy were rejected.
- Execution boundaries: independently running server, viewer as a client, separate world and execution schedule per run. Headless operation and parallel training are required; parallel runs remain outside first-milestone acceptance.
- Communication: one persistent bidirectional connection per connected session; JSON with explicit message types and fields over WebSocket, one JSON object per text message. Same network transport for localhost and remote clients. Protocol versioning is agreed; transport abstraction can be added if needed.
- Extensions: environment definitions build environments and do not specify allowed agent types. Clients declare agent capabilities compatible with simulation-side Rust components/systems. Observations convey perceived conditions without necessarily naming internal effects.
- Capability contracts: explicit public Rust contracts generate authoritative language-neutral definitions alongside implementations. Capability versions are separate from protocol versions. Shared constraints drive validation and generation; registration binds supported definitions, versions, and implementation. Plan export-regeneration checks and behavioral tests. Internal Bevy layouts remain separate; tooling is not implemented.
- Reproducibility: same build/platform, initial state, seeds, and ordered inputs produce identical states. Runs own separate seeded streams for generation, spawning, and action shuffling. Re-execution needs actual admitted inputs, including fallback and membership changes; input-history capture can come later.
- Sharing: future recordings preserve resulting states and observations for inspection without original training clients/models. Recipients may need the producing code version. Cross-version/platform deterministic re-execution is not promised; recording format and cross-platform playback remain open.

Earlier run/session hierarchy, session-owned agents, create/join/resume, correlation/retry, and recovery-window decisions remain in the SADD. Historical per-agent-token summaries in AGORA-002 are superseded by current session ownership.

## Details to settle with features

- Minimal protocol contracts, bootstrap correlation, credentials, and recovery delivery ordering. String IDs/counters and exact-version-only compatibility were proposed but not explicitly agreed.
- Connection-health detection, timeout values, command-response delivery/backpressure, closure notifications, and disposition of admitted requests during whole-run closure.
- Concrete Rust package interfaces, worker allocation, capability schema tooling, deterministic scheduling, and random stream derivation.
- Follow-on capabilities, recording formats, and storage are later work; do not expand the first milestone to cover them.

## Git checkpoint

The session began on `docs/simulation-cdd` at `cd2f8db` with a clean working tree and matching local upstream tracking ref. The maintainer authorized committing and pushing this checkpoint to the current branch. This handoff is prepared before that commit; verify its resulting hash and live push state from Git on resume. Git warns that the global ignore file is inaccessible.

## Verification

Documentation was reviewed against this session's decisions. `git diff --check` passed. Relative Markdown targets and heading anchors in the checkpoint's changed Markdown files were checked. No application builds or tests were run: no implementation packages exist. Schema generation, export-drift checks, behavioral tests, determinism checks, and playback remain future implementation work.

Suggested next-session instruction: "agora-resume — pick up first-feature planning from AGORA-003."
