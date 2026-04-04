mod arena;
mod chat;
mod context;
mod draft_ban;
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
mod phase_ui;

use macroquad::prelude::*;

use arena::{check_match_state, MatchState, ARENA_H, ARENA_W, HALF_W, shop_w};
use combat::{update_attacks, update_movement, update_projectiles, update_targeting};
use game_state::{BuildState, GamePhase};
use match_progress::MatchProgress;
use pack::all_packs;
use projectile::Projectile;
use rendering::SplashEffect;

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
    let mut ctx = context::GameContext::new(true);
    let mut projectiles: Vec<Projectile> = Vec::new();
    let mut lobby = lobby::LobbyState::new();
    let mut battle_accumulator: f32 = 0.0;
    let mut battle_timer: f32 = 0.0;
    let mut battle_frame: u32 = 0;
    const ROUND_TIMEOUT: f32 = 90.0;
    const SYNC_INTERVAL: u32 = 4;
    // Guest keeps recent frame hashes so it can match against the host's frame
    let mut recent_hashes: std::collections::VecDeque<(u32, u64)> = std::collections::VecDeque::with_capacity(5);
    let mut main_settings = settings::MainSettings::default();
    let mut show_surrender_confirm = false;
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
        team::set_player_color(ctx.game_settings.player_color_index);
        ui::set_text_scale(main_settings.ui_scale);
        // Apply opponent color if received
        if let Some(ref n) = ctx.net {
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
        let mouse = if matches!(ctx.phase, GamePhase::Lobby) { screen_mouse } else { world_mouse };

        // Fullscreen toggle (F11)
        if is_key_pressed(KeyCode::F11) {
            is_fullscreen_mode = !is_fullscreen_mode;
            set_fullscreen(is_fullscreen_mode);
        }

        // Camera zoom/pan (available in all non-lobby phases)
        if !matches!(ctx.phase, GamePhase::Lobby) {
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

        match &mut ctx.phase {
            GamePhase::Lobby => {
                match lobby.update(&mut ctx.game_settings, &mut main_settings) {
                    lobby::LobbyResult::StartMultiplayer => {
                        let is_host = lobby.is_room_creator;
                        ctx.net = lobby.net.take();
                        if let Some(ref mut n) = ctx.net {
                            n.is_host = is_host;
                            ctx.mp_opponent_name = n.opponent_name.clone().unwrap_or_else(|| "Opponent".to_string());
                        }
                        ctx.mp_player_name = lobby.player_name.clone();
                        ctx.progress = MatchProgress::new(is_host);
                        ctx.build = BuildState::new(ctx.progress.round_gold(), is_host);
                        if ctx.game_settings.draft_ban_enabled {
                            ctx.phase = GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None };
                        } else {
                            ctx.phase = GamePhase::Build;
                        }
                        continue;
                    }
                    lobby::LobbyResult::StartVsAi => {
                        ctx.net = None;
                        ctx.mp_player_name = lobby.player_name.clone();
                        ctx.mp_opponent_name = "AI".to_string();
                        ctx.progress = MatchProgress::new(true);
                        ctx.build = BuildState::new(ctx.progress.round_gold(), true);
                        if ctx.game_settings.draft_ban_enabled {
                            ctx.phase = GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None };
                        } else {
                            ctx.phase = GamePhase::Build;
                        }
                        continue;
                    }
                    lobby::LobbyResult::Waiting => {}
                }

                match lobby.draw(&mut ctx.game_settings, &mut main_settings) {
                    lobby::LobbyResult::StartMultiplayer => {
                        let is_host = lobby.is_room_creator;
                        ctx.net = lobby.net.take();
                        if let Some(ref mut n) = ctx.net {
                            n.is_host = is_host;
                            ctx.mp_opponent_name = n.opponent_name.clone().unwrap_or_else(|| "Opponent".to_string());
                        }
                        ctx.mp_player_name = lobby.player_name.clone();
                        ctx.progress = MatchProgress::new(is_host);
                        ctx.build = BuildState::new(ctx.progress.round_gold(), is_host);
                        if ctx.game_settings.draft_ban_enabled {
                            ctx.phase = GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None };
                        } else {
                            ctx.phase = GamePhase::Build;
                        }
                        continue;
                    }
                    lobby::LobbyResult::StartVsAi => {
                        ctx.net = None;
                        ctx.mp_player_name = lobby.player_name.clone();
                        ctx.mp_opponent_name = "AI".to_string();
                        ctx.progress = MatchProgress::new(true);
                        ctx.build = BuildState::new(ctx.progress.round_gold(), true);
                        if ctx.game_settings.draft_ban_enabled {
                            ctx.phase = GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None };
                        } else {
                            ctx.phase = GamePhase::Build;
                        }
                        continue;
                    }
                    _ => {}
                }

                next_frame().await;
                continue;
            }

            GamePhase::DraftBan { ref mut bans, ref mut confirmed, ref mut opponent_bans } => {
                match draft_ban::update_and_draw(bans, confirmed, opponent_bans, &mut ctx.net, screen_mouse, left_click) {
                    draft_ban::DraftBanResult::Waiting => {}
                    draft_ban::DraftBanResult::Done(all_bans) => {
                        ctx.progress.banned_kinds = all_bans;
                        ctx.phase = GamePhase::Build;
                    }
                }
                next_frame().await;
                continue;
            }

            GamePhase::Build => {
                // Poll network
                if let Some(ref mut n) = ctx.net {
                    n.poll();
                }

                // Grid toggle
                if is_key_pressed(KeyCode::G) {
                    ctx.show_grid = !ctx.show_grid;
                }

                // Undo (Ctrl+Z)
                if is_key_down(KeyCode::LeftControl) && is_key_pressed(KeyCode::Z) && ctx.build.dragging.is_none() {
                    if let Some(entry) = ctx.build.undo_history.pop() {
                        match entry {
                            game_state::UndoEntry::Place { placed_index } => {
                                if placed_index < ctx.build.placed_packs.len() {
                                    if let Some((_refund, removed_ids)) = ctx.build.sell_pack(placed_index) {
                                        ctx.units.retain(|u| !removed_ids.contains(&u.id));
                                    }
                                }
                            }
                            game_state::UndoEntry::Move { placed_index, old_center } => {
                                if placed_index < ctx.build.placed_packs.len() {
                                    ctx.build.placed_packs[placed_index].center = old_center;
                                    ctx.build.reposition_pack_units(placed_index, &mut ctx.units);
                                }
                            }
                            game_state::UndoEntry::Rotate { placed_index, was_rotated, old_center } => {
                                if placed_index < ctx.build.placed_packs.len() {
                                    ctx.build.placed_packs[placed_index].rotated = was_rotated;
                                    ctx.build.placed_packs[placed_index].center = old_center;
                                    ctx.build.reposition_pack_units(placed_index, &mut ctx.units);
                                }
                            }
                            game_state::UndoEntry::MultiMove { indices, old_centers } => {
                                for (i, &idx) in indices.iter().enumerate() {
                                    if idx < ctx.build.placed_packs.len() {
                                        ctx.build.placed_packs[idx].center = old_centers[i];
                                        ctx.build.reposition_pack_units(idx, &mut ctx.units);
                                    }
                                }
                            }
                            game_state::UndoEntry::Tech { kind, tech_id } => {
                                // Refund tech cost
                                let cost = ctx.progress.player_techs.effective_cost(kind);
                                // unpurchase first so effective_cost returns the right amount next time
                                ctx.progress.player_techs.unpurchase(kind, tech_id);
                                // Refund: cost was (100 + N*100) where N was count before purchase
                                // After unpurchase, effective_cost gives the old cost, so just refund that
                                ctx.build.builder.gold_remaining += cost;
                                // Remove from round tech purchases
                                if let Some(pos) = ctx.build.round_tech_purchases.iter().rposition(|(k, t)| *k == kind && *t == tech_id) {
                                    ctx.build.round_tech_purchases.remove(pos);
                                }
                                // Refresh ctx.units to remove tech effect
                                tech::refresh_units_of_kind(&mut ctx.units, kind, &ctx.progress.player_techs);
                            }
                        }
                    }
                }

                // Timer countdown
                ctx.build.timer -= dt;
                if ctx.build.timer <= 0.0 {
                    if ctx.net.is_some() {
                        // Multiplayer: send ctx.build, transition to waiting
                        net::send_build_complete(&mut ctx.net, &ctx.build);
                        ctx.phase = GamePhase::WaitingForOpponent;
                    } else {
                        // Single-player: start battle immediately with AI
                        ctx.phase = economy::start_ai_battle(
                            &mut ctx.build,
                            &mut ctx.units,
                            &mut projectiles,
                            &mut ctx.progress,
                            &mut ctx.obstacles,
                            &mut ctx.nav_grid,
                            &ctx.game_settings,
                        );
                        battle_accumulator = 0.0;
                        battle_timer = 0.0;
                        battle_frame = 0;
                        recent_hashes.clear();
                    }
                    continue;
                }

                // Shop interaction (left click in shop area, only when not holding a pack)
                if left_click && screen_mouse.x < shop_w() && ctx.build.dragging.is_none() {
                    if let Some(pack_idx) =
                        shop::draw_shop(ctx.build.builder.gold_remaining, screen_mouse, true, &ctx.progress.banned_kinds, game_state::BUILD_LIMIT - ctx.build.packs_bought_this_round)
                    {
                        if let Some(new_units) = ctx.build.purchase_pack(
                            pack_idx,
                            ctx.progress.round,
                            &ctx.progress.player_techs,
                        ) {
                            ctx.units.extend(new_units);
                        }
                    }
                }

                // Tech panel interaction (when a pack is selected)
                let mut click_consumed = false;
                if left_click && ctx.build.selected_pack.is_some() {
                    let sel_idx = ctx.build.selected_pack.unwrap();
                    let placed = &ctx.build.placed_packs[sel_idx];
                    let kind = all_packs()[placed.pack_index].kind;
                    let cs = tech_ui::PackCombatStats::from_units(&ctx.units, &placed.unit_ids);

                    // Check if mouse is in the tech panel area (consume click to prevent drag)
                    // Compute actual panel height to avoid blocking clicks in the entire column
                    let available_count = ctx.progress.player_techs.available_techs(kind).len();
                    let purchased_count = ctx.progress.player_techs.tech_count(kind);
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
                        &ctx.progress.player_techs,
                        ctx.build.builder.gold_remaining,
                        screen_mouse,
                        true,
                        Some(&cs),
                    ) {
                        let cost = ctx.progress.player_techs.effective_cost(kind);
                        if ctx.build.builder.gold_remaining >= cost {
                            ctx.build.builder.gold_remaining -= cost;
                            ctx.progress.player_techs.purchase(kind, tech_id);
                            // Track tech purchase for network sync and undo
                            ctx.build.round_tech_purchases.push((kind, tech_id));
                            ctx.build.undo_history.push(game_state::UndoEntry::Tech { kind, tech_id });
                            // Refresh ALL ctx.units of this kind with new tech stats
                            tech::refresh_units_of_kind(&mut ctx.units, kind, &ctx.progress.player_techs);
                        }
                    }
                }

                // Right-click: sell if on unlocked pack, otherwise deselect
                if right_click && screen_mouse.x > shop_w() && ctx.build.dragging.is_none() {
                    let mut sold = false;
                    if let Some(placed_idx) = ctx.build.pack_at(mouse) {
                        if !ctx.build.placed_packs[placed_idx].locked {
                            if let Some((_, removed_ids)) = ctx.build.sell_pack(placed_idx) {
                                ctx.units.retain(|u| !removed_ids.contains(&u.id));
                                sold = true;
                            }
                        }
                    }
                    if !sold {
                        // Deselect on right-click in empty space or on locked pack
                        ctx.build.selected_pack = None;
                    }
                }

                // Middle-click to rotate (only unlocked)
                if middle_click && screen_mouse.x > shop_w() {
                    if let Some(drag_idx) = ctx.build.dragging {
                        if !ctx.build.placed_packs[drag_idx].locked {
                            ctx.build.rotate_pack(drag_idx, &mut ctx.units);
                        }
                    } else if let Some(placed_idx) = ctx.build.pack_at(mouse) {
                        if !ctx.build.placed_packs[placed_idx].locked {
                            ctx.build.rotate_pack(placed_idx, &mut ctx.units);
                        }
                    }
                }

                // === Multi-drag system ===
                let left_released = is_mouse_button_released(MouseButton::Left);

                // Active multi-drag: move all packs together
                if !ctx.build.multi_dragging.is_empty() {
                    let grid = terrain::GRID_CELL;
                    let snapped_mouse = vec2(
                        (mouse.x / grid).round() * grid,
                        (mouse.y / grid).round() * grid,
                    );
                    for (i, &pack_idx) in ctx.build.multi_dragging.clone().iter().enumerate() {
                        let offset = ctx.build.multi_drag_offsets[i];
                        let new_center = snapped_mouse + offset;
                        let pack = &all_packs()[ctx.build.placed_packs[pack_idx].pack_index];
                        let half = ctx.build.placed_packs[pack_idx].bbox_half_size_for(pack);
                        let clamped = vec2(
                            new_center.x.clamp(half.x, HALF_W - half.x),
                            new_center.y.clamp(half.y, ARENA_H - half.y),
                        );
                        ctx.build.placed_packs[pack_idx].center = clamped;
                        ctx.build.reposition_pack_units(pack_idx, &mut ctx.units);
                    }

                    if left_click {
                        // Drop all — check overlaps
                        let dragging_set: Vec<usize> = ctx.build.multi_dragging.clone();
                        let mut any_overlap = false;
                        for &pack_idx in &dragging_set {
                            let placed = &ctx.build.placed_packs[pack_idx];
                            // Check overlap against non-dragged packs
                            for (j, other) in ctx.build.placed_packs.iter().enumerate() {
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
                                ctx.build.placed_packs[pack_idx].center = ctx.build.multi_drag_pre_centers[i];
                                ctx.build.reposition_pack_units(pack_idx, &mut ctx.units);
                            }
                        } else {
                            // Check if any actually moved
                            let mut any_moved = false;
                            for (i, &pack_idx) in dragging_set.iter().enumerate() {
                                if ctx.build.placed_packs[pack_idx].center != ctx.build.multi_drag_pre_centers[i] {
                                    any_moved = true;
                                    break;
                                }
                            }
                            if any_moved {
                                ctx.build.undo_history.push(game_state::UndoEntry::MultiMove {
                                    indices: dragging_set.clone(),
                                    old_centers: ctx.build.multi_drag_pre_centers.clone(),
                                });
                            }
                        }
                        ctx.build.multi_dragging.clear();
                        ctx.build.multi_drag_offsets.clear();
                        ctx.build.multi_drag_pre_centers.clear();
                    }

                    if right_click {
                        // Cancel — revert all
                        let dragging_set: Vec<usize> = ctx.build.multi_dragging.clone();
                        for (i, &pack_idx) in dragging_set.iter().enumerate() {
                            ctx.build.placed_packs[pack_idx].center = ctx.build.multi_drag_pre_centers[i];
                            ctx.build.reposition_pack_units(pack_idx, &mut ctx.units);
                        }
                        ctx.build.multi_dragging.clear();
                        ctx.build.multi_drag_offsets.clear();
                        ctx.build.multi_drag_pre_centers.clear();
                    }
                }
                // Drag-box: track while held, complete on release
                else if let Some(box_start) = ctx.build.drag_box_start {
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
                            for (i, placed) in ctx.build.placed_packs.iter().enumerate() {
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
                                    offsets.push(ctx.build.placed_packs[idx].center - anchor);
                                    pre_centers.push(ctx.build.placed_packs[idx].center);
                                }
                                ctx.build.multi_dragging = selected_indices;
                                ctx.build.multi_drag_offsets = offsets;
                                ctx.build.multi_drag_pre_centers = pre_centers;
                                ctx.build.selected_pack = None;
                            }
                        }
                        ctx.build.drag_box_start = None;
                    }
                    // Right-click cancels the drag box
                    if right_click {
                        ctx.build.drag_box_start = None;
                    }
                }
                // Start drag-box when clicking empty space (not on a pack, not on UI)
                else if left_click && screen_mouse.x > shop_w() && !click_consumed
                    && ctx.build.dragging.is_none() && ctx.build.multi_dragging.is_empty()
                    && ctx.build.pack_at(mouse).is_none() && ctx.build.selected_pack.is_none()
                {
                    ctx.build.drag_box_start = Some(mouse);
                }

                // === Single-pack click-to-hold/place logic ===
                if ctx.build.multi_dragging.is_empty() && ctx.build.drag_box_start.is_none() {
                if let Some(drag_idx) = ctx.build.dragging {
                    // Currently holding a pack — follow mouse
                    let pack = &all_packs()[ctx.build.placed_packs[drag_idx].pack_index];
                    let half = ctx.build.placed_packs[drag_idx].bbox_half_size_for(pack);
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
                    ctx.build.placed_packs[drag_idx].center = snapped;
                    ctx.build.reposition_pack_units(drag_idx, &mut ctx.units);

                    if left_click {
                        let placed = &ctx.build.placed_packs[drag_idx];
                        let pack_index = placed.pack_index;
                        let rotated = placed.rotated;
                        let old_center = placed.pre_drag_center;
                        if ctx.build.would_overlap(placed.center, pack_index, Some(drag_idx), rotated) {
                            ctx.build.placed_packs[drag_idx].center = old_center;
                            ctx.build.reposition_pack_units(drag_idx, &mut ctx.units);
                        } else if ctx.build.placed_packs[drag_idx].center != old_center {
                            ctx.build.undo_history.push(game_state::UndoEntry::Move { placed_index: drag_idx, old_center });
                        }
                        ctx.build.dragging = None;
                    }

                    if right_click {
                        let prev = ctx.build.placed_packs[drag_idx].pre_drag_center;
                        ctx.build.placed_packs[drag_idx].center = prev;
                        ctx.build.reposition_pack_units(drag_idx, &mut ctx.units);
                        ctx.build.dragging = None;
                    }
                } else if left_click && screen_mouse.x > shop_w() && !click_consumed {
                    // Not holding — selection logic
                    if let Some(placed_idx) = ctx.build.pack_at(mouse) {
                        if ctx.build.selected_pack == Some(placed_idx) {
                            // Already selected -> pick up (if not locked)
                            if !ctx.build.placed_packs[placed_idx].locked {
                                ctx.build.placed_packs[placed_idx].pre_drag_center =
                                    ctx.build.placed_packs[placed_idx].center;
                                ctx.build.dragging = Some(placed_idx);
                                ctx.build.selected_pack = None;
                            }
                        } else {
                            // Select this pack
                            ctx.build.selected_pack = Some(placed_idx);
                        }
                    } else if let Some(sel_idx) = ctx.build.selected_pack {
                        // Clicked empty space with a selected pack -> pick it up (if not locked)
                        if !ctx.build.placed_packs[sel_idx].locked {
                            ctx.build.placed_packs[sel_idx].pre_drag_center =
                                ctx.build.placed_packs[sel_idx].center;
                            ctx.build.dragging = Some(sel_idx);
                            ctx.build.selected_pack = None;
                        } else {
                            ctx.build.selected_pack = None;
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
                    if ctx.net.is_some() {
                        // Multiplayer: send ctx.build data, wait for opponent
                        net::send_build_complete(&mut ctx.net, &ctx.build);
                        ctx.phase = GamePhase::WaitingForOpponent;
                    } else {
                        // Single-player: start battle with AI
                        ctx.phase = economy::start_ai_battle(
                            &mut ctx.build,
                            &mut ctx.units,
                            &mut projectiles,
                            &mut ctx.progress,
                            &mut ctx.obstacles,
                            &mut ctx.nav_grid,
                            &ctx.game_settings,
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
                if let Some(ref mut n) = ctx.net {
                    n.poll();

                    // Check if opponent ctx.build data arrived
                    if let Some(opp_build) = n.take_opponent_build() {
                        // Apply opponent ctx.build
                        let opp_units = ctx.progress.apply_opponent_build(&opp_build);

                        // Remove old opponent ctx.units, respawn from stored packs
                        ctx.units.retain(|u| u.team_id == 0);
                        ctx.units.extend(ctx.progress.respawn_opponent_units());

                        // Also add any newly spawned opponent ctx.units from this round
                        // (apply_opponent_build already added them to opponent_packs,
                        //  respawn_opponent_units covers all stored packs including new ones)
                        // So we don't need to extend with opp_units separately.
                        let _ = opp_units;

                        projectiles.clear();

                        // Generate terrain once per match; subsequent rounds just reset cover HP
                        if ctx.obstacles.is_empty() && ctx.game_settings.terrain_enabled {
                            ctx.obstacles = terrain::generate_terrain(ctx.progress.round, ctx.game_settings.terrain_destructible);
                        } else {
                            terrain::reset_cover_hp(&mut ctx.obstacles);
                        }
                        ctx.nav_grid = Some(terrain::NavGrid::from_obstacles(&ctx.obstacles, ARENA_W, ARENA_H, 15.0));

                        // Seed RNG for deterministic battle
                        macroquad::rand::srand(ctx.progress.round as u64);
                        battle_accumulator = 0.0;
                        battle_timer = 0.0;
                        battle_frame = 0;
                        recent_hashes.clear();

                        // Reset per-round damage stats
                        for unit in ctx.units.iter_mut() {
                            unit.damage_dealt_round = 0.0;
                            unit.damage_soaked_round = 0.0;
                        }

                        ctx.phase = GamePhase::Battle;
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
                if let Some(ref mut n) = ctx.net {
                    n.poll();
                }

                if show_surrender_confirm {
                    // Battle paused while surrender overlay is shown
                } else if ctx.net.is_some() {
                    // Multiplayer: fixed timestep for determinism
                    battle_accumulator += dt;
                    while battle_accumulator >= FIXED_DT {
                        battle_accumulator -= FIXED_DT;
                        update_targeting(&mut ctx.units, &ctx.obstacles);
                        update_movement(&mut ctx.units, FIXED_DT, ARENA_W, ARENA_H, &ctx.obstacles, ctx.nav_grid.as_ref());
                        update_attacks(
                            &mut ctx.units,
                            &mut projectiles,
                            FIXED_DT,
                            &ctx.progress.player_techs,
                            &ctx.progress.opponent_techs,
                            &mut splash_effects,
                        );
                        update_projectiles(&mut projectiles, &mut ctx.units, FIXED_DT, &mut ctx.obstacles, &mut splash_effects);
                        // Death animation timers (inside fixed timestep for determinism)
                        for unit in ctx.units.iter_mut() {
                            if !unit.alive && unit.death_timer > 0.0 {
                                unit.death_timer -= FIXED_DT;
                            }
                        }
                        battle_frame += 1;

                        // --- Sync hashing every SYNC_INTERVAL frames ---
                        if let Some(ref mut n) = ctx.net {
                            if battle_frame % SYNC_INTERVAL == 0 {
                                if n.is_host {
                                    let local_hash = sync::compute_state_hash(&ctx.units, &projectiles, &ctx.obstacles, false);
                                    n.send(net::NetMessage::StateHash { frame: battle_frame, hash: local_hash });
                                } else {
                                    // Guest: store hash for this frame so we can compare when host's hash arrives
                                    let local_hash = sync::compute_state_hash(&ctx.units, &projectiles, &ctx.obstacles, true);
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
                    if let Some(ref mut n) = ctx.net {
                        n.poll();

                        if n.is_host {
                            // Host: respond to state request from guest
                            if let Some(_req_frame) = n.received_state_request.take() {
                                let (units_data, projectiles_data, obstacles_data) =
                                    sync::serialize_state(&ctx.units, &projectiles, &ctx.obstacles);
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
                                    &mut ctx.units,
                                    &mut projectiles,
                                    &mut ctx.obstacles,
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
                    update_targeting(&mut ctx.units, &ctx.obstacles);
                    update_movement(&mut ctx.units, dt, ARENA_W, ARENA_H, &ctx.obstacles, ctx.nav_grid.as_ref());
                    update_attacks(
                        &mut ctx.units,
                        &mut projectiles,
                        dt,
                        &ctx.progress.player_techs,
                        &ctx.progress.opponent_techs,
                        &mut splash_effects,
                    );
                    update_projectiles(&mut projectiles, &mut ctx.units, dt, &mut ctx.obstacles, &mut splash_effects);
                    // Death animation timers
                    for unit in ctx.units.iter_mut() {
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
                        ctx.progress.player_lp = 0;
                        show_surrender_confirm = false;
                        ctx.phase = GamePhase::GameOver(1);
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

                let state = check_match_state(&ctx.units);
                let is_multiplayer = ctx.net.is_some();
                let is_host_game = ctx.net.as_ref().map_or(true, |n| n.is_host);
                let battle_ended = (state != MatchState::InProgress && projectiles.is_empty()) || timed_out;

                // Guest waiting for host's authoritative round result
                if waiting_for_round_end {
                    round_end_timeout -= dt;
                    if let Some(ref mut n) = ctx.net {
                        if let Some(rd) = n.received_round_end.take() {
                            // Flip host's perspective to guest's: host team 0 = guest team 1
                            let flipped_winner = rd.winner.map(|w| 1 - w);
                            let flipped_loser = rd.loser_team.map(|l| 1 - l);

                            let final_state = match flipped_winner {
                                Some(w) => MatchState::Winner(w),
                                None => MatchState::Draw,
                            };

                            // Log desync check (flip host counts to match guest perspective)
                            let local_alive_0 = ctx.units.iter().filter(|u| u.alive && u.team_id == 0).count() as u16;
                            let local_alive_1 = ctx.units.iter().filter(|u| u.alive && u.team_id == 1).count() as u16;
                            if local_alive_0 != rd.alive_1 || local_alive_1 != rd.alive_0 {
                                eprintln!("[DESYNC] Unit count mismatch! Local: {}/{} Host(flipped): {}/{}", local_alive_0, local_alive_1, rd.alive_1, rd.alive_0);
                            }

                            // Apply timeout mutual damage (flipped for guest perspective)
                            if rd.timeout_dmg_0 > 0 || rd.timeout_dmg_1 > 0 {
                                // Host's team 0 = guest's team 1, so flip
                                ctx.progress.player_lp -= rd.timeout_dmg_1;
                                ctx.progress.opponent_lp -= rd.timeout_dmg_0;
                            } else if let Some(loser) = flipped_loser {
                                if loser == 0 {
                                    ctx.progress.player_lp -= rd.lp_damage;
                                } else {
                                    ctx.progress.opponent_lp -= rd.lp_damage;
                                }
                            }

                            waiting_for_round_end = false;
                            show_surrender_confirm = false;
                            ctx.phase = GamePhase::RoundResult {
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
                    let final_state = if timed_out { MatchState::Draw } else { check_match_state(&ctx.units) };

                    // Record AI memory for counter-picking
                    let ai_won = match &final_state {
                        MatchState::Winner(w) => *w == 1,
                        _ => false,
                    };
                    ctx.progress.ai_memory.record_round(&ctx.units, ai_won);

                    // Calculate LP damage
                    let alive_0 = ctx.units.iter().filter(|u| u.alive && u.team_id == 0).count() as i32;
                    let alive_1 = ctx.units.iter().filter(|u| u.alive && u.team_id == 1).count() as i32;

                    // Compute damage and loser — but DON'T apply yet (guest needs
                    // the same values from the network message).
                    let (lp_damage, loser_team, timeout_dmg_0, timeout_dmg_1) = if timed_out {
                        // Timeout: both players take damage equal to opponent's surviving ctx.units
                        (0, None, alive_1, alive_0)
                    } else {
                        match &final_state {
                            MatchState::Winner(winner) => {
                                let damage = MatchProgress::calculate_lp_damage(&ctx.units, *winner);
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
                            let total_hp_0: i32 = ctx.units.iter().filter(|u| u.alive && u.team_id == 0).map(|u| u.hp as i32).sum();
                            let total_hp_1: i32 = ctx.units.iter().filter(|u| u.alive && u.team_id == 1).map(|u| u.hp as i32).sum();
                            let winner = match &final_state {
                                MatchState::Winner(w) => Some(*w),
                                _ => None,
                            };
                            if let Some(ref mut n) = ctx.net {
                                n.send(net::NetMessage::RoundEnd {
                                    winner, lp_damage, loser_team,
                                    alive_0, alive_1, total_hp_0, total_hp_1,
                                    timeout_dmg_0, timeout_dmg_1,
                                });
                            }
                        }

                        // Apply LP damage
                        if timed_out {
                            ctx.progress.player_lp -= timeout_dmg_0;
                            ctx.progress.opponent_lp -= timeout_dmg_1;
                        } else if let Some(loser) = loser_team {
                            if loser == 0 {
                                ctx.progress.player_lp -= lp_damage;
                            } else {
                                ctx.progress.opponent_lp -= lp_damage;
                            }
                        }

                        show_surrender_confirm = false;
                        ctx.phase = GamePhase::RoundResult {
                            match_state: final_state,
                            lp_damage,
                            loser_team,
                        };
                    }
                }
            }

            GamePhase::RoundResult { .. } => {
                // Poll network
                if let Some(ref mut n) = ctx.net {
                    n.poll();
                }

                if is_key_pressed(KeyCode::Space) {
                    if ctx.progress.is_game_over() {
                        ctx.phase = GamePhase::GameOver(ctx.progress.game_winner().unwrap_or(0));
                    } else {
                        // Save leftover gold for next round
                        ctx.progress.player_saved_gold = ctx.build.builder.gold_remaining;

                        // Advance to next round
                        ctx.progress.advance_round();

                        // Lock all current player packs
                        ctx.build.lock_current_packs();
                        let locked_packs: Vec<_> = ctx.build.placed_packs.clone();
                        let next_id = ctx.build.next_id;

                        // Save accumulated stats before clearing
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

                        // Clear ctx.units and respawn all from locked packs
                        ctx.units.clear();
                        ctx.build = BuildState::new_round(ctx.progress.round_gold(), locked_packs, next_id);

                        // Respawn all locked PLAYER pack ctx.units
                        ctx.units.extend(ctx.build.respawn_player_units(&ctx.progress.player_techs));

                        // Restore accumulated stats on respawned ctx.units
                        for unit in ctx.units.iter_mut() {
                            if let Some(&(ddt, dst, ddr, dsr, kt)) = old_stats.get(&unit.id) {
                                unit.damage_dealt_total = ddt;
                                unit.damage_soaked_total = dst;
                                unit.damage_dealt_round = ddr;
                                unit.damage_soaked_round = dsr;
                                unit.kills_total = kt;
                            }
                        }

                        // Respawn opponent ctx.units from stored packs (visible during ctx.build ctx.phase).
                        // Works for both single-player (AI packs) and multiplayer (network packs).
                        ctx.units.extend(ctx.progress.respawn_opponent_units());

                        projectiles.clear();
                        ctx.phase = GamePhase::Build;
                    }
                }
            }

            GamePhase::GameOver(_) => {
                if is_key_pressed(KeyCode::R) {
                    ctx.progress = MatchProgress::new(true);
                    ctx.phase = GamePhase::Lobby;
                    ctx.build = BuildState::new(ctx.progress.round_gold(), true);
                    ctx.units.clear();
                    projectiles.clear();
                    ctx.net = None;
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
                    let is_host = ctx.net.as_ref().map_or(true, |n| n.is_host);
                    ctx.progress = MatchProgress::new(is_host);
                    ctx.build = BuildState::new(ctx.progress.round_gold(), is_host);
                    ctx.units.clear();
                    projectiles.clear();
                    ctx.obstacles.clear();
                    ctx.nav_grid = None;
                    show_surrender_confirm = false;
                    ctx.chat = chat::ChatState::new();
                    splash_effects.clear();
                    waiting_for_round_end = false;
                    ctx.phase = if ctx.game_settings.draft_ban_enabled {
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

        // Skip normal rendering for Lobby ctx.phase (it draws its own UI above)
        if matches!(ctx.phase, GamePhase::Lobby) {
            next_frame().await;
            continue;
        }

        // Always use Camera2D for world-space rendering
        set_camera(&arena_camera);

        rendering::draw_world(
            &ctx.units, &projectiles, &ctx.obstacles, &splash_effects,
            &ctx.build, &ctx.progress, ctx.show_grid,
            matches!(ctx.phase, GamePhase::Build),
            world_mouse,
        );

        // Reset camera for UI overlays (screen-space)
        set_default_camera();

        // === Phase-specific UI (screen-space) ===
        match &ctx.phase {
            GamePhase::Lobby | GamePhase::DraftBan { .. } => {}

            GamePhase::Build => {
                phase_ui::draw_build_ui(&ctx.build, &ctx.progress, &ctx.units, screen_mouse, &arena_camera, &ctx.mp_player_name, &ctx.mp_opponent_name);
            }

            GamePhase::WaitingForOpponent => {
                phase_ui::draw_waiting_ui(&ctx.progress, &ctx.build, &ctx.mp_player_name, &ctx.mp_opponent_name);
            }

            GamePhase::Battle => {
                phase_ui::draw_battle_ui(&ctx.progress, &ctx.units, &ctx.obstacles, battle_timer, ROUND_TIMEOUT, show_surrender_confirm, screen_mouse, world_mouse, &ctx.mp_player_name, &ctx.mp_opponent_name);
            }

            GamePhase::RoundResult { match_state, lp_damage, loser_team } => {
                phase_ui::draw_round_result_ui(&ctx.progress, match_state, *lp_damage, *loser_team, &ctx.game_settings, &ctx.net, &ctx.mp_player_name, &ctx.mp_opponent_name);
            }

            GamePhase::GameOver(winner) => {
                phase_ui::draw_game_over_ui(*winner, &ctx.progress, &ctx.units, &ctx.game_settings, &ctx.net, screen_mouse, &ctx.mp_player_name, &ctx.mp_opponent_name);
            }
        }


        // Disconnection overlay (shown over any ctx.phase if ctx.net is disconnected)
        if let Some(ref n) = ctx.net {
            if n.disconnected {
                phase_ui::draw_disconnect_overlay();
                if is_key_pressed(KeyCode::R) {
                    ctx.progress = MatchProgress::new(true);
                    ctx.phase = GamePhase::Lobby;
                    ctx.build = BuildState::new(ctx.progress.round_gold(), true);
                    ctx.units.clear();
                    projectiles.clear();
                    ctx.net = None;
                    lobby.reset();
                }
            }
        }

        // Chat system
        ctx.chat.receive_from_net(&mut ctx.net);
        ctx.chat.update(&ctx.phase, &mut ctx.net, &ctx.mp_player_name);
        ctx.chat.tick(dt);
        ctx.chat.draw(&ctx.phase, &ctx.mp_player_name);

        next_frame().await;
    }
}

