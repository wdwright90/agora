---
id: SADD-001
status: draft
owner: maintainer
---

# Agora — SADD

This is an initial outline, not an accepted architecture. Use the [SADD template](../templates/sadd.md) as the system design develops.

## Purpose and scope

Agora is being redesigned from a proof of concept. The project is expected to include Rust and Python packages. Product goals, intended users, first milestone, and non-goals remain to be defined.

## System context

To be defined: external actors, integrations, deployment environments, and system boundaries.

## Components and responsibilities

No components are defined yet. Each component will have a [CDD](../components/README.md), including a mapping to implementing packages.

## Interactions and shared contracts

To be defined: component dependencies, communication paths, and canonical [shared contracts](../contracts/README.md).

## Cross-cutting concerns

To be defined: reproducibility, performance, compatibility, error handling, observability, and any security or deployment requirements relevant to the agreed scope.

## Decisions and open questions

- What are the primary use cases and the first usable milestone?
- Which capabilities from the POC should be retained?
- How will responsibilities be divided between Rust and Python?

Record consequential choices in [ADRs](../decisions/README.md). Track POC evaluation in the [migration inventory](../migration/inventory.md).
