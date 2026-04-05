# Changelog

## 2026-04-04

### Patch Notes

- `[internal]` Decomposed main.rs Round 2 — extracted all phase logic into dedicated modules (1,158 → 302 lines)
- `[internal]` Introduced GameContext and BattleState structs for shared/battle-scoped state
- `[internal]` Created context.rs, battle_phase.rs, build_phase.rs, waiting_phase.rs, round_result.rs, game_over.rs
- `[docs]` Added design spec and implementation plan for phase extraction
- `[tooling]` Installed Rust toolchain and MSVC Build Tools on dev machine
- `[internal]` Verified multiplayer functionality after refactor — all features working

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

### Patch Notes

- `[tooling]` Initialized .claude/ directory with project tooling and context documents
- `[tooling]` Imported skills from claude-toolkit: condense-repo, update-docs, handoff
- `[docs]` Created README.md with project overview and build instructions
- `[internal]` Condensation report — analyzed 19 source files for duplicate code and redundant patterns
- `[internal]` Decomposed main.rs Round 1 — extracted rendering, phase UI, chat, draft/ban (2,249 → 1,158 lines)

## Pre-2026-04-03

### Patch Notes

- `[fix]` Fixed multiplayer desync issues and added state synchronization
- `[net]` Added desync detection and determinism fixes
- `[net]` Added deploy zone and name sync for multiplayer
- `[gameplay]` Added multi-pack drag selection
- `[balance]` Balance and UI improvements
- `[fix]` Fixed camera pan, text rendering, and tech panel click areas
