# Planning

## Current Phase: Codebase Refinement

**Goal:** Continue improving code quality and plan next features.

### Completed Phases

1. **Project Tooling Setup** — Claude workflow infrastructure, context documents, skills
2. **main.rs Decomposition (Round 1)** — Extract rendering, phase UI, chat, draft/ban from main.rs (2,249 → 1,158 lines)
3. **main.rs Decomposition (Round 2)** — Introduce GameContext/BattleState, extract all phase logic (1,158 → 302 lines)
4. **Duplicate Code Consolidation** — 6 patterns deduplicated across 7 files (pack spawn, ray-AABB, damage tracking, tiebreak, Vec2 conversions, lobby transitions)
5. **Clippy Cleanup + MouseState** — Fixed 26 clippy warnings, introduced `MouseState` struct centralizing all mouse input queries (32 → 5 warnings, remaining are `too_many_arguments`)

### Next Up

- [ ] **Array-indexed PlayerState** — Replace host/guest fields with `players: [PlayerState; 2]`, remove perspective-relative accessors, rename net opponent_* to peer_*. Spec approved: `docs/superpowers/specs/2026-04-06-array-indexed-playerstate-design.md`
- [ ] R key to rotate packs (gameplay feature)
- [ ] Pause/options menu (gameplay feature)
- [ ] Free camera rotation (future — decouples perspective from player identity)

## Roadmap

1. ~~Duplicate code consolidation~~ (done)
2. ~~Clippy cleanup + input centralization~~ (done)
3. ~~PlayerState & host/guest architecture~~ (done — Role enum, PlayerState struct, camera flip, canonical coordinates)
4. **Array-indexed PlayerState & perspective cleanup** (spec approved, plan next)
5. Small gameplay features (R-to-rotate, pause menu, free camera)
6. Multi-peer networking (future — depends on array-indexed state)
