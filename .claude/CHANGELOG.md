# Changelog

## 2026-04-04

- **main.rs Decomposition Round 2** — Introduced `GameContext` and `BattleState` structs, extracted all remaining phase logic:
  - Created `context.rs` (GameContext with 12 shared fields)
  - Created `battle_phase.rs` (BattleState with 9 fields + battle update logic)
  - Created `build_phase.rs` (Build phase update — undo, drag, shop, tech, begin-round)
  - Created `waiting_phase.rs`, `round_result.rs`, `game_over.rs`
  - main.rs reduced from 1,158 → 302 lines (87% total reduction from original 2,249)
- Design spec: `docs/superpowers/specs/2026-04-04-extract-phase-logic-design.md`
- Implementation plan: `docs/superpowers/plans/2026-04-04-extract-phase-logic.md`
- Added PlayerState struct refactor to backlog as future work
- Installed Rust toolchain + MSVC Build Tools on development machine
- Verified multiplayer functionality after refactor — all features working

### Session Handoff — main.rs Decomposition Complete

**Git State:** branch `main`, 3 modified docs (CHANGELOG, PLANNING, TASKS), up to date with origin
**Tests:** No test suite

**Work Completed:**
- Condensation report identifying duplicate code, unused files, redundant patterns
- main.rs decomposition Round 1: rendering.rs, phase_ui.rs, chat.rs, draft_ban.rs, helper moves (2,249 → 1,158 lines)
- main.rs decomposition Round 2: context.rs, battle_phase.rs, build_phase.rs, waiting_phase.rs, round_result.rs, game_over.rs (1,158 → 302 lines)
- Installed Rust + MSVC Build Tools on this machine
- Release build produced at `target/release/mech-game-ddev.exe`

**In Progress:**
- Nothing — clean stopping point

**Decisions Made:**
- GameContext (12 fields) for shared state, BattleState (9 fields) for battle-only state
- obstacles/nav_grid in GameContext (persist across rounds), projectiles/splash_effects in BattleState (battle lifecycle)
- Lobby stays in main() (self-contained, only consumer of main_settings)
- PlayerState struct deferred — requires combat system rewrite (units Vec mixes teams)

**Blockers:**
- None

**Next Steps:**
1. Address duplicate code from condensation report (pack spawning × 4 locations is highest priority)
2. PlayerState struct refactor (backlog — larger project)
3. Consider extracting Lobby phase from main.rs (currently ~70 lines with duplicated update/draw handlers)

## 2026-04-03

- Initialized `.claude/` directory with CLAUDE.md and GUIDELINES.md
- Added session-hygiene rule (`.claude/rules/session-hygiene.md`)
- Imported skills from claude-toolkit: `condense-repo`, `update-docs`, `handoff`
- Created project tracking documents: TASKS.md, PLANNING.md, CHANGELOG.md
- Integrated claude-toolkit skills for Rust project
- Created README.md with project overview, build instructions, and skills table
- **Condensation Report** — Analyzed all 19 source files for duplicate code, unused files, redundant patterns. Report: `outputs/condensation-report-2026-04-03.md`
- **main.rs Decomposition Round 1** — Extracted 6 modules from main.rs:
  - Moved helpers to natural modules (draw_unit_shape→unit.rs, draw_hud→ui.rs, etc.)
  - Moved send_build_complete→net.rs, start_ai_battle→economy.rs
  - Created rendering.rs (world-space rendering, SplashEffect)
  - Created phase_ui.rs (screen-space UI per phase)
  - Created chat.rs (chat system)
  - Created draft_ban.rs (draft/ban phase)
  - main.rs reduced from 2,249 → 1,158 lines
