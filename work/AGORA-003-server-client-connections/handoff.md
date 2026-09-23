# Handoff

Checkpoint: 2026-09-22. The maintainer authorized server/client connection management and established the run/session/connection hierarchy.

## Resume here

The maintainer stopped for the day after agreeing to session-owned agents and cleanup on session expiry. **The next unanswered proposal is server-driven expiry cleanup at the next safe boundary without advancing the simulation, finishing an executing step first.** This would allow cleanup during setup and pause, as an exception to ordinary client-requested removal timing. It has not been accepted. Resume with this proposal, not response-field enumeration.

The agreed hierarchy is server → independent parallel simulation runs → run-scoped logical sessions → at most one active connection per session. Multiple independent clients can join the same run through separate sessions. Sessions survive connection loss within the previously agreed recovery window; a new connection can recover the session and replace its former connection. Agent recovery still follows the existing SADD step-boundary rule. Canonical rules are in the SADD's runs/sessions/connections section.

Parallel runs are now a required capability rather than a stretch goal; their exclusion from first-milestone acceptance remains unchanged.

Create/join/resume is now agreed: create selects a catalog entry and creates a run plus its first session with creator authority; join creates a session in an existing run without spawning or creator authority; resume attaches a replacement connection to an existing session. Clients can then spawn agents or subscribe as viewers. Initial local joining requires only the run ID; join credentials/access restrictions are deferred.

The latest decision replaces per-agent tokens with session ownership. Retain session-recovery credentials, while server-access permissions remain deferred. Resuming a session restores its owned agents under existing handover rules; cross-session agent transfer is outside MVP scope. On session expiry, clean up all owned agents. Spawn responses no longer contain agent tokens; earlier per-agent recovery response shapes need aggregation for session recovery.

Next, settle expiry cleanup for pending spawns, setup, and paused runs. Previously admitted requests continue after expiry, but agents they create must also be cleaned up. Normal started-run removal boundaries remain in force; no new immediate-removal exception has been accepted. Bootstrap correlation and creator-authority expiry also remain open.

Then discuss authority, joining/recovery, and delivery/failure handling at the feature level. Avoid exhaustive response-field questions. The preceding Start-response proposal was not accepted.

## Current state

The [SADD](../../docs/architecture/sadd.md) owns existing session identities, request correlation/retries, five-minute defaults, agent recovery, pacing, and lifecycle decisions. The [simulation CDD](../../docs/components/simulation/cdd.md) remains draft. AGORA-002 remains active for outstanding simulation design; this task does not mark it complete.

This session has existing uncommitted documentation changes from AGORA-002; preserve them. No implementation packages were introduced.

Verified checkpoint: branch `docs/simulation-cdd`, HEAD `be17bc1` (`Document simulation server interface and coordination decisions`). Modified tracked files are the SADD, simulation CDD, AGORA-002 brief/handoff, and work index. The maintainer subsequently authorized committing and pushing this checkpoint, including the new AGORA-003 directory. This record is prepared before that commit; verify its resulting hash and push state from Git on resume. Live remote state had not been checked at handoff preparation. Git warned that the global ignore file was inaccessible.

No server CDD or detailed shared contracts have been created yet. The SADD and simulation CDD remain drafts, not accepted implementations. AGORA-002 contains the earlier environment, interface, and retry decisions; its token-bearing historical summaries are explicitly superseded by this task's session-ownership decision and current design documents.

## Remaining work

- Resolve expiry cleanup during setup/pause and for spawns admitted before expiry, reconciling the existing rule that admitted operations continue.
- Define creator-authority recovery/expiry and viewer pacing-control claims.
- Define bootstrap correlation for create/join before a run-scoped session exists, session recovery synchronization, and credential lifecycle.
- Define delivery/failure handling for agent observations and command responses, connection-health detection, deadlines, and run cleanup.
- Develop the server CDD and simulation integration after behavior is sufficiently settled. Fill concrete response fields as encountered; do not restart exhaustive enumeration.

## Verification

`git diff --check` passed for the documentation changes. Relative Markdown file targets in the seven changed/new Markdown files were checked; anchors were not checked. No application builds or tests were run.
