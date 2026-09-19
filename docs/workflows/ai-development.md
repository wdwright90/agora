# AI development workflow

## Storage and authority

Maintained project knowledge lives in `docs/` and package documentation. Substantial task records live in `work/`. Disposable prompts, responses, transcripts, and logs live in gitignored `.local/ai-runs/<task-id>/<run-id>/`.

The design hierarchy is SADD → CDD → spec → implementation and tests. Drafts and raw model outputs are proposals. Maintainer acceptance establishes design status; implementation must still be verified. This convention does not override the user's current instructions or tool permissions.

ADRs explain consequential choices. Update the current SADD/CDDs/specs when a decision affects them. Do not allow a task handoff or old transcript to become a competing source of requirements.

## Selecting context

- For system work, read the relevant SADD sections and affected CDDs.
- For component design, read its CDD, relevant system constraints, and shared contracts.
- For implementation, start with the applicable spec and consult the CDD as needed.
- Read applicable package guidance before editing that package.
- Load historical tasks and POC material only when relevant; distinguish historical behavior from accepted requirements.

Root `AGENTS.md` is a short guide to this workflow, not a copy of the documentation. Repository skills are for specific repeatable procedures. Add them when a demonstrated workflow benefits from reuse.

## Task lifecycle

1. Define the goal, scope, exclusions, and acceptance criteria.
2. Gather relevant references and identify assumptions or unresolved decisions.
3. Plan substantial work in independently verifiable steps.
4. Implement and update affected documentation together.
5. Verify against acceptance criteria; record actual checks and limitations.
6. Incorporate validated, durable findings into the appropriate project documents.
7. Complete or hand off with current state, remaining issues, and the next action.

Use the [task template](../templates/task.md) for work spanning sessions or components. Small edits need only a concise description and appropriate verification. There is no mandatory full-document reading or approval cycle for routine changes already within the authorized scope.

Keep completed tasks at stable paths. Their `brief.md` owns task status; the [work index](../../work/index.md) separates active and completed links. Refresh the handoff at meaningful milestones and before stopping unfinished work, rather than logging every tool call.

## Inputs and outputs

`brief.md` is the maintained intent. `context.md` links to inputs and explains their relevance. Exact prompts may be retained locally when useful for experiments or debugging. Do not duplicate whole source documents in each task.

When preserving a run, use a unique run ID such as a UTC timestamp plus a short suffix. A local `manifest.json` should identify:

- Task ID, run ID, and timestamp.
- Source commit and relevant dirty-tree state; preserve a diff if exact reconstruction is needed.
- Tool and model versions when actually available; otherwise mark them unknown.
- Input paths or revisions and artifact locations.
- Commands and outcomes, or links to retained logs.

Raw outputs remain disposable. Retain concise findings and verification summaries in tracked task files. Store large datasets, recordings, and model artifacts outside ordinary Git history; use durable artifact links and checksums when needed. Promote only reviewed, useful fixtures to the appropriate package.

Do not store credentials in prompts or logs. Gitignore controls tracking, not access. Review retained artifacts before sharing. Local artifacts may be removed after useful evidence is retained and the task is complete; no automated deletion is configured.

## Rust and Python boundaries

Each CDD maps the logical component to its implementing packages. Package-local docs explain internals and commands. Shared contracts have one canonical spec under `docs/contracts/`, linked from all participating CDDs. Include applicable interoperability checks when changing those contracts.

## Documentation maintenance

Check relative links, document IDs, metadata, status, and parent references. Keep templates proportional to the task; remove irrelevant sections. Generated API documentation should be produced from source once packages exist. Prose should explain concepts, requirements, and rationale.

Automation for these checks is future work. Do not imply CI validation exists until it is implemented.

## Research basis

These folder names and lifecycle rules are Agora conventions. Codex supports repository instructions and skills, but does not automatically manage the task directories described here.

- [AGENTS.md discovery](https://learn.chatgpt.com/docs/agent-configuration/agents-md): scoped instruction loading. Accessed 2026-09-19.
- [Build skills](https://learn.chatgpt.com/docs/build-skills): repository skills and progressive loading. Accessed 2026-09-19.
- [Long-running tasks](https://developers.openai.com/blog/run-long-horizon-tasks-with-codex): durable plans, state, and verification. Accessed 2026-09-19.
- [Context management guidance](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra): concise instructions and task-relevant references. Accessed 2026-09-19.
