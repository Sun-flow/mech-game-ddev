mod arena;
mod chat;
mod combat;
mod economy;
mod game_state;
mod lobby;
pub mod ui;
mod match_progress;
mod net;
mod pack;
mod projectile;
mod rendering;
mod settings;
mod shop;
mod team;
mod tech;
mod tech_ui;
mod terrain;
mod unit;
mod sync;

use macroquad::prelude::*;

use arena::{check_match_state, MatchState, ARENA_H, ARENA_W, HALF_W, shop_w};
use combat::{update_attacks, update_movement, update_projectiles, update_targeting};
use game_state::{BuildState, GamePhase};
use match_progress::MatchProgress;
use pack::all_packs;
use projectile::Projectile;
use rendering::SplashEffect;
use team::team_color;
use unit::{Unit, UnitKind};

const FIXED_DT: f32 = 1.0 / 60.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "RTS Unit Arena".to_string(),
        window_width: ARENA_W as i32,
        window_height: ARENA_H as i32,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut progress = MatchProgress::new(true);
    let mut phase = GamePhase::Lobby;
    let mut build = BuildState::new(progress.round_gold(), true);
    let mut units: Vec<Unit> = Vec::new();
    let mut projectiles: Vec<Projectile> = Vec::new();
    let mut net: Option<net::NetState> = None;
    let mut lobby = lobby::LobbyState::new();
    let mut battle_accumulator: f32 = 0.0;
    let mut battle_timer: f32 = 0.0;
    let mut battle_frame: u32 = 0;
    const ROUND_TIMEOUT: f32 = 90.0;
    const SYNC_INTERVAL: u32 = 4;
    // Guest keeps recent frame hashes so it can match against the host's frame
    let mut recent_hashes: std::collections::VecDeque<(u32, u64)> = std::collections::VecDeque::with_capacity(5);
    let mut game_settings = settings::GameSettings::default();
    let mut main_settings = settings::MainSettings::default();
    let mut obstacles: Vec<terrain::Obstacle> = Vec::new();
    let mut show_surrender_confirm = false;
    let mut mp_player_name = String::from("Player");
    let mut mp_opponent_name = String::from("Opponent");
    let mut chat = chat::ChatState::new();
    let mut show_grid = false;
    let mut nav_grid: Option<terrain::NavGrid> = None;
    let mut camera_zoom: f32 = 1.0;
    let mut camera_target = vec2(ARENA_W / 2.0, ARENA_H / 2.0);
    let mut is_fullscreen_mode = false;
    let mut splash_effects: Vec<SplashEffect> = Vec::new();
    let mut pan_grab_world: Option<Vec2> = None; // world point pinned to cursor during drag
    let mut waiting_for_round_end = false;
    let mut round_end_timeout: f32 = 0.0;

    loop {
        let dt = get_frame_time().min(0.05);
        let screen_mouse = vec2(mouse_position().0, mouse_position().1);
        let left_click = is_mouse_button_pressed(MouseButton::Left);
        let right_click = is_mouse_button_pressed(MouseButton::Right);
        let middle_click = is_mouse_button_pressed(MouseButton::Middle);
        team::set_player_color(game_settings.player_color_index);
        ui::set_text_scale(main_settings.ui_scale);
        // Apply opponent color if received
        if let Some(ref n) = net {
            if let Some(opp_color) = n.opponent_color {
                team::set_opponent_color(opp_color);
            }
        }

        // Build the arena camera (used for all world-space rendering)
        let arena_camera = Camera2D {
            target: camera_target,
            zoom: vec2(camera_zoom * 2.0 / screen_width(), camera_zoom * 2.0 / screen_height()),
            ..Default::default()
        };
        let world_mouse = arena_camera.screen_to_world(screen_mouse);
        // For UI elements that need screen coords, use screen_mouse
        let mouse = if matches!(phase, GamePhase::Lobby) { screen_mouse } else { world_mouse };

        // Fullscreen toggle (F11)
        if is_key_pressed(KeyCode::F11) {
            is_fullscreen_mode = !is_fullscreen_mode;
            set_fullscreen(is_fullscreen_mode);
        }

        // Camera zoom/pan (available in all non-lobby phases)
        if !matches!(phase, GamePhase::Lobby) {
            // Smooth multiplicative zoom — ~100+ steps between min/max
            let wheel = mouse_wheel().1;
            if wheel != 0.0 {
                let zoom_factor = 1.0 + wheel.signum() * 0.04; // ~4% per scroll tick
                camera_zoom = (camera_zoom * zoom_factor).clamp(0.3, 5.0);
            }
            // "Grab the ground" pan: pin a world point to the cursor
            if is_mouse_button_down(MouseButton::Middle) {
                if pan_grab_world.is_none() {
                    // On drag start, record the world point under the cursor
                    pan_grab_world = Some(arena_camera.screen_to_world(screen_mouse));
                }
                if let Some(grab_pt) = pan_grab_world {
                    // Where is the cursor pointing now in world coords?
                    let current_world = arena_camera.screen_to_world(screen_mouse);
                    // Adjust camera so the grabbed point stays under the cursor
                    camera_target += grab_pt - current_world;
                }
            } else {
                pan_grab_world = None;
            }
            // Clamp camera to 140% of arena (20% margin on each side)
            let margin_x = ARENA_W * 0.2;
            let margin_y = ARENA_H * 0.2;
            camera_target.x = camera_target.x.clamp(-margin_x, ARENA_W + margin_x);
            camera_target.y = camera_target.y.clamp(-margin_y, ARENA_H + margin_y);
        }

        match &mut phase {
            GamePhase::Lobby => {
                match lobby.update(&mut game_settings, &mut main_settings) {
                    lobby::LobbyResult::StartMultiplayer => {
                        let is_host = lobby.is_room_creator;
                        net = lobby.net.take();
                        if let Some(ref mut n) = net {
                            n.is_host = is_host;
                            mp_opponent_name = n.opponent_name.clone().unwrap_or_else(|| "Opponent".to_string());
                        }
                        mp_player_name = lobby.player_name.clone();
                        progress = MatchProgress::new(is_host);
                        build = BuildState::new(progress.round_gold(), is_host);
                        if game_settings.draft_ban_enabled {
                            phase = GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None };
                        } else {
                            phase = GamePhase::Build;
                        }
                        continue;
                    }
                    lobby::LobbyResult::StartVsAi => {
                        net = None;
                        mp_player_name = lobby.player_name.clone();
                        mp_opponent_name = "AI".to_string();
                        progress = MatchProgress::new(true);
                        build = BuildState::new(progress.round_gold(), true);
                        if game_settings.draft_ban_enabled {
                            phase = GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None };
                        } else {
                            phase = GamePhase::Build;
                        }
                        continue;
                    }
                    lobby::LobbyResult::Waiting => {}
                }

                match lobby.draw(&mut game_settings, &mut main_settings) {
                    lobby::LobbyResult::StartMultiplayer => {
                        let is_host = lobby.is_room_creator;
                        net = lobby.net.take();
                        if let Some(ref mut n) = net {
                            n.is_host = is_host;
                            mp_opponent_name = n.opponent_name.clone().unwrap_or_else(|| "Opponent".to_string());
                        }
                        mp_player_name = lobby.player_name.clone();
                        progress = MatchProgress::new(is_host);
                        build = BuildState::new(progress.round_gold(), is_host);
                        if game_settings.draft_ban_enabled {
                            phase = GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None };
                        } else {
                            phase = GamePhase::Build;
                        }
                        continue;
                    }
                    lobby::LobbyResult::StartVsAi => {
                        net = None;
                        mp_player_name = lobby.player_name.clone();
                        mp_opponent_name = "AI".to_string();
                        progress = MatchProgress::new(true);
                        build = BuildState::new(progress.round_gold(), true);
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

                // Title
                let title = "Ban Phase — Select up to 2 unit types to ban";
                let tdims = crate::ui::measure_scaled_text(title, 24);
                crate::ui::draw_scaled_text(title, screen_width() / 2.0 - tdims.width / 2.0, crate::ui::s(50.0), 24.0, WHITE);

                // Draw unit cards in a grid (4 cols)
                let cols = 4;
                let card_w = crate::ui::s(160.0);
                let card_h = crate::ui::s(50.0);
                let gap = crate::ui::s(12.0);
                let grid_w = cols as f32 * card_w + (cols - 1) as f32 * gap;
                let start_x = screen_width() / 2.0 - grid_w / 2.0;
                let start_y = crate::ui::s(90.0);

                for (i, kind) in all_kinds.iter().enumerate() {
                    let col = (i % cols) as f32;
                    let row = (i / cols) as f32;
                    let x = start_x + col * (card_w + gap);
                    let y = start_y + row * (card_h + gap);

                    let is_banned = bans.contains(kind);
                    let is_hovered = screen_mouse.x >= x && screen_mouse.x <= x + card_w && screen_mouse.y >= y && screen_mouse.y <= y + card_h;

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
                    crate::ui::draw_scaled_text(&info, x + crate::ui::s(8.0), y + crate::ui::s(20.0), 14.0, if is_banned { Color::new(1.0, 0.5, 0.5, 1.0) } else { WHITE });

                    if is_banned {
                        let ban_text = "BANNED";
                        let bdims = crate::ui::measure_scaled_text(ban_text, 16);
                        crate::ui::draw_scaled_text(ban_text, x + card_w / 2.0 - bdims.width / 2.0, y + crate::ui::s(40.0), 16.0, RED);
                    } else {
                        let detail = format!("RNG:{:.0} SPD:{:.0} AS:{:.1}", stats.attack_range, stats.move_speed, stats.attack_speed);
                        crate::ui::draw_scaled_text(&detail, x + crate::ui::s(8.0), y + crate::ui::s(38.0), 12.0, LIGHTGRAY);
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
                let btn_w = crate::ui::s(200.0);
                let btn_h = crate::ui::s(45.0);
                let btn_x = screen_width() / 2.0 - btn_w / 2.0;
                let btn_y = start_y + 4.0 * (card_h + gap) + crate::ui::s(20.0);
                let btn_hover = screen_mouse.x >= btn_x && screen_mouse.x <= btn_x + btn_w && screen_mouse.y >= btn_y && screen_mouse.y <= btn_y + btn_h;
                let btn_color = if btn_hover { Color::new(0.2, 0.6, 0.3, 0.9) } else { Color::new(0.15, 0.45, 0.2, 0.8) };
                draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
                draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, WHITE);
                let confirm_text = format!("Confirm Bans ({}/ 2)", bans.len());
                let cdims = crate::ui::measure_scaled_text(&confirm_text, 20);
                crate::ui::draw_scaled_text(&confirm_text, btn_x + btn_w / 2.0 - cdims.width / 2.0, btn_y + btn_h / 2.0 + 6.0, 20.0, WHITE);

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
                    let wait_y = btn_y + btn_h + crate::ui::s(15.0);
                    let dots = ".".repeat((get_time() * 2.0) as usize % 4);
                    let wait_text = format!("Waiting for opponent bans{}", dots);
                    let wdims = crate::ui::measure_scaled_text(&wait_text, 16);
                    crate::ui::draw_scaled_text(&wait_text, screen_width() / 2.0 - wdims.width / 2.0, wait_y, 16.0, LIGHTGRAY);
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
                            game_state::UndoEntry::MultiMove { indices, old_centers } => {
                                for (i, &idx) in indices.iter().enumerate() {
                                    if idx < build.placed_packs.len() {
                                        build.placed_packs[idx].center = old_centers[i];
                                        build.reposition_pack_units(idx, &mut units);
                                    }
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
                                tech::refresh_units_of_kind(&mut units, kind, &progress.player_techs);
                            }
                        }
                    }
                }

                // Timer countdown
                build.timer -= dt;
                if build.timer <= 0.0 {
                    if net.is_some() {
                        // Multiplayer: send build, transition to waiting
                        net::send_build_complete(&mut net, &build);
                        phase = GamePhase::WaitingForOpponent;
                    } else {
                        // Single-player: start battle immediately with AI
                        phase = economy::start_ai_battle(
                            &mut build,
                            &mut units,
                            &mut projectiles,
                            &mut progress,
                            &mut obstacles,
                            &mut nav_grid,
                            &game_settings,
                        );
                        battle_accumulator = 0.0;
                        battle_timer = 0.0;
                        battle_frame = 0;
                        recent_hashes.clear();
                    }
                    continue;
                }

                // Shop interaction (left click in shop area, only when not holding a pack)
                if left_click && screen_mouse.x < shop_w() && build.dragging.is_none() {
                    if let Some(pack_idx) =
                        shop::draw_shop(build.builder.gold_remaining, screen_mouse, true, &progress.banned_kinds, game_state::BUILD_LIMIT - build.packs_bought_this_round)
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
                    // Compute actual panel height to avoid blocking clicks in the entire column
                    let available_count = progress.player_techs.available_techs(kind).len();
                    let purchased_count = progress.player_techs.tech_count(kind);
                    let has_combat = cs.damage_dealt_total > 0.0 || cs.damage_soaked_total > 0.0;
                    let combat_extra = if has_combat { 5.0 * 15.0 + 30.0 } else { 0.0 };
                    let panel_h = crate::ui::s(120.0) + (available_count + purchased_count) as f32 * crate::ui::s(35.0) + crate::ui::s(combat_extra) + crate::ui::s(20.0);
                    if screen_mouse.x >= crate::ui::s(490.0) && screen_mouse.x <= crate::ui::s(700.0)
                        && screen_mouse.y >= crate::ui::s(30.0) && screen_mouse.y <= crate::ui::s(30.0) + panel_h
                    {
                        click_consumed = true;
                    }

                    if let Some(tech_id) = tech_ui::draw_tech_panel(
                        kind,
                        &progress.player_techs,
                        build.builder.gold_remaining,
                        screen_mouse,
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
                            tech::refresh_units_of_kind(&mut units, kind, &progress.player_techs);
                        }
                    }
                }

                // Right-click: sell if on unlocked pack, otherwise deselect
                if right_click && screen_mouse.x > shop_w() && build.dragging.is_none() {
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
                if middle_click && screen_mouse.x > shop_w() {
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

                // === Multi-drag system ===
                let left_released = is_mouse_button_released(MouseButton::Left);

                // Active multi-drag: move all packs together
                if !build.multi_dragging.is_empty() {
                    let grid = terrain::GRID_CELL;
                    let snapped_mouse = vec2(
                        (mouse.x / grid).round() * grid,
                        (mouse.y / grid).round() * grid,
                    );
                    for (i, &pack_idx) in build.multi_dragging.clone().iter().enumerate() {
                        let offset = build.multi_drag_offsets[i];
                        let new_center = snapped_mouse + offset;
                        let pack = &all_packs()[build.placed_packs[pack_idx].pack_index];
                        let half = build.placed_packs[pack_idx].bbox_half_size_for(pack);
                        let clamped = vec2(
                            new_center.x.clamp(half.x, HALF_W - half.x),
                            new_center.y.clamp(half.y, ARENA_H - half.y),
                        );
                        build.placed_packs[pack_idx].center = clamped;
                        build.reposition_pack_units(pack_idx, &mut units);
                    }

                    if left_click {
                        // Drop all — check overlaps
                        let dragging_set: Vec<usize> = build.multi_dragging.clone();
                        let mut any_overlap = false;
                        for &pack_idx in &dragging_set {
                            let placed = &build.placed_packs[pack_idx];
                            // Check overlap against non-dragged packs
                            for (j, other) in build.placed_packs.iter().enumerate() {
                                if dragging_set.contains(&j) { continue; }
                                let p1 = &all_packs()[placed.pack_index];
                                let p2 = &all_packs()[other.pack_index];
                                if placed.overlaps(other, p1, p2) {
                                    any_overlap = true;
                                    break;
                                }
                            }
                            if any_overlap { break; }
                        }

                        if any_overlap {
                            // Revert all to pre-centers
                            for (i, &pack_idx) in dragging_set.iter().enumerate() {
                                build.placed_packs[pack_idx].center = build.multi_drag_pre_centers[i];
                                build.reposition_pack_units(pack_idx, &mut units);
                            }
                        } else {
                            // Check if any actually moved
                            let mut any_moved = false;
                            for (i, &pack_idx) in dragging_set.iter().enumerate() {
                                if build.placed_packs[pack_idx].center != build.multi_drag_pre_centers[i] {
                                    any_moved = true;
                                    break;
                                }
                            }
                            if any_moved {
                                build.undo_history.push(game_state::UndoEntry::MultiMove {
                                    indices: dragging_set.clone(),
                                    old_centers: build.multi_drag_pre_centers.clone(),
                                });
                            }
                        }
                        build.multi_dragging.clear();
                        build.multi_drag_offsets.clear();
                        build.multi_drag_pre_centers.clear();
                    }

                    if right_click {
                        // Cancel — revert all
                        let dragging_set: Vec<usize> = build.multi_dragging.clone();
                        for (i, &pack_idx) in dragging_set.iter().enumerate() {
                            build.placed_packs[pack_idx].center = build.multi_drag_pre_centers[i];
                            build.reposition_pack_units(pack_idx, &mut units);
                        }
                        build.multi_dragging.clear();
                        build.multi_drag_offsets.clear();
                        build.multi_drag_pre_centers.clear();
                    }
                }
                // Drag-box: track while held, complete on release
                else if let Some(box_start) = build.drag_box_start {
                    if left_released {
                        let box_end = mouse;
                        let min_x = box_start.x.min(box_end.x);
                        let max_x = box_start.x.max(box_end.x);
                        let min_y = box_start.y.min(box_end.y);
                        let max_y = box_start.y.max(box_end.y);

                        // Minimum box size threshold to distinguish from accidental micro-drag
                        if (max_x - min_x) > 5.0 || (max_y - min_y) > 5.0 {
                            let packs = all_packs();
                            let mut selected_indices = Vec::new();
                            for (i, placed) in build.placed_packs.iter().enumerate() {
                                if placed.locked { continue; }
                                let pack = &packs[placed.pack_index];
                                let half = placed.bbox_half_size_for(pack);
                                let p_min = placed.center - half;
                                let p_max = placed.center + half;
                                // AABB intersection test
                                if p_min.x < max_x && p_max.x > min_x && p_min.y < max_y && p_max.y > min_y {
                                    selected_indices.push(i);
                                }
                            }

                            if !selected_indices.is_empty() {
                                // Compute anchor as center of selection box
                                let anchor = vec2((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
                                let mut offsets = Vec::new();
                                let mut pre_centers = Vec::new();
                                for &idx in &selected_indices {
                                    offsets.push(build.placed_packs[idx].center - anchor);
                                    pre_centers.push(build.placed_packs[idx].center);
                                }
                                build.multi_dragging = selected_indices;
                                build.multi_drag_offsets = offsets;
                                build.multi_drag_pre_centers = pre_centers;
                                build.selected_pack = None;
                            }
                        }
                        build.drag_box_start = None;
                    }
                    // Right-click cancels the drag box
                    if right_click {
                        build.drag_box_start = None;
                    }
                }
                // Start drag-box when clicking empty space (not on a pack, not on UI)
                else if left_click && screen_mouse.x > shop_w() && !click_consumed
                    && build.dragging.is_none() && build.multi_dragging.is_empty()
                    && build.pack_at(mouse).is_none() && build.selected_pack.is_none()
                {
                    build.drag_box_start = Some(mouse);
                }

                // === Single-pack click-to-hold/place logic ===
                if build.multi_dragging.is_empty() && build.drag_box_start.is_none() {
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
                } else if left_click && screen_mouse.x > shop_w() && !click_consumed {
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
                } // end multi_dragging.is_empty() guard

                // Begin Round button (screen-space UI)
                let btn_w = crate::ui::s(160.0);
                let btn_h = crate::ui::s(40.0);
                let btn_x = screen_width() / 2.0 - btn_w / 2.0;
                let btn_y = screen_height() - crate::ui::s(55.0);
                if left_click
                    && screen_mouse.x >= btn_x
                    && screen_mouse.x <= btn_x + btn_w
                    && screen_mouse.y >= btn_y
                    && screen_mouse.y <= btn_y + btn_h
                {
                    if net.is_some() {
                        // Multiplayer: send build data, wait for opponent
                        net::send_build_complete(&mut net, &build);
                        phase = GamePhase::WaitingForOpponent;
                    } else {
                        // Single-player: start battle with AI
                        phase = economy::start_ai_battle(
                            &mut build,
                            &mut units,
                            &mut projectiles,
                            &mut progress,
                            &mut obstacles,
                            &mut nav_grid,
                            &game_settings,
                        );
                        battle_accumulator = 0.0;
                        battle_timer = 0.0;
                        battle_frame = 0;
                        recent_hashes.clear();
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

                        // Generate terrain once per match; subsequent rounds just reset cover HP
                        if obstacles.is_empty() && game_settings.terrain_enabled {
                            obstacles = terrain::generate_terrain(progress.round, game_settings.terrain_destructible);
                        } else {
                            terrain::reset_cover_hp(&mut obstacles);
                        }
                        nav_grid = Some(terrain::NavGrid::from_obstacles(&obstacles, ARENA_W, ARENA_H, 15.0));

                        // Seed RNG for deterministic battle
                        macroquad::rand::srand(progress.round as u64);
                        battle_accumulator = 0.0;
                        battle_timer = 0.0;
                        battle_frame = 0;
                        recent_hashes.clear();

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
                        update_movement(&mut units, FIXED_DT, ARENA_W, ARENA_H, &obstacles, nav_grid.as_ref());
                        update_attacks(
                            &mut units,
                            &mut projectiles,
                            FIXED_DT,
                            &progress.player_techs,
                            &progress.opponent_techs,
                            &mut splash_effects,
                        );
                        update_projectiles(&mut projectiles, &mut units, FIXED_DT, &mut obstacles, &mut splash_effects);
                        // Death animation timers (inside fixed timestep for determinism)
                        for unit in units.iter_mut() {
                            if !unit.alive && unit.death_timer > 0.0 {
                                unit.death_timer -= FIXED_DT;
                            }
                        }
                        battle_frame += 1;

                        // --- Sync hashing every SYNC_INTERVAL frames ---
                        if let Some(ref mut n) = net {
                            if battle_frame % SYNC_INTERVAL == 0 {
                                if n.is_host {
                                    let local_hash = sync::compute_state_hash(&units, &projectiles, &obstacles, false);
                                    n.send(net::NetMessage::StateHash { frame: battle_frame, hash: local_hash });
                                } else {
                                    // Guest: store hash for this frame so we can compare when host's hash arrives
                                    let local_hash = sync::compute_state_hash(&units, &projectiles, &obstacles, true);
                                    if recent_hashes.len() >= 4 {
                                        recent_hashes.pop_front();
                                    }
                                    recent_hashes.push_back((battle_frame, local_hash));
                                }
                            }
                        }
                    }

                    // --- Desync detection & state sync (outside fixed-timestep loop) ---
                    // Poll network again to pick up any messages that arrived during simulation
                    if let Some(ref mut n) = net {
                        n.poll();

                        if n.is_host {
                            // Host: respond to state request from guest
                            if let Some(_req_frame) = n.received_state_request.take() {
                                let (units_data, projectiles_data, obstacles_data) =
                                    sync::serialize_state(&units, &projectiles, &obstacles);
                                eprintln!("[SYNC] Host sending full state at frame {} ({} + {} + {} bytes)",
                                    battle_frame, units_data.len(), projectiles_data.len(), obstacles_data.len());
                                n.send(net::NetMessage::StateSync {
                                    frame: battle_frame,
                                    units_data,
                                    projectiles_data,
                                    obstacles_data,
                                });
                            }
                        } else {
                            // Guest: check hash from host against our stored hash for that frame
                            if let Some((host_frame, host_hash)) = n.received_state_hash.take() {
                                if let Some(pos) = recent_hashes.iter().position(|(f, _)| *f == host_frame) {
                                    let (_, local_hash) = recent_hashes[pos];
                                    if host_hash != local_hash {
                                        eprintln!("[DESYNC] Hash mismatch at frame {}! Requesting state.", host_frame);
                                        n.send(net::NetMessage::StateRequest { frame: battle_frame });
                                    }
                                    // Remove this and older hashes
                                    recent_hashes.drain(..=pos);
                                }
                            }

                            // Guest: apply state correction from host immediately (mirror positions)
                            if let Some(sync_data) = n.received_state_sync.take() {
                                eprintln!("[SYNC] Guest applying host state correction (host frame {}, local frame {})",
                                    sync_data.frame, battle_frame);
                                sync::apply_state_sync(
                                    &mut units,
                                    &mut projectiles,
                                    &mut obstacles,
                                    &sync_data.units_data,
                                    &sync_data.projectiles_data,
                                    &sync_data.obstacles_data,
                                    true,
                                );
                            }
                        }
                    }
                } else {
                    // Single-player: variable timestep (original behavior)
                    update_targeting(&mut units, &obstacles);
                    update_movement(&mut units, dt, ARENA_W, ARENA_H, &obstacles, nav_grid.as_ref());
                    update_attacks(
                        &mut units,
                        &mut projectiles,
                        dt,
                        &progress.player_techs,
                        &progress.opponent_techs,
                        &mut splash_effects,
                    );
                    update_projectiles(&mut projectiles, &mut units, dt, &mut obstacles, &mut splash_effects);
                    // Death animation timers
                    for unit in units.iter_mut() {
                        if !unit.alive && unit.death_timer > 0.0 {
                            unit.death_timer -= dt;
                        }
                    }
                }

                // Surrender confirmation handling
                if show_surrender_confirm && is_mouse_button_pressed(MouseButton::Left) {
                    let btn_w = crate::ui::s(120.0);
                    let btn_h = crate::ui::s(40.0);
                    let cx = screen_width() / 2.0;
                    let cy = screen_height() / 2.0;
                    // "Yes" button
                    let yes_x = cx - btn_w - crate::ui::s(10.0);
                    let yes_y = cy + crate::ui::s(10.0);
                    if screen_mouse.x >= yes_x && screen_mouse.x <= yes_x + btn_w && screen_mouse.y >= yes_y && screen_mouse.y <= yes_y + btn_h {
                        progress.player_lp = 0;
                        show_surrender_confirm = false;
                        phase = GamePhase::GameOver(1);
                    }
                    // "Cancel" button
                    let no_x = cx + crate::ui::s(10.0);
                    let no_y = cy + crate::ui::s(10.0);
                    if screen_mouse.x >= no_x && screen_mouse.x <= no_x + btn_w && screen_mouse.y >= no_y && screen_mouse.y <= no_y + btn_h {
                        show_surrender_confirm = false;
                    }
                }

                // Round timeout
                battle_timer += dt;
                let timed_out = battle_timer >= ROUND_TIMEOUT;

                let state = check_match_state(&units);
                let is_multiplayer = net.is_some();
                let is_host_game = net.as_ref().map_or(true, |n| n.is_host);
                let battle_ended = (state != MatchState::InProgress && projectiles.is_empty()) || timed_out;

                // Guest waiting for host's authoritative round result
                if waiting_for_round_end {
                    round_end_timeout -= dt;
                    if let Some(ref mut n) = net {
                        if let Some(rd) = n.received_round_end.take() {
                            // Flip host's perspective to guest's: host team 0 = guest team 1
                            let flipped_winner = rd.winner.map(|w| 1 - w);
                            let flipped_loser = rd.loser_team.map(|l| 1 - l);

                            let final_state = match flipped_winner {
                                Some(w) => MatchState::Winner(w),
                                None => MatchState::Draw,
                            };

                            // Log desync check (flip host counts to match guest perspective)
                            let local_alive_0 = units.iter().filter(|u| u.alive && u.team_id == 0).count() as u16;
                            let local_alive_1 = units.iter().filter(|u| u.alive && u.team_id == 1).count() as u16;
                            if local_alive_0 != rd.alive_1 || local_alive_1 != rd.alive_0 {
                                eprintln!("[DESYNC] Unit count mismatch! Local: {}/{} Host(flipped): {}/{}", local_alive_0, local_alive_1, rd.alive_1, rd.alive_0);
                            }

                            // Apply timeout mutual damage (flipped for guest perspective)
                            if rd.timeout_dmg_0 > 0 || rd.timeout_dmg_1 > 0 {
                                // Host's team 0 = guest's team 1, so flip
                                progress.player_lp -= rd.timeout_dmg_1;
                                progress.opponent_lp -= rd.timeout_dmg_0;
                            } else if let Some(loser) = flipped_loser {
                                if loser == 0 {
                                    progress.player_lp -= rd.lp_damage;
                                } else {
                                    progress.opponent_lp -= rd.lp_damage;
                                }
                            }

                            waiting_for_round_end = false;
                            show_surrender_confirm = false;
                            phase = GamePhase::RoundResult {
                                match_state: final_state,
                                lp_damage: rd.lp_damage,
                                loser_team: flipped_loser,
                            };
                        } else if round_end_timeout <= 0.0 {
                            // Timeout — fall back to local computation
                            eprintln!("[DESYNC] Timeout waiting for host RoundEnd, using local values");
                            waiting_for_round_end = false;
                            // Fall through to local computation below
                        }
                    }
                }

                if battle_ended && !waiting_for_round_end {
                    let final_state = if timed_out { MatchState::Draw } else { check_match_state(&units) };

                    // Record AI memory for counter-picking
                    let ai_won = match &final_state {
                        MatchState::Winner(w) => *w == 1,
                        _ => false,
                    };
                    progress.ai_memory.record_round(&units, ai_won);

                    // Calculate LP damage
                    let alive_0 = units.iter().filter(|u| u.alive && u.team_id == 0).count() as i32;
                    let alive_1 = units.iter().filter(|u| u.alive && u.team_id == 1).count() as i32;

                    // Compute damage and loser — but DON'T apply yet (guest needs
                    // the same values from the network message).
                    let (lp_damage, loser_team, timeout_dmg_0, timeout_dmg_1) = if timed_out {
                        // Timeout: both players take damage equal to opponent's surviving units
                        (0, None, alive_1, alive_0)
                    } else {
                        match &final_state {
                            MatchState::Winner(winner) => {
                                let damage = MatchProgress::calculate_lp_damage(&units, *winner);
                                let loser = if *winner == 0 { 1u8 } else { 0u8 };
                                (damage, Some(loser), 0, 0)
                            }
                            MatchState::Draw => (0, None, 0, 0),
                            MatchState::InProgress => unreachable!(),
                        }
                    };

                    if is_multiplayer && !is_host_game {
                        // Guest: wait for host's authoritative result
                        waiting_for_round_end = true;
                        round_end_timeout = 5.0;
                    } else {
                        // Host or single-player: we are authoritative
                        if is_multiplayer {
                            // Host sends round result to guest
                            let alive_0 = alive_0 as u16;
                            let alive_1 = alive_1 as u16;
                            let total_hp_0: i32 = units.iter().filter(|u| u.alive && u.team_id == 0).map(|u| u.hp as i32).sum();
                            let total_hp_1: i32 = units.iter().filter(|u| u.alive && u.team_id == 1).map(|u| u.hp as i32).sum();
                            let winner = match &final_state {
                                MatchState::Winner(w) => Some(*w),
                                _ => None,
                            };
                            if let Some(ref mut n) = net {
                                n.send(net::NetMessage::RoundEnd {
                                    winner, lp_damage, loser_team,
                                    alive_0, alive_1, total_hp_0, total_hp_1,
                                    timeout_dmg_0, timeout_dmg_1,
                                });
                            }
                        }

                        // Apply LP damage
                        if timed_out {
                            progress.player_lp -= timeout_dmg_0;
                            progress.opponent_lp -= timeout_dmg_1;
                        } else if let Some(loser) = loser_team {
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
                        units.extend(build.respawn_player_units(&progress.player_techs));

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

                        // Respawn opponent units from stored packs (visible during build phase).
                        // Works for both single-player (AI packs) and multiplayer (network packs).
                        units.extend(progress.respawn_opponent_units());

                        projectiles.clear();
                        phase = GamePhase::Build;
                    }
                }
            }

            GamePhase::GameOver(_) => {
                if is_key_pressed(KeyCode::R) {
                    progress = MatchProgress::new(true);
                    phase = GamePhase::Lobby;
                    build = BuildState::new(progress.round_gold(), true);
                    units.clear();
                    projectiles.clear();
                    net = None;
                    lobby.reset();
                }

                // Rematch button click — position must match render (panel_y + panel_h + 8 + 15)
                let rmatch_w = crate::ui::s(160.0);
                let rmatch_h = crate::ui::s(40.0);
                let rmatch_x = screen_width() / 2.0 - rmatch_w / 2.0;
                let rmatch_panel_y = screen_height() / 2.0 + 10.0;
                let rmatch_panel_h = crate::ui::s(140.0);
                let rmatch_y = rmatch_panel_y + rmatch_panel_h + crate::ui::s(8.0) + crate::ui::s(15.0);
                if left_click && screen_mouse.x >= rmatch_x && screen_mouse.x <= rmatch_x + rmatch_w
                    && screen_mouse.y >= rmatch_y && screen_mouse.y <= rmatch_y + rmatch_h
                {
                    // Reset for rematch (skip lobby, go straight to Build)
                    let is_host = net.as_ref().map_or(true, |n| n.is_host);
                    progress = MatchProgress::new(is_host);
                    build = BuildState::new(progress.round_gold(), is_host);
                    units.clear();
                    projectiles.clear();
                    obstacles.clear();
                    nav_grid = None;
                    show_surrender_confirm = false;
                    chat = chat::ChatState::new();
                    splash_effects.clear();
                    waiting_for_round_end = false;
                    phase = if game_settings.draft_ban_enabled {
                        GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None }
                    } else {
                        GamePhase::Build
                    };
                }
            }
        }

        rendering::update_splash_effects(&mut splash_effects, dt);

        // === Render ===
        clear_background(Color::new(0.1, 0.1, 0.12, 1.0));

        // Skip normal rendering for Lobby phase (it draws its own UI above)
        if matches!(phase, GamePhase::Lobby) {
            next_frame().await;
            continue;
        }

        // Always use Camera2D for world-space rendering
        set_camera(&arena_camera);

        rendering::draw_world(
            &units, &projectiles, &obstacles, &splash_effects,
            &build, &progress, show_grid,
            matches!(phase, GamePhase::Build),
            world_mouse,
        );

        // Reset camera for UI overlays (screen-space)
        set_default_camera();

        // === Phase-specific UI (screen-space) ===
        match &phase {
            GamePhase::Lobby | GamePhase::DraftBan { .. } => {
                // Handled above with early continue
            }

            GamePhase::Build => {
                shop::draw_shop(build.builder.gold_remaining, screen_mouse, false, &progress.banned_kinds, game_state::BUILD_LIMIT - build.packs_bought_this_round);

                // Pack labels (drawn in screen-space so text isn't distorted by camera zoom)
                {
                    let packs = all_packs();
                    for (_i, placed) in build.placed_packs.iter().enumerate() {
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
                        let half = game_state::PlacedPack::bbox_half_size_rotated(pack, opponent_pack.rotated);
                        let world_pos = vec2(opponent_pack.center.x - half.x + 2.0, opponent_pack.center.y - half.y - 2.0);
                        let screen_pos = arena_camera.world_to_screen(world_pos);
                        let label = format!("{} (R{})", pack.name, opponent_pack.round_placed);
                        crate::ui::draw_scaled_text(&label, screen_pos.x, screen_pos.y, 12.0, Color::new(0.4, 0.4, 0.6, 0.4));
                    }
                }

                // Tech panel (when a pack is selected)
                if let Some(sel_idx) = build.selected_pack {
                    if sel_idx < build.placed_packs.len() {
                        let placed = &build.placed_packs[sel_idx];
                        let kind = all_packs()[placed.pack_index].kind;
                        let cs = tech_ui::PackCombatStats::from_units(&units, &placed.unit_ids);
                        tech_ui::draw_tech_panel(
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
                ui::draw_hud(&progress, build.builder.gold_remaining, build.timer, army_value, 0.0, &mp_player_name, &mp_opponent_name);

                // Begin Round button (screen-space)
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
                draw_rectangle_lines(
                    btn_x,
                    btn_y,
                    btn_w,
                    btn_h,
                    2.0,
                    Color::new(0.3, 0.8, 0.4, 1.0),
                );
                let btn_text = "Begin Round";
                let tdims = crate::ui::measure_scaled_text(btn_text, 22);
                crate::ui::draw_scaled_text(
                    btn_text,
                    btn_x + btn_w / 2.0 - tdims.width / 2.0,
                    btn_y + btn_h / 2.0 + 7.0,
                    22.0,
                    WHITE,
                );

                // Hint text (screen-space)
                crate::ui::draw_scaled_text(
                    "Select → Double-click move | Mid-click rotate | Right-click sell | G: Grid | Ctrl+Z: Undo | Scroll: Zoom",
                    shop_w() + 10.0,
                    screen_height() - crate::ui::s(10.0),
                    13.0,
                    Color::new(0.5, 0.5, 0.5, 0.7),
                );
            }

            GamePhase::WaitingForOpponent => {
                ui::draw_hud(&progress, build.builder.gold_remaining, 0.0, 0, 0.0, &mp_player_name, &mp_opponent_name);

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

            GamePhase::Battle => {
                let remaining = (ROUND_TIMEOUT - battle_timer).max(0.0);
                ui::draw_hud(&progress, 0, 0.0, 0, remaining, &mp_player_name, &mp_opponent_name);

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

                // Obstacle tooltip on hover (hit test in world coords, draw in screen coords)
                if !show_surrender_confirm {
                    for obs in &obstacles {
                        if !obs.alive { continue; }
                        if obs.contains_point(world_mouse) {
                            let tip_x = screen_mouse.x + crate::ui::s(15.0);
                            let tip_y = (screen_mouse.y - crate::ui::s(10.0)).max(5.0);
                            let tip_w = crate::ui::s(170.0);
                            let tip_h = if obs.obstacle_type == terrain::ObstacleType::Cover { crate::ui::s(60.0) } else { crate::ui::s(45.0) };

                            draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.08, 0.08, 0.12, 0.95));
                            draw_rectangle_lines(tip_x, tip_y, tip_w, tip_h, 1.0, Color::new(0.4, 0.5, 0.6, 0.7));

                            let type_name = match obs.obstacle_type {
                                terrain::ObstacleType::Wall => "Wall (Indestructible)",
                                terrain::ObstacleType::Cover => "Cover (Destructible)",
                            };
                            crate::ui::draw_scaled_text(type_name, tip_x + crate::ui::s(6.0), tip_y + crate::ui::s(16.0), 14.0, WHITE);

                            let mut ty = tip_y + crate::ui::s(32.0);
                            if obs.obstacle_type == terrain::ObstacleType::Cover {
                                crate::ui::draw_scaled_text(&format!("HP: {:.0}/{:.0}", obs.hp, obs.max_hp), tip_x + crate::ui::s(6.0), ty, 12.0, LIGHTGRAY);
                                ty += crate::ui::s(14.0);
                            }
                            let team_name = match obs.team_id { 0 => mp_player_name.as_str(), 1 => mp_opponent_name.as_str(), _ => "Neutral" };
                            crate::ui::draw_scaled_text(&format!("Owner: {}", team_name), tip_x + crate::ui::s(6.0), ty, 12.0, LIGHTGRAY);
                            break;
                        }
                    }
                }

                // Surrender confirmation overlay
                if show_surrender_confirm {
                    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.6));
                    let title = "Surrender?";
                    let tdims = crate::ui::measure_scaled_text(title, 36);
                    crate::ui::draw_scaled_text(title, screen_width() / 2.0 - tdims.width / 2.0, screen_height() / 2.0 - crate::ui::s(20.0), 36.0, WHITE);

                    let btn_w: f32 = crate::ui::s(120.0);
                    let btn_h: f32 = crate::ui::s(40.0);
                    let cx = screen_width() / 2.0;
                    let cy = screen_height() / 2.0;

                    // Yes button
                    let yes_x = cx - btn_w - crate::ui::s(10.0);
                    let yes_y = cy + crate::ui::s(10.0);
                    let yes_hover = screen_mouse.x >= yes_x && screen_mouse.x <= yes_x + btn_w && screen_mouse.y >= yes_y && screen_mouse.y <= yes_y + btn_h;
                    let yes_color = if yes_hover { Color::new(0.8, 0.2, 0.2, 0.9) } else { Color::new(0.6, 0.15, 0.15, 0.8) };
                    draw_rectangle(yes_x, yes_y, btn_w, btn_h, yes_color);
                    draw_rectangle_lines(yes_x, yes_y, btn_w, btn_h, 1.0, WHITE);
                    let yt = "Yes";
                    let ydims = crate::ui::measure_scaled_text(yt, 20);
                    crate::ui::draw_scaled_text(yt, yes_x + btn_w / 2.0 - ydims.width / 2.0, yes_y + btn_h / 2.0 + 6.0, 20.0, WHITE);

                    // Cancel button
                    let no_x = cx + crate::ui::s(10.0);
                    let no_y = cy + crate::ui::s(10.0);
                    let no_hover = screen_mouse.x >= no_x && screen_mouse.x <= no_x + btn_w && screen_mouse.y >= no_y && screen_mouse.y <= no_y + btn_h;
                    let no_color = if no_hover { Color::new(0.3, 0.3, 0.35, 0.9) } else { Color::new(0.2, 0.2, 0.25, 0.8) };
                    draw_rectangle(no_x, no_y, btn_w, btn_h, no_color);
                    draw_rectangle_lines(no_x, no_y, btn_w, btn_h, 1.0, WHITE);
                    let nt = "Cancel";
                    let ndims = crate::ui::measure_scaled_text(nt, 20);
                    crate::ui::draw_scaled_text(nt, no_x + btn_w / 2.0 - ndims.width / 2.0, no_y + btn_h / 2.0 + 6.0, 20.0, WHITE);
                }
            }

            GamePhase::RoundResult {
                match_state,
                lp_damage,
                loser_team,
            } => {
                ui::draw_hud(&progress, 0, 0.0, 0, 0.0, &mp_player_name, &mp_opponent_name);

                let text = match match_state {
                    MatchState::Winner(tid) => {
                        let (winner_name, color_idx) = if *tid == 0 {
                            (&mp_player_name, game_settings.player_color_index)
                        } else {
                            let opp_idx = net.as_ref().and_then(|n| n.opponent_color).unwrap_or(1);
                            (&mp_opponent_name, opp_idx)
                        };
                        let color_name = settings::TEAM_COLOR_OPTIONS
                            .get(color_idx as usize)
                            .map(|(name, _)| *name)
                            .unwrap_or("???");
                        format!("{} ({}) wins round {}!", winner_name, color_name, progress.round)
                    }
                    MatchState::Draw => format!("Round {} - Draw!", progress.round),
                    MatchState::InProgress => unreachable!(),
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
                    let loser_name = if *loser == 0 { &mp_player_name } else { &mp_opponent_name };
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

            GamePhase::GameOver(winner) => {
                let (headline, winner_color_idx) = if *winner == 0 {
                    ("YOU WIN!".to_string(), game_settings.player_color_index)
                } else {
                    ("YOU LOSE!".to_string(), net.as_ref().and_then(|n| n.opponent_color).unwrap_or(1))
                };
                let winner_name = if *winner == 0 { &mp_player_name } else { &mp_opponent_name };
                let color_name = settings::TEAM_COLOR_OPTIONS
                    .get(winner_color_idx as usize)
                    .map(|(name, _)| *name)
                    .unwrap_or("???");
                let subtitle = format!("{} ({}) wins!", winner_name, color_name);
                let headline_color = if *winner == 0 {
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
                draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, Color::new(0.4, 0.5, 0.6, 0.7));

                let mut sy = panel_y + crate::ui::s(18.0);
                let sx = panel_x + crate::ui::s(12.0);

                let round_text = format!("Rounds Played: {}", progress.round);
                crate::ui::draw_scaled_text(&round_text, sx, sy, 15.0, LIGHTGRAY);
                sy += crate::ui::s(18.0);

                // MVP
                let mvp = units.iter()
                    .filter(|u| u.team_id == 0)
                    .max_by(|a, b| a.damage_dealt_total.partial_cmp(&b.damage_dealt_total).unwrap_or(std::cmp::Ordering::Equal));
                if let Some(mvp_unit) = mvp {
                    let mvp_text = format!("MVP: {:?} - {:.0} dmg, {} kills", mvp_unit.kind, mvp_unit.damage_dealt_total, mvp_unit.kills_total);
                    crate::ui::draw_scaled_text(&mvp_text, sx, sy, 15.0, Color::new(1.0, 0.85, 0.2, 1.0));
                }
                sy += crate::ui::s(18.0);

                let total_dmg: f32 = units.iter()
                    .filter(|u| u.team_id == 0)
                    .map(|u| u.damage_dealt_total)
                    .sum();
                crate::ui::draw_scaled_text(&format!("Total Damage: {:.0}", total_dmg), sx, sy, 15.0, LIGHTGRAY);
                sy += crate::ui::s(18.0);

                let surviving = units.iter().filter(|u| u.team_id == 0 && u.alive).count();
                let total_units = units.iter().filter(|u| u.team_id == 0).count();
                crate::ui::draw_scaled_text(&format!("Surviving: {} / {}", surviving, total_units), sx, sy, 15.0, LIGHTGRAY);
                sy += crate::ui::s(18.0);

                crate::ui::draw_scaled_text(&format!("LP: {} {} vs {} {}", mp_player_name, progress.player_lp, mp_opponent_name, progress.opponent_lp), sx, sy, 15.0, LIGHTGRAY);

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
                let rmatch_hover = screen_mouse.x >= rmatch_x && screen_mouse.x <= rmatch_x + rmatch_w && screen_mouse.y >= rmatch_y && screen_mouse.y <= rmatch_y + rmatch_h;
                let rmatch_bg = if rmatch_hover { Color::new(0.2, 0.5, 0.3, 0.9) } else { Color::new(0.15, 0.35, 0.2, 0.8) };
                draw_rectangle(rmatch_x, rmatch_y, rmatch_w, rmatch_h, rmatch_bg);
                draw_rectangle_lines(rmatch_x, rmatch_y, rmatch_w, rmatch_h, 2.0, Color::new(0.3, 0.8, 0.4, 1.0));
                let rt = "Rematch";
                let rdims2 = crate::ui::measure_scaled_text(rt, 22);
                crate::ui::draw_scaled_text(rt, rmatch_x + rmatch_w / 2.0 - rdims2.width / 2.0, rmatch_y + rmatch_h / 2.0 + 7.0, 22.0, WHITE);
            }
        }

        // Disconnection overlay (shown over any phase if net is disconnected)
        if let Some(ref n) = net {
            if n.disconnected {
                // Semi-transparent dark overlay
                draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.7));
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

        // Chat system
        chat.receive_from_net(&mut net);
        chat.update(&phase, &mut net, &mp_player_name);
        chat.tick(dt);
        chat.draw(&phase, &mp_player_name);

        next_frame().await;
    }
}

