use crate::pack::{all_packs, PackDef};
use crate::tech::TechState;

/// Build a random army, excluding banned kinds. Returns chosen packs.
pub fn random_army_filtered(gold: u32, banned: &[crate::unit::UnitKind]) -> Vec<PackDef> {
    let mut remaining = gold;
    let mut chosen = Vec::new();
    let packs = all_packs();

    loop {
        let affordable: Vec<&PackDef> = packs
            .iter()
            .filter(|p| p.cost <= remaining && !banned.contains(&p.kind))
            .collect();

        if affordable.is_empty() {
            break;
        }

        let idx = macroquad::rand::gen_range(0, affordable.len());
        remaining -= affordable[idx].cost;
        chosen.push(affordable[idx].clone());
    }

    chosen
}

/// Build a smart army that balances categories and counter-picks based on AI memory.
pub fn smart_army(gold: u32, memory: &crate::match_progress::AiMemory, banned: &[crate::unit::UnitKind]) -> Vec<PackDef> {
    use crate::unit::UnitKind;

    let mut remaining = gold;
    let mut chosen = Vec::new();
    let packs = all_packs();

    // Filter out banned packs
    let available_packs: Vec<&PackDef> = packs.iter()
        .filter(|p| !banned.contains(&p.kind))
        .collect();

    if available_packs.is_empty() {
        return chosen;
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
            .filter(|p| p.cost <= remaining)
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

        let pick = if cat_affordable.is_empty() {
            // Fall back to any affordable pack
            let idx = macroquad::rand::gen_range(0, affordable.len());
            affordable[idx]
        } else {
            let idx = macroquad::rand::gen_range(0, cat_affordable.len());
            cat_affordable[idx]
        };

        let cost = pick.cost as f32;
        if frontline.contains(&pick.kind) { spent_front += cost; }
        else if ranged.contains(&pick.kind) { spent_range += cost; }
        else if support.contains(&pick.kind) { spent_support += cost; }
        else { spent_swarm += cost; }

        remaining -= pick.cost;
        chosen.push((*pick).clone());
    }

    chosen
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

/// Start battle in single-player AI mode. Generates AI army and transitions to Battle.
pub fn start_ai_battle(
    units: &mut Vec<crate::unit::Unit>,
    projectiles: &mut Vec<crate::projectile::Projectile>,
    progress: &mut crate::match_progress::MatchProgress,
    obstacles: &mut Vec<crate::terrain::Obstacle>,
    nav_grid: &mut Option<crate::terrain::NavGrid>,
    game_settings: &crate::settings::GameSettings,
) -> crate::game_state::GamePhase {
    use crate::arena::{ARENA_H, ARENA_W};
    use crate::game_state::GamePhase;

    projectiles.clear();

    // Generate terrain once per match (first round only); subsequent rounds just reset cover HP
    if obstacles.is_empty() && game_settings.terrain_enabled {
        *obstacles = crate::terrain::generate_terrain(progress.round, game_settings.terrain_destructible);
    } else {
        crate::terrain::reset_cover_hp(obstacles);
    }
    *nav_grid = Some(crate::terrain::NavGrid::from_obstacles(obstacles, ARENA_W, ARENA_H, 15.0));

    // Remove old AI (guest) units — they'll be respawned fresh from stored packs
    units.retain(|u| u.player_id != progress.guest.player_id);

    // Respawn all existing opponent (guest) units from previous rounds at full HP
    units.extend(progress.guest.respawn_units());

    // AI buys techs, then spawns NEW army for this round
    let mut ai_gold = progress.round_allowance();
    ai_buy_techs(&mut ai_gold, &mut progress.guest.techs);
    let ai_packs = if game_settings.smart_ai {
        smart_army(ai_gold, &progress.guest.ai_memory, &progress.banned_kinds)
    } else {
        random_army_filtered(ai_gold, &progress.banned_kinds)
    };
    let new_opponent_units = progress.spawn_ai_army(&ai_packs);
    units.extend(new_opponent_units);

    // Seed RNG for this round
    macroquad::rand::srand(progress.round as u64);

    // Reset per-round damage stats
    for unit in units.iter_mut() {
        unit.damage_dealt_round = 0.0;
        unit.damage_soaked_round = 0.0;
    }

    GamePhase::Battle
}
