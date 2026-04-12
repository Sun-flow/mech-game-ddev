# Planning

## Current Phase: Balance & Multiplayer Robustness

**Goal:** Bring the unit roster into a balanced state across tiers and ensure multiplayer stays synced under real network conditions.

### Completed Phases

1. **Project Tooling Setup** — Claude workflow infrastructure, context documents, skills
2. **main.rs Decomposition (Round 1)** — Extract rendering, phase UI, chat, draft/ban from main.rs (2,249 → 1,158 lines)
3. **main.rs Decomposition (Round 2)** — Introduce GameContext/BattleState, extract all phase logic (1,158 → 302 lines)
4. **Duplicate Code Consolidation** — 6 patterns deduplicated across 7 files
5. **Clippy Cleanup + MouseState** — Fixed 26 clippy warnings, introduced `MouseState` struct
6. **PlayerState & canonical player IDs** — Role enum, u16 player IDs, Vec<PlayerState>, sender-embedded net messages, deploy zones on PlayerState, per-player RoundEnd
7. **Escape menu + camera controls** — Escape menu, WASD/arrow panning, Q/E rotation, R pack rotation
8. **Codebase consolidation** — UI helpers (point_in_rect, draw_button, draw_centered_text), `other_players()` iterator, ~200 lines of duplicate code eliminated
9. **Balance pass** — Full unit/tech rebalance. Sniper moved T3→T1, 8 new techs added (5 unit-specific + 3 generic), every unit's stats tuned, proposal docs under `balance/`
10. **Multiplayer sync protocol rewrite** — Rollback+replay recovery, bidirectional hash exchange every frame, proactive host state push on mismatch detection, debounce to prevent flood

### Next Up

- [ ] **Tech panel UI rework** — screen-space, lower-right, wide-and-flat layout
- [ ] **Live verification of sync protocol** — test with sell/undo to confirm zero desyncs
- [ ] More gameplay features (TBD)

## Roadmap

1. ~~Duplicate code consolidation~~ (done)
2. ~~Clippy cleanup + input centralization~~ (done)
3. ~~PlayerState & host/guest architecture~~ (done)
4. ~~Array-indexed PlayerState & perspective cleanup~~ (done)
5. ~~Canonical player-ID system~~ (done)
6. ~~Arbitrary player IDs~~ (done)
7. ~~Escape menu, camera controls, codebase consolidation~~ (done)
8. ~~Balance pass: units + techs~~ (done, `balance/` folder for specs)
9. ~~Deterministic lockstep sync with rollback+replay recovery~~ (done, 18 tests in `src/determinism.rs`)
10. **Live verification** (current) — two-instance testing of new sync protocol under real network conditions
11. Multi-peer networking (future — canonical architecture ready)
