# AGORA-003 — Server and client connection management

Status: complete
Owner: maintainer

## Goal

Develop server/client connection and run-management design through discussion, building on the simulation interface.

## Scope and exclusions

Discuss logical sessions and physical connections, agent/creator/viewer authority, joining and recovery, delivery failures, and fallback/cleanup. Keep cross-component rules canonical in the SADD. Defer exhaustive response-field enumeration; use the established patterns as details arise. Prioritize remaining structural choices that would be expensive to change: client communication, simulation extension boundaries, and reproducibility. Response variants and detailed states can be settled with their features. No implementation or automatic acceptance of draft designs.

## Acceptance criteria

- Record agreed connection, authority, joining, recovery, and delivery behavior.
- Identify unresolved MVP decisions and distinguish later work.
- Capture server component responsibilities in a draft CDD when sufficiently developed, linked to canonical system rules and shared contracts.

Met, and closed on 2026-09-27. The discussion produced the SADD's rules for runs, sessions, recovery, pacing, and delivery, and [AGORA-004](../AGORA-004-first-milestone/brief.md) built the MVP part as [CDD-002](../../docs/components/server/cdd.md), SPEC-002, SPEC-003, and `agora-server`. Session recovery, deadlines, and fallback control remain designed in the SADD but unbuilt, and are recorded in CDD-002's open questions and the AGORA-004 handoff. The [handoff](handoff.md) is a historical record; the SADD and CDDs are current.

## Dependencies and open questions

- [SADD](../../docs/architecture/sadd.md): existing session, correlation, lifecycle, and recovery rules.
- [Simulation CDD](../../docs/components/simulation/cdd.md): execution and request boundaries.
- [AGORA-002 handoff](../AGORA-002-simulation-cdd/handoff.md): prior discussion checkpoint; simulation internals remain unfinished.
