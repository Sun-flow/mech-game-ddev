# Planning

## Current Phase: Gameplay Features

**Goal:** Implement gameplay features now that codebase architecture is clean.

### Completed Phases

1. **Project Tooling Setup** — Claude workflow infrastructure, context documents, skills
2. **main.rs Decomposition (Round 1)** — Extract rendering, phase UI, chat, draft/ban from main.rs (2,249 → 1,158 lines)
3. **main.rs Decomposition (Round 2)** — Introduce GameContext/BattleState, extract all phase logic (1,158 → 302 lines)
4. **Duplicate Code Consolidation** — 6 patterns deduplicated across 7 files (pack spawn, ray-AABB, damage tracking, tiebreak, Vec2 conversions, lobby transitions)
5. **Clippy Cleanup + MouseState** — Fixed 26 clippy warnings, introduced `MouseState` struct centralizing all mouse input queries (32 → 5 warnings, remaining are `too_many_arguments`)

### Next Up

- [ ] More gameplay features (TBD)

## Roadmap

1. ~~Duplicate code consolidation~~ (done)
2. ~~Clippy cleanup + input centralization~~ (done)
3. ~~PlayerState & host/guest architecture~~ (done — Role enum, PlayerState struct, camera flip, canonical coordinates)
4. ~~Array-indexed PlayerState & perspective cleanup~~ (done — players array, peer_* rename, accessor removal)
5. ~~Canonical player-ID system~~ (done — Role deleted, local_player_id, camera rotation, sender-embedded player_id in net messages)
6. ~~Arbitrary player IDs~~ (done — u16 from PeerId, Vec<PlayerState> with lookup helpers, deploy_zone/color on PlayerState, per-player RoundEnd)
7. **Small gameplay features** (R-to-rotate, pause menu)
8. Multi-peer networking (future — canonical architecture ready)
