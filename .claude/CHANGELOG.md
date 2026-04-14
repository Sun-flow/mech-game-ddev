# Changelog

## 2026-04-13 — 2026-04-14

### Patch Notes

- `[ui]` Tech panel reworked — moved from upper-right tall column to bottom-right wide-and-flat bar with stats sidebar, wrapping tech cards (3 per row), and combat stats in the header row
- `[internal]` Added file-based logging via `log` + `simplelog` — debug builds log to `outputs/game-*.log`, release builds compile to no-ops via `release_max_level_off`
- `[internal]` Replaced all 24 `eprintln!` calls with `log` macros (`debug!`, `info!`, `warn!`) across battle_phase, sync, net, waiting_phase
- `[internal]` Log filenames include `MECH_LOG_NAME` env var for multi-instance disambiguation
- `[net]` Added `ReadyForBattle` sync barrier — both clients exchange ready signals before starting battle simulation, eliminating frame offset at battle start
- `[net]` Added `WaitingForBattleStart` game phase for the barrier handshake
- `[net]` Added time dilation — clients track peer frame number and scale dt ±5% when frame advantage exceeds ±3 frames, preventing drift
- `[net]` Added simulation step cap (`MAX_STEPS_PER_FRAME = 2`) — prevents burst stutter from frame bunching
- `[net]` Added accumulator clamp on first battle frame to prevent burst from barrier handshake latency
- `[net]` Increased `HASH_HISTORY_LEN` from 64 to 256 for better hash comparison coverage during drift

### Session Handoff — Tech Panel Rework, Logging, Frame Sync

**Git State:** branch `main`, 12 modified + 2 new files uncommitted
**Tests:** 18 passed, 0 failed

**Work Completed:**
- Tech panel UI fully reworked with visual companion brainstorming (mockups in `.superpowers/brainstorm/`)
- Design spec written at `docs/superpowers/specs/2026-04-13-tech-panel-rework-design.md`
- File-based logging system implemented (`log` + `simplelog` crates)
- Sync barrier (`ReadyForBattle` message + `WaitingForBattleStart` phase)
- Time dilation (peer frame tracking, ±5% dt scaling beyond ±3 frame threshold)
- Step cap (max 2 simulation ticks per render frame)
- Hash window increased to 256 frames
- Multiple multiplayer test sessions confirming zero desyncs

**In Progress:**
- Time dilation + step cap need multiplayer testing — code compiles and passes all 18 determinism tests, but stutter reduction not yet verified in-game

**Decisions Made:**
- Chose `log` + `simplelog` over custom logging module for ecosystem compatibility (matchbox_socket/rustls logs captured automatically) and future-proofing
- Chose time dilation over precomputed frames for frame sync — directly addresses drift rather than masking it, and remains input-friendly for future player commands during battle
- Chose sync barrier over input delay — zero latency cost, just 1 RTT at battle start hidden behind transition
- Tech panel layout: stats sidebar + wrapping tech cards (3 per row), combat stats in header row, 44% screen width

**Blockers:**
- None

**Next Steps:**
1. Test time dilation + step cap in multiplayer — verify stutter is reduced and `<missing>` hash entries decrease
2. Tune dilation parameters if needed (threshold, factor, step cap)
3. Commit all session work (tech panel, logging, sync improvements)

---

## 2026-04-09 — 2026-04-11 (multi-day session, continued)

### Patch Notes (2026-04-11 additions)

- `[fix]` Fixed unit ID drift between host and guest when player sells/undoes packs — `BuildComplete` now transmits `unit_ids` per pack; `apply_peer_build` uses received IDs via `respawn_pack_units` instead of generating sequential IDs
- `[fix]` Fixed mid-battle desync from Vec ordering — host had `[host_units, guest_units]`, guest had `[guest_units, host_units]`, causing different projectile creation order. Added canonical sort by ID at battle start + per-frame sort in `run_one_frame`
- `[fix]` Projectile hash now order-independent — sorted by `(attacker_id, pos)` before hashing in `compute_state_hash`
- `[net]` Kind mismatch warning in `apply_and_fast_forward` — logs `[SYNC WARNING]` if a unit's kind differs between local and snapshot (indicates ID drift bug, never overwrites)
- `[net]` Stats validation in `apply_and_fast_forward` — warns if unit stats don't match `kind + techs` derivation (detects silent tech-state drift)
- `[internal]` Canonical unit Vec ordering enforced at three levels: assembly in `waiting_phase.rs`/`round_result.rs`, and per-frame in `combat::run_one_frame`

### Patch Notes (original)


- `[balance]` Full unit balance pass — all 13 units restated; Chaff HP 120→100; Skirmisher 2.5→2.0 atk speed, range 180→160; Scout HP 500→350, speed 180→190, pack 2x3 kept with nerfed stats; Striker dmg 250→220; Bruiser HP 1700→1500, dmg 150→160, armor 20→25; Sentinel armor 80→60, HP 2000→2200, dmg 80→100, splash 15→20; Ranger range 350→300, dmg 180→170, HP 700→750, atk speed 0.7→0.75; Dragoon pack 5→4, HP 1000→1100, armor 40→35, splash removed; Berserker HP 900→1000, dmg 220→200, rage cap 3x→2.5x; Artillery HP 700→800, dmg 500→450, splash 40→45; Shield HP 1500→1300, armor 50→40, shield_hp 3000→2500, dmg 50→60
- `[balance]` Sniper moved from T3 (300g) to T1 (100g), dmg 1200→800, HP 400→500 — spammable anti-armor specialist
- `[balance]` New generic techs: HardenedFrame (+20% HP), Overdrive (+20% move speed), HighCaliber (+15% damage)
- `[balance]` New unit techs: ChaffFrenzy, ChaffExpendable (on-death atk-speed buff to nearby chaff), ChaffScavenge (spawn chaff on kill based on victim tier), Entrench (Skirmisher+Ranger stationary atk-speed stacks), SentinelTaunt (force-target aura), BerserkerDeathThroes (on-death splash), BerserkerUnstoppable (slow-immune <50% HP), BruiserCharge (2x on first attack after travel), ShieldReflect (15% damage reflected from barrier), ShieldFortress (+1500 barrier HP, immobile), InterceptorFlak (intercepted rockets detonate using rocket's own damage+splash), SniperStabilizer (min range 150→75)
- `[balance]` Existing tech tuning: RangeBoost +30→+40 (removed from Artillery/Sniper), StrikerRapidFire +0.5→+0.4, DragoonFortify +300/+20→+250/+15, ChaffOverwhelm +2→+3 per stack with 10-stack cap, SkirmisherSwarm removed (redundant with Overdrive)
- `[gameplay]` Entrench yellow-glow visual on stacked Skirmishers/Rangers using existing Berserker-rage tint pattern
- `[net]` **Deterministic lockstep sync with rollback+replay recovery** — host-authoritative; both peers hash and exchange every frame; host detects mismatches via guest's incoming hashes and proactively pushes full state (debounced); guest applies with rollback to snapshot frame + fast-forward via deterministic simulation replay
- `[net]` Removed broken `StateRequest` handshake and `apply_state_sync` (which replaced state without rolling back the frame counter — the root cause of compounding desyncs)
- `[net]` `StateHash` now one-per-frame in both directions; `received_state_hash: Option` → `received_state_hashes: Vec` to handle burst delivery
- `[internal]` Extracted `combat::run_one_frame` helper (used by normal battle loop and fast-forward catch-up)
- `[internal]` New `sync::apply_and_fast_forward` — deserialize snapshot, replace state (add missing units by ID, remove extras, update existing), rollback frame counter, replay forward via `combat::run_one_frame` until caught up
- `[internal]` New Unit fields: `spawn_pos` (Bruiser Charge), `stationary_timer` (Entrench), `has_charged` (Bruiser Charge), `expendable_stacks`/`expendable_timer` (Chaff Expendable) — wired into SyncUnit for network sync
- `[internal]` 18 determinism tests in `src/determinism.rs` (#[cfg(test)]) — basic scenarios, Entrench behavior verification, multiplayer sync harness with simulated latency, complex battles with techs, drift injection + recovery, high-latency stress
- `[internal]` Host-authoritative multiplayer sim harness exercises the real `compute_state_hash` and verifies rollback+replay produces identical host/guest state after injected drifts
- `[docs]` Balance proposal docs in `balance/`: `units.md` (stat tables + rationales), `techs.md` (tech tree + new techs), `design-notes.md` (matchup math + tier philosophy + open questions)
- `[docs]` Feedback memories saved: healing policy (self-heal OK, external heal banned), discussion-vs-implementation mode (don't code during debugging discussions)

### Session Handoff — Balance Pass + Deterministic Lockstep Sync

**Git State:** branch `main`, 9 modified source files + 1 new file (`src/determinism.rs`) + new `balance/` folder, not committed. Last commit `87982b0`.

**Tests:** `cargo test determinism` — **18 passed, 0 failed** (6 base determinism + 3 Entrench behavior + 5 multiplayer sync + 3 complex battle + 1 stress). Full build + clippy clean (3 pre-existing `too_many_arguments` warnings, unrelated).

**Work Completed:**

*Balance pass (proposal → implementation):*
- Full collaborative design pass over unit stats and techs
- Iterated through ~15 rounds of feedback (Scout damage kept, Berserker HP bumped to survive Sniper, Ranger range reduced, Entrench/Dig In consolidated, Chaff gets Hardened Frame + Overdrive, Sniper stays 1-unit-pack at T1, etc.)
- Wrote design docs in `balance/` first, then implemented in 6 sub-phases with verification between each
- Every new tech has either a stat-modifier path (`apply_to_stats`) or a behavioral path in `combat.rs`

*Determinism work:*
- Built `src/determinism.rs` as a `#[cfg(test)]` module with a headless combat harness
- Proved combat simulation is deterministic within a single process (tests clone state, seed RNG, run two identical runs, diff field-by-field)
- Tested separately: swarm scenarios (packed Skirmishers), tech-modified stats, Scavenge spawning, long battles, complex mixed-arms with techs
- Audited all HashMap usage — confirmed no iteration-order dependency in combat paths
- Verified Entrench actually works (packed Skirmishers accumulate `stationary_timer` to 2.999s after 3 sim seconds)

*Multiplayer sync investigation & rewrite:*
- Built a `MultiplayerSim` harness in the same test file that simulates network latency with pending-message queues
- Proved the original sync recovery was broken: every state correction applied host's stale snapshot to guest without rolling back the frame counter, creating permanent lag-sized drift that compounded into 500+ sync events per battle
- Researched multiplayer approaches (lockstep, rollback/GGPO, snapshot interpolation, split authority, CRDT) and chose deterministic lockstep with Factorio-style full resync recovery
- Implemented new protocol in both the harness (Phase 1) and production `battle_phase.rs` (Phase 2)
- Verified convergence in all injected-drift scenarios including complex mid-combat state (8 units dead, 2 damaged, drift on a live unit during active combat)

**In Progress:**
- Phase 3 of the multiplayer sync work is live verification — launching two instances and confirming no false positives in real play. Not yet done.

**Decisions Made:**
- **Healing policy:** Self-heal (BerserkerLifesteal) is OK. External heal (dedicated healer units, aura heals) is banned — concern about mandatory "must-have" picks.
- **Sync architecture:** Host-authoritative with rollback+replay. Bidirectional hash exchange every frame. Host pushes state proactively on mismatch, no request/response. Debounce = 12 frames (~200ms, covers typical good-internet RTT). Direct-copy harness (no WebRTC in tests).
- **Fast-forward semantics:** Catch-up simulation is synchronous within a single render frame. Zero wall-clock time. Player sees a single frame with the post-correction state, never intermediate replay frames.
- **Determinism tests in `src/determinism.rs`, not `tests/`:** Binary crate has no `[lib]` target, integration tests would require restructuring. `#[cfg(test)] mod determinism;` in `main.rs` gives test code full access to all modules.
- **Entrench threshold unchanged (2.0):** Initial worry about separation push was unfounded — pack grid gap (12.5) > separation min_dist (10.5), so no push activates at spawn and `stationary_timer` ticks freely.
- **ArtilleryBlastRadius kept** despite overlap with SplashBoost — enables "double-splash" Artillery build path as a 600g commitment.
- **Entrench consolidated** into one tech shared by Skirmisher + Ranger (was two separate techs).
- **Discussion-vs-implementation:** User correction during desync debugging — don't write code while exploring options. Wait for explicit go-ahead before editing.

**Blockers:**
- None. Ready for live multiplayer verification.

**Next Steps:**
1. **Launch 2 instances** and confirm no spurious `[DESYNC]` messages in logs during normal play
2. Verify Build → Battle transition doesn't trigger false desync detection in first few frames
3. Commit all changes once verified (there's ~9 modified files from an enormous session)
4. Clean up `outputs/` if it contains old state
5. Consider dynamic latency estimation for debounce (currently hardcoded to 12 frames / ~200ms)

### Session Handoff — Desync Root Causes Found & Fixed

**Git State:** branch `main`, 14 modified files + 1 new (`src/determinism.rs`) + `balance/` folder, not committed. Last commit `87982b0`.
**Tests:** `cargo test determinism` — **18 passed, 0 failed**

**Work Completed (2026-04-11 continuation):**
- Diagnosed unit ID drift bug via per-frame diagnostic dumps — `BuildComplete` was NOT sending `unit_ids`, so host generated sequential IDs while guest's had gaps from sell/undo. Fix: include unit_ids in message, use `respawn_pack_units` in `apply_peer_build`.
- Diagnosed mid-battle Vec ordering bug — host and guest assembled `ctx.units` in different order (local-first), causing projectile creation order and hash divergence at frame ~200. Fix: canonical sort by unit ID at assembly time + per-frame sort in `run_one_frame`.
- Added kind mismatch warning and stats validation to `apply_and_fast_forward` — detect-and-log, never overwrite. User's design principle: owning client is authoritative for unit identity.
- Sorted projectiles by `(attacker_id, pos)` in `compute_state_hash` for order-independent hashing.

**In Progress:**
- Tech panel UI rework requested — move from world-space to screen-space, lower-right, wide-and-flat layout. Not started.
- Live multiplayer verification of all fixes still pending (fix #1 verified via diagnostic logs — hashes matched for 4/5 rounds, 5th had the sell/undo bug which is now fixed; fix #2 not yet tested live).

**Decisions Made:**
- `BuildComplete` now includes `unit_ids: Vec<u64>` per pack — receiver uses these exact IDs instead of generating new ones.
- Unit Vec is sorted by ID at three levels: assembly (`waiting_phase.rs`, `round_result.rs`) and per-frame (`combat::run_one_frame`). pdqsort is O(n) on already-sorted input.
- Kind mismatches during sync are LOGGED, not overwritten — owning client is authoritative for unit kind. Overwriting would hide future bugs.
- Stats are validated against `kind + techs` derivation during sync — also logged, never overwritten.
- `STATE_SEND_DEBOUNCE_FRAMES` reduced from 60 to 12 (~200ms).
- `DEBUG_DUMP_FRAMES = 30` still active — to be disabled after clean live verification.

**Blockers:**
- None

**Next Steps:**
1. Tech panel UI rework (user's next request — screen-space, lower-right, wide-and-flat)
2. Live multiplayer verification with sell/undo scenario
3. Commit all changes (14 modified files + new files)
4. Centralize battle unit assembly into a shared helper (backlog)
5. Disable DEBUG_DUMP_FRAMES once sync is stable

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
