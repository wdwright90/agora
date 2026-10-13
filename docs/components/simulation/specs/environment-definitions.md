---
id: SPEC-007
status: draft
owner: maintainer
parents: [CDD-001]
packages: [agora-env]
---

# Environment definitions

## Purpose and scope

This spec defines environments as data, as described in the [simulation CDD](../cdd.md#second-milestone-design): what a run is built from, the TOML files that environments are written in, and the files bundled with Agora. Definitions are modular. A layout file draws a layout, and an environment file names the layout it uses, so one layout can serve several environments. The server's catalog is built from the bundled environments ([SPEC-003-R01](../../server/specs/sessions-and-runs.md#catalog-and-identifiers)), and the simulation takes an environment when it is created ([SPEC-001-R01](lifecycle-and-movement.md#creation-and-coordinates)).

It is extended feature by feature. Not yet covered, and not implemented:

- ecology files, which set the properties of non-agent things such as food nutrition, and rules such as keeping a number of food items on the map; an environment file will name an optional ecology as it names its layout (AGORA-005 chunk 4)
- declaring kinds in a definition; definitions use the built-in kinds ([SPEC-006-R04](kinds-and-appearance.md#built-in-kinds))
- layouts generated per run from the run's seed, which would be configured by a definition rather than drawn in one
- loading definitions from files at run time rather than bundling them
- terrain that changes during a run

## Terminology and preconditions

- **Layout:** the terrain kind of every cell in a bounded grid.
- **Environment:** what a run is built from: a kind registry and a layout.
- **Layout file:** a layout written as TOML. Its **map** draws the layout as text, and its **legend** says which kind each character stands for. Its **layout ID** is its file name without the extension.
- **Environment file:** an environment written as TOML, naming its layout by ID.
- **Definition:** a layout file or an environment file.
- **Bundled definition:** a definition compiled into Agora. A bundled environment is named by its catalog entry ID.

## Requirements

### Layouts and environments

- **SPEC-007-R01:** A layout has a width, a height, and one kind ID per cell. Cells are in row-major order from the south-west corner: the cell at `(x, y)` is at index `y * width + x`, using the simulation's coordinates ([SPEC-001-R02](lifecycle-and-movement.md#creation-and-coordinates)). A zero width or height is rejected, as are dimensions whose cell count cannot be represented on the platform and a cell count that does not equal width × height.
- **SPEC-007-R02:** An environment is built from a kind registry and a layout. Every agent gets the built-in `agent` kind ([SPEC-006-R04](kinds-and-appearance.md#built-in-kinds)), so the registry must declare it. Building fails, naming the kind, when a layout kind is not in the registry or is not terrain, or when the registry has no `agent` kind or its `agent` kind is not a creature. A built environment is therefore consistent, and the simulation accepts it without further checks.

### Layout files

- **SPEC-007-R03:** A layout file is a TOML document with exactly these fields:

  | Field | Type | Meaning |
  | --- | --- | --- |
  | `map` | string | The layout, drawn as text ([R05](#layout-files)). |
  | `legend` | table of strings | Each key is a character in the map, and its value is a kind ID ([R04](#layout-files)). |

  Invalid TOML, a missing field, an unknown field, and a field of the wrong type are rejected. So is an empty kind ID. Loading takes a kind registry. An example:

  ```toml
  map = """
  ..#
  ...
  """

  [legend]
  "." = "floor"
  "#" = "wall"
  ```

- **SPEC-007-R04:** Each legend key is exactly one Unicode character, of any script, so a legend is not limited to ASCII. Each value must name a terrain kind in the registry. Every entry is checked, whether or not the map uses it. A key of any other length is rejected, naming the key; an unknown kind or a kind that is not terrain is rejected, naming the character and the kind.
- **SPEC-007-R05:** The map is read line by line. Lines may end with LF or CRLF, and the final line ending is optional; TOML also drops a line ending directly after the opening `"""`. The first line is the northern row and the last is the southern row, and each character is one cell, west to east. So the map's width is its line length and its height is its number of lines. A map with no lines is rejected. A line whose length differs from the first is rejected, naming the line and both lengths. A character not in the legend is rejected, naming it with its line and column. Lines and columns are counted from 1 at the top left.

### Environment files

- **SPEC-007-R07:** An environment file is a TOML document with exactly this field:

  | Field | Type | Meaning |
  | --- | --- | --- |
  | `layout` | string | The layout ID of the layout the environment uses. |

  Invalid TOML, a missing field, an unknown field, and a field of the wrong type are rejected. Loading takes a kind registry and the layouts already loaded with it, by layout ID, and does not read layout files itself. A layout ID that is not among them, including an empty one, is rejected, naming the ID. The loaded environment is checked as in R02. An example:

  ```toml
  layout = "divided-10x10"
  ```

### Bundled definitions

- **SPEC-007-R06:** Agora bundles these definitions, compiled into the program. Layout files are in `rust/agora-env/environments/layouts/` and environment files in `rust/agora-env/environments/`, each named after its ID with a `.toml` extension; every file in those directories is bundled, and there are no other directories. Loading the bundled environments with a kind registry loads every bundled layout first, including any that no environment uses, and then each environment in catalog order. A failure names the file that failed. All of them load with the built-in kinds.

  | Catalog entry | Layout | Size | Terrain |
  | --- | --- | --- | --- |
  | `empty-grid-10x10` | `empty-grid-10x10` | 10 × 10 | all floor |
  | `divided-10x10` | `divided-10x10` | 10 × 10 | floor, with a wall along x = 5 except at y = 4 and y = 5 |

## Interfaces and data

`agora-env` implements this spec. `Layout` holds a grid of `KindId`s, and `Layout::from_toml` loads one from a layout file. `Environment::new` builds a checked environment from a registry and a shared layout, and `Environment::from_toml` loads one from an environment file and a map of loaded layouts. `agora_env::bundled::LAYOUTS` and `ENVIRONMENTS` list the bundled files' IDs and sources, and `bundled::environments` loads them all. Errors are `LayoutError`, `EnvironmentError`, `DefinitionError`, and `BundledError`. Code can build a `Layout` directly, so the map is only the file form.

## Acceptance criteria and verification

Tests are in `rust/agora-env/tests/environment_definitions.rs`. Each test name starts with the requirement IDs it checks.

| Requirement | Check and expected outcome | Test |
| --- | --- | --- |
| R01 | Cells are found at `y * width + x`; zero dimensions and a wrong cell count are rejected. | `r01_*` |
| R02 | Unknown and non-terrain layout kinds are rejected; a registry without the `agent` kind, or with an `agent` kind that is not a creature, is rejected. | `r02_*` |
| R03, R05 | A layout file loads with its first line north; invalid TOML, unknown, missing, and mistyped fields, the earlier combined format, and an empty kind ID are rejected. | `r03_*` |
| R04 | Non-ASCII keys work; empty and multi-character keys, and unknown and non-terrain kinds, are rejected, including unused entries. | `r04_*` |
| R05 | LF, CRLF, and an unterminated last line give the same layout; empty, ragged, and unknown-character maps are rejected with their positions. | `r05_*` |
| R06 | The bundled lists, their files and directories, the two layouts as used by their environments, and a failure naming its environment. | `r06_*` |
| R07 | An environment file uses its named layout; unknown and empty layout IDs are rejected, naming the ID; invalid TOML, missing, mistyped, and unknown fields, including `agent_kind` and a not-yet-supported `ecology`, are rejected; the environment is checked. | `r07_*` |

## Open questions

- The ecology file format, and the environment file's `ecology` field, are specified with AGORA-005 chunk 4.4, when ecologies have properties and rules to hold.
- Whether a map can also place items, for example with a second map or a list of positions, waits until an environment needs placed items.
- How definitions declare their own kinds, and how those combine with the built-in kinds, waits until an environment needs a new kind.
