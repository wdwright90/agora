# Handoff

Checkpoint: 2026-09-22. AGORA-002 remains active. This session developed the simulation/server interface and lifecycle through discussion; no implementation or acceptance of the overall draft design/package mapping was requested.

## Resume here

Superseding decision in AGORA-003: agents are session-owned, per-agent control tokens are removed, and session expiry triggers cleanup of owned agents. The historical token-bearing spawn and per-agent recovery descriptions below are superseded by the current SADD and CDD. Session recovery credentials remain required.

An earlier decision established uniform random spawn placement among all currently unoccupied cells, evaluated separately when each spawn is applied. The policy is recorded in the simulation CDD; failure when no cell is available remains unchanged.

User-defined environments must be saved to the catalog and validated by Agora tooling before run creation, providing both a validation entry point and a reusable library. Programmatic authoring/configuration and catalog selection are the current API direction; uploads remain a possibility and would use the same catalog admission path. Simulation-provided random generation is separate, without prior user catalog admission. The latest decision retains generated initial definitions with their runs and skips catalog addition by default. Explicit catalog addition remains available through validation; the catalog should contain intentionally crafted or selected environments rather than accumulate routine random outputs. These rules are canonical in the SADD's [environment configuration and selection section](../../docs/architecture/sadd.md#environment-configuration-and-selection); concrete API design remains open; the selected MVP scope is described below.

The latest clarification requires retaining all seeds and generation metadata needed to reproduce the environment alongside the generated definition, including generator identity/version, configuration, and any other required inputs or dependency versions. Retaining this information is required; its concrete schema remains open.

The latest milestone decision is to start with a bundled, prevalidated catalog entry for a bounded, empty 10 × 10 grid. Environment-authoring APIs, user catalog admission tooling, uploads, and random environment generation are deferred beyond the first milestone; random agent spawning remains supported.

The latest decision makes catalog entry ID an input to the client-facing create-run request. The viewer automatically supplies the bundled entry's ID for the first milestone, avoiding a catalog-selection UI. ID representation, request schema, and how catalog selection resolves into simulation creation configuration remain open.

The latest decision makes readiness queries return phase/status and current state ID, with missing-agent IDs only during collection. Setup, execution, and started-empty status are explicit and do not imply readiness to advance. Normal request-processing boundaries still apply.

The latest decision applies observation suppression at the next normal request-processing opportunity. A request captured in the fixed post-action batch suppresses observations for that step; requests after the cutoff take effect after step completion. Physical senses and the body remain unchanged.

The latest decision restores observation generation when reconnection handover completes. In a started run, send a fresh current-state observation to the replacement client without stepping or incrementing the state ID; during execution, finish the entire step before handover. Setup observations still wait until Start. The SADD owns this recovery rule.

The latest decision defers viewer-state reads during execution until the entire step completes, returning membership changes and the resulting state ID. Setup/collection reads are prompt, including while paused; newly attached viewers follow the same rule. No partial execution state is exposed.

The latest decision keeps only the newest unsent full-state snapshot per viewer, replacing older pending updates. Slow viewers may skip states without building a snapshot backlog or delaying simulation. This applies only to viewer snapshots; command-response and agent-observation delivery remain separate.

The latest decision uses one completion rule for spawn/removal requests: return the final result when application is resolved, without a separate queued acknowledgement. Immediate setup/empty-run application and deferred step-boundary application have the same client-facing result semantics. Existing direct rejection rules, including already-pending removal, remain unchanged. The earlier two-stage acknowledgement proposal was not adopted.

The latest decision requires client-supplied correlation IDs echoed in success/error responses, including deferred membership results. The SADD owns the rule. Completed requests retried with the same ID and contents return the stored result without re-execution; different contents under the same ID return an error. An identical pending retry stays associated with the original operation and receives its final result when ready, without duplicating work, changing queue order, or producing a queued acknowledgement. Request identity is now scoped by logical client session plus correlation ID. Sessions survive connection loss, and recovery requires proof of session ownership separate from the correlation ID. The server issues the session identity; clients generate request IDs before sending and reuse them for retries, with provided client libraries managing generation for application code. The session-establishment exchange, recovery credentials, and the relationship to agent control tokens remain open, along with numeric ID representation and content comparison.

The latest decision retains completed results for a bounded retry window and tracks pending requests until resolved. Expired retries return an explicit result-expired error without re-execution. The retry window is configurable and defaults to five minutes from completion; retries do not extend expiry. How expired IDs remain recognizable is still open; discarding result payloads must not turn retries into new work.

The latest agreement selects increasing per-session request numbers, with client libraries coordinating allocation and ordered sending of new requests. Numbering survives connection recovery; responses may complete out of order. The server checks pending/retained records and tracks the highest admitted number, rejecting older unrecorded numbers without execution rather than retaining every expired ID. Numeric representation, gap/out-of-order handling, and recovery synchronization remain detailed-contract work.

The latest decision keeps sessions alive while connected and permits recovery for a configurable five-minute window after disconnection. Expired-session recovery returns an explicit error and requires a new session. Session expiry remains separate from result retention, run cleanup, and agent-control recovery.

The latest decision allows already-admitted operations to continue under normal lifecycle rules after session expiry, without cancellation, rollback, or queue reordering. It does not force advancement while paused or restore the expired session. Result disposal/access from a new session remains unspecified.

The latest decision makes a successful spawn result contain the correlation ID, assigned agent ID, and opaque agent-control token. Spawn coordinates are omitted. The SADD owns the client-facing rule; the server owns token issuance.

The latest decision makes a successful removal result contain the correlation ID and removed agent ID, confirming completed removal. Existing already-pending and agent-not-found responses remain for distinct requests; identical retries retain the previously agreed correlation behavior.

The latest decision makes a successful agent-control recovery result contain correlation ID, recovered agent ID, current state ID, and run phase/status, returned after handover and any applicable pending-action reset complete. A fresh observation accompanies it in a started run; setup still defers observations until Start.

The latest decision establishes a common request-error response: correlation ID, stable programmatic error code, human-readable message, and structured details when relevant. Clients use codes/details rather than parsing message text. The SADD owns this rule; concrete codes, detail schemas, and malformed requests without usable correlation IDs remain open.

The latest decision makes a successful create-run response contain correlation ID, assigned run ID, selected catalog entry ID, initial state ID (`0`), and phase (`setup`). The run ID can be shared with joining clients. Creator-authority recovery remains separate and unresolved.

The maintainer has stopped exhaustive request/response enumeration: the general patterns are sufficient, and missing fields should be filled in as encountered. The proposed Start response fields and acknowledgement timing were not accepted; existing Start lifecycle decisions remain in force. Do not resume field-by-field questions by default.

Recommended next major discussion: server/client connection and run management, including session versus agent/creator authority, joining and recovery, and delivery/failure handling. These are follow-up design areas rather than missing simulation operations. Simulation internals (Bevy state/schedules and the post-action request boundary) still need design before implementation. This is a next-step recommendation, not acceptance of a new design or completion of AGORA-002.

The maintainer subsequently authorized that discussion; current work continues in [AGORA-003](../AGORA-003-server-client-connections/brief.md). AGORA-002 remains active for outstanding simulation design.

Generated-definition storage format, retention, and reproduction-metadata schema remain future work. Broader run reproducibility remains distinct from recreating an initial environment.

## Maintained design

- [Simulation CDD](../../docs/components/simulation/cdd.md): operation table with inputs, outputs, lifecycle availability, and explicit open details; simulation lifecycle and state numbering.
- [SADD run lifecycle](../../docs/architecture/sadd.md#run-lifecycle): canonical setup/start, request boundaries, and membership rules.
- [SADD server coordination](../../docs/architecture/sadd.md#server-coordination-and-recovery): authority, submissions, client routing, pacing, and recovery.

These documents own requirements. The following is a navigation summary, not a replacement contract.

## Current decisions

- Actions target current state N; real execution produces N+1 before observations. Submissions contain agent ID, target state ID, and action. First valid submission is fixed; reject later submissions for that agent/state. Multi-agent batches use per-entry acceptance in supplied order and finish processing before advancement is considered.
- Readiness exposes phase/status and current state ID, with missing agent IDs only during collection. Advancement returns resulting state ID, observations by agent, and request-associated membership results. Premature advancement changes no state or queues. No per-action execution outcomes. The server sends one observation batch per client per completed state using current ownership.
- Setup happens once at state 0. Clients connect and spawn; viewer data is available. The creator alone can Start after intended participants confirm successful spawning. Start sends initial observations and opens collection while preserving pacing. Reject Start when there are neither agents nor connected viewers; viewer-only starts are allowed. No setup removal is required.
- After all agent actions finish, capture a fixed batch of waiting requests; handle it before removals, spawns, state increment, and observations. Later arrivals wait until boundary completion. Outside execution/boundary completion, requests are handled promptly under operation-specific rules, even while paused. Populated-run membership changes remain queued. Empty-run admission requires no execution, counter increment, or repeated Start.
- Removals precede spawns; each queue follows server acceptance order. Placement is checked as applied, so later requests for a cell occupied by an earlier spawn fail. Repeated removals follow the response rule described above. Agent-action execution remains randomized.
- Viewer state is separate and initially complete: state ID, grid dimensions, agent IDs and positions. Future large environments require pan/zoom; delivery optimizations remain open. Viewing defaults to a tunable 500 ms minimum interval between step starts, with pause/single-step and no display acknowledgements. The default needs evaluation with a real viewer.

## Follow-up and open details

Python client design must cover coordinating multiple training clients: assembling participants, confirming spawning and preparation, and explicitly starting their shared run. No mandatory super-client or automatic expected-count/slot policy was selected.

Other open details include grid configuration and reproducibility, concrete API/error/request-correlation representations, creator-authority recovery, first pacing interval timing, Bevy/view delivery integration, and concrete recovery-result representations. Future environment-only animation is a use case, but its execution semantics remain undefined. Existing reconnection handover rules still apply; the general request boundary does not silently replace them.

## Repository checkpoint

Current resume checkpoint: branch `docs/simulation-cdd` at `be17bc1`, which committed the previous session changes described below. The working tree was clean before recording the uniform-placement decision; the subsequent spawning, environment/catalog, MVP, and readiness decisions are uncommitted documentation changes. The local remote-tracking branch matched HEAD; live remote state was not checked.

Previous session record: The prior checkpoint is `5d3bdf6` (`Document simulation lifecycle and CDD discussion checkpoint`). The maintainer subsequently authorized committing and pushing this session's SADD, simulation CDD, and handoff changes. This record is prepared before that commit; verify the resulting commit and push from Git history and remote state on resume. Git status warned that the global ignore file was inaccessible but listed only these three modified files.

## Verification

For the 2026-09-22 discussion updates, `git diff --check` passed. Documentation-only changes; no builds or application tests were run. The checks below belong to the previous session.

`git diff --check` passed. A PowerShell check verified all 23 relative Markdown file targets across the SADD, simulation CDD, and this handoff; anchors were not checked. Documentation-only work; no application builds or tests apply. No source packages were introduced.
