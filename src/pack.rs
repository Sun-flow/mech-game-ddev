use macroquad::prelude::*;

use crate::tech::{TechId, TechState};
use crate::unit::{Unit, UnitKind};

#[derive(Clone, Debug)]
pub struct PackDef {
    pub kind: UnitKind,
    pub rows: u8,
    pub cols: u8,
    pub cost: u32,
    pub name: &'static str,
}

impl PackDef {
    pub fn count(&self) -> u8 {
        self.rows * self.cols
    }

    /// Get effective (rows, cols) accounting for rotation.
    pub fn effective_dims(&self, rotated: bool) -> (u8, u8) {
        if rotated { (self.cols, self.rows) } else { (self.rows, self.cols) }
    }
}

/// Spawn units for a pack at a given center position, applying techs.
/// Returns (spawned_units, assigned_ids).
pub fn spawn_pack_units(
    pack: &PackDef,
    center: Vec2,
    rotated: bool,
    team_id: u8,
    techs: &TechState,
    start_id: &mut u64,
) -> (Vec<Unit>, Vec<u64>) {
    let mut stats = pack.kind.stats();
    techs.apply_to_stats(pack.kind, &mut stats);
    let grid_gap = stats.size * 2.5;

    let (eff_rows, eff_cols) = pack.effective_dims(rotated);
    let grid_w = (eff_cols as f32 - 1.0) * grid_gap;
    let grid_h = (eff_rows as f32 - 1.0) * grid_gap;
    let start_x = center.x - grid_w / 2.0;
    let start_y = center.y - grid_h / 2.0;

    let mut units = Vec::new();
    let mut ids = Vec::new();

    for row in 0..eff_rows {
        for col in 0..eff_cols {
            let x = start_x + col as f32 * grid_gap;
            let y = start_y + row as f32 * grid_gap;
            let mut unit = Unit::new(*start_id, pack.kind, vec2(x, y), team_id);
            techs.apply_to_stats(pack.kind, &mut unit.stats);
            unit.hp = unit.stats.max_hp;
            if unit.kind == UnitKind::Scout
                && techs.has_tech(UnitKind::Scout, TechId::ScoutEvasion)
            {
                unit.evasion_chance = 0.25;
            }
            ids.push(*start_id);
            units.push(unit);
            *start_id += 1;
        }
    }

    (units, ids)
}

/// Spawn units for a pack using pre-assigned IDs (for respawning existing packs).
pub fn respawn_pack_units(
    pack: &PackDef,
    center: Vec2,
    rotated: bool,
    team_id: u8,
    techs: &TechState,
    unit_ids: &[u64],
) -> Vec<Unit> {
    let mut stats = pack.kind.stats();
    techs.apply_to_stats(pack.kind, &mut stats);
    let grid_gap = stats.size * 2.5;

    let (eff_rows, eff_cols) = pack.effective_dims(rotated);
    let grid_w = (eff_cols as f32 - 1.0) * grid_gap;
    let grid_h = (eff_rows as f32 - 1.0) * grid_gap;
    let start_x = center.x - grid_w / 2.0;
    let start_y = center.y - grid_h / 2.0;

    let mut units = Vec::new();
    let mut idx = 0;

    for row in 0..eff_rows {
        for col in 0..eff_cols {
            if idx < unit_ids.len() {
                let x = start_x + col as f32 * grid_gap;
                let y = start_y + row as f32 * grid_gap;
                let mut unit = Unit::new(unit_ids[idx], pack.kind, vec2(x, y), team_id);
                techs.apply_to_stats(pack.kind, &mut unit.stats);
                unit.hp = unit.stats.max_hp;
                if unit.kind == UnitKind::Scout
                    && techs.has_tech(UnitKind::Scout, TechId::ScoutEvasion)
                {
                    unit.evasion_chance = 0.25;
                }
                units.push(unit);
            }
            idx += 1;
        }
    }

    units
}

pub fn all_packs() -> &'static [PackDef] {
    &[
        // T1 - 100 gold
        PackDef {
            kind: UnitKind::Chaff,
            rows: 3,
            cols: 6,
            cost: 100,
            name: "Chaff",
        },
        PackDef {
            kind: UnitKind::Skirmisher,
            rows: 2,
            cols: 6,
            cost: 100,
            name: "Skirmishers",
        },
        PackDef {
            kind: UnitKind::Scout,
            rows: 2,
            cols: 3,
            cost: 100,
            name: "Scouts",
        },
        // T2 - 200 gold
        PackDef {
            kind: UnitKind::Striker,
            rows: 1,
            cols: 3,
            cost: 200,
            name: "Strikers",
        },
        PackDef {
            kind: UnitKind::Bruiser,
            rows: 1,
            cols: 2,
            cost: 200,
            name: "Bruisers",
        },
        PackDef {
            kind: UnitKind::Sentinel,
            rows: 1,
            cols: 2,
            cost: 200,
            name: "Sentinels",
        },
        PackDef {
            kind: UnitKind::Ranger,
            rows: 1,
            cols: 3,
            cost: 200,
            name: "Rangers",
        },
        PackDef {
            kind: UnitKind::Dragoon,
            rows: 1,
            cols: 5,
            cost: 200,
            name: "Dragoons",
        },
        PackDef {
            kind: UnitKind::Berserker,
            rows: 1,
            cols: 3,
            cost: 200,
            name: "Berserkers",
        },
        PackDef {
            kind: UnitKind::Interceptor,
            rows: 1,
            cols: 3,
            cost: 200,
            name: "Interceptors",
        },
        // T3 - 300 gold
        PackDef {
            kind: UnitKind::Artillery,
            rows: 1,
            cols: 2,
            cost: 300,
            name: "Artillery",
        },
        PackDef {
            kind: UnitKind::Sniper,
            rows: 1,
            cols: 1,
            cost: 300,
            name: "Sniper",
        },
        PackDef {
            kind: UnitKind::Shield,
            rows: 1,
            cols: 2,
            cost: 300,
            name: "Shields",
        },
    ]
}
