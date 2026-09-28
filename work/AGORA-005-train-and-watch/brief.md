# AGORA-005 — Train and watch (M2)

Status: active
Owner: maintainer

## Goal

Deliver the [second milestone](../../docs/architecture/sadd.md#second-milestone-train-and-watch): a Python trainer teaches a neural network to forage in a few simple environments, and a person can watch an untrained and a trained agent in the viewer and see the difference.

## Scope and exclusions

Delivered as small PRs against `develop`, in the order in [plan.md](plan.md). Scope for each PR is agreed before implementation, following the [features and specs workflow](../../docs/workflows/ai-development.md#features-and-specs). The design decisions agreed while planning are in [context.md](context.md).

Out of scope, and planned in later milestones ([roadmap](../../docs/roadmap.md)):

- several agents in one run, species, and spawning after Start (M3)
- facing, sight cones, rays, and exertion (M4)
- the debug channel, and recording and playback (M5)
- deadlines, fallback control, session recovery, and authentication (M6)
- the director and editing API, live edits, and composing rule sets (M7)
- camera sight (far future)

## Acceptance criteria

- A training run shows measurable learning: for example, average survival time or food eaten per episode clearly improves over a random policy.
- A saved policy can be run in each bundled environment and watched in the viewer.
- At least three bundled environments of increasing difficulty: an open field, a sparse field, and rooms.
- Every PR's specs, tests, and documentation are updated together, and its checks pass.

## Dependencies and open questions

- Builds on the M1 components: [CDD-001](../../docs/components/simulation/cdd.md), [CDD-002](../../docs/components/server/cdd.md), [CDD-003](../../docs/components/client/cdd.md), and [CDD-004](../../docs/components/viewer/cdd.md), and on [ADR-002](../../docs/decisions/0002-inspection-and-debug-visualization.md).
- Open points to settle with their features are listed in [context.md](context.md#settled-with-features).
