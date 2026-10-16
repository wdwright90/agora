---
id: SPEC-008
status: draft
owner: maintainer
parents: [CDD-001]
packages: [agora-env]
---

# Agent definitions

## Purpose and scope

This spec defines agent definitions, as described in the [simulation CDD](../cdd.md#second-milestone-design): the capabilities an agent is spawned with, the TOML files they are written in, and the definitions bundled with Agora. Agents have almost nothing by default. An agent has a capability only when its definition lists it, so clients choose the capabilities they use. Agent definitions are separate from environments ([SPEC-007](environment-definitions.md)), as the SADD's [extension boundaries](../../../architecture/sadd.md#extending-agents-and-environments) require. The server's agent catalog is built from the bundled definitions ([SPEC-003-R01](../../server/specs/sessions-and-runs.md#catalog-and-identifiers)), and the simulation takes a definition when it spawns an agent ([SPEC-001-R03](lifecycle-and-movement.md#spawning)).

It is extended feature by feature. Not yet covered, and not implemented:

- capabilities other than metabolism, such as sight and moving; every agent can currently move
- versioned capability declarations published by the server (AGORA-005 chunk 6), which will replace this file format and become the form clients send
- spawning a catalog agent with capabilities added, or a bare agent with capabilities listed (chunk 6)
- browsing the agent catalog and filtering it by capability
- agent definitions stored on the server and reused later; clients that keep their own definitions do so themselves
- species, which will give agents their own kinds (M3); every agent has the built-in `agent` kind

## Terminology and preconditions

- **Capability:** something an agent has, such as a metabolism, with its own parameters.
- **Agent definition:** the capabilities an agent is spawned with. A definition with no capabilities is a **bare** agent.
- **Agent file:** an agent definition written as TOML. Its ID is its file name without the extension.
- **Bundled agent:** an agent definition compiled into Agora, named by its agent catalog entry ID.

## Requirements

### Agent files

- **SPEC-008-R01:** An agent file is a TOML document whose top-level tables are capabilities, each named as in the capability reference (R02). An agent has exactly the capabilities its file lists, and an empty file is a bare agent. Nothing is filled in by default.

### Capability reference

- **SPEC-008-R02:** These are the supported capabilities and fields. Every field of a listed capability is required. This table is checked against the loader, so it lists exactly what an agent file can contain.

  | Capability | Field | Type | Range | Meaning |
  | --- | --- | --- | --- | --- |
  | `metabolism` | `decay` | number | ≥ 0 | Energy lost every step. |
  | `metabolism` | `rate` | number | ≥ 0 | Energy lost per unit of effort spent by the agent's actions. |

  **`metabolism`** gives the agent energy, from 0 to 1. Energy starts full, falls each step by `decay` plus the step's effort × `rate`, and at 0 the agent starves and is removed ([SPEC-001-R22 to R25](lifecycle-and-movement.md#metabolism)). The agent perceives its energy. An agent without a metabolism has no energy and never starves.

### Validation

- **SPEC-008-R03:** A file that is not TOML, that names a capability or field the reference does not list, or that leaves out a field or gives it the wrong type, is rejected. The error for an unknown capability or field names the supported ones, so a typo shows what could have been written.
- **SPEC-008-R04:** Every number must be finite and at least 0; a value outside that range is rejected, naming its capability and field. TOML integers are accepted where numbers are expected.

### Bundled definitions

- **SPEC-008-R05:** Agora bundles these agent definitions, compiled into the program. Agent files are in `rust/agora-env/agents/`, each named after its ID with a `.toml` extension, and every file in that directory is bundled. Loading the bundled definitions loads each in catalog order, and a failure names the file that failed.

  | ID | Capabilities |
  | --- | --- |
  | `forager` | `metabolism` with `decay` 0.005 and `rate` 1 |

  The forager's numbers are placeholders, to be tuned for training in AGORA-005 chunk 11. Standing still, it starves after about 200 steps; moving every step, after about 100.

## Interfaces and data

`agora-env` implements this spec. `AgentDefinition` holds an optional `Metabolism`; `AgentDefinition::default()` is a bare agent. `AgentDefinition::from_toml` loads and checks a file, `to_toml` writes one, and `check` checks a definition built in code. `agora_env::bundled::AGENTS` lists the bundled files' IDs and sources, and `bundled::agents` loads them. Errors are `AgentDefinitionError` and `BundledError::Agent`.

## Acceptance criteria and verification

Tests are in `rust/agora-env/tests/agent_definitions.rs`. Each test name starts with the requirement ID it checks.

| Requirement | Check and expected outcome | Test |
| --- | --- | --- |
| R01 | A file loads with only its listed capabilities; an empty file is a bare agent; definitions round-trip through their files. | `r01_*` |
| R02 | The capability reference table lists exactly the capabilities and fields the file format supports. | `r02_*` |
| R03 | Unknown capabilities and fields are rejected with errors naming the supported ones; missing and mistyped fields and invalid TOML are rejected. | `r03_*` |
| R04 | 0 and positive numbers, including integers, load; negative, NaN, and infinite values are rejected, naming the field; `check` applies the same rule. | `r04_*` |
| R05 | The bundled list is the forager, with its metabolism; every file in the agents directory is bundled. | `r05_*` |

## Open questions

- The capability declaration format of chunk 6, and how this file format and the bundled definitions move to it.
- How a stored or catalog agent is combined with added capabilities, and what happens when both set the same one.
- Where server-stored agent definitions live, who owns them, and how they are named; expected with authentication (M6) or later.
- Whether energy's starting level becomes a parameter.
