use macroquad::prelude::*;
use std::collections::HashMap;

use crate::economy::ArmyBuilder;
use crate::net::OpponentBuildData;
use crate::pack::all_packs;
use crate::tech::TechState;
use crate::unit::{Unit, UnitKind};

pub const STARTING_LP: i32 = 3000;

/// Tracks what the AI observed in previous rounds for counter-picking.
#[derive(Clone, Debug, Default)]
pub struct AiMemory {
    /// What unit kinds the player had last round (kind → count).
    pub last_enemy_kinds: Vec<(UnitKind, u32)>,
    /// Whether the AI won the last round.
    pub last_result: bool,
}

impl AiMemory {
    /// Record the player's army composition and the round outcome.
    pub fn record_round(&mut self, player_units: &[Unit], ai_won: bool) {
        let mut counts: HashMap<UnitKind, u32> = HashMap::new();
        for u in player_units.iter().filter(|u| u.player_id == 0) {
            *counts.entry(u.kind).or_insert(0) += 1;
        }
        self.last_enemy_kinds = counts.into_iter().collect();
        self.last_result = ai_won;
    }
}

/// Info about an opponent-placed pack that persists across rounds.
#[derive(Clone, Debug)]
pub struct OpponentPlacedPack {
    pub pack_index: usize,
    pub center: Vec2,
    pub unit_ids: Vec<u64>,
    pub rotated: bool,
    pub round_placed: u32,
}

#[derive(Clone, Debug)]
pub struct MatchProgress {
    pub round: u32,
    pub player_lp: i32,
    pub opponent_lp: i32,
    pub player_techs: TechState,
    pub opponent_techs: TechState,
    pub opponent_packs: Vec<OpponentPlacedPack>,
    pub opponent_next_id: u64,
    pub player_saved_gold: u32,
    pub ai_memory: AiMemory,
    pub banned_kinds: Vec<UnitKind>,
}

impl MatchProgress {
    pub fn new(is_host: bool) -> Self {
        Self {
            round: 1,
            player_lp: STARTING_LP,
            opponent_lp: STARTING_LP,
            player_techs: TechState::new(),
            opponent_techs: TechState::new(),
            opponent_packs: Vec::new(),
            opponent_next_id: if is_host { 100_000 } else { 1 },
            player_saved_gold: 0,
            ai_memory: AiMemory::default(),
            banned_kinds: Vec::new(),
        }
    }

    pub fn round_allowance(&self) -> u32 {
        200 * self.round
    }

    pub fn round_gold(&self) -> u32 {
        self.player_saved_gold + self.round_allowance()
    }

    pub fn calculate_lp_damage(surviving_units: &[Unit], player_id: u8) -> i32 {
        let packs = all_packs();
        let mut total = 0i32;
        for unit in surviving_units {
            if !unit.alive || unit.player_id != player_id {
                continue;
            }
            if let Some(pack) = packs.iter().find(|p| p.kind == unit.kind) {
                let per_unit_value = pack.cost as f32 / pack.count() as f32;
                total += per_unit_value as i32;
            }
        }
        total
    }

    pub fn advance_round(&mut self) {
        self.round += 1;
    }

    pub fn is_game_over(&self) -> bool {
        self.player_lp <= 0 || self.opponent_lp <= 0
    }

    pub fn game_winner(&self) -> Option<u8> {
        if self.opponent_lp <= 0 {
            Some(0)
        } else if self.player_lp <= 0 {
            Some(1)
        } else {
            None
        }
    }

    /// Respawn all opponent units from stored packs at full HP with current techs.
    pub fn respawn_opponent_units(&self) -> Vec<Unit> {
        let packs = all_packs();
        let mut units = Vec::new();

        for opp_pack in &self.opponent_packs {
            let pack = &packs[opp_pack.pack_index];
            let spawned = crate::pack::respawn_pack_units(
                pack, opp_pack.center, opp_pack.rotated, 1,
                &self.opponent_techs, &opp_pack.unit_ids,
            );
            units.extend(spawned);
        }

        units
    }

    /// Apply opponent's build data received over the network.
    /// Mirrors x-coordinates so opponent units appear on the right half.
    /// Returns the new units spawned.
    pub fn apply_opponent_build(&mut self, data: &OpponentBuildData) -> Vec<Unit> {
        let packs = all_packs();
        let mut new_units = Vec::new();

        // Apply tech purchases
        for &(kind, tech_id) in &data.tech_purchases {
            self.opponent_techs.purchase(kind, tech_id);
        }

        // Spawn opponent's new packs (mirrored)
        for &(pack_index, (cx, cy), rotated) in &data.new_packs {
            if pack_index >= packs.len() {
                continue;
            }
            let pack = &packs[pack_index];

            // Mirror x: opponent built on their left half, we show on right half
            let mirrored_x = crate::arena::ARENA_W - cx;
            let center = vec2(mirrored_x, cy);

            let (spawned, ids) = crate::pack::spawn_pack_units(
                pack, center, rotated, 1,
                &self.opponent_techs, &mut self.opponent_next_id,
            );
            new_units.extend(spawned);

            self.opponent_packs.push(OpponentPlacedPack {
                pack_index,
                center,
                unit_ids: ids,
                rotated,
                round_placed: self.round,
            });
        }

        new_units
    }

    /// Spawn new AI army from a pre-built ArmyBuilder. Adds packs and returns units.
    pub fn spawn_ai_army_from_builder(&mut self, ai_builder: &ArmyBuilder) -> Vec<Unit> {
        let packs = all_packs();
        let mut new_units = Vec::new();

        let ai_center_x = crate::arena::HALF_W + (crate::arena::HALF_W / 2.0);
        let total_new = ai_builder.packs.len();
        if total_new == 0 {
            return new_units;
        }

        let arena_h = crate::arena::ARENA_H;
        let spacing = arena_h / (total_new as f32 + 1.0);

        for (pack_idx_in_build, pack_def) in ai_builder.packs.iter().enumerate() {
            let pack_index = packs.iter().position(|p| p.name == pack_def.name).unwrap_or(0);
            let pack = &packs[pack_index];

            let center_y = spacing * (pack_idx_in_build as f32 + 1.0);
            let offset_x = macroquad::rand::gen_range(-50.0f32, 50.0);
            let center = vec2(
                (ai_center_x + offset_x).clamp(crate::arena::HALF_W + 50.0, crate::arena::ARENA_W - 50.0),
                center_y,
            );

            let (spawned, ids) = crate::pack::spawn_pack_units(
                pack, center, false, 1,
                &self.opponent_techs, &mut self.opponent_next_id,
            );
            new_units.extend(spawned);

            self.opponent_packs.push(OpponentPlacedPack {
                pack_index,
                center,
                unit_ids: ids,
                rotated: false,
                round_placed: self.round,
            });
        }

        new_units
    }
}
