use macroquad::prelude::*;

use crate::pack::all_packs;
use crate::tech::{TechId, TechState};
use crate::unit::{Unit, UnitKind};

const PANEL_W: f32 = 210.0;
const PANEL_X: f32 = 490.0;
const PANEL_TOP: f32 = 30.0;
const ITEM_H: f32 = 32.0;
const ITEM_MARGIN: f32 = 3.0;

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
        let mut s = PackCombatStats {
            damage_dealt_round: 0.0,
            damage_dealt_total: 0.0,
            damage_soaked_round: 0.0,
            damage_soaked_total: 0.0,
            kills_total: 0,
        };
        for unit in units {
            if unit_ids.contains(&unit.id) {
                s.damage_dealt_round += unit.damage_dealt_round;
                s.damage_dealt_total += unit.damage_dealt_total;
                s.damage_soaked_round += unit.damage_soaked_round;
                s.damage_soaked_total += unit.damage_soaked_total;
                s.kills_total += unit.kills_total;
            }
        }
        s
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
    let packs = all_packs();
    let pack = packs.iter().find(|p| p.kind == kind);
    let pack_name = pack.map_or("Unknown", |p| p.name);

    let mut stats = kind.stats();
    tech_state.apply_to_stats(kind, &mut stats);

    let available = tech_state.available_techs(kind);
    let purchased_count = tech_state.tech_count(kind);
    let has_combat_stats = combat_stats.map_or(false, |cs| cs.damage_dealt_total > 0.0 || cs.damage_soaked_total > 0.0);
    let combat_lines = if has_combat_stats { 5 } else { 0 };
    let panel_h = 120.0
        + (available.len() + purchased_count) as f32 * (ITEM_H + ITEM_MARGIN)
        + combat_lines as f32 * 15.0
        + if has_combat_stats { 30.0 } else { 0.0 }
        + 20.0;

    draw_rectangle(PANEL_X, PANEL_TOP, PANEL_W, panel_h, Color::new(0.08, 0.08, 0.12, 0.92));
    draw_rectangle_lines(PANEL_X, PANEL_TOP, PANEL_W, panel_h, 1.5, Color::new(0.4, 0.6, 0.8, 0.8));

    let mut y = PANEL_TOP + 8.0;

    draw_text(pack_name, PANEL_X + 8.0, y + 14.0, 20.0, WHITE);
    y += 22.0;

    let stat_lines = [
        format!("HP:{:.0} DMG:{:.0} RNG:{:.0}", stats.max_hp, stats.damage, stats.attack_range),
        format!("AS:{:.1} SPD:{:.0} ARM:{:.0}", stats.attack_speed, stats.move_speed, stats.armor),
    ];
    for line in &stat_lines {
        draw_text(line, PANEL_X + 8.0, y + 12.0, 13.0, LIGHTGRAY);
        y += 15.0;
    }
    if stats.splash_radius > 0.0 {
        draw_text(&format!("Splash:{:.0}", stats.splash_radius), PANEL_X + 8.0, y + 12.0, 13.0, LIGHTGRAY);
        y += 15.0;
    }
    if stats.shield_radius > 0.0 {
        draw_text(&format!("Shield:{:.0}", stats.shield_radius), PANEL_X + 8.0, y + 12.0, 13.0, Color::new(0.3, 0.7, 1.0, 1.0));
        y += 15.0;
    }

    // Combat stats section
    if has_combat_stats {
        let ps = combat_stats.unwrap();
        y += 4.0;
        draw_line(PANEL_X + 8.0, y, PANEL_X + PANEL_W - 8.0, y, 1.0, Color::new(0.3, 0.3, 0.4, 0.6));
        y += 6.0;
        draw_text("COMBAT STATS", PANEL_X + 8.0, y + 12.0, 13.0, Color::new(0.8, 0.7, 0.5, 1.0));
        y += 18.0;

        let stat_color = Color::new(0.7, 0.7, 0.7, 0.9);
        let val_color = Color::new(1.0, 0.9, 0.6, 1.0);

        draw_text("Dmg (round):", PANEL_X + 8.0, y + 12.0, 12.0, stat_color);
        draw_text(&format!("{:.0}", ps.damage_dealt_round), PANEL_X + 120.0, y + 12.0, 12.0, val_color);
        y += 15.0;
        draw_text("Dmg (total):", PANEL_X + 8.0, y + 12.0, 12.0, stat_color);
        draw_text(&format!("{:.0}", ps.damage_dealt_total), PANEL_X + 120.0, y + 12.0, 12.0, val_color);
        y += 15.0;
        draw_text("Soaked (round):", PANEL_X + 8.0, y + 12.0, 12.0, stat_color);
        draw_text(&format!("{:.0}", ps.damage_soaked_round), PANEL_X + 120.0, y + 12.0, 12.0, val_color);
        y += 15.0;
        draw_text("Soaked (total):", PANEL_X + 8.0, y + 12.0, 12.0, stat_color);
        draw_text(&format!("{:.0}", ps.damage_soaked_total), PANEL_X + 120.0, y + 12.0, 12.0, val_color);
        y += 15.0;
        draw_text("Kills (total):", PANEL_X + 8.0, y + 12.0, 12.0, stat_color);
        draw_text(&format!("{}", ps.kills_total), PANEL_X + 120.0, y + 12.0, 12.0, Color::new(1.0, 0.4, 0.3, 1.0));
        y += 15.0;
    }

    y += 8.0;
    draw_line(PANEL_X + 8.0, y, PANEL_X + PANEL_W - 8.0, y, 1.0, Color::new(0.3, 0.3, 0.4, 0.6));
    y += 6.0;
    draw_text("UPGRADES", PANEL_X + 8.0, y + 12.0, 15.0, Color::new(0.7, 0.8, 1.0, 1.0));
    y += 20.0;

    let cost = tech_state.effective_cost(kind);
    let mut clicked_tech = None;

    for tech_def in &available {
        let affordable = gold >= cost;
        let btn_y = y;
        let hovered = mouse.x >= PANEL_X + 4.0
            && mouse.x <= PANEL_X + PANEL_W - 4.0
            && mouse.y >= btn_y
            && mouse.y <= btn_y + ITEM_H;

        let bg = if !affordable { Color::new(0.12, 0.12, 0.15, 0.8) }
            else if hovered { Color::new(0.2, 0.3, 0.4, 0.9) }
            else { Color::new(0.15, 0.17, 0.22, 0.9) };
        draw_rectangle(PANEL_X + 4.0, btn_y, PANEL_W - 8.0, ITEM_H, bg);

        let text_color = if affordable { WHITE } else { DARKGRAY };
        draw_text(tech_def.name, PANEL_X + 10.0, btn_y + 13.0, 14.0, text_color);
        draw_text(tech_def.description, PANEL_X + 10.0, btn_y + 26.0, 11.0,
            if affordable { Color::new(0.6, 0.6, 0.6, 1.0) } else { Color::new(0.3, 0.3, 0.3, 1.0) });

        let cost_text = format!("{}g", cost);
        let cost_color = if affordable { Color::new(1.0, 0.85, 0.2, 1.0) } else { Color::new(0.4, 0.3, 0.1, 1.0) };
        let cdims = measure_text(&cost_text, None, 13, 1.0);
        draw_text(&cost_text, PANEL_X + PANEL_W - 12.0 - cdims.width, btn_y + 20.0, 13.0, cost_color);

        if hovered && affordable {
            draw_rectangle_lines(PANEL_X + 4.0, btn_y, PANEL_W - 8.0, ITEM_H, 1.5, Color::new(0.3, 0.7, 1.0, 0.8));
        }
        if hovered && affordable && clicked { clicked_tech = Some(tech_def.id); }

        y += ITEM_H + ITEM_MARGIN;
    }

    if purchased_count > 0 {
        y += 4.0;
        draw_text("PURCHASED", PANEL_X + 8.0, y + 12.0, 12.0, Color::new(0.5, 0.5, 0.5, 0.7));
        y += 18.0;
        let purchased_list = tech_state.purchased.get(&kind).cloned().unwrap_or_default();
        for tech_id in &purchased_list {
            if let Some(tech_def) = crate::tech::all_techs().iter().find(|t| t.id == *tech_id) {
                draw_rectangle(PANEL_X + 4.0, y, PANEL_W - 8.0, ITEM_H - 6.0, Color::new(0.1, 0.15, 0.1, 0.7));
                draw_text(&format!("  {} ", tech_def.name), PANEL_X + 10.0, y + 16.0, 13.0, Color::new(0.4, 0.7, 0.4, 0.8));
                y += ITEM_H - 2.0;
            }
        }
    }

    clicked_tech
}
