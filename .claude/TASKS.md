# Tasks

## Current Tasks

_(No active tasks)_

## Backlog

- [ ] **Concise changelog / patch notes** — Write player-facing patch notes summarizing the refactoring work. Should be concise and non-technical — focus on what changed from a user perspective (e.g., "no gameplay changes, internal code restructuring for maintainability"). Could live in a PATCH_NOTES.md or as a GitHub release note.
- [ ] **PlayerState struct refactor** — Consolidate scattered per-player state (`player_techs`/`opponent_techs`, `player_lp`/`opponent_lp`, `mp_player_name`/`mp_opponent_name`, `placed_packs`/`opponent_packs`) into a `PlayerState` struct. Main challenge: `units` Vec mixes both teams and combat systems iterate cross-team for targeting/collision/damage — splitting into per-player Vecs requires rewriting combat functions. BuildState is also asymmetric (local player builds interactively, opponent arrives via network). Scope as its own project after main.rs decomposition is complete. Touches: combat.rs, match_progress.rs, economy.rs, pack.rs, game_state.rs.

## Completed

- [x] Set up and integrate claude-toolkit skills into project file structure
- [x] Create README.md for the repository
- [x] Condensation report — identify duplicate code, unused files, redundant patterns
- [x] Decompose main.rs Round 1 — extract rendering, phase UI, chat, draft/ban, helpers (2,249 → 1,158 lines)
- [x] Decompose main.rs Round 2 — GameContext/BattleState structs, extract all phase logic (1,158 → 302 lines)

## Session Log

_(Session handoff entries will be appended here by `/handoff`)_
