# Canonical Player-ID System & Camera Rotation

**Date:** 2026-04-08
**Status:** Approved

## Goal

Eliminate all perspective-relative patterns (local/peer, me/them, host/guest Role enum) from game logic. Every piece of game code operates on canonical player_ids. The net layer tags incoming data with the sender's player_id. Replace the camera x-flip hack with proper angular rotation, enabling free camera control via Q/E keys.

## Core Principle

Game code never computes "who is the other player." It receives data that says "player 1 did X" and acts on it canonically. The only place that needs "which player am I" is camera angle default, "YOU WIN/LOSE" headline, and HUD ordering — these use `local_player_id`, a plain `u8`.

## Non-Goals

- Multi-peer networking (NetState stays 1:1)
- Changing network message format/protocol (messages already use canonical player_ids)
- Spectator mode implementation (spectator = `local_player_id = 255`, behavior unchanged)

## Part 1: Replace Role Enum with local_player_id

### Delete `src/role.rs`

The `Role` enum (Host/Guest/Spectator) is a perspective abstraction. It provides two things:
- `player_id() -> u8` — replaced by `local_player_id: u8` on GameContext
- `deploy_x_range() -> (f32, f32)` — replaced by a free function in `arena.rs`

### GameContext changes

```rust
// Before
pub struct GameContext {
    pub role: Role,
    // ...
}

// After
pub struct GameContext {
    pub local_player_id: u8,
    // ...
}
```

Default: `local_player_id: 0` (host). Set during `start_game` based on `is_host`.

### deploy_x_range free function

Add to `arena.rs`:

```rust
pub fn deploy_x_range(player_id: u8) -> (f32, f32) {
    match player_id {
        0 => (0.0, HALF_W),
        1 => (HALF_W, ARENA_W),
        _ => (0.0, 0.0), // spectator
    }
}
```

### Migration table

| Before | After |
|--------|-------|
| `ctx.role.player_id()` | `ctx.local_player_id` |
| `ctx.role.deploy_x_range()` | `deploy_x_range(ctx.local_player_id)` |
| `ctx.role == Role::Guest` | camera angle logic (Part 3) |
| `role` parameter on UI functions | `local_player_id: u8` parameter |

## Part 2: Canonical Display — Remove Perspective Translation

### battle_phase.rs guest round end

Remove `flipped_winner` / `flipped_loser` entirely. The host sends canonical player_ids in the RoundEnd message. Guest uses them directly:

```rust
// Before (broken — flips canonical IDs then compares against canonical)
let flipped_winner = rd.winner.map(|w| 1 - w);
let flipped_loser = rd.loser_team.map(|l| 1 - l);

// After — use canonical values directly
let final_state = match rd.winner {
    Some(w) => MatchState::Winner(w),
    None => MatchState::Draw,
};

// LP damage — index by canonical player_id
if rd.timeout_dmg_0 > 0 || rd.timeout_dmg_1 > 0 {
    ctx.progress.players[0].lp -= rd.timeout_dmg_0;
    ctx.progress.players[1].lp -= rd.timeout_dmg_1;
} else if let Some(loser) = rd.loser_team {
    ctx.progress.players[loser as usize].lp -= rd.lp_damage;
}

ctx.phase = GamePhase::RoundResult {
    match_state: final_state,
    lp_damage: rd.lp_damage,
    loser_team: rd.loser_team,
};
```

### battle_phase.rs desync check

```rust
// Before (flips counts — wrong with canonical coordinates)
if local_alive_0 != rd.alive_1 || local_alive_1 != rd.alive_0 {

// After — compare canonical counts directly
if local_alive_0 != rd.alive_0 || local_alive_1 != rd.alive_1 {
```

### phase_ui.rs — canonical name lookups

All name lookups use `progress.players[player_id as usize].name` directly:

```rust
// Winner name (draw_round_result_ui)
MatchState::Winner(tid) => {
    let winner_name = &progress.players[*tid as usize].name;
    // ...
}

// Loser name
if let Some(loser) = loser_team {
    let loser_name = &progress.players[loser as usize].name;
    // ...
}

// Obstacle owner (draw_battle_ui)
let team_name = if obs.player_id < 2 {
    &progress.players[obs.player_id as usize].name
} else {
    "Neutral"
};
```

### phase_ui.rs — "YOU WIN" / "YOU LOSE"

The one place that needs `local_player_id`:

```rust
let headline = if winner == local_player_id { "YOU WIN!" } else { "YOU LOSE!" };
```

### ui.rs — HUD ordering

First slot = local player, remaining slots = other players:

```rust
let local = &progress.players[local_player_id as usize];
// First: local player info
draw_player_lp(local);
// Then: other players
for (i, player) in progress.players.iter().enumerate() {
    if i != local_player_id as usize {
        draw_player_lp(player);
    }
}
```

### rendering.rs / phase_ui.rs — build overlays

"My packs" = packs in BuildState (local player's current build).
"Other players' packs" = iterate `progress.players`, skip `local_player_id`, draw their stored `.packs`.

## Part 3: Net Layer Tags Incoming Data with Player ID

### Principle

The `1 - local_player_id` derivation lives inside `NetState::poll()` — the only place that computes "messages from this connection come from player X." Game code receives data already tagged with the sender's canonical player_id.

### PeerBuildData

```rust
// Before
pub struct PeerBuildData {
    pub new_packs: Vec<(usize, (f32, f32), bool)>,
    pub tech_purchases: Vec<(UnitKind, TechId)>,
}

// After
pub struct PeerBuildData {
    pub player_id: u8,
    pub new_packs: Vec<(usize, (f32, f32), bool)>,
    pub tech_purchases: Vec<(UnitKind, TechId)>,
}
```

Set in `poll()` when the message arrives:

```rust
NetMessage::BuildComplete { new_packs, tech_purchases, .. } => {
    self.peer_build = Some(PeerBuildData {
        player_id: self.peer_player_id(),
        new_packs,
        tech_purchases,
    });
}
```

Where `peer_player_id()` is a helper:

```rust
fn peer_player_id(&self) -> u8 {
    if self.is_host { 1 } else { 0 }
}
```

### Other net fields

| Before | After |
|--------|-------|
| `peer_surrendered: bool` | `surrendered_player: Option<u8>` |
| `peer_rematch: bool` | `rematch_player: Option<u8>` |
| `peer_bans: Option<Vec<u8>>` | Unchanged (bans are merged, no player_id needed) |
| `peer_color: Option<u8>` | `peer_color: Option<(u8, u8)>` — `(player_id, color_index)` |
| `peer_name: Option<String>` | `peer_name: Option<(u8, String)>` — `(player_id, name)` |

### apply_peer_build signature change

The function receives `&mut MatchProgress` and indexes the correct player internally using the player_id embedded in the build data. The caller doesn't decide which PlayerState to target.

```rust
// Before — caller picks which PlayerState to pass in
pub fn apply_peer_build(player: &mut PlayerState, data: &PeerBuildData, round: u32) -> Vec<Unit>

// After — function uses data.player_id to find the right player
pub fn apply_peer_build(progress: &mut MatchProgress, data: &PeerBuildData) -> Vec<Unit> {
    let player = &mut progress.players[data.player_id as usize];
    let round = progress.round;
    // ... apply tech purchases and spawn packs on player
}
```

### Game code consumption (waiting_phase.rs)

```rust
if let Some(build_data) = n.take_peer_build() {
    let new_units = apply_peer_build(&mut ctx.progress, &build_data);
    let pid = build_data.player_id;
    ctx.units.retain(|u| u.player_id != pid);
    ctx.units.extend(ctx.progress.players[pid as usize].respawn_units());
    // ...
}
```

No computation of "who is the other player." The data says who it's from.

## Part 4: Camera Rotation Replacing X-Flip

### Remove x-flip hack

```rust
// Before
let x_flip = if ctx.role == role::Role::Guest { -1.0 } else { 1.0 };
let arena_camera = Camera2D {
    zoom: vec2(camera_zoom * 2.0 / screen_width() * x_flip, camera_zoom * 2.0 / screen_height()),
    ..
};

// After
let arena_camera = Camera2D {
    target: camera_target,
    zoom: vec2(camera_zoom * 2.0 / screen_width(), camera_zoom * 2.0 / screen_height()),
    rotation: camera_angle,
    ..Default::default()
};
```

### camera_angle state

Add to main loop state:

```rust
let mut camera_angle: f32 = 0.0;
```

Set on game start based on deploy side:

```rust
// Right-side builder defaults to 180 degrees (like sitting across the board)
camera_angle = if deploy_x_range(ctx.local_player_id).0 >= HALF_W { 180.0 } else { 0.0 };
```

### Q/E rotation controls

Smooth continuous rotation at 90 degrees/sec while held:

```rust
if is_key_down(KeyCode::Q) {
    camera_angle -= 90.0 * dt;
}
if is_key_down(KeyCode::E) {
    camera_angle += 90.0 * dt;
}
// Normalize to 0..360
camera_angle = camera_angle.rem_euclid(360.0);
```

Available in all non-lobby phases (same scope as existing zoom/pan controls).

### Input handling

`Camera2D::screen_to_world` accounts for rotation automatically. All clicks, drags, and hover work correctly at any angle without manual correction.

## Files Affected

| File | Changes |
|------|---------|
| `role.rs` | **Deleted** |
| `arena.rs` | Add `deploy_x_range(player_id: u8)` free function |
| `context.rs` | `role: Role` → `local_player_id: u8`, update `start_game` |
| `main.rs` | Camera rotation, remove x_flip, Q/E input, replace all role references |
| `net.rs` | Add `peer_player_id()`, tag incoming data with player_id, rename fields |
| `match_progress.rs` | Update `apply_peer_build` to read player_id from PeerBuildData |
| `battle_phase.rs` | Remove flipped_winner/loser, fix desync check, canonical LP damage |
| `phase_ui.rs` | Canonical name lookups, replace role param with local_player_id |
| `ui.rs` | HUD uses local_player_id for ordering, canonical data |
| `rendering.rs` | Build overlays use local_player_id, remove role param |
| `build_phase.rs` | Use local_player_id for tech/gold access |
| `round_result.rs` | Use local_player_id for gold save |
| `waiting_phase.rs` | Use player_id from tagged build data |
| `game_over.rs` | Replace role references with local_player_id |
| `draft_ban.rs` | Replace role references |
| `chat.rs` | Use local_player_id |

## Testing

No automated tests. Manual verification:
- `cargo check` passes
- `cargo clippy` clean
- Single-player vs AI: build, battle, round progression, game over, rematch
- Multiplayer: lobby sync, draft/ban, build exchange, battle sync, surrender, rematch
- **Specific regression**: guest sees correct winner/loser names at round end and game over
- **Camera**: Q/E rotation works smoothly, input (clicks, drags) remains accurate at all angles
- **Camera default**: host camera starts at 0 degrees, guest at 180 degrees
