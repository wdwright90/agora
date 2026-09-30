---
id: SPEC-006
status: draft
owner: maintainer
parents: [CDD-001]
packages: [agora-env, agora-sim]
---

# Kinds and appearance

## Purpose and scope

This spec defines the kind registry described in the [simulation CDD](../cdd.md#second-milestone-design): every kind of terrain, item, and creature in a run, declared once with its appearance. The registry is the single source of truth for how things look. The simulation reads it, environment definitions refer to it ([SPEC-007](environment-definitions.md)), and the server sends it to viewers ([SPEC-002-R12](../../../contracts/client-protocol.md#viewing)).

It covers the registry's contents and rules, and the built-in kinds used by the bundled catalog. It is extended feature by feature.

Not yet covered, and not implemented:

- items in cells (food)
- sight, which reports appearance rather than kind IDs, so a kind that looks like another cannot be told apart by name
- declaring kinds in environment definitions rather than code
- species with their own appearance

## Terminology and preconditions

- **Kind:** a named sort of thing, such as `wall`, `berry`, or `agent`. Every entity in a run has exactly one.
- **Category:** terrain (exactly one per cell), item (at most one per cell), or creature (including agents).
- **Appearance:** how an item or creature looks: hue, size, and shape.

## Requirements

### The registry

- **SPEC-006-R01:** A kind registry lists kinds in declaration order. Each kind has a non-empty kind ID, unique within the registry, and exactly one category. Building a registry with a duplicate ID fails, naming the ID. A kind can be found by ID and by its index in declaration order. Terrain refers to kinds by index ([SPEC-001-R20](lifecycle-and-movement.md#terrain), [SPEC-002-R12](../../../contracts/client-protocol.md#viewing)), so a run's registry never changes order; if kinds are ever added during a run, they are appended.
- **SPEC-006-R02:** A terrain kind has a class, `floor` or `wall`, and two flags: whether it blocks movement and whether it blocks sight. Terrain that blocks movement stops spawns and moves ([SPEC-001-R20](lifecycle-and-movement.md#terrain)). *(Interim.)* The sight flag has no effect yet; sight will use it.
- **SPEC-006-R03:** Item and creature kinds have an appearance:
  - **hue**, a number from 0 to 1 inclusive giving a position on the colour wheel, where 0 and 1 are the same hue;
  - **size**, a number from 0 to 1 inclusive;
  - **shape**, a class: `agent` or `round`.

  Hue and size outside 0 to 1, including NaN, are rejected when constructed. Adding a shape or terrain class is a change to this spec.

### Built-in kinds

- **SPEC-006-R04:** The built-in registry declares these kinds, in this order. `agent` is the kind every agent has.

  | Kind | Category | Description |
  | --- | --- | --- |
  | `floor` | terrain | class `floor`; blocks neither movement nor sight |
  | `wall` | terrain | class `wall`; blocks movement and sight |
  | `berry` | item | hue 0.02, size 0.3, shape `round` |
  | `agent` | creature | hue 0.6, size 0.5, shape `agent` |

## Interfaces and data

`agora-env` implements the registry as `KindRegistry`, built from `Kind` values (a `KindId` and a `Look`: terrain, item, or creature). `agora_env::builtin` provides the built-in registry and the agent kind's ID. `agora-env` holds no Bevy runtime types; its `reflect` feature derives `bevy_reflect::Reflect` on `KindId`, which a simulation component holds. The wire form is defined by [SPEC-002](../../../contracts/client-protocol.md#viewing). How the simulation uses the registry is in [SPEC-001](lifecycle-and-movement.md#creation-and-coordinates).

## Acceptance criteria and verification

Tests are in `rust/agora-env/tests/kinds_and_appearance.rs`. Each test name starts with the requirement ID it checks.

| Requirement | Check and expected outcome | Test |
| --- | --- | --- |
| R01 | Kinds keep declaration order and are found by ID and index; duplicate and empty IDs are rejected. | `r01_*` |
| R02, R04 | The built-in registry's contents, including the terrain flags. | `r04_*` |
| R03 | Hue and size accept 0, 1, and values between; values outside, NaN, and infinity are rejected. | `r03_*` |

## Open questions

- Whether kinds gain more appearance features, such as brightness or patterns, waits until an environment needs them.
- How environment definitions declare kinds waits until an environment needs a new kind ([SPEC-007](environment-definitions.md#open-questions)).
