use macroquad::prelude::*;

use crate::battle_phase::BattleState;
use crate::context::GameContext;
use crate::game_state::{BuildState, GamePhase};

pub fn update(ctx: &mut GameContext, battle: &mut BattleState) {
    if let Some(ref mut n) = ctx.net {
        n.poll();
    }

    if is_key_pressed(KeyCode::Space) {
        if ctx.progress.is_game_over() {
            ctx.phase = GamePhase::GameOver(ctx.progress.game_winner().unwrap_or(0));
        } else {
            ctx.progress.player_saved_gold = ctx.build.builder.gold_remaining;
            ctx.progress.advance_round();

            ctx.build.lock_current_packs();
            let locked_packs: Vec<_> = ctx.build.placed_packs.clone();
            let next_id = ctx.build.next_id;

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

            ctx.units.clear();
            ctx.build = BuildState::new_round(ctx.progress.round_gold(), locked_packs, next_id);
            ctx.units.extend(ctx.build.respawn_player_units(&ctx.progress.player_techs));

            for unit in ctx.units.iter_mut() {
                if let Some(&(ddt, dst, ddr, dsr, kt)) = old_stats.get(&unit.id) {
                    unit.damage_dealt_total = ddt;
                    unit.damage_soaked_total = dst;
                    unit.damage_dealt_round = ddr;
                    unit.damage_soaked_round = dsr;
                    unit.kills_total = kt;
                }
            }

            ctx.units.extend(ctx.progress.respawn_opponent_units());

            battle.projectiles.clear();
            ctx.phase = GamePhase::Build;
        }
    }
}
