use macroquad::prelude::*;
use std::cell::Cell;

use crate::arena::shop_w;
use crate::match_progress::MatchProgress;
use crate::role::Role;

thread_local! {
    static TEXT_SCALE: Cell<f32> = const { Cell::new(1.0) };
}

/// Reference resolution the UI was designed for.
const REF_W: f32 = 1400.0;

/// Set the global text scale factor (called once per frame from main_settings).
pub fn set_text_scale(scale: f32) {
    TEXT_SCALE.with(|s| s.set(scale));
}

/// Get the current text scale factor.
pub fn text_scale() -> f32 {
    TEXT_SCALE.with(|s| s.get())
}

/// Window-relative UI scale: ratio of current window width to reference width.
/// At 1400px wide, returns 1.0. At 2800px, returns 2.0. At 700px, returns 0.5.
pub fn ui_scale() -> f32 {
    screen_width() / REF_W
}

/// Scale a pixel value by the window-relative UI scale.
/// Use for all UI element dimensions, positions, and spacing.
pub fn s(px: f32) -> f32 {
    px * ui_scale()
}

/// Draw text with both text_scale (user preference) and ui_scale (window size) applied.
pub fn draw_scaled_text(text: &str, x: f32, y: f32, base_font_size: f32, color: Color) {
    let scaled = base_font_size * text_scale() * ui_scale();
    draw_text(text, x, y, scaled, color);
}

/// Measure text with both text_scale and ui_scale applied.
pub fn measure_scaled_text(text: &str, base_font_size: u16) -> TextDimensions {
    let scaled = (base_font_size as f32 * text_scale() * ui_scale()) as u16;
    measure_text(text, None, scaled, 1.0)
}

pub fn draw_hud(progress: &MatchProgress, gold: u32, timer: f32, army_value: u32, battle_remaining: f32, player_name: &str, opponent_name: &str, role: Role) {
    let player_lp = progress.player(role).lp;
    let opponent_lp = progress.opponent(role).lp;

    // Background bar (screen-wide)
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        s(28.0),
        Color::new(0.05, 0.05, 0.08, 0.85),
    );

    // Spread HUD elements evenly across available width
    let hud_left = shop_w() + s(15.0);
    let hud_y = s(19.0);
    let gap = s(30.0); // padding between elements

    let mut x = hud_left;

    // Round
    let round_text = format!("Round: {}", progress.round);
    let round_w = measure_scaled_text(&round_text, 18).width;
    draw_scaled_text(&round_text, x, hud_y, 18.0, WHITE);
    x += round_w + gap;

    // Player LP
    let player_lp_text = format!("{} LP: {}", player_name, player_lp);
    let plp_color = if player_lp > 500 {
        Color::new(0.3, 1.0, 0.4, 1.0)
    } else if player_lp > 200 {
        Color::new(1.0, 0.8, 0.2, 1.0)
    } else {
        Color::new(1.0, 0.3, 0.2, 1.0)
    };
    let plp_w = measure_scaled_text(&player_lp_text, 18).width;
    draw_scaled_text(&player_lp_text, x, hud_y, 18.0, plp_color);
    x += plp_w + gap;

    // Opponent LP
    let opponent_lp_text = format!("{} LP: {}", opponent_name, opponent_lp);
    let alp_color = if opponent_lp > 500 {
        Color::new(0.3, 0.6, 1.0, 1.0)
    } else if opponent_lp > 200 {
        Color::new(1.0, 0.8, 0.2, 1.0)
    } else {
        Color::new(1.0, 0.3, 0.2, 1.0)
    };
    let alp_w = measure_scaled_text(&opponent_lp_text, 18).width;
    draw_scaled_text(&opponent_lp_text, x, hud_y, 18.0, alp_color);
    x += alp_w + gap;

    // Gold (only during build)
    if gold > 0 || timer > 0.0 {
        let gold_text = format!("Gold: {}", gold);
        let gold_w = measure_scaled_text(&gold_text, 18).width;
        draw_scaled_text(&gold_text, x, hud_y, 18.0, Color::new(1.0, 0.85, 0.2, 1.0));
        x += gold_w + gap;

        if army_value > 0 {
            let army_text = format!("Army: {}g", army_value);
            let army_w = measure_scaled_text(&army_text, 16).width;
            draw_scaled_text(&army_text, x, hud_y, 16.0, Color::new(0.7, 0.7, 0.75, 0.8));
            x += army_w + gap;
        }

        if timer > 0.0 {
            let timer_text = format!("Timer: {:.0}s", timer.ceil());
            draw_scaled_text(&timer_text, x, hud_y, 18.0, WHITE);
        }
    }

    // Battle round timer (shown during combat)
    if battle_remaining > 0.0 && battle_remaining < 90.0 {
        let timer_color = if battle_remaining < 15.0 { Color::new(1.0, 0.3, 0.2, 1.0) }
            else if battle_remaining < 30.0 { Color::new(1.0, 0.8, 0.2, 1.0) }
            else { Color::new(0.7, 0.7, 0.7, 1.0) };
        let timer_text = format!("Round: {:.0}s", battle_remaining.ceil());
        draw_scaled_text(&timer_text, x, hud_y, 18.0, timer_color);
    }
}
