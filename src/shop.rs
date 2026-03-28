use macroquad::prelude::*;

use crate::arena::SHOP_W;
use crate::pack::all_packs;

const BUTTON_H: f32 = 42.0;
const BUTTON_MARGIN: f32 = 4.0;
const SHOP_TOP: f32 = 35.0;

/// Draw the shop panel and return the index of a clicked pack (if any).
/// Packs whose kind is in `banned` are hidden from the shop.
pub fn draw_shop(gold_remaining: u32, mouse_pos: Vec2, clicked: bool, banned: &[crate::unit::UnitKind]) -> Option<usize> {
    let packs = all_packs();

    // Semi-transparent background
    draw_rectangle(0.0, 0.0, SHOP_W, 800.0, Color::new(0.05, 0.05, 0.08, 0.85));
    draw_line(SHOP_W, 0.0, SHOP_W, 800.0, 1.0, Color::new(0.3, 0.3, 0.35, 1.0));

    // Title
    draw_text("SHOP", 10.0, 25.0, 24.0, WHITE);
    draw_text(
        &format!("Gold: {}", gold_remaining),
        90.0,
        25.0,
        20.0,
        Color::new(1.0, 0.85, 0.2, 1.0),
    );

    let mut clicked_pack = None;
    let mut hovered_pack: Option<usize> = None;

    let mut slot = 0;
    for (i, pack) in packs.iter().enumerate() {
        if banned.contains(&pack.kind) {
            continue;
        }
        let y = SHOP_TOP + slot as f32 * (BUTTON_H + BUTTON_MARGIN);
        slot += 1;
        let affordable = gold_remaining >= pack.cost;
        let hovered = mouse_pos.x >= 4.0
            && mouse_pos.x <= SHOP_W - 4.0
            && mouse_pos.y >= y
            && mouse_pos.y <= y + BUTTON_H;

        // Button background
        let bg = if !affordable {
            Color::new(0.15, 0.15, 0.18, 0.8)
        } else if hovered {
            Color::new(0.25, 0.3, 0.35, 0.9)
        } else {
            Color::new(0.18, 0.2, 0.24, 0.9)
        };
        draw_rectangle(4.0, y, SHOP_W - 8.0, BUTTON_H, bg);

        // Tier color stripe
        let tier_color = match pack.cost {
            100 => Color::new(0.5, 0.5, 0.5, 1.0),
            200 => Color::new(0.3, 0.6, 1.0, 1.0),
            300 => Color::new(1.0, 0.7, 0.2, 1.0),
            _ => WHITE,
        };
        draw_rectangle(4.0, y, 4.0, BUTTON_H, tier_color);

        // Text
        let text_color = if affordable { WHITE } else { DARKGRAY };
        draw_text(pack.name, 14.0, y + 17.0, 16.0, text_color);
        draw_text(
            &format!("x{} - {}g", pack.count(), pack.cost),
            14.0,
            y + 33.0,
            13.0,
            if affordable {
                LIGHTGRAY
            } else {
                Color::new(0.3, 0.3, 0.3, 1.0)
            },
        );

        // Border on hover
        if hovered && affordable {
            draw_rectangle_lines(4.0, y, SHOP_W - 8.0, BUTTON_H, 2.0, tier_color);
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

        let tip_x = SHOP_W + 6.0;
        let tip_y = mouse_pos.y.clamp(40.0, 600.0);
        let tip_w = 185.0;
        let mut tip_h = 110.0;
        if stats.splash_radius > 0.0 { tip_h += 14.0; }
        if stats.shield_radius > 0.0 { tip_h += 14.0; }

        // Background
        draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.08, 0.08, 0.12, 0.95));
        draw_rectangle_lines(tip_x, tip_y, tip_w, tip_h, 1.0, Color::new(0.4, 0.5, 0.6, 0.7));

        let mut ty = tip_y + 16.0;
        let lx = tip_x + 8.0;
        let label_col = Color::new(0.6, 0.6, 0.6, 1.0);
        let val_col = Color::new(0.9, 0.9, 0.9, 1.0);

        // Name
        draw_text(pack.name, lx, ty, 16.0, WHITE);
        ty += 18.0;

        // HP / DMG
        draw_text(&format!("HP: {:.0}   DMG: {:.0}", stats.max_hp, stats.damage), lx, ty, 12.0, val_col);
        ty += 14.0;

        // AS / Range
        draw_text(&format!("AS: {:.1}   RNG: {:.0}", stats.attack_speed, stats.attack_range), lx, ty, 12.0, val_col);
        ty += 14.0;

        // Speed / Armor
        draw_text(&format!("SPD: {:.0}   ARM: {:.0}", stats.move_speed, stats.armor), lx, ty, 12.0, val_col);
        ty += 14.0;

        // DPS per unit
        let dps = stats.damage * stats.attack_speed;
        let pack_dps = dps * pack.count() as f32;
        draw_text(&format!("DPS/unit: {:.0}  Pack: {:.0}", dps, pack_dps), lx, ty, 12.0, Color::new(1.0, 0.8, 0.5, 1.0));
        ty += 14.0;

        // Total HP
        let total_hp = stats.max_hp * pack.count() as f32;
        draw_text(&format!("Total HP: {:.0}", total_hp), lx, ty, 12.0, label_col);
        ty += 14.0;

        // Splash
        if stats.splash_radius > 0.0 {
            draw_text(&format!("Splash: {:.0}", stats.splash_radius), lx, ty, 12.0, Color::new(1.0, 0.5, 0.3, 0.9));
            ty += 14.0;
        }

        // Shield
        if stats.shield_radius > 0.0 {
            draw_text(&format!("Shield: {:.0}", stats.shield_radius), lx, ty, 12.0, Color::new(0.3, 0.7, 1.0, 0.9));
        }
    }

    clicked_pack
}
