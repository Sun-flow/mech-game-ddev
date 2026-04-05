use macroquad::prelude::*;

use crate::arena::{check_match_state, MatchState, ARENA_H, ARENA_W};
use crate::combat::{update_attacks, update_movement, update_projectiles, update_targeting};
use crate::context::GameContext;
use crate::game_state::GamePhase;
use crate::match_progress::MatchProgress;
use crate::net;
use crate::projectile::Projectile;
use crate::rendering::SplashEffect;
use crate::sync;

pub const FIXED_DT: f32 = 1.0 / 60.0;
pub const ROUND_TIMEOUT: f32 = 90.0;
pub const SYNC_INTERVAL: u32 = 4;

pub struct BattleState {
    pub accumulator: f32,
    pub timer: f32,
    pub frame: u32,
    pub recent_hashes: std::collections::VecDeque<(u32, u64)>,
    pub show_surrender_confirm: bool,
    pub waiting_for_round_end: bool,
    pub round_end_timeout: f32,
    pub projectiles: Vec<Projectile>,
    pub splash_effects: Vec<SplashEffect>,
}

impl BattleState {
    pub fn new() -> Self {
        Self {
            accumulator: 0.0,
            timer: 0.0,
            frame: 0,
            recent_hashes: std::collections::VecDeque::with_capacity(5),
            show_surrender_confirm: false,
            waiting_for_round_end: false,
            round_end_timeout: 0.0,
            projectiles: Vec::new(),
            splash_effects: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.accumulator = 0.0;
        self.timer = 0.0;
        self.frame = 0;
        self.recent_hashes.clear();
        self.show_surrender_confirm = false;
        self.waiting_for_round_end = false;
        self.round_end_timeout = 0.0;
        self.projectiles.clear();
        self.splash_effects.clear();
    }
}

pub fn update(ctx: &mut GameContext, battle: &mut BattleState, ms: &crate::input::MouseState, dt: f32) {
    let screen_mouse = ms.screen_mouse;
    // Surrender toggle
    if is_key_pressed(KeyCode::Escape) {
        battle.show_surrender_confirm = !battle.show_surrender_confirm;
    }

    // Poll network
    if let Some(ref mut n) = ctx.net {
        n.poll();
    }

    if battle.show_surrender_confirm {
        // Battle paused while surrender overlay is shown
    } else if ctx.net.is_some() {
        // Multiplayer: fixed timestep for determinism
        battle.accumulator += dt;
        while battle.accumulator >= FIXED_DT {
            battle.accumulator -= FIXED_DT;
            update_targeting(&mut ctx.units, &ctx.obstacles);
            update_movement(&mut ctx.units, FIXED_DT, ARENA_W, ARENA_H, &ctx.obstacles, ctx.nav_grid.as_ref());
            update_attacks(
                &mut ctx.units,
                &mut battle.projectiles,
                FIXED_DT,
                &ctx.progress.player_techs,
                &ctx.progress.opponent_techs,
                &mut battle.splash_effects,
            );
            update_projectiles(&mut battle.projectiles, &mut ctx.units, FIXED_DT, &mut ctx.obstacles, &mut battle.splash_effects);
            // Death animation timers (inside fixed timestep for determinism)
            for unit in ctx.units.iter_mut() {
                if !unit.alive && unit.death_timer > 0.0 {
                    unit.death_timer -= FIXED_DT;
                }
            }
            battle.frame += 1;

            // --- Sync hashing every SYNC_INTERVAL frames ---
            if let Some(ref mut n) = ctx.net {
                if battle.frame.is_multiple_of(SYNC_INTERVAL) {
                    if n.is_host {
                        let local_hash = sync::compute_state_hash(&ctx.units, &battle.projectiles, &ctx.obstacles, false);
                        n.send(net::NetMessage::StateHash { frame: battle.frame, hash: local_hash });
                    } else {
                        // Guest: store hash for this frame so we can compare when host's hash arrives
                        let local_hash = sync::compute_state_hash(&ctx.units, &battle.projectiles, &ctx.obstacles, true);
                        if battle.recent_hashes.len() >= 4 {
                            battle.recent_hashes.pop_front();
                        }
                        battle.recent_hashes.push_back((battle.frame, local_hash));
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
                        sync::serialize_state(&ctx.units, &battle.projectiles, &ctx.obstacles);
                    eprintln!("[SYNC] Host sending full state at frame {} ({} + {} + {} bytes)",
                        battle.frame, units_data.len(), projectiles_data.len(), obstacles_data.len());
                    n.send(net::NetMessage::StateSync {
                        frame: battle.frame,
                        units_data,
                        projectiles_data,
                        obstacles_data,
                    });
                }
            } else {
                // Guest: check hash from host against our stored hash for that frame
                if let Some((host_frame, host_hash)) = n.received_state_hash.take() {
                    if let Some(pos) = battle.recent_hashes.iter().position(|(f, _)| *f == host_frame) {
                        let (_, local_hash) = battle.recent_hashes[pos];
                        if host_hash != local_hash {
                            eprintln!("[DESYNC] Hash mismatch at frame {}! Requesting state.", host_frame);
                            n.send(net::NetMessage::StateRequest { frame: battle.frame });
                        }
                        // Remove this and older hashes
                        battle.recent_hashes.drain(..=pos);
                    }
                }

                // Guest: apply state correction from host immediately (mirror positions)
                if let Some(sync_data) = n.received_state_sync.take() {
                    eprintln!("[SYNC] Guest applying host state correction (host frame {}, local frame {})",
                        sync_data.frame, battle.frame);
                    sync::apply_state_sync(
                        &mut ctx.units,
                        &mut battle.projectiles,
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
            &mut battle.projectiles,
            dt,
            &ctx.progress.player_techs,
            &ctx.progress.opponent_techs,
            &mut battle.splash_effects,
        );
        update_projectiles(&mut battle.projectiles, &mut ctx.units, dt, &mut ctx.obstacles, &mut battle.splash_effects);
        // Death animation timers
        for unit in ctx.units.iter_mut() {
            if !unit.alive && unit.death_timer > 0.0 {
                unit.death_timer -= dt;
            }
        }
    }

    // Surrender confirmation handling
    if battle.show_surrender_confirm && ms.left_click {
        let btn_w = crate::ui::s(120.0);
        let btn_h = crate::ui::s(40.0);
        let cx = screen_width() / 2.0;
        let cy = screen_height() / 2.0;
        // "Yes" button
        let yes_x = cx - btn_w - crate::ui::s(10.0);
        let yes_y = cy + crate::ui::s(10.0);
        if screen_mouse.x >= yes_x && screen_mouse.x <= yes_x + btn_w && screen_mouse.y >= yes_y && screen_mouse.y <= yes_y + btn_h {
            ctx.progress.player_lp = 0;
            battle.show_surrender_confirm = false;
            ctx.phase = GamePhase::GameOver(1);
        }
        // "Cancel" button
        let no_x = cx + crate::ui::s(10.0);
        let no_y = cy + crate::ui::s(10.0);
        if screen_mouse.x >= no_x && screen_mouse.x <= no_x + btn_w && screen_mouse.y >= no_y && screen_mouse.y <= no_y + btn_h {
            battle.show_surrender_confirm = false;
        }
    }

    // Round timeout
    battle.timer += dt;
    let timed_out = battle.timer >= ROUND_TIMEOUT;

    let state = check_match_state(&ctx.units);
    let is_multiplayer = ctx.net.is_some();
    let is_host_game = ctx.net.as_ref().is_none_or(|n| n.is_host);
    let battle_ended = (state != MatchState::InProgress && battle.projectiles.is_empty()) || timed_out;

    // Guest waiting for host's authoritative round result
    if battle.waiting_for_round_end {
        battle.round_end_timeout -= dt;
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

                battle.waiting_for_round_end = false;
                battle.show_surrender_confirm = false;
                ctx.phase = GamePhase::RoundResult {
                    match_state: final_state,
                    lp_damage: rd.lp_damage,
                    loser_team: flipped_loser,
                };
            } else if battle.round_end_timeout <= 0.0 {
                // Timeout — fall back to local computation
                eprintln!("[DESYNC] Timeout waiting for host RoundEnd, using local values");
                battle.waiting_for_round_end = false;
                // Fall through to local computation below
            }
        }
    }

    if battle_ended && !battle.waiting_for_round_end {
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
            battle.waiting_for_round_end = true;
            battle.round_end_timeout = 5.0;
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

            battle.show_surrender_confirm = false;
            ctx.phase = GamePhase::RoundResult {
                match_state: final_state,
                lp_damage,
                loser_team,
            };
        }
    }
}
