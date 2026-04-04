# mech-game-ddev

A 2-player RTS arena auto-battler built in Rust with macroquad.

## Overview

Players draft and ban units, build armies by placing unit packs in a deploy zone, then watch real-time combat play out over multiple rounds. Supports single-player (vs AI) and peer-to-peer multiplayer via WebRTC.

## Build & Run

Requires [Rust](https://www.rust-lang.org/tools/install).

```bash
cargo run              # Debug build + run
cargo run --release    # Release build + run
```

## Tech Stack

- **macroquad 0.4** — 2D game engine / rendering
- **matchbox_socket** — WebRTC peer-to-peer networking
- **serde + bincode** — Network state serialization
- **tokio** — Async runtime

## Game Flow

```
Lobby → Draft/Ban → Build → Battle → Round Result → (next round or Game Over)
```

- **Lobby** — Create or join a multiplayer room, or start single-player vs AI
- **Draft/Ban** — Each player bans unit types from the match
- **Build** — Purchase and place unit packs in your deploy zone (timed)
- **Battle** — Real-time combat with A* pathfinding, line-of-sight, and tech upgrades
- **Round Result** — LP damage dealt, gold awarded, tech upgrades chosen

## Claude Code Skills

This project uses Claude Code with custom skills in `.claude/skills/`:

| Skill | Description |
|-------|-------------|
| `/condense-repo` | Identify duplicate code, unused files, and consolidation opportunities |
| `/update-docs` | Sync context documents (TASKS.md, PLANNING.md, CLAUDE.md, README.md) |
| `/handoff` | Session handoff — sync docs, summarize work, prepare context for next session |
