---
name: update-docs
description: Syncs Claude context documents
---

# Update Documentation

Syncs all Claude context documents after completing work. Auto-detects what needs updating based on session context.

## Invocation

```
/update-docs
```

**No arguments required.** The skill auto-detects what needs updating based on session context.

## Documents Managed

| Document | Update Triggers |
|----------|-----------------|
| `.claude/PLANNING.md` | Phase completion, new phase start, roadmap changes |
| `.claude/TASKS.md` | Task completion, backlog changes (no session narrative) |
| `.claude/CHANGELOG.md` | Work completed, session handoff entries |
| `CLAUDE.md` | New skill added (Available Skills section) |
| `README.md` | New skill, directory changes, feature changes |

## Process

1. **Detect Available Documents:**
   - Check which of the managed documents actually exist in this project
   - Only attempt to update documents that exist — do not create missing ones
   - Note: CLAUDE.md may be at root or `.claude/CLAUDE.md` depending on project

2. **Analyze Session Context:**
   - Review what work was done in the current session
   - Identify files created, modified, or deleted
   - Note any new skills, features, or structural changes

3. **Determine Updates Needed:**
   - Were any skills created/modified? → Update CLAUDE.md + README.md
   - Were tasks completed or new work done? → Update TASKS.md + CHANGELOG.md
   - Did project phase change? → Update PLANNING.md
   - Did directory structure change? → Update README.md

4. **Update PLANNING.md** (if it exists and applicable):
   - Mark completed phases
   - Update current focus/phase
   - Update dates and timestamps
   - Add any roadmap changes discovered

5. **Update TASKS.md** (if it exists and applicable):
   - Check off completed tasks
   - Update backlog with new items discovered
   - Note any blockers or dependencies
   - Do NOT add session narrative — keep lean

6. **Update CHANGELOG.md** (if it exists and applicable):
   - Add completed work entries under the current date heading
   - Include brief descriptions of what was done

7. **Update CLAUDE.md** (if it exists and applicable):
   - Add new skills to Available Skills section
   - Update existing skill descriptions if changed
   - Maintain alphabetical or logical ordering

8. **Update README.md** (if it exists and applicable):
   - Sync skills table with CLAUDE.md
   - Update directory structure if changed
   - Add new features or usage patterns
   - Update examples if relevant

9. **Report Changes:**
   - Summarize what documents were updated
   - List key changes made to each
   - Note any items requiring user attention

## Auto-Detection Logic

### Skill Changes
- Check: Were any `.claude/skills/*/SKILL.md` files created or modified?
- Action: Update CLAUDE.md Available Skills section and README.md skills table

### Task Progress
- Check: Were tasks discussed, completed, or new work done?
- Action: Update TASKS.md with completions and session notes

### Phase Changes
- Check: Did the project reach a milestone or change focus?
- Action: Update PLANNING.md with phase status and dates

### Structural Changes
- Check: Were directories created, files reorganized, or outputs changed?
- Action: Update README.md directory structure

## Output Format

After updating, report changes in this format:

```
## Documentation Updated

### Files Modified
- `.claude/TASKS.md` - Checked off 2 tasks, added session notes
- `CLAUDE.md` - Added `/new-skill` to Available Skills
- `README.md` - Updated skills table

### Files Skipped (not present)
- `.claude/PLANNING.md` - Does not exist in this project

### Summary
[Brief description of session work and documentation sync]
```

## Example Usage

At the end of a session:

```
/update-docs
```

The skill will:
1. Detect which documents exist
2. Analyze what work was done
3. Update all relevant documentation
4. Report what was changed

## Notes

- Run this skill at the end of sessions or after significant work
- The skill reads current document state to avoid redundant updates
- If no updates are needed, the skill will report that documents are current
- Missing documents are skipped gracefully — not all projects have all four files
- Manual updates are still valid; this skill supplements rather than replaces them
