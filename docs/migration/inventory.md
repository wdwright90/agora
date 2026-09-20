# POC migration inventory

Status: initial structural review underway; no porting decisions made.

The sibling `agora-poc` directory is a reference implementation. Its contents and documentation must be reviewed before selecting code to port. Do not assume its existing roadmap is the new project's scope.

For each reviewed item, record the source commit, relative paths, and any relevant uncommitted modifications. Use `port`, `adapt`, `replace`, or `omit` only after evaluation; use `undecided` beforehand.

| Source component / paths | Source revision | New requirement / CDD | Disposition | Rationale | Tests / fixtures | Migration task |
| --- | --- | --- | --- | --- | --- | --- |

An initial review of workspace boundaries and simulation, server, and viewer entry points is recorded in [AGORA-001 context](../../work/AGORA-001-rust-package-design/context.md), including source revision, working-tree state, and differences from current requirements. Detailed source inventory and reuse evaluation remain pending; all reviewed code has an undecided disposition.
