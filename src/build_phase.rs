use macroquad::prelude::*;

use crate::arena::{ARENA_H, shop_w};
use crate::battle_phase::BattleState;
use crate::context::GameContext;
use crate::economy;
use crate::game_state::{self, GamePhase};
use crate::net;
use crate::pack::all_packs;
use crate::tech;
use crate::tech_ui;
use crate::shop;
use crate::terrain;

pub fn update(
    ctx: &mut GameContext,
    battle: &mut BattleState,
    ms: &crate::input::MouseState,
    dt: f32,
) {
    let screen_mouse = ms.screen_mouse;
    let mouse = ms.world_mouse;
    let left_click = ms.left_click;
    let right_click = ms.right_click;
    let middle_click = ms.middle_click;
    // Poll network
    if let Some(ref mut n) = ctx.net {
        n.poll();
    }

    // Grid toggle
    if is_key_pressed(KeyCode::G) {
        ctx.show_grid = !ctx.show_grid;
    }

    // Undo (Ctrl+Z)
    if is_key_down(KeyCode::LeftControl) && is_key_pressed(KeyCode::Z) && ctx.build.dragging.is_none() {
        if let Some(entry) = ctx.build.undo_history.pop() {
            match entry {
                game_state::UndoEntry::Place { placed_index } => {
                    if placed_index < ctx.build.placed_packs.len() {
                        if let Some((_refund, removed_ids)) = ctx.build.sell_pack(placed_index) {
                            ctx.units.retain(|u| !removed_ids.contains(&u.id));
                        }
                    }
                }
                game_state::UndoEntry::Move { placed_index, old_center } => {
                    if placed_index < ctx.build.placed_packs.len() {
                        ctx.build.placed_packs[placed_index].center = old_center;
                        ctx.build.reposition_pack_units(placed_index, &mut ctx.units);
                    }
                }
                game_state::UndoEntry::Rotate { placed_index, was_rotated, old_center } => {
                    if placed_index < ctx.build.placed_packs.len() {
                        ctx.build.placed_packs[placed_index].rotated = was_rotated;
                        ctx.build.placed_packs[placed_index].center = old_center;
                        ctx.build.reposition_pack_units(placed_index, &mut ctx.units);
                    }
                }
                game_state::UndoEntry::MultiMove { indices, old_centers } => {
                    for (i, &idx) in indices.iter().enumerate() {
                        if idx < ctx.build.placed_packs.len() {
                            ctx.build.placed_packs[idx].center = old_centers[i];
                            ctx.build.reposition_pack_units(idx, &mut ctx.units);
                        }
                    }
                }
                game_state::UndoEntry::Tech { kind, tech_id } => {
                    let role = ctx.role;
                    // Refund tech cost
                    let cost = ctx.progress.player(role).techs.effective_cost(kind);
                    // unpurchase first so effective_cost returns the right amount next time
                    ctx.progress.player_mut(role).techs.unpurchase(kind, tech_id);
                    // Refund: cost was (100 + N*100) where N was count before purchase
                    // After unpurchase, effective_cost gives the old cost, so just refund that
                    ctx.build.builder.gold_remaining += cost;
                    // Remove from round tech purchases
                    if let Some(pos) = ctx.build.round_tech_purchases.iter().rposition(|(k, t)| *k == kind && *t == tech_id) {
                        ctx.build.round_tech_purchases.remove(pos);
                    }
                    // Refresh ctx.units to remove tech effect
                    tech::refresh_units_of_kind(&mut ctx.units, kind, &ctx.progress.player(role).techs);
                }
            }
        }
    }

    // Timer countdown
    ctx.build.timer -= dt;
    if ctx.build.timer <= 0.0 {
        if ctx.net.is_some() {
            // Multiplayer: send ctx.build, transition to waiting
            net::send_build_complete(&mut ctx.net, &ctx.build);
            ctx.phase = GamePhase::WaitingForOpponent;
        } else {
            // Single-player: start battle immediately with AI
            ctx.phase = economy::start_ai_battle(
                &mut ctx.units,
                &mut battle.projectiles,
                &mut ctx.progress,
                &mut ctx.obstacles,
                &mut ctx.nav_grid,
                &ctx.game_settings,
            );
            battle.reset();
        }
        return;
    }

    // Shop interaction (left click in shop area, only when not holding a pack)
    let role = ctx.role;
    if left_click && screen_mouse.x < shop_w() && ctx.build.dragging.is_none() {
        if let Some(pack_idx) =
            shop::draw_shop(ctx.build.builder.gold_remaining, screen_mouse, true, &ctx.progress.banned_kinds, game_state::BUILD_LIMIT - ctx.build.packs_bought_this_round)
        {
            if let Some(new_units) = ctx.build.purchase_pack(
                pack_idx,
                ctx.progress.round,
                &ctx.progress.player(role).techs,
                ctx.role.deploy_x_range(),
            ) {
                ctx.units.extend(new_units);
            }
        }
    }

    // Tech panel interaction (when a pack is selected)
    let mut click_consumed = false;
    if left_click {
        if let Some(sel_idx) = ctx.build.selected_pack {
        let placed = &ctx.build.placed_packs[sel_idx];
        let kind = all_packs()[placed.pack_index].kind;
        let cs = tech_ui::PackCombatStats::from_units(&ctx.units, &placed.unit_ids);

        // Check if mouse is in the tech panel area (consume click to prevent drag)
        // Compute actual panel height to avoid blocking clicks in the entire column
        let available_count = ctx.progress.player(role).techs.available_techs(kind).len();
        let purchased_count = ctx.progress.player(role).techs.tech_count(kind);
        let has_combat = cs.damage_dealt_total > 0.0 || cs.damage_soaked_total > 0.0;
        let combat_extra = if has_combat { 5.0 * 15.0 + 30.0 } else { 0.0 };
        let panel_h = crate::ui::s(120.0) + (available_count + purchased_count) as f32 * crate::ui::s(35.0) + crate::ui::s(combat_extra) + crate::ui::s(20.0);
        if screen_mouse.x >= crate::ui::s(490.0) && screen_mouse.x <= crate::ui::s(700.0)
            && screen_mouse.y >= crate::ui::s(30.0) && screen_mouse.y <= crate::ui::s(30.0) + panel_h
        {
            click_consumed = true;
        }

        if let Some(tech_id) = tech_ui::draw_tech_panel(
            kind,
            &ctx.progress.player(role).techs,
            ctx.build.builder.gold_remaining,
            screen_mouse,
            true,
            Some(&cs),
        ) {
            let cost = ctx.progress.player(role).techs.effective_cost(kind);
            if ctx.build.builder.gold_remaining >= cost {
                ctx.build.builder.gold_remaining -= cost;
                ctx.progress.player_mut(role).techs.purchase(kind, tech_id);
                // Track tech purchase for network sync and undo
                ctx.build.round_tech_purchases.push((kind, tech_id));
                ctx.build.undo_history.push(game_state::UndoEntry::Tech { kind, tech_id });
                // Refresh ALL ctx.units of this kind with new tech stats
                tech::refresh_units_of_kind(&mut ctx.units, kind, &ctx.progress.player(role).techs);
            }
        }
    }
    }

    // Right-click: sell if on unlocked pack, otherwise deselect
    if right_click && screen_mouse.x > shop_w() && ctx.build.dragging.is_none() {
        let mut sold = false;
        if let Some(placed_idx) = ctx.build.pack_at(mouse) {
            if !ctx.build.placed_packs[placed_idx].locked {
                if let Some((_, removed_ids)) = ctx.build.sell_pack(placed_idx) {
                    ctx.units.retain(|u| !removed_ids.contains(&u.id));
                    sold = true;
                }
            }
        }
        if !sold {
            // Deselect on right-click in empty space or on locked pack
            ctx.build.selected_pack = None;
        }
    }

    // Middle-click to rotate (only unlocked)
    if middle_click && screen_mouse.x > shop_w() {
        if let Some(drag_idx) = ctx.build.dragging {
            if !ctx.build.placed_packs[drag_idx].locked {
                ctx.build.rotate_pack(drag_idx, &mut ctx.units, ctx.role.deploy_x_range());
            }
        } else if let Some(placed_idx) = ctx.build.pack_at(mouse) {
            if !ctx.build.placed_packs[placed_idx].locked {
                ctx.build.rotate_pack(placed_idx, &mut ctx.units, ctx.role.deploy_x_range());
            }
        }
    }

    // === Multi-drag system ===
    let left_released = is_mouse_button_released(MouseButton::Left);

    // Active multi-drag: move all packs together
    if !ctx.build.multi_dragging.is_empty() {
        let grid = terrain::GRID_CELL;
        let snapped_mouse = vec2(
            (mouse.x / grid).round() * grid,
            (mouse.y / grid).round() * grid,
        );
        for (i, &pack_idx) in ctx.build.multi_dragging.clone().iter().enumerate() {
            let offset = ctx.build.multi_drag_offsets[i];
            let new_center = snapped_mouse + offset;
            let pack = &all_packs()[ctx.build.placed_packs[pack_idx].pack_index];
            let half = ctx.build.placed_packs[pack_idx].bbox_half_size_for(pack);
            let (dmin, dmax) = ctx.role.deploy_x_range();
            let clamped = vec2(
                new_center.x.clamp(dmin + half.x, dmax - half.x),
                new_center.y.clamp(half.y, ARENA_H - half.y),
            );
            ctx.build.placed_packs[pack_idx].center = clamped;
            ctx.build.reposition_pack_units(pack_idx, &mut ctx.units);
        }

        if left_click {
            // Drop all — check overlaps
            let dragging_set: Vec<usize> = ctx.build.multi_dragging.clone();
            let mut any_overlap = false;
            for &pack_idx in &dragging_set {
                let placed = &ctx.build.placed_packs[pack_idx];
                // Check overlap against non-dragged packs
                for (j, other) in ctx.build.placed_packs.iter().enumerate() {
                    if dragging_set.contains(&j) { continue; }
                    let p1 = &all_packs()[placed.pack_index];
                    let p2 = &all_packs()[other.pack_index];
                    if placed.overlaps(other, p1, p2) {
                        any_overlap = true;
                        break;
                    }
                }
                if any_overlap { break; }
            }

            if any_overlap {
                // Revert all to pre-centers
                for (i, &pack_idx) in dragging_set.iter().enumerate() {
                    ctx.build.placed_packs[pack_idx].center = ctx.build.multi_drag_pre_centers[i];
                    ctx.build.reposition_pack_units(pack_idx, &mut ctx.units);
                }
            } else {
                // Check if any actually moved
                let mut any_moved = false;
                for (i, &pack_idx) in dragging_set.iter().enumerate() {
                    if ctx.build.placed_packs[pack_idx].center != ctx.build.multi_drag_pre_centers[i] {
                        any_moved = true;
                        break;
                    }
                }
                if any_moved {
                    ctx.build.undo_history.push(game_state::UndoEntry::MultiMove {
                        indices: dragging_set.clone(),
                        old_centers: ctx.build.multi_drag_pre_centers.clone(),
                    });
                }
            }
            ctx.build.multi_dragging.clear();
            ctx.build.multi_drag_offsets.clear();
            ctx.build.multi_drag_pre_centers.clear();
        }

        if right_click {
            // Cancel — revert all
            let dragging_set: Vec<usize> = ctx.build.multi_dragging.clone();
            for (i, &pack_idx) in dragging_set.iter().enumerate() {
                ctx.build.placed_packs[pack_idx].center = ctx.build.multi_drag_pre_centers[i];
                ctx.build.reposition_pack_units(pack_idx, &mut ctx.units);
            }
            ctx.build.multi_dragging.clear();
            ctx.build.multi_drag_offsets.clear();
            ctx.build.multi_drag_pre_centers.clear();
        }
    }
    // Drag-box: track while held, complete on release
    else if let Some(box_start) = ctx.build.drag_box_start {
        if left_released {
            let box_end = mouse;
            let min_x = box_start.x.min(box_end.x);
            let max_x = box_start.x.max(box_end.x);
            let min_y = box_start.y.min(box_end.y);
            let max_y = box_start.y.max(box_end.y);

            // Minimum box size threshold to distinguish from accidental micro-drag
            if (max_x - min_x) > 5.0 || (max_y - min_y) > 5.0 {
                let packs = all_packs();
                let mut selected_indices = Vec::new();
                for (i, placed) in ctx.build.placed_packs.iter().enumerate() {
                    if placed.locked { continue; }
                    let pack = &packs[placed.pack_index];
                    let half = placed.bbox_half_size_for(pack);
                    let p_min = placed.center - half;
                    let p_max = placed.center + half;
                    // AABB intersection test
                    if p_min.x < max_x && p_max.x > min_x && p_min.y < max_y && p_max.y > min_y {
                        selected_indices.push(i);
                    }
                }

                if !selected_indices.is_empty() {
                    // Compute anchor as center of selection box
                    let anchor = vec2((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
                    let mut offsets = Vec::new();
                    let mut pre_centers = Vec::new();
                    for &idx in &selected_indices {
                        offsets.push(ctx.build.placed_packs[idx].center - anchor);
                        pre_centers.push(ctx.build.placed_packs[idx].center);
                    }
                    ctx.build.multi_dragging = selected_indices;
                    ctx.build.multi_drag_offsets = offsets;
                    ctx.build.multi_drag_pre_centers = pre_centers;
                    ctx.build.selected_pack = None;
                }
            }
            ctx.build.drag_box_start = None;
        }
        // Right-click cancels the drag box
        if right_click {
            ctx.build.drag_box_start = None;
        }
    }
    // Start drag-box when clicking empty space (not on a pack, not on UI)
    else if left_click && screen_mouse.x > shop_w() && !click_consumed
        && ctx.build.dragging.is_none() && ctx.build.multi_dragging.is_empty()
        && ctx.build.pack_at(mouse).is_none() && ctx.build.selected_pack.is_none()
    {
        ctx.build.drag_box_start = Some(mouse);
    }

    // === Single-pack click-to-hold/place logic ===
    if ctx.build.multi_dragging.is_empty() && ctx.build.drag_box_start.is_none() {
    if let Some(drag_idx) = ctx.build.dragging {
        // Currently holding a pack — follow mouse
        let pack = &all_packs()[ctx.build.placed_packs[drag_idx].pack_index];
        let half = ctx.build.placed_packs[drag_idx].bbox_half_size_for(pack);
        let (dmin, dmax) = ctx.role.deploy_x_range();
        let clamped = vec2(
            mouse.x.clamp(dmin + half.x, dmax - half.x),
            mouse.y.clamp(half.y, ARENA_H - half.y),
        );
        // Snap to grid
        let grid = terrain::GRID_CELL;
        let snapped = vec2(
            (clamped.x / grid).round() * grid,
            (clamped.y / grid).round() * grid,
        );
        ctx.build.placed_packs[drag_idx].center = snapped;
        ctx.build.reposition_pack_units(drag_idx, &mut ctx.units);

        if left_click {
            let placed = &ctx.build.placed_packs[drag_idx];
            let pack_index = placed.pack_index;
            let rotated = placed.rotated;
            let old_center = placed.pre_drag_center;
            if ctx.build.would_overlap(placed.center, pack_index, Some(drag_idx), rotated) {
                ctx.build.placed_packs[drag_idx].center = old_center;
                ctx.build.reposition_pack_units(drag_idx, &mut ctx.units);
            } else if ctx.build.placed_packs[drag_idx].center != old_center {
                ctx.build.undo_history.push(game_state::UndoEntry::Move { placed_index: drag_idx, old_center });
            }
            ctx.build.dragging = None;
        }

        if right_click {
            let prev = ctx.build.placed_packs[drag_idx].pre_drag_center;
            ctx.build.placed_packs[drag_idx].center = prev;
            ctx.build.reposition_pack_units(drag_idx, &mut ctx.units);
            ctx.build.dragging = None;
        }
    } else if left_click && screen_mouse.x > shop_w() && !click_consumed {
        // Not holding — selection logic
        if let Some(placed_idx) = ctx.build.pack_at(mouse) {
            if ctx.build.selected_pack == Some(placed_idx) {
                // Already selected -> pick up (if not locked)
                if !ctx.build.placed_packs[placed_idx].locked {
                    ctx.build.placed_packs[placed_idx].pre_drag_center =
                        ctx.build.placed_packs[placed_idx].center;
                    ctx.build.dragging = Some(placed_idx);
                    ctx.build.selected_pack = None;
                }
            } else {
                // Select this pack
                ctx.build.selected_pack = Some(placed_idx);
            }
        } else if let Some(sel_idx) = ctx.build.selected_pack {
            // Clicked empty space with a selected pack -> pick it up (if not locked)
            if !ctx.build.placed_packs[sel_idx].locked {
                ctx.build.placed_packs[sel_idx].pre_drag_center =
                    ctx.build.placed_packs[sel_idx].center;
                ctx.build.dragging = Some(sel_idx);
                ctx.build.selected_pack = None;
            } else {
                ctx.build.selected_pack = None;
            }
        }
    }
    } // end multi_dragging.is_empty() guard

    // Begin Round button (screen-space UI)
    let btn_w = crate::ui::s(160.0);
    let btn_h = crate::ui::s(40.0);
    let btn_x = screen_width() / 2.0 - btn_w / 2.0;
    let btn_y = screen_height() - crate::ui::s(55.0);
    if left_click
        && screen_mouse.x >= btn_x
        && screen_mouse.x <= btn_x + btn_w
        && screen_mouse.y >= btn_y
        && screen_mouse.y <= btn_y + btn_h
    {
        if ctx.net.is_some() {
            // Multiplayer: send ctx.build data, wait for opponent
            net::send_build_complete(&mut ctx.net, &ctx.build);
            ctx.phase = GamePhase::WaitingForOpponent;
        } else {
            // Single-player: start battle with AI
            ctx.phase = economy::start_ai_battle(
                &mut ctx.units,
                &mut battle.projectiles,
                &mut ctx.progress,
                &mut ctx.obstacles,
                &mut ctx.nav_grid,
                &ctx.game_settings,
            );
            battle.reset();
        }
    }
}
