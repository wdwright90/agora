# Plan

The M2 chunks, in dependency order. Each is one or two PRs, agreed before implementation. Chunks 2 to 6 build on each other; chunks 9 and 10 can start once chunk 6 is done. Estimate: 14 to 18 PRs over 3 to 5 weeks, with chunks 9 to 11 the main uncertainty.

| # | Chunk | Contents | Status |
| --- | --- | --- | --- |
| 1 | Run lifecycle | `leave_run`, `close_run`, `run_closed` and `agent_removed {reason}` pushes, sequential sessions per connection | planned |
| 2 | Registry and appearance | registered kinds (terrain, items, creatures) with appearance as the single source of truth; `Reflect` on simulation components (ADR-002) | planned |
| 3 | Environment definitions | layout and ecology rules as data, in a data-only environment package; catalog entries built from definitions | planned |
| 4 | Metabolism and food | energy decay and move cost, diets, food items, automatic eating, starvation with removal, the food respawn rule | planned |
| 5 | Sight | line of sight, the three-slot structured grid, edges as walls, radius as a sight stat | planned |
| 6 | Capability definitions | sight, energy, and move declared with their output layouts, published by the server | planned |
| 7 | Rust client flattener | named arrays per sense; the reference implementation for the shared fixtures | planned |
| 8 | Viewer | walls, and food and agents drawn from their appearance; an agent's energy in the panel | planned |
| 9 | Python client library | asyncio client, protocol types, and a flattener matching the Rust one on the shared fixtures | planned |
| 10 | Gymnasium environment and trainer | the environment wrapper, SB3 PPO over parallel runs, checkpoints, and a play mode for the viewer | planned |
| 11 | Environments and acceptance | open field, sparse field, and rooms tuned; training beats random; the watch-it demonstration | planned |
