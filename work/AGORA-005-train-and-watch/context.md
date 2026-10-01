# Context: decisions from M2 planning

The maintainer agreed these decisions on 2026-09-27, while planning the milestone. System-level rules are also in the [SADD](../../docs/architecture/sadd.md) and simulation design in [CDD-001](../../docs/components/simulation/cdd.md); this record keeps the reasoning and the details that will become spec requirements with their features. The old proof-of-concept (`agora-poc`) was used as evidence only: its `forage-v1` environment, with sight, satiation, and respawning food, trained with PPO to about 7.6 times shorter episodes.

## Scope

- Single-agent learning first; several agents are M3.
- Foraging with sight, hunger, and food, plus walls to make environments differ: an open field, a sparse field, and rooms.
- The long-term aim is a deep, immersive simulation, with feelings such as exertion, lack of sleep, and hunger kept distinct. M2 takes the first step only.

## Metabolism and food

- An agent's **metabolism** is a capability set at spawn, with defaults from the environment (revised on 2026-09-30: a built-in default, not the environment's; see [chunk 4 decisions](#chunk-4-decisions)). It holds an energy level from 0 to 1, a per-step decay, an extra cost for moving, and a **diet**.
- A **diet** maps food kinds to an efficiency: 0 means the agent cannot eat that kind, and a negative value could later mean poison. It can grow into several nutrients digested differently.
- **Food items** have a kind and a nutrition value, lie in cells, and do not block movement. A cell holds at most one item.
- **Eating is automatic** for now: an agent that ends its move on an item it can eat consumes it and gains nutrition × efficiency, capped at full. Items it cannot eat stay. An explicit `eat` action may come later.
- At energy 0 the agent **starves** and is removed, which ends its episode.
- The simulation keeps a per-agent record of what it ate, for inspection and recordings; it is not part of perception.
- Agents without a metabolism are unaffected by food, which is harmless scenery to them.

## Perception

- CDD-001's principle is refined: do not expose hidden simulation state, but translate directly where the simulated quantity and the agent's perception naturally match.
- The agent perceives its **energy at full precision**, so it can learn that moving costs more than waiting. A separate exertion or fatigue signal is later work (M4).
- Whether an agent perceives that it has just eaten, beyond its energy rising, is left open until diets make it matter.

## Appearance

- A **registry** declares every kind (terrain, items, and creatures) once, with its appearance. It is the single source of truth for sight, the viewer's drawing, and future camera sight, following ADR-002's ownership rule.
- **Hue** and **size** are numbers from 0 to 1, so new kinds are easy to add and mimics can be close but slightly off. **Shape** is a class. Once there are several agent types, they are told apart by classes.
- **Terrain** has a class (floor or wall, and more later) plus flags where relevant: whether it blocks movement and whether it blocks sight.
- Agents get a default appearance, and species later get their own, so agents stay visually distinct.
- Brightness, patterns, and other features wait until an environment needs them.

## Sight

- A **square window** centred on the agent with north up, as a stepping stone to a **facing cone** in M4.
- **Range** is a stat on the agent's sight capability (radius, for example 5 for 11 × 11 cells). More stats come later.
- **Line of sight** from the start: walls block sight, so walls do not feel broken.
- Each visible cell has three **slots**: terrain (exactly one), item (at most one), and creature (at most one). The agent sees itself in the centre cell. A hidden cell is **unseen**, which hides everything about it. Memory is the client's job.
- Cells beyond the grid's edge are treated as **walls**, so the edge looks and behaves like a wall and anything past it is unseen. Environments with wrapping edges would differ; that is a later, per-environment rule.

### On the wire

The observation carries a structured grid, readable and language-neutral, in a fixed cell order declared by the sight capability (rows north to south, each west to east, agent at the centre). A sketch, to be specified with its feature:

```json
{
  "sight": {
    "radius": 5,
    "cells": [
      { "terrain": "wall" },
      "unseen",
      { "terrain": "floor", "item": { "hue": 0.02, "shape": "round", "size": 0.3 } },
      { "terrain": "floor", "creature": { "hue": 0.6, "shape": "agent", "size": 0.5 } }
    ]
  },
  "energy": 0.73
}
```

Missing slots mean nothing is there, and numbers are sent at full precision.

### The flattener

Both client libraries provide a generic flattener, driven by the capability definitions and checked against shared fixtures so Rust and Python produce identical arrays. It returns **named arrays per sense** (for example `sight` as layers × 11 × 11, and `energy` as one value), with an optional helper that flattens everything into one vector. Sight layers:

| Part | Encoding |
| --- | --- |
| unseen | one layer, 1 where unseen; other layers are 0 there |
| terrain class | one layer per class |
| terrain flags | one layer each for blocks movement and blocks sight |
| item or creature present | one layer each |
| hue | two layers per slot: the cosine and sine of the hue angle, because hue wraps around |
| size | one layer per slot |
| shape class | one layer per class, per slot |

Adding a shape or terrain class adds layers, which is a capability version change; a new kind using existing classes changes nothing.

### Recorded for later

- **Polar rays** (fixed angles, reporting the nearest hit and its distance, as in Unity ML-Agents): compact and range-independent, and a natural fit for a facing cone, but they miss what lies behind the first hit or between rays, and do not carry terrain area. Revisit with facing in M4, most likely first as a flattener option, since the structured grid already carries what rays need.
- **Camera sight**: rendering the agent's view as an image from the shared appearance registry, first as CPU sprite compositing in 2D and eventually from a 3D world. It gives real visual learning but costs far more training and rendering, and needs a binary transport encoding. Kept as a far-future sense, separate from the simulation core per ADR-001. A rasterized image of the grid features is a cheaper middle step.
- A compact binary encoding for observations, if their size ever matters.

## Environments and ecology

- (Refined on 2026-09-30 into separate layout and ecology files; see [chunk 4 decisions](#chunk-4-decisions).) An environment is a **layout** (the grid and its walls) plus **ecology rules** (for example, keep 12 berries on the map, respawning at random free cells). A catalog entry names a combination, so the same layout can appear with and without food. Composing rule sets freely is M7.
- The **simulation manages** ecology through rules: Rust systems configured by the environment definition, using the run's seeded random streams. New kinds of rule need Rust code; configuring existing rules is data.
- A privileged **director and editing API** (for curricula, experiments, and viewer editing) is the long-term plan, in M7.
- Environment definitions move into their own data-only package, as discussed on PR #5: its trigger, a real definition format, is met by this milestone.

## Episodes and run lifecycle

- **A new run per episode**, for isolation and simple lifecycle rules.
- **`leave_run`**: a session ends at once, its agents are removed, and a run with no sessions left is released immediately, so finished runs do not linger.
- **`close_run`**: the creator closes the run for everyone. The others receive a `run_closed {reason}` push, and the run is released. The same push covers a setup run whose creator expires.
- **`agent_removed {agent_id, reason}`** tells a client its agent was removed, for example because it starved.
- A connection may hold **several sessions in sequence** (at most one at a time), so a trainer can reuse one connection across episodes.
- The step limit that truncates an episode belongs to the trainer.

## Python

- The Python client and trainer live in **`python/` in this repository**, for easy access and as a ready-made trainer for people who do not want to customize. Custom clients can still be separate projects built on the shared contracts.
- An `asyncio` + `websockets` client library, a **Gymnasium** environment wrapper, and **Stable-Baselines3 PPO** (PyTorch, with its multi-input policy for named arrays), packaged with **uv**. A custom trainer only if SB3 falls short.
- Training runs several environments in parallel as separate runs. Watching a training run would slow it to the viewer's interval, so the trainer's "play" mode runs a saved policy in its own run for the viewer to join.
- Reward is computed by the trainer from observations; the simulation reports what happened and does not score it.

## Chunk 4 decisions

Agreed with the maintainer on 2026-09-30, while scoping chunk 4.

**Delivery.** Chunk 4 is four PRs: step stages, modular definitions, metabolism, and food ([plan](plan.md)).

**Step stages** ([ADR-003](../../docs/decisions/0003-simulation-step-stages.md), [CDD-001](../../docs/components/simulation/cdd.md#step-stages)):

- A step is a fixed sequence of stages, and every new system is placed in one: Actions, Interactions, Metabolism, Removal, Ecology, Membership, Commit, then Perception. The order must stay easy to read and adjust, so it is set in one list, and a test compares it with the CDD's table.
- Ordering across stages matters more than within one; conflicting systems within a stage must declare their order.
- One thread per run for now. The maintainer will often train a single agent over sequential runs, where parallelism across runs does not help, so multithreading inside a run should stay possible; the ambiguity checks and per-system random streams keep it a configuration change. To revisit once a step does enough work to measure.
- Energy does not decay while paused: a pause runs no stages.
- The word *stage* was chosen during implementation because CDD-001 already calls setup, collection, and execution *phases*.

**Modular definitions:**

- Layouts and ecologies are separate files, each with its own IDs. An environment file names the layout and the optional ecology it combines. The maintainer wants this flexibility early, and expects more axes of control later, so the format stays modular.
- An **ecology sets the properties of non-agent things**, such as a berry's nutrition, as well as its rules, so food can be tuned across training environments easily. A consequence: the same kind can be worth different amounts in different environments, and two differently nourishing foods in one environment need two kinds.
- **The agent kind leaves the environment**, which the SADD's extension boundaries already required: environment definitions do not declare agent types. Every agent gets the built-in `agent` kind until species arrive (M3).

**Metabolism and exertion:**

- Metabolism is an agent property. Until capability declarations (chunk 6), every agent gets a built-in default metabolism.
- Actions add the effort they spend to an **exertion** component, and each action owns its cost: a move adds effort per cell moved, and a failed move adds none. The Metabolism stage converts exertion to energy with the metabolism's rate (1 for now), applies decay, clears exertion, and then checks for starvation, after every energy change in the step. Metabolism therefore never needs to know which actions exist, and a later fatigue sense (M4) can read the same exertion.

**Direction, not yet designed:**

- Agents will eventually take several actions per step, such as moving and talking, with some actions exclusive. The candidate model is **action channels**: at most one action per channel per step, and actions in different channels combine. Exclusivity is hard to pin down with few actions, so it is a direction to design towards, not a requirement.
- Eating may become an **explicit action**, so an agent can choose whether to eat. It stays automatic for now to make training easier, possibly kept later as an option.

## Settled with features

- The exact energy numbers (decay, move effort, nutrition) and food counts per environment, tuned for training.
- Capability definition format and how the server publishes it.
- Whether `leave_run` keeps pending requests, and the exact closure reasons.
- Whether energy decay continues while paused: no, settled with the step stages (a pause runs no stages).
