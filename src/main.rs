mod arena;
mod combat;
mod economy;
mod game_state;
mod lobby;
mod match_progress;
mod net;
mod pack;
mod projectile;
mod shop;
mod team;
mod tech;
mod tech_ui;
mod unit;

use macroquad::prelude::*;

use arena::{check_match_state, MatchState, ARENA_H, ARENA_W, HALF_W, SHOP_W};
use combat::{update_attacks, update_movement, update_projectiles, update_targeting};
use economy::{ai_buy_techs, ArmyBuilder};
use game_state::{BuildState, GamePhase};
use match_progress::MatchProgress;
use pack::all_packs;
use projectile::{projectile_visual_radius, Projectile};
use team::{team_color, team_projectile_color};
use tech::TechState;
use unit::{ProjectileType, Unit, UnitKind, UnitShape};

const FIXED_DT: f32 = 1.0 / 60.0;

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
    let mut progress = MatchProgress::new();
    let mut phase = GamePhase::Lobby;
    let mut build = BuildState::new(progress.round_gold());
    let mut units: Vec<Unit> = Vec::new();
    let mut projectiles: Vec<Projectile> = Vec::new();
    let mut _army_0 = ArmyBuilder::new(0);
    let mut _army_1 = ArmyBuilder::new(0);

    let mut net: Option<net::NetState> = None;
    let mut lobby = lobby::LobbyState::new();
    let mut battle_accumulator: f32 = 0.0;

    loop {
        let dt = get_frame_time().min(0.05);
        let mouse = vec2(mouse_position().0, mouse_position().1);
        let left_click = is_mouse_button_pressed(MouseButton::Left);
        let right_click = is_mouse_button_pressed(MouseButton::Right);
        let middle_click = is_mouse_button_pressed(MouseButton::Middle);

        match &mut phase {
            GamePhase::Lobby => {
                match lobby.update() {
                    lobby::LobbyResult::StartMultiplayer => {
                        net = lobby.net.take();
                        phase = GamePhase::Build;
                        continue;
                    }
                    lobby::LobbyResult::StartVsAi => {
                        net = None;
                        phase = GamePhase::Build;
                        continue;
                    }
                    lobby::LobbyResult::Waiting => {}
                }

                lobby.draw();

                // No longer need the S-key hint since we have a button
                if false {
                    let hint = "";
                    let hdims = measure_text(hint, None, 16, 1.0);
                    draw_text(
                        hint,
                        ARENA_W / 2.0 - hdims.width / 2.0,
                        ARENA_H / 2.0 + 100.0,
                        16.0,
                        Color::new(0.6, 0.6, 0.4, 0.8),
                    );
                }

                next_frame().await;
                continue;
            }

            GamePhase::Build => {
                // Poll network
                if let Some(ref mut n) = net {
                    n.poll();
                }

                // Timer countdown
                build.timer -= dt;
                if build.timer <= 0.0 {
                    if net.is_some() {
                        // Multiplayer: send build, transition to waiting
                        send_build_complete(&mut net, &build, &progress);
                        phase = GamePhase::WaitingForOpponent;
                    } else {
                        // Single-player: start battle immediately with AI
                        phase = start_battle_ai(
                            &mut build,
                            &mut units,
                            &mut projectiles,
                            &mut progress,
                        );
                        battle_accumulator = 0.0;
                    }
                    continue;
                }

                // Shop interaction (left click in shop area, only when not holding a pack)
                if left_click && mouse.x < SHOP_W && build.dragging.is_none() {
                    if let Some(pack_idx) =
                        shop::draw_shop(build.builder.gold_remaining, mouse, true)
                    {
                        if let Some(new_units) = build.purchase_pack(
                            pack_idx,
                            progress.round,
                            &progress.player_techs,
                        ) {
                            units.extend(new_units);
                        }
                    }
                }

                // Tech panel interaction (when a pack is selected)
                if left_click && build.selected_pack.is_some() {
                    let sel_idx = build.selected_pack.unwrap();
                    let placed = &build.placed_packs[sel_idx];
                    let kind = all_packs()[placed.pack_index].kind;
                    let cs = tech_ui::PackCombatStats::from_units(&units, &placed.unit_ids);
                    if let Some(tech_id) = tech_ui::draw_tech_panel(
                        kind,
                        &progress.player_techs,
                        build.builder.gold_remaining,
                        mouse,
                        true,
                        Some(&cs),
                    ) {
                        let cost = progress.player_techs.effective_cost(kind);
                        if build.builder.gold_remaining >= cost {
                            build.builder.gold_remaining -= cost;
                            progress.player_techs.purchase(kind, tech_id);
                            // Track tech purchase for network sync
                            build.round_tech_purchases.push((kind, tech_id));
                            // Refresh ALL units of this kind with new tech stats
                            refresh_units_of_kind(&mut units, kind, &progress.player_techs);
                        }
                    }
                }

                // Right-click: sell if on unlocked pack, otherwise deselect
                if right_click && mouse.x > SHOP_W && build.dragging.is_none() {
                    let mut sold = false;
                    if let Some(placed_idx) = build.pack_at(mouse) {
                        if !build.placed_packs[placed_idx].locked {
                            if let Some((_, removed_ids)) = build.sell_pack(placed_idx) {
                                units.retain(|u| !removed_ids.contains(&u.id));
                                sold = true;
                            }
                        }
                    }
                    if !sold {
                        // Deselect on right-click in empty space or on locked pack
                        build.selected_pack = None;
                    }
                }

                // Middle-click to rotate (only unlocked)
                if middle_click && mouse.x > SHOP_W {
                    if let Some(drag_idx) = build.dragging {
                        if !build.placed_packs[drag_idx].locked {
                            build.rotate_pack(drag_idx, &mut units);
                        }
                    } else if let Some(placed_idx) = build.pack_at(mouse) {
                        if !build.placed_packs[placed_idx].locked {
                            build.rotate_pack(placed_idx, &mut units);
                        }
                    }
                }

                // Click-to-hold/place logic with selection
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

                    if left_click {
                        let placed = &build.placed_packs[drag_idx];
                        let pack_index = placed.pack_index;
                        let rotated = placed.rotated;
                        if build.would_overlap(placed.center, pack_index, Some(drag_idx), rotated) {
                            let prev = build.placed_packs[drag_idx].pre_drag_center;
                            build.placed_packs[drag_idx].center = prev;
                            build.reposition_pack_units(drag_idx, &mut units);
                        }
                        build.dragging = None;
                    }

                    if right_click {
                        let prev = build.placed_packs[drag_idx].pre_drag_center;
                        build.placed_packs[drag_idx].center = prev;
                        build.reposition_pack_units(drag_idx, &mut units);
                        build.dragging = None;
                    }
                } else if left_click && mouse.x > SHOP_W {
                    // Not holding — selection logic
                    if let Some(placed_idx) = build.pack_at(mouse) {
                        if build.selected_pack == Some(placed_idx) {
                            // Already selected -> pick up (if not locked)
                            if !build.placed_packs[placed_idx].locked {
                                build.placed_packs[placed_idx].pre_drag_center =
                                    build.placed_packs[placed_idx].center;
                                build.dragging = Some(placed_idx);
                                build.selected_pack = None;
                            }
                        } else {
                            // Select this pack
                            build.selected_pack = Some(placed_idx);
                        }
                    } else if let Some(sel_idx) = build.selected_pack {
                        // Clicked empty space with a selected pack -> pick it up (if not locked)
                        if !build.placed_packs[sel_idx].locked {
                            build.placed_packs[sel_idx].pre_drag_center =
                                build.placed_packs[sel_idx].center;
                            build.dragging = Some(sel_idx);
                            build.selected_pack = None;
                        } else {
                            build.selected_pack = None;
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
                    if net.is_some() {
                        // Multiplayer: send build data, wait for opponent
                        send_build_complete(&mut net, &build, &progress);
                        phase = GamePhase::WaitingForOpponent;
                    } else {
                        // Single-player: start battle with AI
                        phase = start_battle_ai(
                            &mut build,
                            &mut units,
                            &mut projectiles,
                            &mut progress,
                        );
                        battle_accumulator = 0.0;
                    }
                    continue;
                }
            }

            GamePhase::WaitingForOpponent => {
                // Poll network
                if let Some(ref mut n) = net {
                    n.poll();

                    // Check if opponent build data arrived
                    if let Some(opp_build) = n.take_opponent_build() {
                        // Apply opponent build
                        let opp_units = progress.apply_opponent_build(&opp_build);

                        // Remove old opponent units, respawn from stored packs
                        units.retain(|u| u.team_id == 0);
                        units.extend(progress.respawn_opponent_units());

                        // Also add any newly spawned opponent units from this round
                        // (apply_opponent_build already added them to opponent_packs,
                        //  respawn_opponent_units covers all stored packs including new ones)
                        // So we don't need to extend with opp_units separately.
                        let _ = opp_units;

                        projectiles.clear();

                        // Seed RNG for deterministic battle
                        macroquad::rand::srand(progress.round as u64);
                        battle_accumulator = 0.0;

                        // Reset per-round damage stats
                        for unit in units.iter_mut() {
                            unit.damage_dealt_round = 0.0;
                            unit.damage_soaked_round = 0.0;
                        }

                        phase = GamePhase::Battle;
                        continue;
                    }
                }
            }

            GamePhase::Battle => {
                // Poll network
                if let Some(ref mut n) = net {
                    n.poll();
                }

                if net.is_some() {
                    // Multiplayer: fixed timestep for determinism
                    battle_accumulator += dt;
                    while battle_accumulator >= FIXED_DT {
                        battle_accumulator -= FIXED_DT;
                        update_targeting(&mut units);
                        update_movement(&mut units, FIXED_DT, ARENA_W, ARENA_H);
                        update_attacks(
                            &mut units,
                            &mut projectiles,
                            FIXED_DT,
                            &progress.player_techs,
                            &progress.opponent_techs,
                        );
                        update_projectiles(&mut projectiles, &mut units, FIXED_DT);
                    }
                } else {
                    // Single-player: variable timestep (original behavior)
                    update_targeting(&mut units);
                    update_movement(&mut units, dt, ARENA_W, ARENA_H);
                    update_attacks(
                        &mut units,
                        &mut projectiles,
                        dt,
                        &progress.player_techs,
                        &progress.opponent_techs,
                    );
                    update_projectiles(&mut projectiles, &mut units, dt);
                }

                let state = check_match_state(&units);
                if state != MatchState::InProgress && projectiles.is_empty() {
                    let final_state = check_match_state(&units);

                    // Calculate LP damage
                    let (lp_damage, loser_team) = match &final_state {
                        MatchState::Winner(winner) => {
                            let damage = MatchProgress::calculate_lp_damage(&units, *winner);
                            let loser = if *winner == 0 { 1u8 } else { 0u8 };
                            (damage, Some(loser))
                        }
                        MatchState::Draw => (0, None),
                        MatchState::InProgress => unreachable!(),
                    };

                    // Apply LP damage
                    if let Some(loser) = loser_team {
                        if loser == 0 {
                            progress.player_lp -= lp_damage;
                        } else {
                            progress.opponent_lp -= lp_damage;
                        }
                    }

                    phase = GamePhase::RoundResult {
                        match_state: final_state,
                        lp_damage,
                        loser_team,
                    };
                }
            }

            GamePhase::RoundResult { .. } => {
                // Poll network
                if let Some(ref mut n) = net {
                    n.poll();
                }

                if is_key_pressed(KeyCode::Space) {
                    if progress.is_game_over() {
                        phase = GamePhase::GameOver(progress.game_winner().unwrap_or(0));
                    } else {
                        // Save leftover gold for next round
                        progress.player_saved_gold = build.builder.gold_remaining;

                        // Advance to next round
                        progress.advance_round();

                        // Lock all current player packs
                        build.lock_current_packs();
                        let locked_packs: Vec<_> = build.placed_packs.clone();
                        let next_id = build.next_id;

                        // Save accumulated stats before clearing
                        let old_stats: std::collections::HashMap<u64, (f32, f32, f32, f32, u32)> =
                            units
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

                        // Clear units and respawn all from locked packs
                        units.clear();
                        build = BuildState::new_round(progress.round_gold(), locked_packs, next_id);

                        // Respawn all locked PLAYER pack units
                        units.extend(respawn_player_units(&build, &progress));

                        // Restore accumulated stats on respawned units
                        for unit in units.iter_mut() {
                            if let Some(&(ddt, dst, ddr, dsr, kt)) = old_stats.get(&unit.id) {
                                unit.damage_dealt_total = ddt;
                                unit.damage_soaked_total = dst;
                                unit.damage_dealt_round = ddr;
                                unit.damage_soaked_round = dsr;
                                unit.kills_total = kt;
                            }
                        }

                        // Respawn opponent units (visible during build phase)
                        if net.is_none() {
                            // Single-player: respawn AI units
                            units.extend(progress.respawn_opponent_units());
                        } else {
                            // Multiplayer: respawn from stored opponent packs
                            units.extend(progress.respawn_opponent_units());
                        }

                        projectiles.clear();
                        phase = GamePhase::Build;
                    }
                }
            }

            GamePhase::GameOver(_) => {
                if is_key_pressed(KeyCode::R) {
                    progress = MatchProgress::new();
                    phase = GamePhase::Lobby;
                    build = BuildState::new(progress.round_gold());
                    units.clear();
                    projectiles.clear();
                    _army_0 = ArmyBuilder::new(0);
                    _army_1 = ArmyBuilder::new(0);
                    net = None;
                    lobby.reset();
                }
            }
        }

        // === Render ===
        clear_background(Color::new(0.1, 0.1, 0.12, 1.0));

        // Skip normal rendering for Lobby phase (it draws its own UI above)
        if matches!(phase, GamePhase::Lobby) {
            next_frame().await;
            continue;
        }

        draw_rectangle_lines(0.0, 0.0, ARENA_W, ARENA_H, 2.0, GRAY);
        draw_center_divider();

        // Draw shield barrier circles
        for unit in &units {
            if !unit.alive || !unit.is_shield() {
                continue;
            }
            let tc = team_color(unit.team_id);
            let hp_frac = unit.hp / unit.stats.max_hp;
            let alpha = 0.12 + 0.12 * hp_frac;
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
                Color::new(tc.r, tc.g, tc.b, 0.4 * hp_frac + 0.1),
            );
        }

        // Draw units
        for unit in &units {
            if !unit.alive {
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

        // Draw projectiles
        for proj in &projectiles {
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

        // === Phase-specific UI ===
        match &phase {
            GamePhase::Lobby => {
                // Handled above with early continue
            }

            GamePhase::Build => {
                shop::draw_shop(build.builder.gold_remaining, mouse, false);

                // Pack bounding boxes
                let packs = all_packs();
                for (i, placed) in build.placed_packs.iter().enumerate() {
                    let pack = &packs[placed.pack_index];
                    let half = placed.bbox_half_size_for(pack);
                    let min = placed.center - half;

                    let bbox_color = if build.dragging == Some(i)
                        && build.would_overlap(
                            placed.center,
                            placed.pack_index,
                            Some(i),
                            placed.rotated,
                        )
                    {
                        Color::new(1.0, 0.2, 0.2, 0.6)
                    } else if build.dragging == Some(i) {
                        Color::new(0.2, 1.0, 0.3, 0.5)
                    } else if build.selected_pack == Some(i) {
                        Color::new(0.2, 0.8, 1.0, 0.8) // cyan highlight for selected
                    } else if placed.locked {
                        Color::new(0.3, 0.3, 0.4, 0.25) // dimmer for locked
                    } else {
                        Color::new(0.5, 0.5, 0.5, 0.3)
                    };

                    let thickness = if build.selected_pack == Some(i) {
                        2.5
                    } else {
                        1.5
                    };
                    draw_rectangle_lines(
                        min.x,
                        min.y,
                        half.x * 2.0,
                        half.y * 2.0,
                        thickness,
                        bbox_color,
                    );

                    // Pack label
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
                    draw_text(&label, min.x + 2.0, min.y - 2.0, 14.0, label_color);
                }

                // Opponent pack bounding boxes (from previous rounds, visible during build)
                for opponent_pack in &progress.opponent_packs {
                    let pack = &packs[opponent_pack.pack_index];
                    let half = game_state::PlacedPack::bbox_half_size_rotated(
                        pack,
                        opponent_pack.rotated,
                    );
                    let min = opponent_pack.center - half;
                    let bbox_color = Color::new(0.3, 0.3, 0.5, 0.2);
                    draw_rectangle_lines(
                        min.x,
                        min.y,
                        half.x * 2.0,
                        half.y * 2.0,
                        1.0,
                        bbox_color,
                    );
                    let label = format!("{} (R{})", pack.name, opponent_pack.round_placed);
                    draw_text(
                        &label,
                        min.x + 2.0,
                        min.y - 2.0,
                        12.0,
                        Color::new(0.4, 0.4, 0.6, 0.4),
                    );
                }

                // Tech panel (when a pack is selected)
                if let Some(sel_idx) = build.selected_pack {
                    if sel_idx < build.placed_packs.len() {
                        let placed = &build.placed_packs[sel_idx];
                        let kind = packs[placed.pack_index].kind;
                        let cs = tech_ui::PackCombatStats::from_units(&units, &placed.unit_ids);
                        tech_ui::draw_tech_panel(
                            kind,
                            &progress.player_techs,
                            build.builder.gold_remaining,
                            mouse,
                            false,
                            Some(&cs),
                        );
                    }
                }

                // Top HUD
                draw_hud(&progress, build.builder.gold_remaining, build.timer);

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
                draw_rectangle_lines(
                    btn_x,
                    btn_y,
                    btn_w,
                    btn_h,
                    2.0,
                    Color::new(0.3, 0.8, 0.4, 1.0),
                );
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
                    "Click to select | Double-click to move | Middle-click rotate | Right-click sell",
                    SHOP_W + 10.0,
                    ARENA_H - 10.0,
                    13.0,
                    Color::new(0.5, 0.5, 0.5, 0.7),
                );
            }

            GamePhase::WaitingForOpponent => {
                draw_hud(&progress, build.builder.gold_remaining, 0.0);

                let dots = ".".repeat(((get_time() * 2.0) as usize % 4));
                let wait_text = format!("Waiting for opponent{}", dots);
                let wdims = measure_text(&wait_text, None, 28, 1.0);
                draw_text(
                    &wait_text,
                    ARENA_W / 2.0 - wdims.width / 2.0,
                    ARENA_H / 2.0,
                    28.0,
                    Color::new(0.7, 0.7, 0.9, 1.0),
                );
            }

            GamePhase::Battle => {
                draw_hud(&progress, 0, 0.0);
                let alive_0 = units.iter().filter(|u| u.alive && u.team_id == 0).count();
                let alive_1 = units.iter().filter(|u| u.alive && u.team_id == 1).count();
                draw_text(
                    &format!("Red: {}", alive_0),
                    10.0,
                    ARENA_H - 15.0,
                    20.0,
                    team_color(0),
                );
                let blue_text = format!("Blue: {}", alive_1);
                let bdims = measure_text(&blue_text, None, 20, 1.0);
                draw_text(
                    &blue_text,
                    ARENA_W - bdims.width - 10.0,
                    ARENA_H - 15.0,
                    20.0,
                    team_color(1),
                );
            }

            GamePhase::RoundResult {
                match_state,
                lp_damage,
                loser_team,
            } => {
                draw_hud(&progress, 0, 0.0);

                let text = match match_state {
                    MatchState::Winner(tid) => {
                        let name = if *tid == 0 { "Red" } else { "Blue" };
                        format!("{} wins round {}!", name, progress.round)
                    }
                    MatchState::Draw => format!("Round {} - Draw!", progress.round),
                    MatchState::InProgress => unreachable!(),
                };

                let dims = measure_text(&text, None, 36, 1.0);
                draw_text(
                    &text,
                    ARENA_W / 2.0 - dims.width / 2.0,
                    ARENA_H / 2.0 - 30.0,
                    36.0,
                    WHITE,
                );

                if let Some(loser) = loser_team {
                    let loser_name = if *loser == 0 { "Player" } else { "Opponent" };
                    let dmg_text = format!("{} loses {} LP", loser_name, lp_damage);
                    let ddims = measure_text(&dmg_text, None, 22, 1.0);
                    draw_text(
                        &dmg_text,
                        ARENA_W / 2.0 - ddims.width / 2.0,
                        ARENA_H / 2.0 + 5.0,
                        22.0,
                        Color::new(1.0, 0.4, 0.3, 1.0),
                    );
                }

                let next_text = if progress.is_game_over() {
                    "Press Space to see results"
                } else {
                    "Press Space for next round"
                };
                let ndims = measure_text(next_text, None, 18, 1.0);
                draw_text(
                    next_text,
                    ARENA_W / 2.0 - ndims.width / 2.0,
                    ARENA_H / 2.0 + 35.0,
                    18.0,
                    LIGHTGRAY,
                );
            }

            GamePhase::GameOver(winner) => {
                let text = if *winner == 0 {
                    "YOU WIN!"
                } else {
                    "YOU LOSE!"
                };
                let color = if *winner == 0 {
                    Color::new(0.2, 1.0, 0.3, 1.0)
                } else {
                    Color::new(1.0, 0.3, 0.2, 1.0)
                };
                let dims = measure_text(text, None, 48, 1.0);
                draw_text(
                    text,
                    ARENA_W / 2.0 - dims.width / 2.0,
                    ARENA_H / 2.0 - 20.0,
                    48.0,
                    color,
                );

                let round_text = format!("Final Round: {}", progress.round);
                let rdims = measure_text(&round_text, None, 22, 1.0);
                draw_text(
                    &round_text,
                    ARENA_W / 2.0 - rdims.width / 2.0,
                    ARENA_H / 2.0 + 15.0,
                    22.0,
                    WHITE,
                );

                draw_text(
                    "Press R to restart",
                    ARENA_W / 2.0 - 80.0,
                    ARENA_H / 2.0 + 45.0,
                    18.0,
                    LIGHTGRAY,
                );
            }
        }

        // Disconnection overlay (shown over any phase if net is disconnected)
        if let Some(ref n) = net {
            if n.disconnected {
                // Semi-transparent dark overlay
                draw_rectangle(0.0, 0.0, ARENA_W, ARENA_H, Color::new(0.0, 0.0, 0.0, 0.7));
                let disc_text = "Opponent Disconnected";
                let ddims = measure_text(disc_text, None, 36, 1.0);
                draw_text(
                    disc_text,
                    ARENA_W / 2.0 - ddims.width / 2.0,
                    ARENA_H / 2.0 - 10.0,
                    36.0,
                    Color::new(1.0, 0.3, 0.2, 1.0),
                );
                let hint = "Press R to return to lobby";
                let hdims = measure_text(hint, None, 18, 1.0);
                draw_text(
                    hint,
                    ARENA_W / 2.0 - hdims.width / 2.0,
                    ARENA_H / 2.0 + 20.0,
                    18.0,
                    LIGHTGRAY,
                );

                if is_key_pressed(KeyCode::R) {
                    progress = MatchProgress::new();
                    phase = GamePhase::Lobby;
                    build = BuildState::new(progress.round_gold());
                    units.clear();
                    projectiles.clear();
                    net = None;
                    lobby.reset();
                }
            }
        }

        next_frame().await;
    }
}

/// Send BuildComplete message over the network with this round's new packs and tech purchases.
fn send_build_complete(
    net: &mut Option<net::NetState>,
    build: &BuildState,
    _progress: &MatchProgress,
) {
    if let Some(ref mut n) = net {
        // Collect new (unlocked) packs as Vec<(pack_index, center, rotated)>
        let new_packs: Vec<(usize, (f32, f32), bool)> = build
            .placed_packs
            .iter()
            .filter(|p| !p.locked)
            .map(|p| (p.pack_index, (p.center.x, p.center.y), p.rotated))
            .collect();

        let tech_purchases = build.round_tech_purchases.clone();

        n.send(net::NetMessage::BuildComplete {
            new_packs,
            tech_purchases,
            gold_remaining: build.builder.gold_remaining,
        });
    }
}

/// Start battle in single-player AI mode. Generates AI army and transitions to Battle.
fn start_battle_ai(
    _build: &mut BuildState,
    units: &mut Vec<Unit>,
    projectiles: &mut Vec<Projectile>,
    progress: &mut MatchProgress,
) -> GamePhase {
    projectiles.clear();

    // Remove old opponent units (they'll be respawned fresh from stored packs)
    units.retain(|u| u.team_id == 0);

    // Respawn all existing opponent units from previous rounds at full HP
    units.extend(progress.respawn_opponent_units());

    // AI buys techs, then spawns NEW army for this round
    let mut ai_gold = progress.round_allowance();
    ai_buy_techs(&mut ai_gold, &mut progress.opponent_techs);
    let new_opponent_units = progress.spawn_new_ai_army(ai_gold);
    units.extend(new_opponent_units);

    // Seed RNG for this round
    macroquad::rand::srand(progress.round as u64);

    // Reset per-round damage stats
    for unit in units.iter_mut() {
        unit.damage_dealt_round = 0.0;
        unit.damage_soaked_round = 0.0;
    }

    GamePhase::Battle
}

fn draw_hud(progress: &MatchProgress, gold: u32, timer: f32) {
    // Background bar
    draw_rectangle(
        0.0,
        0.0,
        ARENA_W,
        28.0,
        Color::new(0.05, 0.05, 0.08, 0.85),
    );

    let mut x = SHOP_W + 10.0;

    // Round
    let round_text = format!("Round: {}", progress.round);
    draw_text(&round_text, x, 19.0, 18.0, WHITE);
    x += 100.0;

    // Player LP
    let player_lp_text = format!("Player LP: {}", progress.player_lp);
    let plp_color = if progress.player_lp > 500 {
        Color::new(0.3, 1.0, 0.4, 1.0)
    } else if progress.player_lp > 200 {
        Color::new(1.0, 0.8, 0.2, 1.0)
    } else {
        Color::new(1.0, 0.3, 0.2, 1.0)
    };
    draw_text(&player_lp_text, x, 19.0, 18.0, plp_color);
    x += 160.0;

    // Opponent LP
    let opponent_lp_text = format!("Opponent LP: {}", progress.opponent_lp);
    let alp_color = if progress.opponent_lp > 500 {
        Color::new(0.3, 0.6, 1.0, 1.0)
    } else if progress.opponent_lp > 200 {
        Color::new(1.0, 0.8, 0.2, 1.0)
    } else {
        Color::new(1.0, 0.3, 0.2, 1.0)
    };
    draw_text(&opponent_lp_text, x, 19.0, 18.0, alp_color);
    x += 160.0;

    // Gold (only during build)
    if gold > 0 || timer > 0.0 {
        let gold_text = format!("Gold: {}", gold);
        draw_text(
            &gold_text,
            x,
            19.0,
            18.0,
            Color::new(1.0, 0.85, 0.2, 1.0),
        );
        x += 110.0;

        if timer > 0.0 {
            let timer_text = format!("Timer: {:.0}s", timer.ceil());
            draw_text(&timer_text, x, 19.0, 18.0, WHITE);
        }
    }
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

/// Respawn all player units from locked packs at full HP with current techs.
fn respawn_player_units(build: &BuildState, progress: &MatchProgress) -> Vec<Unit> {
    let mut spawned = Vec::new();
    for placed in &build.placed_packs {
        let pack = &all_packs()[placed.pack_index];
        let stats = pack.kind.stats();
        let grid_gap = stats.size * 2.5;
        let eff_rows = placed.effective_rows(pack);
        let eff_cols = placed.effective_cols(pack);
        let grid_w = (eff_cols as f32 - 1.0) * grid_gap;
        let grid_h = (eff_rows as f32 - 1.0) * grid_gap;
        let start_x = placed.center.x - grid_w / 2.0;
        let start_y = placed.center.y - grid_h / 2.0;

        let mut idx = 0;
        for row in 0..eff_rows {
            for col in 0..eff_cols {
                if idx < placed.unit_ids.len() {
                    let uid = placed.unit_ids[idx];
                    let x = start_x + col as f32 * grid_gap;
                    let y = start_y + row as f32 * grid_gap;
                    let mut unit = Unit::new(uid, pack.kind, vec2(x, y), 0);
                    progress.player_techs.apply_to_stats(pack.kind, &mut unit.stats);
                    unit.hp = unit.stats.max_hp;
                    if unit.kind == UnitKind::Scout
                        && progress
                            .player_techs
                            .has_tech(UnitKind::Scout, tech::TechId::ScoutEvasion)
                    {
                        unit.evasion_chance = 0.25;
                    }
                    spawned.push(unit);
                }
                idx += 1;
            }
        }
    }
    spawned
}

/// Refresh all units of a given kind to have updated tech-modified stats.
fn refresh_units_of_kind(units: &mut [Unit], kind: UnitKind, tech_state: &TechState) {
    for unit in units.iter_mut() {
        if unit.kind != kind || !unit.alive {
            continue;
        }
        let hp_frac = unit.hp / unit.stats.max_hp;
        // Reset to base stats, then re-apply techs
        unit.stats = kind.stats();
        tech_state.apply_to_stats(kind, &mut unit.stats);
        unit.hp = unit.stats.max_hp * hp_frac;
        // Apply evasion
        if kind == UnitKind::Scout
            && tech_state.has_tech(UnitKind::Scout, tech::TechId::ScoutEvasion)
        {
            unit.evasion_chance = 0.25;
        }
    }
}
