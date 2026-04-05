# PlayerState & Host/Guest Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace player/opponent perspective model with canonical host/guest state, introduce PlayerState struct, camera flip for guest rendering, simplify network sync.

**Architecture:** All per-player state consolidates into `PlayerState` structs owned by `MatchProgress` (host + guest). A `Role` enum on `GameContext` determines perspective. Guest client uses negative x-zoom camera flip. State sync drops all coordinate mirroring.

**Tech Stack:** Rust, macroquad, bincode (serialization)

**Spec:** `docs/superpowers/specs/2026-04-05-playerstate-host-guest-design.md`

---

### Task 1: Add Role enum and player_id rename on Unit

**Files:**
- Create: `src/role.rs`
- Modify: `src/unit.rs:69,90-100`
- Modify: `src/main.rs:1-28` (mod declaration)

- [ ] **Step 1: Create src/role.rs**

```rust
use crate::arena::{ARENA_W, HALF_W};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Role {
    Host,
    Guest,
    Spectator,
}

impl Role {
    pub fn deploy_x_range(&self) -> (f32, f32) {
        match self {
            Role::Host => (0.0, HALF_W),
            Role::Guest => (HALF_W, ARENA_W),
            Role::Spectator => (0.0, 0.0),
        }
    }

    pub fn player_id(&self) -> u8 {
        match self {
            Role::Host => 0,
            Role::Guest => 1,
            Role::Spectator => 255,
        }
    }
}
```

- [ ] **Step 2: Add mod declaration in main.rs**

Add `mod role;` to the module declarations at the top of `src/main.rs`.

- [ ] **Step 3: Rename team_id to player_id on Unit**

In `src/unit.rs`, rename the `team_id` field to `player_id` on the `Unit` struct (line 69) and in `Unit::new()` (lines 90, 100).

- [ ] **Step 4: Fix all team_id references across codebase**

Search for all `team_id` references in `src/*.rs` and rename to `player_id`. This is a mechanical find-and-replace across ~19 files. Key files: `combat.rs`, `battle_phase.rs`, `build_phase.rs`, `rendering.rs`, `sync.rs`, `phase_ui.rs`, `terrain.rs`, `pack.rs`, `match_progress.rs`, `waiting_phase.rs`, `economy.rs`, `game_state.rs`, `net.rs`.

Also rename in the `Obstacle` struct in `terrain.rs` and the `Projectile` struct in `projectile.rs`.

- [ ] **Step 5: Run cargo check, fix any compilation errors**

Run: `cargo check`
Expected: Clean compilation after all renames.

- [ ] **Step 6: Commit**

```bash
git add src/role.rs src/unit.rs src/main.rs src/*.rs
git commit -m "refactor: add Role enum, rename team_id to player_id"
```

---

### Task 2: Create PlayerState struct and restructure MatchProgress

**Files:**
- Modify: `src/match_progress.rs:33-71`
- Modify: `src/game_state.rs:31-39` (PlacedPack unification)

- [ ] **Step 1: Unify PlacedPack — remove OpponentPlacedPack**

In `src/match_progress.rs`, delete the `OpponentPlacedPack` struct (lines 34-41). All references to `OpponentPlacedPack` will use `game_state::PlacedPack` instead. The existing `PlacedPack` in `game_state.rs` already has all needed fields: `pack_index`, `center`, `unit_ids`, `rotated`, `locked`, `round_placed`.

Remove `pre_drag_center` from `PlacedPack` in `game_state.rs` — move it to `BuildState` as a separate tracking field (it's build-session UI state, not pack identity). Add `drag_pre_centers: Vec<Vec2>` to `BuildState` to track pre-drag positions by index.

- [ ] **Step 2: Create PlayerState struct in match_progress.rs**

Replace the paired fields in `MatchProgress` with:

```rust
use crate::game_state::PlacedPack;

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub player_id: u8,
    pub lp: i32,
    pub techs: TechState,
    pub name: String,
    pub next_id: u64,
    pub gold: u32,
    pub packs: Vec<PlacedPack>,
    pub ai_memory: AiMemory,
}

impl PlayerState {
    pub fn new_host() -> Self {
        Self {
            player_id: 0,
            lp: STARTING_LP,
            techs: TechState::new(),
            name: "Player".to_string(),
            next_id: 1,
            gold: 0,
            packs: Vec::new(),
            ai_memory: AiMemory::default(),
        }
    }

    pub fn new_guest() -> Self {
        Self {
            player_id: 1,
            lp: STARTING_LP,
            techs: TechState::new(),
            name: "Opponent".to_string(),
            next_id: 100_000,
            gold: 0,
            packs: Vec::new(),
            ai_memory: AiMemory::default(),
        }
    }
}
```

- [ ] **Step 3: Restructure MatchProgress**

Replace the current `MatchProgress` struct with:

```rust
#[derive(Clone, Debug)]
pub struct MatchProgress {
    pub round: u32,
    pub host: PlayerState,
    pub guest: PlayerState,
    pub banned_kinds: Vec<UnitKind>,
}

impl MatchProgress {
    pub fn new() -> Self {
        Self {
            round: 1,
            host: PlayerState::new_host(),
            guest: PlayerState::new_guest(),
            banned_kinds: Vec::new(),
        }
    }

    pub fn round_allowance(&self) -> u32 {
        200 * self.round
    }

    pub fn player(&self, role: crate::role::Role) -> &PlayerState {
        match role {
            crate::role::Role::Host => &self.host,
            _ => &self.guest,
        }
    }

    pub fn player_mut(&mut self, role: crate::role::Role) -> &mut PlayerState {
        match role {
            crate::role::Role::Host => &mut self.host,
            _ => &mut self.guest,
        }
    }

    pub fn opponent(&self, role: crate::role::Role) -> &PlayerState {
        match role {
            crate::role::Role::Host => &self.guest,
            _ => &self.host,
        }
    }

    pub fn opponent_mut(&mut self, role: crate::role::Role) -> &mut PlayerState {
        match role {
            crate::role::Role::Host => &mut self.guest,
            _ => &mut self.host,
        }
    }
}
```

Remove `MatchProgress::new(is_host: bool)` — the new constructor takes no args. Remove `round_gold()` — callers compute `player.gold + progress.round_allowance()` directly or gold is managed as a live balance on PlayerState.

- [ ] **Step 4: Move respawn and LP methods to use PlayerState**

Rewrite `respawn_opponent_units()` as a method on `PlayerState`:

```rust
impl PlayerState {
    pub fn respawn_units(&self) -> Vec<Unit> {
        let packs_defs = all_packs();
        let mut units = Vec::new();
        for placed in &self.packs {
            let pack = &packs_defs[placed.pack_index];
            let spawned = crate::pack::respawn_pack_units(
                pack, placed.center, placed.rotated, self.player_id,
                &self.techs, &placed.unit_ids,
            );
            units.extend(spawned);
        }
        units
    }
}
```

Move `calculate_lp_damage` to a standalone function (it doesn't depend on MatchProgress state — it takes a unit slice and player_id).

Move `is_game_over()` and `game_winner()` to check `host.lp` and `guest.lp` directly.

- [ ] **Step 5: Run cargo check — expect many errors from callers**

This will break all code referencing old `MatchProgress` fields. That's expected — we fix callers in subsequent tasks.

Run: `cargo check 2>&1 | head -50`

- [ ] **Step 6: Commit (compiles with errors — checkpoint)**

```bash
git add src/match_progress.rs src/game_state.rs
git commit -m "refactor: introduce PlayerState struct, unify PlacedPack type"
```

---

### Task 3: Add Role to GameContext, remove player/opponent names

**Files:**
- Modify: `src/context.rs:9-70`

- [ ] **Step 1: Add Role to GameContext, remove name fields**

In `src/context.rs`, add `role: Role` field to `GameContext`. Remove `mp_player_name` and `mp_opponent_name` — these now live on `progress.host.name` and `progress.guest.name`.

```rust
pub struct GameContext {
    pub role: Role,
    pub progress: MatchProgress,
    pub phase: GamePhase,
    pub build: BuildState,
    pub units: Vec<Unit>,
    pub net: Option<net::NetState>,
    pub obstacles: Vec<terrain::Obstacle>,
    pub nav_grid: Option<terrain::NavGrid>,
    pub game_settings: settings::GameSettings,
    pub show_grid: bool,
    pub chat: chat::ChatState,
}
```

- [ ] **Step 2: Update GameContext::new() and start_game()**

`GameContext::new()` takes no `is_host` parameter — defaults to `Role::Host`. `start_game()` sets `self.role` based on connection state.

```rust
impl GameContext {
    pub fn new() -> Self {
        let progress = MatchProgress::new();
        let build = BuildState::new();
        Self {
            role: Role::Host,
            progress,
            phase: GamePhase::Lobby,
            build,
            units: Vec::new(),
            net: None,
            obstacles: Vec::new(),
            nav_grid: None,
            game_settings: settings::GameSettings::default(),
            show_grid: false,
            chat: chat::ChatState::new(),
        }
    }

    pub fn start_game(
        &mut self,
        net: Option<net::NetState>,
        is_host: bool,
        player_name: String,
        draft_ban_enabled: bool,
    ) {
        self.role = if is_host { Role::Host } else { Role::Guest };
        self.net = net;
        if let Some(ref mut n) = self.net {
            n.is_host = is_host;
            self.progress.opponent_mut(self.role).name =
                n.opponent_name.clone().unwrap_or_else(|| "Opponent".to_string());
        }
        self.progress.player_mut(self.role).name = player_name;
        self.progress = MatchProgress::new();
        // Set gold for first round
        let allowance = self.progress.round_allowance();
        self.progress.host.gold = allowance;
        self.progress.guest.gold = allowance;
        self.build = BuildState::new();
        if draft_ban_enabled {
            self.phase = GamePhase::DraftBan {
                bans: Vec::new(),
                confirmed: false,
                opponent_bans: None,
            };
        } else {
            self.phase = GamePhase::Build;
        }
    }
}
```

- [ ] **Step 3: Run cargo check — note remaining errors**

Run: `cargo check 2>&1 | head -50`

- [ ] **Step 4: Commit**

```bash
git add src/context.rs
git commit -m "refactor: add Role to GameContext, remove player/opponent name fields"
```

---

### Task 4: Slim down BuildState

**Files:**
- Modify: `src/game_state.rs:84-385`
- Modify: `src/economy.rs:5-275`

- [ ] **Step 1: Remove packs, gold, and next_id from BuildState**

Rewrite `BuildState` to contain only session UI state:

```rust
pub struct BuildState {
    pub timer: f32,
    pub selected_pack: Option<usize>,
    pub dragging: Option<usize>,
    pub drag_offset: Vec2,
    pub drag_pre_centers: Vec<Vec2>,
    pub multi_dragging: Vec<usize>,
    pub multi_drag_offsets: Vec<Vec2>,
    pub multi_drag_pre_centers: Vec<Vec2>,
    pub drag_box_start: Option<Vec2>,
    pub round_tech_purchases: Vec<(crate::unit::UnitKind, crate::tech::TechId)>,
    pub undo_history: Vec<UndoEntry>,
    pub packs_bought_this_round: u32,
}
```

`BuildState::new()` takes no parameters — all fields are default/empty.

`BuildState::new_round()` takes no parameters — resets timer, clears drag state, clears undo history, resets `packs_bought_this_round`.

- [ ] **Step 2: Rewrite BuildState methods to take &mut PlayerState**

All methods that previously operated on `self.placed_packs`, `self.builder.gold_remaining`, or `self.next_id` now take `player: &mut PlayerState` as a parameter:

- `purchase_pack(&mut self, player: &mut PlayerState, pack_index: usize, round: u32) -> Option<Vec<Unit>>`
- `sell_pack(&mut self, player: &mut PlayerState, placed_idx: usize) -> Option<(usize, Vec<u64>)>`
- `rotate_pack(&self, player: &mut PlayerState, placed_idx: usize, units: &mut [Unit])`
- `reposition_pack_units(&self, player: &PlayerState, placed_index: usize, units: &mut [Unit])`
- `pack_at(&self, player: &PlayerState, point: Vec2) -> Option<usize>`
- `lock_current_packs(player: &mut PlayerState)` — becomes a standalone function or method on PlayerState
- `respawn_player_units` — already moved to `PlayerState::respawn_units()` in Task 2

Inside these methods, replace `self.placed_packs` with `player.packs`, `self.builder.gold_remaining` with `player.gold`, `self.next_id` with `player.next_id`.

- [ ] **Step 3: Remove ArmyBuilder from economy.rs**

Remove the `ArmyBuilder` struct and its methods. Update `random_army_filtered()` and `smart_army()` to return a `Vec<usize>` of pack indices (or a simple list of chosen packs) instead of an `ArmyBuilder`. Update `start_ai_battle()` to operate on `&mut PlayerState` directly:

```rust
pub fn start_ai_battle(
    progress: &mut MatchProgress,
    units: &mut Vec<Unit>,
) {
    let guest = &mut progress.guest;
    // Clear old guest units
    units.retain(|u| u.player_id == 0);
    // Respawn existing packs
    units.extend(guest.respawn_units());
    // Buy techs with guest gold
    ai_buy_techs(&mut guest.gold, &mut guest.techs, ...);
    // Build new army
    let new_packs = smart_army(guest.gold, &guest.ai_memory, &progress.banned_kinds);
    // Spawn units from chosen packs, add to guest.packs
    for pack_index in new_packs {
        let pack = &all_packs()[pack_index];
        // ... spawn at position, add to guest.packs, deduct guest.gold
    }
}
```

- [ ] **Step 4: Run cargo check, fix compilation errors**

Run: `cargo check`

- [ ] **Step 5: Commit**

```bash
git add src/game_state.rs src/economy.rs
git commit -m "refactor: slim BuildState, remove ArmyBuilder, methods take &mut PlayerState"
```

---

### Task 5: Update all callers — build_phase, round_result, waiting_phase

**Files:**
- Modify: `src/build_phase.rs`
- Modify: `src/round_result.rs`
- Modify: `src/waiting_phase.rs`

- [ ] **Step 1: Update build_phase.rs**

All references to `ctx.build.placed_packs` become `me.packs` where `me = ctx.progress.player_mut(ctx.role)`. All references to `ctx.build.builder.gold_remaining` become `me.gold`. All `HALF_W` clamp references use `ctx.role.deploy_x_range()`.

Key changes:
- Pack purchase: `ctx.build.purchase_pack(me, pack_index, ctx.progress.round)`
- Pack sell: `ctx.build.sell_pack(me, placed_idx)`
- Pack at: `ctx.build.pack_at(me, mouse)`
- Gold display: `me.gold`
- Tech purchase: `me.techs.purchase(kind, tech_id)`, deduct from `me.gold`
- Deploy clamp: `let (min_x, max_x) = ctx.role.deploy_x_range();` then clamp `x` to `min_x + half.x .. max_x - half.x`

- [ ] **Step 2: Update round_result.rs**

Replace `ctx.progress.player_saved_gold = ctx.build.builder.gold_remaining` — gold is already live on PlayerState, no save needed. Round advancement adds allowance:

```rust
let me = ctx.progress.player_mut(ctx.role);
me.gold += ctx.progress.round_allowance(); // carry-over + new allowance
```

Replace `ctx.build.respawn_player_units(&ctx.progress.player_techs)` with `me.respawn_units()`.

Replace `ctx.progress.respawn_opponent_units()` with `ctx.progress.opponent(ctx.role).respawn_units()`.

Replace `ctx.build = BuildState::new_round(...)` with `ctx.build = BuildState::new_round()` (no params).

Lock packs: `PlayerState::lock_packs(&mut me)` or `me.lock_packs()`.

- [ ] **Step 3: Update waiting_phase.rs**

Replace `ctx.progress.apply_opponent_build(&opp_build)` — this method moves to `MatchProgress` and updates the opponent's `PlayerState` directly.

Replace `ctx.units.retain(|u| u.team_id == 0)` with `ctx.units.retain(|u| u.player_id == ctx.role.player_id())` — keep only local player's units, then respawn opponent.

- [ ] **Step 4: Run cargo check, fix compilation errors**

Run: `cargo check`

- [ ] **Step 5: Commit**

```bash
git add src/build_phase.rs src/round_result.rs src/waiting_phase.rs
git commit -m "refactor: update build/round/waiting phases for PlayerState"
```

---

### Task 6: Update battle_phase, game_over, main.rs

**Files:**
- Modify: `src/battle_phase.rs`
- Modify: `src/game_over.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Update battle_phase.rs**

Replace all `ctx.progress.player_lp` / `ctx.progress.opponent_lp` with `ctx.progress.host.lp` / `ctx.progress.guest.lp` (canonical — not perspective-relative).

LP damage assignment after battle: determine winner by unit survival, apply damage to the losing player's `lp` field using canonical host/guest.

Replace `is_host_game` checks with `ctx.role`.

- [ ] **Step 2: Update game_over.rs**

Replace `MatchProgress::new(is_host)` with `MatchProgress::new()`. Replace `BuildState::new(gold, is_host)` with `BuildState::new()`. Set `ctx.role` appropriately for rematch.

- [ ] **Step 3: Update main.rs**

Replace `GameContext::new(true)` with `GameContext::new()`.

In `start_game` calls, remove `is_host` parameter threading — `start_game` sets `ctx.role` internally.

Replace all `ctx.mp_player_name` / `ctx.mp_opponent_name` references with `ctx.progress.player(ctx.role).name` / `ctx.progress.opponent(ctx.role).name`.

- [ ] **Step 4: Run cargo check, fix compilation errors**

Run: `cargo check`

- [ ] **Step 5: Commit**

```bash
git add src/battle_phase.rs src/game_over.rs src/main.rs
git commit -m "refactor: update battle/gameover/main for PlayerState and Role"
```

---

### Task 7: Update UI code — phase_ui, ui, rendering, shop, tech_ui

**Files:**
- Modify: `src/phase_ui.rs`
- Modify: `src/ui.rs`
- Modify: `src/rendering.rs`
- Modify: `src/shop.rs`
- Modify: `src/tech_ui.rs`

- [ ] **Step 1: Update phase_ui.rs**

Replace all `mp_player_name` / `mp_opponent_name` parameters with reads from `PlayerState`. The `draw_*` functions should take `&MatchProgress` and `Role` instead of separate name strings.

Replace `progress.player_lp` / `progress.opponent_lp` with `progress.player(role).lp` / `progress.opponent(role).lp`.

Replace opponent pack label rendering — iterate `progress.opponent(role).packs` instead of `progress.opponent_packs`.

- [ ] **Step 2: Update ui.rs**

Update `draw_hud` to read LP from `PlayerState` references instead of separate fields.

- [ ] **Step 3: Update rendering.rs**

In `draw_build_overlays`, use role-derived deploy zone colors:
- Local player's half gets blue overlay
- Opponent's half gets red overlay
- With canonical coordinates, host blue is left (0..HALF_W), guest blue is right (HALF_W..ARENA_W)

- [ ] **Step 4: Run cargo check, fix remaining errors**

Run: `cargo check`

- [ ] **Step 5: Commit**

```bash
git add src/phase_ui.rs src/ui.rs src/rendering.rs src/shop.rs src/tech_ui.rs
git commit -m "refactor: update UI code for PlayerState and Role"
```

---

### Task 8: Camera flip for guest

**Files:**
- Modify: `src/main.rs` (camera construction)

- [ ] **Step 1: Apply negative x-zoom for guest**

In the camera construction in `src/main.rs`, flip x-zoom for guest:

```rust
let x_flip = if ctx.role == role::Role::Guest { -1.0 } else { 1.0 };
let arena_camera = Camera2D {
    target: camera_target,
    zoom: vec2(
        camera_zoom * 2.0 / screen_width() * x_flip,
        camera_zoom * 2.0 / screen_height(),
    ),
    ..Default::default()
};
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check`

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: camera flip for guest perspective via negative x-zoom"
```

---

### Task 9: Deploy zone parameterization

**Files:**
- Modify: `src/build_phase.rs`
- Modify: `src/game_state.rs`

- [ ] **Step 1: Replace all HALF_W clamps with role-derived bounds**

In `src/build_phase.rs`, replace:
```rust
mouse.x.clamp(half.x, HALF_W - half.x)
```
with:
```rust
let (deploy_min, deploy_max) = ctx.role.deploy_x_range();
mouse.x.clamp(deploy_min + half.x, deploy_max - half.x)
```

Apply the same pattern to all HALF_W references in:
- `build_phase.rs` lines 214, 341 (drag clamp, pack follow)
- `game_state.rs` lines 170, 173, 291 (purchase_pack placement, rotate_pack clamp)

- [ ] **Step 2: Update rendering deploy zone overlays**

In `src/rendering.rs` `draw_build_overlays`, the deploy zone colors should reflect the local player's perspective. Since the camera is flipped for guest, drawing the host zone (0..HALF_W) as blue and guest zone (HALF_W..ARENA_W) as red in canonical coordinates will appear correct on both clients.

- [ ] **Step 3: Run cargo check**

Run: `cargo check`

- [ ] **Step 4: Commit**

```bash
git add src/build_phase.rs src/game_state.rs src/rendering.rs
git commit -m "feat: deploy zone parameterization from Role"
```

---

### Task 10: Simplify state sync — remove mirroring

**Files:**
- Modify: `src/sync.rs`
- Modify: `src/battle_phase.rs`
- Modify: `src/match_progress.rs`
- Modify: `src/net.rs`

- [ ] **Step 1: Remove mirror parameter from sync functions**

In `src/sync.rs`:
- `compute_state_hash` — remove `mirror: bool` parameter. Remove all `if mirror` branches. Hash canonical data directly.
- `apply_state_sync` — remove `mirror: bool` parameter. Remove all `if mirror` branches. Apply data directly without coordinate flipping or player_id swapping.

- [ ] **Step 2: Update battle_phase.rs sync calls**

Remove `true`/`false` mirror arguments from `compute_state_hash` and `apply_state_sync` calls. Both host and guest now call with the same canonical data.

- [ ] **Step 3: Remove coordinate mirroring from apply_opponent_build**

In `src/match_progress.rs`, the `apply_opponent_build` method currently mirrors x-coordinates (`ARENA_W - cx`). Remove this — opponent sends canonical coordinates (already on the correct side of the arena).

- [ ] **Step 4: Update net.rs BuildComplete to use canonical coordinates**

The `send_build_complete` function in `src/net.rs` already sends raw coordinates. Verify it doesn't do any transformation. The receiver in `apply_opponent_build` no longer mirrors — it stores directly.

- [ ] **Step 5: Run cargo check**

Run: `cargo check`

- [ ] **Step 6: Run cargo clippy**

Run: `cargo clippy`

- [ ] **Step 7: Commit**

```bash
git add src/sync.rs src/battle_phase.rs src/match_progress.rs src/net.rs
git commit -m "refactor: remove state sync mirroring, canonical coordinates throughout"
```

---

### Task 11: Final verification and cleanup

**Files:**
- All source files

- [ ] **Step 1: Full cargo check**

Run: `cargo check`
Expected: Clean compilation, zero errors.

- [ ] **Step 2: Full cargo clippy**

Run: `cargo clippy`
Expected: Only pre-existing `too_many_arguments` warnings (4 remaining). No new warnings.

- [ ] **Step 3: Search for any remaining old patterns**

Search for remnants of the old model:
```
grep -r "team_id" src/
grep -r "player_lp\|opponent_lp" src/
grep -r "player_techs\|opponent_techs" src/
grep -r "mp_player_name\|mp_opponent_name" src/
grep -r "OpponentPlacedPack" src/
grep -r "ArmyBuilder" src/
grep -r "player_saved_gold" src/
```
Expected: Zero matches for all patterns.

- [ ] **Step 4: Update documentation**

Run `/update-docs` to sync TASKS.md, PLANNING.md, CHANGELOG.md.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "refactor: complete PlayerState & host/guest architecture migration"
```
