---
name: condense-repo
description: Identify consolidation opportunities in the codebase - duplicate code, unused files, and redundant patterns. Generates actionable refactoring reports.
---

# Condense Repo

Identify consolidation opportunities in the codebase — duplicate code, unused files, and redundant patterns.

## Invocation

```
/condense-repo [options]
```

**Options:**
- `--scope path` — Limit analysis to specific directory
- `--report-only` — Generate report without proposing changes
- `--deep` — Include more thorough analysis (slower)

## Process

1. **Discover Project Structure:**
   - Walk the directory tree starting from the project root
   - Auto-detect source directories (`src/`, `lib/`, or top-level source files)
   - Auto-detect test directories (`tests/`, `test/`)
   - Auto-detect script/config directories
   - Note the primary language(s) by file extension distribution

2. **Identify Duplicate Code:**
   - Find similar function implementations across the codebase
   - Find repeated code blocks (>10 lines)
   - Note copy-paste patterns
   - Check for similar patterns across languages if multi-language project

3. **Find Unused Files:**
   - Check for unreferenced modules using language-appropriate import analysis:
     - **Rust:** `use` / `mod` / `pub use` statements
     - **Python:** `import` / `from ... import` statements
     - **JS/TS:** `import` / `require()` statements
     - **Go:** `import` blocks
   - Find orphaned scripts
   - Identify stale test files (tests for deleted code)

4. **Detect Redundant Patterns:**
   - Multiple implementations of same functionality
   - Overlapping utility functions
   - Inconsistent approaches to same problem

5. **Analyze Dependencies:**
   - Check for circular imports
   - Find unused imports
   - Note overly complex dependency chains

6. **Generate Report:**
   - Save to `outputs/condensation-report-[YYYY-MM-DD].md`
   - Summarize findings by category
   - Propose specific refactoring actions

## Output Format

```markdown
# Condensation Report

**Generated:** [Date]
**Scope:** [Full repository or path]
**Files Analyzed:** [count]
**Languages Detected:** [Python, JS/TS, Go, etc.]

## Summary

| Category | Count | Priority |
|----------|-------|----------|
| Duplicate code | N | Medium |
| Unused files | N | Low |
| Redundant patterns | N | High |

## Duplicate Code

### Pattern: [Name]
**Files:** [file1], [file2]
**Lines:** ~N each
**Recommendation:** [Action]

## Unused Files

### [filename]
- Last modified: [date]
- No imports found
- **Recommendation:** Delete or archive

## Redundant Patterns

### [Pattern name]
- [Description of redundancy]
**Recommendation:** [How to consolidate]

## Proposed Actions

1. **High Priority**
   - [Action items]

2. **Medium Priority**
   - [Action items]

3. **Low Priority**
   - [Action items]

## Metrics

- Estimated lines saved: ~N
- Files to remove: N
- New shared modules: N
```

## Example Usage

Full repository analysis:
```
/condense-repo
```

Limit to source directory:
```
/condense-repo --scope src/
```

Report only (no change proposals):
```
/condense-repo --report-only
```

## When to Use

- After completing a major feature
- When codebase feels cluttered
- Before starting a refactoring effort
- Monthly maintenance check

## Notes

- This skill generates reports, does not execute changes
- Proposed changes require user approval before implementation
- Focus on high-impact consolidations first
- Preserve working code — don't break things for cleanliness
