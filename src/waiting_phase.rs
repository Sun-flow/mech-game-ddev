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

        if let Some(build_data) = n.take_peer_build() {
            let pid = build_data.player_id;
            let _new_units = crate::match_progress::apply_peer_build(
                &mut ctx.progress,
                &build_data,
            );

            ctx.units.retain(|u| u.player_id != pid);
            ctx.units.extend(ctx.progress.players[pid as usize].respawn_units());

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
