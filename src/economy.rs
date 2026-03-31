use macroquad::prelude::*;

use crate::pack::{all_packs, PackDef};
use crate::tech::TechState;
use crate::unit::Unit;

#[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn gold_spent(&self) -> u32 {
        STARTING_GOLD - self.gold_remaining
    }

    /// Spawn all purchased packs as units on one side of the arena.
    #[allow(dead_code)]
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
#[allow(dead_code)]
pub fn random_army(gold: u32) -> ArmyBuilder {
    random_army_filtered(gold, &[])
}

/// Build a random army, excluding banned kinds.
pub fn random_army_filtered(gold: u32, banned: &[crate::unit::UnitKind]) -> ArmyBuilder {
    let mut builder = ArmyBuilder::new(gold);
    let packs = all_packs();

    loop {
        let affordable: Vec<&PackDef> = packs
            .iter()
            .filter(|p| p.cost <= builder.gold_remaining && !banned.contains(&p.kind))
            .collect();

        if affordable.is_empty() {
            break;
        }

        let idx = macroquad::rand::gen_range(0, affordable.len());
        builder.buy_pack(affordable[idx]);
    }

    builder
}

/// Build a smart army that balances categories and counter-picks based on AI memory.
pub fn smart_army(gold: u32, memory: &crate::match_progress::AiMemory, banned: &[crate::unit::UnitKind]) -> ArmyBuilder {
    use crate::unit::UnitKind;

    let mut builder = ArmyBuilder::new(gold);
    let packs = all_packs();

    // Filter out banned packs
    let available_packs: Vec<&PackDef> = packs.iter()
        .filter(|p| !banned.contains(&p.kind))
        .collect();

    if available_packs.is_empty() {
        return builder;
    }

    // Categorize packs
    let frontline = [UnitKind::Sentinel, UnitKind::Bruiser, UnitKind::Dragoon];
    let ranged = [UnitKind::Striker, UnitKind::Ranger, UnitKind::Artillery, UnitKind::Sniper];
    let support = [UnitKind::Shield, UnitKind::Interceptor];
    let swarm = [UnitKind::Chaff, UnitKind::Skirmisher, UnitKind::Scout, UnitKind::Berserker];

    // Base budget percentages (adjusted by counter-picking)
    let mut front_pct: f32 = 0.35;
    let mut range_pct: f32 = 0.35;
    let mut support_pct: f32 = 0.15;
    let mut swarm_pct: f32 = 0.15;

    // Counter-pick adjustments based on memory
    if !memory.last_enemy_kinds.is_empty() && !memory.last_result {
        let total_enemy: u32 = memory.last_enemy_kinds.iter().map(|(_, c)| c).sum();
        if total_enemy > 0 {
            let enemy_ranged: u32 = memory.last_enemy_kinds.iter()
                .filter(|(k, _)| ranged.contains(k))
                .map(|(_, c)| c).sum();
            let enemy_front: u32 = memory.last_enemy_kinds.iter()
                .filter(|(k, _)| frontline.contains(k))
                .map(|(_, c)| c).sum();
            let enemy_swarm: u32 = memory.last_enemy_kinds.iter()
                .filter(|(k, _)| swarm.contains(k))
                .map(|(_, c)| c).sum();

            let r_frac = enemy_ranged as f32 / total_enemy as f32;
            let f_frac = enemy_front as f32 / total_enemy as f32;
            let s_frac = enemy_swarm as f32 / total_enemy as f32;

            // Heavy ranged → more support (shields/interceptors) and swarm
            if r_frac > 0.4 {
                support_pct += 0.15;
                swarm_pct += 0.10;
                range_pct -= 0.15;
                front_pct -= 0.10;
            }
            // Heavy frontline → more ranged
            if f_frac > 0.4 {
                range_pct += 0.15;
                front_pct -= 0.15;
            }
            // Heavy swarm → more splash (artillery, bruiser = frontline)
            if s_frac > 0.4 {
                front_pct += 0.10;
                range_pct += 0.05;
                swarm_pct -= 0.15;
            }
        }
    }

    // Normalize
    let total = front_pct + range_pct + support_pct + swarm_pct;
    front_pct /= total;
    range_pct /= total;
    support_pct /= total;
    swarm_pct /= total;

    let budget = gold as f32;
    let mut spent_front: f32 = 0.0;
    let mut spent_range: f32 = 0.0;
    let mut spent_support: f32 = 0.0;
    let mut spent_swarm: f32 = 0.0;

    // Purchase loop: pick the most under-budget category, buy a random pack from it
    loop {
        let affordable: Vec<&&PackDef> = available_packs.iter()
            .filter(|p| p.cost <= builder.gold_remaining)
            .collect();
        if affordable.is_empty() {
            break;
        }

        // Find most under-budget category
        let front_deficit = front_pct - spent_front / budget;
        let range_deficit = range_pct - spent_range / budget;
        let support_deficit = support_pct - spent_support / budget;
        let swarm_deficit = swarm_pct - spent_swarm / budget;

        let max_deficit = front_deficit.max(range_deficit).max(support_deficit).max(swarm_deficit);

        let target_cats: &[UnitKind] = if max_deficit == front_deficit {
            &frontline
        } else if max_deficit == range_deficit {
            &ranged
        } else if max_deficit == support_deficit {
            &support
        } else {
            &swarm
        };

        // Try to buy from target category
        let cat_affordable: Vec<&&PackDef> = affordable.iter()
            .filter(|p| target_cats.contains(&p.kind))
            .copied()
            .collect();

        let chosen = if cat_affordable.is_empty() {
            // Fall back to any affordable pack
            let idx = macroquad::rand::gen_range(0, affordable.len());
            affordable[idx]
        } else {
            let idx = macroquad::rand::gen_range(0, cat_affordable.len());
            cat_affordable[idx]
        };

        let cost = chosen.cost as f32;
        if frontline.contains(&chosen.kind) { spent_front += cost; }
        else if ranged.contains(&chosen.kind) { spent_range += cost; }
        else if support.contains(&chosen.kind) { spent_support += cost; }
        else { spent_swarm += cost; }

        builder.buy_pack(chosen);
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
