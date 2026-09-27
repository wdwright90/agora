---
id: CDD-004
status: draft
owner: maintainer
parent: SADD-001
packages: [agora-viewer]
---

# Viewer — Component Design Description

## Responsibility and boundaries

The viewer is the Bevy application people use to watch a run: it draws the run's world, starts the run when it created it, and controls pacing. It is an ordinary client of the server ([SADD](../../architecture/sadd.md#runs-sessions-and-connections)): it sees only what the server sends and never runs the simulation ([ADR-001](../../decisions/0001-rust-package-boundaries.md)). What it draws comes from the stable view in the [client protocol](../../contracts/client-protocol.md#viewing); deeper inspection follows [ADR-002](../../decisions/0002-inspection-and-debug-visualization.md) and is not built yet.

Out of scope for now: browsing and choosing runs, launching agent clients from profiles, recording playback, live editing, pan and zoom, and the ADR-002 debug channel.

## Package mapping

The `agora-viewer` package (`rust/agora-viewer`) implements this component as a library and the `agora-viewer` executable. It depends on `agora-client` (without its `demo` feature), `agora-protocol` through the client, Bevy 0.19 with the `2d` feature set, and `bevy_egui` 0.42. It does not depend on `agora-sim`. The library exists so the viewer's logic can be tested without a window.

## Internal structure

- **Network bridge (`network`):** a background thread runs the client library on its own Tokio runtime, so Bevy never waits on the network. It creates or joins the run, watches it, and tries to claim pacing control. UI commands (Start, claim, set pacing, step) reach it over a channel; results that need the user's attention (a failed command, a lost connection) come back over another. Once watching, it hands Bevy the view and pacing latest-value handles.
- **State (`state`):** a Bevy resource holding the connection status, whether this viewer created the run, the newest view and pacing state, and the latest failure message. A system drains the bridge's events and copies the latest values each frame.
- **Rendering (`render`):** spawns the grid once its size is known, keeps one entity per agent in step with each new view, eases movement between cells, and fits the camera to the grid beside the control panel. It works from the view's data (entities with positions), not from knowledge of simulation types, as ADR-002 requires.
- **Controls (`controls`) and panel (`ui`):** a pure function decides which controls are enabled; an `egui` window shows the status and the controls.

## Interfaces and dependencies

The executable takes `--server` (default `ws://127.0.0.1:7878`) and exactly one of `--create` or `--join <RUN_ID>`. One window shows one run. Choosing the run is kept out of the viewer so that a future run browser, which needs a way to list runs, can open viewer windows by launching `--join`; the maintainer chose this on 2026-09-27.

## Data flow and lifecycle

- **Startup:** the bridge connects, creates or joins, watches, and claims pacing control. A claim refused because another viewer holds control is ignored; the viewer still watches. Any other failure reports the viewer as disconnected.
- **Each frame:** the state system takes new events and latest values. When the view changed, the render system syncs the grid and agents: new agent IDs get an entity with a colour derived from the ID and an ID label, known agents are retargeted to their new cell, and agents no longer in the view are despawned. Motion eases each agent to its target.
- **Controls:** buttons send commands through the bridge. The server's answer shows up in the next view or pacing state; a rejection shows as a message in the panel.
- **Shutdown:** closing the window drops the bridge's command channel, and the network thread ends, closing the connection.

## Design constraints and rationale

- **Latest-value handles, read per frame.** A slow frame shows the newest state and never works through a backlog, matching the server's newest-only view delivery.
- **Complete views make syncing simple.** Every view lists every agent, so the render system compares entities with the view instead of applying changes.
- **Easing is presentation only.** A move animates over at most 300 ms and at most 80% of the step interval, so it finishes before the next step starts; while paused it takes 300 ms, and unlimited pacing snaps. The drawn position never feeds back into anything.
- **Start needs an agent.** The server allows a Start with only viewers, but such a run could never step, so the viewer enables Start only in setup with at least one agent, and only for the creator.
- **Claiming is offered whenever this viewer lacks control.** The protocol reports only whether this viewer holds control, not whether control is vacant, so the server decides and a refusal is shown as a message.
- **`egui` for the panel.** It is much quicker to build a tool panel with than `bevy_ui`. The panel is an anchored `egui` window, and the camera keeps the grid clear of it.

## Detailed specifications

- [SPEC-005 — Viewer](specs/viewer.md) (draft).
- [SPEC-002 — Client protocol](../../contracts/client-protocol.md) (draft) and [SPEC-004 — Rust client library](../client/specs/client-library.md) (draft), which the viewer uses.

## Open questions

- The ADR-002 debug channel: entity selection, the inspector, and debug drawing.
- Pan and zoom, and viewing large environments.
- Whether an agent's sprite should reflect more than its position once the stable view grows.
