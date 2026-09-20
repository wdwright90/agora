# Documentation index

## Design hierarchy

1. [SADD](architecture/sadd.md): system structure, component responsibilities, dependencies, and cross-cutting constraints.
2. [Component design descriptions](components/README.md): high-level design of individual logical components.
3. Component specs and [shared contracts](contracts/README.md): detailed, actionable requirements and acceptance criteria.
4. Implementation and tests: packages will be introduced after their scope is defined.

Components are logical boundaries and can span multiple Rust crates or Python packages. CDDs map components to their implementing packages.

## Project workflow

- [Rust workspace](../rust/README.md)
- [Contributing](../CONTRIBUTING.md)
- [AI development workflow](workflows/ai-development.md)
- [Active work](../work/index.md)
- [Architecture decisions](decisions/README.md)
- [Migration inventory](migration/inventory.md)
- [Templates](templates/README.md)
- [Repository skills](../.agents/skills/README.md)

## Document status

Design documents use `draft`, `accepted`, or `superseded`. Track implementation progress in task records, not by changing the meaning of design status. Drafts must identify unresolved questions. Superseded documents link to their replacements and are not current requirements.

Document IDs are stable and unique within each prefix: `SADD`, `CDD`, `SPEC`, and `ADR`. Use descriptive filenames; allocate the next unused ID when creating a document. Templates do not allocate IDs.

The SADD currently captures the project purpose and high-level requirements in draft form; detailed architecture remains to be defined. No component design, detailed spec, or product roadmap has been accepted yet.
