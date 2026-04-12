# Tasks

## Current Tasks

- [ ] **Tech panel UI rework** — move stats/tech panel from world-space to screen-space, lower-right corner, flush with borders, wide-and-flat layout instead of tall-and-thin
- [ ] **Verify multiplayer sync live** — both fixes applied (unit_ids in BuildComplete, canonical Vec ordering), needs a clean multiplayer test with sell/undo to confirm zero desyncs
- [ ] **Commit balance + sync + determinism work** — 14 modified source files + new `src/determinism.rs` + `balance/` folder, all uncommitted

## Backlog

- [ ] **Keybindings reference in pause menu** — Show a list of current keybindings somewhere in the escape menu (either as a sub-view like Settings, or inline). WASD/arrows = pan, Q/E = rotate, R = rotate pack, G = grid, Escape = menu, Enter = chat, etc.
- [ ] **Balance docs → implementation parity check** — `balance/units.md`, `balance/techs.md`, `balance/design-notes.md` describe all changes; verify code matches docs (should match, but no automated check exists)
- [ ] **Dynamic latency estimation** — `STATE_SEND_DEBOUNCE_FRAMES` is hardcoded at 60; could measure RTT and scale dynamically
- [ ] **Bidirectional state correction** — currently host-authoritative only; could allow guest → host state push too, with a tiebreaker rule
- [ ] **Entrench visual feedback tuning** — yellow glow may be too subtle against some team colors
- [ ] **New-round false-positive guard** — ensure sync protocol doesn't trigger during the first few frames after Battle phase start when pack spawn state might briefly differ
- [ ] **Centralize battle unit assembly** — extract a `prepare_battle_units()` helper called from both `waiting_phase.rs` and `round_result.rs` to reduce duplication and enforce canonical ordering in one place
- [ ] **Remove DEBUG_DUMP_FRAMES** — set to 0 or remove entirely once sync is verified stable (currently 30, adds verbose logging to first 30 frames of each battle)

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
- [x] PlayerState core implementation — Role enum, player_id rename, PlayerState struct, MatchProgress restructure, camera flip, deploy zone parameterization, sync mirroring removed (Tasks 1-10)
- [x] Array-indexed PlayerState — Replaced host/guest with `players: [PlayerState; 2]`, removed perspective accessors, renamed net opponent_* → peer_*, extracted apply_peer_build free function
- [x] Canonical player-ID system — Deleted Role enum, replaced with local_player_id u8. Camera rotation replacing x-flip. Sender-embedded player_id in net messages. Removed flipped_winner/loser bug.
- [x] Arbitrary player IDs — player_id u8→u16 from PeerId. Vec<PlayerState> with lookup helpers. Deploy zone/color on PlayerState. Per-player RoundEnd. HashMap team colors. Combat takes &[PlayerState] for techs.
- [x] Free camera rotation — Q/E smooth rotation at 90 deg/sec, camera angle replaces x-flip mirror
- [x] Escape menu — Resume/Settings/Surrender during all match phases, single-player pause, replaces old surrender confirm
- [x] WASD/arrow camera panning — relative to screen orientation, works at any rotation angle
- [x] R to rotate packs — alternative to middle-click during build phase
- [x] **Balance pass proposal (balance/)** — full unit/tech rebalance proposal with rationales, matchups, design notes
- [x] **Balance pass implementation** — all 13 unit stat changes, 8 new techs (3 generic + 5 unit-specific), Sniper T3→T1, pack resizes, Berserker rage formula, every tech's apply_to_stats or combat.rs behavioral hook
- [x] **Determinism test harness** — `src/determinism.rs`, 18 tests covering basic scenarios, Entrench verification, multiplayer sync with simulated latency, complex battles with techs
- [x] **HashMap audit** — verified no iteration-order non-determinism in combat paths
- [x] **Multiplayer sync protocol rewrite** — rollback+replay on state correction, bidirectional hash exchange every frame, proactive host push, debounce
- [x] **FIXED: duplicate unit pack on guest** — root cause was `BuildComplete` not sending `unit_ids`, so sell/undo gaps caused ID drift between host and guest. Fix: include unit_ids in BuildComplete, use `respawn_pack_units` in `apply_peer_build`.
- [x] **FIXED: mid-battle desync from Vec ordering** — host and guest assembled `ctx.units` in different order (local-first vs peer-first), causing different projectile creation order. Fix: canonical sort by ID at battle start + per-frame sort in `run_one_frame`.
- [x] **Kind/stats validation in sync** — `apply_and_fast_forward` now warns on kind mismatch (never overwrites) and validates stats against kind+techs derivation.

## Session Log

_(Session handoff entries will be appended here by `/handoff`)_
