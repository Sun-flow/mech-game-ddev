use macroquad::prelude::*;

use crate::arena::{MatchState, ARENA_H, HALF_W, shop_w};
use crate::economy::ArmyBuilder;
use crate::pack::{all_packs, PackDef};
use crate::tech::TechState;
use crate::unit::Unit;

pub const BUILD_TIMER: f32 = 60.0;

#[derive(Clone, Debug)]
pub enum GamePhase {
    Lobby,
    DraftBan {
        bans: Vec<crate::unit::UnitKind>,
        confirmed: bool,
        opponent_bans: Option<Vec<crate::unit::UnitKind>>,
    },
    Build,
    WaitingForOpponent,
    Battle,
    RoundResult {
        match_state: MatchState,
        lp_damage: i32,
        loser_team: Option<u8>,
    },
    GameOver(u8),
}

#[derive(Clone, Debug)]
pub struct PlacedPack {
    pub pack_index: usize,
    pub center: Vec2,
    pub unit_ids: Vec<u64>,
    pub pre_drag_center: Vec2,
    pub rotated: bool,
    pub locked: bool,
    pub round_placed: u32,
}

impl PlacedPack {
    pub fn effective_rows(&self, pack: &PackDef) -> u8 {
        if self.rotated { pack.cols } else { pack.rows }
    }

    pub fn effective_cols(&self, pack: &PackDef) -> u8 {
        if self.rotated { pack.rows } else { pack.cols }
    }

    pub fn bbox_half_size_rotated(pack: &PackDef, rotated: bool) -> Vec2 {
        let stats = pack.kind.stats();
        let grid_gap = stats.size * 2.5;
        let (rows, cols) = if rotated {
            (pack.cols, pack.rows)
        } else {
            (pack.rows, pack.cols)
        };
        let w = (cols as f32 - 1.0) * grid_gap + stats.size * 2.0;
        let h = (rows as f32 - 1.0) * grid_gap + stats.size * 2.0;
        vec2(w / 2.0, h / 2.0)
    }

    pub fn bbox_half_size_for(&self, pack: &PackDef) -> Vec2 {
        Self::bbox_half_size_rotated(pack, self.rotated)
    }

    pub fn contains(&self, point: Vec2, pack: &PackDef) -> bool {
        let half = self.bbox_half_size_for(pack);
        let min = self.center - half;
        let max = self.center + half;
        point.x >= min.x && point.x <= max.x && point.y >= min.y && point.y <= max.y
    }

    pub fn overlaps(&self, other: &PlacedPack, self_pack: &PackDef, other_pack: &PackDef) -> bool {
        let h1 = self.bbox_half_size_for(self_pack);
        let h2 = other.bbox_half_size_for(other_pack);
        let min1 = self.center - h1;
        let max1 = self.center + h1;
        let min2 = other.center - h2;
        let max2 = other.center + h2;
        min1.x < max2.x && max1.x > min2.x && min1.y < max2.y && max1.y > min2.y
    }
}

#[derive(Clone, Debug)]
pub enum UndoEntry {
    Place { placed_index: usize },
    Move { placed_index: usize, old_center: Vec2 },
    Rotate { placed_index: usize, was_rotated: bool, old_center: Vec2 },
    Tech { kind: crate::unit::UnitKind, tech_id: crate::tech::TechId },
    MultiMove { indices: Vec<usize>, old_centers: Vec<Vec2> },
}

pub const BUILD_LIMIT: u32 = 3;

pub struct BuildState {
    pub builder: ArmyBuilder,
    pub placed_packs: Vec<PlacedPack>,
    pub dragging: Option<usize>,
    pub selected_pack: Option<usize>,
    pub timer: f32,
    pub next_id: u64,
    pub round_tech_purchases: Vec<(crate::unit::UnitKind, crate::tech::TechId)>,
    pub undo_history: Vec<UndoEntry>,
    /// Number of packs bought this round (resets each round).
    pub packs_bought_this_round: u32,
    /// Multi-drag state: indices of packs being dragged together
    pub multi_dragging: Vec<usize>,
    /// Offset of each multi-dragged pack from the drag anchor point
    pub multi_drag_offsets: Vec<Vec2>,
    /// Saved centers before multi-drag started (for undo/cancel)
    pub multi_drag_pre_centers: Vec<Vec2>,
    /// World-space start point of the drag-box selection rectangle
    pub drag_box_start: Option<Vec2>,
}

impl BuildState {
    pub fn new(gold: u32) -> Self {
        Self {
            builder: ArmyBuilder::new(gold),
            placed_packs: Vec::new(),
            dragging: None,
            selected_pack: None,
            timer: BUILD_TIMER,
            next_id: 1,
            round_tech_purchases: Vec::new(),
            undo_history: Vec::new(),
            packs_bought_this_round: 0,
            multi_dragging: Vec::new(),
            multi_drag_offsets: Vec::new(),
            multi_drag_pre_centers: Vec::new(),
            drag_box_start: None,
        }
    }

    /// Create a new round's build state, carrying over locked packs from previous rounds.
    pub fn new_round(
        gold: u32,
        locked_packs: Vec<PlacedPack>,
        next_id: u64,
    ) -> Self {
        Self {
            builder: ArmyBuilder::new(gold),
            placed_packs: locked_packs,
            dragging: None,
            selected_pack: None,
            timer: BUILD_TIMER,
            next_id,
            round_tech_purchases: Vec::new(),
            undo_history: Vec::new(),
            packs_bought_this_round: 0,
            multi_dragging: Vec::new(),
            multi_drag_offsets: Vec::new(),
            multi_drag_pre_centers: Vec::new(),
            drag_box_start: None,
        }
    }

    /// Purchase a pack from the shop and place it on the board.
    pub fn purchase_pack(
        &mut self,
        pack_index: usize,
        round: u32,
        tech_state: &TechState,
    ) -> Option<Vec<Unit>> {
        let packs = all_packs();
        let pack = &packs[pack_index];

        // Check build limit for this round
        if self.packs_bought_this_round >= BUILD_LIMIT {
            return None;
        }

        if !self.builder.buy_pack(pack) {
            return None;
        }

        self.packs_bought_this_round += 1;

        // Find a default placement position in the deploy zone (center of build area)
        let half = PlacedPack::bbox_half_size_rotated(pack, false);
        let deploy_cx = shop_w() + (HALF_W - shop_w()) / 2.0;
        let deploy_cy = ARENA_H / 2.0;
        let min_x = shop_w() + half.x;
        let max_x = HALF_W - half.x;
        let min_y = half.y;
        let max_y = ARENA_H - half.y;

        let overlaps_existing = |cx: f32, cy: f32| -> bool {
            let padding = 5.0;
            for existing in &self.placed_packs {
                let ep = &packs[existing.pack_index];
                let eh = existing.bbox_half_size_for(ep);
                let dx = (cx - existing.center.x).abs();
                let dy = (cy - existing.center.y).abs();
                if dx < half.x + eh.x + padding && dy < half.y + eh.y + padding {
                    return true;
                }
            }
            false
        };

        let center = if !overlaps_existing(deploy_cx, deploy_cy) {
            vec2(deploy_cx, deploy_cy)
        } else {
            // Spiral outward from center to find closest free position
            let mut best = vec2(deploy_cx, deploy_cy);
            let mut found = false;
            let radius_step = 10.0;
            let angle_steps = 16;
            'search: for ring in 1..60 {
                let r = ring as f32 * radius_step;
                for a in 0..angle_steps {
                    let angle = (a as f32 / angle_steps as f32) * std::f32::consts::TAU;
                    let cx = (deploy_cx + r * angle.cos()).clamp(min_x, max_x);
                    let cy = (deploy_cy + r * angle.sin()).clamp(min_y, max_y);
                    if !overlaps_existing(cx, cy) {
                        best = vec2(cx, cy);
                        found = true;
                        break 'search;
                    }
                }
            }
            if !found {
                // Fallback: just clamp to center
                best = vec2(deploy_cx.clamp(min_x, max_x), deploy_cy.clamp(min_y, max_y));
            }
            best
        };

        // Spawn units with tech bonuses applied
        let mut stats = pack.kind.stats();
        tech_state.apply_to_stats(pack.kind, &mut stats);
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
                let mut unit = Unit::new(self.next_id, pack.kind, vec2(x, y), 0);
                // Apply tech stat bonuses
                tech_state.apply_to_stats(pack.kind, &mut unit.stats);
                unit.hp = unit.stats.max_hp;
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
            rotated: false,
            locked: false,
            round_placed: round,
        });

        let placed_index = self.placed_packs.len() - 1;
        self.undo_history.push(UndoEntry::Place { placed_index });

        Some(spawned)
    }

    /// Sell a placed pack. Returns (refund, unit_ids_to_remove). Only works on unlocked packs.
    pub fn sell_pack(&mut self, placed_index: usize) -> Option<(u32, Vec<u64>)> {
        if self.placed_packs[placed_index].locked {
            return None;
        }
        // Restore build limit since this was a this-round purchase
        if self.packs_bought_this_round > 0 {
            self.packs_bought_this_round -= 1;
        }
        let placed = self.placed_packs.remove(placed_index);
        let pack = &all_packs()[placed.pack_index];
        self.builder.gold_remaining += pack.cost;
        if let Some(pos) = self.builder.packs.iter().position(|p| p.name == pack.name) {
            self.builder.packs.remove(pos);
        }
        // Fix selected_pack index if needed
        if let Some(sel) = self.selected_pack {
            if sel == placed_index {
                self.selected_pack = None;
            } else if sel > placed_index {
                self.selected_pack = Some(sel - 1);
            }
        }
        Some((pack.cost, placed.unit_ids))
    }

    pub fn reposition_pack_units(&self, placed_index: usize, units: &mut [Unit]) {
        let placed = &self.placed_packs[placed_index];
        let pack = &all_packs()[placed.pack_index];
        let stats = pack.kind.stats();
        let grid_gap = stats.size * 2.5;
        let eff_rows = placed.effective_rows(pack);
        let eff_cols = placed.effective_cols(pack);
        let grid_w = (eff_cols as f32 - 1.0) * grid_gap;
        let grid_h = (eff_rows as f32 - 1.0) * grid_gap;
        let start_x = placed.center.x - grid_w / 2.0;
        let start_y = placed.center.y - grid_h / 2.0;

        let mut idx = 0;
        for row in 0..eff_rows {
            for col in 0..eff_cols {
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

    pub fn rotate_pack(&mut self, placed_index: usize, units: &mut [Unit]) -> bool {
        if self.placed_packs[placed_index].locked {
            return false;
        }
        let packs = all_packs();
        let placed = &self.placed_packs[placed_index];
        let pack = &packs[placed.pack_index];
        let new_rotated = !placed.rotated;

        let new_half = PlacedPack::bbox_half_size_rotated(pack, new_rotated);
        let center = placed.center;
        let clamped = vec2(
            center.x.clamp(new_half.x, HALF_W - new_half.x),
            center.y.clamp(new_half.y, ARENA_H - new_half.y),
        );

        let test = PlacedPack {
            pack_index: placed.pack_index,
            center: clamped,
            unit_ids: Vec::new(),
            pre_drag_center: clamped,
            rotated: new_rotated,
            locked: false,
            round_placed: 0,
        };

        for (i, existing) in self.placed_packs.iter().enumerate() {
            if i == placed_index {
                continue;
            }
            let ep = &packs[existing.pack_index];
            if test.overlaps(existing, pack, ep) {
                return false;
            }
        }

        let old_rotated = self.placed_packs[placed_index].rotated;
        let old_center = self.placed_packs[placed_index].center;
        self.undo_history.push(UndoEntry::Rotate { placed_index, was_rotated: old_rotated, old_center });

        self.placed_packs[placed_index].rotated = new_rotated;
        self.placed_packs[placed_index].center = clamped;
        self.reposition_pack_units(placed_index, units);
        true
    }

    pub fn would_overlap(&self, center: Vec2, pack_index: usize, skip_placed: Option<usize>, rotated: bool) -> bool {
        let packs = all_packs();
        let pack = &packs[pack_index];

        let test = PlacedPack {
            pack_index,
            center,
            unit_ids: Vec::new(),
            pre_drag_center: center,
            rotated,
            locked: false,
            round_placed: 0,
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

    /// Lock all current-round (unlocked) packs for carry-over.
    pub fn lock_current_packs(&mut self) {
        for pack in &mut self.placed_packs {
            pack.locked = true;
        }
    }
}
