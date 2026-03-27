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
use game_state::{BuildState, GamePhase, PlacedPack};
use pack::all_packs;
use projectile::{Projectile, PROJECTILE_RADIUS};
use team::{team_color, team_projectile_color};
use unit::{Unit, UnitShape};

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
        let left_held = is_mouse_button_down(MouseButton::Left);
        let left_released = is_mouse_button_released(MouseButton::Left);
        let right_click = is_mouse_button_pressed(MouseButton::Right);

        match &mut phase {
            GamePhase::Build => {
                // Timer countdown
                build.timer -= dt;
                if build.timer <= 0.0 {
                    phase = start_battle(&mut build, &mut units, &mut projectiles, &mut army_0, &mut army_1);
                    continue;
                }

                // Shop interaction (left click in shop area)
                if left_click && mouse.x < SHOP_W {
                    if let Some(pack_idx) = shop::draw_shop(build.builder.gold_remaining, mouse, true) {
                        if let Some(new_units) = build.purchase_pack(pack_idx) {
                            units.extend(new_units);
                        }
                    }
                }

                // Right-click to sell a pack
                if right_click && mouse.x > SHOP_W {
                    if let Some(placed_idx) = build.pack_at(mouse) {
                        let (_, removed_ids) = build.sell_pack(placed_idx);
                        units.retain(|u| !removed_ids.contains(&u.id));
                    }
                }

                // Begin drag
                if left_click && mouse.x > SHOP_W && build.dragging.is_none() {
                    if let Some(placed_idx) = build.pack_at(mouse) {
                        build.placed_packs[placed_idx].pre_drag_center = build.placed_packs[placed_idx].center;
                        build.drag_offset = build.placed_packs[placed_idx].center - mouse;
                        build.dragging = Some(placed_idx);
                    }
                }

                // Continue drag
                if let Some(drag_idx) = build.dragging {
                    if left_held {
                        let pack = &all_packs()[build.placed_packs[drag_idx].pack_index];
                        let half = PlacedPack::bbox_half_size(pack);
                        let new_center = mouse + build.drag_offset;
                        // Clamp to player's half
                        let clamped = vec2(
                            new_center.x.clamp(half.x, HALF_W - half.x),
                            new_center.y.clamp(half.y, ARENA_H - half.y),
                        );
                        build.placed_packs[drag_idx].center = clamped;
                        build.reposition_pack_units(drag_idx, &mut units);
                    }

                    if left_released {
                        // Check for overlap
                        let placed = &build.placed_packs[drag_idx];
                        let pack_index = placed.pack_index;
                        if build.would_overlap(placed.center, pack_index, Some(drag_idx)) {
                            // Snap back
                            let prev = build.placed_packs[drag_idx].pre_drag_center;
                            build.placed_packs[drag_idx].center = prev;
                            build.reposition_pack_units(drag_idx, &mut units);
                        }
                        build.dragging = None;
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

        // Draw units
        for unit in &units {
            if !unit.alive {
                continue;
            }
            let color = team_color(unit.team_id);
            draw_unit_shape(unit.pos, unit.stats.size, unit.stats.shape, color);

            // HP bar
            let bar_w = unit.stats.size * 2.0;
            let bar_h = 3.0;
            let bar_x = unit.pos.x - bar_w / 2.0;
            let bar_y = unit.pos.y - unit.stats.size - 8.0;
            let hp_frac = unit.hp / unit.stats.max_hp;
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

        // Draw projectiles
        for proj in &projectiles {
            if !proj.alive {
                continue;
            }
            let color = team_projectile_color(proj.team_id);
            draw_circle(proj.pos.x, proj.pos.y, PROJECTILE_RADIUS, color);
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
                    let half = PlacedPack::bbox_half_size(pack);
                    let min = placed.center - half;

                    // Determine color: red if overlapping while dragging, otherwise faint white
                    let bbox_color = if build.dragging == Some(i)
                        && build.would_overlap(placed.center, placed.pack_index, Some(i))
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
                    "Left-click shop to buy | Drag packs to place | Right-click to sell",
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
    }
}
