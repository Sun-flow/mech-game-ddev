# Tasks

## Current Tasks

_(No active tasks)_

## Backlog

- [ ] **PlayerState struct refactor** — Consolidate scattered per-player state (`player_techs`/`opponent_techs`, `player_lp`/`opponent_lp`, `mp_player_name`/`mp_opponent_name`, `placed_packs`/`opponent_packs`) into a `PlayerState` struct. Main challenge: `units` Vec mixes both teams and combat systems iterate cross-team for targeting/collision/damage — splitting into per-player Vecs requires rewriting combat functions. BuildState is also asymmetric (local player builds interactively, opponent arrives via network). Scope as its own project after main.rs decomposition is complete. Touches: combat.rs, match_progress.rs, economy.rs, pack.rs, game_state.rs.

## Completed

- [x] Set up and integrate claude-toolkit skills into project file structure
- [x] Create README.md for the repository

## Session Log

_(Session handoff entries will be appended here by `/handoff`)_
