use macroquad::prelude::*;

use crate::arena::{MatchState, ARENA_H, HALF_W, SHOP_W};
use crate::economy::{random_army, ArmyBuilder, STARTING_GOLD};
use crate::pack::{all_packs, PackDef};
use crate::unit::Unit;

pub const BUILD_TIMER: f32 = 60.0;

#[derive(Clone, Debug)]
pub enum GamePhase {
    Build,
    Battle,
    Result(MatchState),
}

#[derive(Clone, Debug)]
pub struct PlacedPack {
    pub pack_index: usize,
    pub center: Vec2,
    pub unit_ids: Vec<u64>,
    pub pre_drag_center: Vec2,
}

impl PlacedPack {
    /// Compute bounding box half-sizes for a pack.
    pub fn bbox_half_size(pack: &PackDef) -> Vec2 {
        let stats = pack.kind.stats();
        let grid_gap = stats.size * 2.5;
        let w = (pack.cols as f32 - 1.0) * grid_gap + stats.size * 2.0;
        let h = (pack.rows as f32 - 1.0) * grid_gap + stats.size * 2.0;
        vec2(w / 2.0, h / 2.0)
    }

    /// Check if a point is inside this pack's bounding box.
    pub fn contains(&self, point: Vec2, pack: &PackDef) -> bool {
        let half = Self::bbox_half_size(pack);
        let min = self.center - half;
        let max = self.center + half;
        point.x >= min.x && point.x <= max.x && point.y >= min.y && point.y <= max.y
    }

    /// Check if two packs' bounding boxes overlap (AABB test).
    pub fn overlaps(&self, other: &PlacedPack, self_pack: &PackDef, other_pack: &PackDef) -> bool {
        let h1 = Self::bbox_half_size(self_pack);
        let h2 = Self::bbox_half_size(other_pack);
        let min1 = self.center - h1;
        let max1 = self.center + h1;
        let min2 = other.center - h2;
        let max2 = other.center + h2;
        min1.x < max2.x && max1.x > min2.x && min1.y < max2.y && max1.y > min2.y
    }
}

pub struct BuildState {
    pub builder: ArmyBuilder,
    pub placed_packs: Vec<PlacedPack>,
    pub dragging: Option<usize>,
    pub drag_offset: Vec2,
    pub timer: f32,
    pub next_id: u64,
}

impl BuildState {
    pub fn new() -> Self {
        Self {
            builder: ArmyBuilder::new(STARTING_GOLD),
            placed_packs: Vec::new(),
            dragging: None,
            drag_offset: Vec2::ZERO,
            timer: BUILD_TIMER,
            next_id: 1,
        }
    }

    /// Purchase a pack from the shop and place it on the board.
    /// Returns the units spawned by this pack.
    pub fn purchase_pack(&mut self, pack_index: usize) -> Option<Vec<Unit>> {
        let packs = all_packs();
        let pack = &packs[pack_index];

        if !self.builder.buy_pack(pack) {
            return None;
        }

        // Find a default placement position that doesn't overlap
        let default_x = SHOP_W + 100.0;
        let mut default_y = 80.0;
        let half = PlacedPack::bbox_half_size(pack);

        // Try stacking vertically
        for existing in &self.placed_packs {
            let ep = &packs[existing.pack_index];
            let eh = PlacedPack::bbox_half_size(ep);
            if (default_x - existing.center.x).abs() < half.x + eh.x {
                let bottom = existing.center.y + eh.y;
                if default_y < bottom + half.y + 5.0 {
                    default_y = bottom + half.y + 5.0;
                }
            }
        }

        // Clamp to arena
        default_y = default_y.clamp(half.y, ARENA_H - half.y);

        let center = vec2(default_x, default_y);

        // Spawn units in grid formation
        let stats = pack.kind.stats();
        let grid_gap = stats.size * 2.5;
        let grid_w = (pack.cols as f32 - 1.0) * grid_gap;
        let grid_h = (pack.rows as f32 - 1.0) * grid_gap;
        let start_x = center.x - grid_w / 2.0;
        let start_y = center.y - grid_h / 2.0;

        let mut spawned = Vec::new();
        let mut ids = Vec::new();

        for row in 0..pack.rows {
            for col in 0..pack.cols {
                let x = start_x + col as f32 * grid_gap;
                let y = start_y + row as f32 * grid_gap;
                let unit = Unit::new(self.next_id, pack.kind, vec2(x, y), 0);
                ids.push(self.next_id);
                spawned.push(unit);
                self.next_id += 1;
            }
        }

        self.placed_packs.push(PlacedPack {
            pack_index,
            center,
            unit_ids: ids,
            pre_drag_center: center,
        });

        Some(spawned)
    }

    /// Sell a placed pack (by index in placed_packs). Returns the pack cost refunded.
    pub fn sell_pack(&mut self, placed_index: usize) -> (u32, Vec<u64>) {
        let placed = self.placed_packs.remove(placed_index);
        let pack = &all_packs()[placed.pack_index];
        self.builder.gold_remaining += pack.cost;
        // Remove from builder's packs list too
        if let Some(pos) = self.builder.packs.iter().position(|p| p.name == pack.name) {
            self.builder.packs.remove(pos);
        }
        (pack.cost, placed.unit_ids)
    }

    /// Move all units in a pack to match the pack's center position.
    pub fn reposition_pack_units(&self, placed_index: usize, units: &mut [Unit]) {
        let placed = &self.placed_packs[placed_index];
        let pack = &all_packs()[placed.pack_index];
        let stats = pack.kind.stats();
        let grid_gap = stats.size * 2.5;
        let grid_w = (pack.cols as f32 - 1.0) * grid_gap;
        let grid_h = (pack.rows as f32 - 1.0) * grid_gap;
        let start_x = placed.center.x - grid_w / 2.0;
        let start_y = placed.center.y - grid_h / 2.0;

        let mut idx = 0;
        for row in 0..pack.rows {
            for col in 0..pack.cols {
                let target_pos = vec2(
                    start_x + col as f32 * grid_gap,
                    start_y + row as f32 * grid_gap,
                );
                if idx < placed.unit_ids.len() {
                    let uid = placed.unit_ids[idx];
                    if let Some(unit) = units.iter_mut().find(|u| u.id == uid) {
                        unit.pos = target_pos;
                    }
                }
                idx += 1;
            }
        }
    }

    /// Check if a placement would overlap any other pack (excluding skip_index).
    pub fn would_overlap(&self, center: Vec2, pack_index: usize, skip_placed: Option<usize>) -> bool {
        let packs = all_packs();
        let pack = &packs[pack_index];

        let test = PlacedPack {
            pack_index,
            center,
            unit_ids: Vec::new(),
            pre_drag_center: center,
        };

        for (i, existing) in self.placed_packs.iter().enumerate() {
            if Some(i) == skip_placed {
                continue;
            }
            let ep = &packs[existing.pack_index];
            if test.overlaps(existing, pack, ep) {
                return true;
            }
        }
        false
    }

    /// Find which placed pack (if any) contains the given point.
    pub fn pack_at(&self, point: Vec2) -> Option<usize> {
        let packs = all_packs();
        let mut best: Option<(usize, f32)> = None;
        for (i, placed) in self.placed_packs.iter().enumerate() {
            let pack = &packs[placed.pack_index];
            if placed.contains(point, pack) {
                let dist = placed.center.distance(point);
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((i, dist));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// Spawn the AI army on the right half.
    pub fn spawn_ai_army(&mut self) -> (Vec<Unit>, ArmyBuilder) {
        let ai_builder = random_army(STARTING_GOLD);
        let ai_x = HALF_W + (HALF_W / 2.0);
        let ai_units = ai_builder.spawn_army(1, ai_x, ARENA_H, &mut self.next_id);
        (ai_units, ai_builder)
    }
}
