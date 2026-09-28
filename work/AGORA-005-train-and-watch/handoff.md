# Handoff

Checkpoint: 2026-09-27. Planning is done: the maintainer agreed the milestone's scope, design decisions, and chunk order ([context.md](context.md), [plan.md](plan.md)), and the [roadmap](../../docs/roadmap.md) orders later milestones. The planning documents are in review as #16 on branch `docs/milestone-2-plan`. No implementation has started.

## Resume here

Once the planning PR has merged, agree the scope of chunk 1 (run lifecycle: `leave_run`, `close_run`, the `run_closed` and `agent_removed` pushes, and sequential sessions per connection) with the maintainer, then implement it on a branch off `develop`.

## State

- M1 is complete and in `main` ([AGORA-004](../AGORA-004-first-milestone/handoff.md) has its follow-ups, several of which are now scheduled in the roadmap).
- Design decisions are recorded in the SADD's [second milestone](../../docs/architecture/sadd.md#second-milestone-train-and-watch) section and CDD-001, with the reasoning and wire sketches in [context.md](context.md).
- `Agent` and `Position` in `agora-sim` still need `Reflect` (ADR-002); chunk 2 does it.
