mod arena;
mod battle_phase;
mod build_phase;
mod chat;
mod context;
mod draft_ban;
mod combat;
mod economy;
mod game_over;
mod game_state;
mod input;
mod lobby;
pub mod ui;
mod match_progress;
mod net;
mod pack;
mod projectile;
mod rendering;
mod role;
mod round_result;
mod settings;
mod shop;
mod team;
mod tech;
mod tech_ui;
mod terrain;
mod unit;
mod sync;
mod phase_ui;
mod waiting_phase;

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
    let mut ctx = context::GameContext::new();
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
        // Set colors canonically: my color goes to my player_id slot
        team::set_color(ctx.role.player_id(), ctx.game_settings.player_color_index);
        if let Some(ref n) = ctx.net {
            if let Some(opp_color) = n.opponent_color {
                team::set_color(ctx.role.opponent_id(), opp_color);
            }
        }
        ui::set_text_scale(main_settings.ui_scale);

        // Build the arena camera (used for all world-space rendering)
        let x_flip = if ctx.role == role::Role::Guest { -1.0 } else { 1.0 };
        let arena_camera = Camera2D {
            target: camera_target,
            zoom: vec2(camera_zoom * 2.0 / screen_width() * x_flip, camera_zoom * 2.0 / screen_height()),
            ..Default::default()
        };
        let world_mouse = arena_camera.screen_to_world(screen_mouse);
        let mouse = input::MouseState {
            screen_mouse,
            world_mouse,
            left_click,
            right_click,
            middle_click: is_mouse_button_pressed(MouseButton::Middle),
            left_down: is_mouse_button_down(MouseButton::Left),
            middle_down: is_mouse_button_down(MouseButton::Middle),
            scroll: mouse_wheel().1,
        };

        // Fullscreen toggle (F11)
        if is_key_pressed(KeyCode::F11) {
            is_fullscreen_mode = !is_fullscreen_mode;
            set_fullscreen(is_fullscreen_mode);
        }

        // Camera zoom/pan (available in all non-lobby phases)
        if !matches!(ctx.phase, GamePhase::Lobby) {
            // Smooth multiplicative zoom — ~100+ steps between min/max
            if mouse.scroll != 0.0 {
                let zoom_factor = 1.0 + mouse.scroll.signum() * 0.04; // ~4% per scroll tick
                camera_zoom = (camera_zoom * zoom_factor).clamp(0.3, 5.0);
            }
            // "Grab the ground" pan: pin a world point to the cursor
            if mouse.middle_down {
                if pan_grab_world.is_none() {
                    // On drag start, record the world point under the cursor
                    pan_grab_world = Some(arena_camera.screen_to_world(mouse.screen_mouse));
                }
                if let Some(grab_pt) = pan_grab_world {
                    // Where is the cursor pointing now in world coords?
                    let current_world = arena_camera.screen_to_world(mouse.screen_mouse);
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
                match lobby.update(&mut ctx.game_settings, &mut main_settings, &mouse) {
                    lobby::LobbyResult::StartMultiplayer => {
                        let is_host = lobby.is_room_creator;
                        ctx.start_game(lobby.net.take(), is_host, lobby.player_name.clone(), ctx.game_settings.draft_ban_enabled);
                        continue;
                    }
                    lobby::LobbyResult::StartVsAi => {
                        ctx.start_game(None, true, lobby.player_name.clone(), ctx.game_settings.draft_ban_enabled);
                        ctx.progress.guest.name = "AI".to_string();
                        continue;
                    }
                    lobby::LobbyResult::Waiting => {}
                }

                match lobby.draw(&mut ctx.game_settings, &mut main_settings, &mouse) {
                    lobby::LobbyResult::StartMultiplayer => {
                        let is_host = lobby.is_room_creator;
                        ctx.start_game(lobby.net.take(), is_host, lobby.player_name.clone(), ctx.game_settings.draft_ban_enabled);
                        continue;
                    }
                    lobby::LobbyResult::StartVsAi => {
                        ctx.start_game(None, true, lobby.player_name.clone(), ctx.game_settings.draft_ban_enabled);
                        ctx.progress.guest.name = "AI".to_string();
                        continue;
                    }
                    _ => {}
                }

                next_frame().await;
                continue;
            }

            GamePhase::DraftBan { ref mut bans, ref mut confirmed, ref mut opponent_bans } => {
                match draft_ban::update_and_draw(bans, confirmed, opponent_bans, &mut ctx.net, mouse.screen_mouse, mouse.left_click) {
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
                build_phase::update(&mut ctx, &mut battle, &mouse, dt);
            }

            GamePhase::WaitingForOpponent => {
                if waiting_phase::update(&mut ctx, &mut battle) {
                    continue;
                }
            }

            GamePhase::Battle => {
                battle_phase::update(&mut ctx, &mut battle, &mouse, dt);
            }

            GamePhase::RoundResult { .. } => {
                round_result::update(&mut ctx, &mut battle);
            }

            GamePhase::GameOver(_) => {
                game_over::update(&mut ctx, &mut battle, &mut lobby, mouse.screen_mouse, mouse.left_click);
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

        let is_build = matches!(ctx.phase, GamePhase::Build);
        rendering::draw_world(
            &ctx.units, &battle.projectiles, &ctx.obstacles, &battle.splash_effects,
            ctx.show_grid && is_build,
        );

        if is_build {
            rendering::draw_build_overlays(&ctx.build, &ctx.progress, mouse.world_mouse, ctx.role);
        }

        // Reset camera for UI overlays (screen-space)
        set_default_camera();

        // === Phase-specific UI (screen-space) ===
        match &ctx.phase {
            GamePhase::Lobby | GamePhase::DraftBan { .. } => {}

            GamePhase::Build => {
                phase_ui::draw_build_ui(&ctx.build, &ctx.progress, &ctx.units, mouse.screen_mouse, &arena_camera, ctx.role);
            }

            GamePhase::WaitingForOpponent => {
                phase_ui::draw_waiting_ui(&ctx.progress, &ctx.build, ctx.role);
            }

            GamePhase::Battle => {
                phase_ui::draw_battle_ui(&ctx.progress, &ctx.units, &ctx.obstacles, battle.timer, battle_phase::ROUND_TIMEOUT, battle.show_surrender_confirm, mouse.screen_mouse, mouse.world_mouse, ctx.role);
            }

            GamePhase::RoundResult { match_state, lp_damage, loser_team } => {
                phase_ui::draw_round_result_ui(&ctx.progress, match_state, *lp_damage, *loser_team, ctx.role);
            }

            GamePhase::GameOver(winner) => {
                phase_ui::draw_game_over_ui(*winner, &ctx.progress, &ctx.units, mouse.screen_mouse, ctx.role);
            }
        }


        // Disconnection overlay (shown over any ctx.phase if ctx.net is disconnected)
        if let Some(ref n) = ctx.net {
            if n.disconnected {
                phase_ui::draw_disconnect_overlay();
                if is_key_pressed(KeyCode::R) {
                    ctx.progress = MatchProgress::new();
                    ctx.phase = GamePhase::Lobby;
                    ctx.build = BuildState::new(ctx.progress.round_allowance(), true);
                    ctx.units.clear();
                    battle.projectiles.clear();
                    ctx.net = None;
                    lobby.reset();
                }
            }
        }

        // Chat system
        ctx.chat.receive_from_net(&mut ctx.net, ctx.role.opponent_id());
        let my_name = ctx.progress.player(ctx.role).name.clone();
        ctx.chat.update(&ctx.phase, &mut ctx.net, &my_name, ctx.role.player_id());
        ctx.chat.tick(dt);
        ctx.chat.draw(&ctx.phase, &my_name);

        next_frame().await;
    }
}
