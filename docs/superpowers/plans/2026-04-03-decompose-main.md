# Decompose main.rs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Break `src/main.rs` (2,249 lines) into focused modules so it becomes a ~350-line game loop coordinator.

**Architecture:** Extract phase logic into per-phase modules (`build_phase.rs`, `battle_phase.rs`, `draft_ban.rs`), rendering into `rendering.rs`, UI overlays into `phase_ui.rs`, chat into `chat.rs`, and move helper functions to their natural homes (`ui.rs`, `unit.rs`, `arena.rs`, `tech.rs`, `net.rs`, `economy.rs`). Each extraction is a standalone refactor that compiles and runs after completion. The game loop in `main()` calls into these modules.

**Tech Stack:** Rust, macroquad (immediate-mode 2D game framework), matchbox_socket (WebRTC networking)

**Key constraint:** This is an immediate-mode game loop with a single `async fn main()` containing ~30 mutable local variables. Phase handlers need access to subsets of this state. We'll pass state as function arguments (not a god struct) to keep module boundaries clean and avoid a large refactor of the state model.

---

## File Structure

### New files to create
| File | Responsibility |
|------|---------------|
| `src/rendering.rs` | World-space rendering: arena border, center divider, grid, shields, units, projectiles, splash effects, build overlays |
| `src/phase_ui.rs` | Screen-space UI per phase: build UI, waiting UI, battle HUD/tooltips/surrender overlay, round result, game over, disconnect overlay |
| `src/build_phase.rs` | Build phase update logic: undo, timer, shop interaction, tech panel, sell, rotate, drag systems, begin-round button |
| `src/battle_phase.rs` | Battle phase update logic: simulation loop (SP & MP), sync hashing, desync detection, surrender handling, round end |
| `src/draft_ban.rs` | Draft/ban phase: unit selection UI, network sync, transition |
| `src/chat.rs` | Chat system: input handling, network send/receive, rendering |

### Existing files to modify
| File | Changes |
|------|---------|
| `src/main.rs` | Becomes ~350-line coordinator: mod declarations, window_conf, state init, game loop dispatching to modules |
| `src/ui.rs` | Receives `draw_hud()` |
| `src/unit.rs` | Receives `draw_unit_shape()` |
| `src/arena.rs` | Receives `draw_center_divider()` |
| `src/tech.rs` | Receives `refresh_units_of_kind()` |
| `src/net.rs` | Receives `send_build_complete()` |
| `src/economy.rs` | Receives `start_battle_ai()` (renamed to `start_ai_battle()`) |

### What stays in main.rs
- All `mod` declarations (moved to top, including `settings`/`terrain`)
- `SplashEffect` struct
- `window_conf()`
- `async fn main()` containing: state initialization, the game loop skeleton, camera setup, input polling, `match phase` dispatching to module functions, frame-end `next_frame().await`

---

## Phased Execution

This plan is broken into 6 phases. Each phase is independently compilable. Complete one phase, run `cargo check` + `cargo clippy`, commit, then proceed. **Max 5 files touched per phase.**

---

### Task 1: Move helper functions to their natural modules

Move 6 small functions out of main.rs into existing modules. This is the "Step 0" dead code / cleanup pass.

**Files:**
- Modify: `src/main.rs` (remove functions, add use imports)
- Modify: `src/ui.rs` (add `draw_hud`)
- Modify: `src/unit.rs` (add `draw_unit_shape`)
- Modify: `src/arena.rs` (add `draw_center_divider`)
- Modify: `src/tech.rs` (add `refresh_units_of_kind`)

**Note:** `send_build_complete` and `start_battle_ai` depend on types from multiple modules and will move in later tasks when their destination modules are created.

- [ ] **Step 1: Move `draw_unit_shape` to `unit.rs`**

Cut lines 2141-2188 from `main.rs`. Add to end of `src/unit.rs`:

```rust
use macroquad::prelude::*;

pub fn draw_unit_shape(pos: Vec2, size: f32, shape: UnitShape, color: Color) {
    match shape {
        UnitShape::Circle => draw_circle(pos.x, pos.y, size, color),
        UnitShape::Square => {
            draw_rectangle(pos.x - size, pos.y - size, size * 2.0, size * 2.0, color)
        }
        UnitShape::Triangle => {
            draw_triangle(
                vec2(pos.x, pos.y - size),
                vec2(pos.x - size, pos.y + size),
                vec2(pos.x + size, pos.y + size),
                color,
            );
        }
        UnitShape::Diamond => {
            let top = vec2(pos.x, pos.y - size * 1.3);
            let right = vec2(pos.x + size, pos.y);
            let bottom = vec2(pos.x, pos.y + size * 1.3);
            let left = vec2(pos.x - size, pos.y);
            draw_triangle(top, right, bottom, color);
            draw_triangle(top, left, bottom, color);
        }
        UnitShape::Hexagon => draw_poly(pos.x, pos.y, 6, size, 0.0, color),
        UnitShape::Pentagon => draw_poly(pos.x, pos.y, 5, size, 0.0, color),
        UnitShape::Dot => draw_circle(pos.x, pos.y, size, color),
        UnitShape::Star => {
            let s = size;
            draw_triangle(
                vec2(pos.x, pos.y - s),
                vec2(pos.x - s * 0.87, pos.y + s * 0.5),
                vec2(pos.x + s * 0.87, pos.y + s * 0.5),
                color,
            );
            draw_triangle(
                vec2(pos.x, pos.y + s),
                vec2(pos.x - s * 0.87, pos.y - s * 0.5),
                vec2(pos.x + s * 0.87, pos.y - s * 0.5),
                color,
            );
        }
        UnitShape::Cross => {
            let arm = size * 0.35;
            draw_rectangle(pos.x - arm, pos.y - size, arm * 2.0, size * 2.0, color);
            draw_rectangle(pos.x - size, pos.y - arm, size * 2.0, arm * 2.0, color);
        }
        UnitShape::Octagon => draw_poly(pos.x, pos.y, 8, size, 22.5, color),
    }
}
```

In `main.rs`, replace all calls `draw_unit_shape(...)` with `unit::draw_unit_shape(...)`. There are 2 call sites (lines 1282, 1303).

- [ ] **Step 2: Move `draw_center_divider` to `arena.rs`**

Cut lines 2129-2139 from `main.rs`. Add to end of `src/arena.rs`:

```rust
use macroquad::prelude::*;

pub fn draw_center_divider() {
    let dash_len = 10.0;
    let gap_len = 8.0;
    let color = Color::new(0.3, 0.3, 0.35, 0.4);
    let mut y = 0.0;
    while y < ARENA_H {
        let end = (y + dash_len).min(ARENA_H);
        draw_line(HALF_W, y, HALF_W, end, 1.0, color);
        y += dash_len + gap_len;
    }
}
```

`arena.rs` already has `use crate::unit::Unit;` but needs `use macroquad::prelude::*;` — check if it's already there (it isn't currently — the file only uses `crate::unit::Unit`). Add `use macroquad::prelude::*;` at the top.

In `main.rs`, replace `draw_center_divider()` call (line 1227) with `arena::draw_center_divider()`. Update the `use arena::` line to include `draw_center_divider`.

- [ ] **Step 3: Move `draw_hud` to `ui.rs`**

Cut lines 2050-2127 from `main.rs`. Add to `src/ui.rs`:

```rust
use crate::arena::shop_w;
use crate::match_progress::MatchProgress;

pub fn draw_hud(
    progress: &MatchProgress,
    gold: u32,
    timer: f32,
    army_value: u32,
    battle_remaining: f32,
    player_name: &str,
    opponent_name: &str,
) {
    // Background bar
    draw_rectangle(0.0, 0.0, screen_width(), s(28.0), Color::new(0.05, 0.05, 0.08, 0.85));

    let hud_left = shop_w() + s(15.0);
    let hud_y = s(19.0);
    let gap = s(30.0);
    let mut x = hud_left;

    // Round
    let round_text = format!("Round: {}", progress.round);
    let round_w = measure_scaled_text(&round_text, 18).width;
    draw_scaled_text(&round_text, x, hud_y, 18.0, WHITE);
    x += round_w + gap;

    // Player LP
    let player_lp_text = format!("{} LP: {}", player_name, progress.player_lp);
    let plp_color = if progress.player_lp > 500 {
        Color::new(0.3, 1.0, 0.4, 1.0)
    } else if progress.player_lp > 200 {
        Color::new(1.0, 0.8, 0.2, 1.0)
    } else {
        Color::new(1.0, 0.3, 0.2, 1.0)
    };
    let plp_w = measure_scaled_text(&player_lp_text, 18).width;
    draw_scaled_text(&player_lp_text, x, hud_y, 18.0, plp_color);
    x += plp_w + gap;

    // Opponent LP
    let opponent_lp_text = format!("{} LP: {}", opponent_name, progress.opponent_lp);
    let alp_color = if progress.opponent_lp > 500 {
        Color::new(0.3, 0.6, 1.0, 1.0)
    } else if progress.opponent_lp > 200 {
        Color::new(1.0, 0.8, 0.2, 1.0)
    } else {
        Color::new(1.0, 0.3, 0.2, 1.0)
    };
    let alp_w = measure_scaled_text(&opponent_lp_text, 18).width;
    draw_scaled_text(&opponent_lp_text, x, hud_y, 18.0, alp_color);
    x += alp_w + gap;

    // Gold (only during build)
    if gold > 0 || timer > 0.0 {
        let gold_text = format!("Gold: {}", gold);
        let gold_w = measure_scaled_text(&gold_text, 18).width;
        draw_scaled_text(&gold_text, x, hud_y, 18.0, Color::new(1.0, 0.85, 0.2, 1.0));
        x += gold_w + gap;

        if army_value > 0 {
            let army_text = format!("Army: {}g", army_value);
            let army_w = measure_scaled_text(&army_text, 16).width;
            draw_scaled_text(&army_text, x, hud_y, 16.0, Color::new(0.7, 0.7, 0.75, 0.8));
            x += army_w + gap;
        }

        if timer > 0.0 {
            let timer_text = format!("Timer: {:.0}s", timer.ceil());
            draw_scaled_text(&timer_text, x, hud_y, 18.0, WHITE);
        }
    }

    // Battle round timer
    if battle_remaining > 0.0 && battle_remaining < 90.0 {
        let timer_color = if battle_remaining < 15.0 {
            Color::new(1.0, 0.3, 0.2, 1.0)
        } else if battle_remaining < 30.0 {
            Color::new(1.0, 0.8, 0.2, 1.0)
        } else {
            Color::new(0.7, 0.7, 0.7, 1.0)
        };
        let timer_text = format!("Round: {:.0}s", battle_remaining.ceil());
        draw_scaled_text(&timer_text, x, hud_y, 18.0, timer_color);
    }
}
```

In `main.rs`, replace all `draw_hud(...)` calls (4 sites: lines 1524, 1570, 1586, 1679) with `ui::draw_hud(...)`. Update the `use` imports at top of `main.rs` (the `ui` module is already `pub mod ui`, so just use `ui::draw_hud` or add to existing use).

- [ ] **Step 4: Move `refresh_units_of_kind` to `tech.rs`**

Cut lines 2231-2248 from `main.rs`. Add to end of `src/tech.rs`:

```rust
use crate::unit::{Unit, UnitKind};

pub fn refresh_units_of_kind(units: &mut [Unit], kind: UnitKind, tech_state: &TechState) {
    for unit in units.iter_mut() {
        if unit.kind != kind || !unit.alive {
            continue;
        }
        let hp_frac = unit.hp / unit.stats.max_hp;
        unit.stats = kind.stats();
        tech_state.apply_to_stats(kind, &mut unit.stats);
        unit.hp = unit.stats.max_hp * hp_frac;
        if kind == UnitKind::Scout && tech_state.has_tech(UnitKind::Scout, TechId::ScoutEvasion) {
            unit.evasion_chance = 0.25;
        }
    }
}
```

In `main.rs`, replace calls to `refresh_units_of_kind(...)` (2 sites: lines 408, 492) with `tech::refresh_units_of_kind(...)`.

- [ ] **Step 5: Move `respawn_player_units` to `game_state.rs`**

Cut lines 2191-2228 from `main.rs`. This function takes `&BuildState` and `&MatchProgress`. It belongs on `BuildState` as a method, or as a free function in `game_state.rs`.

Add as a method on `BuildState` in `src/game_state.rs`:

```rust
use crate::pack::{all_packs, PackDef, respawn_pack_units};

impl BuildState {
    /// Respawn all player units from locked packs at full HP with current techs.
    pub fn respawn_player_units(&self, player_techs: &TechState) -> Vec<Unit> {
        let mut spawned = Vec::new();
        for placed in &self.placed_packs {
            let pack = &all_packs()[placed.pack_index];
            let units = respawn_pack_units(
                pack,
                placed.center,
                placed.rotated,
                0,
                player_techs,
                &placed.unit_ids,
            );
            spawned.extend(units);
        }
        spawned
    }
}
```

This eliminates the duplicate grid-layout code from main.rs by reusing the existing `respawn_pack_units` from `pack.rs`.

In `main.rs`, replace the call `respawn_player_units(&build, &progress)` (line 1143) with `build.respawn_player_units(&progress.player_techs)`.

- [ ] **Step 6: Fix `mod settings; mod terrain;` placement**

Move `mod settings; mod terrain;` from line 2249 to the top of `main.rs` with the other mod declarations (after line 16).

- [ ] **Step 7: Verify**

Run: `cargo check`
Run: `cargo clippy -- -D warnings` (or just `cargo clippy`)
Expected: no errors. Fix any issues.

- [ ] **Step 8: Commit**

```bash
git add src/main.rs src/ui.rs src/unit.rs src/arena.rs src/tech.rs src/game_state.rs
git commit -m "refactor: move helper functions from main.rs to natural modules"
```

---

### Task 2: Extract `send_build_complete` and `start_battle_ai`

**Files:**
- Modify: `src/main.rs` (remove functions, update calls)
- Modify: `src/net.rs` (add `send_build_complete`)
- Modify: `src/economy.rs` (add `start_ai_battle`)

- [ ] **Step 1: Move `send_build_complete` to `net.rs`**

Cut lines 1977-1999 from `main.rs`. Add to `src/net.rs`:

```rust
use crate::game_state::BuildState;

pub fn send_build_complete(net: &mut Option<NetState>, build: &BuildState) {
    if let Some(ref mut n) = net {
        let new_packs: Vec<(usize, (f32, f32), bool)> = build
            .placed_packs
            .iter()
            .filter(|p| !p.locked)
            .map(|p| (p.pack_index, (p.center.x, p.center.y), p.rotated))
            .collect();
        let tech_purchases = build.round_tech_purchases.clone();
        n.send(NetMessage::BuildComplete {
            new_packs,
            tech_purchases,
            gold_remaining: build.builder.gold_remaining,
        });
    }
}
```

In `main.rs`, replace calls to `send_build_complete(...)` (2 sites: lines 419, 744) with `net::send_build_complete(...)`.

- [ ] **Step 2: Move `start_battle_ai` to `economy.rs`**

Cut lines 2001-2048 from `main.rs`. Add to `src/economy.rs`:

```rust
use crate::arena::{ARENA_W, ARENA_H};
use crate::game_state::{BuildState, GamePhase};
use crate::match_progress::MatchProgress;
use crate::projectile::Projectile;
use crate::settings::GameSettings;
use crate::terrain;
use crate::unit::Unit;

pub fn start_ai_battle(
    _build: &mut BuildState,
    units: &mut Vec<Unit>,
    projectiles: &mut Vec<Projectile>,
    progress: &mut MatchProgress,
    obstacles: &mut Vec<terrain::Obstacle>,
    nav_grid: &mut Option<terrain::NavGrid>,
    game_settings: &GameSettings,
) -> GamePhase {
    projectiles.clear();

    if obstacles.is_empty() && game_settings.terrain_enabled {
        *obstacles = terrain::generate_terrain(progress.round, game_settings.terrain_destructible);
    } else {
        terrain::reset_cover_hp(obstacles);
    }
    *nav_grid = Some(terrain::NavGrid::from_obstacles(obstacles, ARENA_W, ARENA_H, 15.0));

    units.retain(|u| u.team_id == 0);
    units.extend(progress.respawn_opponent_units());

    let mut ai_gold = progress.round_allowance();
    ai_buy_techs(&mut ai_gold, &mut progress.opponent_techs);
    let ai_builder = if game_settings.smart_ai {
        smart_army(ai_gold, &progress.ai_memory, &progress.banned_kinds)
    } else {
        random_army_filtered(ai_gold, &progress.banned_kinds)
    };
    let new_opponent_units = progress.spawn_ai_army_from_builder(&ai_builder);
    units.extend(new_opponent_units);

    macroquad::rand::srand(progress.round as u64);

    for unit in units.iter_mut() {
        unit.damage_dealt_round = 0.0;
        unit.damage_soaked_round = 0.0;
    }

    GamePhase::Battle
}
```

In `main.rs`, replace both calls to `start_battle_ai(...)` (lines 423, 748) with `economy::start_ai_battle(...)`. Update the `use economy::` import to include `start_ai_battle`.

- [ ] **Step 3: Verify**

Run: `cargo check`
Run: `cargo clippy`
Expected: no errors. Fix any issues.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/net.rs src/economy.rs
git commit -m "refactor: move send_build_complete and start_ai_battle out of main.rs"
```

---

### Task 3: Extract rendering into `rendering.rs`

Extract all world-space rendering (lines 1214-1459) and the `SplashEffect` struct.

**Files:**
- Create: `src/rendering.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/rendering.rs`**

```rust
use macroquad::prelude::*;

use crate::arena::{draw_center_divider, ARENA_H, ARENA_W, HALF_W};
use crate::game_state::{BuildState, PlacedPack};
use crate::match_progress::MatchProgress;
use crate::pack::all_packs;
use crate::projectile::{projectile_visual_radius, Projectile};
use crate::team::{team_color, team_projectile_color};
use crate::terrain;
use crate::unit::{draw_unit_shape, ProjectileType, Unit, UnitKind, UnitShape};

/// Visual effect for AOE splash damage (expanding, fading circle).
pub struct SplashEffect {
    pub pos: Vec2,
    pub radius: f32,
    pub timer: f32,
    pub max_timer: f32,
    pub team_id: u8,
}

/// Update splash effect timers and remove expired ones.
pub fn update_splash_effects(effects: &mut Vec<SplashEffect>, dt: f32) {
    for effect in effects.iter_mut() {
        effect.timer -= dt;
    }
    effects.retain(|e| e.timer > 0.0);
}

/// Draw all world-space elements: arena border, terrain, grid, shields, units, projectiles, splash, build overlays.
pub fn draw_world(
    units: &[Unit],
    projectiles: &[Projectile],
    obstacles: &[terrain::Obstacle],
    splash_effects: &[SplashEffect],
    build: &BuildState,
    progress: &MatchProgress,
    show_grid: bool,
    is_build_phase: bool,
    world_mouse: Vec2,
) {
    // Arena border and terrain
    draw_rectangle_lines(0.0, 0.0, ARENA_W, ARENA_H, 2.0, GRAY);
    draw_center_divider();
    terrain::draw_obstacles(obstacles);

    // Grid overlay during Build phase
    if show_grid && is_build_phase {
        draw_grid();
    }

    // Shield barriers
    draw_shields(units);

    // Units (including death animations)
    draw_units(units);

    // Projectiles
    draw_projectiles(projectiles);

    // Splash effects
    draw_splash_effects(splash_effects);

    // Build phase world overlays
    if is_build_phase {
        draw_build_overlays(build, progress);
    }
}

fn draw_grid() {
    let grid = terrain::GRID_CELL;
    let line_color = Color::new(0.3, 0.3, 0.35, 0.15);
    let mut gx = 0.0;
    while gx <= ARENA_W {
        draw_line(gx, 0.0, gx, ARENA_H, 1.0, line_color);
        gx += grid;
    }
    let mut gy = 0.0;
    while gy <= ARENA_H {
        draw_line(0.0, gy, ARENA_W, gy, 1.0, line_color);
        gy += grid;
    }
}

fn draw_shields(units: &[Unit]) {
    for unit in units {
        if !unit.alive || !unit.is_shield() || unit.shield_hp <= 0.0 {
            continue;
        }
        let tc = team_color(unit.team_id);
        let shield_frac = if unit.stats.shield_hp > 0.0 {
            unit.shield_hp / unit.stats.shield_hp
        } else {
            0.0
        };
        let alpha = 0.12 + 0.12 * shield_frac;
        draw_circle(
            unit.pos.x,
            unit.pos.y,
            unit.stats.shield_radius,
            Color::new(tc.r, tc.g, tc.b, alpha),
        );
        draw_circle_lines(
            unit.pos.x,
            unit.pos.y,
            unit.stats.shield_radius,
            1.5,
            Color::new(tc.r, tc.g, tc.b, 0.4 * shield_frac + 0.1),
        );
    }
}

fn draw_units(units: &[Unit]) {
    for unit in units {
        if !unit.alive && unit.death_timer <= 0.0 {
            continue;
        }

        // Death animation: shrink and fade
        if !unit.alive && unit.death_timer > 0.0 {
            let frac = unit.death_timer / 0.5;
            let alpha = frac * 0.8;
            let draw_size = unit.stats.size * frac;
            let mut color = team_color(unit.team_id);
            color.a = alpha;
            draw_unit_shape(unit.pos, draw_size, unit.stats.shape, color);
            continue;
        }

        let mut color = team_color(unit.team_id);
        if unit.kind == UnitKind::Berserker {
            let hp_frac = unit.hp / unit.stats.max_hp;
            let rage = 1.0 - hp_frac;
            color.r = (color.r + rage * 0.5).min(1.0);
            color.g = (color.g * (1.0 - rage * 0.5)).max(0.1);
        }
        // Slow visual indicator
        if unit.slow_timer > 0.0 {
            draw_circle_lines(
                unit.pos.x,
                unit.pos.y,
                unit.stats.size + 3.0,
                1.0,
                Color::new(0.2, 0.5, 1.0, 0.5),
            );
        }
        draw_unit_shape(unit.pos, unit.stats.size, unit.stats.shape, color);
        // HP bar (only show when damaged)
        let hp_frac = unit.hp / unit.stats.max_hp;
        if hp_frac < 1.0 {
            let bar_w = unit.stats.size * 2.0;
            let bar_h = 3.0;
            let bar_x = unit.pos.x - bar_w / 2.0;
            let bar_y = unit.pos.y - unit.stats.size - 8.0;
            draw_rectangle(bar_x, bar_y, bar_w, bar_h, DARKGRAY);
            let hp_color = if hp_frac > 0.5 {
                GREEN
            } else if hp_frac > 0.25 {
                YELLOW
            } else {
                RED
            };
            draw_rectangle(bar_x, bar_y, bar_w * hp_frac, bar_h, hp_color);
        }
    }
}

fn draw_projectiles(projectiles: &[Projectile]) {
    for proj in projectiles {
        if !proj.alive {
            continue;
        }
        let color = team_projectile_color(proj.team_id);
        let r = projectile_visual_radius(proj.proj_type);
        match proj.proj_type {
            ProjectileType::Laser => {
                let dir = proj.vel.normalize_or_zero();
                let tail = proj.pos - dir * 8.0;
                draw_line(tail.x, tail.y, proj.pos.x, proj.pos.y, 2.0, color);
                draw_circle(proj.pos.x, proj.pos.y, r, WHITE);
            }
            ProjectileType::Bullet => {
                draw_circle(proj.pos.x, proj.pos.y, r, color);
            }
            ProjectileType::Rocket => {
                let dir = proj.vel.normalize_or_zero();
                let tail = proj.pos - dir * 6.0;
                draw_line(
                    tail.x,
                    tail.y,
                    proj.pos.x,
                    proj.pos.y,
                    3.0,
                    Color::new(1.0, 0.5, 0.2, 0.4),
                );
                draw_circle(proj.pos.x, proj.pos.y, r, color);
            }
        }
    }
}

fn draw_splash_effects(effects: &[SplashEffect]) {
    for effect in effects {
        let progress = 1.0 - (effect.timer / effect.max_timer);
        let current_radius = effect.radius * (0.3 + 0.7 * progress);
        let alpha = 0.4 * (effect.timer / effect.max_timer);
        let tc = team_color(effect.team_id);
        draw_circle(
            effect.pos.x,
            effect.pos.y,
            current_radius,
            Color::new(tc.r, tc.g, tc.b, alpha * 0.3),
        );
        draw_circle_lines(
            effect.pos.x,
            effect.pos.y,
            current_radius,
            2.0,
            Color::new(tc.r, tc.g, tc.b, alpha),
        );
    }
}

fn draw_build_overlays(build: &BuildState, progress: &MatchProgress) {
    // Placement zone overlay
    draw_rectangle(0.0, 0.0, HALF_W, ARENA_H, Color::new(0.2, 0.3, 0.5, 0.05));
    draw_rectangle(HALF_W, 0.0, HALF_W, ARENA_H, Color::new(0.5, 0.2, 0.2, 0.05));

    // Drag-box selection rectangle
    if let Some(box_start) = build.drag_box_start {
        // Note: world_mouse is not passed here — caller should set build.drag_box_end or
        // we draw based on current drag_box_start only. For now, the drag box visual
        // is drawn in main.rs since it needs world_mouse. We'll revisit.
        // Actually, we can compute it from the camera in the caller. Leave for now.
    }

    // Pack bounding boxes
    let packs = all_packs();
    for (i, placed) in build.placed_packs.iter().enumerate() {
        let pack = &packs[placed.pack_index];
        let half = placed.bbox_half_size_for(pack);
        let min = placed.center - half;

        let is_multi_dragged = build.multi_dragging.contains(&i);
        let bbox_color = if is_multi_dragged {
            let mut overlap = false;
            for (j, other) in build.placed_packs.iter().enumerate() {
                if build.multi_dragging.contains(&j) {
                    continue;
                }
                let p1 = &packs[placed.pack_index];
                let p2 = &packs[other.pack_index];
                if placed.overlaps(other, p1, p2) {
                    overlap = true;
                    break;
                }
            }
            if overlap {
                Color::new(1.0, 0.2, 0.2, 0.6)
            } else {
                Color::new(0.2, 1.0, 0.3, 0.5)
            }
        } else if build.dragging == Some(i)
            && build.would_overlap(placed.center, placed.pack_index, Some(i), placed.rotated)
        {
            Color::new(1.0, 0.2, 0.2, 0.6)
        } else if build.dragging == Some(i) {
            Color::new(0.2, 1.0, 0.3, 0.5)
        } else if build.selected_pack == Some(i) {
            Color::new(0.2, 0.8, 1.0, 0.8)
        } else if placed.locked {
            Color::new(0.3, 0.3, 0.4, 0.25)
        } else {
            Color::new(0.5, 0.5, 0.5, 0.3)
        };

        let thickness = if build.selected_pack == Some(i) {
            2.5
        } else {
            1.5
        };
        draw_rectangle_lines(min.x, min.y, half.x * 2.0, half.y * 2.0, thickness, bbox_color);
    }

    // Opponent pack bounding boxes
    for opponent_pack in &progress.opponent_packs {
        let pack = &packs[opponent_pack.pack_index];
        let half = PlacedPack::bbox_half_size_rotated(pack, opponent_pack.rotated);
        let min = opponent_pack.center - half;
        let bbox_color = Color::new(0.3, 0.3, 0.5, 0.2);
        draw_rectangle_lines(min.x, min.y, half.x * 2.0, half.y * 2.0, 1.0, bbox_color);
    }
}
```

- [ ] **Step 2: Update `main.rs` to use `rendering.rs`**

Add `mod rendering;` to the module declarations.

Replace the `SplashEffect` struct with `use rendering::SplashEffect;`.

Replace lines 1208-1212 (splash update) with:
```rust
rendering::update_splash_effects(&mut splash_effects, dt);
```

Replace lines 1214-1459 (everything from `clear_background` through the end of build overlays, but keep `clear_background` and `set_camera`) with a call to `rendering::draw_world(...)`:

```rust
clear_background(Color::new(0.1, 0.1, 0.12, 1.0));
if matches!(phase, GamePhase::Lobby) {
    next_frame().await;
    continue;
}
set_camera(&arena_camera);

// Draw drag box (needs world_mouse which rendering doesn't have)
let drag_box_world_mouse = world_mouse;

rendering::draw_world(
    &units, &projectiles, &obstacles, &splash_effects,
    &build, &progress, show_grid,
    matches!(phase, GamePhase::Build),
    world_mouse,
);

// Drag-box visual (still in world-space, needs world_mouse)
if matches!(phase, GamePhase::Build) {
    if let Some(box_start) = build.drag_box_start {
        let box_end = world_mouse;
        let min_x = box_start.x.min(box_end.x);
        let min_y = box_start.y.min(box_end.y);
        let w = (box_start.x - box_end.x).abs();
        let h = (box_start.y - box_end.y).abs();
        draw_rectangle(min_x, min_y, w, h, Color::new(0.2, 0.5, 1.0, 0.15));
        draw_rectangle_lines(min_x, min_y, w, h, 1.5, Color::new(0.3, 0.6, 1.0, 0.8));
    }
}

set_default_camera();
```

Remove the empty `if let Some(box_start) = build.drag_box_start` block from rendering.rs `draw_build_overlays` (it was a placeholder).

- [ ] **Step 3: Verify**

Run: `cargo check`
Run: `cargo clippy`
Expected: no errors. Fix any issues.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/rendering.rs
git commit -m "refactor: extract world-space rendering to rendering.rs"
```

---

### Task 4: Extract chat system into `chat.rs`

**Files:**
- Create: `src/chat.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/chat.rs`**

```rust
use macroquad::prelude::*;

use crate::game_state::GamePhase;
use crate::net;
use crate::team;

pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub open: bool,
}

pub struct ChatMessage {
    pub name: String,
    pub text: String,
    pub team_id: u8,
    pub lifetime: f32,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            open: false,
        }
    }

    /// Receive incoming chat messages from network.
    pub fn receive_from_net(&mut self, net: &mut Option<net::NetState>) {
        if let Some(ref mut n) = net {
            for (name, text) in n.received_chats.drain(..) {
                self.messages.push(ChatMessage {
                    name,
                    text,
                    team_id: 1,
                    lifetime: 5.0,
                });
            }
        }
    }

    /// Handle chat input and sending. Returns true if chat consumed the Enter key.
    pub fn update(
        &mut self,
        phase: &GamePhase,
        net: &mut Option<net::NetState>,
        player_name: &str,
    ) {
        let chat_allowed = matches!(
            phase,
            GamePhase::Build | GamePhase::Battle | GamePhase::RoundResult { .. }
        );
        if !chat_allowed {
            return;
        }

        if is_key_pressed(KeyCode::Enter) {
            if self.open {
                if !self.input.is_empty() {
                    let text = if self.input.len() > 100 {
                        self.input[..100].to_string()
                    } else {
                        self.input.clone()
                    };
                    self.messages.push(ChatMessage {
                        name: player_name.to_string(),
                        text: text.clone(),
                        team_id: 0,
                        lifetime: 5.0,
                    });
                    if let Some(ref mut n) = net {
                        n.send(net::NetMessage::ChatMessage(player_name.to_string(), text));
                    }
                }
                self.input.clear();
                self.open = false;
            } else {
                self.open = true;
            }
        }

        if self.open {
            if is_key_pressed(KeyCode::Escape) {
                self.open = false;
                self.input.clear();
            }
            while let Some(ch) = get_char_pressed() {
                if ch == '\r' || ch == '\n' {
                    continue;
                }
                if ch == '\u{8}' {
                    self.input.pop();
                } else if self.input.len() < 100 && (ch.is_ascii_graphic() || ch == ' ') {
                    self.input.push(ch);
                }
            }
        }
    }

    /// Update lifetimes and remove expired messages.
    pub fn tick(&mut self, dt: f32) {
        for msg in self.messages.iter_mut() {
            msg.lifetime -= dt;
        }
        self.messages.retain(|m| m.lifetime > 0.0);
    }

    /// Draw chat messages and input box.
    pub fn draw(&self, phase: &GamePhase, player_name: &str) {
        let chat_x = screen_width() / 2.0;
        let mut chat_y = crate::ui::s(45.0);
        for msg in self.messages.iter().rev().take(5).collect::<Vec<_>>().into_iter().rev() {
            let alpha = (msg.lifetime / 5.0).min(1.0);
            let color = team::team_color(msg.team_id);
            let display_color = Color::new(color.r, color.g, color.b, alpha);

            let is_emote = msg.text.starts_with('/');
            let display_text = match msg.text.as_str() {
                "/gg" => "GG".to_string(),
                "/gl" => "Good Luck!".to_string(),
                "/nice" => "Nice!".to_string(),
                "/wow" => "Wow!".to_string(),
                _ => msg.text.clone(),
            };
            let full_display = format!("{}: {}", msg.name, display_text);
            let font_size = if is_emote { 20.0 } else { 15.0 };
            let dims = crate::ui::measure_scaled_text(&full_display, font_size as u16);
            crate::ui::draw_scaled_text(
                &full_display,
                chat_x - dims.width / 2.0,
                chat_y,
                font_size,
                display_color,
            );
            chat_y += font_size + 4.0;
        }

        // Input box
        if self.open {
            let input_y = screen_height() - crate::ui::s(45.0);
            let input_w = crate::ui::s(450.0);
            let input_x = screen_width() / 2.0 - input_w / 2.0;
            let input_h = crate::ui::s(30.0);
            draw_rectangle(
                input_x,
                input_y,
                input_w,
                input_h,
                Color::new(0.05, 0.05, 0.1, 0.92),
            );
            draw_rectangle_lines(
                input_x,
                input_y,
                input_w,
                input_h,
                1.5,
                Color::new(0.4, 0.5, 0.6, 0.9),
            );
            let name_prefix = format!("{}: ", player_name);
            let name_w = crate::ui::measure_scaled_text(&name_prefix, 15).width;
            crate::ui::draw_scaled_text(
                &name_prefix,
                input_x + 8.0,
                input_y + 20.0,
                15.0,
                Color::new(0.6, 0.8, 1.0, 0.9),
            );
            let cursor = if (get_time() * 2.0) as u32 % 2 == 0 {
                "|"
            } else {
                ""
            };
            crate::ui::draw_scaled_text(
                &format!("{}{}", self.input, cursor),
                input_x + 8.0 + name_w,
                input_y + 20.0,
                15.0,
                WHITE,
            );
        } else {
            let chat_allowed = matches!(
                phase,
                GamePhase::Build | GamePhase::Battle | GamePhase::RoundResult { .. }
            );
            if chat_allowed {
                crate::ui::draw_scaled_text(
                    "Enter: Chat",
                    screen_width() - crate::ui::s(100.0),
                    screen_height() - crate::ui::s(5.0),
                    12.0,
                    Color::new(0.4, 0.4, 0.4, 0.6),
                );
            }
        }
    }
}
```

- [ ] **Step 2: Update `main.rs`**

Add `mod chat;` to the module declarations.

Replace the chat state variables (lines 74-76):
```rust
// Before:
let mut chat_messages: Vec<(String, String, u8, f32)> = Vec::new();
let mut chat_input = String::new();
let mut chat_open = false;
```
With:
```rust
let mut chat = chat::ChatState::new();
```

Replace the entire chat block (lines 1873-1971) with:
```rust
// Chat system
chat.receive_from_net(&mut net);
chat.update(&phase, &mut net, &mp_player_name);
chat.tick(dt);
chat.draw(&phase, &mp_player_name);
```

Remove the `let player_name = lobby.player_name.clone();` line (1874) since we pass `mp_player_name` directly.

Also update `GameOver` rematch handler (line 1196) — replace `chat_messages.clear()` with `chat = chat::ChatState::new()`.

- [ ] **Step 3: Verify**

Run: `cargo check`
Run: `cargo clippy`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/chat.rs
git commit -m "refactor: extract chat system to chat.rs"
```

---

### Task 5: Extract draft/ban phase into `draft_ban.rs`

**Files:**
- Create: `src/draft_ban.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/draft_ban.rs`**

This module handles the entire DraftBan phase: drawing the UI, handling clicks, polling network, and returning a transition signal.

```rust
use macroquad::prelude::*;

use crate::game_state::GamePhase;
use crate::match_progress::MatchProgress;
use crate::net;
use crate::unit::UnitKind;

/// All selectable unit kinds for banning.
pub const ALL_KINDS: [UnitKind; 13] = [
    UnitKind::Striker,
    UnitKind::Sentinel,
    UnitKind::Ranger,
    UnitKind::Scout,
    UnitKind::Bruiser,
    UnitKind::Artillery,
    UnitKind::Chaff,
    UnitKind::Sniper,
    UnitKind::Skirmisher,
    UnitKind::Dragoon,
    UnitKind::Berserker,
    UnitKind::Shield,
    UnitKind::Interceptor,
];

/// Result of a draft/ban update tick.
pub enum DraftBanResult {
    /// Still in progress, keep polling.
    Waiting,
    /// Phase complete — here are the combined bans to apply.
    Done(Vec<UnitKind>),
}

/// Run one frame of the draft/ban phase. Handles input, drawing, and network.
/// Returns `DraftBanResult::Done(bans)` when both players have confirmed.
pub fn update_and_draw(
    bans: &mut Vec<UnitKind>,
    confirmed: &mut bool,
    opponent_bans: &mut Option<Vec<UnitKind>>,
    net: &mut Option<net::NetState>,
    screen_mouse: Vec2,
    left_click: bool,
) -> DraftBanResult {
    // Draw background
    clear_background(Color::new(0.08, 0.08, 0.12, 1.0));

    // Title
    let title = "Ban Phase \u{2014} Select up to 2 unit types to ban";
    let tdims = crate::ui::measure_scaled_text(title, 24);
    crate::ui::draw_scaled_text(
        title,
        screen_width() / 2.0 - tdims.width / 2.0,
        crate::ui::s(50.0),
        24.0,
        WHITE,
    );

    // Draw unit cards in a grid (4 cols)
    let cols = 4;
    let card_w = crate::ui::s(160.0);
    let card_h = crate::ui::s(50.0);
    let gap = crate::ui::s(12.0);
    let grid_w = cols as f32 * card_w + (cols - 1) as f32 * gap;
    let start_x = screen_width() / 2.0 - grid_w / 2.0;
    let start_y = crate::ui::s(90.0);

    for (i, kind) in ALL_KINDS.iter().enumerate() {
        let col = (i % cols) as f32;
        let row = (i / cols) as f32;
        let x = start_x + col * (card_w + gap);
        let y = start_y + row * (card_h + gap);

        let is_banned = bans.contains(kind);
        let is_hovered = screen_mouse.x >= x
            && screen_mouse.x <= x + card_w
            && screen_mouse.y >= y
            && screen_mouse.y <= y + card_h;

        let bg = if is_banned {
            Color::new(0.6, 0.15, 0.15, 0.9)
        } else if is_hovered {
            Color::new(0.2, 0.25, 0.35, 0.9)
        } else {
            Color::new(0.12, 0.12, 0.18, 0.9)
        };

        draw_rectangle(x, y, card_w, card_h, bg);
        draw_rectangle_lines(
            x,
            y,
            card_w,
            card_h,
            1.0,
            if is_banned { RED } else { GRAY },
        );

        let name = format!("{:?}", kind);
        let stats = kind.stats();
        let info = format!("{} HP:{:.0} DMG:{:.0}", name, stats.max_hp, stats.damage);
        crate::ui::draw_scaled_text(
            &info,
            x + crate::ui::s(8.0),
            y + crate::ui::s(20.0),
            14.0,
            if is_banned {
                Color::new(1.0, 0.5, 0.5, 1.0)
            } else {
                WHITE
            },
        );

        if is_banned {
            let ban_text = "BANNED";
            let bdims = crate::ui::measure_scaled_text(ban_text, 16);
            crate::ui::draw_scaled_text(
                ban_text,
                x + card_w / 2.0 - bdims.width / 2.0,
                y + crate::ui::s(40.0),
                16.0,
                RED,
            );
        } else {
            let detail = format!(
                "RNG:{:.0} SPD:{:.0} AS:{:.1}",
                stats.attack_range, stats.move_speed, stats.attack_speed
            );
            crate::ui::draw_scaled_text(
                &detail,
                x + crate::ui::s(8.0),
                y + crate::ui::s(38.0),
                12.0,
                LIGHTGRAY,
            );
        }

        // Click to toggle ban
        if left_click && is_hovered && !*confirmed {
            if is_banned {
                bans.retain(|k| k != kind);
            } else if bans.len() < 2 {
                bans.push(*kind);
            }
        }
    }

    // Confirm button
    let btn_w = crate::ui::s(200.0);
    let btn_h = crate::ui::s(45.0);
    let btn_x = screen_width() / 2.0 - btn_w / 2.0;
    let btn_y = start_y + 4.0 * (card_h + gap) + crate::ui::s(20.0);
    let btn_hover = screen_mouse.x >= btn_x
        && screen_mouse.x <= btn_x + btn_w
        && screen_mouse.y >= btn_y
        && screen_mouse.y <= btn_y + btn_h;
    let btn_color = if btn_hover {
        Color::new(0.2, 0.6, 0.3, 0.9)
    } else {
        Color::new(0.15, 0.45, 0.2, 0.8)
    };
    draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
    draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, WHITE);
    let confirm_text = format!("Confirm Bans ({}/ 2)", bans.len());
    let cdims = crate::ui::measure_scaled_text(&confirm_text, 20);
    crate::ui::draw_scaled_text(
        &confirm_text,
        btn_x + btn_w / 2.0 - cdims.width / 2.0,
        btn_y + btn_h / 2.0 + 6.0,
        20.0,
        WHITE,
    );

    // Poll network for opponent bans
    if let Some(ref mut n) = net {
        n.poll();
        if let Some(ob) = n.opponent_bans.take() {
            let opp: Vec<UnitKind> = ob
                .iter()
                .filter_map(|&idx| ALL_KINDS.get(idx as usize).copied())
                .collect();
            *opponent_bans = Some(opp);
        }
    }

    // Confirm button click
    if left_click && btn_hover && !*confirmed {
        *confirmed = true;
        if let Some(ref mut n) = net {
            let ban_indices: Vec<u8> = bans
                .iter()
                .map(|k| ALL_KINDS.iter().position(|ak| ak == k).unwrap_or(0) as u8)
                .collect();
            n.send(net::NetMessage::BanSelection(ban_indices));
        }
    }

    // Show waiting indicator
    if *confirmed && net.is_some() && opponent_bans.is_none() {
        let wait_y = btn_y + btn_h + crate::ui::s(15.0);
        let dots = ".".repeat((get_time() * 2.0) as usize % 4);
        let wait_text = format!("Waiting for opponent bans{}", dots);
        let wdims = crate::ui::measure_scaled_text(&wait_text, 16);
        crate::ui::draw_scaled_text(
            &wait_text,
            screen_width() / 2.0 - wdims.width / 2.0,
            wait_y,
            16.0,
            LIGHTGRAY,
        );
    }

    // Transition when ready
    let ready = *confirmed && (net.is_none() || opponent_bans.is_some());
    if ready {
        let mut all_bans = bans.clone();
        if let Some(ref ob) = opponent_bans {
            all_bans.extend(ob.iter());
        }
        DraftBanResult::Done(all_bans)
    } else {
        DraftBanResult::Waiting
    }
}
```

- [ ] **Step 2: Update `main.rs`**

Add `mod draft_ban;` to the module declarations.

Replace the entire `GamePhase::DraftBan { ... }` match arm (lines 222-350) with:

```rust
GamePhase::DraftBan { ref mut bans, ref mut confirmed, ref mut opponent_bans } => {
    match draft_ban::update_and_draw(bans, confirmed, opponent_bans, &mut net, screen_mouse, left_click) {
        draft_ban::DraftBanResult::Waiting => {}
        draft_ban::DraftBanResult::Done(all_bans) => {
            progress.banned_kinds = all_bans;
            phase = GamePhase::Build;
        }
    }
    next_frame().await;
    continue;
}
```

- [ ] **Step 3: Verify**

Run: `cargo check`
Run: `cargo clippy`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/draft_ban.rs
git commit -m "refactor: extract draft/ban phase to draft_ban.rs"
```

---

### Task 6: Extract phase-specific UI into `phase_ui.rs`

This is the largest extraction: all screen-space UI rendering per phase (lines 1463-1871).

**Files:**
- Create: `src/phase_ui.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/phase_ui.rs`**

This module contains one function per phase's UI rendering. Each function draws its UI in screen-space (after `set_default_camera()`).

```rust
use macroquad::prelude::*;

use crate::arena::shop_w;
use crate::game_state::{self, BuildState, GamePhase, PlacedPack};
use crate::match_progress::MatchProgress;
use crate::net;
use crate::pack::all_packs;
use crate::settings;
use crate::team::team_color;
use crate::terrain;
use crate::unit::Unit;

/// Draw Build phase UI: shop, pack labels, tech panel, HUD, begin-round button, hints.
pub fn draw_build_ui(
    build: &BuildState,
    progress: &MatchProgress,
    units: &[Unit],
    screen_mouse: Vec2,
    arena_camera: &Camera2D,
    game_settings: &settings::GameSettings,
    mp_player_name: &str,
    mp_opponent_name: &str,
) {
    crate::shop::draw_shop(
        build.builder.gold_remaining,
        screen_mouse,
        false,
        &progress.banned_kinds,
        game_state::BUILD_LIMIT - build.packs_bought_this_round,
    );

    // Pack labels (screen-space so text isn't distorted by camera zoom)
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
        for opponent_pack in &progress.opponent_packs {
            let pack = &packs[opponent_pack.pack_index];
            let half = PlacedPack::bbox_half_size_rotated(pack, opponent_pack.rotated);
            let world_pos = vec2(
                opponent_pack.center.x - half.x + 2.0,
                opponent_pack.center.y - half.y - 2.0,
            );
            let screen_pos = arena_camera.world_to_screen(world_pos);
            let label = format!("{} (R{})", pack.name, opponent_pack.round_placed);
            crate::ui::draw_scaled_text(
                &label,
                screen_pos.x,
                screen_pos.y,
                12.0,
                Color::new(0.4, 0.4, 0.6, 0.4),
            );
        }
    }

    // Tech panel (when a pack is selected)
    if let Some(sel_idx) = build.selected_pack {
        if sel_idx < build.placed_packs.len() {
            let placed = &build.placed_packs[sel_idx];
            let kind = all_packs()[placed.pack_index].kind;
            let cs = crate::tech_ui::PackCombatStats::from_units(units, &placed.unit_ids);
            crate::tech_ui::draw_tech_panel(
                kind,
                &progress.player_techs,
                build.builder.gold_remaining,
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
    crate::ui::draw_hud(
        progress,
        build.builder.gold_remaining,
        build.timer,
        army_value,
        0.0,
        mp_player_name,
        mp_opponent_name,
    );

    // Begin Round button
    let btn_w = crate::ui::s(160.0);
    let btn_h = crate::ui::s(40.0);
    let btn_x = screen_width() / 2.0 - btn_w / 2.0;
    let btn_y = screen_height() - crate::ui::s(55.0);
    let btn_hovered = screen_mouse.x >= btn_x
        && screen_mouse.x <= btn_x + btn_w
        && screen_mouse.y >= btn_y
        && screen_mouse.y <= btn_y + btn_h;
    let btn_bg = if btn_hovered {
        Color::new(0.2, 0.6, 0.3, 0.9)
    } else {
        Color::new(0.15, 0.4, 0.2, 0.8)
    };
    draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_bg);
    draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 2.0, Color::new(0.3, 0.8, 0.4, 1.0));
    let btn_text = "Begin Round";
    let tdims = crate::ui::measure_scaled_text(btn_text, 22);
    crate::ui::draw_scaled_text(
        btn_text,
        btn_x + btn_w / 2.0 - tdims.width / 2.0,
        btn_y + btn_h / 2.0 + 7.0,
        22.0,
        WHITE,
    );

    // Hint text
    crate::ui::draw_scaled_text(
        "Select \u{2192} Double-click move | Mid-click rotate | Right-click sell | G: Grid | Ctrl+Z: Undo | Scroll: Zoom",
        shop_w() + 10.0,
        screen_height() - crate::ui::s(10.0),
        13.0,
        Color::new(0.5, 0.5, 0.5, 0.7),
    );
}

/// Draw WaitingForOpponent UI.
pub fn draw_waiting_ui(
    progress: &MatchProgress,
    build: &BuildState,
    mp_player_name: &str,
    mp_opponent_name: &str,
) {
    crate::ui::draw_hud(progress, build.builder.gold_remaining, 0.0, 0, 0.0, mp_player_name, mp_opponent_name);

    let dots = ".".repeat((get_time() * 2.0) as usize % 4);
    let wait_text = format!("Waiting for opponent{}", dots);
    let wdims = crate::ui::measure_scaled_text(&wait_text, 28);
    crate::ui::draw_scaled_text(
        &wait_text,
        screen_width() / 2.0 - wdims.width / 2.0,
        screen_height() / 2.0,
        28.0,
        Color::new(0.7, 0.7, 0.9, 1.0),
    );
}

/// Draw Battle phase UI: HUD, alive counts, obstacle tooltip, surrender overlay.
pub fn draw_battle_ui(
    progress: &MatchProgress,
    units: &[Unit],
    obstacles: &[terrain::Obstacle],
    battle_timer: f32,
    round_timeout: f32,
    show_surrender_confirm: bool,
    screen_mouse: Vec2,
    world_mouse: Vec2,
    mp_player_name: &str,
    mp_opponent_name: &str,
) {
    let remaining = (round_timeout - battle_timer).max(0.0);
    crate::ui::draw_hud(progress, 0, 0.0, 0, remaining, mp_player_name, mp_opponent_name);

    let alive_0 = units.iter().filter(|u| u.alive && u.team_id == 0).count();
    let alive_1 = units.iter().filter(|u| u.alive && u.team_id == 1).count();
    crate::ui::draw_scaled_text(
        &format!("Red: {}", alive_0),
        crate::ui::s(10.0),
        screen_height() - crate::ui::s(15.0),
        20.0,
        team_color(0),
    );
    let blue_text = format!("Blue: {}", alive_1);
    let bdims = crate::ui::measure_scaled_text(&blue_text, 20);
    crate::ui::draw_scaled_text(
        &blue_text,
        screen_width() - bdims.width - crate::ui::s(10.0),
        screen_height() - crate::ui::s(15.0),
        20.0,
        team_color(1),
    );

    // Obstacle tooltip on hover
    if !show_surrender_confirm {
        for obs in obstacles {
            if !obs.alive {
                continue;
            }
            if obs.contains_point(world_mouse) {
                let tip_x = screen_mouse.x + crate::ui::s(15.0);
                let tip_y = (screen_mouse.y - crate::ui::s(10.0)).max(5.0);
                let tip_w = crate::ui::s(170.0);
                let tip_h = if obs.obstacle_type == terrain::ObstacleType::Cover {
                    crate::ui::s(60.0)
                } else {
                    crate::ui::s(45.0)
                };

                draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.08, 0.08, 0.12, 0.95));
                draw_rectangle_lines(
                    tip_x,
                    tip_y,
                    tip_w,
                    tip_h,
                    1.0,
                    Color::new(0.4, 0.5, 0.6, 0.7),
                );

                let type_name = match obs.obstacle_type {
                    terrain::ObstacleType::Wall => "Wall (Indestructible)",
                    terrain::ObstacleType::Cover => "Cover (Destructible)",
                };
                crate::ui::draw_scaled_text(
                    type_name,
                    tip_x + crate::ui::s(6.0),
                    tip_y + crate::ui::s(16.0),
                    14.0,
                    WHITE,
                );

                let mut ty = tip_y + crate::ui::s(32.0);
                if obs.obstacle_type == terrain::ObstacleType::Cover {
                    crate::ui::draw_scaled_text(
                        &format!("HP: {:.0}/{:.0}", obs.hp, obs.max_hp),
                        tip_x + crate::ui::s(6.0),
                        ty,
                        12.0,
                        LIGHTGRAY,
                    );
                    ty += crate::ui::s(14.0);
                }
                let team_name = match obs.team_id {
                    0 => mp_player_name,
                    1 => mp_opponent_name,
                    _ => "Neutral",
                };
                crate::ui::draw_scaled_text(
                    &format!("Owner: {}", team_name),
                    tip_x + crate::ui::s(6.0),
                    ty,
                    12.0,
                    LIGHTGRAY,
                );
                break;
            }
        }
    }

    // Surrender confirmation overlay
    if show_surrender_confirm {
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.6),
        );
        let title = "Surrender?";
        let tdims = crate::ui::measure_scaled_text(title, 36);
        crate::ui::draw_scaled_text(
            title,
            screen_width() / 2.0 - tdims.width / 2.0,
            screen_height() / 2.0 - crate::ui::s(20.0),
            36.0,
            WHITE,
        );

        let btn_w: f32 = crate::ui::s(120.0);
        let btn_h: f32 = crate::ui::s(40.0);
        let cx = screen_width() / 2.0;
        let cy = screen_height() / 2.0;

        let yes_x = cx - btn_w - crate::ui::s(10.0);
        let yes_y = cy + crate::ui::s(10.0);
        let yes_hover = screen_mouse.x >= yes_x
            && screen_mouse.x <= yes_x + btn_w
            && screen_mouse.y >= yes_y
            && screen_mouse.y <= yes_y + btn_h;
        let yes_color = if yes_hover {
            Color::new(0.8, 0.2, 0.2, 0.9)
        } else {
            Color::new(0.6, 0.15, 0.15, 0.8)
        };
        draw_rectangle(yes_x, yes_y, btn_w, btn_h, yes_color);
        draw_rectangle_lines(yes_x, yes_y, btn_w, btn_h, 1.0, WHITE);
        let yt = "Yes";
        let ydims = crate::ui::measure_scaled_text(yt, 20);
        crate::ui::draw_scaled_text(
            yt,
            yes_x + btn_w / 2.0 - ydims.width / 2.0,
            yes_y + btn_h / 2.0 + 6.0,
            20.0,
            WHITE,
        );

        let no_x = cx + crate::ui::s(10.0);
        let no_y = cy + crate::ui::s(10.0);
        let no_hover = screen_mouse.x >= no_x
            && screen_mouse.x <= no_x + btn_w
            && screen_mouse.y >= no_y
            && screen_mouse.y <= no_y + btn_h;
        let no_color = if no_hover {
            Color::new(0.3, 0.3, 0.35, 0.9)
        } else {
            Color::new(0.2, 0.2, 0.25, 0.8)
        };
        draw_rectangle(no_x, no_y, btn_w, btn_h, no_color);
        draw_rectangle_lines(no_x, no_y, btn_w, btn_h, 1.0, WHITE);
        let nt = "Cancel";
        let ndims = crate::ui::measure_scaled_text(nt, 20);
        crate::ui::draw_scaled_text(
            nt,
            no_x + btn_w / 2.0 - ndims.width / 2.0,
            no_y + btn_h / 2.0 + 6.0,
            20.0,
            WHITE,
        );
    }
}

/// Draw RoundResult UI.
pub fn draw_round_result_ui(
    progress: &MatchProgress,
    match_state: &crate::arena::MatchState,
    lp_damage: i32,
    loser_team: Option<u8>,
    game_settings: &settings::GameSettings,
    net: &Option<net::NetState>,
    mp_player_name: &str,
    mp_opponent_name: &str,
) {
    crate::ui::draw_hud(progress, 0, 0.0, 0, 0.0, mp_player_name, mp_opponent_name);

    let text = match match_state {
        crate::arena::MatchState::Winner(tid) => {
            let (winner_name, color_idx) = if *tid == 0 {
                (mp_player_name, game_settings.player_color_index)
            } else {
                let opp_idx = net.as_ref().and_then(|n| n.opponent_color).unwrap_or(1);
                (mp_opponent_name, opp_idx)
            };
            let color_name = settings::TEAM_COLOR_OPTIONS
                .get(color_idx as usize)
                .map(|(name, _)| *name)
                .unwrap_or("???");
            format!("{} ({}) wins round {}!", winner_name, color_name, progress.round)
        }
        crate::arena::MatchState::Draw => format!("Round {} - Draw!", progress.round),
        crate::arena::MatchState::InProgress => unreachable!(),
    };

    let dims = crate::ui::measure_scaled_text(&text, 36);
    crate::ui::draw_scaled_text(
        &text,
        screen_width() / 2.0 - dims.width / 2.0,
        screen_height() / 2.0 - crate::ui::s(30.0),
        36.0,
        WHITE,
    );

    if let Some(loser) = loser_team {
        let loser_name = if loser == 0 { mp_player_name } else { mp_opponent_name };
        let dmg_text = format!("{} loses {} LP", loser_name, lp_damage);
        let ddims = crate::ui::measure_scaled_text(&dmg_text, 22);
        crate::ui::draw_scaled_text(
            &dmg_text,
            screen_width() / 2.0 - ddims.width / 2.0,
            screen_height() / 2.0 + crate::ui::s(5.0),
            22.0,
            Color::new(1.0, 0.4, 0.3, 1.0),
        );
    }

    let next_text = if progress.is_game_over() {
        "Press Space to see results"
    } else {
        "Press Space for next round"
    };
    let ndims = crate::ui::measure_scaled_text(next_text, 18);
    crate::ui::draw_scaled_text(
        next_text,
        screen_width() / 2.0 - ndims.width / 2.0,
        screen_height() / 2.0 + crate::ui::s(35.0),
        18.0,
        LIGHTGRAY,
    );
}

/// Draw GameOver UI: headline, stats panel, rematch button.
pub fn draw_game_over_ui(
    winner: u8,
    progress: &MatchProgress,
    units: &[Unit],
    game_settings: &settings::GameSettings,
    net: &Option<net::NetState>,
    screen_mouse: Vec2,
    mp_player_name: &str,
    mp_opponent_name: &str,
) {
    let (headline, winner_color_idx) = if winner == 0 {
        ("YOU WIN!".to_string(), game_settings.player_color_index)
    } else {
        (
            "YOU LOSE!".to_string(),
            net.as_ref().and_then(|n| n.opponent_color).unwrap_or(1),
        )
    };
    let winner_name = if winner == 0 { mp_player_name } else { mp_opponent_name };
    let color_name = settings::TEAM_COLOR_OPTIONS
        .get(winner_color_idx as usize)
        .map(|(name, _)| *name)
        .unwrap_or("???");
    let subtitle = format!("{} ({}) wins!", winner_name, color_name);
    let headline_color = if winner == 0 {
        Color::new(0.2, 1.0, 0.3, 1.0)
    } else {
        Color::new(1.0, 0.3, 0.2, 1.0)
    };
    let dims = crate::ui::measure_scaled_text(&headline, 48);
    crate::ui::draw_scaled_text(
        &headline,
        screen_width() / 2.0 - dims.width / 2.0,
        screen_height() / 2.0 - crate::ui::s(40.0),
        48.0,
        headline_color,
    );
    let sub_dims = crate::ui::measure_scaled_text(&subtitle, 22);
    let (_, (cr, cg, cb)) = settings::TEAM_COLOR_OPTIONS
        .get(winner_color_idx as usize)
        .copied()
        .unwrap_or(("White", (1.0, 1.0, 1.0)));
    crate::ui::draw_scaled_text(
        &subtitle,
        screen_width() / 2.0 - sub_dims.width / 2.0,
        screen_height() / 2.0 - crate::ui::s(10.0),
        22.0,
        Color::new(cr, cg, cb, 1.0),
    );

    // Stats panel
    let panel_w = crate::ui::s(320.0);
    let panel_h = crate::ui::s(140.0);
    let panel_x = screen_width() / 2.0 - panel_w / 2.0;
    let panel_y = screen_height() / 2.0 + 10.0;
    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.08, 0.08, 0.12, 0.9));
    draw_rectangle_lines(
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        1.0,
        Color::new(0.4, 0.5, 0.6, 0.7),
    );

    let mut sy = panel_y + crate::ui::s(18.0);
    let sx = panel_x + crate::ui::s(12.0);

    crate::ui::draw_scaled_text(
        &format!("Rounds Played: {}", progress.round),
        sx,
        sy,
        15.0,
        LIGHTGRAY,
    );
    sy += crate::ui::s(18.0);

    let mvp = units
        .iter()
        .filter(|u| u.team_id == 0)
        .max_by(|a, b| {
            a.damage_dealt_total
                .partial_cmp(&b.damage_dealt_total)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    if let Some(mvp_unit) = mvp {
        crate::ui::draw_scaled_text(
            &format!(
                "MVP: {:?} - {:.0} dmg, {} kills",
                mvp_unit.kind, mvp_unit.damage_dealt_total, mvp_unit.kills_total
            ),
            sx,
            sy,
            15.0,
            Color::new(1.0, 0.85, 0.2, 1.0),
        );
    }
    sy += crate::ui::s(18.0);

    let total_dmg: f32 = units
        .iter()
        .filter(|u| u.team_id == 0)
        .map(|u| u.damage_dealt_total)
        .sum();
    crate::ui::draw_scaled_text(&format!("Total Damage: {:.0}", total_dmg), sx, sy, 15.0, LIGHTGRAY);
    sy += crate::ui::s(18.0);

    let surviving = units.iter().filter(|u| u.team_id == 0 && u.alive).count();
    let total_units = units.iter().filter(|u| u.team_id == 0).count();
    crate::ui::draw_scaled_text(
        &format!("Surviving: {} / {}", surviving, total_units),
        sx,
        sy,
        15.0,
        LIGHTGRAY,
    );
    sy += crate::ui::s(18.0);

    crate::ui::draw_scaled_text(
        &format!(
            "LP: {} {} vs {} {}",
            mp_player_name, progress.player_lp, mp_opponent_name, progress.opponent_lp
        ),
        sx,
        sy,
        15.0,
        LIGHTGRAY,
    );

    let below_panel = panel_y + panel_h + crate::ui::s(8.0);
    crate::ui::draw_scaled_text(
        "Press R to return to lobby",
        screen_width() / 2.0 - crate::ui::s(100.0),
        below_panel,
        16.0,
        DARKGRAY,
    );

    // Rematch button
    let rmatch_w = crate::ui::s(160.0);
    let rmatch_h = crate::ui::s(40.0);
    let rmatch_x = screen_width() / 2.0 - rmatch_w / 2.0;
    let rmatch_y = below_panel + crate::ui::s(15.0);
    let rmatch_hover = screen_mouse.x >= rmatch_x
        && screen_mouse.x <= rmatch_x + rmatch_w
        && screen_mouse.y >= rmatch_y
        && screen_mouse.y <= rmatch_y + rmatch_h;
    let rmatch_bg = if rmatch_hover {
        Color::new(0.2, 0.5, 0.3, 0.9)
    } else {
        Color::new(0.15, 0.35, 0.2, 0.8)
    };
    draw_rectangle(rmatch_x, rmatch_y, rmatch_w, rmatch_h, rmatch_bg);
    draw_rectangle_lines(
        rmatch_x,
        rmatch_y,
        rmatch_w,
        rmatch_h,
        2.0,
        Color::new(0.3, 0.8, 0.4, 1.0),
    );
    let rt = "Rematch";
    let rdims2 = crate::ui::measure_scaled_text(rt, 22);
    crate::ui::draw_scaled_text(
        rt,
        rmatch_x + rmatch_w / 2.0 - rdims2.width / 2.0,
        rmatch_y + rmatch_h / 2.0 + 7.0,
        22.0,
        WHITE,
    );
}

/// Draw disconnection overlay (shown over any phase).
pub fn draw_disconnect_overlay() {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.7),
    );
    let disc_text = "Opponent Disconnected";
    let ddims = crate::ui::measure_scaled_text(disc_text, 36);
    crate::ui::draw_scaled_text(
        disc_text,
        screen_width() / 2.0 - ddims.width / 2.0,
        screen_height() / 2.0 - crate::ui::s(10.0),
        36.0,
        Color::new(1.0, 0.3, 0.2, 1.0),
    );
    let hint = "Press R to return to lobby";
    let hdims = crate::ui::measure_scaled_text(hint, 18);
    crate::ui::draw_scaled_text(
        hint,
        screen_width() / 2.0 - hdims.width / 2.0,
        screen_height() / 2.0 + crate::ui::s(20.0),
        18.0,
        LIGHTGRAY,
    );
}
```

- [ ] **Step 2: Update `main.rs`**

Add `mod phase_ui;` to the module declarations.

Replace the entire screen-space UI `match &phase` block (lines 1463-1835) with calls to `phase_ui`:

```rust
match &phase {
    GamePhase::Lobby | GamePhase::DraftBan { .. } => {}
    GamePhase::Build => {
        phase_ui::draw_build_ui(
            &build, &progress, &units, screen_mouse, &arena_camera,
            &game_settings, &mp_player_name, &mp_opponent_name,
        );
    }
    GamePhase::WaitingForOpponent => {
        phase_ui::draw_waiting_ui(&progress, &build, &mp_player_name, &mp_opponent_name);
    }
    GamePhase::Battle => {
        phase_ui::draw_battle_ui(
            &progress, &units, &obstacles, battle_timer, ROUND_TIMEOUT,
            show_surrender_confirm, screen_mouse, world_mouse,
            &mp_player_name, &mp_opponent_name,
        );
    }
    GamePhase::RoundResult { match_state, lp_damage, loser_team } => {
        phase_ui::draw_round_result_ui(
            &progress, match_state, *lp_damage, *loser_team,
            &game_settings, &net, &mp_player_name, &mp_opponent_name,
        );
    }
    GamePhase::GameOver(winner) => {
        phase_ui::draw_game_over_ui(
            *winner, &progress, &units, &game_settings, &net,
            screen_mouse, &mp_player_name, &mp_opponent_name,
        );
    }
}
```

Replace the disconnect overlay block (lines 1837-1871) with:

```rust
if let Some(ref n) = net {
    if n.disconnected {
        phase_ui::draw_disconnect_overlay();
        if is_key_pressed(KeyCode::R) {
            progress = MatchProgress::new(true);
            phase = GamePhase::Lobby;
            build = BuildState::new(progress.round_gold(), true);
            units.clear();
            projectiles.clear();
            net = None;
            lobby.reset();
        }
    }
}
```

- [ ] **Step 3: Verify**

Run: `cargo check`
Run: `cargo clippy`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/phase_ui.rs
git commit -m "refactor: extract phase-specific UI to phase_ui.rs"
```

---

## Post-Plan Notes

After all 6 tasks, `main.rs` should be approximately **350-450 lines** containing:
- Module declarations (~25 lines)
- `window_conf()` (~10 lines)
- State initialization (~35 lines)
- Game loop with: camera setup, input polling, `match phase` dispatching to module functions, rendering calls, chat calls, `next_frame().await`
- The Build and Battle phase update logic still lives in `main.rs` within the match arms — extracting those into `build_phase.rs` and `battle_phase.rs` can be a follow-up plan once this foundation is solid

The phase update logic (Build ~400 lines, Battle ~280 lines) remains in `main.rs` for now because it mutates many local variables simultaneously. Extracting it requires either a `GameContext` struct or extensive parameter lists. That's a separate, more invasive refactor best done after this plan stabilizes.

## Estimated final state

| File | Lines (approx) |
|------|----------------|
| `main.rs` | ~400 (down from 2,249) |
| `rendering.rs` | ~250 |
| `phase_ui.rs` | ~400 |
| `draft_ban.rs` | ~180 |
| `chat.rs` | ~150 |
| `ui.rs` (expanded) | ~120 |
| `unit.rs` (expanded) | +50 |
| `arena.rs` (expanded) | +15 |
| `tech.rs` (expanded) | +20 |
| `net.rs` (expanded) | +25 |
| `economy.rs` (expanded) | +50 |
