use macroquad::prelude::*;
use std::cell::Cell;

thread_local! {
    static TEXT_SCALE: Cell<f32> = Cell::new(1.0);
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
