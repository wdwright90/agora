---
name: agora-resume
description: Restore context when resuming work in the Agora repository by reading project guidance, selecting the relevant task handoff, and checking current Git state. Use for session resumption or a request to pick up previous Agora work. Optionally pass the task to resume (for example, AGORA-004).
---

# Resume Agora work (Claude Code entry point)

This is a thin wrapper so Claude Code can invoke the canonical repository skill. The procedure lives in [.agents/skills/agora-resume/SKILL.md](../../../.agents/skills/agora-resume/SKILL.md); do not duplicate or diverge from it here.

1. Read `.agents/skills/agora-resume/SKILL.md` from the repository root.
2. Follow it exactly. If arguments were passed to this skill, treat them as the task the user named in its step 2.
