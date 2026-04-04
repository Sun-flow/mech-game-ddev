# Extract Phase Logic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract all phase update logic from `main.rs` into dedicated modules using a `GameContext` struct, reducing main.rs from ~1,158 to ~400 lines.

**Architecture:** Introduce `GameContext` (shared mutable state) and `BattleState` (battle-phase-only state) structs. Each phase gets its own module with an `update()` function that takes `&mut GameContext` plus any phase-specific state. `main()` becomes a thin coordinator: init, input, dispatch, render, next_frame.

**Tech Stack:** Rust, macroquad

**Spec:** `docs/superpowers/specs/2026-04-04-extract-phase-logic-design.md`

---

### Task 1: Create `context.rs` with `GameContext` struct

**Files:**
- Create: `src/context.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/context.rs`**

```rust
use macroquad::prelude::*;

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
    pub mp_player_name: String,
    pub mp_opponent_name: String,
    pub chat: chat::ChatState,
}

impl GameContext {
    pub fn new(is_host: bool) -> Self {
        let progress = MatchProgress::new(is_host);
        let build = BuildState::new(progress.round_gold(), is_host);
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
            mp_player_name: String::from("Player"),
            mp_opponent_name: String::from("Opponent"),
            chat: chat::ChatState::new(),
        }
    }
}
```

- [ ] **Step 2: Add `mod context;` to main.rs and refactor locals into `GameContext`**

In `src/main.rs`, add `mod context;` to the module declarations. Then replace the individual local variables with a `GameContext` instance.

Replace these locals (lines 49-70):
```rust
let mut progress = MatchProgress::new(true);
let mut phase = GamePhase::Lobby;
let mut build = BuildState::new(progress.round_gold(), true);
let mut units: Vec<Unit> = Vec::new();
// ...
let mut net: Option<net::NetState> = None;
// ...
let mut game_settings = settings::GameSettings::default();
// ...
let mut obstacles: Vec<terrain::Obstacle> = Vec::new();
// ...
let mut mp_player_name = String::from("Player");
let mut mp_opponent_name = String::from("Opponent");
let mut chat = chat::ChatState::new();
let mut show_grid = false;
let mut nav_grid: Option<terrain::NavGrid> = None;
```

With:
```rust
let mut ctx = context::GameContext::new(true);
```

Keep these locals in main (they stay out of GameContext):
```rust
let mut lobby = lobby::LobbyState::new();
let mut battle_accumulator: f32 = 0.0;
let mut battle_timer: f32 = 0.0;
let mut battle_frame: u32 = 0;
const ROUND_TIMEOUT: f32 = 90.0;
const SYNC_INTERVAL: u32 = 4;
let mut recent_hashes: std::collections::VecDeque<(u32, u64)> = std::collections::VecDeque::with_capacity(5);
let mut main_settings = settings::MainSettings::default();
let mut show_surrender_confirm = false;
let mut camera_zoom: f32 = 1.0;
let mut camera_target = vec2(ARENA_W / 2.0, ARENA_H / 2.0);
let mut is_fullscreen_mode = false;
let mut splash_effects: Vec<SplashEffect> = Vec::new();
let mut pan_grab_world: Option<Vec2> = None;
let mut waiting_for_round_end = false;
let mut round_end_timeout: f32 = 0.0;
let mut projectiles: Vec<Projectile> = Vec::new();
```

Then do a mechanical find-and-replace throughout main.rs:
- `progress` → `ctx.progress` (but NOT inside closures/methods that already have a `progress` parameter)
- `phase` → `ctx.phase`
- `build` → `ctx.build`
- `units` → `ctx.units`
- `net` → `ctx.net`
- `obstacles` → `ctx.obstacles`
- `nav_grid` → `ctx.nav_grid`
- `game_settings` → `ctx.game_settings`
- `show_grid` → `ctx.show_grid`
- `mp_player_name` → `ctx.mp_player_name`
- `mp_opponent_name` → `ctx.mp_opponent_name`
- `chat` → `ctx.chat`

**Be careful with:** `net::send_build_complete` (the `net::` is a module path, not the variable), `net::NetMessage` (same), and any `&mut net` patterns which become `&mut ctx.net`.

The loop preamble references that need updating:
- `team::set_player_color(game_settings.player_color_index)` → `team::set_player_color(ctx.game_settings.player_color_index)`
- `if let Some(ref n) = net {` → `if let Some(ref n) = ctx.net {`
- `let mouse = if matches!(phase, ...)` → `let mouse = if matches!(ctx.phase, ...)`

Also update the rendering calls at the bottom of the loop:
- `rendering::update_splash_effects(&mut splash_effects, dt)` stays (splash_effects is still local)
- `rendering::draw_world(&units, &projectiles, &obstacles, &splash_effects, &build, &progress, show_grid, matches!(phase, ...), world_mouse)` → `rendering::draw_world(&ctx.units, &projectiles, &ctx.obstacles, &splash_effects, &ctx.build, &ctx.progress, ctx.show_grid, matches!(ctx.phase, ...), world_mouse)`
- Phase UI calls: update all `&progress` → `&ctx.progress`, `&build` → `&ctx.build`, etc.
- Chat calls: `chat.receive_from_net(...)` → `ctx.chat.receive_from_net(...)`, etc.

- [ ] **Step 3: Verify**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo check`
Expected: clean build with no errors.

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo clippy`
Fix any new warnings from the refactor.

- [ ] **Step 4: Commit**

```bash
git add src/context.rs src/main.rs
git commit -m "refactor: introduce GameContext struct, migrate locals from main()"
```

---

### Task 2: Create `BattleState` and migrate battle locals

**Files:**
- Create: `src/battle_phase.rs` (initially just the struct)
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/battle_phase.rs` with `BattleState` struct**

```rust
use macroquad::prelude::*;

use crate::projectile::Projectile;
use crate::rendering::SplashEffect;

pub const FIXED_DT: f32 = 1.0 / 60.0;
pub const ROUND_TIMEOUT: f32 = 90.0;
pub const SYNC_INTERVAL: u32 = 4;

pub struct BattleState {
    pub accumulator: f32,
    pub timer: f32,
    pub frame: u32,
    pub recent_hashes: std::collections::VecDeque<(u32, u64)>,
    pub show_surrender_confirm: bool,
    pub waiting_for_round_end: bool,
    pub round_end_timeout: f32,
    pub projectiles: Vec<Projectile>,
    pub splash_effects: Vec<SplashEffect>,
}

impl BattleState {
    pub fn new() -> Self {
        Self {
            accumulator: 0.0,
            timer: 0.0,
            frame: 0,
            recent_hashes: std::collections::VecDeque::with_capacity(5),
            show_surrender_confirm: false,
            waiting_for_round_end: false,
            round_end_timeout: 0.0,
            projectiles: Vec::new(),
            splash_effects: Vec::new(),
        }
    }

    /// Reset battle state for a new round. Called when entering Battle phase.
    pub fn reset(&mut self) {
        self.accumulator = 0.0;
        self.timer = 0.0;
        self.frame = 0;
        self.recent_hashes.clear();
        self.show_surrender_confirm = false;
        self.waiting_for_round_end = false;
        self.round_end_timeout = 0.0;
        self.projectiles.clear();
        self.splash_effects.clear();
    }
}
```

- [ ] **Step 2: Update `main.rs` to use `BattleState`**

Add `mod battle_phase;` to module declarations.

Replace the battle-related locals:
```rust
// Remove these:
let mut battle_accumulator: f32 = 0.0;
let mut battle_timer: f32 = 0.0;
let mut battle_frame: u32 = 0;
const ROUND_TIMEOUT: f32 = 90.0;
const SYNC_INTERVAL: u32 = 4;
let mut recent_hashes = ...;
let mut show_surrender_confirm = false;
let mut splash_effects: Vec<SplashEffect> = Vec::new();
let mut waiting_for_round_end = false;
let mut round_end_timeout: f32 = 0.0;
let mut projectiles: Vec<Projectile> = Vec::new();

// Add:
let mut battle = battle_phase::BattleState::new();
```

Remove `const FIXED_DT: f32 = 1.0 / 60.0;` from main.rs (now in battle_phase.rs).

Mechanical find-and-replace throughout main.rs:
- `battle_accumulator` → `battle.accumulator`
- `battle_timer` → `battle.timer`
- `battle_frame` → `battle.frame`
- `recent_hashes` → `battle.recent_hashes`
- `show_surrender_confirm` → `battle.show_surrender_confirm`
- `waiting_for_round_end` → `battle.waiting_for_round_end`
- `round_end_timeout` → `battle.round_end_timeout`
- `projectiles` → `battle.projectiles`
- `splash_effects` → `battle.splash_effects`
- `FIXED_DT` → `battle_phase::FIXED_DT`
- `ROUND_TIMEOUT` → `battle_phase::ROUND_TIMEOUT`
- `SYNC_INTERVAL` → `battle_phase::SYNC_INTERVAL`

Be careful with: `projectiles.is_empty()` → `battle.projectiles.is_empty()`, and the rendering call which passes `&projectiles` → `&battle.projectiles`, `&splash_effects` → `&battle.splash_effects`.

Also replace scattered reset patterns like:
```rust
battle_accumulator = 0.0;
battle_timer = 0.0;
battle_frame = 0;
recent_hashes.clear();
```
With: `battle.reset();`

There are 2 locations where this reset pattern appears:
1. Build phase timer expiry / begin-round button (lines ~307-310 and ~632-635) — when starting battle in single-player
2. WaitingForOpponent (lines ~673-676) — when entering battle in multiplayer

For the GameOver rematch handler (line ~1070-1073), replace the scattered clears:
```rust
show_surrender_confirm = false;
splash_effects.clear();
waiting_for_round_end = false;
projectiles.clear();
```
With: `battle.reset();`

Update `rendering::update_splash_effects(&mut splash_effects, dt)` → `rendering::update_splash_effects(&mut battle.splash_effects, dt)`

Clean up unused imports from main.rs: `SplashEffect`, `Projectile` may now only be used via battle_phase. Check before removing.

- [ ] **Step 3: Verify**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo check`
Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo clippy`
Fix any issues.

- [ ] **Step 4: Commit**

```bash
git add src/battle_phase.rs src/main.rs
git commit -m "refactor: introduce BattleState, migrate battle locals from main()"
```

---

### Task 3: Extract Build phase logic to `build_phase.rs`

**Files:**
- Create: `src/build_phase.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/build_phase.rs`**

Create the file with a single public `update` function. The function signature:

```rust
use macroquad::prelude::*;

use crate::arena::{ARENA_H, HALF_W, shop_w};
use crate::battle_phase::BattleState;
use crate::context::GameContext;
use crate::game_state;
use crate::net;
use crate::pack::all_packs;
use crate::terrain;
use crate::tech;
use crate::tech_ui;

pub fn update(
    ctx: &mut GameContext,
    battle: &mut BattleState,
    screen_mouse: Vec2,
    mouse: Vec2,
    left_click: bool,
    right_click: bool,
    middle_click: bool,
    dt: f32,
) {
    // Paste the ENTIRE contents of the GamePhase::Build match arm here
    // (lines 228-638 of current main.rs)
    //
    // Replace all `phase` with `ctx.phase`
    // Replace all `build` with `ctx.build`
    // Replace all `units` with `ctx.units`
    // Replace all `progress` with `ctx.progress`
    // Replace all `net` with `ctx.net`
    // Replace all `obstacles` with `ctx.obstacles`
    // Replace all `nav_grid` with `ctx.nav_grid`
    // Replace all `show_grid` with `ctx.show_grid`
    // Replace all `game_settings` with `ctx.game_settings`
    //
    // For battle reset patterns, use `battle.reset()`
    // For `projectiles`, use `battle.projectiles`
    //
    // The `economy::start_ai_battle` call needs updating:
    //   Its signature takes &mut projectiles, &mut obstacles, &mut nav_grid
    //   These now live in different places (projectiles in battle, obstacles/nav_grid in ctx)
    //   Either update the function signature in economy.rs, or pass the individual refs
}
```

The function body is the exact code from the `GamePhase::Build` match arm (lines 228-638), with variable names prefixed by `ctx.` or `battle.` as appropriate.

**Important:** The `economy::start_ai_battle()` call currently takes `&mut projectiles`, `&mut obstacles`, `&mut nav_grid` as separate params. Now `projectiles` is in `battle` and `obstacles`/`nav_grid` are in `ctx`. Update the call to pass `&mut battle.projectiles`, `&mut ctx.obstacles`, `&mut ctx.nav_grid`. Also update `economy::start_ai_battle`'s signature in `src/economy.rs` if needed (it may already use generic refs).

- [ ] **Step 2: Update `main.rs`**

Add `mod build_phase;` to module declarations.

Replace the `GamePhase::Build` match arm (lines 227-639) with:
```rust
GamePhase::Build => {
    build_phase::update(&mut ctx, &mut battle, screen_mouse, mouse, left_click, right_click, middle_click, dt);
}
```

- [ ] **Step 3: Verify**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo check`
Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo clippy`
Fix any issues.

- [ ] **Step 4: Commit**

```bash
git add src/build_phase.rs src/main.rs src/economy.rs
git commit -m "refactor: extract Build phase logic to build_phase.rs"
```

---

### Task 4: Extract Battle phase logic to `battle_phase.rs`

**Files:**
- Modify: `src/battle_phase.rs` (add `update` function)
- Modify: `src/main.rs`

- [ ] **Step 1: Add `update` function to `src/battle_phase.rs`**

Add below the existing `BattleState` impl:

```rust
use crate::arena::{check_match_state, MatchState, ARENA_H, ARENA_W};
use crate::combat::{update_attacks, update_movement, update_projectiles, update_targeting};
use crate::context::GameContext;
use crate::game_state::GamePhase;
use crate::match_progress::MatchProgress;
use crate::net;
use crate::sync;

pub fn update(ctx: &mut GameContext, battle: &mut BattleState, screen_mouse: Vec2, dt: f32) {
    // Paste the ENTIRE contents of the GamePhase::Battle match arm here
    // (lines 690-971 of current main.rs)
    //
    // Replace variable references with ctx.* or battle.* prefixes
    // Key mappings:
    //   show_surrender_confirm → battle.show_surrender_confirm
    //   battle_accumulator → battle.accumulator
    //   battle_timer → battle.timer
    //   battle_frame → battle.frame
    //   recent_hashes → battle.recent_hashes
    //   waiting_for_round_end → battle.waiting_for_round_end
    //   round_end_timeout → battle.round_end_timeout
    //   projectiles → battle.projectiles
    //   splash_effects → battle.splash_effects
    //   units → ctx.units
    //   obstacles → ctx.obstacles
    //   nav_grid → ctx.nav_grid
    //   progress → ctx.progress
    //   net → ctx.net
    //   phase → ctx.phase
    //   FIXED_DT, ROUND_TIMEOUT, SYNC_INTERVAL → use directly (they're in this module)
}
```

- [ ] **Step 2: Update `main.rs`**

Replace the `GamePhase::Battle` match arm with:
```rust
GamePhase::Battle => {
    battle_phase::update(&mut ctx, &mut battle, screen_mouse, dt);
}
```

Also update the phase_ui call for Battle which references `battle_timer` and `ROUND_TIMEOUT`:
```rust
GamePhase::Battle => {
    phase_ui::draw_battle_ui(&ctx.progress, &ctx.units, &ctx.obstacles, battle.timer, battle_phase::ROUND_TIMEOUT, battle.show_surrender_confirm, screen_mouse, world_mouse, &ctx.mp_player_name, &ctx.mp_opponent_name);
}
```

- [ ] **Step 3: Verify**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo check`
Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo clippy`
Fix any issues.

- [ ] **Step 4: Commit**

```bash
git add src/battle_phase.rs src/main.rs
git commit -m "refactor: extract Battle phase logic to battle_phase.rs"
```

---

### Task 5: Extract remaining phases (WaitingForOpponent, RoundResult, GameOver)

**Files:**
- Create: `src/waiting_phase.rs`
- Create: `src/round_result.rs`
- Create: `src/game_over.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/waiting_phase.rs`**

```rust
use macroquad::prelude::*;

use crate::arena::{ARENA_H, ARENA_W};
use crate::battle_phase::BattleState;
use crate::context::GameContext;
use crate::game_state::GamePhase;
use crate::terrain;

pub fn update(ctx: &mut GameContext, battle: &mut BattleState) {
    // Paste the contents of GamePhase::WaitingForOpponent match arm
    // (lines 641-688 of current main.rs)
    //
    // Replace: net → ctx.net, progress → ctx.progress, units → ctx.units,
    //   obstacles → ctx.obstacles, nav_grid → ctx.nav_grid,
    //   game_settings → ctx.game_settings, phase → ctx.phase,
    //   projectiles → battle.projectiles
    //
    // Replace the battle reset pattern with battle.reset()
}
```

- [ ] **Step 2: Create `src/round_result.rs`**

```rust
use macroquad::prelude::*;

use crate::battle_phase::BattleState;
use crate::context::GameContext;
use crate::game_state::{BuildState, GamePhase};

pub fn update(ctx: &mut GameContext, battle: &mut BattleState) {
    // Paste the contents of GamePhase::RoundResult match arm
    // (lines 974-1039 of current main.rs)
    //
    // Replace: progress → ctx.progress, build → ctx.build, units → ctx.units,
    //   net → ctx.net, phase → ctx.phase, projectiles → battle.projectiles
}
```

Note: RoundResult needs `&mut BattleState` because it calls `battle.projectiles.clear()` (line 1035).

- [ ] **Step 3: Create `src/game_over.rs`**

```rust
use macroquad::prelude::*;

use crate::battle_phase::BattleState;
use crate::context::GameContext;
use crate::game_state::{BuildState, GamePhase};
use crate::lobby;
use crate::match_progress::MatchProgress;

pub fn update(
    ctx: &mut GameContext,
    battle: &mut BattleState,
    lobby: &mut lobby::LobbyState,
    screen_mouse: Vec2,
    left_click: bool,
) {
    // Paste the contents of GamePhase::GameOver match arm
    // (lines 1041-1080 of current main.rs)
    //
    // Replace: progress → ctx.progress, phase → ctx.phase, build → ctx.build,
    //   units → ctx.units, net → ctx.net, game_settings → ctx.game_settings,
    //   chat → ctx.chat, obstacles → ctx.obstacles, nav_grid → ctx.nav_grid
    //
    // For the rematch reset, call battle.reset() instead of individual clears
    //
    // R key return-to-lobby: lobby.reset(), set ctx.net = None, etc.
}
```

- [ ] **Step 4: Update `main.rs`**

Add `mod waiting_phase;`, `mod round_result;`, `mod game_over;` to module declarations.

Replace the three match arms with:
```rust
GamePhase::WaitingForOpponent => {
    waiting_phase::update(&mut ctx, &mut battle);
}

GamePhase::RoundResult { .. } => {
    round_result::update(&mut ctx, &mut battle);
}

GamePhase::GameOver(_) => {
    game_over::update(&mut ctx, &mut battle, &mut lobby, screen_mouse, left_click);
}
```

Also update the disconnect overlay handler (lines 1132-1147) — it resets state similar to GameOver. Extract the reset into a helper or inline it with `ctx.*` prefixes.

- [ ] **Step 5: Verify**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo check`
Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo clippy`
Fix any issues.

- [ ] **Step 6: Commit**

```bash
git add src/waiting_phase.rs src/round_result.rs src/game_over.rs src/main.rs
git commit -m "refactor: extract WaitingForOpponent, RoundResult, GameOver phases"
```

---

### Task 6: Clean up main.rs and update dependent modules

**Files:**
- Modify: `src/main.rs` (final cleanup)
- Modify: `src/rendering.rs` (update draw_world signature if needed)
- Modify: `src/phase_ui.rs` (update function signatures if needed)
- Modify: `src/economy.rs` (update start_ai_battle signature if needed)

- [ ] **Step 1: Clean up main.rs imports**

Remove all `use` imports that are no longer needed in main.rs. After the extraction, main.rs should only need imports for:
- `macroquad::prelude::*`
- `context::GameContext`
- `battle_phase::BattleState`
- `game_state::GamePhase`
- `arena::{ARENA_H, ARENA_W}`
- Types needed for the Lobby arm (which stays inline)
- Types needed for the rendering/UI dispatch calls

Grep for each import to confirm it's still used before removing.

- [ ] **Step 2: Update rendering calls**

The `rendering::draw_world` function currently takes individual refs. After the refactor, the call in main.rs should pass from the right locations:

```rust
rendering::draw_world(
    &ctx.units,
    &battle.projectiles,
    &ctx.obstacles,
    &battle.splash_effects,
    &ctx.build,
    &ctx.progress,
    ctx.show_grid,
    matches!(ctx.phase, GamePhase::Build),
    world_mouse,
);
```

Similarly for `rendering::update_splash_effects(&mut battle.splash_effects, dt)`.

- [ ] **Step 3: Verify final state**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo check`
Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo clippy`

Check: `wc -l src/main.rs` — should be ~350-450 lines.

Fix any remaining issues.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor: final main.rs cleanup after phase extraction"
```
