---
id: ADR-001
status: accepted
owner: maintainer
date: 2026-09-19
---

# Rust package boundaries

## Context

The [SADD](../architecture/sadd.md) requires shared runs across clients, optional visualization, language-neutral interfaces, and later recording and environment tooling. The first milestone is deliberately small. [POC findings](../../work/AGORA-001-rust-package-design/context.md) provide historical evidence, not successor requirements.

## Options considered

- One Rust application with modules: simple to start, but couples client and viewer reuse to server implementation and dependencies.
- Reuse the POC split unchanged: familiar, but leaves recording within simulation and carries a one-environment-per-connection model that does not meet current requirements.
- Separate packages at application and data boundaries, keeping smaller features in modules: allows independent clients and visualization while limiting initial package count.

## Decision

Adopt the third option with the [package responsibilities and dependency direction in the SADD](../architecture/sadd.md#rust-package-mapping). Start with protocol, simulation, server, client, and viewer packages; introduce recording when that feature begins.

Keep the simulation authoritative and independent of rendering and network transport. The viewer uses live state supplied by the server or recorded state, rather than executing the simulation itself. Launched agents remain ordinary clients with their own inference runtime, following the maintainer's agreed starting approach.

Use configuration and Rust extension points for environments and senses. Avoid a separate package for every sense, machine, generator, or lifecycle concern before there is a concrete reuse need.

## Consequences

The client and viewer can run on different machines from the server without exposing Bevy internals. Python models can remain in Python clients even when launched by a Rust viewer. Live view state must be distinguishable from agent-specific observations.

The first milestone needs a minimal live viewing path rather than the POC's recording-file follow mode. Protocol representations must remain language-neutral; transport, capability schema, environment format, recording format, and process supervision still need detailed design. Recording will have its own lifecycle and compatibility needs without requiring resumable execution checkpoints.

## Acceptance and supersession

The maintainer accepted this package mapping on 2026-09-24, before the first package (`agora-sim`) was created. No foreseeable problems exist under the current design; boundaries can be revised by a later ADR if implementation shows the need. The earlier agreements on reusable profiles and local client launching, Rust-based extensions, and inspection-only playback remain separate requirements.

## Revisions

- 2026-09-24: For the MVP, `agora-sim` does not depend on `agora-protocol`. The server maps simulation types to protocol messages, which keeps wire concerns out of the simulation. Revisit when capability definitions need shared types. The SADD package mapping records the current dependencies.
