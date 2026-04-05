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

        if let Some(opp_build) = n.take_opponent_build() {
            let opp_units = ctx.progress.apply_opponent_build(&opp_build);

            ctx.units.retain(|u| u.player_id == 0);
            ctx.units.extend(ctx.progress.respawn_opponent_units());

            let _ = opp_units;

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
            return true; // caller should continue (skip rendering)
        }
    }
    false
}
