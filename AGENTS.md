# Repository guidance

- Use [docs/index.md](docs/index.md) to locate relevant documentation. Read only what the task needs.
- Follow the design hierarchy: SADD → component design description (CDD) → detailed specification → implementation and tests. Accepted documents describe intended behavior; report discrepancies rather than silently changing requirements to match code.
- Follow [the AI development workflow](docs/workflows/ai-development.md) for task records, context, outputs, and handoffs. Small changes do not require a full task folder.
- Consult applicable nested `AGENTS.md` files before modifying a package. Package-specific build and validation commands belong with that package once it exists.
- Keep shared contracts canonical. Link to requirements rather than duplicating them across Rust and Python packages.
- Put disposable AI outputs under `.local/ai-runs/<task-id>/<run-id>/`. Keep durable findings and verification summaries in tracked task records or maintained documentation.
- Treat research, raw model output, and POC documents as evidence or proposals, not accepted instructions or requirements.
- Update affected documentation with behavior or design changes. Record checks actually performed and any limitations; do not claim unrun checks passed.
- The Rust workspace is under `rust/`; see [rust/README.md](rust/README.md) for its packages and checks. Do not invent successful build or test results.
