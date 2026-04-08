# Arbitrary Player IDs & Remaining Canonical Cleanup

**Date:** 2026-04-08
**Status:** Approved

## Goal

Eliminate all remaining hardcoded player_id assumptions (0/1, host/guest) by making player_id an arbitrary `u16` derived from the WebRTC PeerId. This forces truly canonical code — no array index tricks, no `players[0]`/`players[1]`, no `is_host`-based identity derivation.

## Core Principle

Player IDs are arbitrary numbers with no inherent meaning. Game code accesses player state by looking up an ID, never by indexing. If a piece of code assumes any particular player_id value, it is a bug.

## Non-Goals

- Multi-peer networking (NetState stays 1:1)
- Spectator mode
- Changing the signaling server or matchbox protocol

## Part 1: player_id type change — u8 to u16

Every `player_id` field, parameter, and variable across the codebase changes from `u8` to `u16`:

- `PlayerState.player_id: u16`
- `Unit.player_id: u16`
- `GameContext.local_player_id: u16`
- All NetMessage `player_id` fields
- `PeerBuildData.player_id: u16`
- `ChatMessage.player_id: u16`
- `team::set_color(player_id: u16, ...)`
- `team::team_color(player_id: u16)`
- `arena::deploy_x_range(player_id: u16)` — see Part 7 for how this works with arbitrary IDs
- All function parameters and local variables that hold player_id

## Part 2: Player ID assignment from PeerId

### Multiplayer

Each client derives its player_id from the first 2 bytes of its matchbox `PeerId` (a UUID):

```rust
fn player_id_from_peer(peer_id: &PeerId) -> u16 {
    let bytes = peer_id.0.as_bytes();
    u16::from_be_bytes([bytes[0], bytes[1]])
}
```

This happens during lobby connection. Each client knows its own PeerId (from the socket) and the peer's PeerId (from the connection event). Both player_ids are known before the game starts.

The lobby stores `local_player_id: u16` derived from its own PeerId, and sends it in NameSync/ColorChoice messages. The peer's player_id arrives in their messages.

### Single-player / AI

The human player gets a random u16. The AI player gets a different random u16:

```rust
let human_pid = macroquad::rand::gen_range(1000u16, 60000);
let ai_pid = loop {
    let id = macroquad::rand::gen_range(1000u16, 60000);
    if id != human_pid { break id; }
};
```

### Player ordering in MatchProgress

`MatchProgress.players: Vec<PlayerState>` — players are added in connection order. The Vec index has no meaning. All access is by player_id via lookup helpers.

## Part 3: MatchProgress — Vec with lookup helpers

### Data structure change

```rust
pub struct MatchProgress {
    pub round: u32,
    pub players: Vec<PlayerState>,
    pub banned_kinds: Vec<UnitKind>,
}
```

### Lookup helpers

```rust
impl MatchProgress {
    pub fn player(&self, pid: u16) -> &PlayerState {
        self.players.iter().find(|p| p.player_id == pid).unwrap()
    }

    pub fn player_mut(&mut self, pid: u16) -> &mut PlayerState {
        self.players.iter_mut().find(|p| p.player_id == pid).unwrap()
    }
}
```

These are NOT perspective accessors — they look up by canonical ID, not by "am I host or guest." Any code that needs a player's state asks for it by ID.

### is_game_over / game_winner

```rust
pub fn is_game_over(&self) -> bool {
    self.players.iter().any(|p| p.lp <= 0)
}

pub fn game_winner(&self) -> Option<u16> {
    let alive: Vec<_> = self.players.iter().filter(|p| p.lp > 0).collect();
    if alive.len() == 1 {
        Some(alive[0].player_id)
    } else if self.players.iter().all(|p| p.lp <= 0) {
        None // draw — everyone dead
    } else {
        None // game not over
    }
}
```

### MatchProgress::new()

Takes the player IDs as parameters instead of hardcoding:

```rust
pub fn new(player_ids: &[u16]) -> Self {
    Self {
        round: 1,
        players: player_ids.iter().map(|&pid| PlayerState::new(pid)).collect(),
        banned_kinds: Vec::new(),
    }
}
```

## Part 4: combat.rs — techs lookup by player_id

Replace `host_techs: &TechState, guest_techs: &TechState` parameters with a reference to the players vec:

```rust
// Before
pub fn update_attacks(units: &mut [Unit], projectiles: &mut Vec<Projectile>, dt: f32,
    host_techs: &TechState, guest_techs: &TechState, splash: &mut Vec<SplashEffect>)

// After
pub fn update_attacks(units: &mut [Unit], projectiles: &mut Vec<Projectile>, dt: f32,
    players: &[PlayerState], splash: &mut Vec<SplashEffect>)
```

Inside combat, tech lookup by unit's player_id:

```rust
fn techs_for(players: &[PlayerState], player_id: u16) -> &TechState {
    &players.iter().find(|p| p.player_id == player_id).unwrap().techs
}
```

## Part 5: RoundEnd message — per-player data keyed by ID

Replace hardcoded `alive_0`/`alive_1`/`timeout_dmg_0`/`timeout_dmg_1`/`total_hp_0`/`total_hp_1` with a vec of per-player entries:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundEndPlayerData {
    pub player_id: u16,
    pub alive_count: u16,
    pub total_hp: i32,
    pub timeout_damage: i32,
}

// In NetMessage:
RoundEnd {
    winner: Option<u16>,
    lp_damage: i32,
    loser: Option<u16>,
    per_player: Vec<RoundEndPlayerData>,
},
```

The host builds this by iterating all players. The guest reads per-player data by player_id.

`RoundEndData` (the internal struct) mirrors this:

```rust
pub struct RoundEndData {
    pub winner: Option<u16>,
    pub lp_damage: i32,
    pub loser: Option<u16>,
    pub per_player: Vec<RoundEndPlayerData>,
}
```

## Part 6: Colors canonical — stored on PlayerState

Add `color_index: u8` to `PlayerState`. Colors are set by each client on their own PlayerState, synced via ColorChoice messages.

```rust
pub struct PlayerState {
    pub player_id: u16,
    pub lp: i32,
    pub techs: TechState,
    pub name: String,
    pub next_id: u64,
    pub gold: u32,
    pub packs: Vec<PlacedPack>,
    pub ai_memory: AiMemory,
    pub color_index: u8,
}
```

`team::set_color` is called per-player from PlayerState data:

```rust
for player in &ctx.progress.players {
    team::set_color(player.player_id, player.color_index);
}
```

The lobby `host_color_index` field is replaced — each player's color is tracked on their PlayerState by player_id.

## Part 7: Deploy zones — assigned at game start, not hardcoded by ID

Deploy zones cannot be derived from an arbitrary player_id. Instead, they are assigned at game start and stored on PlayerState:

```rust
pub struct PlayerState {
    // ... existing fields ...
    pub deploy_zone: (f32, f32),  // (x_min, x_max)
}
```

The `arena::deploy_x_range(player_id)` free function is replaced by reading `player.deploy_zone`. At game start, the host assigns zones:

- First player to connect: left half `(0.0, HALF_W)`
- Second player: right half `(HALF_W, ARENA_W)`

In single-player: human gets left, AI gets right (or based on camera angle default).

In multiplayer, the host (room creator) assigns itself the left zone and the peer the right zone, communicated via SettingsSync. In single-player, human gets left, AI gets right.

## Part 8: economy.rs / spawn_ai_army — AI player_id as parameter

`spawn_ai_army` and `start_ai_battle` take the AI's player_id as a parameter:

```rust
pub fn spawn_ai_army(&mut self, ai_packs: &[PackDef], ai_player_id: u16) -> Vec<Unit>
```

No more `players[1]` hardcoding. The caller passes whichever player_id was assigned to the AI.

`start_ai_battle` similarly takes `ai_player_id: u16` and uses `self.player_mut(ai_player_id)` for all AI state access.

## Part 9: BuildState — next_id from PlayerState, no is_host

`BuildState::new` takes `next_id: u64` instead of `is_host: bool`:

```rust
pub fn new(gold: u32, next_id: u64) -> Self {
    Self {
        gold_remaining: gold,
        next_id,
        // ...
    }
}
```

The caller passes `progress.player(local_player_id).next_id`. The `is_host` parameter is eliminated.

## Part 10: GamePhase::GameOver — stores u16

`GamePhase::GameOver(u16)` — winner is an arbitrary player_id, not 0/1.

`GamePhase::RoundResult.loser_team` → `loser: Option<u16>`.

## Files Affected

| File | Changes |
|------|---------|
| `match_progress.rs` | Vec<PlayerState>, lookup helpers, new() takes &[u16], is_game_over/game_winner generic |
| `net.rs` | All player_id fields u16, RoundEnd per-player vec, PeerBuildData u16 |
| `context.rs` | local_player_id u16, start_game assigns zones + colors on PlayerState |
| `main.rs` | Color from PlayerState, deploy zone from PlayerState |
| `battle_phase.rs` | RoundEnd per-player data, canonical LP/damage, surrender uses game_winner |
| `combat.rs` | Replace host_techs/guest_techs with &[PlayerState], techs_for() lookup |
| `game_state.rs` | BuildState::new takes next_id, GamePhase u16 types |
| `arena.rs` | Remove deploy_x_range free function (deploy zone lives on PlayerState) |
| `economy.rs` | spawn_ai_army/start_ai_battle take ai_player_id parameter |
| `unit.rs` | player_id: u16 |
| `team.rs` | set_color/team_color take u16 |
| `phase_ui.rs` | All u16, lookup by player_id |
| `ui.rs` | draw_hud takes u16 |
| `rendering.rs` | u16 |
| `build_phase.rs` | Deploy zone from PlayerState, send_build_complete u16 |
| `round_result.rs` | u16 |
| `waiting_phase.rs` | u16 |
| `game_over.rs` | u16 |
| `chat.rs` | u16 |
| `lobby.rs` | Derive player_id from PeerId, assign deploy zones |
| `draft_ban.rs` | u16 |
| `sync.rs` | u16 in state hash if player_id is used |
| `projectile.rs` | player_id: u16 |
| `terrain.rs` | obstacle player_id: u16, color lookup |

## Testing

No automated tests. Manual verification:
- `cargo check` passes
- `cargo clippy` clean
- Single-player vs AI: build, battle, round progression, game over, rematch
- Multiplayer: lobby sync, draft/ban, build exchange, battle sync, surrender, rematch
- Verify player IDs are arbitrary (print to console on connect)
- Verify colors, names, deploy zones all work correctly for both host and guest
- Verify camera defaults still work (right-side player gets 180 degrees)
