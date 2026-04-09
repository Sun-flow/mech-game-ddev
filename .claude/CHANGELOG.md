# Changelog

## 2026-04-08

### Patch Notes

- `[fix]` Fixed guest seeing wrong winner name at round end (flipped_winner/flipped_loser bug)
- `[fix]` Fixed desync check comparing flipped alive counts instead of canonical
- `[internal]` Deleted Role enum, replaced with `local_player_id: u16` on GameContext
- `[internal]` Added `deploy_x_range(player_id)` free function to arena.rs (later moved to PlayerState.deploy_zone)
- `[net]` Network messages now carry sender's player_id — no `1 - local` derivation anywhere
- `[internal]` Removed x-flip camera hack, replaced with Camera2D rotation field
- `[gameplay]` Added Q/E smooth camera rotation at 90 degrees/sec
- `[internal]` Changed player_id from u8 to u16, derived from WebRTC PeerId UUID
- `[internal]` Changed MatchProgress.players from `[PlayerState; 2]` to `Vec<PlayerState>` with `player()`/`player_mut()` lookup helpers
- `[internal]` Moved deploy_zone and color_index onto PlayerState
- `[internal]` Combat `update_attacks` takes `&[PlayerState]` instead of separate host_techs/guest_techs
- `[net]` RoundEnd message uses `Vec<RoundEndPlayerData>` instead of hardcoded alive_0/alive_1 fields
- `[internal]` Team color system uses `HashMap<u16, u8>` instead of two AtomicU8 statics
- `[internal]` BuildState::new takes next_id instead of is_host
- `[internal]` spawn_ai_army and start_ai_battle take ai_player_id parameter
- `[docs]` Added Architecture Principles section to GUIDELINES.md
- `[docs]` Specs and plans for canonical player-ID system and arbitrary player IDs

### Session Handoff — Canonical Player-ID System & Arbitrary Player IDs

**Git State:** branch `main`, 4 uncommitted doc changes (CHANGELOG, GUIDELINES, PLANNING, TASKS), commit `4c61925`
**Tests:** No test suite. Manual testing launched but results not yet reported.

**Work Completed:**
- Executed 9-task array-indexed PlayerState plan from previous session's spec (subagent-driven)
- Identified and fixed guest wrong-winner-name bug (flipped_winner/flipped_loser)
- Brainstormed canonical player-ID design — eliminated all perspective-relative patterns
- Brainstormed arbitrary player-ID design — u16 from PeerId, Vec-based storage
- Executed canonical player-ID plan (9 tasks): deleted Role, camera rotation, sender-embedded player_id
- Executed arbitrary player-ID plan (11 tasks): u16 everywhere, Vec<PlayerState>, lookup helpers, per-player RoundEnd, HashMap colors
- Fixed 7 remaining `players[id as usize]` index patterns that would crash with arbitrary IDs
- Cleaned up 4 stale worktrees from previous sessions

**In Progress:**
- User launched 2 game instances for testing but hasn't reported results yet

**Decisions Made:**
- Player IDs are arbitrary u16 derived from first 2 bytes of WebRTC PeerId UUID
- "Game code never computes 'who is the other player'" — saved as architecture principle
- Network messages carry sender's player_id, eliminating all `1 - local` derivation
- Camera uses rotation field instead of x-flip mirror; default 180 degrees for right-side builder
- MatchProgress uses Vec<PlayerState> with player(pid)/player_mut(pid) helpers, never index-based access
- Deploy zones and colors stored on PlayerState, not derived from Role/host/guest identity
- BuildState takes next_id: u64 instead of is_host: bool
- AI player_id is a parameter, not hardcoded as 1

**Blockers:**
- None — pending user test results

**Next Steps:**
1. Get test results from user — verify winner names, camera rotation, multiplayer sync
2. Push all changes to origin
3. R key to rotate packs (small gameplay feature)
4. Pause/options menu

### Patch Notes (continued)

- `[gameplay]` Added escape menu with Resume/Settings/Surrender — available during all match phases
- `[gameplay]` Added WASD/arrow camera panning relative to screen orientation
- `[gameplay]` Added Q/E camera rotation at 90 degrees/sec
- `[gameplay]` Added R key to rotate packs (alternative to middle-click)
- `[ui]` Settings sub-view in escape menu reuses existing game settings panel
- `[internal]` Removed old show_surrender_confirm system from BattleState
- `[internal]` Extracted draw_settings_content from draw_settings_panel to avoid double overlay
- `[fix]` Fixed rematch crash — deploy zones/colors now preserved across rematch
- `[fix]` Fixed W/S camera panning direction (was inverted)
- `[fix]` Fixed camera panning at non-90-degree rotations using screen_to_world derivation
- `[fix]` Fixed surrender not communicated to peer — now sends Surrender message and polls in all phases
- `[fix]` Fixed remaining players[id as usize] index patterns that crashed with arbitrary player IDs

### Session Handoff — Escape Menu, Camera Controls, Bug Fixes

**Git State:** branch `main`, clean, pushed to origin at `d21dfb5`
**Tests:** No test suite. Manual multiplayer testing passed — surrender, rematch, camera, R-rotate all verified.

**Work Completed:**
- Designed and implemented escape menu (Resume/Settings/Surrender) during all match phases
- Single-player pauses while escape menu is open; multiplayer continues
- Old surrender confirm overlay fully removed; surrender now lives in escape menu
- WASD/arrow camera panning relative to screen orientation (accounts for camera rotation)
- R key to rotate packs alongside existing middle-click
- Settings sub-view extracted from draw_settings_panel to avoid double overlay
- Fixed surrender not communicating to peer — sends net message and polls in all phases
- Fixed rematch crash (deploy zones/colors not preserved in new MatchProgress)
- Fixed W/S panning direction inversion
- Fixed camera panning direction at non-90-degree angles using screen_to_world
- Fixed 13 remaining players[lpid] index patterns and 3 hardcoded player_id 0/1 patterns

**In Progress:**
- Nothing — clean stopping point

**Decisions Made:**
- Escape menu overlay blocks ALL game and camera input — only menu buttons are interactive
- Single-player pause: dt not applied to combat or build timer while menu is open
- Settings sub-view reuses existing settings::draw_settings_content (extracted from draw_settings_panel)
- Camera panning derives world-space directions from screen_to_world (no manual trig conventions)
- Surrender sends net message immediately; peer polls for surrendered_player every frame during match

**Blockers:**
- None

**Next Steps:**
1. Polish settings panel layout in escape menu vs lobby
2. More gameplay features (user to decide)

## 2026-04-09

### Patch Notes

- `[internal]` Added UI helpers: `point_in_rect`, `draw_button`, `draw_centered_text`, `center_x`/`center_y`
- `[internal]` Added `MatchProgress::other_players()` iterator helper
- `[internal]` Adopted helpers across 10 files, eliminating ~200 lines of duplicate UI code
- `[ui]` Escape menu settings now shows client settings only (UI scale), not match settings
- `[docs]` Saved feedback: match settings = host-only, lobby-only; client settings = escape menu

### Session Handoff — Codebase Consolidation & Settings Scope

**Git State:** branch `main`, 1 uncommitted change (TASKS.md), pushed to origin at `55409ce`
**Tests:** No test suite

**Work Completed:**
- Ran condensation report — identified 4 high-priority and 6 medium-priority consolidation opportunities
- Implemented all high-priority helpers in ui.rs (point_in_rect, draw_button, draw_centered_text, center_x/center_y)
- Added other_players() iterator to MatchProgress
- Swept 10 files to adopt all helpers (replaced ~9 rect checks, ~5 button patterns, ~20 centered text patterns, ~9 skip-by-id loops)
- Fixed escape menu settings to show client settings only (removed match settings)
- Saved settings scope feedback to memory

**In Progress:**
- Nothing — clean stopping point

**Decisions Made:**
- Match settings (terrain, draft/ban, smart AI) are host-only, lobby-only — never shown in escape menu
- Client settings (UI scale, keybindings) belong in escape menu — editable anytime
- Net polls left as-is after verifying main loop polls AFTER phase updates, so per-phase polls are the primary mechanism
- draw_button has 8 args (1 over clippy threshold) — acceptable for now, could take a struct later

**Blockers:**
- None

**Next Steps:**
1. Keybindings reference list in pause menu
2. More gameplay features (user to decide)

## 2026-04-07

### Patch Notes

- `[docs]` Designed array-indexed PlayerState refactor — spec at `docs/superpowers/specs/2026-04-06-array-indexed-playerstate-design.md`
- `[docs]` Updated PLANNING.md roadmap — marked PlayerState phase 3 complete, added phase 4 (array-indexed) and phase 6 (multi-peer networking)
- `[internal]` Removed resolved camera flip winding fix from backlog

- `[internal]` Replaced `host`/`guest` fields with `players: [PlayerState; 2]` in MatchProgress
- `[internal]` Replaced `new_host()`/`new_guest()` with unified `PlayerState::new(player_id)` constructor
- `[internal]` Removed 6 perspective-relative accessors (`player()`, `opponent()`, `player_mut()`, `opponent_mut()`, `player_lp()`, `opponent_lp()`)
- `[internal]` Extracted `apply_opponent_build` method to free function `apply_peer_build(&mut PlayerState, &PeerBuildData, round)`
- `[net]` Renamed all net `opponent_*` fields/types to `peer_*` (7 fields, 1 type, 1 method)
- `[internal]` Removed `Role::opponent_id()`, all call sites use `1 - role.player_id()` with TODO markers
- `[internal]` Renamed `DraftBan::opponent_bans` to `peer_bans` throughout
- `[internal]` Migrated 14 source files to array-indexed `players[local]`/`players[peer]` access pattern
- `[docs]` Wrote 9-task implementation plan at `docs/superpowers/plans/2026-04-07-array-indexed-playerstate.md`
- `[tooling]` Cleaned up 4 stale worktrees and branches from previous session

### Session Handoff — Array-Indexed PlayerState Implementation

**Git State:** branch `main`, 2 uncommitted doc changes (PLANNING, TASKS), pushed to origin at `4b6989d`
**Tests:** No test suite

**Work Completed:**
- Wrote 9-task implementation plan using writing-plans skill
- Executed all 9 tasks via subagent-driven development (sonnet model for implementation)
- Task 1: MatchProgress core restructure (players array, new constructor, remove accessors, apply_peer_build)
- Task 2: Net layer rename (opponent_* → peer_*, OpponentBuildData → PeerBuildData)
- Task 3: Role::opponent_id removal, DraftBan field rename
- Tasks 4-8: Call site migration across 12 files (context, main, battle, waiting, round_result, phase_ui, ui, rendering, build_phase, economy, draft_ban, game_over)
- Task 9: Final verification — zero errors, zero new clippy warnings
- Final code review caught one missed rename (chat.rs opponent_id parameter → peer_id), fixed
- Cleaned up 4 stale worktrees and their branches

**In Progress:**
- Nothing — clean stopping point

**Decisions Made:**
- Subagent-driven development (not worktrees) for sequential tasks — confirmed previous feedback that worktrees cause merge conflicts
- Sonnet model sufficient for mechanical migration tasks — all 8 implementer subagents completed successfully
- Spec reviewer dispatched only for final review (not per-task) — mechanical tasks with literal code blocks don't need per-task spec review
- Guest perspective LP flip logic preserved as-is — pre-existing pattern, works correctly, flagged as future cleanup target

**Blockers:**
- None

**Next Steps:**
1. R key to rotate packs (small gameplay feature)
2. Pause/options menu
3. Free camera rotation (future)

### Session Handoff — Array-Indexed PlayerState Design

**Git State:** branch `main`, 2 uncommitted doc changes (PLANNING, TASKS), ahead of origin by 1 commit (`7aa8f58`), spec committed
**Tests:** No test suite

**Work Completed:**
- Audited all perspective-relative patterns across codebase (opponent_*, my_*, player/opponent accessors) — 15 files affected
- Brainstormed and approved design: `players: [PlayerState; 2]` replacing `host`/`guest` fields
- Decided on net layer approach: rename opponent_* → peer_* (transient buffers), no structural networking changes
- Wrote and committed design spec with full translation table for call site migration

**In Progress:**
- Spec approved, needs implementation plan (invoke writing-plans skill next session)

**Decisions Made:**
- Fixed array `[PlayerState; 2]` chosen over Vec or HashMap — minimal change, upgrade to Vec later for N-player
- Net layer stays 1:1 with `peer_*` rename only — networking restructure deferred to future multi-peer work
- `1 - local_id` used as 2-player hack for peer index, marked with TODO for future N-player refactor
- `PlayerState::new(player_id)` replaces `new_host()`/`new_guest()` — default name becomes `"Player {id+1}"`
- User wants free camera rotation as future task — could replace current camera flip approach
- `apply_opponent_build` becomes free function `apply_peer_build(&mut PlayerState, &PeerBuildData, round)`

**Blockers:**
- None

**Next Steps:**
1. Write implementation plan for array-indexed PlayerState refactor (invoke writing-plans skill)
2. Execute the implementation plan
3. R key to rotate packs
4. Pause/options menu

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
