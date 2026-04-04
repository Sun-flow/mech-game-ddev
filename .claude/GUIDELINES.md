# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build              # Debug build
cargo run                # Debug build + run
cargo build --release    # Release build
cargo run --release      # Release build + run
```

No test suite exists. Testing is done manually by running the game.

## Project Overview

**mech-game-ddev** is a 2-player RTS arena auto-battler written in Rust using **macroquad 0.4** for rendering and **matchbox_socket** (WebRTC) for peer-to-peer multiplayer. Players draft/ban units, build armies by placing unit packs in a deploy zone, then watch real-time combat play out over multiple rounds.

## Architecture

The game runs a fixed-timestep loop (60 FPS) in `main.rs` driven by a **phase state machine**:

```
Lobby → DraftBan → Build → WaitingForOpponent → Battle → RoundResult → (next round or GameOver)
```

### Key Modules

- **main.rs** — Game loop, phase transitions, rendering, input handling. This is the largest file (~3000+ lines) and acts as the central orchestrator.
- **game_state.rs** — `GamePhase` enum and `BuildState` (army placement state during build phase).
- **combat.rs** — Targeting (LOS + pathfinding fallback), movement, attack execution, projectile updates.
- **unit.rs** — `Unit` struct, `UnitKind` enum (12 types: Striker, Sentinel, Ranger, Scout, Bruiser, Artillery, Chaff, Sniper, Skirmisher, Dragoon, Berserker, Shield), stats and abilities.
- **terrain.rs** — Obstacles (walls/cover), `NavGrid` for A* pathfinding, collision and line-of-sight checks.
- **net.rs** — Network message types (`NetMsg` enum) and `NetState` managing the WebRTC socket.
- **sync.rs** — Deterministic state hashing (frame hash every 4 frames) and full state serialization for desync recovery.
- **pack.rs** — `PackDef` definitions grouping units into purchasable packs with gold costs.
- **tech.rs** — 15 technologies (3 universal + 12 unit-specific) that modify unit stats at spawn time.
- **match_progress.rs** — Round tracking, LP (life points), gold economy, AI memory of opponent compositions.
- **economy.rs** — AI army building (random and smart counter-picking strategies).
- **lobby.rs** — Lobby UI, multiplayer room creation/joining, settings sync.
- **shop.rs** — Shop UI for purchasing unit packs during build phase.
- **settings.rs** — `GameSettings` (gameplay toggles) and `MainSettings` (UI scale).
- **team.rs** — Team color system (6 options).

### Multiplayer Networking

- Peer-to-peer via WebRTC (matchbox_socket). Player 1 (host) is authoritative for settings.
- State is serialized with **serde + bincode**.
- Desync detection: both peers hash game state every 4 frames. On mismatch, the guest requests full state sync from the host.
- Network messages include: settings sync, build completion, ban selections, chat, surrender, rematch, state hash/sync.

### Determinism

Combat must be deterministic across peers. Key constraints:
- Fixed timestep (1/60s), no floating-point-dependent randomness during battle.
- A* pathfinding on a shared `NavGrid` (10px cells).
- All combat state changes go through the same update functions on both peers.

### Arena & UI

- Arena size: 1680×960. Left half = Player 1 deploy zone, right half = Player 2.
- UI scales relative to window width (reference width: 1400px).
- Camera supports zoom and pan during battle.
