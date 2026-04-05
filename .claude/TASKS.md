# Tasks

## Current Tasks

_(No active tasks)_

## Backlog

- [ ] **R key to rotate packs** — Add KeyCode::R as an alternative to middle-click for rotating packs during build phase. Check `is_key_pressed(KeyCode::R)` in build_phase.rs alongside existing middle_click rotation logic.
- [ ] **Pause/options menu** — Add an in-match pause menu triggered by Escape. Should overlay current phase, provide access to settings, and option to surrender/quit. Currently Escape is used for surrender confirm in battle phase and back-navigation in lobby — needs to coexist.
- [ ] **PlayerState & host/guest refactor** — 11-task implementation plan ready at `docs/superpowers/plans/2026-04-05-playerstate-host-guest.md`. Canonical host/guest state, PlayerState struct, Role enum, camera flip, deploy zone parameterization, sync simplification. Subsumes phase_ui too_many_arguments and camera architecture tasks.
- [ ] **Camera mode architecture** — Review `set_camera`/`set_default_camera` naming. Deferred — assessed as acceptable during this session.

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
- [x] rendering::draw_world signature — Extracted build overlays, reduced 9 → 5 args
- [x] PlayerState design & planning — Spec and 11-task implementation plan written and approved

## Session Log

_(Session handoff entries will be appended here by `/handoff`)_
