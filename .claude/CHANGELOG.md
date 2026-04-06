# Changelog

## 2026-04-05

### Patch Notes

- `[internal]` Extracted `grid_positions()` and `apply_unit_techs()` helpers, deduplicated pack spawning across pack.rs and game_state.rs
- `[internal]` Extracted `ray_intersects_aabb()` helper, deduplicated slab-method intersection in terrain.rs
- `[internal]` Extracted `apply_damage()` and `is_closer()` helpers, deduplicated damage tracking and tiebreaking in combat.rs
- `[internal]` Added `v2()`/`t2()` helpers, deduplicated Vec2 conversions in sync.rs
- `[internal]` Added `GameContext::start_game()`, deduplicated 4 lobby transition blocks in main.rs
- `[internal]` Fixed 26 clippy warnings across 11 files (is_multiple_of, is_none_or, is_some_and, or_default, question_mark, collapsible_if, unnecessary_cast, needless_borrow, ptr_arg, etc.)
- `[internal]` Introduced `MouseState` struct in `input.rs` — centralized all mouse input queries into a single per-frame struct
- `[internal]` Adopted `MouseState` across build_phase, battle_phase, lobby (update + draw), eliminating scattered inline macroquad input queries
- `[internal]` Extracted `draw_build_overlays` from `draw_world`, reducing signature from 9 → 5 args
- `[docs]` Designed PlayerState & host/guest architecture — spec at `docs/superpowers/specs/2026-04-05-playerstate-host-guest-design.md`
- `[docs]` Wrote 11-task implementation plan at `docs/superpowers/plans/2026-04-05-playerstate-host-guest.md`
- `[internal]` Introduced `Role` enum (Host/Guest/Spectator) in `role.rs` with `deploy_x_range()` and `player_id()` methods
- `[internal]` Renamed `team_id` to `player_id` across entire codebase (~80 occurrences, 16 files)
- `[internal]` Created `PlayerState` struct (player_id, lp, techs, name, next_id, gold, packs, ai_memory)
- `[internal]` Restructured `MatchProgress` with canonical `host: PlayerState` + `guest: PlayerState`
- `[internal]` Added `Role` to `GameContext`, added `player()`/`opponent()` accessor methods on MatchProgress
- `[internal]` Unified `PlacedPack` type — deleted `OpponentPlacedPack`
- `[internal]` Guest camera flip via negative x-zoom — automatic input correction
- `[internal]` Deploy zone parameterization from `Role::deploy_x_range()` — replaced hardcoded `HALF_W`
- `[net]` Removed all coordinate mirroring from state sync — canonical coordinates throughout
- `[tooling]` Added patch notes step to handoff skill with tagged changelog entries
- `[docs]` Restructured CHANGELOG.md with Patch Notes + Session Handoff format

### Session Handoff — PlayerState Implementation (Tasks 1-10)

**Git State:** branch `main`, clean, commit `5adc6b6`, up to date with origin
**Tests:** No test suite

**Work Completed:**
- Designed PlayerState & host/guest architecture through collaborative brainstorm (spec + 11-task plan)
- Executed Tasks 1-10 via subagent-driven development
- Task 1: Role enum + player_id rename (mechanical, clean)
- Tasks 2-7: PlayerState struct, MatchProgress restructure, GameContext Role, UI updates (subagent batch)
- Tasks 8-10: Camera flip, deploy zone parameterization, sync mirroring removal (applied manually after worktree conflicts)
- Code compiles clean with `cargo check` and `cargo clippy` (6 pre-existing too_many_arguments)

**In Progress:**
- Task 11 cleanup: `ArmyBuilder` still exists in economy.rs/game_state.rs, `BuildState` still has `builder`/`placed_packs`/`next_id` fields that should move to PlayerState, phase_ui still takes name params separately, `team.rs` params not renamed. Worktree subagent attempted this but diverged from design — needs inline cleanup.

**Decisions Made:**
- Worktree-based subagents cause merge conflicts when tasks are sequential (each forks from different base). For tightly coupled tasks, batching into one subagent is better.
- When worktree merges conflict, applying targeted changes manually on main is faster than resolving multi-file conflicts.

**Blockers:**
- None — remaining work is wiring cleanup, not architectural

**Next Steps:**
1. Complete Task 11 cleanup inline: remove ArmyBuilder, slim BuildState, wire phase_ui names from PlayerState, rename team.rs params
2. R key to rotate packs
3. Pause/options menu

### Session Handoff — PlayerState Design & Planning

**Git State:** branch `main`, 2 uncommitted doc changes (PLANNING, TASKS), commit `cf38485`, up to date with origin
**Tests:** No test suite

**Work Completed:**
- Reviewed yesterday's dedup commit — all 6 refactors verified correct
- Fixed 26 clippy warnings (32 → 4 remaining, all `too_many_arguments`)
- Created `MouseState` struct, adopted across build_phase, battle_phase, lobby, settings
- Extracted `draw_build_overlays` from `draw_world` (9 → 5 args)
- Designed PlayerState & canonical host/guest architecture through collaborative brainstorm
- Wrote and committed design spec and 11-task implementation plan

**In Progress:**
- PlayerState refactor ready to execute — plan approved, subagent-driven approach chosen

**Decisions Made:**
- Canonical host/guest state model: both clients store identical data, guest uses camera flip for perspective
- `team_id` renamed to `player_id` (u8), `player_id=0` is host, `>=1` are guests — supports future multi-player
- `Role` enum (Host, Guest, Spectator) on GameContext as single source of truth
- `PlayerState` struct: player_id, lp, techs, name, next_id, gold (live balance), packs (unified PlacedPack), ai_memory
- `MatchProgress` holds `host: PlayerState` + `guest: PlayerState` + round + banned_kinds
- `BuildState` slimmed to session UI state only — methods take `&mut PlayerState`
- `ArmyBuilder` removed — gold tracked directly on PlayerState
- Camera flip via negative x-zoom for guest — macroquad transforms handle input automatically
- Deploy zone derived from `Role::deploy_x_range()` — replaces hardcoded HALF_W
- All state sync mirroring removed — canonical coordinates throughout
- Tool decline = hard stop feedback saved to memory

**Blockers:**
- None

**Next Steps:**
1. Execute PlayerState implementation plan (11 tasks, subagent-driven)
2. R key to rotate packs
3. Pause/options menu

### Session Handoff — Clippy Cleanup + MouseState

**Git State:** branch `main`, 16 modified files + 1 new file (src/input.rs), ahead of origin by 1 commit
**Tests:** No test suite

**Work Completed:**
- Reviewed yesterday's dedup commit (7e59f0a) — all 6 refactors verified correct, cargo check passes
- Fixed 26 clippy warnings across 11 source files (32 → 5 remaining)
- Remaining 5 warnings are all `too_many_arguments` — categorized into independent vs PlayerState-overlapping tasks
- Created `input::MouseState` struct centralizing mouse position, clicks, button-down state, and scroll
- Adopted `MouseState` in build_phase (8 → 4 args), battle_phase, lobby (update + draw), settings (left_down param)
- Eliminated all inline macroquad mouse queries outside of main.rs construction site

**In Progress:**
- Nothing — clean stopping point

**Decisions Made:**
- Renamed `InputState` → `MouseState` since all fields are mouse-related; keyboard input stays as inline `is_key_pressed` calls (context-specific, would bloat struct)
- `SwatchLayout` struct skipped — too narrow (1 function, 2 call sites, different values each time)
- `phase_ui` too_many_arguments (3 functions) deferred to PlayerState refactor — `mp_player_name`/`mp_opponent_name` will consolidate into PlayerState
- `settings::draw_color_swatches` too_many_arguments left as-is — layout params are contextually different per call site
- Tool decline = hard stop feedback saved to memory for future sessions

**Blockers:**
- None

**Next Steps:**
1. R key to rotate packs (small gameplay feature)
2. Pause/options menu (Escape-triggered, needs to coexist with existing Escape uses)
3. rendering::draw_world signature refactor (9 args → pass &GameContext + &BattleState)
4. PlayerState struct refactor (larger project, includes phase_ui fixes)

### Session Handoff — Code Deduplication

**Git State:** branch `main`, 2 uncommitted doc changes (PLANNING, TASKS), commit `7e59f0a`
**Tests:** No test suite

**Work Completed:**
- Deduplicated 6 code patterns from condensation report across 7 source files
- Added patch notes system integrated into handoff skill via CHANGELOG.md
- Restructured CHANGELOG.md with tagged entries and session handoff sections
- Ran all 5 dedup refactors in parallel via isolated worktrees

**In Progress:**
- Nothing — clean stopping point

**Decisions Made:**
- Patch notes live in CHANGELOG.md (not separate file), tagged with `[gameplay]`, `[internal]`, etc.
- Simple extract-function refactors don't need formal plans — just execute
- Worktree agents make uncommitted changes (not commits), so merging back requires file copy
- Removed dead `effective_rows`/`effective_cols` methods from PlacedPack after grid_positions replaced their usage

**Blockers:**
- None

**Next Steps:**
1. Fix 32 pre-existing clippy warnings (low effort, good hygiene)
2. PlayerState struct refactor (backlog — larger project)
3. Gameplay features (TBD)

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
