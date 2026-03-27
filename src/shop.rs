use macroquad::prelude::*;

use crate::arena::SHOP_W;
use crate::pack::all_packs;

const BUTTON_H: f32 = 42.0;
const BUTTON_MARGIN: f32 = 4.0;
const SHOP_TOP: f32 = 35.0;

/// Draw the shop panel and return the index of a clicked pack (if any).
pub fn draw_shop(gold_remaining: u32, mouse_pos: Vec2, clicked: bool) -> Option<usize> {
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

    for (i, pack) in packs.iter().enumerate() {
        let y = SHOP_TOP + i as f32 * (BUTTON_H + BUTTON_MARGIN);
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
            100 => Color::new(0.5, 0.5, 0.5, 1.0),   // T1 gray
            200 => Color::new(0.3, 0.6, 1.0, 1.0),    // T2 blue
            300 => Color::new(1.0, 0.7, 0.2, 1.0),    // T3 gold
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

        if hovered && affordable && clicked {
            clicked_pack = Some(i);
        }
    }

    clicked_pack
}
