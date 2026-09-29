---
id: SPEC-005
status: draft
owner: maintainer
parents: [CDD-004]
packages: [agora-viewer]
---

# Viewer

## Purpose and scope

This spec defines the observable behavior of the [viewer](../cdd.md): startup, commands, drawing, motion, controls, and the panel. Server behavior belongs to [SPEC-003](../../server/specs/sessions-and-runs.md), messages to [SPEC-002](../../../contracts/client-protocol.md), and the client library to [SPEC-004](../../client/specs/client-library.md).

## Terminology and preconditions

- **Creator:** a viewer started with `--create`, which holds the run's creator authority.
- **Controller:** a viewer holding pacing control.

## Requirements

### Startup and commands

- **SPEC-005-R01:** `agora-viewer` takes a server URL and exactly one of `--create`, which creates a run on `empty-grid-10x10`, or `--join <RUN_ID>`, which joins an existing run. It then watches the run and tries to claim pacing control; if another viewer holds control, it keeps watching without it. If the server cannot be reached, or the connection closes, the panel shows the viewer as disconnected with the reason.
- **SPEC-005-R02:** Start, claiming control, setting the pacing mode, granting a step, and closing the run are sent to the server as the user chooses them. A rejected command is shown in the panel with the server's reason, and the viewer keeps running.

### Drawing

- **SPEC-005-R03:** Once a view arrives, the viewer draws the grid's cells centred in the world, with `(0, 0)` at the south-west and `y` growing north, and one shape per agent in the agent's cell, coloured by its ID and labelled with it. After each new view, agents that appeared are added, agents that moved are sent to their new cell, and agents no longer in the view are removed.
- **SPEC-005-R04:** A moving agent eases from its previous position to its new cell over the shorter of 300 ms and 80% of the step interval; over 300 ms while paused; and at once under unlimited pacing. The camera fits the whole grid in the window area beside the control panel.

### Controls and panel

- **SPEC-005-R05:** Controls are enabled as follows:
  - Start: the creator, in setup, with at least one agent.
  - Claim pacing control: offered while this viewer is not the controller.
  - Pause: the controller, while not paused. Resume and Step: the controller, while paused.
  - The interval slider (20 ms to 5 s) and Unlimited: the controller.
  - Close run: the creator, once watching.
- **SPEC-005-R07:** When the creator closes the run, the viewer shows that it closed the run. When the server closes the run for another reason, or for a joined viewer, the viewer shows the run as closed with the reason.
- **SPEC-005-R06:** The panel shows the run ID with a button to copy it, the state ID and phase, the agent count, the pacing mode, and whether this viewer controls pacing.

## Acceptance criteria and verification

Unit tests for R05 are in `rust/agora-viewer/src/controls.rs`. The other tests are in `rust/agora-viewer/tests/viewer.rs`; they run the network bridge, and a headless viewer app without a window, against an in-process server. What appears on screen is checked by hand.

| Requirement | Check and expected outcome | Test |
| --- | --- | --- |
| R01 | Creating watches the run as creator with control and the default interval; joining a run whose control is held watches without it; an unreachable server reports a disconnection. | `r01_*` |
| R02 | A joined viewer's Start is reported as failed with `not_creator`; the creator's Start starts the run. | `r02_*` |
| R03 | Two agents get entities at their cells; a move retargets an entity; an agent removed by session expiry loses its entity; cell centres are symmetric with `y` growing north. | `r03_*`, `r03_r04_*` |
| R04 | The motion target follows the move. Easing timing and camera fit are checked by hand. | `r03_r04_*`, manual |
| R05 | Start, claim, pause, resume, step, pacing, and close controls are enabled exactly as listed. | `r05_*` (unit) |
| R07 | A joined viewer's Close is rejected with `not_creator`; the creator's Close reports "you closed the run", and the joined viewer reports the run closed by its creator. | `r07_*` |
| R06 | Checked by hand. | manual |

### Manual check

Run the server, `agora-viewer --create`, and two `agora-demo join <RUN_ID>` clients with the run ID copied from the panel. Confirm the grid and both agents appear during setup, Start begins stepping at about two steps a second, agents glide between cells, Pause stops them, Step advances one state, Resume and the slider change the speed, and Unlimited runs as fast as the clients answer.
