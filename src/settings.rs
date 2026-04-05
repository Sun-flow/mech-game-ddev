use macroquad::prelude::*;
use serde::{Serialize, Deserialize};

/// Persistent settings (across matches) — not synced with opponent.
#[derive(Clone, Debug)]
pub struct MainSettings {
    /// UI text/element scale (0.75 to 2.0, default 1.0)
    pub ui_scale: f32,
}

impl MainSettings {
    pub fn default() -> Self {
        Self { ui_scale: 1.0 }
    }
}

/// Draw a slider for UI scale in the lobby settings screen. Returns true if value changed.
pub fn draw_ui_scale_slider(main_settings: &mut MainSettings, mouse: Vec2, clicked: bool, left_down: bool, panel_x: f32, y: f32) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static SLIDER_ACTIVE: AtomicBool = AtomicBool::new(false);

    let label = "Text Size";
    crate::ui::draw_scaled_text(label, panel_x + 20.0, y + 17.0, 18.0, WHITE);

    let slider_x = panel_x + 140.0;
    let slider_w = 200.0;
    let slider_y = y + 6.0;
    let slider_h = 12.0;

    // Track
    draw_rectangle(slider_x, slider_y, slider_w, slider_h, Color::new(0.2, 0.2, 0.25, 0.8));
    draw_rectangle_lines(slider_x, slider_y, slider_w, slider_h, 1.0, Color::new(0.4, 0.4, 0.5, 0.8));

    // Knob position
    let frac = (main_settings.ui_scale - 0.75) / (2.0 - 0.75);
    let knob_x = slider_x + frac * slider_w;
    let knob_r = 8.0;
    draw_circle(knob_x, slider_y + slider_h / 2.0, knob_r, Color::new(0.9, 0.9, 0.95, 1.0));

    // Value label
    let val_text = format!("{:.2}x", main_settings.ui_scale);
    crate::ui::draw_scaled_text(&val_text, slider_x + slider_w + 12.0, y + 17.0, 16.0, LIGHTGRAY);

    // Start drag only if click lands on the slider
    let on_slider = mouse.x >= slider_x - knob_r && mouse.x <= slider_x + slider_w + knob_r
        && mouse.y >= slider_y - knob_r && mouse.y <= slider_y + slider_h + knob_r;
    if clicked && on_slider {
        SLIDER_ACTIVE.store(true, Ordering::Relaxed);
    }
    if !left_down {
        SLIDER_ACTIVE.store(false, Ordering::Relaxed);
    }

    if SLIDER_ACTIVE.load(Ordering::Relaxed) {
        let t = ((mouse.x - slider_x) / slider_w).clamp(0.0, 1.0);
        // Snap to 0.05 increments
        let raw = 0.75 + t * (2.0 - 0.75);
        main_settings.ui_scale = (raw / 0.05).round() * 0.05;
        main_settings.ui_scale = main_settings.ui_scale.clamp(0.75, 2.0);
    }
}

/// Game settings configurable in the lobby before starting a match.
/// Gameplay-changing settings are toggleable; visual/UX features are always on.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameSettings {
    /// Enable terrain obstacles on the battlefield
    pub terrain_enabled: bool,
    /// Allow destructible cover (in addition to indestructible walls)
    pub terrain_destructible: bool,
    /// Enable draft/ban phase where each player bans 2 unit types
    pub draft_ban_enabled: bool,
    /// Enable smarter AI (counter-picking, balanced compositions)
    pub smart_ai: bool,
    /// Player's chosen team color index (0-5)
    pub player_color_index: u8,
}

impl GameSettings {
    pub fn default() -> Self {
        Self {
            terrain_enabled: false,
            terrain_destructible: true,
            draft_ban_enabled: false,
            smart_ai: false,
            player_color_index: 0, // Red by default
        }
    }
}

/// Available team colors for customization
pub const TEAM_COLOR_OPTIONS: &[(&str, (f32, f32, f32))] = &[
    ("Red",    (0.9, 0.2, 0.2)),
    ("Blue",   (0.2, 0.4, 0.9)),
    ("Green",  (0.2, 0.8, 0.3)),
    ("Yellow", (0.9, 0.8, 0.2)),
    ("Purple", (0.7, 0.2, 0.9)),
    ("Orange", (0.9, 0.5, 0.1)),
];

/// Draw a row of color swatches and return the clicked index (if any).
/// `disabled_index` dims and crosses out a color (e.g., host's color in guest picker).
pub fn draw_color_swatches(
    selected: u8,
    mouse: Vec2,
    clicked: bool,
    center_x: f32,
    y: f32,
    swatch_size: f32,
    swatch_gap: f32,
    disabled_index: Option<u8>,
) -> Option<u8> {
    let colors = TEAM_COLOR_OPTIONS;
    let total_w = colors.len() as f32 * swatch_size + (colors.len() - 1) as f32 * swatch_gap;
    let sx_start = center_x - total_w / 2.0;
    let mut clicked_color = None;

    for (i, (name, (r, g, b))) in colors.iter().enumerate() {
        let sx = sx_start + i as f32 * (swatch_size + swatch_gap);
        let is_disabled = disabled_index == Some(i as u8);
        let is_selected = i as u8 == selected;
        let is_hovered = mouse.x >= sx && mouse.x <= sx + swatch_size && mouse.y >= y && mouse.y <= y + swatch_size;

        if is_disabled {
            draw_rectangle(sx, y, swatch_size, swatch_size, Color::new(*r * 0.3, *g * 0.3, *b * 0.3, 0.5));
            draw_line(sx, y, sx + swatch_size, y + swatch_size, 2.0, Color::new(1.0, 0.3, 0.3, 0.7));
            draw_line(sx + swatch_size, y, sx, y + swatch_size, 2.0, Color::new(1.0, 0.3, 0.3, 0.7));
        } else {
            draw_rectangle(sx, y, swatch_size, swatch_size, Color::new(*r, *g, *b, 1.0));
            if is_selected {
                draw_rectangle_lines(sx - 2.0, y - 2.0, swatch_size + 4.0, swatch_size + 4.0, 3.0, WHITE);
            } else if is_hovered {
                draw_rectangle_lines(sx - 1.0, y - 1.0, swatch_size + 2.0, swatch_size + 2.0, 2.0, Color::new(0.7, 0.7, 0.7, 0.8));
            }
            if clicked && is_hovered {
                clicked_color = Some(i as u8);
            }
        }

        let ndims = crate::ui::measure_scaled_text(name, 11);
        let label_color = if is_disabled { Color::new(0.4, 0.4, 0.4, 0.5) } else { LIGHTGRAY };
        crate::ui::draw_scaled_text(name, sx + swatch_size / 2.0 - ndims.width / 2.0, y + swatch_size + 14.0, 11.0, label_color);
    }

    clicked_color
}

/// Draw the match settings panel overlay. Returns true if "Start" was clicked.
pub fn draw_settings_panel(settings: &mut GameSettings, mouse: Vec2, clicked: bool) -> bool {
    let sw = screen_width();
    let sh = screen_height();

    // Dark overlay
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.5));

    // Panel
    let panel_w = crate::ui::s(400.0);
    let panel_h = crate::ui::s(420.0);
    let px = sw / 2.0 - panel_w / 2.0;
    let py = sh / 2.0 - panel_h / 2.0;
    draw_rectangle(px, py, panel_w, panel_h, Color::new(0.1, 0.1, 0.15, 0.95));
    draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, Color::new(0.4, 0.4, 0.5, 1.0));

    // Title
    let title = "Match Settings";
    let tdims = crate::ui::measure_scaled_text(title, 24);
    crate::ui::draw_scaled_text(title, px + panel_w / 2.0 - tdims.width / 2.0, py + 30.0, 24.0, WHITE);

    let mut y = py + crate::ui::s(55.0);
    let row_h = crate::ui::s(40.0);
    let toggle_x = px + panel_w - crate::ui::s(70.0);
    let toggle_w = crate::ui::s(50.0);
    let toggle_h = crate::ui::s(24.0);
    let label_x = px + crate::ui::s(20.0);

    let mut back_clicked = false;

    // Toggle helper
    struct Toggle {
        label: &'static str,
        enabled: bool,
        active: bool, // can the toggle be interacted with?
    }

    let toggles = [
        Toggle { label: "Terrain", enabled: settings.terrain_enabled, active: true },
        Toggle { label: "Destructible Cover", enabled: settings.terrain_destructible, active: settings.terrain_enabled },
        Toggle { label: "Draft/Ban Phase", enabled: settings.draft_ban_enabled, active: true },
        Toggle { label: "Smart AI", enabled: settings.smart_ai, active: true },
    ];

    for (i, toggle) in toggles.iter().enumerate() {
        let row_y = y + i as f32 * row_h;
        let text_alpha = if toggle.active { 1.0 } else { 0.4 };
        crate::ui::draw_scaled_text(toggle.label, label_x, row_y + 17.0, 18.0, Color::new(1.0, 1.0, 1.0, text_alpha));

        // Toggle switch
        let tx = toggle_x;
        let ty = row_y + 3.0;
        let bg = if toggle.enabled && toggle.active {
            Color::new(0.2, 0.7, 0.3, 0.9)
        } else {
            Color::new(0.3, 0.3, 0.35, 0.7)
        };
        draw_rectangle(tx, ty, toggle_w, toggle_h, bg);
        draw_rectangle_lines(tx, ty, toggle_w, toggle_h, 1.0, Color::new(0.5, 0.5, 0.6, 0.8));

        // Slider knob
        let knob_x = if toggle.enabled { tx + toggle_w - toggle_h } else { tx };
        draw_rectangle(knob_x, ty, toggle_h, toggle_h, Color::new(0.9, 0.9, 0.95, 1.0));

        // On/Off label
        let state_text = if toggle.enabled { "ON" } else { "OFF" };
        crate::ui::draw_scaled_text(state_text, tx + toggle_w + 8.0, row_y + 17.0, 12.0, Color::new(0.6, 0.6, 0.6, text_alpha));

        // Click to toggle
        if toggle.active && clicked && mouse.x >= tx && mouse.x <= tx + toggle_w && mouse.y >= ty && mouse.y <= ty + toggle_h {
            match i {
                0 => settings.terrain_enabled = !settings.terrain_enabled,
                1 => settings.terrain_destructible = !settings.terrain_destructible,
                2 => settings.draft_ban_enabled = !settings.draft_ban_enabled,
                3 => settings.smart_ai = !settings.smart_ai,
                _ => {}
            }
        }
    }

    // Team color picker
    y += 4.0 * row_h + 10.0;
    crate::ui::draw_scaled_text("Team Color", label_x, y + 17.0, 18.0, WHITE);
    y += 30.0;

    let swatch_size = crate::ui::s(36.0);
    let swatch_gap = crate::ui::s(12.0);

    if let Some(color_idx) = draw_color_swatches(
        settings.player_color_index, mouse, clicked,
        px + panel_w / 2.0, y, swatch_size, swatch_gap, None,
    ) {
        settings.player_color_index = color_idx;
    }

    // Back button
    let back_w = crate::ui::s(120.0);
    let back_h = crate::ui::s(36.0);
    let back_x = px + panel_w / 2.0 - back_w / 2.0;
    let back_y = py + panel_h - crate::ui::s(50.0);
    let back_hover = mouse.x >= back_x && mouse.x <= back_x + back_w && mouse.y >= back_y && mouse.y <= back_y + back_h;
    let back_bg = if back_hover { Color::new(0.3, 0.3, 0.35, 0.9) } else { Color::new(0.2, 0.2, 0.25, 0.8) };
    draw_rectangle(back_x, back_y, back_w, back_h, back_bg);
    draw_rectangle_lines(back_x, back_y, back_w, back_h, 1.0, GRAY);
    let bt = "Start Game";
    let bdims = crate::ui::measure_scaled_text(bt, 20);
    crate::ui::draw_scaled_text(bt, back_x + back_w / 2.0 - bdims.width / 2.0, back_y + back_h / 2.0 + 6.0, 20.0, WHITE);

    if clicked && back_hover {
        back_clicked = true;
    }

    back_clicked
}
