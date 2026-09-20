---
name: agora-resume
description: Restore context when resuming work in the Agora repository by reading project guidance, selecting the relevant task handoff, and checking current Git state. Use for session resumption or a request to pick up previous Agora work.
---

# Resume Agora work

Orient the session using maintained repository records. Resolve the paths below from the repository root, including when invoked from a subdirectory.

1. Read [AGENTS.md](../../../AGENTS.md), the [documentation index](../../../docs/index.md), the [AI development workflow](../../../docs/workflows/ai-development.md), and the [work index](../../../work/index.md). Reuse content already read in the current session when still current.
2. Select the task named by the user. Otherwise, use the relevant active task; if none exists, consult the most recent relevant completed task's handoff for follow-up work. If several tasks are plausible and context does not distinguish them, ask which to resume. Do not load every historical handoff.
3. Read the selected task's `brief.md` and `handoff.md`. Read its `context.md` and referenced SADD, CDD, or spec sections only as needed for the next action. Completed discovery or proposal work does not mean the proposed design is accepted or implemented.
4. Inspect `git status --short` and recent commits. Compare them with the handoff, identify uncommitted work or stale checkpoint claims, and preserve existing changes. A handoff describes a prior checkpoint; current files and Git history establish what is present now.
5. Summarize the task, verified current state, unresolved decisions, and next concrete action, linking to the relevant records. If the user requested continuation, proceed within that scope; if they requested orientation only, stop after the summary. Consult applicable nested `AGENTS.md` files before editing.

Keep requirements in their canonical design documents and follow the linked workflow for task updates. This skill restores context; invoking it alone does not authorize commits, pushes, or acceptance of draft designs. If a task record or handoff is missing, report the gap and use the available project documents and Git history without inventing prior decisions.
