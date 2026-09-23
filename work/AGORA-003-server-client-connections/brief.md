# AGORA-003 — Server and client connection management

Status: active
Owner: maintainer

## Goal

Develop server/client connection and run-management design through discussion, building on the simulation interface.

## Scope and exclusions

Discuss logical sessions and physical connections, agent/creator/viewer authority, joining and recovery, delivery failures, and fallback/cleanup. Keep cross-component rules canonical in the SADD. Defer exhaustive response-field enumeration; use the established patterns as details arise. No implementation or automatic acceptance of draft designs.

## Acceptance criteria

- Record agreed connection, authority, joining, recovery, and delivery behavior.
- Identify unresolved MVP decisions and distinguish later work.
- Capture server component responsibilities in a draft CDD when sufficiently developed, linked to canonical system rules and shared contracts.

## Dependencies and open questions

- [SADD](../../docs/architecture/sadd.md): existing session, correlation, lifecycle, and recovery rules.
- [Simulation CDD](../../docs/components/simulation/cdd.md): execution and request boundaries.
- [AGORA-002 handoff](../AGORA-002-simulation-cdd/handoff.md): prior discussion checkpoint; simulation internals remain unfinished.
