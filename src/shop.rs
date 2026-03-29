use macroquad::prelude::*;

use crate::arena::shop_w;
use crate::pack::all_packs;
use crate::ui::s;

/// Draw the shop panel and return the index of a clicked pack (if any).
/// Packs whose kind is in `banned` are hidden from the shop.
pub fn draw_shop(gold_remaining: u32, mouse_pos: Vec2, clicked: bool, banned: &[crate::unit::UnitKind], builds_remaining: u32) -> Option<usize> {
    let packs = all_packs();
    let button_h = s(42.0);
    let button_margin = s(4.0);
    let shop_top = s(50.0);
    let pad = s(4.0);
    let at_limit = builds_remaining == 0;

    // Semi-transparent background
    draw_rectangle(0.0, 0.0, shop_w(), screen_height(), Color::new(0.05, 0.05, 0.08, 0.85));
    draw_line(shop_w(), 0.0, shop_w(), screen_height(), 1.0, Color::new(0.3, 0.3, 0.35, 1.0));

    // Title + build limit
    crate::ui::draw_scaled_text("SHOP", s(10.0), s(18.0), 20.0, WHITE);
    crate::ui::draw_scaled_text(
        &format!("Gold: {}", gold_remaining),
        s(10.0),
        s(34.0),
        14.0,
        Color::new(1.0, 0.85, 0.2, 1.0),
    );
    let limit_color = if at_limit { Color::new(1.0, 0.3, 0.2, 1.0) } else { Color::new(0.5, 0.8, 0.5, 1.0) };
    crate::ui::draw_scaled_text(
        &format!("Builds: {}", builds_remaining),
        s(90.0),
        s(34.0),
        14.0,
        limit_color,
    );

    let mut clicked_pack = None;
    let mut hovered_pack: Option<usize> = None;

    let mut slot = 0;
    for (i, pack) in packs.iter().enumerate() {
        if banned.contains(&pack.kind) {
            continue;
        }
        let y = shop_top + slot as f32 * (button_h + button_margin);
        slot += 1;
        let affordable = gold_remaining >= pack.cost && !at_limit;
        let hovered = mouse_pos.x >= pad
            && mouse_pos.x <= shop_w() - pad
            && mouse_pos.y >= y
            && mouse_pos.y <= y + button_h;

        // Button background
        let bg = if !affordable {
            Color::new(0.15, 0.15, 0.18, 0.8)
        } else if hovered {
            Color::new(0.25, 0.3, 0.35, 0.9)
        } else {
            Color::new(0.18, 0.2, 0.24, 0.9)
        };
        draw_rectangle(pad, y, shop_w() - pad * 2.0, button_h, bg);

        // Tier color stripe
        let tier_color = match pack.cost {
            100 => Color::new(0.5, 0.5, 0.5, 1.0),
            200 => Color::new(0.3, 0.6, 1.0, 1.0),
            300 => Color::new(1.0, 0.7, 0.2, 1.0),
            _ => WHITE,
        };
        draw_rectangle(pad, y, pad, button_h, tier_color);

        // Text
        let text_color = if affordable { WHITE } else { DARKGRAY };
        crate::ui::draw_scaled_text(pack.name, s(14.0), y + s(17.0), 16.0, text_color);
        crate::ui::draw_scaled_text(
            &format!("x{} - {}g", pack.count(), pack.cost),
            s(14.0),
            y + s(33.0),
            13.0,
            if affordable {
                LIGHTGRAY
            } else {
                Color::new(0.3, 0.3, 0.3, 1.0)
            },
        );

        // Border on hover
        if hovered && affordable {
            draw_rectangle_lines(pad, y, shop_w() - pad * 2.0, button_h, 2.0, tier_color);
        }

        if hovered {
            hovered_pack = Some(i);
        }

        if hovered && affordable && clicked {
            clicked_pack = Some(i);
        }
    }

    // Draw tooltip for hovered pack
    if let Some(idx) = hovered_pack {
        let pack = &packs[idx];
        let stats = pack.kind.stats();

        let tip_x = shop_w() + s(6.0);
        let tip_y = mouse_pos.y.clamp(s(40.0), screen_height() - s(200.0));
        let tip_w = s(185.0);
        let mut tip_h = s(110.0);
        if stats.splash_radius > 0.0 { tip_h += s(14.0); }
        if stats.shield_radius > 0.0 { tip_h += s(14.0); }

        // Background
        draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.08, 0.08, 0.12, 0.95));
        draw_rectangle_lines(tip_x, tip_y, tip_w, tip_h, 1.0, Color::new(0.4, 0.5, 0.6, 0.7));

        let mut ty = tip_y + s(16.0);
        let lx = tip_x + s(8.0);
        let label_col = Color::new(0.6, 0.6, 0.6, 1.0);
        let val_col = Color::new(0.9, 0.9, 0.9, 1.0);
        let line_h = s(14.0);

        // Name
        crate::ui::draw_scaled_text(pack.name, lx, ty, 16.0, WHITE);
        ty += s(18.0);

        crate::ui::draw_scaled_text(&format!("HP: {:.0}   DMG: {:.0}", stats.max_hp, stats.damage), lx, ty, 12.0, val_col);
        ty += line_h;

        crate::ui::draw_scaled_text(&format!("AS: {:.1}   RNG: {:.0}", stats.attack_speed, stats.attack_range), lx, ty, 12.0, val_col);
        ty += line_h;

        crate::ui::draw_scaled_text(&format!("SPD: {:.0}   ARM: {:.0}", stats.move_speed, stats.armor), lx, ty, 12.0, val_col);
        ty += line_h;

        let dps = stats.damage * stats.attack_speed;
        let pack_dps = dps * pack.count() as f32;
        crate::ui::draw_scaled_text(&format!("DPS/unit: {:.0}  Pack: {:.0}", dps, pack_dps), lx, ty, 12.0, Color::new(1.0, 0.8, 0.5, 1.0));
        ty += line_h;

        let total_hp = stats.max_hp * pack.count() as f32;
        crate::ui::draw_scaled_text(&format!("Total HP: {:.0}", total_hp), lx, ty, 12.0, label_col);
        ty += line_h;

        if stats.splash_radius > 0.0 {
            crate::ui::draw_scaled_text(&format!("Splash: {:.0}", stats.splash_radius), lx, ty, 12.0, Color::new(1.0, 0.5, 0.3, 0.9));
            ty += line_h;
        }

        if stats.shield_radius > 0.0 {
            crate::ui::draw_scaled_text(&format!("Shield: {:.0}", stats.shield_radius), lx, ty, 12.0, Color::new(0.3, 0.7, 1.0, 0.9));
        }
    }

    clicked_pack
}
