# Session Hygiene

## End of Session
- Run `/update-docs` to sync all context documents.
- Run `/handoff` to capture session context, git state, test state, and next steps.
- `/handoff` runs `/update-docs` internally, so if running both, just run `/handoff`.

## Before Compaction
- When the PreCompact hook fires, run `/handoff` immediately.
- This preserves session continuity in `.claude/TASKS.md` before context is compressed.

## Skill-Gap Awareness
- If you find yourself lacking a relevant skill to complete a task, pause and discuss it with the user.
- Propose what kind of skill would be needed rather than improvising a workaround.
- At session end, consider whether any novel methods used should become a new skill.

## Skill Improvement
- After using existing skills, note if they felt incomplete or could be improved.
- Propose improvements at session end — don't silently modify skill files.
