---
id: ADR-002
status: accepted
owner: maintainer
date: 2026-09-27
---

# Simulation inspection and debug visualization

## Context

The [SADD](../architecture/sadd.md) keeps the simulation authoritative and separate from visualization: the viewer is a client that sees only what the server sends, and it never runs the simulation ([ADR-001](0001-rust-package-boundaries.md)). That split enables remote viewing, headless training, playback without the simulation, and a viewer that cannot disturb a run.

It also means that, as built for the first milestone, a viewer sees only a small stable view: agent IDs and positions ([SPEC-002-R12](../contracts/client-protocol.md#viewing)). The simulation is expected to become the largest and most complicated part of Agora, with senses, properties, machines, and environment objects. Two problems follow if nothing else is designed:

- **Drift.** Every feature someone wants to see needs a protocol view field, a server mapping, and a viewer renderer, kept in agreement by hand.
- **Shallow debugging.** A rendering of outcomes cannot explain behavior. Answering "why did this sense not detect that food?" needs what the sense actually sampled, which never leaves the simulation.

The maintainer raised this before the first viewer was built, to settle how debugging and visualization will stay in sync with the simulation.

## Options considered

- **Hand-written view types for everything.** Extend the stable view for each feature and render each one in the viewer. Simple, but it has both problems above, and it grows the language-neutral protocol with debugging detail.
- **Run the simulation inside the viewer** to use Bevy's own tools directly. Contradicts ADR-001 and the SADD: remote viewing, recordings, and a viewer independent of execution would all be lost.
- **Layered viewer data with ownership beside the simulation code.** Keep the stable view small, and add an opt-in debug channel whose content is produced by the simulation's own types and systems, so it cannot drift from them.
- **Share the simulation's component types with the viewer now,** through a separate data-only crate. Gives compile-time agreement and typed inspection, but ties the viewer to simulation internals and adds a crate before there is anything to share (see Decision, below).

## Decision

Adopt layered viewer data, with an ownership rule and a planned, triggered extraction of shared types.

**Three layers.**

1. **Stable view.** World state for normal display, defined in the language-neutral client protocol. It stays small and deliberate, and it is what recordings and non-Rust viewers rely on.
2. **Debug channel.** Opt-in, per viewer and per selected entity, carried in a separate message family from the stable view. It is not a stable contract: it is versioned with the build, and the server and viewer must come from the same code version, as the SADD already accepts for recordings. It carries:
   - **Reflected component dumps.** Simulation components derive Bevy's `Reflect`, and the server serializes the selected entity's actual components as a generic name-and-field tree, which the viewer shows in a generic inspector.
   - **Debug drawing emitted by systems.** A system records simple primitives (cells, lines, circles, labels) for what it actually did, such as the cells a sense sampled and what it detected. The viewer draws the primitives generically. The geometry comes from the same code path that computed the result.
   - **Mirrored agent observations.** A viewer can see the observation an agent actually received, next to the world state.
3. **Ownership.** Each simulation component or system owns, in the same module as its implementation, its stable view export (if any), its reflection, and its debug drawing. Tests check this, for example that every registered component is reflectable. The viewer stays generic for debugging and has custom rendering only for the stable view. This follows the SADD's rule that capability definitions live beside their Rust implementations.

**Shared types later, on a trigger.** The simulation's component data types are not shared with the viewer yet. When the first component is complex enough to want a typed inspector, or the generic reflected tree proves too weak to debug a sense, extract a data-only crate (component data types, `Reflect` derives, and debug-drawing primitives; no systems). The viewer may depend on it **for the debug layer only**, to decode dumps into real types and to use typed or off-the-shelf Bevy inspectors on a mirrored world. It must never shape the stable view or recordings, and the viewer must use those types for display only and never run simulation systems. This mirrors the environment-definition package expected in the [package mapping](../architecture/sadd.md#rust-package-mapping); the two may end up side by side.

## Consequences

- Adding a sense or other behavior means writing its debug output next to it; the viewer shows it without changes. Debug data cannot silently drift, because it is produced from the real types and code paths.
- Deep inspection is available without growing the language-neutral protocol, and non-Rust clients are unaffected.
- The debug channel requires same-build server and viewer. A mismatch degrades to a generic tree until typed decoding exists, and then fails to decode.
- Debug output has a runtime cost, so it is produced only for entities a viewer has selected.
- Now, before the debug channel is built:
  - simulation components derive `Reflect` from now on; the two existing ones, `Agent` and `Position`, gain it the next time the simulation changes;
  - the viewer's rendering is data-driven (entities with a kind and a position), not written around specific simulation types;
  - the stable view is not extended with debugging detail.
- Follow-up design: the debug message family, entity selection, the primitive set, reflection serialization, and how observation mirroring is requested. These are specified with the feature that builds them.

## Acceptance and supersession

The maintainer accepted the three layers, the ownership rule, and deferring the shared-types crate until one of the triggers is met on 2026-09-27, during planning of the first viewer (AGORA-004, PR 6). The SADD's [inspection and debug visualization](../architecture/sadd.md#inspection-and-debug-visualization) section summarizes the current design.
