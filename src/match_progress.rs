use macroquad::prelude::*;
use std::collections::HashMap;

use crate::game_state::PlacedPack;
use crate::pack::PackDef;
use crate::net::OpponentBuildData;
use crate::pack::all_packs;
use crate::tech::TechState;
use crate::unit::{Unit, UnitKind};

pub const STARTING_LP: i32 = 3000;

/// Tracks what the AI observed in previous rounds for counter-picking.
#[derive(Clone, Debug, Default)]
pub struct AiMemory {
    /// What unit kinds the player had last round (kind -> count).
    pub last_enemy_kinds: Vec<(UnitKind, u32)>,
    /// Whether the AI won the last round.
    pub last_result: bool,
}

impl AiMemory {
    /// Record the human player's army composition and the round outcome.
    pub fn record_round(&mut self, player_units: &[Unit], human_player_id: u8, ai_won: bool) {
        let mut counts: HashMap<UnitKind, u32> = HashMap::new();
        for u in player_units.iter().filter(|u| u.player_id == human_player_id) {
            *counts.entry(u.kind).or_insert(0) += 1;
        }
        self.last_enemy_kinds = counts.into_iter().collect();
        self.last_result = ai_won;
    }
}

/// Canonical per-player state within a match.
#[derive(Clone, Debug)]
pub struct PlayerState {
    pub player_id: u8,
    pub lp: i32,
    pub techs: TechState,
    pub name: String,
    pub next_id: u64,
    pub gold: u32,
    pub packs: Vec<PlacedPack>,
    pub ai_memory: AiMemory,
}

impl PlayerState {
    pub fn new(player_id: u8) -> Self {
        Self {
            player_id,
            lp: STARTING_LP,
            techs: TechState::new(),
            name: format!("Player {}", player_id + 1),
            next_id: player_id as u64 * 100_000 + 1,
            gold: 0,
            packs: Vec::new(),
            ai_memory: AiMemory::default(),
        }
    }

    /// Respawn all units from stored packs at full HP with current techs.
    pub fn respawn_units(&self) -> Vec<Unit> {
        let packs = all_packs();
        let mut units = Vec::new();

        for placed in &self.packs {
            let pack = &packs[placed.pack_index];
            let spawned = crate::pack::respawn_pack_units(
                pack,
                placed.center,
                placed.rotated,
                self.player_id,
                &self.techs,
                &placed.unit_ids,
            );
            units.extend(spawned);
        }

        units
    }

    /// Lock all current packs for carry-over between rounds.
    pub fn lock_packs(&mut self) {
        for pack in &mut self.packs {
            pack.locked = true;
        }
    }
}

#[derive(Clone, Debug)]
pub struct MatchProgress {
    pub round: u32,
    pub players: [PlayerState; 2],
    pub banned_kinds: Vec<UnitKind>,
}

impl MatchProgress {
    pub fn new() -> Self {
        Self {
            round: 1,
            players: [PlayerState::new(0), PlayerState::new(1)],
            banned_kinds: Vec::new(),
        }
    }

    pub fn round_allowance(&self) -> u32 {
        200 * self.round
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
        self.players[0].lp <= 0 || self.players[1].lp <= 0
    }

    pub fn game_winner(&self) -> Option<u8> {
        if self.players[1].lp <= 0 {
            Some(0)
        } else if self.players[0].lp <= 0 {
            Some(1)
        } else {
            None
        }
    }

    /// Spawn new AI army from a list of purchased packs. Adds packs and returns units.
    pub fn spawn_ai_army(&mut self, ai_packs: &[PackDef]) -> Vec<Unit> {
        let packs = all_packs();
        let mut new_units = Vec::new();

        let ai_center_x = crate::arena::HALF_W + (crate::arena::HALF_W / 2.0);
        let total_new = ai_packs.len();
        if total_new == 0 {
            return new_units;
        }

        let arena_h = crate::arena::ARENA_H;
        let spacing = arena_h / (total_new as f32 + 1.0);

        for (pack_idx_in_build, pack_def) in ai_packs.iter().enumerate() {
            let pack_index = packs.iter().position(|p| p.name == pack_def.name).unwrap_or(0);
            let pack = &packs[pack_index];

            let center_y = spacing * (pack_idx_in_build as f32 + 1.0);
            let offset_x = macroquad::rand::gen_range(-50.0f32, 50.0);
            let center = vec2(
                (ai_center_x + offset_x)
                    .clamp(crate::arena::HALF_W + 50.0, crate::arena::ARENA_W - 50.0),
                center_y,
            );

            let (spawned, ids) = crate::pack::spawn_pack_units(
                pack,
                center,
                false,
                self.players[1].player_id,
                &self.players[1].techs,
                &mut self.players[1].next_id,
            );
            new_units.extend(spawned);

            self.players[1].packs.push(PlacedPack {
                pack_index,
                center,
                unit_ids: ids,
                pre_drag_center: center,
                rotated: false,
                locked: true,
                round_placed: self.round,
            });
        }

        new_units
    }
}

/// Apply peer's build data received over the network.
/// Canonical coordinates — no mirroring needed.
pub fn apply_peer_build(player: &mut PlayerState, data: &OpponentBuildData, round: u32) -> Vec<Unit> {
    let packs = all_packs();
    let mut new_units = Vec::new();

    // Apply tech purchases
    for &(kind, tech_id) in &data.tech_purchases {
        player.techs.purchase(kind, tech_id);
    }

    // Spawn peer's new packs (canonical coordinates)
    for &(pack_index, (cx, cy), rotated) in &data.new_packs {
        if pack_index >= packs.len() {
            continue;
        }
        let pack = &packs[pack_index];
        let center = vec2(cx, cy);

        let (spawned, ids) = crate::pack::spawn_pack_units(
            pack,
            center,
            rotated,
            player.player_id,
            &player.techs,
            &mut player.next_id,
        );
        new_units.extend(spawned);

        player.packs.push(PlacedPack {
            pack_index,
            center,
            unit_ids: ids,
            pre_drag_center: center,
            rotated,
            locked: true,
            round_placed: round,
        });
    }

    new_units
}
