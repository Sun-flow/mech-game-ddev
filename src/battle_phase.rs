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
    let local_player_id = ctx.local_player_id;

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
                &ctx.progress.players,
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
                    let local_hash = sync::compute_state_hash(&ctx.units, &battle.projectiles, &ctx.obstacles);
                    if n.is_host {
                        n.send(net::NetMessage::StateHash { frame: battle.frame, hash: local_hash });
                    } else {
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
            &ctx.progress.players,
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
            ctx.progress.players[local_player_id as usize].lp = 0;
            battle.show_surrender_confirm = false;
            let winner = ctx.progress.game_winner().unwrap_or(0);
            ctx.phase = GamePhase::GameOver(winner);
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
                // Use canonical values directly — no flipping
                let final_state = match rd.winner {
                    Some(w) => MatchState::Winner(w),
                    None => MatchState::Draw,
                };

                // Desync check — compare per-player alive counts
                for pp in &rd.per_player {
                    let local_alive = ctx.units.iter().filter(|u| u.alive && u.player_id == pp.player_id).count() as u16;
                    if local_alive != pp.alive_count {
                        eprintln!("[DESYNC] Player {} alive mismatch! Local: {} Host: {}", pp.player_id, local_alive, pp.alive_count);
                    }
                }

                // Apply LP damage
                let has_timeout = rd.per_player.iter().any(|pp| pp.timeout_damage > 0);
                if has_timeout {
                    for pp in &rd.per_player {
                        ctx.progress.player_mut(pp.player_id).lp -= pp.timeout_damage;
                    }
                } else if let Some(loser) = rd.loser {
                    ctx.progress.player_mut(loser).lp -= rd.lp_damage;
                }

                battle.waiting_for_round_end = false;
                battle.show_surrender_confirm = false;
                ctx.phase = GamePhase::RoundResult {
                    match_state: final_state,
                    lp_damage: rd.lp_damage,
                    loser_team: rd.loser,
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
        for player in ctx.progress.players.iter_mut() {
            if player.player_id != ctx.local_player_id {
                let ai_won = match &final_state {
                    MatchState::Winner(w) => *w == player.player_id,
                    _ => false,
                };
                player.ai_memory.record_round(&ctx.units, ctx.local_player_id, ai_won);
            }
        }

        // Compute damage and loser — but DON'T apply yet (guest needs
        // the same values from the network message).
        let (lp_damage, loser_team) = if timed_out {
            (0, None)
        } else {
            match &final_state {
                MatchState::Winner(winner) => {
                    let damage = MatchProgress::calculate_lp_damage(&ctx.units, *winner);
                    let loser: u16 = ctx.progress.players.iter()
                        .find(|p| p.player_id != *winner)
                        .map(|p| p.player_id)
                        .unwrap_or(*winner);
                    (damage, Some(loser))
                }
                MatchState::Draw => (0, None),
                MatchState::InProgress => unreachable!(),
            }
        };

        // Build per-player data for the network message and LP damage application
        let per_player: Vec<net::RoundEndPlayerData> = ctx.progress.players.iter().map(|p| {
            let pid = p.player_id;
            let alive_count = ctx.units.iter().filter(|u| u.alive && u.player_id == pid).count() as u16;
            let total_hp: i32 = ctx.units.iter().filter(|u| u.alive && u.player_id == pid).map(|u| u.hp as i32).sum();
            let timeout_damage = if timed_out {
                ctx.units.iter().filter(|u| u.alive && u.player_id != pid).count() as i32
            } else {
                0
            };
            net::RoundEndPlayerData { player_id: pid, alive_count, total_hp, timeout_damage }
        }).collect();

        if is_multiplayer && !is_host_game {
            // Guest: wait for host's authoritative result
            battle.waiting_for_round_end = true;
            battle.round_end_timeout = 5.0;
        } else {
            // Host or single-player: we are authoritative
            if is_multiplayer {
                // Host sends round result to guest
                let winner = match &final_state {
                    MatchState::Winner(w) => Some(*w),
                    _ => None,
                };
                if let Some(ref mut n) = ctx.net {
                    n.send(net::NetMessage::RoundEnd {
                        winner, lp_damage, loser: loser_team, per_player: per_player.clone(),
                    });
                }
            }

            // Apply LP damage
            if timed_out {
                for pp in &per_player {
                    ctx.progress.player_mut(pp.player_id).lp -= pp.timeout_damage;
                }
            } else if let Some(loser) = loser_team {
                ctx.progress.player_mut(loser).lp -= lp_damage;
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
