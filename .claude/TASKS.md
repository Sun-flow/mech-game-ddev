# Tasks

## Current Tasks

_(No active tasks)_

## Backlog

- [ ] **R key to rotate packs** — Add KeyCode::R as an alternative to middle-click for rotating packs during build phase. Check `is_key_pressed(KeyCode::R)` in build_phase.rs alongside existing middle_click rotation logic.
- [ ] **Pause/options menu** — Add an in-match pause menu triggered by Escape. Should overlay current phase, provide access to settings, and option to surrender/quit. Currently Escape is used for surrender confirm in battle phase and back-navigation in lobby — needs to coexist.
- [x] **rendering::draw_world signature** — Extracted build overlays to separate `draw_build_overlays` call, reducing draw_world from 9 → 5 args. Phase decision moved to caller.
- [ ] **Camera mode architecture** — Review `set_camera`/`set_default_camera` usage in main.rs render loop. Names are ambiguous — clarify world-space vs screen-space rendering contexts, consider whether camera management should be more explicit.
- [ ] **phase_ui too_many_arguments (3 functions)** — `draw_battle_ui` (10 args), `draw_round_result_ui` (8), `draw_game_over_ui` (8). All take `mp_player_name`/`mp_opponent_name` separately — these fields move into PlayerState. **Do alongside PlayerState refactor.**
- [ ] **PlayerState struct refactor** — Consolidate scattered per-player state (`player_techs`/`opponent_techs`, `player_lp`/`opponent_lp`, `mp_player_name`/`mp_opponent_name`, `placed_packs`/`opponent_packs`) into a `PlayerState` struct. Main challenge: `units` Vec mixes both teams and combat systems iterate cross-team for targeting/collision/damage — splitting into per-player Vecs requires rewriting combat functions. BuildState is also asymmetric (local player builds interactively, opponent arrives via network). Scope as its own project. Touches: combat.rs, match_progress.rs, economy.rs, pack.rs, game_state.rs, phase_ui.rs.

## Completed

- [x] Set up and integrate claude-toolkit skills into project file structure
- [x] Create README.md for the repository
- [x] Condensation report — identify duplicate code, unused files, redundant patterns
- [x] Decompose main.rs Round 1 — extract rendering, phase UI, chat, draft/ban, helpers (2,249 → 1,158 lines)
- [x] Decompose main.rs Round 2 — GameContext/BattleState structs, extract all phase logic (1,158 → 302 lines)
- [x] Patch notes system — integrated into handoff skill via CHANGELOG.md
- [x] Deduplicate code patterns — 6 patterns across 7 files (pack spawn, ray-AABB, damage tracking, tiebreak, Vec2 conversions, lobby transitions)
- [x] Fix pre-existing clippy warnings — 26 mechanical warnings fixed across 11 files, remaining 5 are `too_many_arguments`
- [x] MouseState struct — Centralized all mouse input queries into `input::MouseState`, adopted by main loop, build_phase, battle_phase, lobby

## Session Log

_(Session handoff entries will be appended here by `/handoff`)_
