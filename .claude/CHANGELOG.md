# Changelog

## 2026-04-03

- Initialized `.claude/` directory with CLAUDE.md and GUIDELINES.md
- Added session-hygiene rule (`.claude/rules/session-hygiene.md`)
- Imported skills from claude-toolkit: `condense-repo`, `update-docs`, `handoff`
- Created project tracking documents: TASKS.md, PLANNING.md, CHANGELOG.md
- Integrated claude-toolkit skills for Rust project:
  - Updated `/handoff` to write session logs to CHANGELOG.md instead of TASKS.md
  - Updated `/update-docs` to manage CHANGELOG.md
  - Adapted `/condense-repo` import analysis for Rust (`use`/`mod`/`pub use`)
  - Updated CLAUDE.md forced verification to use `cargo check`/`cargo clippy`
  - Added Available Skills and Context Documents sections to CLAUDE.md
  - Updated CLAUDE.md rename search checklist for Rust patterns
- Created README.md with project overview, build instructions, and skills table
