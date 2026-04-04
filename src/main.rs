mod arena;
mod battle_phase;
mod build_phase;
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

use arena::{ARENA_H, ARENA_W};
use game_state::{BuildState, GamePhase};
use match_progress::MatchProgress;

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
    let mut battle = battle_phase::BattleState::new();
    let mut lobby = lobby::LobbyState::new();
    let mut main_settings = settings::MainSettings::default();
    let mut camera_zoom: f32 = 1.0;
    let mut camera_target = vec2(ARENA_W / 2.0, ARENA_H / 2.0);
    let mut is_fullscreen_mode = false;
    let mut pan_grab_world: Option<Vec2> = None; // world point pinned to cursor during drag

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
                build_phase::update(&mut ctx, &mut battle, screen_mouse, mouse, left_click, right_click, middle_click, dt);
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

                        // Generate terrain once per match; subsequent rounds just reset cover HP
                        if ctx.obstacles.is_empty() && ctx.game_settings.terrain_enabled {
                            ctx.obstacles = terrain::generate_terrain(ctx.progress.round, ctx.game_settings.terrain_destructible);
                        } else {
                            terrain::reset_cover_hp(&mut ctx.obstacles);
                        }
                        ctx.nav_grid = Some(terrain::NavGrid::from_obstacles(&ctx.obstacles, ARENA_W, ARENA_H, 15.0));

                        // Seed RNG for deterministic battle
                        macroquad::rand::srand(ctx.progress.round as u64);
                        battle.reset();

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
                battle_phase::update(&mut ctx, &mut battle, screen_mouse, dt);
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

                        battle.projectiles.clear();
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
                    battle.projectiles.clear();
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
                    ctx.obstacles.clear();
                    ctx.nav_grid = None;
                    ctx.chat = chat::ChatState::new();
                    battle.reset();
                    ctx.phase = if ctx.game_settings.draft_ban_enabled {
                        GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None }
                    } else {
                        GamePhase::Build
                    };
                }
            }
        }

        rendering::update_splash_effects(&mut battle.splash_effects, dt);

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
            &ctx.units, &battle.projectiles, &ctx.obstacles, &battle.splash_effects,
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
                phase_ui::draw_battle_ui(&ctx.progress, &ctx.units, &ctx.obstacles, battle.timer, battle_phase::ROUND_TIMEOUT, battle.show_surrender_confirm, screen_mouse, world_mouse, &ctx.mp_player_name, &ctx.mp_opponent_name);
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
                    battle.projectiles.clear();
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

