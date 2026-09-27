# Repository skills

Add a skill here when a specific, repeated project workflow benefits from reusable instructions or supporting scripts. Each skill gets its own directory with a `SKILL.md` and concise trigger description.

Keep general working rules in the root `AGENTS.md` and project design in `docs/`. Link to canonical documentation instead of copying it into skills.

- [agora-resume](agora-resume/SKILL.md): restore session context from guidance, the relevant task handoff, and current Git state. Invoke with `$agora-resume` and optionally name the task to resume. In Claude Code, use `/agora-resume`, a wrapper in [.claude/skills](../../.claude/skills/agora-resume/SKILL.md) that defers to this skill.
