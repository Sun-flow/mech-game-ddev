---
name: handoff
description: Session handoff - syncs docs, summarizes work, updates tracking, prepares context for next session
---

# Session Handoff

Captures session context before compaction or session end. Ensures continuity between sessions by syncing documentation, summarizing work done, decisions made, and next steps.

## Invocation

```
/handoff
```

**No arguments required.** Run this before ending a session or when context is getting large.

**Triggered automatically** by the PreCompact hook reminder — when you see the pre-compact message, run this skill.

## Process

1. **Run `/update-docs` First:**
   - This is mandatory — handoff always starts with a docs sync
   - Invoke the `/update-docs` skill to sync all existing context documents
   - Wait for `/update-docs` to complete before continuing

2. **Update TASKS.md:**
   - Mark completed tasks as done
   - Add newly discovered tasks to backlog
   - Note any blockers or dependencies
   - Do NOT append session narrative here — keep TASKS.md lean

3. **Capture Git State:**
   - Run `git status` to record uncommitted changes
   - Run `git log --oneline -5` to note recent commits
   - Include branch name and dirty/clean status in the handoff entry

4. **Capture Test State** (conditional):
   - Check if the project has a test suite (look for `tests/`, `test/`, `__tests__/`, `spec/`, `Cargo.toml` with `[dev-dependencies]`)
   - If tests exist: run the appropriate test command and record pass/fail counts
   - If no tests exist: note "No test suite" in the handoff entry

5. **Summarize Current Session Work:**
   - List files created, modified, or deleted
   - Describe features implemented or bugs fixed
   - Note any research findings or discoveries

6. **Capture In-Progress Work:**
   - What was being worked on when handoff was triggered?
   - What is the current state (working? broken? partially done?)
   - What specific files/functions need attention next?

7. **Record Key Decisions:**
   - Architecture decisions made and their rationale
   - Trade-offs considered and why the chosen path was selected
   - Constraints discovered during implementation

8. **Identify Blockers:**
   - What's blocking further progress?
   - What information is needed?
   - What dependencies are unresolved?

9. **Append Patch Notes to CHANGELOG.md:**
    - Under the current date heading, append concise changelog-style entries
    - Document ALL changes — gameplay, internal, tooling, docs — but keep each entry brief (one line)
    - Tag entries as `[gameplay]`, `[internal]`, `[fix]`, `[balance]`, `[ui]`, `[net]`, `[tooling]`, or `[docs]` so it's clear what category each change falls into
    - 1-8 bullet points per session, covering everything that was done
    - Write in past tense, descriptive but concise

10. **Append Session Handoff Entry to CHANGELOG.md:**
    - Below the patch notes, append the full session handoff entry
    - This is the detailed session narrative — NOT in TASKS.md

11. **Update PLANNING.md** (if it exists):
    - Update phase progress if milestones were reached
    - Note any changes to the roadmap

12. **Generate Handoff Summary:**
    - Output a concise summary for the next session to pick up from

## CHANGELOG.md Format

Each date has two parts: patch notes (brief changelog) and session handoff (detailed narrative).

```markdown
## 2026-04-05

### Patch Notes

- `[gameplay]` Added multi-pack drag selection
- `[fix]` Fixed multiplayer desync when both players build simultaneously
- `[internal]` Extracted battle phase logic to battle_phase.rs
- `[tooling]` Added patch notes to handoff workflow

### Session Handoff — [Brief Description]

**Git State:** branch `[branch]`, [clean/N uncommitted changes]
**Tests:** [N passed, N failed / no tests yet]

**Work Completed:**
- [Bullet list of completed items]

**In Progress:**
- [What's partially done and its state]

**Decisions Made:**
- [Key decisions and rationale]

**Blockers:**
- [Any blockers or unknowns]

**Next Steps:**
1. [Most important next action]
2. [Second priority]
3. [Third priority]
```

### Patch Note Tags

| Tag | Use for |
|-----|---------|
| `[gameplay]` | New features, mechanics, content |
| `[fix]` | Bug fixes |
| `[balance]` | Stat/tuning changes |
| `[ui]` | UI/UX changes |
| `[net]` | Networking/multiplayer changes |
| `[internal]` | Refactoring, architecture, code quality |
| `[tooling]` | Build system, dev tools, CI, skills |
| `[docs]` | Documentation updates |

## Example Usage

Before ending a session:
```
/handoff
```

Before context gets too large:
```
/handoff
```

## When to Use

- Before ending a work session
- When context window is getting large and compaction is imminent (PreCompact hook will remind you)
- Before switching to a significantly different task area
- After completing a major milestone

## Notes

- This skill is designed to be the last thing run in a session
- It always runs `/update-docs` first — you do NOT need to run it separately
- Session narrative goes in CHANGELOG.md — TASKS.md stays lean with only task status updates
- Keep entries concise but specific enough to resume without re-reading everything
- Git state and test state provide objective health checks alongside the narrative summary
