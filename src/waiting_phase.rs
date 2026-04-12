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
            ctx.units.extend(ctx.progress.player(pid).respawn_units());

            if ctx.obstacles.is_empty() && ctx.game_settings.terrain_enabled {
                ctx.obstacles = terrain::generate_terrain(ctx.progress.round, ctx.game_settings.terrain_destructible);
            } else {
                terrain::reset_cover_hp(&mut ctx.obstacles);
            }
            ctx.nav_grid = Some(terrain::NavGrid::from_obstacles(&ctx.obstacles, ARENA_W, ARENA_H, 15.0));

            // Canonical unit ordering — both sides must have identical Vec order
            ctx.units.sort_unstable_by_key(|u| u.id);

            macroquad::rand::srand(ctx.progress.round as u64);
            battle.reset();

            for unit in ctx.units.iter_mut() {
                unit.damage_dealt_round = 0.0;
                unit.damage_soaked_round = 0.0;
            }

            // Debug dump of initial battle state
            eprintln!("[BATTLE-START] round={} total_units={}",
                ctx.progress.round, ctx.units.len());
            for pl in &ctx.progress.players {
                let count = ctx.units.iter().filter(|u| u.player_id == pl.player_id).count();
                let pack_count = pl.packs.len();
                eprintln!("  player {} ({}): {} units from {} packs",
                    pl.player_id, pl.name, count, pack_count);
            }
            // Per-unit dump, sorted by ID for direct host/guest comparison
            let mut ids: Vec<(u64, crate::unit::UnitKind, u16, f32, f32, f32)> = ctx.units
                .iter()
                .map(|u| (u.id, u.kind, u.player_id, u.pos.x, u.pos.y, u.hp))
                .collect();
            ids.sort_by_key(|(id, _, _, _, _, _)| *id);
            for (id, kind, pid, px, py, hp) in &ids {
                eprintln!("  unit id={} kind={:?} pid={} pos=({:.1},{:.1}) hp={:.1}",
                    id, kind, pid, px, py, hp);
            }

            ctx.phase = GamePhase::Battle;
            return true;
        }
    }
    false
}
