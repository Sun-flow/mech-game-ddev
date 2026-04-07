# Array-Indexed PlayerState Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `host`/`guest` named fields with `players: [PlayerState; 2]` array, remove all perspective-relative accessors, and rename net `opponent_*` to `peer_*`.

**Architecture:** MatchProgress changes from `host: PlayerState` + `guest: PlayerState` to `players: [PlayerState; 2]`. All call sites switch from `progress.player(role)` / `progress.opponent(role)` to `progress.players[local]` / `progress.players[peer]` where `local = role.player_id() as usize` and `peer = 1 - local`. The net layer renames `opponent_*` fields to `peer_*` with no structural changes.

**Tech Stack:** Rust, macroquad

**Spec:** `docs/superpowers/specs/2026-04-06-array-indexed-playerstate-design.md`

---

### Task 1: Restructure MatchProgress and PlayerState constructors

**Files:**
- Modify: `src/match_progress.rs`

This is the core data model change. Replace `host`/`guest` fields with `players: [PlayerState; 2]`, replace `new_host()`/`new_guest()` with `new(player_id: u8)`, remove all accessor methods (`player`, `player_mut`, `opponent`, `opponent_mut`, `player_lp`, `opponent_lp`), and convert `apply_opponent_build` to a free function `apply_peer_build`.

- [ ] **Step 1: Replace PlayerState constructors**

Replace `new_host()` and `new_guest()` with a single `new(player_id: u8)`:

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

Delete `new_host()` and `new_guest()`.

- [ ] **Step 2: Replace MatchProgress fields and constructor**

Change the struct definition:

```rust
#[derive(Clone, Debug)]
pub struct MatchProgress {
    pub round: u32,
    pub players: [PlayerState; 2],
    pub banned_kinds: Vec<UnitKind>,
}
```

Update `MatchProgress::new()`:

```rust
pub fn new() -> Self {
    Self {
        round: 1,
        players: [PlayerState::new(0), PlayerState::new(1)],
        banned_kinds: Vec::new(),
    }
}
```

- [ ] **Step 3: Remove all accessor methods**

Delete these methods from the `impl MatchProgress` block:
- `player(&self, role)` and `player_mut(&mut self, role)`
- `opponent(&self, role)` and `opponent_mut(&mut self, role)`
- `player_lp(&self, role)` and `opponent_lp(&self, role)`

- [ ] **Step 4: Update is_game_over and game_winner**

Replace `self.host` / `self.guest` with indexed access:

```rust
pub fn is_game_over(&self) -> bool {
    self.players[0].lp <= 0 || self.players[1].lp <= 0
}

pub fn game_winner(&self) -> Option<u8> {
    if self.players[1].lp <= 0 {
        Some(0)
    } else if self.players[0].lp <= 0 {
        Some(1)
    } else {
        None
    }
}
```

- [ ] **Step 5: Convert apply_opponent_build to free function apply_peer_build**

Remove `apply_opponent_build` from `impl MatchProgress`. Add a free function outside the impl block. Use the old type name `OpponentBuildData` for now — it gets renamed to `PeerBuildData` in Task 2. After Task 2, update the type reference in this function.

```rust
/// Apply peer's build data received over the network.
/// Canonical coordinates — no mirroring needed.
pub fn apply_peer_build(player: &mut PlayerState, data: &crate::net::OpponentBuildData, round: u32) -> Vec<Unit> {
    let packs = all_packs();
    let mut new_units = Vec::new();

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

- [ ] **Step 6: Update spawn_ai_army**

Replace `self.guest` with `self.players[1]`:

```rust
pub fn spawn_ai_army(&mut self, ai_packs: &[PackDef]) -> Vec<Unit> {
    let packs = all_packs();
    let mut new_units = Vec::new();

    let ai_center_x = crate::arena::HALF_W + (crate::arena::HALF_W / 2.0);
    let total_new = ai_packs.len();
    if total_new == 0 {
        return new_units;
    }

    let arena_h = crate::arena::ARENA_H;
    let spacing = arena_h / (total_new as f32 + 1.0);

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
            pack,
            center,
            false,
            self.players[1].player_id,
            &self.players[1].techs,
            &mut self.players[1].next_id,
        );
        new_units.extend(spawned);

        self.players[1].packs.push(PlacedPack {
            pack_index,
            center,
            unit_ids: ids,
            pre_drag_center: center,
            rotated: false,
            locked: true,
            round_placed: self.round,
        });
    }

    new_units
}
```

- [ ] **Step 7: Remove unused import**

Remove `use crate::role::Role;` since the accessor methods that used it are deleted. Keep `use crate::net::OpponentBuildData;` for now — `apply_peer_build` still references it until Task 2 renames it to `PeerBuildData`.

- [ ] **Step 8: Run `cargo check`**

Run: `cargo check 2>&1`

Expected: Errors in every file that used the old accessors/fields. This is expected — they will be fixed in subsequent tasks. Verify that `match_progress.rs` itself has no internal errors.

- [ ] **Step 9: Commit**

```bash
git add src/match_progress.rs
git commit -m "refactor: replace host/guest fields with players array in MatchProgress"
```

---

### Task 2: Rename net layer opponent_* to peer_*

**Files:**
- Modify: `src/net.rs`

Rename all `opponent_*` fields and `OpponentBuildData` type to `peer_*` / `PeerBuildData`. This is a mechanical rename within a single file plus the type name.

- [ ] **Step 1: Rename OpponentBuildData to PeerBuildData**

```rust
#[derive(Clone, Debug)]
pub struct PeerBuildData {
    pub new_packs: Vec<(usize, (f32, f32), bool)>,
    pub tech_purchases: Vec<(UnitKind, TechId)>,
}
```

- [ ] **Step 2: Rename all opponent_* fields in NetState**

```rust
pub struct NetState {
    // ... unchanged fields ...
    pub peer_build: Option<PeerBuildData>,
    // ... unchanged ...
    pub peer_surrendered: bool,
    pub peer_rematch: bool,
    pub peer_bans: Option<Vec<u8>>,
    // ... unchanged ...
    pub peer_color: Option<u8>,
    pub peer_name: Option<String>,
    // ... rest unchanged ...
}
```

- [ ] **Step 3: Update NetState::new() initializer**

Replace all `opponent_` prefixes with `peer_`:

```rust
peer_build: None,
// ...
peer_surrendered: false,
peer_rematch: false,
peer_bans: None,
// ...
peer_color: None,
peer_name: None,
```

- [ ] **Step 4: Update poll() message handlers**

Replace every `self.opponent_` reference in the `poll()` method with `self.peer_`:

- `self.opponent_build = Some(OpponentBuildData {` → `self.peer_build = Some(PeerBuildData {`
- `self.opponent_surrendered = true;` → `self.peer_surrendered = true;`
- `self.opponent_rematch = true;` → `self.peer_rematch = true;`
- `self.opponent_bans = Some(bans);` → `self.peer_bans = Some(bans);`
- `self.opponent_color = Some(idx);` → `self.peer_color = Some(idx);`
- `self.opponent_name = Some(name);` → `self.peer_name = Some(name);`

- [ ] **Step 5: Rename take_opponent_build to take_peer_build**

```rust
pub fn take_peer_build(&mut self) -> Option<PeerBuildData> {
    self.peer_build.take()
}
```

- [ ] **Step 6: Update match_progress.rs type reference**

In `src/match_progress.rs`, update the `apply_peer_build` function signature and import to use the new name:

Replace `use crate::net::OpponentBuildData;` with `use crate::net::PeerBuildData;`

Replace `data: &crate::net::OpponentBuildData` with `data: &crate::net::PeerBuildData` in the `apply_peer_build` function signature. (If using the full path, just update the path; if using the import, update the import.)

- [ ] **Step 7: Run `cargo check`**

Run: `cargo check 2>&1`

Expected: Errors in files that reference the old names (context.rs, main.rs, waiting_phase.rs, draft_ban.rs). These are fixed in subsequent tasks.

- [ ] **Step 8: Commit**

```bash
git add src/net.rs src/match_progress.rs
git commit -m "refactor: rename net opponent_* fields to peer_*"
```

---

### Task 3: Remove Role::opponent_id and rename DraftBan field

**Files:**
- Modify: `src/role.rs`
- Modify: `src/game_state.rs`

- [ ] **Step 1: Remove opponent_id from Role**

Delete the `opponent_id` method from `impl Role` in `src/role.rs`:

```rust
// DELETE THIS ENTIRE METHOD:
    /// The opponent's player_id.
    pub fn opponent_id(self) -> u8 {
        match self {
            Role::Host => 1,
            Role::Guest => 0,
            Role::Spectator => 255,
        }
    }
```

- [ ] **Step 2: Rename opponent_bans field in GamePhase::DraftBan**

In `src/game_state.rs`, rename the field:

```rust
#[derive(Clone, Debug)]
pub enum GamePhase {
    Lobby,
    DraftBan {
        bans: Vec<crate::unit::UnitKind>,
        confirmed: bool,
        peer_bans: Option<Vec<crate::unit::UnitKind>>,
    },
    // ... rest unchanged
}
```

- [ ] **Step 3: Commit**

```bash
git add src/role.rs src/game_state.rs
git commit -m "refactor: remove Role::opponent_id, rename DraftBan opponent_bans to peer_bans"
```

---

### Task 4: Migrate context.rs and main.rs

**Files:**
- Modify: `src/context.rs`
- Modify: `src/main.rs`

These are the entry points that wire everything together.

- [ ] **Step 1: Update context.rs start_game**

Replace all `player(role)` / `opponent(role)` / `opponent_name` references:

```rust
pub fn start_game(
    &mut self,
    net: Option<net::NetState>,
    is_host: bool,
    player_name: String,
    draft_ban_enabled: bool,
) {
    self.net = net;
    self.role = if is_host { Role::Host } else { Role::Guest };

    let local = self.role.player_id() as usize;
    // TODO: 2-player assumption — derive peer index from connection identity when supporting N players
    let peer = 1 - local;

    let mut peer_name = "Opponent".to_string();
    if let Some(ref mut n) = self.net {
        n.is_host = is_host;
        peer_name = n.peer_name.clone().unwrap_or_else(|| "Opponent".to_string());
    }

    self.progress = MatchProgress::new();

    // Set names on PlayerState
    self.progress.players[local].name = player_name;
    self.progress.players[peer].name = peer_name;

    // Initialize gold with round allowance
    let allowance = self.progress.round_allowance();
    self.progress.players[local].gold = allowance;

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
```

- [ ] **Step 2: Update main.rs color setup**

Replace `opponent_id()` and `opponent_color`:

```rust
// Set colors canonically: my color goes to my player_id slot
team::set_color(ctx.role.player_id(), ctx.game_settings.player_color_index);
if let Some(ref n) = ctx.net {
    // TODO: 2-player assumption — derive peer index from connection identity when supporting N players
    let peer_id = 1 - ctx.role.player_id();
    if let Some(opp_color) = n.peer_color {
        team::set_color(peer_id, opp_color);
    }
}
```

- [ ] **Step 3: Update main.rs VS AI start**

Replace `ctx.progress.guest.name` with indexed access:

```rust
lobby::LobbyResult::StartVsAi => {
    ctx.start_game(None, true, lobby.player_name.clone(), ctx.game_settings.draft_ban_enabled);
    ctx.progress.players[1].name = "AI".to_string();
    continue;
}
```

Apply this in both the `update` and `draw` match arms (lines ~137 and ~151).

- [ ] **Step 4: Update main.rs DraftBan pattern**

Rename the field binding:

```rust
GamePhase::DraftBan { ref mut bans, ref mut confirmed, ref mut peer_bans } => {
    match draft_ban::update_and_draw(bans, confirmed, peer_bans, &mut ctx.net, mouse.screen_mouse, mouse.left_click) {
```

- [ ] **Step 5: Update main.rs chat system**

Replace `opponent_id()` with `1 - role.player_id()`:

```rust
// TODO: 2-player assumption — derive peer index from connection identity when supporting N players
let peer_id = 1 - ctx.role.player_id();
ctx.chat.receive_from_net(&mut ctx.net, peer_id);
let local = ctx.role.player_id() as usize;
let my_name = ctx.progress.players[local].name.clone();
ctx.chat.update(&ctx.phase, &mut ctx.net, &my_name, ctx.role.player_id());
ctx.chat.tick(dt);
ctx.chat.draw(&ctx.phase, &my_name);
```

- [ ] **Step 6: Run `cargo check`**

Run: `cargo check 2>&1`

Verify context.rs and main.rs compile. Other files may still have errors.

- [ ] **Step 7: Commit**

```bash
git add src/context.rs src/main.rs
git commit -m "refactor: migrate context.rs and main.rs to array-indexed PlayerState"
```

---

### Task 5: Migrate battle_phase.rs

**Files:**
- Modify: `src/battle_phase.rs`

This file has the most `host`/`guest` references — direct field access for LP and techs.

- [ ] **Step 1: Update update_attacks tech parameters**

The `update_attacks` function takes two `&TechState` params (host techs, guest techs). Replace with indexed access. In the multiplayer section (line ~84):

```rust
update_attacks(
    &mut ctx.units,
    &mut battle.projectiles,
    FIXED_DT,
    &ctx.progress.players[0].techs,
    &ctx.progress.players[1].techs,
    &mut battle.splash_effects,
);
```

In the single-player section (line ~169):

```rust
update_attacks(
    &mut ctx.units,
    &mut battle.projectiles,
    dt,
    &ctx.progress.players[0].techs,
    &ctx.progress.players[1].techs,
    &mut battle.splash_effects,
);
```

- [ ] **Step 2: Update surrender handling**

Replace `player_mut(role)` and `opponent_id()`:

```rust
if screen_mouse.x >= yes_x && screen_mouse.x <= yes_x + btn_w && screen_mouse.y >= yes_y && screen_mouse.y <= yes_y + btn_h {
    let local = role.player_id() as usize;
    ctx.progress.players[local].lp = 0;
    battle.show_surrender_confirm = false;
    // TODO: 2-player assumption — derive peer index from connection identity when supporting N players
    let peer_pid = 1 - role.player_id();
    ctx.phase = GamePhase::GameOver(peer_pid);
}
```

- [ ] **Step 3: Update guest round end handling**

Replace `ctx.progress.host.lp` / `ctx.progress.guest.lp` in the `waiting_for_round_end` block:

```rust
// Apply timeout mutual damage
if rd.timeout_dmg_0 > 0 || rd.timeout_dmg_1 > 0 {
    ctx.progress.players[0].lp -= rd.timeout_dmg_0;
    ctx.progress.players[1].lp -= rd.timeout_dmg_1;
} else if let Some(loser) = flipped_loser {
    if loser == 0 {
        ctx.progress.players[1].lp -= rd.lp_damage;
    } else {
        ctx.progress.players[0].lp -= rd.lp_damage;
    }
}
```

Note: The `flipped_loser` logic uses guest perspective where `0 = guest`. When `loser == 0` (guest lost), we subtract from `players[1]` (guest). When `loser == 1` (host lost), we subtract from `players[0]` (host).

- [ ] **Step 4: Update AI memory recording**

Replace `ctx.progress.guest.ai_memory` and `ctx.progress.host.player_id`:

```rust
ctx.progress.players[1].ai_memory.record_round(&ctx.units, ctx.progress.players[0].player_id, ai_won);
```

- [ ] **Step 5: Update host/single-player LP damage application**

Replace `ctx.progress.host.lp` / `ctx.progress.guest.lp`:

```rust
// Apply LP damage using canonical player indices
if timed_out {
    ctx.progress.players[0].lp -= timeout_dmg_0;
    ctx.progress.players[1].lp -= timeout_dmg_1;
} else if let Some(loser) = loser_team {
    // loser 0 = host lost, loser 1 = guest lost
    ctx.progress.players[loser as usize].lp -= lp_damage;
}
```

- [ ] **Step 6: Run `cargo check`**

Run: `cargo check 2>&1`

- [ ] **Step 7: Commit**

```bash
git add src/battle_phase.rs
git commit -m "refactor: migrate battle_phase.rs to array-indexed PlayerState"
```

---

### Task 6: Migrate waiting_phase.rs and round_result.rs

**Files:**
- Modify: `src/waiting_phase.rs`
- Modify: `src/round_result.rs`

- [ ] **Step 1: Update waiting_phase.rs**

Replace `take_opponent_build`, `apply_opponent_build`, `opponent_id()`, and `opponent(role)`:

```rust
pub fn update(ctx: &mut GameContext, battle: &mut BattleState) -> bool {
    if let Some(ref mut n) = ctx.net {
        n.poll();

        if let Some(peer_build) = n.take_peer_build() {
            let local = ctx.role.player_id() as usize;
            // TODO: 2-player assumption — derive peer index from connection identity when supporting N players
            let peer = 1 - local;
            let round = ctx.progress.round;
            let opp_units = crate::match_progress::apply_peer_build(
                &mut ctx.progress.players[peer],
                &peer_build,
                round,
            );

            let peer_pid = ctx.progress.players[peer].player_id;
            ctx.units.retain(|u| u.player_id != peer_pid);
            ctx.units.extend(ctx.progress.players[peer].respawn_units());

            let _ = opp_units;

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

Replace `player(role)`, `player_mut(role)`, `opponent(role)`:

```rust
pub fn update(ctx: &mut GameContext, battle: &mut BattleState) {
    if let Some(ref mut n) = ctx.net {
        n.poll();
    }

    if is_key_pressed(KeyCode::Space) {
        if ctx.progress.is_game_over() {
            ctx.phase = GamePhase::GameOver(ctx.progress.game_winner().unwrap_or(0));
        } else {
            let local = ctx.role.player_id() as usize;
            // TODO: 2-player assumption — derive peer index from connection identity when supporting N players
            let peer = 1 - local;

            // Save gold carry-over
            ctx.progress.players[local].gold = ctx.build.gold_remaining;

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
            let round_gold = ctx.progress.players[local].gold + ctx.progress.round_allowance();
            ctx.build = BuildState::new_round(round_gold, locked_packs, next_id);
            ctx.units.extend(ctx.build.respawn_player_units(&ctx.progress.players[local].techs, ctx.role.player_id()));

            for unit in ctx.units.iter_mut() {
                if let Some(&(ddt, dst, ddr, dsr, kt)) = old_stats.get(&unit.id) {
                    unit.damage_dealt_total = ddt;
                    unit.damage_soaked_total = dst;
                    unit.damage_dealt_round = ddr;
                    unit.damage_soaked_round = dsr;
                    unit.kills_total = kt;
                }
            }

            ctx.units.extend(ctx.progress.players[peer].respawn_units());

            battle.projectiles.clear();
            ctx.phase = GamePhase::Build;
        }
    }
}
```

- [ ] **Step 3: Run `cargo check`**

Run: `cargo check 2>&1`

- [ ] **Step 4: Commit**

```bash
git add src/waiting_phase.rs src/round_result.rs
git commit -m "refactor: migrate waiting_phase.rs and round_result.rs to array-indexed PlayerState"
```

---

### Task 7: Migrate UI files (phase_ui.rs, ui.rs, rendering.rs)

**Files:**
- Modify: `src/phase_ui.rs`
- Modify: `src/ui.rs`
- Modify: `src/rendering.rs`

All display-only code. Replace `player(role)` / `opponent(role)` with indexed access.

- [ ] **Step 1: Update phase_ui.rs draw_build_ui**

Replace `progress.opponent(role).packs` and `progress.player(role).techs`:

```rust
pub fn draw_build_ui(
    build: &BuildState,
    progress: &MatchProgress,
    units: &[Unit],
    screen_mouse: Vec2,
    arena_camera: &Camera2D,
    role: Role,
) {
    let local = role.player_id() as usize;
    // TODO: 2-player assumption — derive peer index from connection identity when supporting N players
    let peer = 1 - local;

    crate::shop::draw_shop(build.gold_remaining, screen_mouse, false, &progress.banned_kinds, game_state::BUILD_LIMIT - build.packs_bought_this_round);

    // Pack labels
    {
        let packs = all_packs();
        for placed in build.placed_packs.iter() {
            // ... (unchanged label drawing for own packs)
        }
        for opponent_pack in &progress.players[peer].packs {
            // ... (unchanged — just source changes from progress.opponent(role).packs)
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
                &progress.players[local].techs,
                build.gold_remaining,
                screen_mouse,
                false,
                Some(&cs),
            );
        }
    }

    // Top HUD (unchanged call)
    // ... rest unchanged
}
```

- [ ] **Step 2: Update phase_ui.rs draw_battle_ui**

Replace `progress.player(role).name` / `progress.opponent(role).name`:

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
    role: Role,
) {
    let local = role.player_id() as usize;
    // TODO: 2-player assumption
    let peer = 1 - local;
    let mp_player_name = &progress.players[local].name;
    let mp_opponent_name = &progress.players[peer].name;
    // ... rest of function uses mp_player_name/mp_opponent_name unchanged
```

- [ ] **Step 3: Update phase_ui.rs draw_round_result_ui**

Replace accessor calls:

```rust
pub fn draw_round_result_ui(
    progress: &MatchProgress,
    match_state: &MatchState,
    lp_damage: i32,
    loser_team: Option<u8>,
    role: Role,
) {
    let local = role.player_id() as usize;
    let peer = 1 - local;
    let mp_player_name = &progress.players[local].name;
    let mp_opponent_name = &progress.players[peer].name;
    // ... rest unchanged
```

- [ ] **Step 4: Update phase_ui.rs draw_game_over_ui**

Replace accessor calls:

```rust
pub fn draw_game_over_ui(
    winner: u8,
    progress: &MatchProgress,
    units: &[Unit],
    screen_mouse: Vec2,
    role: Role,
) {
    let local = role.player_id() as usize;
    let peer = 1 - local;
    let mp_player_name = &progress.players[local].name;
    let mp_opponent_name = &progress.players[peer].name;
    let local_pid = role.player_id();
    // ... rest unchanged, but update LP line:
    crate::ui::draw_scaled_text(&format!("LP: {} {} vs {} {}", mp_player_name, progress.players[local].lp, mp_opponent_name, progress.players[peer].lp), sx, sy, 15.0, LIGHTGRAY);
```

- [ ] **Step 5: Update ui.rs draw_hud**

Replace `progress.player(role)` / `progress.opponent(role)`:

```rust
pub fn draw_hud(progress: &MatchProgress, gold: u32, timer: f32, army_value: u32, battle_remaining: f32, role: Role) {
    let local = role.player_id() as usize;
    // TODO: 2-player assumption
    let peer = 1 - local;
    let player = &progress.players[local];
    let opponent = &progress.players[peer];
    let player_lp = player.lp;
    let opponent_lp = opponent.lp;
    let player_name = &player.name;
    let opponent_name = &opponent.name;
    // ... rest unchanged
```

- [ ] **Step 6: Update rendering.rs draw_build_overlays**

Replace `progress.opponent(role).packs`:

```rust
pub fn draw_build_overlays(build: &BuildState, progress: &MatchProgress, world_mouse: Vec2, role: crate::role::Role) {
    // ... drag-box and player pack bounding boxes unchanged ...

    // Opponent pack bounding boxes
    let local = role.player_id() as usize;
    // TODO: 2-player assumption
    let peer = 1 - local;
    let packs = all_packs();
    for opponent_pack in &progress.players[peer].packs {
        // ... rest unchanged
    }
}
```

- [ ] **Step 7: Run `cargo check`**

Run: `cargo check 2>&1`

- [ ] **Step 8: Commit**

```bash
git add src/phase_ui.rs src/ui.rs src/rendering.rs
git commit -m "refactor: migrate UI files to array-indexed PlayerState"
```

---

### Task 8: Migrate build_phase.rs, economy.rs, draft_ban.rs, game_over.rs

**Files:**
- Modify: `src/build_phase.rs`
- Modify: `src/economy.rs`
- Modify: `src/draft_ban.rs`
- Modify: `src/game_over.rs`

- [ ] **Step 1: Update build_phase.rs**

Replace all `progress.player(role)` and `progress.player_mut(role)` with indexed access. There are ~10 occurrences in this file:

At the top of the function, add:
```rust
let local = ctx.role.player_id() as usize;
```

Then replace throughout:
- `ctx.progress.player(role).techs` → `ctx.progress.players[local].techs`
- `ctx.progress.player_mut(role).techs` → `ctx.progress.players[local].techs`

The tech undo block (lines ~69-83) becomes:
```rust
game_state::UndoEntry::Tech { kind, tech_id } => {
    let cost = ctx.progress.players[local].techs.effective_cost(kind);
    ctx.progress.players[local].techs.unpurchase(kind, tech_id);
    ctx.build.gold_remaining += cost;
    if let Some(pos) = ctx.build.round_tech_purchases.iter().rposition(|(k, t)| *k == kind && *t == tech_id) {
        ctx.build.round_tech_purchases.remove(pos);
    }
    tech::refresh_units_of_kind(&mut ctx.units, kind, &ctx.progress.players[local].techs);
}
```

The shop interaction (line ~119):
```rust
&ctx.progress.players[local].techs,
```

The tech panel section (lines ~138-165): replace all `ctx.progress.player(role).techs` with `ctx.progress.players[local].techs` and `ctx.progress.player_mut(role).techs` with `ctx.progress.players[local].techs`.

- [ ] **Step 2: Update economy.rs start_ai_battle**

Replace `progress.guest` with `progress.players[1]`:

```rust
// Remove old AI (guest) units
units.retain(|u| u.player_id != progress.players[1].player_id);

// Respawn all existing opponent (guest) units from previous rounds at full HP
units.extend(progress.players[1].respawn_units());

// AI buys techs, then spawns NEW army for this round
let mut ai_gold = progress.round_allowance();
ai_buy_techs(&mut ai_gold, &mut progress.players[1].techs);
let ai_packs = if game_settings.smart_ai {
    smart_army(ai_gold, &progress.players[1].ai_memory, &progress.banned_kinds)
} else {
    random_army_filtered(ai_gold, &progress.banned_kinds)
};
```

- [ ] **Step 3: Update draft_ban.rs**

Replace `n.opponent_bans` with `n.peer_bans`:

Line 104: `if let Some(ob) = n.peer_bans.take() {`

The function parameter `opponent_bans` can stay as-is since it's a local variable name referring to the `GamePhase::DraftBan` field (which was renamed to `peer_bans` in Task 3). Actually, the parameter name should match the field name for clarity. Update the parameter name throughout:

```rust
pub fn update_and_draw(
    bans: &mut Vec<UnitKind>,
    confirmed: &mut bool,
    peer_bans: &mut Option<Vec<UnitKind>>,
    net: &mut Option<crate::net::NetState>,
    screen_mouse: Vec2,
    left_click: bool,
) -> DraftBanResult {
```

Then update all references from `opponent_bans` to `peer_bans` within the function body (lines 104, 108, 124, 133, 136).

- [ ] **Step 4: Update game_over.rs**

Replace `opponent_bans: None` with `peer_bans: None`:

```rust
GamePhase::DraftBan { bans: Vec::new(), confirmed: false, peer_bans: None }
```

- [ ] **Step 5: Run `cargo check`**

Run: `cargo check 2>&1`

Expected: Clean compilation (all files migrated).

- [ ] **Step 6: Run `cargo clippy`**

Run: `cargo clippy 2>&1`

Expected: Only pre-existing `too_many_arguments` warnings.

- [ ] **Step 7: Commit**

```bash
git add src/build_phase.rs src/economy.rs src/draft_ban.rs src/game_over.rs
git commit -m "refactor: migrate remaining files to array-indexed PlayerState"
```

---

### Task 9: Final verification and cleanup

**Files:**
- All source files (verification only)

- [ ] **Step 1: Search for any remaining old references**

Run these searches to verify nothing was missed:

```bash
grep -rn "\.host\b" src/ --include="*.rs" | grep -v "is_host"
grep -rn "\.guest\b" src/ --include="*.rs"
grep -rn "opponent_id()" src/ --include="*.rs"
grep -rn "\.player(role)" src/ --include="*.rs"
grep -rn "\.opponent(" src/ --include="*.rs"
grep -rn "OpponentBuildData" src/ --include="*.rs"
grep -rn "opponent_build" src/ --include="*.rs"
grep -rn "opponent_surrendered\|opponent_rematch\|opponent_bans\|opponent_color\|opponent_name" src/ --include="*.rs"
grep -rn "new_host\|new_guest" src/ --include="*.rs"
```

Expected: No matches for any of these patterns.

- [ ] **Step 2: Run cargo check**

Run: `cargo check 2>&1`

Expected: Clean compilation.

- [ ] **Step 3: Run cargo clippy**

Run: `cargo clippy 2>&1`

Expected: Only pre-existing `too_many_arguments` warnings.

- [ ] **Step 4: Commit any cleanup**

If any stray references were found and fixed:

```bash
git add -A
git commit -m "refactor: cleanup remaining host/guest references"
```
