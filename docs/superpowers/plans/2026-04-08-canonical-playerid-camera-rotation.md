# Canonical Player-ID System & Camera Rotation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate all perspective-relative patterns (Role enum, local/peer, flipped_winner) and replace with canonical player_id lookups. Replace camera x-flip with angular rotation + Q/E controls.

**Architecture:** Delete `role.rs`, add `local_player_id: u8` to GameContext, add `deploy_x_range(player_id)` free function to arena.rs. Network messages carry sender's player_id. Display code indexes `players[player_id]` directly. Camera uses `rotation` field instead of negative x-zoom.

**Tech Stack:** Rust, macroquad 0.4, matchbox_socket (WebRTC)

**Spec:** `docs/superpowers/specs/2026-04-08-canonical-playerid-camera-rotation-design.md`

---

### Task 1: Foundation — deploy_x_range, local_player_id, delete Role

**Files:**
- Modify: `src/arena.rs`
- Modify: `src/context.rs`
- Delete: `src/role.rs`
- Modify: `src/main.rs` (remove `mod role;`)

- [ ] **Step 1: Add deploy_x_range to arena.rs**

Add after the existing `shop_w()` function:

```rust
/// Deploy zone x-range for a given player_id.
pub fn deploy_x_range(player_id: u8) -> (f32, f32) {
    match player_id {
        0 => (0.0, HALF_W),
        1 => (HALF_W, ARENA_W),
        _ => (0.0, 0.0), // spectator
    }
}
```

- [ ] **Step 2: Replace Role with local_player_id in GameContext**

In `src/context.rs`, replace `use crate::role::Role;` with `use crate::arena;`. Change the struct field and update both methods:

```rust
use crate::arena;
use crate::chat;
use crate::game_state::{BuildState, GamePhase};
use crate::match_progress::MatchProgress;
use crate::net;
use crate::settings;
use crate::terrain;
use crate::unit::Unit;

pub struct GameContext {
    pub progress: MatchProgress,
    pub phase: GamePhase,
    pub build: BuildState,
    pub units: Vec<Unit>,
    pub net: Option<net::NetState>,
    pub obstacles: Vec<terrain::Obstacle>,
    pub nav_grid: Option<terrain::NavGrid>,
    pub game_settings: settings::GameSettings,
    pub show_grid: bool,
    pub local_player_id: u8,
    pub chat: chat::ChatState,
}

impl GameContext {
    pub fn start_game(
        &mut self,
        net: Option<net::NetState>,
        is_host: bool,
        player_name: String,
        draft_ban_enabled: bool,
    ) {
        self.net = net;
        self.local_player_id = if is_host { 0 } else { 1 };

        let mut peer_name = "Opponent".to_string();
        if let Some(ref mut n) = self.net {
            n.is_host = is_host;
            // peer_name is Option<(u8, String)> after Task 2 changes net.rs
            peer_name = n.peer_name.take().map(|(_, name)| name).unwrap_or_else(|| "Opponent".to_string());
        }

        self.progress = MatchProgress::new();

        // Set names using canonical player_id
        self.progress.players[self.local_player_id as usize].name = player_name;
        // Set peer name on all other players
        for (i, p) in self.progress.players.iter_mut().enumerate() {
            if i != self.local_player_id as usize {
                p.name = peer_name.clone();
            }
        }

        // Initialize gold with round allowance
        let allowance = self.progress.round_allowance();
        self.progress.players[self.local_player_id as usize].gold = allowance;

        self.build = BuildState::new(allowance, is_host);
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

    pub fn new() -> Self {
        let progress = MatchProgress::new();
        let allowance = progress.round_allowance();
        let build = BuildState::new(allowance, true);
        Self {
            progress,
            phase: GamePhase::Lobby,
            build,
            units: Vec::new(),
            net: None,
            obstacles: Vec::new(),
            nav_grid: None,
            game_settings: settings::GameSettings::default(),
            show_grid: false,
            local_player_id: 0,
            chat: chat::ChatState::new(),
        }
    }
}
```

Note: `n.peer_name` type changes to `Option<(u8, String)>` in Task 2 — if building incrementally, keep the old type here and update in Task 2. The `take()` pattern extracts the name regardless.

- [ ] **Step 3: Delete role.rs and remove mod declaration**

Delete `src/role.rs`. In `src/main.rs`, remove the line `mod role;`.

- [ ] **Step 4: Run `cargo check`**

Expected: Errors in all files that reference `Role`, `ctx.role`, or `role::`. These will be fixed in subsequent tasks.

- [ ] **Step 5: Commit**

```bash
git add src/arena.rs src/context.rs src/main.rs
git rm src/role.rs
git commit -m "refactor: replace Role enum with local_player_id, add deploy_x_range"
```

---

### Task 2: Net message format — sender-embedded player_id

**Files:**
- Modify: `src/net.rs`

- [ ] **Step 1: Update NetMessage variants**

Add `player_id: u8` to variants that need sender identification. Change `ChatMessage` from tuple to struct variant:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetMessage {
    ReadyToStart,
    SettingsSync(crate::settings::GameSettings),
    BuildComplete {
        player_id: u8,
        new_packs: Vec<(usize, (f32, f32), bool)>,
        tech_purchases: Vec<(UnitKind, TechId)>,
        gold_remaining: u32,
    },
    ChatMessage { player_id: u8, name: String, text: String },
    Surrender { player_id: u8 },
    RematchRequest { player_id: u8 },
    BanSelection(Vec<u8>),
    ColorChoice { player_id: u8, color_index: u8 },
    NameSync { player_id: u8, name: String },
    RoundEnd {
        winner: Option<u8>,
        lp_damage: i32,
        loser_team: Option<u8>,
        alive_0: u16,
        alive_1: u16,
        total_hp_0: i32,
        total_hp_1: i32,
        timeout_dmg_0: i32,
        timeout_dmg_1: i32,
    },
    StateHash { frame: u32, hash: u64 },
    StateRequest { frame: u32 },
    StateSync {
        frame: u32,
        units_data: Vec<u8>,
        projectiles_data: Vec<u8>,
        obstacles_data: Vec<u8>,
    },
}
```

- [ ] **Step 2: Update PeerBuildData**

```rust
#[derive(Clone, Debug)]
pub struct PeerBuildData {
    pub player_id: u8,
    pub new_packs: Vec<(usize, (f32, f32), bool)>,
    pub tech_purchases: Vec<(UnitKind, TechId)>,
}
```

- [ ] **Step 3: Update NetState fields**

```rust
pub struct NetState {
    pub socket: WebRtcSocket,
    pub message_loop: Pin<Box<dyn Future<Output = Result<(), matchbox_socket::Error>>>>,
    pub peer_id: Option<PeerId>,
    pub is_host: bool,
    pub peer_ready: bool,
    pub peer_build: Option<PeerBuildData>,
    pub disconnected: bool,
    pub received_chats: Vec<(u8, String, String)>, // (player_id, name, text)
    pub surrendered_player: Option<u8>,
    pub rematch_player: Option<u8>,
    pub peer_bans: Option<Vec<u8>>,
    pub received_settings: Option<crate::settings::GameSettings>,
    pub peer_color: Option<(u8, u8)>,       // (player_id, color_index)
    pub peer_name: Option<(u8, String)>,     // (player_id, name)
    pub received_round_end: Option<RoundEndData>,
    pub received_state_hash: Option<(u32, u64)>,
    pub received_state_request: Option<u32>,
    pub received_state_sync: Option<StateSyncData>,
}
```

- [ ] **Step 4: Update NetState::new() initializer**

```rust
peer_build: None,
// ...
received_chats: Vec::new(),
surrendered_player: None,
rematch_player: None,
peer_bans: None,
// ...
peer_color: None,
peer_name: None,
```

- [ ] **Step 5: Update poll() message handlers**

```rust
NetMessage::BuildComplete { player_id, new_packs, tech_purchases, gold_remaining: _ } => {
    self.peer_build = Some(PeerBuildData {
        player_id,
        new_packs,
        tech_purchases,
    });
}
NetMessage::ChatMessage { player_id, name, text } => {
    self.received_chats.push((player_id, name, text));
}
NetMessage::Surrender { player_id } => {
    self.surrendered_player = Some(player_id);
}
NetMessage::RematchRequest { player_id } => {
    self.rematch_player = Some(player_id);
}
NetMessage::BanSelection(bans) => {
    self.peer_bans = Some(bans);
}
NetMessage::SettingsSync(settings) => {
    self.received_settings = Some(settings);
}
NetMessage::ColorChoice { player_id, color_index } => {
    self.peer_color = Some((player_id, color_index));
}
NetMessage::NameSync { player_id, name } => {
    self.peer_name = Some((player_id, name));
}
```

- [ ] **Step 6: Update send_build_complete**

Add `local_player_id: u8` parameter:

```rust
pub fn send_build_complete(
    net: &mut Option<NetState>,
    build: &BuildState,
    local_player_id: u8,
) {
    if let Some(ref mut n) = net {
        let new_packs: Vec<(usize, (f32, f32), bool)> = build
            .placed_packs
            .iter()
            .filter(|p| !p.locked)
            .map(|p| (p.pack_index, (p.center.x, p.center.y), p.rotated))
            .collect();

        let tech_purchases = build.round_tech_purchases.clone();

        n.send(NetMessage::BuildComplete {
            player_id: local_player_id,
            new_packs,
            tech_purchases,
            gold_remaining: build.gold_remaining,
        });
    }
}
```

- [ ] **Step 7: Commit**

```bash
git add src/net.rs
git commit -m "refactor: add sender player_id to net messages, update NetState fields"
```

---

### Task 3: Update apply_peer_build signature

**Files:**
- Modify: `src/match_progress.rs`

- [ ] **Step 1: Change apply_peer_build to use MatchProgress + embedded player_id**

```rust
/// Apply peer's build data received over the network.
/// Uses the player_id embedded in the build data to find the correct PlayerState.
pub fn apply_peer_build(progress: &mut MatchProgress, data: &PeerBuildData) -> Vec<Unit> {
    let packs = all_packs();
    let mut new_units = Vec::new();
    let round = progress.round;
    let player = &mut progress.players[data.player_id as usize];

    // Apply tech purchases
    for &(kind, tech_id) in &data.tech_purchases {
        player.techs.purchase(kind, tech_id);
    }

    // Spawn peer's new packs (canonical coordinates)
    for &(pack_index, (cx, cy), rotated) in &data.new_packs {
        if pack_index >= packs.len() {
            continue;
        }
        let pack = &packs[pack_index];
        let center = vec2(cx, cy);

        let (spawned, ids) = crate::pack::spawn_pack_units(
            pack,
            center,
            rotated,
            player.player_id,
            &player.techs,
            &mut player.next_id,
        );
        new_units.extend(spawned);

        player.packs.push(PlacedPack {
            pack_index,
            center,
            unit_ids: ids,
            pre_drag_center: center,
            rotated,
            locked: true,
            round_placed: round,
        });
    }

    new_units
}
```

- [ ] **Step 2: Commit**

```bash
git add src/match_progress.rs
git commit -m "refactor: apply_peer_build uses embedded player_id from build data"
```

---

### Task 4: Migrate main.rs — camera rotation, color, chat

**Files:**
- Modify: `src/main.rs`

This is the largest single-file change. Remove `mod role;` (done in Task 1), remove x-flip, add camera_angle + Q/E, update color setup, chat, and all `ctx.role` references.

- [ ] **Step 1: Add camera_angle state and remove x_flip**

In the variable declarations at the top of main(), add `camera_angle`:

```rust
let mut camera_angle: f32 = 0.0;
```

Replace the camera construction (remove x_flip):

```rust
// Build the arena camera (used for all world-space rendering)
let arena_camera = Camera2D {
    target: camera_target,
    zoom: vec2(camera_zoom * 2.0 / screen_width(), camera_zoom * 2.0 / screen_height()),
    rotation: camera_angle,
    ..Default::default()
};
```

- [ ] **Step 2: Add Q/E rotation controls**

Inside the `if !matches!(ctx.phase, GamePhase::Lobby)` block, after the camera pan code (before the clamp), add:

```rust
// Q/E camera rotation (90 degrees/sec)
if is_key_down(KeyCode::Q) {
    camera_angle -= 90.0 * dt;
}
if is_key_down(KeyCode::E) {
    camera_angle += 90.0 * dt;
}
camera_angle = camera_angle.rem_euclid(360.0);
```

- [ ] **Step 3: Set camera_angle on game start**

After each `ctx.start_game(...)` call in the Lobby match arms, set the camera angle:

```rust
// After ctx.start_game(...) for multiplayer:
camera_angle = if arena::deploy_x_range(ctx.local_player_id).0 >= arena::HALF_W { 180.0 } else { 0.0 };

// After ctx.start_game(...) for VS AI:
camera_angle = 0.0; // host always starts at 0
```

- [ ] **Step 4: Update color setup**

Replace the color setup block:

```rust
// Set colors canonically: my color goes to my player_id slot
team::set_color(ctx.local_player_id, ctx.game_settings.player_color_index);
if let Some(ref n) = ctx.net {
    if let Some((pid, color_idx)) = n.peer_color {
        team::set_color(pid, color_idx);
    }
}
```

- [ ] **Step 5: Update build overlay and phase UI calls**

Replace all `ctx.role` with `ctx.local_player_id` in the render calls:

```rust
if is_build {
    rendering::draw_build_overlays(&ctx.build, &ctx.progress, mouse.world_mouse, ctx.local_player_id);
}
```

```rust
GamePhase::Build => {
    phase_ui::draw_build_ui(&ctx.build, &ctx.progress, &ctx.units, mouse.screen_mouse, &arena_camera, ctx.local_player_id);
}
GamePhase::WaitingForOpponent => {
    phase_ui::draw_waiting_ui(&ctx.progress, &ctx.build, ctx.local_player_id);
}
GamePhase::Battle => {
    phase_ui::draw_battle_ui(&ctx.progress, &ctx.units, &ctx.obstacles, battle.timer, battle_phase::ROUND_TIMEOUT, battle.show_surrender_confirm, mouse.screen_mouse, mouse.world_mouse, ctx.local_player_id);
}
GamePhase::RoundResult { match_state, lp_damage, loser_team } => {
    phase_ui::draw_round_result_ui(&ctx.progress, match_state, *lp_damage, *loser_team, ctx.local_player_id);
}
GamePhase::GameOver(winner) => {
    phase_ui::draw_game_over_ui(*winner, &ctx.progress, &ctx.units, mouse.screen_mouse, ctx.local_player_id);
}
```

- [ ] **Step 6: Update chat system**

Replace the chat block:

```rust
// Chat system — receive uses player_id from tagged messages
ctx.chat.receive_from_net(&mut ctx.net);
let my_name = ctx.progress.players[ctx.local_player_id as usize].name.clone();
ctx.chat.update(&ctx.phase, &mut ctx.net, &my_name, ctx.local_player_id);
ctx.chat.tick(dt);
ctx.chat.draw(&ctx.phase, &my_name);
```

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "refactor: camera rotation replacing x-flip, canonical color/chat in main.rs"
```

---

### Task 5: Migrate battle_phase.rs — remove flipped_winner, canonical LP

**Files:**
- Modify: `src/battle_phase.rs`

- [ ] **Step 1: Replace `let role = ctx.role;` with local_player_id**

At the top of the `update` function:

```rust
let local_player_id = ctx.local_player_id;
```

- [ ] **Step 2: Update surrender handling**

```rust
if screen_mouse.x >= yes_x && screen_mouse.x <= yes_x + btn_w && screen_mouse.y >= yes_y && screen_mouse.y <= yes_y + btn_h {
    ctx.progress.players[local_player_id as usize].lp = 0;
    battle.show_surrender_confirm = false;
    // Winner is whoever didn't surrender — find by checking LP
    let winner = ctx.progress.game_winner().unwrap_or(0);
    ctx.phase = GamePhase::GameOver(winner);
}
```

- [ ] **Step 3: Remove flipped_winner/flipped_loser in guest round end**

Replace the entire `waiting_for_round_end` block's inner handling:

```rust
if let Some(rd) = n.received_round_end.take() {
    // Use canonical values directly — no flipping
    let final_state = match rd.winner {
        Some(w) => MatchState::Winner(w),
        None => MatchState::Draw,
    };

    // Desync check — compare canonical counts directly
    let local_alive_0 = ctx.units.iter().filter(|u| u.alive && u.player_id == 0).count() as u16;
    let local_alive_1 = ctx.units.iter().filter(|u| u.alive && u.player_id == 1).count() as u16;
    if local_alive_0 != rd.alive_0 || local_alive_1 != rd.alive_1 {
        eprintln!("[DESYNC] Unit count mismatch! Local: {}/{} Host: {}/{}", local_alive_0, local_alive_1, rd.alive_0, rd.alive_1);
    }

    // Apply LP damage — canonical indexing
    if rd.timeout_dmg_0 > 0 || rd.timeout_dmg_1 > 0 {
        ctx.progress.players[0].lp -= rd.timeout_dmg_0;
        ctx.progress.players[1].lp -= rd.timeout_dmg_1;
    } else if let Some(loser) = rd.loser_team {
        ctx.progress.players[loser as usize].lp -= rd.lp_damage;
    }

    battle.waiting_for_round_end = false;
    battle.show_surrender_confirm = false;
    ctx.phase = GamePhase::RoundResult {
        match_state: final_state,
        lp_damage: rd.lp_damage,
        loser_team: rd.loser_team,
    };
}
```

- [ ] **Step 4: Remove unused `use crate::role::Role;` import if present**

Check for and remove any Role import.

- [ ] **Step 5: Run `cargo check`**

- [ ] **Step 6: Commit**

```bash
git add src/battle_phase.rs
git commit -m "refactor: remove flipped_winner/loser, canonical LP damage in battle_phase"
```

---

### Task 6: Migrate waiting_phase.rs and round_result.rs

**Files:**
- Modify: `src/waiting_phase.rs`
- Modify: `src/round_result.rs`

- [ ] **Step 1: Update waiting_phase.rs**

Use `player_id` from tagged build data — no local/peer computation:

```rust
pub fn update(ctx: &mut GameContext, battle: &mut BattleState) -> bool {
    if let Some(ref mut n) = ctx.net {
        n.poll();

        if let Some(build_data) = n.take_peer_build() {
            let pid = build_data.player_id;
            let _new_units = crate::match_progress::apply_peer_build(
                &mut ctx.progress,
                &build_data,
            );

            ctx.units.retain(|u| u.player_id != pid);
            ctx.units.extend(ctx.progress.players[pid as usize].respawn_units());

            if ctx.obstacles.is_empty() && ctx.game_settings.terrain_enabled {
                ctx.obstacles = terrain::generate_terrain(ctx.progress.round, ctx.game_settings.terrain_destructible);
            } else {
                terrain::reset_cover_hp(&mut ctx.obstacles);
            }
            ctx.nav_grid = Some(terrain::NavGrid::from_obstacles(&ctx.obstacles, ARENA_W, ARENA_H, 15.0));

            macroquad::rand::srand(ctx.progress.round as u64);
            battle.reset();

            for unit in ctx.units.iter_mut() {
                unit.damage_dealt_round = 0.0;
                unit.damage_soaked_round = 0.0;
            }

            ctx.phase = GamePhase::Battle;
            return true;
        }
    }
    false
}
```

- [ ] **Step 2: Update round_result.rs**

Replace `ctx.role.player_id()` with `ctx.local_player_id`. For respawning other players' units, iterate players skipping local:

```rust
pub fn update(ctx: &mut GameContext, battle: &mut BattleState) {
    if let Some(ref mut n) = ctx.net {
        n.poll();
    }

    if is_key_pressed(KeyCode::Space) {
        if ctx.progress.is_game_over() {
            ctx.phase = GamePhase::GameOver(ctx.progress.game_winner().unwrap_or(0));
        } else {
            let lpid = ctx.local_player_id as usize;

            // Save gold carry-over
            ctx.progress.players[lpid].gold = ctx.build.gold_remaining;

            ctx.progress.advance_round();

            // Lock current packs on the player's state
            ctx.build.lock_current_packs();
            let locked_packs: Vec<_> = ctx.build.placed_packs.clone();
            let next_id = ctx.build.next_id;

            let old_stats: std::collections::HashMap<u64, (f32, f32, f32, f32, u32)> =
                ctx.units
                    .iter()
                    .map(|u| {
                        (
                            u.id,
                            (
                                u.damage_dealt_total,
                                u.damage_soaked_total,
                                u.damage_dealt_round,
                                u.damage_soaked_round,
                                u.kills_total,
                            ),
                        )
                    })
                    .collect();

            ctx.units.clear();

            // New round gold = saved gold + round allowance
            let round_gold = ctx.progress.players[lpid].gold + ctx.progress.round_allowance();
            ctx.build = BuildState::new_round(round_gold, locked_packs, next_id);
            ctx.units.extend(ctx.build.respawn_player_units(&ctx.progress.players[lpid].techs, ctx.local_player_id));

            for unit in ctx.units.iter_mut() {
                if let Some(&(ddt, dst, ddr, dsr, kt)) = old_stats.get(&unit.id) {
                    unit.damage_dealt_total = ddt;
                    unit.damage_soaked_total = dst;
                    unit.damage_dealt_round = ddr;
                    unit.damage_soaked_round = dsr;
                    unit.kills_total = kt;
                }
            }

            // Respawn other players' units
            for (i, player) in ctx.progress.players.iter().enumerate() {
                if i != lpid {
                    ctx.units.extend(player.respawn_units());
                }
            }

            battle.projectiles.clear();
            ctx.phase = GamePhase::Build;
        }
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add src/waiting_phase.rs src/round_result.rs
git commit -m "refactor: waiting_phase uses tagged player_id, round_result uses local_player_id"
```

---

### Task 7: Migrate UI files — canonical display

**Files:**
- Modify: `src/phase_ui.rs`
- Modify: `src/ui.rs`
- Modify: `src/rendering.rs`

All `role: Role` parameters become `local_player_id: u8`. All name lookups use `progress.players[player_id as usize].name` directly.

- [ ] **Step 1: Update phase_ui.rs function signatures and imports**

Remove `use crate::role::Role;`. Change all function signatures from `role: Role` to `local_player_id: u8`. Add `use crate::arena;`.

- [ ] **Step 2: Update draw_build_ui**

```rust
pub fn draw_build_ui(
    build: &BuildState,
    progress: &MatchProgress,
    units: &[Unit],
    screen_mouse: Vec2,
    arena_camera: &Camera2D,
    local_player_id: u8,
) {
    let lpid = local_player_id as usize;
    crate::shop::draw_shop(build.gold_remaining, screen_mouse, false, &progress.banned_kinds, game_state::BUILD_LIMIT - build.packs_bought_this_round);

    // Pack labels
    {
        let packs = all_packs();
        for placed in build.placed_packs.iter() {
            let pack = &packs[placed.pack_index];
            let half = placed.bbox_half_size_for(pack);
            let world_pos = vec2(placed.center.x - half.x + 2.0, placed.center.y - half.y - 2.0);
            let screen_pos = arena_camera.world_to_screen(world_pos);
            let label = if placed.locked {
                format!("{} (R{})", pack.name, placed.round_placed)
            } else {
                pack.name.to_string()
            };
            let label_color = if placed.locked {
                Color::new(0.5, 0.5, 0.5, 0.4)
            } else {
                Color::new(0.7, 0.7, 0.7, 0.6)
            };
            crate::ui::draw_scaled_text(&label, screen_pos.x, screen_pos.y, 14.0, label_color);
        }
        // Other players' packs
        for (i, player) in progress.players.iter().enumerate() {
            if i == lpid { continue; }
            for opponent_pack in &player.packs {
                let pack = &packs[opponent_pack.pack_index];
                let half = PlacedPack::bbox_half_size_rotated(pack, opponent_pack.rotated);
                let world_pos = vec2(opponent_pack.center.x - half.x + 2.0, opponent_pack.center.y - half.y - 2.0);
                let screen_pos = arena_camera.world_to_screen(world_pos);
                let label = format!("{} (R{})", pack.name, opponent_pack.round_placed);
                crate::ui::draw_scaled_text(&label, screen_pos.x, screen_pos.y, 12.0, Color::new(0.4, 0.4, 0.6, 0.4));
            }
        }
    }

    // Tech panel
    if let Some(sel_idx) = build.selected_pack {
        if sel_idx < build.placed_packs.len() {
            let placed = &build.placed_packs[sel_idx];
            let kind = all_packs()[placed.pack_index].kind;
            let cs = crate::tech_ui::PackCombatStats::from_units(units, &placed.unit_ids);
            crate::tech_ui::draw_tech_panel(
                kind,
                &progress.players[lpid].techs,
                build.gold_remaining,
                screen_mouse,
                false,
                Some(&cs),
            );
        }
    }

    // Top HUD
    let army_value: u32 = {
        let packs = all_packs();
        build.placed_packs.iter().map(|p| packs[p.pack_index].cost).sum()
    };
    crate::ui::draw_hud(progress, build.gold_remaining, build.timer, army_value, 0.0, local_player_id);

    // Begin Round button + hint text remain unchanged
    // ... (keep existing button/hint code, no role references)
```

- [ ] **Step 3: Update draw_waiting_ui**

```rust
pub fn draw_waiting_ui(
    progress: &MatchProgress,
    build: &BuildState,
    local_player_id: u8,
) {
    crate::ui::draw_hud(progress, build.gold_remaining, 0.0, 0, 0.0, local_player_id);
    // ... rest unchanged
```

- [ ] **Step 4: Update draw_battle_ui — canonical name lookups**

```rust
pub fn draw_battle_ui(
    progress: &MatchProgress,
    units: &[Unit],
    obstacles: &[terrain::Obstacle],
    battle_timer: f32,
    round_timeout: f32,
    show_surrender_confirm: bool,
    screen_mouse: Vec2,
    world_mouse: Vec2,
    local_player_id: u8,
) {
    let remaining = (round_timeout - battle_timer).max(0.0);
    crate::ui::draw_hud(progress, 0, 0.0, 0, remaining, local_player_id);

    // ... alive counts + display unchanged ...

    // Obstacle tooltip — canonical name lookup
    if !show_surrender_confirm {
        for obs in obstacles {
            if !obs.alive { continue; }
            if obs.contains_point(world_mouse) {
                // ... tooltip drawing unchanged until team_name ...
                let team_name = if (obs.player_id as usize) < progress.players.len() {
                    progress.players[obs.player_id as usize].name.as_str()
                } else {
                    "Neutral"
                };
                crate::ui::draw_scaled_text(&format!("Owner: {}", team_name), tip_x + crate::ui::s(6.0), ty, 12.0, LIGHTGRAY);
                break;
            }
        }
    }

    // ... surrender overlay unchanged ...
}
```

- [ ] **Step 5: Update draw_round_result_ui — canonical lookups**

```rust
pub fn draw_round_result_ui(
    progress: &MatchProgress,
    match_state: &MatchState,
    lp_damage: i32,
    loser_team: Option<u8>,
    local_player_id: u8,
) {
    crate::ui::draw_hud(progress, 0, 0.0, 0, 0.0, local_player_id);

    let text = match match_state {
        MatchState::Winner(tid) => {
            let winner_name = &progress.players[*tid as usize].name;
            let color_idx = crate::team::color_index(*tid);
            let color_name = settings::TEAM_COLOR_OPTIONS
                .get(color_idx as usize)
                .map(|(name, _)| *name)
                .unwrap_or("???");
            format!("{} ({}) wins round {}!", winner_name, color_name, progress.round)
        }
        MatchState::Draw => format!("Round {} - Draw!", progress.round),
        MatchState::InProgress => unreachable!(),
    };

    // ... dims + draw unchanged ...

    if let Some(loser) = loser_team {
        let loser_name = &progress.players[loser as usize].name;
        let dmg_text = format!("{} loses {} LP", loser_name, lp_damage);
        // ... draw unchanged ...
    }

    // ... next_text unchanged ...
}
```

- [ ] **Step 6: Update draw_game_over_ui — canonical + local_player_id for headline**

```rust
pub fn draw_game_over_ui(
    winner: u8,
    progress: &MatchProgress,
    units: &[Unit],
    screen_mouse: Vec2,
    local_player_id: u8,
) {
    let headline = if winner == local_player_id { "YOU WIN!".to_string() } else { "YOU LOSE!".to_string() };
    let winner_name = &progress.players[winner as usize].name;
    let winner_color_idx = crate::team::color_index(winner);
    let color_name = settings::TEAM_COLOR_OPTIONS
        .get(winner_color_idx as usize)
        .map(|(name, _)| *name)
        .unwrap_or("???");
    let subtitle = format!("{} ({}) wins!", winner_name, color_name);
    let headline_color = if winner == local_player_id {
        Color::new(0.2, 1.0, 0.3, 1.0)
    } else {
        Color::new(1.0, 0.3, 0.2, 1.0)
    };

    // ... draw headline + subtitle unchanged ...

    // Stats panel — use local_player_id for "my" stats
    let lpid = local_player_id;

    // MVP
    let mvp = units.iter()
        .filter(|u| u.player_id == lpid)
        .max_by(|a, b| a.damage_dealt_total.partial_cmp(&b.damage_dealt_total).unwrap_or(std::cmp::Ordering::Equal));
    // ... rest of MVP display unchanged ...

    let total_dmg: f32 = units.iter()
        .filter(|u| u.player_id == lpid)
        .map(|u| u.damage_dealt_total)
        .sum();
    // ... draw unchanged ...

    let surviving = units.iter().filter(|u| u.player_id == lpid && u.alive).count();
    let total_units = units.iter().filter(|u| u.player_id == lpid).count();
    // ... draw unchanged ...

    // LP line — canonical lookups
    let lpid_idx = local_player_id as usize;
    let my_name = &progress.players[lpid_idx].name;
    let my_lp = progress.players[lpid_idx].lp;
    // Show all other players' LP
    let mut lp_parts = format!("LP: {} {}", my_name, my_lp);
    for (i, player) in progress.players.iter().enumerate() {
        if i != lpid_idx {
            lp_parts.push_str(&format!(" vs {} {}", player.name, player.lp));
        }
    }
    crate::ui::draw_scaled_text(&lp_parts, sx, sy, 15.0, LIGHTGRAY);

    // ... rematch button + return hint unchanged ...
}
```

- [ ] **Step 7: Update ui.rs draw_hud**

```rust
pub fn draw_hud(progress: &MatchProgress, gold: u32, timer: f32, army_value: u32, battle_remaining: f32, local_player_id: u8) {
    let lpid = local_player_id as usize;
    let player = &progress.players[lpid];
    let player_lp = player.lp;
    let player_name = &player.name;

    // ... background bar unchanged ...

    // Round
    // ... unchanged ...

    // Local player LP (first slot)
    let player_lp_text = format!("{} LP: {}", player_name, player_lp);
    // ... color + draw unchanged ...

    // Other players' LP
    for (i, other) in progress.players.iter().enumerate() {
        if i == lpid { continue; }
        let olp_text = format!("{} LP: {}", other.name, other.lp);
        let alp_color = if other.lp > 500 {
            Color::new(0.3, 0.6, 1.0, 1.0)
        } else if other.lp > 200 {
            Color::new(1.0, 0.8, 0.2, 1.0)
        } else {
            Color::new(1.0, 0.3, 0.2, 1.0)
        };
        let alp_w = measure_scaled_text(&olp_text, 18).width;
        draw_scaled_text(&olp_text, x, hud_y, 18.0, alp_color);
        x += alp_w + gap;
    }

    // ... gold + timer unchanged ...
}
```

Remove `use crate::role::Role;`.

- [ ] **Step 8: Update rendering.rs draw_build_overlays**

Change signature and body:

```rust
pub fn draw_build_overlays(build: &BuildState, progress: &MatchProgress, world_mouse: Vec2, local_player_id: u8) {
    // ... placement zone + drag-box + player pack bounding boxes unchanged ...

    // Other players' pack bounding boxes
    let lpid = local_player_id as usize;
    let packs = all_packs();
    for (i, player) in progress.players.iter().enumerate() {
        if i == lpid { continue; }
        for opponent_pack in &player.packs {
            let pack = &packs[opponent_pack.pack_index];
            let half = PlacedPack::bbox_half_size_rotated(pack, opponent_pack.rotated);
            let min = opponent_pack.center - half;
            let bbox_color = Color::new(0.3, 0.3, 0.5, 0.2);
            draw_rectangle_lines(min.x, min.y, half.x * 2.0, half.y * 2.0, 1.0, bbox_color);
        }
    }
}
```

Remove `use crate::role::Role;` if present (it uses `crate::role::Role` inline currently).

- [ ] **Step 9: Commit**

```bash
git add src/phase_ui.rs src/ui.rs src/rendering.rs
git commit -m "refactor: canonical display in UI files, replace Role with local_player_id"
```

---

### Task 8: Migrate remaining files

**Files:**
- Modify: `src/build_phase.rs`
- Modify: `src/game_over.rs`
- Modify: `src/chat.rs`
- Modify: `src/lobby.rs`

- [ ] **Step 1: Update build_phase.rs**

Replace all `ctx.role.player_id()` with `ctx.local_player_id` and `ctx.role.deploy_x_range()` with `arena::deploy_x_range(ctx.local_player_id)`. Add `use crate::arena;` if not present. Update `send_build_complete` calls to pass `ctx.local_player_id`.

Key changes:
- `let local = ctx.role.player_id() as usize;` → `let lpid = ctx.local_player_id as usize;`
- `ctx.progress.players[local]` → `ctx.progress.players[lpid]`
- `ctx.role.deploy_x_range()` → `arena::deploy_x_range(ctx.local_player_id)`
- `ctx.role.player_id()` → `ctx.local_player_id`
- `net::send_build_complete(&mut ctx.net, &ctx.build);` → `net::send_build_complete(&mut ctx.net, &ctx.build, ctx.local_player_id);`

- [ ] **Step 2: Update game_over.rs**

Replace `ctx.net.as_ref().is_none_or(|n| n.is_host)` logic — `BuildState::new` still takes `is_host` for next_id offset. Keep that. No role references to remove.

- [ ] **Step 3: Update chat.rs**

Change `receive_from_net` to no longer take a `peer_id` parameter — it reads player_id from the tagged chat messages:

```rust
pub fn receive_from_net(&mut self, net: &mut Option<net::NetState>) {
    if let Some(ref mut n) = net {
        for (player_id, name, text) in n.received_chats.drain(..) {
            self.messages.push(ChatMessage {
                name,
                text,
                player_id,
                lifetime: 5.0,
            });
        }
    }
}
```

Update the `update` method to include `player_id` when sending chat:

```rust
if let Some(ref mut n) = net {
    n.send(net::NetMessage::ChatMessage {
        player_id: local_id,
        name: player_name.to_string(),
        text,
    });
}
```

- [ ] **Step 4: Update lobby.rs**

The lobby sends NameSync and ColorChoice. It needs to include `player_id`. Derive from `is_room_creator`:

```rust
let my_pid: u8 = if self.is_room_creator { 0 } else { 1 };
net.send(crate::net::NetMessage::NameSync { player_id: my_pid, name: self.player_name.clone() });
net.send(crate::net::NetMessage::ColorChoice { player_id: my_pid, color_index: game_settings.player_color_index });
```

Update both locations where these are sent (the WaitingForPeer → Connected transition and the color change in draw).

- [ ] **Step 5: Run `cargo check`**

Expected: Clean compilation.

- [ ] **Step 6: Run `cargo clippy`**

Expected: Only pre-existing `too_many_arguments` warnings.

- [ ] **Step 7: Commit**

```bash
git add src/build_phase.rs src/game_over.rs src/chat.rs src/lobby.rs
git commit -m "refactor: migrate remaining files to canonical player_id system"
```

---

### Task 9: Final verification and cleanup

**Files:** All source files (verification only)

- [ ] **Step 1: Search for stale references**

```bash
grep -rn "role" src/ --include="*.rs" | grep -v "//\|is_room_creator\|color\|role_name"
grep -rn "local.*=.*1 - " src/ --include="*.rs"
grep -rn "peer.*=.*1 - " src/ --include="*.rs"
grep -rn "flipped_winner\|flipped_loser\|opponent_id\|x_flip" src/ --include="*.rs"
```

Expected: No matches (except maybe comments).

- [ ] **Step 2: Run cargo check**

Run: `cargo check 2>&1`
Expected: Clean.

- [ ] **Step 3: Run cargo clippy**

Run: `cargo clippy 2>&1`
Expected: Only pre-existing `too_many_arguments` warnings.

- [ ] **Step 4: Commit any cleanup**

If stray references found:

```bash
git add -A
git commit -m "refactor: cleanup stale perspective references"
```
