# Planning

## Current Phase: Codebase Refinement

**Goal:** Continue improving code quality, address duplicate code patterns, and plan next features.

### Completed Phases

1. **Project Tooling Setup** — Claude workflow infrastructure, context documents, skills
2. **main.rs Decomposition (Round 1)** — Extract rendering, phase UI, chat, draft/ban from main.rs (2,249 → 1,158 lines)
3. **main.rs Decomposition (Round 2)** — Introduce GameContext/BattleState, extract all phase logic (1,158 → 302 lines)

### Next Up

- [ ] Address duplicate code patterns from condensation report (pack spawning, damage tracking, ray-AABB intersection)
- [ ] PlayerState struct refactor (see backlog)

## Roadmap

1. Duplicate code consolidation (pack spawning × 4 locations, damage tracking × 3, ray-AABB × 2)
2. PlayerState struct — unify per-player state
3. Gameplay features (TBD)
