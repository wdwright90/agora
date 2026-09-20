# Handoff

Discovery and the draft package proposal are complete for maintainer review. No Rust packages exist. The [SADD](../../docs/architecture/sadd.md#proposed-rust-package-mapping) owns the proposal; [ADR-001](../../docs/decisions/0001-rust-package-boundaries.md) explains the tradeoffs. POC structural findings and revision are in [context](context.md).

Discovery captured in the SADD: local-first development with machine-independent client access; future authenticated access for friends; client-requested environments and one or more agents; recorded training runs and automatic closure when unused; configuration and programmatic environment authoring; visual editor as stretch goal; visualizer-led local agent inspection. Parallel environments remain outside the first milestone and retain their earlier stretch-goal classification pending prioritization.

Further user direction captured: reusable profiles and local launching of ordinary clients; inference owned by each client; Rust-based extensions with language-neutral property and sense descriptions; inspection-only playback; environment modification during live observation.

Acceptance criteria met: high-level needs and open questions captured, POC differences recorded, package responsibilities and dependency direction proposed with first-milestone versus later scope. The proposal starts with protocol, simulation, server, client, and viewer, adding recording later. Package acceptance and implementation remain separate follow-up work.

Next: maintainer review of package boundaries, followed by CDDs and minimal shared contracts. Shared-run control, random environment selection versus generation, move and boundary rules, capability descriptions, transport, and process supervision remain open.

Verification: inspected POC source entry points and manifest, recorded source revision and Git status. Documentation whitespace checked with `git diff --check`; relative Markdown file targets checked in affected design and task documents. No implementation builds or tests performed.

## End-of-session checkpoint — 2026-09-19

The session also established the project purpose, workflows, first empty-grid milestone, default wait-for-all stepping, randomized conflict priority, and an empty Cargo workspace under `rust/`. These are maintained in the SADD and workspace documentation. Cargo recognized the workspace through `cargo locate-project --workspace --manifest-path rust/Cargo.toml`; `rust/target/` is ignored.

AGORA-001 is complete as a discovery and proposal task. The SADD and ADR-001 remain draft; completion does not imply acceptance of the package split or implementation. No CDDs, member packages, or application tests exist.

The user requested a Git commit of today's documentation, workspace scaffold, and task records. This handoff is included in that checkpoint; use Git history and working-tree status to verify the resulting commit. No remote push was requested.

Suggested next-session instruction: "Read AGENTS.md, docs/index.md, and work/AGORA-001-rust-package-design/handoff.md. Resume by reviewing the proposed Rust package boundaries in the SADD and ADR-001 with me. Keep implementation deferred until we have agreed the relevant design and contracts."
