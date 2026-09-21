# Handoff

The maintainer and assistant discussed step execution, validation, reversible fallback control, reconnection, live effects, perception, MVP movement, spawning/removal, observer pacing, and inactivity. Decisions are recorded in the draft [simulation CDD](../../docs/components/simulation/cdd.md) and [SADD](../../docs/architecture/sadd.md#server-coordination-and-recovery).

Latest agreement: apply queued explicit removals before queued spawns, after execution and before observations. New agents first act in the next step. No implementation exists; the CDD and package mapping remain draft.

Next session: define the simulation's interface with the server, starting with what constitutes a submitted action, how step readiness is represented, and what advancing one step returns. Establish this component boundary before choosing concrete Bevy internals. Review the captured CDD as needed; grid setup remains open. Future rich senses and composable actions must not expand the MVP inadvertently.

The maintainer requested a branch and commit checkpoint before stopping for the day. Resume AGORA-002 from this handoff; no implementation or design-status acceptance was requested. Verify the resulting commit and branch from Git history rather than treating this pre-commit record as proof of completion.

Verification: git diff --check passed; relative Markdown file targets passed for all six affected documents (anchors not checked). Python was unavailable, so the link check used PowerShell. No application builds or tests apply to this documentation-only work.
