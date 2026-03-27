use macroquad::prelude::*;

use crate::pack::{all_packs, PackDef};
use crate::tech::TechState;
use crate::unit::Unit;

pub const STARTING_GOLD: u32 = 1000;

#[derive(Clone, Debug)]
pub struct ArmyBuilder {
    pub gold_remaining: u32,
    pub packs: Vec<PackDef>,
}

impl ArmyBuilder {
    pub fn new(gold: u32) -> Self {
        Self {
            gold_remaining: gold,
            packs: Vec::new(),
        }
    }

    pub fn buy_pack(&mut self, pack: &PackDef) -> bool {
        if self.gold_remaining >= pack.cost {
            self.gold_remaining -= pack.cost;
            self.packs.push(pack.clone());
            true
        } else {
            false
        }
    }

    pub fn gold_spent(&self) -> u32 {
        STARTING_GOLD - self.gold_remaining
    }

    /// Spawn all purchased packs as units on one side of the arena.
    pub fn spawn_army(&self, team_id: u8, side_x: f32, arena_h: f32, next_id: &mut u64) -> Vec<Unit> {
        let mut units = Vec::new();
        let total_packs = self.packs.len();
        if total_packs == 0 {
            return units;
        }

        let spacing_y = arena_h / (total_packs as f32 + 1.0);

        for (pack_idx, pack) in self.packs.iter().enumerate() {
            let center_y = spacing_y * (pack_idx as f32 + 1.0);
            let unit_stats = pack.kind.stats();
            let grid_gap = unit_stats.size * 2.5;

            let grid_w = (pack.cols as f32 - 1.0) * grid_gap;
            let grid_h = (pack.rows as f32 - 1.0) * grid_gap;
            let start_x = side_x - grid_w / 2.0;
            let start_y = center_y - grid_h / 2.0;

            for row in 0..pack.rows {
                for col in 0..pack.cols {
                    let x = start_x + col as f32 * grid_gap;
                    let y = start_y + row as f32 * grid_gap;
                    units.push(Unit::new(*next_id, pack.kind, vec2(x, y), team_id));
                    *next_id += 1;
                }
            }
        }

        units
    }
}

/// Build a random army within budget by picking random packs.
pub fn random_army(gold: u32) -> ArmyBuilder {
    let mut builder = ArmyBuilder::new(gold);
    let packs = all_packs();

    loop {
        let affordable: Vec<&PackDef> = packs
            .iter()
            .filter(|p| p.cost <= builder.gold_remaining)
            .collect();

        if affordable.is_empty() {
            break;
        }

        let idx = macroquad::rand::gen_range(0, affordable.len());
        builder.buy_pack(affordable[idx]);
    }

    builder
}

/// AI buys random techs, spending up to ~30% of available gold.
pub fn ai_buy_techs(gold: &mut u32, tech_state: &mut TechState) {
    use crate::unit::UnitKind;

    let tech_budget = *gold / 3; // spend up to 1/3 of gold on techs
    let mut spent = 0u32;

    let all_kinds = [
        UnitKind::Striker, UnitKind::Sentinel, UnitKind::Ranger, UnitKind::Scout,
        UnitKind::Bruiser, UnitKind::Artillery, UnitKind::Chaff, UnitKind::Sniper,
        UnitKind::Skirmisher, UnitKind::Dragoon, UnitKind::Berserker,
        UnitKind::Shield, UnitKind::Interceptor,
    ];

    // Try a few random tech purchases
    for _ in 0..5 {
        if spent >= tech_budget {
            break;
        }
        let kind_idx = macroquad::rand::gen_range(0, all_kinds.len());
        let kind = all_kinds[kind_idx];

        let available = tech_state.available_techs(kind);
        if available.is_empty() {
            continue;
        }

        let cost = tech_state.effective_cost(kind);
        if cost > *gold || spent + cost > tech_budget {
            continue;
        }

        let tech_idx = macroquad::rand::gen_range(0, available.len());
        let tech_id = available[tech_idx].id;
        tech_state.purchase(kind, tech_id);
        *gold -= cost;
        spent += cost;
    }
}
