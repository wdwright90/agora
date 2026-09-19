# Task folder template

For substantial work, create `work/AGORA-NNN-short-name/`. Use the next unused task ID and add it to the work index. The following sections describe separate files; create optional files only when useful.

## brief.md

```markdown
# AGORA-NNN — <Title>

Status: planned | active | blocked | complete
Owner: <owner>

## Goal
<Intended outcome>

## Scope and exclusions
<Affected components/packages and boundaries>

## Acceptance criteria
<Observable conditions for completion>

## Dependencies and open questions
<Linked tasks or decisions>
```

## context.md

List relevant specs, CDDs, ADRs, source files/symbols, tests, and external sources. Explain why each is relevant. Record source revisions and relevant uncommitted changes. Separate facts, assumptions, and open questions. Use links rather than copying canonical documents.

## plan.md

List implementation steps, status, dependencies, and verification per milestone. Update the current plan as work changes; preserve consequential rationale in ADRs or findings.

## findings.md (optional)

Record useful research, evidence, source links/access dates, uncertainty, and recommendations. These are findings or proposals until checked and incorporated into maintained documentation.

## handoff.md

Record the latest verified state, changes made, actual checks and outcomes, unresolved issues, and the next concrete action. Link to retained evidence. For completed work, summarize acceptance-criteria results and follow-ups. Keep this concise enough for a new session to resume from it.
