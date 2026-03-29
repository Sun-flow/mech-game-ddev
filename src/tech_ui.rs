use macroquad::prelude::*;

use crate::pack::all_packs;
use crate::tech::{TechId, TechState};
use crate::ui::s;
use crate::unit::{Unit, UnitKind};

fn panel_w() -> f32 { s(210.0) }
fn panel_x() -> f32 { s(490.0) }
fn panel_top() -> f32 { s(30.0) }
fn item_h() -> f32 { s(32.0) }
fn item_margin() -> f32 { s(3.0) }

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
        let mut st = PackCombatStats {
            damage_dealt_round: 0.0,
            damage_dealt_total: 0.0,
            damage_soaked_round: 0.0,
            damage_soaked_total: 0.0,
            kills_total: 0,
        };
        for unit in units {
            if unit_ids.contains(&unit.id) {
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

/// Draw the tech panel for a selected pack. Returns a TechId if one was clicked for purchase.
pub fn draw_tech_panel(
    kind: UnitKind,
    tech_state: &TechState,
    gold: u32,
    mouse: Vec2,
    clicked: bool,
    combat_stats: Option<&PackCombatStats>,
) -> Option<TechId> {
    let px = panel_x();
    let pw = panel_w();
    let pt = panel_top();
    let ih = item_h();
    let im = item_margin();
    let pad = s(8.0);
    let line_h = s(15.0);

    let packs = all_packs();
    let pack = packs.iter().find(|p| p.kind == kind);
    let pack_name = pack.map_or("Unknown", |p| p.name);

    let mut stats = kind.stats();
    tech_state.apply_to_stats(kind, &mut stats);

    let available = tech_state.available_techs(kind);
    let purchased_count = tech_state.tech_count(kind);
    let has_combat_stats = combat_stats.map_or(false, |cs| cs.damage_dealt_total > 0.0 || cs.damage_soaked_total > 0.0);
    let combat_lines = if has_combat_stats { 5 } else { 0 };
    let ph = s(120.0)
        + (available.len() + purchased_count) as f32 * (ih + im)
        + combat_lines as f32 * line_h
        + if has_combat_stats { s(30.0) } else { 0.0 }
        + s(20.0);

    draw_rectangle(px, pt, pw, ph, Color::new(0.08, 0.08, 0.12, 0.92));
    draw_rectangle_lines(px, pt, pw, ph, 1.5, Color::new(0.4, 0.6, 0.8, 0.8));

    let mut y = pt + pad;

    crate::ui::draw_scaled_text(pack_name, px + pad, y + s(14.0), 20.0, WHITE);
    y += s(22.0);

    let stat_lines = [
        format!("HP:{:.0} DMG:{:.0} RNG:{:.0}", stats.max_hp, stats.damage, stats.attack_range),
        format!("AS:{:.1} SPD:{:.0} ARM:{:.0}", stats.attack_speed, stats.move_speed, stats.armor),
    ];
    for line in &stat_lines {
        crate::ui::draw_scaled_text(line, px + pad, y + s(12.0), 13.0, LIGHTGRAY);
        y += line_h;
    }
    if stats.splash_radius > 0.0 {
        crate::ui::draw_scaled_text(&format!("Splash:{:.0}", stats.splash_radius), px + pad, y + s(12.0), 13.0, LIGHTGRAY);
        y += line_h;
    }
    if stats.shield_radius > 0.0 {
        crate::ui::draw_scaled_text(&format!("Shield:{:.0}", stats.shield_radius), px + pad, y + s(12.0), 13.0, Color::new(0.3, 0.7, 1.0, 1.0));
        y += line_h;
    }

    // Combat stats section
    if has_combat_stats {
        let ps = combat_stats.unwrap();
        y += s(4.0);
        draw_line(px + pad, y, px + pw - pad, y, 1.0, Color::new(0.3, 0.3, 0.4, 0.6));
        y += s(6.0);
        crate::ui::draw_scaled_text("COMBAT STATS", px + pad, y + s(12.0), 13.0, Color::new(0.8, 0.7, 0.5, 1.0));
        y += s(18.0);

        let stat_color = Color::new(0.7, 0.7, 0.7, 0.9);
        let val_color = Color::new(1.0, 0.9, 0.6, 1.0);
        let val_x = px + s(120.0);

        crate::ui::draw_scaled_text("Dmg (round):", px + pad, y + s(12.0), 12.0, stat_color);
        crate::ui::draw_scaled_text(&format!("{:.0}", ps.damage_dealt_round), val_x, y + s(12.0), 12.0, val_color);
        y += line_h;
        crate::ui::draw_scaled_text("Dmg (total):", px + pad, y + s(12.0), 12.0, stat_color);
        crate::ui::draw_scaled_text(&format!("{:.0}", ps.damage_dealt_total), val_x, y + s(12.0), 12.0, val_color);
        y += line_h;
        crate::ui::draw_scaled_text("Soaked (round):", px + pad, y + s(12.0), 12.0, stat_color);
        crate::ui::draw_scaled_text(&format!("{:.0}", ps.damage_soaked_round), val_x, y + s(12.0), 12.0, val_color);
        y += line_h;
        crate::ui::draw_scaled_text("Soaked (total):", px + pad, y + s(12.0), 12.0, stat_color);
        crate::ui::draw_scaled_text(&format!("{:.0}", ps.damage_soaked_total), val_x, y + s(12.0), 12.0, val_color);
        y += line_h;
        crate::ui::draw_scaled_text("Kills (total):", px + pad, y + s(12.0), 12.0, stat_color);
        crate::ui::draw_scaled_text(&format!("{}", ps.kills_total), val_x, y + s(12.0), 12.0, Color::new(1.0, 0.4, 0.3, 1.0));
        y += line_h;
    }

    y += pad;
    draw_line(px + pad, y, px + pw - pad, y, 1.0, Color::new(0.3, 0.3, 0.4, 0.6));
    y += s(6.0);
    crate::ui::draw_scaled_text("UPGRADES", px + pad, y + s(12.0), 15.0, Color::new(0.7, 0.8, 1.0, 1.0));
    y += s(20.0);

    let cost = tech_state.effective_cost(kind);
    let mut clicked_tech = None;
    let btn_pad = s(4.0);

    for tech_def in &available {
        let affordable = gold >= cost;
        let btn_y = y;
        let hovered = mouse.x >= px + btn_pad
            && mouse.x <= px + pw - btn_pad
            && mouse.y >= btn_y
            && mouse.y <= btn_y + ih;

        let bg = if !affordable { Color::new(0.12, 0.12, 0.15, 0.8) }
            else if hovered { Color::new(0.2, 0.3, 0.4, 0.9) }
            else { Color::new(0.15, 0.17, 0.22, 0.9) };
        draw_rectangle(px + btn_pad, btn_y, pw - btn_pad * 2.0, ih, bg);

        let text_color = if affordable { WHITE } else { DARKGRAY };
        crate::ui::draw_scaled_text(tech_def.name, px + s(10.0), btn_y + s(13.0), 14.0, text_color);
        crate::ui::draw_scaled_text(tech_def.description, px + s(10.0), btn_y + s(26.0), 11.0,
            if affordable { Color::new(0.6, 0.6, 0.6, 1.0) } else { Color::new(0.3, 0.3, 0.3, 1.0) });

        let cost_text = format!("{}g", cost);
        let cost_color = if affordable { Color::new(1.0, 0.85, 0.2, 1.0) } else { Color::new(0.4, 0.3, 0.1, 1.0) };
        let cdims = crate::ui::measure_scaled_text(&cost_text, 13);
        crate::ui::draw_scaled_text(&cost_text, px + pw - s(12.0) - cdims.width, btn_y + s(20.0), 13.0, cost_color);

        if hovered && affordable {
            draw_rectangle_lines(px + btn_pad, btn_y, pw - btn_pad * 2.0, ih, 1.5, Color::new(0.3, 0.7, 1.0, 0.8));
        }
        if hovered && affordable && clicked { clicked_tech = Some(tech_def.id); }

        y += ih + im;
    }

    if purchased_count > 0 {
        y += s(4.0);
        crate::ui::draw_scaled_text("PURCHASED", px + pad, y + s(12.0), 12.0, Color::new(0.5, 0.5, 0.5, 0.7));
        y += s(18.0);
        let purchased_list = tech_state.purchased.get(&kind).cloned().unwrap_or_default();
        for tech_id in &purchased_list {
            if let Some(tech_def) = crate::tech::all_techs().iter().find(|t| t.id == *tech_id) {
                draw_rectangle(px + btn_pad, y, pw - btn_pad * 2.0, ih - s(6.0), Color::new(0.1, 0.15, 0.1, 0.7));
                crate::ui::draw_scaled_text(&format!("  {} ", tech_def.name), px + s(10.0), y + s(16.0), 13.0, Color::new(0.4, 0.7, 0.4, 0.8));
                y += ih - s(2.0);
            }
        }
    }

    clicked_tech
}
