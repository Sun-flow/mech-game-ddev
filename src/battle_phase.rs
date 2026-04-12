use macroquad::prelude::*;

use crate::arena::{check_match_state, MatchState, ARENA_H, ARENA_W};
use crate::combat::run_one_frame;
use crate::context::GameContext;
use crate::game_state::GamePhase;
use crate::match_progress::MatchProgress;
use crate::net;
use crate::projectile::Projectile;
use crate::rendering::SplashEffect;
use crate::sync;

pub const FIXED_DT: f32 = 1.0 / 60.0;
pub const ROUND_TIMEOUT: f32 = 90.0;

/// Number of recent hashes each peer keeps for comparison against incoming
/// peer hashes. Must be larger than round-trip latency in frames — 64 frames
/// is ~1 second, comfortably more than any reasonable network RTT.
pub const HASH_HISTORY_LEN: usize = 64;

/// Debounce window for host state sends. After sending a state correction,
/// the host won't send another for this many frames — enough for the guest
/// to apply the correction and for its post-correction hashes to arrive
/// back at the host (approximately 2 * round-trip latency).
///
/// 12 frames ≈ 200 ms, which covers typical good-internet RTT (50-100 ms
/// each way) with margin. If real latency exceeds this, the worst case is
/// redundant state sends, not broken recovery — the protocol still converges.
pub const STATE_SEND_DEBOUNCE_FRAMES: u32 = 12;

/// For diagnostics: dump per-frame PlayerState, unit data, and network events
/// for the first N frames of each battle round. Set to 0 to disable entirely.
pub const DEBUG_DUMP_FRAMES: u32 = 30;

/// Dump the full game state for one frame, tagged with frame number and role.
/// Output is on stderr so it interleaves with normal `[DESYNC]` / `[SYNC]` logs.
fn debug_dump_frame(
    frame: u32,
    local_player_id: u16,
    is_host: bool,
    players: &[crate::match_progress::PlayerState],
    units: &[crate::unit::Unit],
    projectiles: &[Projectile],
    local_hash: u64,
) {
    let role = if is_host { "HOST" } else { "GUEST" };
    eprintln!(
        "[DUMP {} pid={} f={}] hash={:016x} units={} projs={}",
        role,
        local_player_id,
        frame,
        local_hash,
        units.len(),
        projectiles.len()
    );

    for pl in players {
        let unit_count = units.iter().filter(|u| u.player_id == pl.player_id).count();
        let alive = units
            .iter()
            .filter(|u| u.player_id == pl.player_id && u.alive)
            .count();
        let tech_count: usize = pl.techs.purchased.values().map(|v| v.len()).sum();
        eprintln!(
            "  player {} name={:?} packs={} next_id={} techs={} units={}({} alive)",
            pl.player_id, pl.name, pl.packs.len(), pl.next_id, tech_count, unit_count, alive
        );
        for (i, p) in pl.packs.iter().enumerate() {
            let kind = crate::pack::all_packs()
                .get(p.pack_index)
                .map(|pd| format!("{:?}", pd.kind))
                .unwrap_or_else(|| format!("idx={}", p.pack_index));
            eprintln!(
                "    pack[{}] kind={} center=({:.0},{:.0}) locked={} rotated={} round_placed={} unit_ids={:?}",
                i, kind, p.center.x, p.center.y, p.locked, p.rotated, p.round_placed, p.unit_ids
            );
        }
    }

    // Sorted unit dump (canonical order — matches the hash computation)
    let mut sorted: Vec<&crate::unit::Unit> = units.iter().collect();
    sorted.sort_by_key(|u| u.id);
    for u in &sorted {
        eprintln!(
            "    unit id={} kind={:?} pid={} pos=({:.1},{:.1}) hp={:.1}/{:.1} alive={} target={:?}",
            u.id, u.kind, u.player_id, u.pos.x, u.pos.y, u.hp, u.stats.max_hp, u.alive, u.target_id
        );
    }
}

pub struct BattleState {
    pub accumulator: f32,
    pub timer: f32,
    pub frame: u32,
    /// Local sliding window of (frame, hash) pairs for the most recent
    /// HASH_HISTORY_LEN frames. Used to compare against incoming peer
    /// hashes. Populated on both host and guest.
    pub recent_hashes: std::collections::VecDeque<(u32, u64)>,
    /// Frame of the last state send (host only). Used for debounce to
    /// prevent flooding corrections during a single drift event.
    pub last_state_send_frame: Option<u32>,
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
            recent_hashes: std::collections::VecDeque::with_capacity(HASH_HISTORY_LEN),
            last_state_send_frame: None,
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
        self.last_state_send_frame = None;
        self.waiting_for_round_end = false;
        self.round_end_timeout = 0.0;
        self.projectiles.clear();
        self.splash_effects.clear();
    }
}

pub fn update(ctx: &mut GameContext, battle: &mut BattleState, _ms: &crate::input::MouseState, dt: f32) {
    // Poll network
    if let Some(ref mut n) = ctx.net {
        n.poll();
    }

    if ctx.show_escape_menu && ctx.net.is_none() {
        // Single-player: pause simulation while escape menu is open
    } else if ctx.net.is_some() {
        // Multiplayer: fixed timestep for determinism
        battle.accumulator += dt;
        while battle.accumulator >= FIXED_DT {
            battle.accumulator -= FIXED_DT;
            run_one_frame(
                &mut ctx.units,
                &mut battle.projectiles,
                &mut ctx.obstacles,
                ctx.nav_grid.as_ref(),
                &mut ctx.progress.players,
                &mut battle.splash_effects,
                FIXED_DT,
                ARENA_W,
                ARENA_H,
            );
            battle.frame += 1;

            // --- Hash every frame, store locally, send to peer ---
            let local_hash = sync::compute_state_hash(&ctx.units, &battle.projectiles, &ctx.obstacles);
            battle.recent_hashes.push_back((battle.frame, local_hash));
            while battle.recent_hashes.len() > HASH_HISTORY_LEN {
                battle.recent_hashes.pop_front();
            }
            let is_host = ctx.net.as_ref().is_none_or(|n| n.is_host);
            if let Some(ref mut n) = ctx.net {
                n.send(net::NetMessage::StateHash { frame: battle.frame, hash: local_hash });
            }

            // --- Diagnostic per-frame dump (first DEBUG_DUMP_FRAMES frames) ---
            if DEBUG_DUMP_FRAMES > 0 && battle.frame <= DEBUG_DUMP_FRAMES {
                debug_dump_frame(
                    battle.frame,
                    ctx.local_player_id,
                    is_host,
                    &ctx.progress.players,
                    &ctx.units,
                    &battle.projectiles,
                    local_hash,
                );
            }
        }

        // --- Process incoming sync messages (outside fixed-timestep loop) ---
        if let Some(ref mut n) = ctx.net {
            n.poll();

            // Drain all incoming peer hashes. Both host and guest do this.
            // - Host: on mismatch, proactively pushes state to guest (debounced).
            // - Guest: compares to its own hashes only for logging; host is
            //   authoritative and will correct.
            let peer_hashes: Vec<(u32, u64)> = std::mem::take(&mut n.received_state_hashes);
            let has_state_sync = n.received_state_sync.is_some();
            if DEBUG_DUMP_FRAMES > 0 && battle.frame <= DEBUG_DUMP_FRAMES + 5 && (!peer_hashes.is_empty() || has_state_sync) {
                let role = if n.is_host { "HOST" } else { "GUEST" };
                eprintln!(
                    "[NET {} f={}] rx_hashes={} rx_state_sync={}",
                    role,
                    battle.frame,
                    peer_hashes.len(),
                    has_state_sync
                );
                for (f, h) in &peer_hashes {
                    let local_h = battle.recent_hashes.iter().find(|(lf, _)| *lf == *f).map(|(_, lh)| *lh);
                    eprintln!(
                        "  peer_hash f={} peer={:016x} local={}",
                        f,
                        h,
                        match local_h {
                            Some(lh) => format!("{:016x} match={}", lh, lh == *h),
                            None => "<missing>".to_string(),
                        }
                    );
                }
            }
            let mut detected_mismatch_frame: Option<u32> = None;
            for (peer_frame, peer_hash) in peer_hashes {
                let local_hash = battle.recent_hashes
                    .iter()
                    .find(|(f, _)| *f == peer_frame)
                    .map(|(_, h)| *h);
                if let Some(lh) = local_hash {
                    if lh != peer_hash && detected_mismatch_frame.is_none() {
                        eprintln!(
                            "[DESYNC] {} detected hash mismatch at peer frame {} (local frame {})",
                            if n.is_host { "Host" } else { "Guest" },
                            peer_frame,
                            battle.frame,
                        );
                        detected_mismatch_frame = Some(peer_frame);
                    }
                }
            }

            // Host-authoritative: if host detected a mismatch, proactively push state.
            if n.is_host && detected_mismatch_frame.is_some() {
                let debounce_ok = battle.last_state_send_frame.is_none_or(|last| {
                    battle.frame.saturating_sub(last) >= STATE_SEND_DEBOUNCE_FRAMES
                });
                if debounce_ok {
                    let (units_data, projectiles_data, obstacles_data) =
                        sync::serialize_state(&ctx.units, &battle.projectiles, &ctx.obstacles);
                    eprintln!(
                        "[SYNC] Host pushing state correction at frame {} ({} + {} + {} bytes)",
                        battle.frame,
                        units_data.len(),
                        projectiles_data.len(),
                        obstacles_data.len()
                    );
                    n.send(net::NetMessage::StateSync {
                        frame: battle.frame,
                        units_data,
                        projectiles_data,
                        obstacles_data,
                    });
                    battle.last_state_send_frame = Some(battle.frame);
                }
            }

            // Guest applies incoming state correction with rollback + replay.
            // Host never applies corrections from guest (Phase 1 host-authoritative).
            if !n.is_host {
                if let Some(sync_data) = n.received_state_sync.take() {
                    let before_frame = battle.frame;
                    eprintln!(
                        "[SYNC] Guest applying host correction: snapshot_frame={} local_frame={}",
                        sync_data.frame, before_frame
                    );
                    match sync::apply_and_fast_forward(
                        sync_data.frame,
                        &mut battle.frame,
                        &mut ctx.units,
                        &mut battle.projectiles,
                        &mut ctx.obstacles,
                        ctx.nav_grid.as_ref(),
                        &mut ctx.progress.players,
                        &mut battle.splash_effects,
                        &sync_data.units_data,
                        &sync_data.projectiles_data,
                        &sync_data.obstacles_data,
                        FIXED_DT,
                        ARENA_W,
                        ARENA_H,
                    ) {
                        Ok(frames_replayed) => {
                            eprintln!(
                                "[SYNC] Guest rollback+replay complete: replayed {} frames ({}→{})",
                                frames_replayed, sync_data.frame, battle.frame
                            );
                            // Our recent hashes are stale post-rollback; clear them so
                            // future comparisons use fresh post-correction hashes.
                            battle.recent_hashes.clear();
                        }
                        Err(e) => {
                            eprintln!("[SYNC] Guest failed to apply correction: {}", e);
                        }
                    }
                }
            }
        }
    } else {
        // Single-player: variable timestep (original behavior)
        run_one_frame(
            &mut ctx.units,
            &mut battle.projectiles,
            &mut ctx.obstacles,
            ctx.nav_grid.as_ref(),
            &mut ctx.progress.players,
            &mut battle.splash_effects,
            dt,
            ARENA_W,
            ARENA_H,
        );
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

            ctx.phase = GamePhase::RoundResult {
                match_state: final_state,
                lp_damage,
                loser_team,
            };
        }
    }
}
