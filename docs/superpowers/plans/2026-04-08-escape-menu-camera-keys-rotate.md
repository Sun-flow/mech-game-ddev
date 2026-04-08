# Escape Menu, Camera Keys, R-to-Rotate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an in-match escape menu (resume/settings/surrender), WASD/arrow camera panning that respects camera rotation, and R key to rotate packs.

**Architecture:** Escape menu state lives on `GameContext`. Main loop handles Escape key routing, input blocking, and pause logic. The old `show_surrender_confirm` in BattleState is removed — surrender moves to the escape menu. Settings sub-view reuses the existing `settings::draw_settings_panel` and `settings::draw_ui_scale_slider`.

**Tech Stack:** Rust, macroquad 0.4

**Spec:** `docs/superpowers/specs/2026-04-08-escape-menu-camera-keys-rotate-design.md`

---

### Task 1: Add escape menu state and R-to-rotate

**Files:**
- Modify: `src/context.rs`
- Modify: `src/build_phase.rs`

- [ ] **Step 1: Add show_escape_menu to GameContext**

In `src/context.rs`, add field to the struct and initialize in `new()`:

```rust
pub show_escape_menu: bool,
```

In the struct definition, after `pub show_grid: bool,`:
```rust
pub show_escape_menu: bool,
```

In `new()`, after `show_grid: false,`:
```rust
show_escape_menu: false,
```

- [ ] **Step 2: Add R key rotation in build_phase.rs**

In `src/build_phase.rs`, find the middle-click rotation block (around line 190-201). Add `is_key_pressed(KeyCode::R)` as an alternative trigger. The current code:

```rust
    // Middle-click to rotate (only unlocked)
    if middle_click && screen_mouse.x > shop_w() {
```

Change to:

```rust
    // Middle-click or R to rotate (only unlocked)
    let rotate_pressed = middle_click || (is_key_pressed(KeyCode::R) && !ctx.chat.open);
    if rotate_pressed && screen_mouse.x > shop_w() {
```

The `!ctx.chat.open` guard prevents R from rotating packs while typing in chat.

Also add a guard at the top of the `update` function to skip all input when escape menu is open. After the network poll (around line 28-29), add:

```rust
    // Skip all game input when escape menu is open
    if ctx.show_escape_menu {
        // In single-player, don't count down build timer while paused
        if ctx.net.is_none() {
            return;
        }
        // In multiplayer, still count down timer but skip input
        ctx.build.timer -= dt;
        if ctx.build.timer <= 0.0 {
            net::send_build_complete(&mut ctx.net, &ctx.build, ctx.local_player_id);
            ctx.phase = GamePhase::WaitingForOpponent;
        }
        return;
    }
```

- [ ] **Step 3: Commit**

```bash
git add src/context.rs src/build_phase.rs
git commit -m "feat: add escape menu state, R-to-rotate packs, input blocking during menu"
```

---

### Task 2: Remove old surrender confirm system

**Files:**
- Modify: `src/battle_phase.rs`
- Modify: `src/phase_ui.rs`

- [ ] **Step 1: Remove show_surrender_confirm from BattleState**

In `src/battle_phase.rs`, remove `pub show_surrender_confirm: bool` from the `BattleState` struct. Remove it from `new()` and `reset()`.

- [ ] **Step 2: Remove surrender toggle and handling from battle_phase update**

Remove the Escape key surrender toggle (lines ~62-63):
```rust
    // DELETE:
    if is_key_pressed(KeyCode::Escape) {
        battle.show_surrender_confirm = !battle.show_surrender_confirm;
    }
```

Remove the `if battle.show_surrender_confirm` branch that pauses simulation (line ~71). Change:
```rust
    if battle.show_surrender_confirm {
        // Battle paused while surrender overlay is shown
    } else if ctx.net.is_some() {
```
To:
```rust
    if ctx.show_escape_menu && ctx.net.is_none() {
        // Single-player: pause simulation while escape menu is open
    } else if ctx.net.is_some() {
```

Remove the entire surrender confirmation click handling block (lines ~180-201 — the `if battle.show_surrender_confirm && ms.left_click { ... }` block).

Remove `battle.show_surrender_confirm = false;` from the two places in the round-end logic (search for it — appears before setting `ctx.phase = GamePhase::RoundResult`).

- [ ] **Step 3: Remove surrender overlay from phase_ui.rs draw_battle_ui**

Remove the `show_surrender_confirm: bool` parameter from `draw_battle_ui` signature. Remove the entire "Surrender confirmation overlay" block (lines ~207-240).

- [ ] **Step 4: Update main.rs call to draw_battle_ui**

In `src/main.rs`, remove `battle.show_surrender_confirm` from the `draw_battle_ui` call:

```rust
// Before:
phase_ui::draw_battle_ui(&ctx.progress, &ctx.units, &ctx.obstacles, battle.timer, battle_phase::ROUND_TIMEOUT, battle.show_surrender_confirm, mouse.screen_mouse, mouse.world_mouse, ctx.local_player_id);

// After:
phase_ui::draw_battle_ui(&ctx.progress, &ctx.units, &ctx.obstacles, battle.timer, battle_phase::ROUND_TIMEOUT, mouse.screen_mouse, mouse.world_mouse, ctx.local_player_id);
```

- [ ] **Step 5: Run `cargo check`**

Expected: Clean.

- [ ] **Step 6: Commit**

```bash
git add src/battle_phase.rs src/phase_ui.rs src/main.rs
git commit -m "refactor: remove old surrender confirm system, add single-player pause in escape menu"
```

---

### Task 3: Escape key routing and WASD/arrow camera controls in main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add escape key routing**

In `src/main.rs`, BEFORE the `match &mut ctx.phase` block (after the camera controls, around line 133), add escape key handling for match phases:

```rust
        // Escape menu toggle (only during match phases)
        let in_match = matches!(ctx.phase, GamePhase::Build | GamePhase::WaitingForOpponent | GamePhase::Battle | GamePhase::RoundResult { .. });
        if in_match && is_key_pressed(KeyCode::Escape) {
            if ctx.chat.open {
                ctx.chat.open = false;
                ctx.chat.input.clear();
            } else {
                ctx.show_escape_menu = !ctx.show_escape_menu;
            }
        }
```

- [ ] **Step 2: Wrap camera controls with escape menu guard**

The existing camera control block starts with `if !matches!(ctx.phase, GamePhase::Lobby)`. Add the escape menu check:

```rust
        if !matches!(ctx.phase, GamePhase::Lobby) && !ctx.show_escape_menu {
```

This blocks zoom, pan, Q/E rotation, and WASD when menu is open.

- [ ] **Step 3: Add WASD/arrow camera panning**

Inside the camera control block (after the Q/E rotation code, before the closing `}`), add:

```rust
            // WASD / Arrow key camera panning (relative to screen orientation)
            if !ctx.chat.open {
                let pan_speed = 400.0 * dt;
                let angle_rad = camera_angle.to_radians();
                let screen_right = vec2(angle_rad.cos(), angle_rad.sin());
                let screen_up = vec2(-angle_rad.sin(), angle_rad.cos());

                if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
                    camera_target -= screen_right * pan_speed;
                }
                if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
                    camera_target += screen_right * pan_speed;
                }
                if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
                    camera_target -= screen_up * pan_speed;
                }
                if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
                    camera_target += screen_up * pan_speed;
                }
            }
```

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: escape key routing, WASD/arrow camera panning relative to screen"
```

---

### Task 4: Escape menu rendering and interaction

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add escape menu rendering**

In `src/main.rs`, AFTER the phase-specific UI section and BEFORE the disconnection overlay (around line 262), add the escape menu rendering:

```rust
        // === Escape Menu Overlay ===
        if ctx.show_escape_menu {
            // Dark overlay
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.7));

            let btn_w = ui::s(200.0);
            let btn_h = ui::s(40.0);
            let cx = screen_width() / 2.0;
            let gap = ui::s(12.0);
            let title = "PAUSED";
            let tdims = ui::measure_scaled_text(title, 36);
            let menu_top = screen_height() / 2.0 - ui::s(100.0);
            ui::draw_scaled_text(title, cx - tdims.width / 2.0, menu_top, 36.0, WHITE);

            let mut btn_y = menu_top + ui::s(40.0);

            // Helper closure for menu buttons
            let draw_menu_btn = |label: &str, y: f32, mouse: Vec2, clicked: bool| -> bool {
                let bx = cx - btn_w / 2.0;
                let hover = mouse.x >= bx && mouse.x <= bx + btn_w && mouse.y >= y && mouse.y <= y + btn_h;
                let bg = if hover { Color::new(0.25, 0.25, 0.3, 0.95) } else { Color::new(0.15, 0.15, 0.2, 0.9) };
                draw_rectangle(bx, y, btn_w, btn_h, bg);
                draw_rectangle_lines(bx, y, btn_w, btn_h, 1.0, Color::new(0.5, 0.5, 0.6, 0.8));
                let dims = ui::measure_scaled_text(label, 20);
                ui::draw_scaled_text(label, bx + btn_w / 2.0 - dims.width / 2.0, y + btn_h / 2.0 + 6.0, 20.0, WHITE);
                hover && clicked
            };

            // Resume button
            if draw_menu_btn("Resume", btn_y, screen_mouse, left_click) {
                ctx.show_escape_menu = false;
            }
            btn_y += btn_h + gap;

            // Settings button
            if draw_menu_btn("Settings", btn_y, screen_mouse, left_click) {
                // TODO: switch to settings sub-view (Task 5)
            }
            btn_y += btn_h + gap;

            // Surrender button
            if draw_menu_btn("Surrender", btn_y, screen_mouse, left_click) {
                ctx.progress.player_mut(ctx.local_player_id).lp = 0;
                ctx.show_escape_menu = false;
                let winner = ctx.progress.game_winner().unwrap_or(0);
                ctx.phase = GamePhase::GameOver(winner);
            }
        }
```

- [ ] **Step 2: Run `cargo check`**

Expected: Clean.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: escape menu overlay with resume, settings (placeholder), surrender"
```

---

### Task 5: Settings sub-view in escape menu

**Files:**
- Modify: `src/context.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add escape menu mode enum or bool**

Add a field to track whether the settings sub-view is active. In `src/context.rs`, add:

```rust
pub escape_menu_settings: bool,
```

After `show_escape_menu`. Initialize to `false` in `new()`.

- [ ] **Step 2: Update escape menu rendering for settings sub-view**

In `src/main.rs`, replace the Settings button placeholder in the escape menu block. When `ctx.escape_menu_settings` is true, render the settings panel instead of the menu buttons:

```rust
        if ctx.show_escape_menu {
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.7));

            if ctx.escape_menu_settings {
                // Settings sub-view
                let panel_w = ui::s(400.0);
                let panel_h = ui::s(350.0);
                let px = screen_width() / 2.0 - panel_w / 2.0;
                let py = screen_height() / 2.0 - panel_h / 2.0;
                draw_rectangle(px, py, panel_w, panel_h, Color::new(0.1, 0.1, 0.15, 0.95));
                draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, Color::new(0.4, 0.4, 0.5, 1.0));

                let title = "Settings";
                let tdims = ui::measure_scaled_text(title, 24);
                ui::draw_scaled_text(title, px + panel_w / 2.0 - tdims.width / 2.0, py + ui::s(30.0), 24.0, WHITE);

                // Game settings
                settings::draw_settings_panel(&mut ctx.game_settings, screen_mouse, left_click);

                // UI scale slider
                settings::draw_ui_scale_slider(&mut main_settings, screen_mouse, left_click, mouse.left_down, px, py + ui::s(55.0));

                // Back button
                let back_w = ui::s(120.0);
                let back_h = ui::s(35.0);
                let back_x = px + panel_w / 2.0 - back_w / 2.0;
                let back_y = py + panel_h - ui::s(50.0);
                let back_hover = screen_mouse.x >= back_x && screen_mouse.x <= back_x + back_w && screen_mouse.y >= back_y && screen_mouse.y <= back_y + back_h;
                let back_bg = if back_hover { Color::new(0.3, 0.3, 0.35, 0.9) } else { Color::new(0.2, 0.2, 0.25, 0.8) };
                draw_rectangle(back_x, back_y, back_w, back_h, back_bg);
                draw_rectangle_lines(back_x, back_y, back_w, back_h, 1.0, WHITE);
                let bt = "Back";
                let bdims = ui::measure_scaled_text(bt, 18);
                ui::draw_scaled_text(bt, back_x + back_w / 2.0 - bdims.width / 2.0, back_y + back_h / 2.0 + 5.0, 18.0, WHITE);
                if back_hover && left_click {
                    ctx.escape_menu_settings = false;
                }
            } else {
                // Main escape menu (existing code from Task 4)
                // ... title, Resume, Settings, Surrender buttons ...
            }
        }
```

Update the Settings button to set the flag:
```rust
            if draw_menu_btn("Settings", btn_y, screen_mouse, left_click) {
                ctx.escape_menu_settings = true;
            }
```

- [ ] **Step 3: Update escape key to handle settings sub-view**

In the escape key routing (from Task 3), when escape is pressed and the settings sub-view is open, go back to the main menu instead of closing:

```rust
        if in_match && is_key_pressed(KeyCode::Escape) {
            if ctx.chat.open {
                ctx.chat.open = false;
                ctx.chat.input.clear();
            } else if ctx.escape_menu_settings {
                ctx.escape_menu_settings = false;
            } else {
                ctx.show_escape_menu = !ctx.show_escape_menu;
            }
        }
```

- [ ] **Step 4: Reset escape menu state when menu closes**

When `show_escape_menu` is set to false (Resume button or Escape toggle), also reset `escape_menu_settings`:

In the Resume button handler:
```rust
            if draw_menu_btn("Resume", btn_y, screen_mouse, left_click) {
                ctx.show_escape_menu = false;
                ctx.escape_menu_settings = false;
            }
```

And in the Surrender handler:
```rust
            if draw_menu_btn("Surrender", btn_y, screen_mouse, left_click) {
                ctx.progress.player_mut(ctx.local_player_id).lp = 0;
                ctx.show_escape_menu = false;
                ctx.escape_menu_settings = false;
                let winner = ctx.progress.game_winner().unwrap_or(0);
                ctx.phase = GamePhase::GameOver(winner);
            }
```

- [ ] **Step 5: Run `cargo check`**

Expected: Clean.

- [ ] **Step 6: Run `cargo clippy`**

Expected: Only pre-existing warnings.

- [ ] **Step 7: Commit**

```bash
git add src/context.rs src/main.rs
git commit -m "feat: settings sub-view in escape menu with game settings and UI scale"
```

---

### Task 6: Final verification

**Files:** All modified files (verification only)

- [ ] **Step 1: Verify escape menu doesn't activate in wrong phases**

Grep for `show_escape_menu` to verify it's only toggled during match phases:

```bash
grep -rn "show_escape_menu" src/ --include="*.rs"
```

- [ ] **Step 2: Verify old surrender system fully removed**

```bash
grep -rn "show_surrender_confirm" src/ --include="*.rs"
```

Expected: No matches.

- [ ] **Step 3: Run cargo check and clippy**

```bash
cargo check 2>&1
cargo clippy 2>&1
```

- [ ] **Step 4: Commit any cleanup**

```bash
git add -A
git commit -m "cleanup: final verification for escape menu and camera controls"
```
