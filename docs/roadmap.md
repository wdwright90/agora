# Roadmap

The maintainer agreed this milestone order on 2026-09-27. Each milestone is a usable step, delivered as small PRs under its own task record in [work](../work/index.md). Detailed requirements are written with each feature, not ahead of it. The [SADD](architecture/sadd.md) owns what each milestone must achieve; this page orders them and records rough estimates, which are revised at each milestone boundary.

## M1 — First usable milestone (complete)

An empty 10 × 10 grid, two connected agents moving, and the viewer showing them. Delivered by [AGORA-004](../work/AGORA-004-first-milestone/brief.md) and accepted on 2026-09-27. See the SADD's [first usable milestone](architecture/sadd.md#first-usable-milestone).

## M2 — Train and watch (next)

A Python trainer teaches a neural network to forage in a few simple environments, and a person can watch an untrained and a trained agent in the viewer and see the difference. Agents have a metabolism (energy, a diet, and starvation), sight with line of sight, and eat food automatically; environments combine a layout with ecology rules. See the SADD's [second milestone](architecture/sadd.md#second-milestone-train-and-watch) and [AGORA-005](../work/AGORA-005-train-and-watch/brief.md) for the decisions and plan.

Estimate: 3 to 5 weeks, about 14 to 18 PRs. The platform and simulation work is fairly predictable (about 2 to 3 weeks); getting training to work well is the main uncertainty.

## M3 — Multiple agents

Several agents in one run: competition for food, species with different diets and appearances, spawning after Start, and shared or separate policies.

## M4 — Directional sight and richer bodies

Facing and a turn action, a sight cone, polar rays or a rasterized image as encoding options, and exertion separated from hunger as the first step toward distinct internal feelings.

## M5 — Inspection and recording

The debug channel from [ADR-002](decisions/0002-inspection-and-debug-visualization.md) (an inspector, a sight overlay, and mirrored observations), and recording and playback.

## M6 — Robustness and remote use

Deadlines and fallback control, session recovery, and authentication for remote training.

## M7 — Editing and composition

A director and editing API for privileged clients and the viewer, live edits, and composing layouts with rule sets to produce many varied learning environments.

## Far future

- **Camera sight:** a sense that renders the agent's view as an image from the shared appearance registry, first in 2D and eventually from a 3D world.
- A 3D world viewed from above.

Milestones after M2 are ordered but not estimated.
