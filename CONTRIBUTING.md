# Contributing

## Starting work

Find the relevant documents in [the index](docs/index.md). The [SADD](docs/architecture/sadd.md) describes the system; CDDs describe components; specs define actionable behavior. Draft documents are proposals. Record maintainer acceptance before treating a new design as an established project requirement.

Use [the AI development workflow](docs/workflows/ai-development.md) for substantial work. Small fixes can use the issue or commit description for scope and validation.

## Making changes

Keep changes scoped and reviewable. Update affected documentation in the same change. Preserve a single canonical definition for each shared contract. Meaningful architecture changes should include a decision record explaining their rationale.

Markdown files use relative links and descriptive names. Documents with IDs keep those IDs when renamed. Each document states whether it is draft, accepted, or superseded; acceptance does not imply implementation is complete.

## Verification and completion

Check Markdown links and template consistency for documentation changes. For code changes, use the affected packages' documented checks and applicable cross-language contract tests. Report commands, outcomes, and limitations.

Before committing, inspect the diff and staged file list. Exclude local run artifacts and credentials. A substantial task is complete when its acceptance criteria are verified, affected documents are updated, and its handoff records the outcome and follow-up work.
