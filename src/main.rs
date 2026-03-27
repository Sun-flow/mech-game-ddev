mod arena;
mod combat;
mod economy;
mod game_state;
mod pack;
mod projectile;
mod shop;
mod team;
mod unit;

use macroquad::prelude::*;

use arena::{check_match_state, MatchState, ARENA_H, ARENA_W, HALF_W, SHOP_W};
use combat::{update_attacks, update_movement, update_projectiles, update_targeting};
use economy::ArmyBuilder;
use game_state::{BuildState, GamePhase};
use pack::all_packs;
use projectile::{projectile_visual_radius, Projectile};
use team::{team_color, team_projectile_color};
use unit::{ProjectileType, Unit, UnitKind, UnitShape};

fn window_conf() -> Conf {
    Conf {
        window_title: "RTS Unit Arena".to_string(),
        window_width: ARENA_W as i32,
        window_height: ARENA_H as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut phase = GamePhase::Build;
    let mut build = BuildState::new();
    let mut units: Vec<Unit> = Vec::new();
    let mut projectiles: Vec<Projectile> = Vec::new();
    let mut army_0 = ArmyBuilder::new(0);
    let mut army_1 = ArmyBuilder::new(0);

    loop {
        let dt = get_frame_time().min(0.05);
        let mouse = vec2(mouse_position().0, mouse_position().1);
        let left_click = is_mouse_button_pressed(MouseButton::Left);
        let right_click = is_mouse_button_pressed(MouseButton::Right);
        let middle_click = is_mouse_button_pressed(MouseButton::Middle);

        match &mut phase {
            GamePhase::Build => {
                // Timer countdown
                build.timer -= dt;
                if build.timer <= 0.0 {
                    phase = start_battle(&mut build, &mut units, &mut projectiles, &mut army_0, &mut army_1);
                    continue;
                }

                // Shop interaction (left click in shop area, only when not holding a pack)
                if left_click && mouse.x < SHOP_W && build.dragging.is_none() {
                    if let Some(pack_idx) = shop::draw_shop(build.builder.gold_remaining, mouse, true) {
                        if let Some(new_units) = build.purchase_pack(pack_idx) {
                            units.extend(new_units);
                        }
                    }
                }

                // Right-click to sell a pack (only when not holding one)
                if right_click && mouse.x > SHOP_W && build.dragging.is_none() {
                    if let Some(placed_idx) = build.pack_at(mouse) {
                        let (_, removed_ids) = build.sell_pack(placed_idx);
                        units.retain(|u| !removed_ids.contains(&u.id));
                    }
                }

                // Middle-click to rotate a pack
                if middle_click && mouse.x > SHOP_W {
                    if let Some(drag_idx) = build.dragging {
                        // Rotate the held pack
                        build.rotate_pack(drag_idx, &mut units);
                    } else if let Some(placed_idx) = build.pack_at(mouse) {
                        // Rotate a placed pack in-place
                        build.rotate_pack(placed_idx, &mut units);
                    }
                }

                // Click-to-hold/place logic
                if let Some(drag_idx) = build.dragging {
                    // Currently holding a pack — follow mouse
                    let pack = &all_packs()[build.placed_packs[drag_idx].pack_index];
                    let half = build.placed_packs[drag_idx].bbox_half_size_for(pack);
                    let clamped = vec2(
                        mouse.x.clamp(half.x, HALF_W - half.x),
                        mouse.y.clamp(half.y, ARENA_H - half.y),
                    );
                    build.placed_packs[drag_idx].center = clamped;
                    build.reposition_pack_units(drag_idx, &mut units);

                    // Left click to place
                    if left_click {
                        let placed = &build.placed_packs[drag_idx];
                        let pack_index = placed.pack_index;
                        let rotated = placed.rotated;
                        if build.would_overlap(placed.center, pack_index, Some(drag_idx), rotated) {
                            // Can't place here — snap back
                            let prev = build.placed_packs[drag_idx].pre_drag_center;
                            build.placed_packs[drag_idx].center = prev;
                            build.reposition_pack_units(drag_idx, &mut units);
                        }
                        build.dragging = None;
                    }

                    // Right click to cancel and snap back
                    if right_click {
                        let prev = build.placed_packs[drag_idx].pre_drag_center;
                        build.placed_packs[drag_idx].center = prev;
                        build.reposition_pack_units(drag_idx, &mut units);
                        build.dragging = None;
                    }
                } else {
                    // Not holding — left click to pick up a pack
                    if left_click && mouse.x > SHOP_W {
                        if let Some(placed_idx) = build.pack_at(mouse) {
                            build.placed_packs[placed_idx].pre_drag_center = build.placed_packs[placed_idx].center;
                            build.dragging = Some(placed_idx);
                        }
                    }
                }

                // Begin Round button
                let btn_w = 160.0;
                let btn_h = 40.0;
                let btn_x = ARENA_W / 2.0 - btn_w / 2.0;
                let btn_y = ARENA_H - 55.0;
                if left_click
                    && mouse.x >= btn_x
                    && mouse.x <= btn_x + btn_w
                    && mouse.y >= btn_y
                    && mouse.y <= btn_y + btn_h
                {
                    phase = start_battle(&mut build, &mut units, &mut projectiles, &mut army_0, &mut army_1);
                    continue;
                }
            }

            GamePhase::Battle => {
                update_targeting(&mut units);
                update_movement(&mut units, dt, ARENA_W, ARENA_H);
                update_attacks(&mut units, &mut projectiles, dt);
                update_projectiles(&mut projectiles, &mut units, dt);

                let state = check_match_state(&units);
                if state != MatchState::InProgress && projectiles.is_empty() {
                    let final_state = check_match_state(&units);
                    phase = GamePhase::Result(final_state);
                }
            }

            GamePhase::Result(_) => {
                if is_key_pressed(KeyCode::R) {
                    phase = GamePhase::Build;
                    build = BuildState::new();
                    units.clear();
                    projectiles.clear();
                    army_0 = ArmyBuilder::new(0);
                    army_1 = ArmyBuilder::new(0);
                }
            }
        }

        // === Render ===
        clear_background(Color::new(0.1, 0.1, 0.12, 1.0));

        // Draw arena border
        draw_rectangle_lines(0.0, 0.0, ARENA_W, ARENA_H, 2.0, GRAY);

        // Draw center divider (faint dashed line)
        draw_center_divider();

        // Draw shield barrier circles (behind units)
        for unit in &units {
            if !unit.alive || !unit.is_shield() {
                continue;
            }
            let tc = team_color(unit.team_id);
            let hp_frac = unit.hp / unit.stats.max_hp;
            let alpha = 0.12 + 0.12 * hp_frac;
            let barrier_color = Color::new(tc.r, tc.g, tc.b, alpha);
            draw_circle(unit.pos.x, unit.pos.y, unit.stats.shield_radius, barrier_color);
            draw_circle_lines(
                unit.pos.x,
                unit.pos.y,
                unit.stats.shield_radius,
                1.5,
                Color::new(tc.r, tc.g, tc.b, 0.4 * hp_frac + 0.1),
            );
        }

        // Draw units
        for unit in &units {
            if !unit.alive {
                continue;
            }
            let mut color = team_color(unit.team_id);

            // Berserker rage glow: tint toward bright red/orange as HP drops
            if unit.kind == UnitKind::Berserker {
                let hp_frac = unit.hp / unit.stats.max_hp;
                let rage = 1.0 - hp_frac;
                color.r = (color.r + rage * 0.5).min(1.0);
                color.g = (color.g * (1.0 - rage * 0.5)).max(0.1);
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

        // Draw projectiles (different visuals per type)
        for proj in &projectiles {
            if !proj.alive {
                continue;
            }
            let color = team_projectile_color(proj.team_id);
            let r = projectile_visual_radius(proj.proj_type);

            match proj.proj_type {
                ProjectileType::Laser => {
                    // Elongated bright line in direction of travel
                    let dir = proj.vel.normalize_or_zero();
                    let tail = proj.pos - dir * 8.0;
                    draw_line(tail.x, tail.y, proj.pos.x, proj.pos.y, 2.0, color);
                    draw_circle(proj.pos.x, proj.pos.y, r, WHITE);
                }
                ProjectileType::Bullet => {
                    draw_circle(proj.pos.x, proj.pos.y, r, color);
                }
                ProjectileType::Rocket => {
                    // Larger circle with a faint trail
                    let dir = proj.vel.normalize_or_zero();
                    let tail = proj.pos - dir * 6.0;
                    let trail_color = Color::new(1.0, 0.5, 0.2, 0.4);
                    draw_line(tail.x, tail.y, proj.pos.x, proj.pos.y, 3.0, trail_color);
                    draw_circle(proj.pos.x, proj.pos.y, r, color);
                }
            }
        }

        // Phase-specific UI
        match &phase {
            GamePhase::Build => {
                // Shop panel
                shop::draw_shop(build.builder.gold_remaining, mouse, false);

                // Pack bounding boxes
                let packs = all_packs();
                for (i, placed) in build.placed_packs.iter().enumerate() {
                    let pack = &packs[placed.pack_index];
                    let half = placed.bbox_half_size_for(pack);
                    let min = placed.center - half;

                    // Determine color: red if overlapping while dragging, otherwise faint white
                    let bbox_color = if build.dragging == Some(i)
                        && build.would_overlap(placed.center, placed.pack_index, Some(i), placed.rotated)
                    {
                        Color::new(1.0, 0.2, 0.2, 0.6)
                    } else if build.dragging == Some(i) {
                        Color::new(0.2, 1.0, 0.3, 0.5)
                    } else {
                        Color::new(0.5, 0.5, 0.5, 0.3)
                    };

                    draw_rectangle_lines(min.x, min.y, half.x * 2.0, half.y * 2.0, 1.5, bbox_color);

                    // Pack name label
                    draw_text(
                        pack.name,
                        min.x + 2.0,
                        min.y - 2.0,
                        14.0,
                        Color::new(0.7, 0.7, 0.7, 0.6),
                    );
                }

                // Timer
                let timer_text = format!("Build Phase: {:.0}s", build.timer.ceil());
                let dims = measure_text(&timer_text, None, 24, 1.0);
                draw_text(
                    &timer_text,
                    ARENA_W / 2.0 - dims.width / 2.0,
                    25.0,
                    24.0,
                    WHITE,
                );

                // Gold display (centered, below timer)
                let gold_text = format!("Gold: {}", build.builder.gold_remaining);
                let gdims = measure_text(&gold_text, None, 20, 1.0);
                draw_text(
                    &gold_text,
                    ARENA_W / 2.0 - gdims.width / 2.0,
                    48.0,
                    20.0,
                    Color::new(1.0, 0.85, 0.2, 1.0),
                );

                // Begin Round button
                let btn_w = 160.0;
                let btn_h = 40.0;
                let btn_x = ARENA_W / 2.0 - btn_w / 2.0;
                let btn_y = ARENA_H - 55.0;
                let btn_hovered = mouse.x >= btn_x
                    && mouse.x <= btn_x + btn_w
                    && mouse.y >= btn_y
                    && mouse.y <= btn_y + btn_h;
                let btn_bg = if btn_hovered {
                    Color::new(0.2, 0.6, 0.3, 0.9)
                } else {
                    Color::new(0.15, 0.4, 0.2, 0.8)
                };
                draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_bg);
                draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 2.0, Color::new(0.3, 0.8, 0.4, 1.0));
                let btn_text = "Begin Round";
                let tdims = measure_text(btn_text, None, 22, 1.0);
                draw_text(
                    btn_text,
                    btn_x + btn_w / 2.0 - tdims.width / 2.0,
                    btn_y + btn_h / 2.0 + 7.0,
                    22.0,
                    WHITE,
                );

                // Hint text
                draw_text(
                    "Click shop to buy | Click pack to pick up/place | Middle-click to rotate | Right-click to sell/cancel",
                    SHOP_W + 10.0,
                    ARENA_H - 10.0,
                    14.0,
                    Color::new(0.5, 0.5, 0.5, 0.7),
                );
            }

            GamePhase::Battle => {
                // Battle HUD
                let alive_0 = units.iter().filter(|u| u.alive && u.team_id == 0).count();
                let alive_1 = units.iter().filter(|u| u.alive && u.team_id == 1).count();
                draw_text(
                    &format!("Red: {}", alive_0),
                    10.0,
                    20.0,
                    20.0,
                    team_color(0),
                );
                let blue_text = format!("Blue: {}", alive_1);
                let bdims = measure_text(&blue_text, None, 20, 1.0);
                draw_text(
                    &blue_text,
                    ARENA_W - bdims.width - 10.0,
                    20.0,
                    20.0,
                    team_color(1),
                );
            }

            GamePhase::Result(result) => {
                let text = match result {
                    MatchState::Winner(tid) => {
                        let name = match tid {
                            0 => "Red",
                            1 => "Blue",
                            _ => "Unknown",
                        };
                        format!("{} Team Wins!", name)
                    }
                    MatchState::Draw => "Draw!".to_string(),
                    MatchState::InProgress => unreachable!(),
                };
                let font_size = 40.0;
                let dims = measure_text(&text, None, font_size as u16, 1.0);
                draw_text(
                    &text,
                    ARENA_W / 2.0 - dims.width / 2.0,
                    ARENA_H / 2.0,
                    font_size,
                    WHITE,
                );
                draw_text(
                    "Press R to play again",
                    ARENA_W / 2.0 - 100.0,
                    ARENA_H / 2.0 + 30.0,
                    20.0,
                    LIGHTGRAY,
                );
            }
        }

        next_frame().await;
    }
}

fn start_battle(
    build: &mut BuildState,
    units: &mut Vec<Unit>,
    projectiles: &mut Vec<Projectile>,
    army_0: &mut ArmyBuilder,
    army_1: &mut ArmyBuilder,
) -> GamePhase {
    projectiles.clear();

    // Save player army info
    *army_0 = build.builder.clone();

    // Spawn AI army on the right
    let (ai_units, ai_army) = build.spawn_ai_army();
    *army_1 = ai_army;
    units.extend(ai_units);

    GamePhase::Battle
}

fn draw_center_divider() {
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

fn draw_unit_shape(pos: Vec2, size: f32, shape: UnitShape, color: Color) {
    match shape {
        UnitShape::Circle => {
            draw_circle(pos.x, pos.y, size, color);
        }
        UnitShape::Square => {
            draw_rectangle(pos.x - size, pos.y - size, size * 2.0, size * 2.0, color);
        }
        UnitShape::Triangle => {
            let top = vec2(pos.x, pos.y - size);
            let bl = vec2(pos.x - size, pos.y + size);
            let br = vec2(pos.x + size, pos.y + size);
            draw_triangle(top, bl, br, color);
        }
        UnitShape::Diamond => {
            let top = vec2(pos.x, pos.y - size * 1.3);
            let right = vec2(pos.x + size, pos.y);
            let bottom = vec2(pos.x, pos.y + size * 1.3);
            let left = vec2(pos.x - size, pos.y);
            draw_triangle(top, right, bottom, color);
            draw_triangle(top, left, bottom, color);
        }
        UnitShape::Hexagon => {
            draw_poly(pos.x, pos.y, 6, size, 0.0, color);
        }
        UnitShape::Pentagon => {
            draw_poly(pos.x, pos.y, 5, size, 0.0, color);
        }
        UnitShape::Dot => {
            draw_circle(pos.x, pos.y, size, color);
        }
        UnitShape::Star => {
            // 6-pointed star: two overlapping triangles
            let s = size;
            // Upward triangle
            let t1a = vec2(pos.x, pos.y - s);
            let t1b = vec2(pos.x - s * 0.87, pos.y + s * 0.5);
            let t1c = vec2(pos.x + s * 0.87, pos.y + s * 0.5);
            draw_triangle(t1a, t1b, t1c, color);
            // Downward triangle
            let t2a = vec2(pos.x, pos.y + s);
            let t2b = vec2(pos.x - s * 0.87, pos.y - s * 0.5);
            let t2c = vec2(pos.x + s * 0.87, pos.y - s * 0.5);
            draw_triangle(t2a, t2b, t2c, color);
        }
        UnitShape::Cross => {
            // Plus/crosshair shape
            let arm = size * 0.35;
            // Vertical bar
            draw_rectangle(pos.x - arm, pos.y - size, arm * 2.0, size * 2.0, color);
            // Horizontal bar
            draw_rectangle(pos.x - size, pos.y - arm, size * 2.0, arm * 2.0, color);
        }
        UnitShape::Octagon => {
            draw_poly(pos.x, pos.y, 8, size, 22.5, color);
        }
    }
}
