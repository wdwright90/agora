---
id: ADR-003
status: accepted
owner: maintainer
date: 2026-09-30
---

# Simulation step stages

## Context

The [simulation CDD](../components/simulation/cdd.md) left the schedule implementation open, and the [SADD's reproducibility rules](../architecture/sadd.md#execution-reproducibility) require identical results from identical seeds and inputs. Through M1, a step was a single function: shuffle the agents, execute their moves, increment the state ID, and build observations. The simulation used Bevy ECS to store the world but had no schedule.

The second milestone adds metabolism, food, eating, starvation, and ecology rules ([AGORA-005](../../work/AGORA-005-train-and-watch/context.md)), and later milestones add many more systems. Their relative order decides outcomes, for example whether an agent with almost no energy eats before it starves. That order needs to be consistent, written down once, easy to read, and easy to change as the simulation grows.

## Options considered

- **Keep a hand-written step function.** Simple for a few systems, but the order is implicit in code, every new system edits the same function, and nothing checks that two systems touching the same data run in a deliberate order.
- **A Bevy schedule with explicit ordering between individual systems.** Expresses dependencies precisely, but the overall order is spread across many `before` and `after` declarations and is hard to read as a whole.
- **A Bevy schedule divided into ordered stages,** each a system set, with each system placed in one stage. The overall order is one short list; ordering inside a stage is needed only where systems conflict.

## Decision

A step runs as a fixed sequence of **stages**: Actions, Interactions, Metabolism, Removal, Ecology, Membership, and Commit, followed by Perception. CDD-001's [stage table](../components/simulation/cdd.md#step-stages) describes each stage. The word *stage* avoids confusion with the run's phases (setup, collection, and execution); all stages run within execution.

- **Every system belongs to one stage,** and each stage finishes before the next begins. The order is set in one place in `agora-sim`, and a test checks that it matches the CDD's table, so the code and the documentation cannot drift apart.
- **Earlier stages record facts and later stages act on them.** For example, actions add the effort they spend to an agent's exertion, and the Metabolism stage turns exertion into an energy cost without knowing which actions exist. Each action owns its cost.
- **Ordering across stages matters more than ordering within one.** Within a stage, systems that touch the same data must declare their order: Bevy's ambiguity detection is set to fail the schedule build, so an undeclared conflict is found when the simulation is built, including in tests.
- **Only the Actions stage is sequential per agent,** in the step's shuffled order with effects applied immediately (CDD-001 steps 2 and 3). Later stages apply to the whole world once.
- **Perception is its own schedule,** run after each step and whenever observations are needed without a step, such as at Start. A paused run runs no stages.
- **Each system that draws random numbers uses its own seeded stream,** so adding a system does not change another's draws.
- **A step runs on one thread for now.** Because conflicting systems must be ordered and randomness is per system, results do not depend on how systems are scheduled onto threads, so running a step's systems on several threads later is a configuration change rather than a redesign.

## Consequences

- Adding a system means choosing its stage. Adding or reordering a stage means changing one list in code and one table in the CDD, and recording the reason when the change is significant.
- Specs describe behavior by stage, for example "in the Metabolism stage", rather than by position in code.
- Stages are declared before they have systems, so the full intended order is visible from the start.
- Parallel execution inside a run is deferred. It matters most for a single agent trained over sequential runs, where parallelism across runs does not help; it is worth measuring once a step does enough work to benefit.
- Multiple actions per step and explicit eating (CDD-001's direction for actions) fit this structure: further actions run in the Actions stage or a later stage, and their costs flow through exertion.

## Acceptance and supersession

The maintainer agreed the stages, their order, the exertion approach, and starting on one thread on 2026-09-30, while scoping AGORA-005 chunk 4. Implemented in `agora-sim` and specified in [SPEC-001-R21](../components/simulation/specs/lifecycle-and-movement.md#advancement).
