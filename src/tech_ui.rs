use std::collections::HashSet;

use macroquad::prelude::*;

use crate::pack::all_packs;
use crate::tech::{TechId, TechState};
use crate::ui::s;
use crate::unit::{Unit, UnitKind};

// Panel occupies the right ~44% of screen, flush with bottom-right corner.
fn panel_w() -> f32 { s(616.0) }
fn stats_w() -> f32 { s(85.0) }
fn header_h() -> f32 { s(26.0) }
fn card_h() -> f32 { s(38.0) }
fn card_gap() -> f32 { s(4.0) }
fn pad() -> f32 { s(8.0) }
const CARDS_PER_ROW: usize = 3;

/// Aggregated combat stats for a pack's units.
pub struct PackCombatStats {
    pub damage_dealt_round: f32,
    pub damage_dealt_total: f32,
    pub damage_soaked_round: f32,
    pub damage_soaked_total: f32,
    pub kills_total: u32,
}

impl PackCombatStats {
    pub fn from_units(units: &[Unit], unit_ids: &[u64]) -> Self {
        let id_set: HashSet<u64> = unit_ids.iter().copied().collect();
        let mut st = PackCombatStats {
            damage_dealt_round: 0.0,
            damage_dealt_total: 0.0,
            damage_soaked_round: 0.0,
            damage_soaked_total: 0.0,
            kills_total: 0,
        };
        for unit in units {
            if id_set.contains(&unit.id) {
                st.damage_dealt_round += unit.damage_dealt_round;
                st.damage_dealt_total += unit.damage_dealt_total;
                st.damage_soaked_round += unit.damage_soaked_round;
                st.damage_soaked_total += unit.damage_soaked_total;
                st.kills_total += unit.kills_total;
            }
        }
        st
    }
}

/// Returns the bounding rectangle (x, y, w, h) of the tech panel for hit-testing.
pub fn panel_rect(kind: UnitKind, tech_state: &TechState) -> (f32, f32, f32, f32) {
    let pw = panel_w();
    let total_techs = tech_state.available_techs(kind).len() + tech_state.tech_count(kind);
    let tech_rows = if total_techs == 0 { 0 } else { total_techs.div_ceil(CARDS_PER_ROW) };
    let body_h = if tech_rows == 0 {
        stats_column_height(kind)
    } else {
        let cards_h = tech_rows as f32 * card_h() + (tech_rows.saturating_sub(1)) as f32 * card_gap();
        cards_h.max(stats_column_height(kind))
    };
    let ph = header_h() + pad() + body_h + pad();
    let px = screen_width() - pw;
    let py = screen_height() - ph;
    (px, py, pw, ph)
}

/// Height of the stats column content.
fn stats_column_height(kind: UnitKind) -> f32 {
    let stats = kind.stats();
    let mut lines = 6; // HP, DMG, RNG, AS, SPD, ARM
    if stats.splash_radius > 0.0 { lines += 1; }
    if stats.shield_radius > 0.0 { lines += 1; }
    lines as f32 * s(14.0)
}

/// Draw the tech panel for a selected pack. Returns a TechId if one was clicked for purchase.
pub fn draw_tech_panel(
    kind: UnitKind,
    tech_state: &TechState,
    gold: u32,
    mouse: Vec2,
    clicked: bool,
    combat_stats: Option<&PackCombatStats>,
) -> Option<TechId> {
    let (px, py, pw, ph) = panel_rect(kind, tech_state);

    let packs = all_packs();
    let pack_name = packs.iter().find(|p| p.kind == kind).map_or("Unknown", |p| p.name);

    let mut stats = kind.stats();
    tech_state.apply_to_stats(kind, &mut stats);

    // Panel background
    draw_rectangle(px, py, pw, ph, Color::new(0.08, 0.08, 0.12, 0.92));
    draw_rectangle_lines(px, py, pw, ph, 1.5, Color::new(0.4, 0.6, 0.8, 0.8));

    // === Header row: unit name (left), combat stats (right) ===
    let hdr_y = py;
    draw_line(px, hdr_y + header_h(), px + pw, hdr_y + header_h(), 1.0, Color::new(0.3, 0.3, 0.4, 0.6));

    crate::ui::draw_scaled_text(pack_name, px + pad(), hdr_y + s(17.0), 15.0, WHITE);

    let has_combat = combat_stats.is_some_and(|cs| cs.damage_dealt_total > 0.0 || cs.damage_soaked_total > 0.0);
    if has_combat {
        let cs = combat_stats.unwrap();
        let combat_text = format!(
            "Dmg:{:.0}  Soaked:{:.0}  Kills:{}",
            cs.damage_dealt_total, cs.damage_soaked_total, cs.kills_total
        );
        let dims = crate::ui::measure_scaled_text(&combat_text, 11);
        crate::ui::draw_scaled_text(
            &combat_text,
            px + pw - pad() - dims.width,
            hdr_y + s(17.0),
            11.0,
            Color::new(0.8, 0.7, 0.5, 0.9),
        );
    }

    // === Body: stats sidebar (left) | tech cards (right) ===
    let body_y = hdr_y + header_h() + pad();
    let sw = stats_w();

    // Stats sidebar
    let stat_label_color = Color::new(0.65, 0.65, 0.65, 1.0);
    let stat_val_color = WHITE;
    let line_h = s(14.0);
    let label_x = px + pad();
    let val_x = px + s(45.0);
    let mut sy = body_y;

    let stat_rows: Vec<(&str, String)> = {
        let mut rows = vec![
            ("HP", format!("{:.0}", stats.max_hp)),
            ("DMG", format!("{:.0}", stats.damage)),
            ("RNG", format!("{:.0}", stats.attack_range)),
            ("AS", format!("{:.1}", stats.attack_speed)),
            ("SPD", format!("{:.0}", stats.move_speed)),
            ("ARM", format!("{:.0}", stats.armor)),
        ];
        if stats.splash_radius > 0.0 {
            rows.push(("SPLS", format!("{:.0}", stats.splash_radius)));
        }
        if stats.shield_radius > 0.0 {
            rows.push(("SHLD", format!("{:.0}", stats.shield_radius)));
        }
        rows
    };

    for (label, val) in &stat_rows {
        crate::ui::draw_scaled_text(label, label_x, sy + s(11.0), 11.0, stat_label_color);
        crate::ui::draw_scaled_text(val, val_x, sy + s(11.0), 11.0, stat_val_color);
        sy += line_h;
    }

    // Vertical divider between stats and techs
    let divider_x = px + sw;
    let body_bottom = py + ph - pad();
    draw_line(divider_x, body_y - s(2.0), divider_x, body_bottom, 1.0, Color::new(0.3, 0.3, 0.4, 0.5));

    // === Tech cards area ===
    let cards_x = divider_x + pad();
    let cards_area_w = px + pw - pad() - cards_x;

    let available = tech_state.available_techs(kind);
    let purchased_list = tech_state.purchased.get(&kind).cloned().unwrap_or_default();
    let cost = tech_state.effective_cost(kind);

    let mut clicked_tech = None;
    let mut card_idx = 0;

    // Available techs first
    for tech_def in &available {
        let row = card_idx / CARDS_PER_ROW;
        let col = card_idx % CARDS_PER_ROW;
        let card_w = (cards_area_w - (CARDS_PER_ROW - 1) as f32 * card_gap()) / CARDS_PER_ROW as f32;
        let cx = cards_x + col as f32 * (card_w + card_gap());
        let cy = body_y + row as f32 * (card_h() + card_gap());

        let affordable = gold >= cost;
        let hovered = mouse.x >= cx && mouse.x <= cx + card_w
            && mouse.y >= cy && mouse.y <= cy + card_h();

        let bg = if !affordable {
            Color::new(0.12, 0.12, 0.15, 0.8)
        } else if hovered {
            Color::new(0.2, 0.3, 0.4, 0.9)
        } else {
            Color::new(0.15, 0.17, 0.22, 0.9)
        };
        draw_rectangle(cx, cy, card_w, card_h(), bg);

        let text_color = if affordable { WHITE } else { DARKGRAY };
        crate::ui::draw_scaled_text(tech_def.name, cx + s(5.0), cy + s(13.0), 11.0, text_color);

        // Cost right-aligned on the name row
        let cost_text = format!("{}g", cost);
        let cost_color = if affordable {
            Color::new(1.0, 0.85, 0.2, 1.0)
        } else {
            Color::new(0.4, 0.3, 0.1, 1.0)
        };
        let cdims = crate::ui::measure_scaled_text(&cost_text, 11);
        crate::ui::draw_scaled_text(&cost_text, cx + card_w - s(5.0) - cdims.width, cy + s(13.0), 11.0, cost_color);

        // Description
        let desc_color = if affordable {
            Color::new(0.6, 0.6, 0.6, 1.0)
        } else {
            Color::new(0.3, 0.3, 0.3, 1.0)
        };
        crate::ui::draw_scaled_text(tech_def.description, cx + s(5.0), cy + s(27.0), 10.0, desc_color);

        if hovered && affordable {
            draw_rectangle_lines(cx, cy, card_w, card_h(), 1.5, Color::new(0.3, 0.7, 1.0, 0.8));
        }
        if hovered && affordable && clicked {
            clicked_tech = Some(tech_def.id);
        }

        card_idx += 1;
    }

    // Purchased techs
    for tech_id in &purchased_list {
        if let Some(tech_def) = crate::tech::all_techs().iter().find(|t| t.id == *tech_id) {
            let row = card_idx / CARDS_PER_ROW;
            let col = card_idx % CARDS_PER_ROW;
            let card_w = (cards_area_w - (CARDS_PER_ROW - 1) as f32 * card_gap()) / CARDS_PER_ROW as f32;
            let cx = cards_x + col as f32 * (card_w + card_gap());
            let cy = body_y + row as f32 * (card_h() + card_gap());

            draw_rectangle(cx, cy, card_w, card_h(), Color::new(0.1, 0.15, 0.1, 0.7));
            crate::ui::draw_scaled_text(
                &format!("\u{2713} {}", tech_def.name),
                cx + s(5.0), cy + s(13.0), 11.0,
                Color::new(0.4, 0.7, 0.4, 0.8),
            );
            crate::ui::draw_scaled_text(
                tech_def.description,
                cx + s(5.0), cy + s(27.0), 10.0,
                Color::new(0.3, 0.5, 0.3, 0.6),
            );

            card_idx += 1;
        }
    }

    clicked_tech
}
