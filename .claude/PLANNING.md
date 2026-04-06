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

- [ ] **PlayerState & host/guest refactor** — 11-task plan ready, execute via subagent-driven development
- [ ] R key to rotate packs (gameplay feature)
- [ ] Pause/options menu (gameplay feature)

## Roadmap

1. ~~Duplicate code consolidation~~ (done)
2. ~~Clippy cleanup + input centralization~~ (done)
3. **PlayerState & host/guest architecture** (core done — cleanup remaining: remove ArmyBuilder, slim BuildState, wire phase_ui)
4. Small gameplay features (R-to-rotate, pause menu)
5. Gameplay features (TBD)
