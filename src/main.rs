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
    let mut game_settings = settings::GameSettings::default();
    let mut obstacles: Vec<terrain::Obstacle> = Vec::new();
    let mut show_surrender_confirm = false;
    let mut chat_messages: Vec<(String, String, u8, f32)> = Vec::new(); // (name, text, team_id, lifetime)
    let mut chat_input = String::new();
    let mut chat_open = false;
    let mut show_grid = false;
    let mut camera_zoom: f32 = 1.0;
    let mut camera_target = vec2(ARENA_W / 2.0, ARENA_H / 2.0);

    loop {
        let dt = get_frame_time().min(0.05);
        let mouse = vec2(mouse_position().0, mouse_position().1);
        let left_click = is_mouse_button_pressed(MouseButton::Left);
        let right_click = is_mouse_button_pressed(MouseButton::Right);
        let middle_click = is_mouse_button_pressed(MouseButton::Middle);
        team::set_player_color(game_settings.player_color_index);

        match &mut phase {
            GamePhase::Lobby => {
                match lobby.update(&mut game_settings) {
                    lobby::LobbyResult::StartMultiplayer => {
                        net = lobby.net.take();
                        if game_settings.draft_ban_enabled {
                            phase = GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None };
                        } else {
                            phase = GamePhase::Build;
                        }
                        continue;
                    }
                    lobby::LobbyResult::StartVsAi => {
                        net = None;
                        if game_settings.draft_ban_enabled {
                            phase = GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None };
                        } else {
                            phase = GamePhase::Build;
                        }
                        continue;
                    }
                    lobby::LobbyResult::Waiting => {}
                }

                match lobby.draw(&mut game_settings) {
                    lobby::LobbyResult::StartVsAi => {
                        net = None;
                        if game_settings.draft_ban_enabled {
                            phase = GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None };
                        } else {
                            phase = GamePhase::Build;
                        }
                        continue;
                    }
                    _ => {}
                }

                next_frame().await;
                continue;
            }

            GamePhase::DraftBan { ref mut bans, ref mut confirmed, ref mut opponent_bans } => {
                // Draft/Ban phase: each player bans up to 2 unit types
                use crate::unit::UnitKind;
                let all_kinds = [
                    UnitKind::Striker, UnitKind::Sentinel, UnitKind::Ranger, UnitKind::Scout,
                    UnitKind::Bruiser, UnitKind::Artillery, UnitKind::Chaff, UnitKind::Sniper,
                    UnitKind::Skirmisher, UnitKind::Dragoon, UnitKind::Berserker,
                    UnitKind::Shield, UnitKind::Interceptor,
                ];

                // Draw background
                clear_background(Color::new(0.08, 0.08, 0.12, 1.0));
                draw_rectangle_lines(0.0, 0.0, ARENA_W, ARENA_H, 2.0, GRAY);

                // Title
                let title = "Ban Phase — Select up to 2 unit types to ban";
                let tdims = measure_text(title, None, 24, 1.0);
                draw_text(title, ARENA_W / 2.0 - tdims.width / 2.0, 50.0, 24.0, WHITE);

                // Draw unit cards in a grid (4 cols)
                let cols = 4;
                let card_w = 160.0;
                let card_h = 50.0;
                let gap = 12.0;
                let grid_w = cols as f32 * card_w + (cols - 1) as f32 * gap;
                let start_x = ARENA_W / 2.0 - grid_w / 2.0;
                let start_y = 90.0;

                for (i, kind) in all_kinds.iter().enumerate() {
                    let col = (i % cols) as f32;
                    let row = (i / cols) as f32;
                    let x = start_x + col * (card_w + gap);
                    let y = start_y + row * (card_h + gap);

                    let is_banned = bans.contains(kind);
                    let is_hovered = mouse.x >= x && mouse.x <= x + card_w && mouse.y >= y && mouse.y <= y + card_h;

                    let bg = if is_banned {
                        Color::new(0.6, 0.15, 0.15, 0.9)
                    } else if is_hovered {
                        Color::new(0.2, 0.25, 0.35, 0.9)
                    } else {
                        Color::new(0.12, 0.12, 0.18, 0.9)
                    };

                    draw_rectangle(x, y, card_w, card_h, bg);
                    draw_rectangle_lines(x, y, card_w, card_h, 1.0, if is_banned { RED } else { GRAY });

                    let name = format!("{:?}", kind);
                    let stats = kind.stats();
                    let info = format!("{} HP:{:.0} DMG:{:.0}", name, stats.max_hp, stats.damage);
                    draw_text(&info, x + 8.0, y + 20.0, 14.0, if is_banned { Color::new(1.0, 0.5, 0.5, 1.0) } else { WHITE });

                    if is_banned {
                        let ban_text = "BANNED";
                        let bdims = measure_text(ban_text, None, 16, 1.0);
                        draw_text(ban_text, x + card_w / 2.0 - bdims.width / 2.0, y + 40.0, 16.0, RED);
                    } else {
                        let detail = format!("RNG:{:.0} SPD:{:.0} AS:{:.1}", stats.attack_range, stats.move_speed, stats.attack_speed);
                        draw_text(&detail, x + 8.0, y + 38.0, 12.0, LIGHTGRAY);
                    }

                    // Click to toggle ban
                    if left_click && is_hovered {
                        if is_banned {
                            bans.retain(|k| k != kind);
                        } else if bans.len() < 2 {
                            bans.push(*kind);
                        }
                    }
                }

                // Confirm button
                let btn_w = 200.0;
                let btn_h = 45.0;
                let btn_x = ARENA_W / 2.0 - btn_w / 2.0;
                let btn_y = start_y + 4.0 * (card_h + gap) + 20.0;
                let btn_hover = mouse.x >= btn_x && mouse.x <= btn_x + btn_w && mouse.y >= btn_y && mouse.y <= btn_y + btn_h;
                let btn_color = if btn_hover { Color::new(0.2, 0.6, 0.3, 0.9) } else { Color::new(0.15, 0.45, 0.2, 0.8) };
                draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
                draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, WHITE);
                let confirm_text = format!("Confirm Bans ({}/ 2)", bans.len());
                let cdims = measure_text(&confirm_text, None, 20, 1.0);
                draw_text(&confirm_text, btn_x + btn_w / 2.0 - cdims.width / 2.0, btn_y + btn_h / 2.0 + 6.0, 20.0, WHITE);

                // Poll network for opponent bans
                if let Some(ref mut n) = net {
                    n.poll();
                    if let Some(ob) = n.opponent_bans.take() {
                        let opp: Vec<UnitKind> = ob.iter().filter_map(|&idx| {
                            all_kinds.get(idx as usize).copied()
                        }).collect();
                        *opponent_bans = Some(opp);
                    }
                }

                // Confirm button click: lock in our bans and send to opponent
                if left_click && btn_hover && !*confirmed {
                    *confirmed = true;
                    if let Some(ref mut n) = net {
                        let ban_indices: Vec<u8> = bans.iter().map(|k| {
                            all_kinds.iter().position(|ak| ak == k).unwrap_or(0) as u8
                        }).collect();
                        n.send(net::NetMessage::BanSelection(ban_indices));
                    }
                }

                // Show waiting indicator
                if *confirmed && net.is_some() && opponent_bans.is_none() {
                    let wait_y = btn_y + btn_h + 15.0;
                    let dots = ".".repeat((get_time() * 2.0) as usize % 4);
                    let wait_text = format!("Waiting for opponent bans{}", dots);
                    let wdims = measure_text(&wait_text, None, 16, 1.0);
                    draw_text(&wait_text, ARENA_W / 2.0 - wdims.width / 2.0, wait_y, 16.0, LIGHTGRAY);
                }

                // Transition when ready
                let ready = *confirmed && (net.is_none() || opponent_bans.is_some());
                if ready {
                    let mut all_bans = bans.clone();
                    if let Some(ref ob) = opponent_bans {
                        all_bans.extend(ob.iter());
                    }
                    progress.banned_kinds = all_bans;
                    phase = GamePhase::Build;
                }

                next_frame().await;
                continue;
            }

            GamePhase::Build => {
                // Poll network
                if let Some(ref mut n) = net {
                    n.poll();
                }

                // Grid toggle
                if is_key_pressed(KeyCode::G) {
                    show_grid = !show_grid;
                }

                // Undo (Ctrl+Z)
                if is_key_down(KeyCode::LeftControl) && is_key_pressed(KeyCode::Z) && build.dragging.is_none() {
                    if let Some(entry) = build.undo_history.pop() {
                        match entry {
                            game_state::UndoEntry::Place { placed_index } => {
                                if placed_index < build.placed_packs.len() {
                                    if let Some((_refund, removed_ids)) = build.sell_pack(placed_index) {
                                        units.retain(|u| !removed_ids.contains(&u.id));
                                    }
                                }
                            }
                            game_state::UndoEntry::Move { placed_index, old_center } => {
                                if placed_index < build.placed_packs.len() {
                                    build.placed_packs[placed_index].center = old_center;
                                    build.reposition_pack_units(placed_index, &mut units);
                                }
                            }
                            game_state::UndoEntry::Rotate { placed_index, was_rotated, old_center } => {
                                if placed_index < build.placed_packs.len() {
                                    build.placed_packs[placed_index].rotated = was_rotated;
                                    build.placed_packs[placed_index].center = old_center;
                                    build.reposition_pack_units(placed_index, &mut units);
                                }
                            }
                            game_state::UndoEntry::Tech { kind, tech_id } => {
                                // Refund tech cost
                                let cost = progress.player_techs.effective_cost(kind);
                                // unpurchase first so effective_cost returns the right amount next time
                                progress.player_techs.unpurchase(kind, tech_id);
                                // Refund: cost was (100 + N*100) where N was count before purchase
                                // After unpurchase, effective_cost gives the old cost, so just refund that
                                build.builder.gold_remaining += cost;
                                // Remove from round tech purchases
                                if let Some(pos) = build.round_tech_purchases.iter().rposition(|(k, t)| *k == kind && *t == tech_id) {
                                    build.round_tech_purchases.remove(pos);
                                }
                                // Refresh units to remove tech effect
                                refresh_units_of_kind(&mut units, kind, &progress.player_techs);
                            }
                        }
                    }
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
                            &mut obstacles,
                            &game_settings,
                        );
                        battle_accumulator = 0.0;
                    }
                    continue;
                }

                // Shop interaction (left click in shop area, only when not holding a pack)
                if left_click && mouse.x < SHOP_W && build.dragging.is_none() {
                    if let Some(pack_idx) =
                        shop::draw_shop(build.builder.gold_remaining, mouse, true, &progress.banned_kinds)
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
                let mut click_consumed = false;
                if left_click && build.selected_pack.is_some() {
                    let sel_idx = build.selected_pack.unwrap();
                    let placed = &build.placed_packs[sel_idx];
                    let kind = all_packs()[placed.pack_index].kind;
                    let cs = tech_ui::PackCombatStats::from_units(&units, &placed.unit_ids);

                    // Check if mouse is in the tech panel area (consume click to prevent drag)
                    if mouse.x >= 490.0 && mouse.x <= 700.0 && mouse.y >= 30.0 {
                        click_consumed = true;
                    }

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
                            // Track tech purchase for network sync and undo
                            build.round_tech_purchases.push((kind, tech_id));
                            build.undo_history.push(game_state::UndoEntry::Tech { kind, tech_id });
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
                    // Snap to grid
                    let grid = terrain::GRID_CELL;
                    let snapped = vec2(
                        (clamped.x / grid).round() * grid,
                        (clamped.y / grid).round() * grid,
                    );
                    build.placed_packs[drag_idx].center = snapped;
                    build.reposition_pack_units(drag_idx, &mut units);

                    if left_click {
                        let placed = &build.placed_packs[drag_idx];
                        let pack_index = placed.pack_index;
                        let rotated = placed.rotated;
                        let old_center = placed.pre_drag_center;
                        if build.would_overlap(placed.center, pack_index, Some(drag_idx), rotated) {
                            build.placed_packs[drag_idx].center = old_center;
                            build.reposition_pack_units(drag_idx, &mut units);
                        } else if build.placed_packs[drag_idx].center != old_center {
                            build.undo_history.push(game_state::UndoEntry::Move { placed_index: drag_idx, old_center });
                        }
                        build.dragging = None;
                    }

                    if right_click {
                        let prev = build.placed_packs[drag_idx].pre_drag_center;
                        build.placed_packs[drag_idx].center = prev;
                        build.reposition_pack_units(drag_idx, &mut units);
                        build.dragging = None;
                    }
                } else if left_click && mouse.x > SHOP_W && !click_consumed {
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
                            &mut obstacles,
                            &game_settings,
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

                        // Generate terrain if enabled
                        obstacles.clear();
                        if game_settings.terrain_enabled {
                            obstacles = terrain::generate_terrain(progress.round, game_settings.terrain_destructible);
                        }

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
                // Zoom control (scroll wheel)
                let wheel = mouse_wheel().1;
                if wheel != 0.0 && !show_surrender_confirm {
                    camera_zoom = (camera_zoom + wheel * 0.1).clamp(0.5, 2.0);
                }
                // Pan with middle-click drag
                if is_mouse_button_down(MouseButton::Middle) && !show_surrender_confirm {
                    let delta = vec2(
                        mouse_delta_position().x * -ARENA_W / camera_zoom,
                        mouse_delta_position().y * -ARENA_H / camera_zoom,
                    );
                    camera_target += delta;
                    camera_target.x = camera_target.x.clamp(0.0, ARENA_W);
                    camera_target.y = camera_target.y.clamp(0.0, ARENA_H);
                }

                // Surrender toggle
                if is_key_pressed(KeyCode::Escape) {
                    show_surrender_confirm = !show_surrender_confirm;
                }

                // Poll network
                if let Some(ref mut n) = net {
                    n.poll();
                }

                if show_surrender_confirm {
                    // Battle paused while surrender overlay is shown
                } else if net.is_some() {
                    // Multiplayer: fixed timestep for determinism
                    battle_accumulator += dt;
                    while battle_accumulator >= FIXED_DT {
                        battle_accumulator -= FIXED_DT;
                        update_targeting(&mut units, &obstacles);
                        update_movement(&mut units, FIXED_DT, ARENA_W, ARENA_H, &obstacles);
                        update_attacks(
                            &mut units,
                            &mut projectiles,
                            FIXED_DT,
                            &progress.player_techs,
                            &progress.opponent_techs,
                        );
                        update_projectiles(&mut projectiles, &mut units, FIXED_DT, &mut obstacles);
                        // Death animation timers (inside fixed timestep for determinism)
                        for unit in units.iter_mut() {
                            if !unit.alive && unit.death_timer > 0.0 {
                                unit.death_timer -= FIXED_DT;
                            }
                        }
                    }
                } else {
                    // Single-player: variable timestep (original behavior)
                    update_targeting(&mut units, &obstacles);
                    update_movement(&mut units, dt, ARENA_W, ARENA_H, &obstacles);
                    update_attacks(
                        &mut units,
                        &mut projectiles,
                        dt,
                        &progress.player_techs,
                        &progress.opponent_techs,
                    );
                    update_projectiles(&mut projectiles, &mut units, dt, &mut obstacles);
                    // Death animation timers
                    for unit in units.iter_mut() {
                        if !unit.alive && unit.death_timer > 0.0 {
                            unit.death_timer -= dt;
                        }
                    }
                }

                // Surrender confirmation handling
                if show_surrender_confirm && is_mouse_button_pressed(MouseButton::Left) {
                    let mouse = Vec2::from(mouse_position());
                    let btn_w = 120.0;
                    let btn_h = 40.0;
                    let cx = ARENA_W / 2.0;
                    let cy = ARENA_H / 2.0;
                    // "Yes" button
                    let yes_x = cx - btn_w - 10.0;
                    let yes_y = cy + 10.0;
                    if mouse.x >= yes_x && mouse.x <= yes_x + btn_w && mouse.y >= yes_y && mouse.y <= yes_y + btn_h {
                        progress.player_lp = 0;
                        show_surrender_confirm = false;
                        phase = GamePhase::GameOver(1);
                    }
                    // "Cancel" button
                    let no_x = cx + 10.0;
                    let no_y = cy + 10.0;
                    if mouse.x >= no_x && mouse.x <= no_x + btn_w && mouse.y >= no_y && mouse.y <= no_y + btn_h {
                        show_surrender_confirm = false;
                    }
                }

                let state = check_match_state(&units);
                if state != MatchState::InProgress && projectiles.is_empty() {
                    let final_state = check_match_state(&units);

                    // Record AI memory for counter-picking
                    let ai_won = match &final_state {
                        MatchState::Winner(w) => *w == 1,
                        _ => false,
                    };
                    progress.ai_memory.record_round(&units, ai_won);

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

                    show_surrender_confirm = false;
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

                // Rematch button click
                let rmatch_w = 160.0;
                let rmatch_h = 40.0;
                let rmatch_x = ARENA_W / 2.0 - rmatch_w / 2.0;
                let rmatch_y = ARENA_H / 2.0 + 65.0;
                if left_click && mouse.x >= rmatch_x && mouse.x <= rmatch_x + rmatch_w
                    && mouse.y >= rmatch_y && mouse.y <= rmatch_y + rmatch_h
                {
                    // Reset for rematch (skip lobby, go straight to Build)
                    progress = MatchProgress::new();
                    build = BuildState::new(progress.round_gold());
                    units.clear();
                    projectiles.clear();
                    obstacles.clear();
                    phase = if game_settings.draft_ban_enabled {
                        GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None }
                    } else {
                        GamePhase::Build
                    };
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

        // Apply camera zoom during Battle
        if matches!(phase, GamePhase::Battle) && (camera_zoom - 1.0).abs() > 0.01 {
            set_camera(&Camera2D {
                target: camera_target,
                zoom: vec2(camera_zoom * 2.0 / screen_width(), camera_zoom * 2.0 / screen_height()),
                ..Default::default()
            });
        } else {
            // Reset camera for non-battle or default zoom
            if !matches!(phase, GamePhase::Battle) {
                camera_zoom = 1.0;
                camera_target = vec2(ARENA_W / 2.0, ARENA_H / 2.0);
            }
            set_default_camera();
        }

        draw_rectangle_lines(0.0, 0.0, ARENA_W, ARENA_H, 2.0, GRAY);
        draw_center_divider();
        terrain::draw_obstacles(&obstacles);

        // Grid overlay during Build phase
        if show_grid && matches!(phase, GamePhase::Build) {
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

        // Draw units (including death animations)
        for unit in &units {
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

        // Reset camera for UI overlays (screen-space)
        set_default_camera();

        // === Phase-specific UI ===
        match &phase {
            GamePhase::Lobby | GamePhase::DraftBan { .. } => {
                // Handled above with early continue
            }

            GamePhase::Build => {
                shop::draw_shop(build.builder.gold_remaining, mouse, false, &progress.banned_kinds);

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
                let army_value: u32 = {
                    let packs = all_packs();
                    build.placed_packs.iter().map(|p| packs[p.pack_index].cost).sum()
                };
                draw_hud(&progress, build.builder.gold_remaining, build.timer, army_value);

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
                    "Click to select | Double-click to move | Middle-click rotate | Right-click sell | G: Grid | Ctrl+Z: Undo",
                    SHOP_W + 10.0,
                    ARENA_H - 10.0,
                    13.0,
                    Color::new(0.5, 0.5, 0.5, 0.7),
                );
            }

            GamePhase::WaitingForOpponent => {
                draw_hud(&progress, build.builder.gold_remaining, 0.0, 0);

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
                draw_hud(&progress, 0, 0.0, 0);
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

                // Obstacle tooltip on hover
                if !show_surrender_confirm {
                    for obs in &obstacles {
                        if !obs.alive { continue; }
                        if obs.contains_point(mouse) {
                            let tip_x = mouse.x + 15.0;
                            let tip_y = (mouse.y - 10.0).max(5.0);
                            let tip_w = 170.0;
                            let tip_h = if obs.obstacle_type == terrain::ObstacleType::Cover { 60.0 } else { 45.0 };

                            draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.08, 0.08, 0.12, 0.95));
                            draw_rectangle_lines(tip_x, tip_y, tip_w, tip_h, 1.0, Color::new(0.4, 0.5, 0.6, 0.7));

                            let type_name = match obs.obstacle_type {
                                terrain::ObstacleType::Wall => "Wall (Indestructible)",
                                terrain::ObstacleType::Cover => "Cover (Destructible)",
                            };
                            draw_text(type_name, tip_x + 6.0, tip_y + 16.0, 14.0, WHITE);

                            let mut ty = tip_y + 32.0;
                            if obs.obstacle_type == terrain::ObstacleType::Cover {
                                draw_text(&format!("HP: {:.0}/{:.0}", obs.hp, obs.max_hp), tip_x + 6.0, ty, 12.0, LIGHTGRAY);
                                ty += 14.0;
                            }
                            let team_name = match obs.team_id { 0 => "Player", 1 => "Opponent", _ => "Neutral" };
                            draw_text(&format!("Owner: {}", team_name), tip_x + 6.0, ty, 12.0, LIGHTGRAY);
                            break;
                        }
                    }
                }

                // Surrender confirmation overlay
                if show_surrender_confirm {
                    draw_rectangle(0.0, 0.0, ARENA_W, ARENA_H, Color::new(0.0, 0.0, 0.0, 0.6));
                    let title = "Surrender?";
                    let tdims = measure_text(title, None, 36, 1.0);
                    draw_text(title, ARENA_W / 2.0 - tdims.width / 2.0, ARENA_H / 2.0 - 20.0, 36.0, WHITE);

                    let btn_w: f32 = 120.0;
                    let btn_h: f32 = 40.0;
                    let cx = ARENA_W / 2.0;
                    let cy = ARENA_H / 2.0;
                    let mouse = Vec2::from(mouse_position());

                    // Yes button
                    let yes_x = cx - btn_w - 10.0;
                    let yes_y = cy + 10.0;
                    let yes_hover = mouse.x >= yes_x && mouse.x <= yes_x + btn_w && mouse.y >= yes_y && mouse.y <= yes_y + btn_h;
                    let yes_color = if yes_hover { Color::new(0.8, 0.2, 0.2, 0.9) } else { Color::new(0.6, 0.15, 0.15, 0.8) };
                    draw_rectangle(yes_x, yes_y, btn_w, btn_h, yes_color);
                    draw_rectangle_lines(yes_x, yes_y, btn_w, btn_h, 1.0, WHITE);
                    let yt = "Yes";
                    let ydims = measure_text(yt, None, 20, 1.0);
                    draw_text(yt, yes_x + btn_w / 2.0 - ydims.width / 2.0, yes_y + btn_h / 2.0 + 6.0, 20.0, WHITE);

                    // Cancel button
                    let no_x = cx + 10.0;
                    let no_y = cy + 10.0;
                    let no_hover = mouse.x >= no_x && mouse.x <= no_x + btn_w && mouse.y >= no_y && mouse.y <= no_y + btn_h;
                    let no_color = if no_hover { Color::new(0.3, 0.3, 0.35, 0.9) } else { Color::new(0.2, 0.2, 0.25, 0.8) };
                    draw_rectangle(no_x, no_y, btn_w, btn_h, no_color);
                    draw_rectangle_lines(no_x, no_y, btn_w, btn_h, 1.0, WHITE);
                    let nt = "Cancel";
                    let ndims = measure_text(nt, None, 20, 1.0);
                    draw_text(nt, no_x + btn_w / 2.0 - ndims.width / 2.0, no_y + btn_h / 2.0 + 6.0, 20.0, WHITE);
                }
            }

            GamePhase::RoundResult {
                match_state,
                lp_damage,
                loser_team,
            } => {
                draw_hud(&progress, 0, 0.0, 0);

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

                // Stats panel
                let panel_w = 320.0;
                let panel_h = 140.0;
                let panel_x = ARENA_W / 2.0 - panel_w / 2.0;
                let panel_y = ARENA_H / 2.0 + 10.0;
                draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.08, 0.08, 0.12, 0.9));
                draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, Color::new(0.4, 0.5, 0.6, 0.7));

                let mut sy = panel_y + 18.0;
                let sx = panel_x + 12.0;

                let round_text = format!("Rounds Played: {}", progress.round);
                draw_text(&round_text, sx, sy, 15.0, LIGHTGRAY);
                sy += 18.0;

                // MVP
                let mvp = units.iter()
                    .filter(|u| u.team_id == 0)
                    .max_by(|a, b| a.damage_dealt_total.partial_cmp(&b.damage_dealt_total).unwrap_or(std::cmp::Ordering::Equal));
                if let Some(mvp_unit) = mvp {
                    let mvp_text = format!("MVP: {:?} - {:.0} dmg, {} kills", mvp_unit.kind, mvp_unit.damage_dealt_total, mvp_unit.kills_total);
                    draw_text(&mvp_text, sx, sy, 15.0, Color::new(1.0, 0.85, 0.2, 1.0));
                }
                sy += 18.0;

                let total_dmg: f32 = units.iter()
                    .filter(|u| u.team_id == 0)
                    .map(|u| u.damage_dealt_total)
                    .sum();
                draw_text(&format!("Total Damage: {:.0}", total_dmg), sx, sy, 15.0, LIGHTGRAY);
                sy += 18.0;

                let surviving = units.iter().filter(|u| u.team_id == 0 && u.alive).count();
                let total_units = units.iter().filter(|u| u.team_id == 0).count();
                draw_text(&format!("Surviving: {} / {}", surviving, total_units), sx, sy, 15.0, LIGHTGRAY);
                sy += 18.0;

                draw_text(&format!("LP: {} vs {}", progress.player_lp, progress.opponent_lp), sx, sy, 15.0, LIGHTGRAY);

                let below_panel = panel_y + panel_h + 8.0;
                draw_text(
                    "Press R to return to lobby",
                    ARENA_W / 2.0 - 100.0,
                    below_panel,
                    16.0,
                    DARKGRAY,
                );

                // Rematch button
                let rmatch_w = 160.0;
                let rmatch_h = 40.0;
                let rmatch_x = ARENA_W / 2.0 - rmatch_w / 2.0;
                let rmatch_y = below_panel + 15.0;
                let rmatch_hover = mouse.x >= rmatch_x && mouse.x <= rmatch_x + rmatch_w && mouse.y >= rmatch_y && mouse.y <= rmatch_y + rmatch_h;
                let rmatch_bg = if rmatch_hover { Color::new(0.2, 0.5, 0.3, 0.9) } else { Color::new(0.15, 0.35, 0.2, 0.8) };
                draw_rectangle(rmatch_x, rmatch_y, rmatch_w, rmatch_h, rmatch_bg);
                draw_rectangle_lines(rmatch_x, rmatch_y, rmatch_w, rmatch_h, 2.0, Color::new(0.3, 0.8, 0.4, 1.0));
                let rt = "Rematch";
                let rdims2 = measure_text(rt, None, 22, 1.0);
                draw_text(rt, rmatch_x + rmatch_w / 2.0 - rdims2.width / 2.0, rmatch_y + rmatch_h / 2.0 + 7.0, 22.0, WHITE);
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

        // === Chat System ===
        let player_name = lobby.player_name.clone();

        // Receive chat messages from network
        if let Some(ref mut n) = net {
            for msg in n.received_chats.drain(..) {
                // Opponent messages come as "name: text"
                chat_messages.push(("Opponent".to_string(), msg, 1, 5.0));
            }
        }

        // Chat input (available in Build, Battle, RoundResult)
        let chat_allowed = matches!(phase, GamePhase::Build | GamePhase::Battle | GamePhase::RoundResult { .. });
        if chat_allowed {
            if is_key_pressed(KeyCode::Enter) {
                if chat_open {
                    // Send message
                    if !chat_input.is_empty() {
                        let text = if chat_input.len() > 100 { chat_input[..100].to_string() } else { chat_input.clone() };
                        chat_messages.push((player_name.clone(), text.clone(), 0, 5.0));
                        if let Some(ref mut n) = net {
                            n.send(net::NetMessage::ChatMessage(text));
                        }
                    }
                    chat_input.clear();
                    chat_open = false;
                } else {
                    chat_open = true;
                }
            }
            if chat_open {
                if is_key_pressed(KeyCode::Escape) {
                    chat_open = false;
                    chat_input.clear();
                }
                while let Some(ch) = get_char_pressed() {
                    if ch == '\r' || ch == '\n' { continue; }
                    if ch == '\u{8}' { // backspace
                        chat_input.pop();
                    } else if chat_input.len() < 100 && (ch.is_ascii_graphic() || ch == ' ') {
                        chat_input.push(ch);
                    }
                }
            }
        }

        // Update chat lifetimes
        for (_, _, _, lifetime) in chat_messages.iter_mut() {
            *lifetime -= dt;
        }
        chat_messages.retain(|(_, _, _, lt)| *lt > 0.0);

        // Render chat messages (floating at top of arena)
        let chat_x = ARENA_W / 2.0;
        let mut chat_y = 45.0;
        for (name, text, team_id, lifetime) in chat_messages.iter().rev().take(5).collect::<Vec<_>>().into_iter().rev() {
            let alpha = (*lifetime / 5.0).min(1.0);
            let color = if *team_id == 0 {
                team::team_color(0)
            } else {
                team::team_color(1)
            };
            let display_color = Color::new(color.r, color.g, color.b, alpha);

            // Check for emotes
            let is_emote = text.starts_with('/');
            let display_text = match text.as_str() {
                "/gg" => "GG".to_string(),
                "/gl" => "Good Luck!".to_string(),
                "/nice" => "Nice!".to_string(),
                "/wow" => "Wow!".to_string(),
                _ => text.clone(),
            };
            let full_display = if is_emote {
                format!("{}: {}", name, display_text)
            } else {
                format!("{}: {}", name, display_text)
            };
            let font_size = if is_emote { 20.0 } else { 15.0 };
            let dims = measure_text(&full_display, None, font_size as u16, 1.0);
            draw_text(&full_display, chat_x - dims.width / 2.0, chat_y, font_size, display_color);
            chat_y += font_size + 4.0;
        }

        // Render chat input box
        if chat_open {
            let input_y = ARENA_H - 45.0;
            let input_w = 450.0;
            let input_x = ARENA_W / 2.0 - input_w / 2.0;
            let input_h = 30.0;
            draw_rectangle(input_x, input_y, input_w, input_h, Color::new(0.05, 0.05, 0.1, 0.92));
            draw_rectangle_lines(input_x, input_y, input_w, input_h, 1.5, Color::new(0.4, 0.5, 0.6, 0.9));
            let name_prefix = format!("{}: ", player_name);
            let name_w = measure_text(&name_prefix, None, 15, 1.0).width;
            draw_text(&name_prefix, input_x + 8.0, input_y + 20.0, 15.0, Color::new(0.6, 0.8, 1.0, 0.9));
            let cursor = if (get_time() * 2.0) as u32 % 2 == 0 { "|" } else { "" };
            draw_text(&format!("{}{}", chat_input, cursor), input_x + 8.0 + name_w, input_y + 20.0, 15.0, WHITE);
        } else if chat_allowed {
            draw_text("Enter: Chat", ARENA_W - 100.0, ARENA_H - 5.0, 12.0, Color::new(0.4, 0.4, 0.4, 0.6));
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
    obstacles: &mut Vec<terrain::Obstacle>,
    game_settings: &settings::GameSettings,
) -> GamePhase {
    projectiles.clear();

    // Generate terrain if enabled
    obstacles.clear();
    if game_settings.terrain_enabled {
        *obstacles = terrain::generate_terrain(progress.round, game_settings.terrain_destructible);
    }

    // Remove old opponent units (they'll be respawned fresh from stored packs)
    units.retain(|u| u.team_id == 0);

    // Respawn all existing opponent units from previous rounds at full HP
    units.extend(progress.respawn_opponent_units());

    // AI buys techs, then spawns NEW army for this round
    let mut ai_gold = progress.round_allowance();
    ai_buy_techs(&mut ai_gold, &mut progress.opponent_techs);
    let ai_builder = if game_settings.smart_ai {
        economy::smart_army(ai_gold, &progress.ai_memory, &progress.banned_kinds)
    } else {
        economy::random_army_filtered(ai_gold, &progress.banned_kinds)
    };
    let new_opponent_units = progress.spawn_ai_army_from_builder(&ai_builder);
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

fn draw_hud(progress: &MatchProgress, gold: u32, timer: f32, army_value: u32) {
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

        if army_value > 0 {
            let army_text = format!("Army: {}g", army_value);
            draw_text(&army_text, x, 19.0, 16.0, Color::new(0.7, 0.7, 0.75, 0.8));
            x += 100.0;
        }

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
mod settings; mod terrain;
