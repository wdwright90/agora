# Handoff

Checkpoint: 2026-09-21. AGORA-002 remains active. This session developed the simulation/server interface and lifecycle through discussion; no implementation or acceptance of the overall draft design/package mapping was requested.

## Resume here

The last agreed decision was repeated-removal handling: already-pending for an agent queued for removal, without changing queue order; agent-not-found for an agent that no longer exists.

The next unanswered proposal is **uniform random spawn placement among currently unoccupied cells**, evaluated as each spawn is applied. The maintainer stopped before answering; uniform selection is not an agreed requirement. Failure when no cell is available was already captured. Resume with this placement-policy discussion, then review remaining interface gaps.

## Maintained design

- [Simulation CDD](../../docs/components/simulation/cdd.md): operation table with inputs, outputs, lifecycle availability, and explicit open details; simulation lifecycle and state numbering.
- [SADD run lifecycle](../../docs/architecture/sadd.md#run-lifecycle): canonical setup/start, request boundaries, and membership rules.
- [SADD server coordination](../../docs/architecture/sadd.md#server-coordination-and-recovery): authority, submissions, client routing, pacing, and recovery.

These documents own requirements. The following is a navigation summary, not a replacement contract.

## Current decisions

- Actions target current state N; real execution produces N+1 before observations. Submissions contain agent ID, target state ID, and action. First valid submission is fixed; reject later submissions for that agent/state. Multi-agent batches use per-entry acceptance in supplied order and finish processing before advancement is considered.
- Readiness exposes current target state and missing agent IDs. Advancement returns resulting state ID, observations by agent, and request-associated membership results. Premature advancement changes no state or queues. No per-action execution outcomes. The server sends one observation batch per client per completed state using current ownership.
- Setup happens once at state 0. Clients connect and spawn; viewer data is available. The creator alone can Start after intended participants confirm successful spawning. Start sends initial observations and opens collection while preserving pacing. Reject Start when there are neither agents nor connected viewers; viewer-only starts are allowed. No setup removal is required.
- After all agent actions finish, capture a fixed batch of waiting requests; handle it before removals, spawns, state increment, and observations. Later arrivals wait until boundary completion. Outside execution/boundary completion, requests are handled promptly under operation-specific rules, even while paused. Populated-run membership changes remain queued. Empty-run admission requires no execution, counter increment, or repeated Start.
- Removals precede spawns; each queue follows server acceptance order. Placement is checked as applied, so later requests for a cell occupied by an earlier spawn fail. Repeated removals follow the response rule described above. Agent-action execution remains randomized.
- Viewer state is separate and initially complete: state ID, grid dimensions, agent IDs and positions. Future large environments require pan/zoom; delivery optimizations remain open. Viewing defaults to a tunable 500 ms minimum interval between step starts, with pause/single-step and no display acknowledgements. The default needs evaluation with a real viewer.

## Follow-up and open details

Python client design must cover coordinating multiple training clients: assembling participants, confirming spawning and preparation, and explicitly starting their shared run. No mandatory super-client or automatic expected-count/slot policy was selected.

Other open details include grid configuration and reproducibility, concrete API/error/request-correlation representations, creator-authority recovery, first pacing interval timing, Bevy/view delivery integration, readiness query behavior outside collection, and operation-specific recovery/suppression timing. Future environment-only animation is a use case, but its execution semantics remain undefined. Existing reconnection handover rules still apply; the general request boundary does not silently replace them.

## Repository checkpoint

Verified branch: `docs/simulation-cdd`. The prior checkpoint is `5d3bdf6` (`Document simulation lifecycle and CDD discussion checkpoint`). The maintainer subsequently authorized committing and pushing this session's SADD, simulation CDD, and handoff changes. This record is prepared before that commit; verify the resulting commit and push from Git history and remote state on resume. Git status warned that the global ignore file was inaccessible but listed only these three modified files.

## Verification

`git diff --check` passed. A PowerShell check verified all 23 relative Markdown file targets across the SADD, simulation CDD, and this handoff; anchors were not checked. Documentation-only work; no application builds or tests apply. No source packages were introduced.
