# Arbitrary Player IDs & Canonical Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make player_id an arbitrary u16 derived from WebRTC PeerId, eliminating all hardcoded 0/1 assumptions. Vec-based player storage with ID lookup replaces array indexing.

**Architecture:** `PlayerState.player_id` becomes `u16`, `MatchProgress.players` becomes `Vec<PlayerState>` with `player(pid)`/`player_mut(pid)` lookup helpers. Colors and deploy zones move onto PlayerState. Combat takes `&[PlayerState]` for tech lookup. Net RoundEnd uses per-player vec. Team color system uses HashMap keyed by u16. Player IDs derived from `socket.id().0.as_bytes()` in multiplayer, random in single-player.

**Tech Stack:** Rust, macroquad 0.4, matchbox_socket (WebRTC), matchbox_protocol (PeerId/Uuid)

**Spec:** `docs/superpowers/specs/2026-04-08-arbitrary-playerid-canonical-cleanup-design.md`

---

### Task 1: Core data model — PlayerState, MatchProgress, GamePhase

**Files:**
- Modify: `src/match_progress.rs`
- Modify: `src/game_state.rs`

- [ ] **Step 1: Update PlayerState**

Add `deploy_zone` and `color_index` fields. Change `player_id` to `u16`. Update constructor:

```rust
#[derive(Clone, Debug)]
pub struct PlayerState {
    pub player_id: u16,
    pub lp: i32,
    pub techs: TechState,
    pub name: String,
    pub next_id: u64,
    pub gold: u32,
    pub packs: Vec<PlacedPack>,
    pub ai_memory: AiMemory,
    pub deploy_zone: (f32, f32),
    pub color_index: u8,
}

impl PlayerState {
    pub fn new(player_id: u16) -> Self {
        Self {
            player_id,
            lp: STARTING_LP,
            techs: TechState::new(),
            name: format!("Player {}", player_id),
            next_id: player_id as u64 * 100_000 + 1,
            gold: 0,
            packs: Vec::new(),
            ai_memory: AiMemory::default(),
            deploy_zone: (0.0, 0.0),
            color_index: 0,
        }
    }

    // respawn_units and lock_packs unchanged
}
```

- [ ] **Step 2: Update AiMemory**

Change `human_player_id` parameter to `u16`:

```rust
pub fn record_round(&mut self, player_units: &[Unit], human_player_id: u16, ai_won: bool) {
    let mut counts: HashMap<UnitKind, u32> = HashMap::new();
    for u in player_units.iter().filter(|u| u.player_id == human_player_id) {
        *counts.entry(u.kind).or_insert(0) += 1;
    }
    self.last_enemy_kinds = counts.into_iter().collect();
    self.last_result = ai_won;
}
```

- [ ] **Step 3: Update MatchProgress — Vec with lookup helpers**

```rust
#[derive(Clone, Debug)]
pub struct MatchProgress {
    pub round: u32,
    pub players: Vec<PlayerState>,
    pub banned_kinds: Vec<UnitKind>,
}

impl MatchProgress {
    pub fn new(player_ids: &[u16]) -> Self {
        Self {
            round: 1,
            players: player_ids.iter().map(|&pid| PlayerState::new(pid)).collect(),
            banned_kinds: Vec::new(),
        }
    }

    pub fn player(&self, pid: u16) -> &PlayerState {
        self.players.iter().find(|p| p.player_id == pid).unwrap()
    }

    pub fn player_mut(&mut self, pid: u16) -> &mut PlayerState {
        self.players.iter_mut().find(|p| p.player_id == pid).unwrap()
    }

    pub fn round_allowance(&self) -> u32 {
        200 * self.round
    }

    pub fn calculate_lp_damage(surviving_units: &[Unit], player_id: u16) -> i32 {
        let packs = all_packs();
        let mut total = 0i32;
        for unit in surviving_units {
            if !unit.alive || unit.player_id != player_id {
                continue;
            }
            if let Some(pack) = packs.iter().find(|p| p.kind == unit.kind) {
                let per_unit_value = pack.cost as f32 / pack.count() as f32;
                total += per_unit_value as i32;
            }
        }
        total
    }

    pub fn advance_round(&mut self) {
        self.round += 1;
    }

    pub fn is_game_over(&self) -> bool {
        self.players.iter().any(|p| p.lp <= 0)
    }

    pub fn game_winner(&self) -> Option<u16> {
        let alive: Vec<_> = self.players.iter().filter(|p| p.lp > 0).collect();
        if alive.len() == 1 {
            Some(alive[0].player_id)
        } else {
            None
        }
    }
}
```

- [ ] **Step 4: Update spawn_ai_army — take ai_player_id parameter**

```rust
pub fn spawn_ai_army(&mut self, ai_packs: &[PackDef], ai_player_id: u16) -> Vec<Unit> {
    let packs = all_packs();
    let mut new_units = Vec::new();
    let ai = self.player_mut(ai_player_id);

    let ai_center_x = crate::arena::HALF_W + (crate::arena::HALF_W / 2.0);
    let total_new = ai_packs.len();
    if total_new == 0 {
        return new_units;
    }

    let arena_h = crate::arena::ARENA_H;
    let spacing = arena_h / (total_new as f32 + 1.0);
    let round = self.round;

    let ai = self.player_mut(ai_player_id);
    for (pack_idx_in_build, pack_def) in ai_packs.iter().enumerate() {
        let pack_index = packs.iter().position(|p| p.name == pack_def.name).unwrap_or(0);
        let pack = &packs[pack_index];

        let center_y = spacing * (pack_idx_in_build as f32 + 1.0);
        let offset_x = macroquad::rand::gen_range(-50.0f32, 50.0);
        let center = vec2(
            (ai_center_x + offset_x)
                .clamp(crate::arena::HALF_W + 50.0, crate::arena::ARENA_W - 50.0),
            center_y,
        );

        let (spawned, ids) = crate::pack::spawn_pack_units(
            pack, center, false, ai.player_id, &ai.techs, &mut ai.next_id,
        );
        new_units.extend(spawned);

        ai.packs.push(PlacedPack {
            pack_index, center, unit_ids: ids, pre_drag_center: center,
            rotated: false, locked: true, round_placed: round,
        });
    }

    new_units
}
```

Note: There's a borrow issue — `self.player_mut` borrows `self` mutably, but we also read `self.round`. Fix by reading `round` before the mutable borrow. The implementer should handle this by storing `let round = self.round;` first, then getting the mutable reference.

- [ ] **Step 5: Update apply_peer_build**

Update the player_id type in the signature (data.player_id is now u16, progress.players is now Vec):

```rust
pub fn apply_peer_build(progress: &mut MatchProgress, data: &PeerBuildData) -> Vec<Unit> {
    let packs = all_packs();
    let mut new_units = Vec::new();
    let round = progress.round;
    let player = progress.player_mut(data.player_id);

    // ... rest identical, just uses the lookup helper instead of index
}
```

- [ ] **Step 6: Update GamePhase and GameState types**

In `src/game_state.rs`:

```rust
pub enum GamePhase {
    // ...
    GameOver(u16),  // was u8
    RoundResult {
        match_state: MatchState,
        lp_damage: i32,
        loser_team: Option<u16>,  // was Option<u8>
    },
    // ... rest unchanged
}
```

Update `BuildState::new` — replace `is_host: bool` with `next_id: u64`:

```rust
pub fn new(gold: u32, next_id: u64) -> Self {
    Self {
        gold_remaining: gold,
        placed_packs: Vec::new(),
        dragging: None,
        selected_pack: None,
        timer: BUILD_TIMER,
        next_id,
        round_tech_purchases: Vec::new(),
        undo_history: Vec::new(),
        packs_bought_this_round: 0,
        multi_dragging: Vec::new(),
        multi_drag_offsets: Vec::new(),
        multi_drag_pre_centers: Vec::new(),
        drag_box_start: None,
    }
}
```

- [ ] **Step 7: Commit**

```bash
git add src/match_progress.rs src/game_state.rs
git commit -m "refactor: PlayerState u16, MatchProgress Vec with lookup helpers, deploy_zone/color on PlayerState"
```

---

### Task 2: Entity types — u8 to u16

**Files:**
- Modify: `src/unit.rs`
- Modify: `src/projectile.rs`
- Modify: `src/terrain.rs`
- Modify: `src/rendering.rs` (SplashEffect)
- Modify: `src/sync.rs`
- Modify: `src/pack.rs`

Mechanical type change: every `player_id: u8` becomes `player_id: u16` in struct fields, function parameters, and constructors across these 6 files. Also update comments that say "0 = player, 1 = opponent" to just "player_id".

Key locations:
- `unit.rs`: `Unit.player_id`, `Unit::new()` param
- `projectile.rs`: `Projectile.player_id`, `Projectile::new()` param
- `terrain.rs`: `Obstacle.player_id`, `Obstacle::cover()` param, `blocks_projectile()` param, `ray_hits_blocking_obstacle()` param — update comment `// 255 = neutral, 0 = player, 1 = opponent` to `// u16::MAX = neutral`
- `rendering.rs`: `SplashEffect.player_id`
- `sync.rs`: `SyncUnit.player_id`, `SyncProjectile.player_id`, `SyncObstacle.player_id` — all u8 → u16
- `pack.rs`: `spawn_pack_units()` and `respawn_pack_units()` player_id params

- [ ] **Step 1: Change all player_id fields and params from u8 to u16**

For each file, find every `player_id: u8` and change to `player_id: u16`. In terrain.rs, change the neutral sentinel from `255` to `u16::MAX` (65535), and update the `wall()` constructor accordingly.

- [ ] **Step 2: Update MatchState::Winner**

In `src/arena.rs`, change `Winner(u8)` to `Winner(u16)`:

```rust
pub enum MatchState {
    InProgress,
    Winner(u16),
    Draw,
}
```

Also remove the `deploy_x_range` free function (deploy zones now live on PlayerState).

- [ ] **Step 3: Commit**

```bash
git add src/unit.rs src/projectile.rs src/terrain.rs src/rendering.rs src/sync.rs src/pack.rs src/arena.rs
git commit -m "refactor: player_id u8 to u16 across entity types"
```

---

### Task 3: Team color system — HashMap for arbitrary IDs

**Files:**
- Modify: `src/team.rs`

Replace the hardcoded `AtomicU8` statics with a thread-local `HashMap<u16, u8>`:

```rust
use macroquad::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

pub const TEAM_COLORS: &[Color] = &[
    Color::new(0.9, 0.2, 0.2, 1.0), // Red
    Color::new(0.2, 0.4, 0.9, 1.0), // Blue
    Color::new(0.2, 0.8, 0.3, 1.0), // Green
    Color::new(0.9, 0.8, 0.2, 1.0), // Yellow
];

thread_local! {
    static COLOR_OVERRIDES: RefCell<HashMap<u16, u8>> = RefCell::new(HashMap::new());
}

pub fn set_color(player_id: u16, index: u8) {
    COLOR_OVERRIDES.with(|c| c.borrow_mut().insert(player_id, index));
}

pub fn team_color(player_id: u16) -> Color {
    let options = crate::settings::TEAM_COLOR_OPTIONS;
    let override_idx = COLOR_OVERRIDES.with(|c| c.borrow().get(&player_id).copied());
    if let Some(idx) = override_idx {
        if (idx as usize) < options.len() {
            let (_, (r, g, b)) = options[idx as usize];
            return Color::new(r, g, b, 1.0);
        }
    }
    // Fallback: cycle through default colors by hash
    let fallback_idx = (player_id as usize) % TEAM_COLORS.len();
    TEAM_COLORS[fallback_idx]
}

pub fn color_index(player_id: u16) -> u8 {
    COLOR_OVERRIDES.with(|c| c.borrow().get(&player_id).copied().unwrap_or(255))
}

pub fn team_projectile_color(player_id: u16) -> Color {
    let base = team_color(player_id);
    Color::new(
        (base.r + 0.3).min(1.0),
        (base.g + 0.3).min(1.0),
        (base.b + 0.3).min(1.0),
        1.0,
    )
}
```

- [ ] **Step 1: Replace team.rs with the above implementation**

- [ ] **Step 2: Commit**

```bash
git add src/team.rs
git commit -m "refactor: team color system uses HashMap<u16, u8> for arbitrary player IDs"
```

---

### Task 4: Net messages — u16 types, RoundEnd per-player data

**Files:**
- Modify: `src/net.rs`

- [ ] **Step 1: Update all player_id fields from u8 to u16**

In `NetMessage` variants: `BuildComplete`, `ChatMessage`, `Surrender`, `RematchRequest`, `ColorChoice`, `NameSync` — all `player_id: u8` → `player_id: u16`.

`PeerBuildData.player_id: u16`.

- [ ] **Step 2: Replace RoundEnd with per-player data**

Add `RoundEndPlayerData` struct. Replace fixed fields with per-player vec:

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

Update `RoundEndData` (internal struct) to match:

```rust
#[derive(Clone, Debug)]
pub struct RoundEndData {
    pub winner: Option<u16>,
    pub lp_damage: i32,
    pub loser: Option<u16>,
    pub per_player: Vec<RoundEndPlayerData>,
}
```

- [ ] **Step 3: Update poll() for RoundEnd**

```rust
NetMessage::RoundEnd { winner, lp_damage, loser, per_player } => {
    self.received_round_end = Some(RoundEndData {
        winner, lp_damage, loser, per_player,
    });
}
```

- [ ] **Step 4: Update NetState fields**

Change `received_chats: Vec<(u8, String, String)>` → `Vec<(u16, String, String)>`.
Change `surrendered_player: Option<u8>` → `Option<u16>`.
Change `rematch_player: Option<u8>` → `Option<u16>`.
Change `peer_color: Option<(u8, u8)>` → `Option<(u16, u8)>`.
Change `peer_name: Option<(u8, String)>` → `Option<(u16, String)>`.

- [ ] **Step 5: Update send_build_complete**

Change `local_player_id: u8` → `local_player_id: u16`.

- [ ] **Step 6: Commit**

```bash
git add src/net.rs
git commit -m "refactor: net messages u16 player_id, RoundEnd per-player data"
```

---

### Task 5: Combat — techs from &[PlayerState]

**Files:**
- Modify: `src/combat.rs`

- [ ] **Step 1: Change update_attacks signature**

Replace `host_techs: &TechState, guest_techs: &TechState` with `players: &[crate::match_progress::PlayerState]`:

```rust
pub fn update_attacks(
    units: &mut [Unit],
    projectiles: &mut Vec<Projectile>,
    dt: f32,
    players: &[crate::match_progress::PlayerState],
    splash_effects: &mut Vec<crate::rendering::SplashEffect>,
) {
```

- [ ] **Step 2: Replace tech_for_player closure**

```rust
let tech_for_player = |player_id: u16| -> &TechState {
    &players.iter().find(|p| p.player_id == player_id).unwrap().techs
};
```

- [ ] **Step 3: Update any other u8 references in combat.rs**

The `player_id` parameter in the spawn closure at line ~623 changes from `u8` to `u16`.

- [ ] **Step 4: Commit**

```bash
git add src/combat.rs
git commit -m "refactor: combat takes &[PlayerState] for tech lookup by player_id"
```

---

### Task 6: Player ID assignment — lobby, context, net

**Files:**
- Modify: `src/net.rs` (add helper to derive player_id from PeerId)
- Modify: `src/lobby.rs`
- Modify: `src/context.rs`

- [ ] **Step 1: Add player_id_from_peer helper to net.rs**

```rust
/// Derive a u16 player_id from a matchbox PeerId (UUID).
pub fn player_id_from_peer(peer_id: &PeerId) -> u16 {
    let bytes = peer_id.0.as_bytes();
    u16::from_be_bytes([bytes[0], bytes[1]])
}
```

- [ ] **Step 2: Update NetState to track local_player_id**

Add `pub local_player_id: u16` to NetState. Set it when the socket assigns our ID:

In `poll()`, after the existing peer connection handling, add ID assignment from our own socket:

The lobby will call `net.socket.id()` to get our PeerId and derive the local_player_id. Add a method:

```rust
pub fn derive_local_player_id(&mut self) -> Option<u16> {
    self.socket.id().map(|pid| player_id_from_peer(&pid))
}
```

- [ ] **Step 3: Update lobby.rs**

Replace `is_room_creator`-based player_id derivation with PeerId derivation. In the `WaitingForPeer → Connected` transition:

```rust
// Derive player IDs from PeerIds
let my_pid = net.derive_local_player_id().unwrap_or(macroquad::rand::gen_range(1000, 60000));
net.local_player_id = my_pid;

net.send(crate::net::NetMessage::NameSync { player_id: my_pid, name: self.player_name.clone() });
net.send(crate::net::NetMessage::ColorChoice { player_id: my_pid, color_index: game_settings.player_color_index });
```

Also update the color change send in the draw method to use the same PeerId-derived ID.

Replace `host_color_index: Option<u8>` with tracking by player_id. Since the peer's color arrives via `ColorChoice { player_id, color_index }`, it can be applied to `PlayerState.color_index` during `start_game`.

- [ ] **Step 4: Update context.rs start_game**

```rust
pub fn start_game(
    &mut self,
    net: Option<net::NetState>,
    is_host: bool,
    player_name: String,
    draft_ban_enabled: bool,
    local_player_id: u16,
    peer_player_id: Option<u16>,
) {
    self.net = net;
    self.local_player_id = local_player_id;

    // Build player ID list
    let mut player_ids = vec![local_player_id];
    if let Some(ppid) = peer_player_id {
        player_ids.push(ppid);
    }

    self.progress = MatchProgress::new(&player_ids);

    // Set names
    self.progress.player_mut(local_player_id).name = player_name;
    if let Some(ref n) = self.net {
        if let Some((pid, name)) = n.peer_name.clone() {
            self.progress.player_mut(pid).name = name;
        }
    }

    // Assign deploy zones — host gets left, peer gets right
    let (left, right) = ((0.0, crate::arena::HALF_W), (crate::arena::HALF_W, crate::arena::ARENA_W));
    if is_host {
        self.progress.player_mut(local_player_id).deploy_zone = left;
        if let Some(ppid) = peer_player_id {
            self.progress.player_mut(ppid).deploy_zone = right;
        }
    } else {
        self.progress.player_mut(local_player_id).deploy_zone = right;
        if let Some(ppid) = peer_player_id {
            self.progress.player_mut(ppid).deploy_zone = left;
        }
    }

    // Set colors from received data
    if let Some(ref n) = self.net {
        self.progress.player_mut(local_player_id).color_index = self.game_settings.player_color_index;
        if let Some((pid, color_idx)) = n.peer_color {
            self.progress.player_mut(pid).color_index = color_idx;
        }
    } else {
        self.progress.player_mut(local_player_id).color_index = self.game_settings.player_color_index;
    }

    // Initialize gold
    let allowance = self.progress.round_allowance();
    self.progress.player_mut(local_player_id).gold = allowance;

    let next_id = self.progress.player(local_player_id).next_id;
    self.build = BuildState::new(allowance, next_id);

    if let Some(ref mut n) = self.net {
        n.is_host = is_host;
    }

    if draft_ban_enabled {
        self.phase = GamePhase::DraftBan {
            bans: Vec::new(),
            confirmed: false,
            peer_bans: None,
        };
    } else {
        self.phase = GamePhase::Build;
    }
}
```

Update `GameContext::new()`: `local_player_id: 0` (temporary, set properly on start_game).

- [ ] **Step 5: Update context.rs field type**

```rust
pub local_player_id: u16,
```

- [ ] **Step 6: Commit**

```bash
git add src/net.rs src/lobby.rs src/context.rs
git commit -m "refactor: player ID from PeerId, start_game assigns deploy zones and colors"
```

---

### Task 7: Migrate main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update local_player_id type to u16**

All references to `ctx.local_player_id` are already u16 from Task 6.

- [ ] **Step 2: Update start_game calls**

The lobby `StartMultiplayer` paths need to pass `local_player_id` and `peer_player_id`:

```rust
lobby::LobbyResult::StartMultiplayer => {
    let is_host = lobby.is_room_creator;
    let net = lobby.net.as_ref().unwrap();
    let local_pid = net.local_player_id;
    // Peer player_id comes from received messages — derive from the peer's PeerId
    let peer_pid = net.peer_id.map(|pid| net::player_id_from_peer(&pid));
    ctx.start_game(lobby.net.take(), is_host, lobby.player_name.clone(), ctx.game_settings.draft_ban_enabled, local_pid, peer_pid);
    camera_angle = if ctx.progress.player(ctx.local_player_id).deploy_zone.0 >= arena::HALF_W { 180.0 } else { 0.0 };
    continue;
}
```

The `StartVsAi` path generates random IDs:

```rust
lobby::LobbyResult::StartVsAi => {
    let human_pid = macroquad::rand::gen_range(1000u16, 60000);
    let ai_pid = loop {
        let id = macroquad::rand::gen_range(1000u16, 60000);
        if id != human_pid { break id; }
    };
    ctx.start_game(None, true, lobby.player_name.clone(), ctx.game_settings.draft_ban_enabled, human_pid, Some(ai_pid));
    ctx.progress.player_mut(ai_pid).name = "AI".to_string();
    ctx.progress.player_mut(ai_pid).deploy_zone = (arena::HALF_W, arena::ARENA_W);
    camera_angle = 0.0;
    continue;
}
```

- [ ] **Step 3: Update color setup**

Set colors from PlayerState each frame:

```rust
for player in &ctx.progress.players {
    team::set_color(player.player_id, player.color_index);
}
```

- [ ] **Step 4: Update camera angle default**

Camera angle uses deploy zone from PlayerState:

```rust
camera_angle = if ctx.progress.player(ctx.local_player_id).deploy_zone.0 >= arena::HALF_W { 180.0 } else { 0.0 };
```

- [ ] **Step 5: Update deploy_x_range calls**

Replace `arena::deploy_x_range(ctx.local_player_id)` → `ctx.progress.player(ctx.local_player_id).deploy_zone` wherever used (none in main.rs currently, but verify).

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "refactor: main.rs uses arbitrary player IDs, colors from PlayerState"
```

---

### Task 8: Migrate battle_phase.rs

**Files:**
- Modify: `src/battle_phase.rs`

- [ ] **Step 1: Update update_attacks calls**

Replace `&ctx.progress.players[0].techs, &ctx.progress.players[1].techs` with `&ctx.progress.players` (two call sites — multiplayer and single-player):

```rust
update_attacks(
    &mut ctx.units,
    &mut battle.projectiles,
    FIXED_DT,
    &ctx.progress.players,
    &mut battle.splash_effects,
);
```

- [ ] **Step 2: Update RoundEnd send — per-player data**

Replace the hardcoded alive_0/alive_1/total_hp_0/total_hp_1 with per-player data:

```rust
let per_player: Vec<net::RoundEndPlayerData> = ctx.progress.players.iter().map(|p| {
    let pid = p.player_id;
    let alive_count = ctx.units.iter().filter(|u| u.alive && u.player_id == pid).count() as u16;
    let total_hp: i32 = ctx.units.iter().filter(|u| u.alive && u.player_id == pid).map(|u| u.hp as i32).sum();
    net::RoundEndPlayerData {
        player_id: pid,
        alive_count,
        total_hp,
        timeout_damage: 0, // set below for timeout
    }
}).collect();
```

For timeout: compute timeout damage per player (damage = sum of all OTHER players' surviving unit values):

```rust
let (lp_damage, loser_team, per_player) = if timed_out {
    let mut pp: Vec<net::RoundEndPlayerData> = ctx.progress.players.iter().map(|p| {
        let pid = p.player_id;
        let alive_count = ctx.units.iter().filter(|u| u.alive && u.player_id == pid).count() as u16;
        let total_hp: i32 = ctx.units.iter().filter(|u| u.alive && u.player_id == pid).map(|u| u.hp as i32).sum();
        // Timeout damage = count of opponent's surviving units
        let timeout_damage = ctx.units.iter().filter(|u| u.alive && u.player_id != pid).count() as i32;
        net::RoundEndPlayerData { player_id: pid, alive_count, total_hp, timeout_damage }
    }).collect();
    (0, None, pp)
} else {
    // ... winner/loser logic unchanged but use u16
};
```

Send:

```rust
n.send(net::NetMessage::RoundEnd {
    winner, lp_damage, loser: loser_team,
    per_player,
});
```

- [ ] **Step 3: Update guest round end — per-player data**

Replace the old `rd.timeout_dmg_0`/`rd.timeout_dmg_1` pattern with per-player lookup:

```rust
if let Some(rd) = n.received_round_end.take() {
    let final_state = match rd.winner {
        Some(w) => MatchState::Winner(w),
        None => MatchState::Draw,
    };

    // Desync check — compare per-player alive counts
    for pp in &rd.per_player {
        let local_alive = ctx.units.iter().filter(|u| u.alive && u.player_id == pp.player_id).count() as u16;
        if local_alive != pp.alive_count {
            eprintln!("[DESYNC] Player {} alive mismatch! Local: {} Host: {}", pp.player_id, local_alive, pp.alive_count);
        }
    }

    // Apply LP damage
    let has_timeout = rd.per_player.iter().any(|pp| pp.timeout_damage > 0);
    if has_timeout {
        for pp in &rd.per_player {
            ctx.progress.player_mut(pp.player_id).lp -= pp.timeout_damage;
        }
    } else if let Some(loser) = rd.loser {
        ctx.progress.player_mut(loser).lp -= rd.lp_damage;
    }

    battle.waiting_for_round_end = false;
    battle.show_surrender_confirm = false;
    ctx.phase = GamePhase::RoundResult {
        match_state: final_state,
        lp_damage: rd.lp_damage,
        loser_team: rd.loser,
    };
}
```

- [ ] **Step 4: Update host/single-player LP damage**

Replace indexed access with player_mut lookup:

```rust
if timed_out {
    for pp in &per_player {
        ctx.progress.player_mut(pp.player_id).lp -= pp.timeout_damage;
    }
} else if let Some(loser) = loser_team {
    ctx.progress.player_mut(loser).lp -= lp_damage;
}
```

- [ ] **Step 5: Update AI memory recording**

Replace hardcoded `players[1]`/`players[0]` with a loop or explicit player_id lookup. For single-player, the AI player_id needs to be known. Since we're in the battle_ended block and the AI is always the non-local player:

```rust
// Record AI memory — for each non-local player
for player in ctx.progress.players.iter_mut() {
    if player.player_id != ctx.local_player_id {
        let ai_won = match &final_state {
            MatchState::Winner(w) => *w == player.player_id,
            _ => false,
        };
        player.ai_memory.record_round(&ctx.units, ctx.local_player_id, ai_won);
    }
}
```

- [ ] **Step 6: Commit**

```bash
git add src/battle_phase.rs
git commit -m "refactor: battle_phase per-player RoundEnd, canonical LP damage"
```

---

### Task 9: Migrate economy.rs

**Files:**
- Modify: `src/economy.rs`

- [ ] **Step 1: Update start_ai_battle — take ai_player_id parameter**

```rust
pub fn start_ai_battle(
    units: &mut Vec<crate::unit::Unit>,
    projectiles: &mut Vec<crate::projectile::Projectile>,
    progress: &mut crate::match_progress::MatchProgress,
    obstacles: &mut Vec<crate::terrain::Obstacle>,
    nav_grid: &mut Option<crate::terrain::NavGrid>,
    game_settings: &crate::settings::GameSettings,
    ai_player_id: u16,
) -> crate::game_state::GamePhase {
    // ...
    units.retain(|u| u.player_id != ai_player_id);
    units.extend(progress.player(ai_player_id).respawn_units());

    let mut ai_gold = progress.round_allowance();
    ai_buy_techs(&mut ai_gold, &mut progress.player_mut(ai_player_id).techs);
    let ai_packs = if game_settings.smart_ai {
        smart_army(ai_gold, &progress.player(ai_player_id).ai_memory, &progress.banned_kinds)
    } else {
        random_army_filtered(ai_gold, &progress.banned_kinds)
    };
    let new_units = progress.spawn_ai_army(&ai_packs, ai_player_id);
    units.extend(new_units);
    // ... rest unchanged
}
```

- [ ] **Step 2: Commit**

```bash
git add src/economy.rs
git commit -m "refactor: economy takes ai_player_id parameter"
```

---

### Task 10: Migrate remaining files

**Files:**
- Modify: `src/build_phase.rs`
- Modify: `src/round_result.rs`
- Modify: `src/waiting_phase.rs`
- Modify: `src/phase_ui.rs`
- Modify: `src/ui.rs`
- Modify: `src/rendering.rs`
- Modify: `src/game_over.rs`
- Modify: `src/chat.rs`
- Modify: `src/draft_ban.rs`
- Modify: `src/arena.rs` (remove deploy_x_range function)

- [ ] **Step 1: build_phase.rs**

- Replace `ctx.progress.players[lpid]` with `ctx.progress.player(ctx.local_player_id)` / `ctx.progress.player_mut(ctx.local_player_id)`
- Replace `arena::deploy_x_range(ctx.local_player_id)` with `ctx.progress.player(ctx.local_player_id).deploy_zone`
- Update `send_build_complete` calls to pass `ctx.local_player_id` (u16)
- Update `start_ai_battle` calls to pass `ai_player_id` — need to find the AI player. For single-player, the AI is the non-local player:
```rust
let ai_pid = ctx.progress.players.iter().find(|p| p.player_id != ctx.local_player_id).unwrap().player_id;
economy::start_ai_battle(..., ai_pid);
```

- [ ] **Step 2: round_result.rs**

Replace `ctx.progress.players[lpid]` with `ctx.progress.player(ctx.local_player_id)` / `player_mut`. Replace other-player respawn loop to use `players.iter().filter(|p| p.player_id != ctx.local_player_id)`.

- [ ] **Step 3: waiting_phase.rs**

Already uses `build_data.player_id` — just ensure it's u16 compatible. Update `apply_peer_build` call (signature changed in Task 1). `progress.player(pid)` instead of `progress.players[pid as usize]`.

- [ ] **Step 4: phase_ui.rs**

All `local_player_id: u8` params → `u16`. All `progress.players[lpid]` → `progress.player(local_player_id)`. All `progress.players.iter().enumerate()` skip-local patterns stay the same but use `p.player_id != local_player_id` instead of `i != lpid`. All `*tid as usize` index lookups → `progress.player(*tid)`. All `loser as usize` → `progress.player(loser)`.

- [ ] **Step 5: ui.rs**

`local_player_id: u8` → `u16`. `progress.players[lpid]` → `progress.player(local_player_id)`. Other players loop unchanged.

- [ ] **Step 6: rendering.rs**

`local_player_id: u8` → `u16`. Replace `progress.players.iter().enumerate()` with `.iter().filter(|p| p.player_id != local_player_id)`.

- [ ] **Step 7: game_over.rs**

Replace `BuildState::new(allowance, is_host)` with `BuildState::new(allowance, ctx.progress.player(ctx.local_player_id).next_id)`. Replace `MatchProgress::new()` with `MatchProgress::new(&player_ids)` where player_ids are collected from the existing progress. All `u8` → `u16` for player_id types.

- [ ] **Step 8: chat.rs**

`player_id: u8` → `u16` in `ChatMessage` struct and all params.

- [ ] **Step 9: draft_ban.rs**

No player_id changes needed (bans are unit kinds, not player-keyed). Verify no u8 player_id references.

- [ ] **Step 10: arena.rs**

Remove the `deploy_x_range` function (deploy zones now live on PlayerState).

- [ ] **Step 11: Run `cargo check`**

Expected: Clean.

- [ ] **Step 12: Run `cargo clippy`**

Expected: Only pre-existing `too_many_arguments` warnings.

- [ ] **Step 13: Commit**

```bash
git add src/build_phase.rs src/round_result.rs src/waiting_phase.rs src/phase_ui.rs src/ui.rs src/rendering.rs src/game_over.rs src/chat.rs src/draft_ban.rs src/arena.rs
git commit -m "refactor: migrate remaining files to arbitrary u16 player_id system"
```

---

### Task 11: Final verification and cleanup

**Files:** All source files (verification only)

- [ ] **Step 1: Search for stale patterns**

```bash
grep -rn "player_id: u8" src/ --include="*.rs"
grep -rn "players\[0\]\|players\[1\]" src/ --include="*.rs"
grep -rn "is_host { 0\|is_host { 1\|is_room_creator { 0\|is_room_creator { 1" src/ --include="*.rs"
grep -rn "host_techs\|guest_techs" src/ --include="*.rs"
grep -rn "deploy_x_range" src/ --include="*.rs"
grep -rn "host_color_index" src/ --include="*.rs"
```

Expected: No matches.

- [ ] **Step 2: Run cargo check**

Expected: Clean.

- [ ] **Step 3: Run cargo clippy**

Expected: Only pre-existing warnings.

- [ ] **Step 4: Commit any cleanup**

```bash
git add -A
git commit -m "refactor: cleanup remaining hardcoded player_id patterns"
```
