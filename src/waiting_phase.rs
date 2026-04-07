use macroquad::prelude::*;

use crate::arena::{ARENA_H, ARENA_W};
use crate::battle_phase::BattleState;
use crate::context::GameContext;
use crate::game_state::GamePhase;
use crate::terrain;

/// Returns true if the caller should `continue` (skip rendering this frame).
pub fn update(ctx: &mut GameContext, battle: &mut BattleState) -> bool {
    if let Some(ref mut n) = ctx.net {
        n.poll();

        if let Some(peer_build) = n.take_peer_build() {
            let local = ctx.role.player_id() as usize;
            // TODO: 2-player assumption — derive peer index from connection identity when supporting N players
            let peer = 1 - local;
            let round = ctx.progress.round;
            let _peer_units = crate::match_progress::apply_peer_build(
                &mut ctx.progress.players[peer],
                &peer_build,
                round,
            );

            let peer_pid = ctx.progress.players[peer].player_id;
            ctx.units.retain(|u| u.player_id != peer_pid);
            ctx.units.extend(ctx.progress.players[peer].respawn_units());

            if ctx.obstacles.is_empty() && ctx.game_settings.terrain_enabled {
                ctx.obstacles = terrain::generate_terrain(ctx.progress.round, ctx.game_settings.terrain_destructible);
            } else {
                terrain::reset_cover_hp(&mut ctx.obstacles);
            }
            ctx.nav_grid = Some(terrain::NavGrid::from_obstacles(&ctx.obstacles, ARENA_W, ARENA_H, 15.0));

            macroquad::rand::srand(ctx.progress.round as u64);
            battle.reset();

            for unit in ctx.units.iter_mut() {
                unit.damage_dealt_round = 0.0;
                unit.damage_soaked_round = 0.0;
            }

            ctx.phase = GamePhase::Battle;
            return true;
        }
    }
    false
}
