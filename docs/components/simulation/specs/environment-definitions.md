---
id: SPEC-007
status: draft
owner: maintainer
parents: [CDD-001]
packages: [agora-env]
---

# Environment definitions

## Purpose and scope

This spec defines environments as data, as described in the [simulation CDD](../cdd.md#second-milestone-design): what a run is built from, the TOML format that definitions are written in, and the definitions bundled with Agora. The server's catalog is built from the bundled definitions ([SPEC-003-R01](../../server/specs/sessions-and-runs.md#catalog-and-identifiers)), and the simulation takes an environment when it is created ([SPEC-001-R01](lifecycle-and-movement.md#creation-and-coordinates)).

It is extended feature by feature. Not yet covered, and not implemented:

- ecology rules, such as keeping a number of food items on the map (AGORA-005 chunk 4)
- declaring kinds in a definition; definitions use the built-in kinds ([SPEC-006-R04](kinds-and-appearance.md#built-in-kinds))
- layouts generated per run from the run's seed, which would be configured by a definition rather than drawn in one
- loading definitions from files at run time rather than bundling them
- terrain that changes during a run

## Terminology and preconditions

- **Layout:** the terrain kind of every cell in a bounded grid.
- **Environment:** what a run is built from: a kind registry, a layout, and the kind given to every agent.
- **Definition:** an environment written as TOML. Its **map** draws the layout as text, and its **legend** says which kind each character stands for.
- **Bundled definition:** a definition compiled into Agora, named by its catalog entry ID.

## Requirements

### Layouts and environments

- **SPEC-007-R01:** A layout has a width, a height, and one kind ID per cell. Cells are in row-major order from the south-west corner: the cell at `(x, y)` is at index `y * width + x`, using the simulation's coordinates ([SPEC-001-R02](lifecycle-and-movement.md#creation-and-coordinates)). A zero width or height is rejected, as are dimensions whose cell count cannot be represented on the platform and a cell count that does not equal width × height.
- **SPEC-007-R02:** An environment is built from a kind registry, a layout, and an agent kind. Building it fails, naming the kind, when a layout kind is not in the registry or is not terrain, or when the agent kind is not in the registry or is not a creature. A built environment is therefore consistent, and the simulation accepts it without further checks.

### Definition format

- **SPEC-007-R03:** A definition is a TOML document with exactly these fields:

  | Field | Type | Meaning |
  | --- | --- | --- |
  | `agent_kind` | string | The kind given to every agent. |
  | `layout.map` | string | The layout, drawn as text ([R05](#definition-format)). |
  | `layout.legend` | table of strings | Each key is a character in the map, and its value is a kind ID ([R04](#definition-format)). |

  Invalid TOML, a missing field, an unknown field, and a field of the wrong type are rejected. So is an empty kind ID. Loading takes a kind registry, and the loaded environment is checked as in R02. An example:

  ```toml
  agent_kind = "agent"

  [layout]
  map = """
  ..#
  ...
  """

  [layout.legend]
  "." = "floor"
  "#" = "wall"
  ```

- **SPEC-007-R04:** Each legend key is exactly one Unicode character, of any script, so a legend is not limited to ASCII. Each value must name a terrain kind in the registry. Every entry is checked, whether or not the map uses it. A key of any other length is rejected, naming the key; an unknown kind or a kind that is not terrain is rejected, naming the character and the kind.
- **SPEC-007-R05:** The map is read line by line. Lines may end with LF or CRLF, and the final line ending is optional; TOML also drops a line ending directly after the opening `"""`. The first line is the northern row and the last is the southern row, and each character is one cell, west to east. So the map's width is its line length and its height is its number of lines. A map with no lines is rejected. A line whose length differs from the first is rejected, naming the line and both lengths. A character not in the legend is rejected, naming it with its line and column. Lines and columns are counted from 1 at the top left.

### Bundled definitions

- **SPEC-007-R06:** Agora bundles these definitions, in this order, compiled into the program. Each is a file in `rust/agora-env/environments/` named after its catalog entry ID, every file there is bundled, and each loads with the built-in kinds and the `agent` agent kind.

  | Catalog entry | Size | Layout |
  | --- | --- | --- |
  | `empty-grid-10x10` | 10 × 10 | all floor |
  | `divided-10x10` | 10 × 10 | floor, with a wall along x = 5 except at y = 4 and y = 5 |

## Interfaces and data

`agora-env` implements this spec. `Layout` holds a grid of `KindId`s. `Environment::new` builds a checked environment, and `Environment::from_toml` loads one from a definition. `agora_env::bundled::DEFINITIONS` lists the bundled definitions' IDs and sources. Errors are `LayoutError`, `EnvironmentError`, and `DefinitionError`. Code can build a `Layout` directly, so the map is only the file form.

## Acceptance criteria and verification

Tests are in `rust/agora-env/tests/environment_definitions.rs`. Each test name starts with the requirement IDs it checks.

| Requirement | Check and expected outcome | Test |
| --- | --- | --- |
| R01 | Cells are found at `y * width + x`; zero dimensions and a wrong cell count are rejected. | `r01_*` |
| R02 | Unknown and non-terrain layout kinds, and unknown and non-creature agent kinds, are rejected. | `r02_*` |
| R03, R05 | A definition loads with its first line north; invalid TOML, unknown, missing, and mistyped fields, and unknown and empty agent kinds are rejected. | `r03_*` |
| R04 | Non-ASCII keys work; empty and multi-character keys, and unknown and non-terrain kinds, are rejected, including unused entries. | `r04_*` |
| R05 | LF, CRLF, and an unterminated last line give the same layout; empty, ragged, and unknown-character maps are rejected with their positions. | `r05_*` |
| R06 | The bundled list, its files, and the two layouts. | `r06_*` |

## Open questions

- How a catalog entry combines a layout with ecology rules, so the same layout can appear with and without food, is settled with ecology rules (AGORA-005 chunk 4). A definition may then refer to a layout rather than drawing its own.
- Whether a map can also place items, for example with a second map or a list of positions, waits until an environment needs placed items.
- How definitions declare their own kinds, and how those combine with the built-in kinds, waits until an environment needs a new kind.
