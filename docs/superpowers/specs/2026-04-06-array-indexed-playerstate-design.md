# Array-Indexed PlayerState: Remove Perspective-Relative References

**Date:** 2026-04-06
**Status:** Approved

## Goal

Replace the `host`/`guest` named fields and all perspective-relative accessors (`opponent()`, `opponent_id()`, etc.) with a `[PlayerState; 2]` array indexed by `player_id`. This eliminates the player/opponent framing throughout the codebase, making game state access uniform and laying groundwork for future N-player support.

## Non-Goals

- Multi-peer networking (NetState stays 1:1)
- Free camera rotation (separate future task)
- Changing network message format or protocol

## Data Model Changes

### MatchProgress

```rust
// Before
pub struct MatchProgress {
    pub round: u32,
    pub host: PlayerState,
    pub guest: PlayerState,
    pub banned_kinds: Vec<UnitKind>,
}

// After
pub struct MatchProgress {
    pub round: u32,
    pub players: [PlayerState; 2],
    pub banned_kinds: Vec<UnitKind>,
}
```

### PlayerState

Replace `new_host()` / `new_guest()` with a single constructor:

```rust
pub fn new(player_id: u8) -> Self {
    Self {
        player_id,
        lp: STARTING_LP,
        techs: TechState::new(),
        name: format!("Player {}", player_id + 1),
        next_id: player_id as u64 * 100_000 + 1,
        gold: 0,
        packs: Vec::new(),
        ai_memory: AiMemory::default(),
    }
}
```

### Role

Remove `opponent_id()`. Keep `player_id()` and `deploy_x_range()`.

### Removed Accessors (MatchProgress)

All of these are deleted:
- `player(&self, role)` / `player_mut(&mut self, role)`
- `opponent(&self, role)` / `opponent_mut(&mut self, role)`
- `player_lp(&self, role)` / `opponent_lp(&self, role)`
- `apply_opponent_build(&mut self, data, role)`

## Net Layer Changes

Rename only — no structural changes to NetState:

| Before | After |
|--------|-------|
| `opponent_build` | `peer_build` |
| `opponent_surrendered` | `peer_surrendered` |
| `opponent_rematch` | `peer_rematch` |
| `opponent_bans` | `peer_bans` |
| `opponent_color` | `peer_color` |
| `opponent_name` | `peer_name` |
| `OpponentBuildData` | `PeerBuildData` |
| `take_opponent_build()` | `take_peer_build()` |

## Call Site Migration

### Index Pattern

```rust
let local = role.player_id() as usize;
// TODO: 2-player assumption — derive peer index from connection identity when supporting N players
let peer = (1 - role.player_id()) as usize;
```

### Translation Table

| Before | After |
|--------|-------|
| `progress.player(role)` | `progress.players[local]` |
| `progress.player_mut(role)` | `progress.players[local]` |
| `progress.opponent(role)` | `progress.players[peer]` |
| `progress.opponent_mut(role)` | `progress.players[peer]` |
| `progress.host` | `progress.players[0]` |
| `progress.guest` | `progress.players[1]` |
| `role.opponent_id()` | `1 - role.player_id()` |

### apply_opponent_build

Becomes a free function or method that takes `&mut PlayerState` directly instead of `&mut MatchProgress` + `Role`:

```rust
pub fn apply_peer_build(player: &mut PlayerState, data: &PeerBuildData, round: u32) -> Vec<Unit>
```

Called as: `apply_peer_build(&mut progress.players[peer], &build_data, progress.round)`

### GamePhase::DraftBan

`opponent_bans` field renamed to `peer_bans`.

### UI Display

UI files (`phase_ui.rs`, `ui.rs`) derive local variable names from indexed access:

```rust
let local_name = &progress.players[local].name;
let peer_name = &progress.players[peer].name;
```

The display logic (which name to show where) stays the same — just sourced from array indices instead of perspective accessors.

## Files Affected

1. `match_progress.rs` — Core restructure (PlayerState::new, MatchProgress fields, remove accessors, move apply_peer_build)
2. `role.rs` — Remove opponent_id()
3. `net.rs` — Rename opponent_* to peer_*, rename OpponentBuildData to PeerBuildData
4. `game_state.rs` — Rename opponent_bans in DraftBan variant
5. `context.rs` — Update start_game to use indexed access for peer name/color
6. `main.rs` — Update all progress.player()/opponent() calls, peer color mapping
7. `battle_phase.rs` — Update round end logic, LP damage application
8. `waiting_phase.rs` — Update peer build consumption
9. `phase_ui.rs` — Update display name derivation
10. `ui.rs` — Update LP bar display
11. `rendering.rs` — Update pack bounding box iteration
12. `draft_ban.rs` — Rename opponent_bans references
13. `game_over.rs` — Update DraftBan initialization
14. `chat.rs` — Update receive_from_net parameter naming
15. `lobby.rs` — No changes expected (operates before MatchProgress exists)

## Testing

No automated tests. Manual verification:
- `cargo check` passes
- `cargo clippy` clean
- Single-player vs AI: build, battle, round progression, game over, rematch
- Multiplayer: lobby sync, draft/ban, build exchange, battle sync, surrender, rematch
