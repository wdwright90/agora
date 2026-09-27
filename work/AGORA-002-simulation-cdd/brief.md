# AGORA-002 — Simulation CDD

Status: complete
Owner: maintainer

## Goal

Develop the simulation CDD through discussion, starting with a simulation step's lifecycle.

## Scope and exclusions

Capture agreed simulation behavior and affected system boundaries. Separate MVP requirements from future perception and action design. No implementation or automatic acceptance of the overall package mapping.

The maintainer has deferred exhaustive request/response field enumeration. Apply the established response, correlation, retry, and error patterns as missing details arise during subsequent design or implementation; record genuinely new behavior decisions separately.

## Acceptance criteria

- Draft component responsibilities, lifecycle, interfaces, and unresolved decisions.
- Keep cross-component decisions canonical in the SADD.
- Review remaining MVP choices before detailed specs or implementation.

Met, and closed on 2026-09-27. The discussion produced [CDD-001](../../docs/components/simulation/cdd.md) and the SADD's simulation rules, and [AGORA-004](../AGORA-004-first-milestone/brief.md) turned the MVP choices into SPEC-001 and `agora-sim`. CDD-001 stays a draft that evolves with feature work; its open questions carry the undecided points forward. The [handoff](handoff.md) is a historical record; where it differs from the SADD or CDD-001 (for example, per-agent control tokens, replaced by session ownership in AGORA-003), those documents are current.

## Dependencies and open questions

See [CDD-001](../../docs/components/simulation/cdd.md) and its open questions. Server and shared-contract design remain follow-up work.
