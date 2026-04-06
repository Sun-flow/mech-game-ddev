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
            let role = ctx.role;

            // Save gold carry-over
            ctx.progress.player_mut(role).gold = ctx.build.gold_remaining;

            ctx.progress.advance_round();

            // Lock current packs on the player's state
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

            // New round gold = saved gold + round allowance
            let round_gold = ctx.progress.player(role).gold + ctx.progress.round_allowance();
            ctx.build = BuildState::new_round(round_gold, locked_packs, next_id);
            ctx.units.extend(ctx.build.respawn_player_units(&ctx.progress.player(role).techs, ctx.role.player_id()));

            for unit in ctx.units.iter_mut() {
                if let Some(&(ddt, dst, ddr, dsr, kt)) = old_stats.get(&unit.id) {
                    unit.damage_dealt_total = ddt;
                    unit.damage_soaked_total = dst;
                    unit.damage_dealt_round = ddr;
                    unit.damage_soaked_round = dsr;
                    unit.kills_total = kt;
                }
            }

            ctx.units.extend(ctx.progress.opponent(role).respawn_units());

            battle.projectiles.clear();
            ctx.phase = GamePhase::Build;
        }
    }
}
