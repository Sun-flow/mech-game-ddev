use macroquad::prelude::*;
use serde::{Serialize, Deserialize};

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

/// Draw the settings panel overlay. Returns true if "Back" was clicked.
pub fn draw_settings_panel(settings: &mut GameSettings, mouse: Vec2, clicked: bool) -> bool {
    let arena_w = crate::arena::ARENA_W;
    let arena_h = crate::arena::ARENA_H;

    // Dark overlay
    draw_rectangle(0.0, 0.0, arena_w, arena_h, Color::new(0.0, 0.0, 0.0, 0.5));

    // Panel
    let panel_w = 400.0;
    let panel_h = 420.0;
    let px = arena_w / 2.0 - panel_w / 2.0;
    let py = arena_h / 2.0 - panel_h / 2.0;
    draw_rectangle(px, py, panel_w, panel_h, Color::new(0.1, 0.1, 0.15, 0.95));
    draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, Color::new(0.4, 0.4, 0.5, 1.0));

    // Title
    let title = "Game Settings";
    let tdims = measure_text(title, None, 24, 1.0);
    draw_text(title, px + panel_w / 2.0 - tdims.width / 2.0, py + 30.0, 24.0, WHITE);

    let mut y = py + 55.0;
    let row_h = 40.0;
    let toggle_x = px + panel_w - 70.0;
    let toggle_w = 50.0;
    let toggle_h = 24.0;
    let label_x = px + 20.0;

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
        draw_text(toggle.label, label_x, row_y + 17.0, 18.0, Color::new(1.0, 1.0, 1.0, text_alpha));

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
        draw_text(state_text, tx + toggle_w + 8.0, row_y + 17.0, 12.0, Color::new(0.6, 0.6, 0.6, text_alpha));

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
    draw_text("Team Color", label_x, y + 17.0, 18.0, WHITE);
    y += 30.0;

    let swatch_size = 36.0;
    let swatch_gap = 12.0;
    let total_swatch_w = TEAM_COLOR_OPTIONS.len() as f32 * swatch_size + (TEAM_COLOR_OPTIONS.len() - 1) as f32 * swatch_gap;
    let swatch_start_x = px + panel_w / 2.0 - total_swatch_w / 2.0;

    for (i, (name, (r, g, b))) in TEAM_COLOR_OPTIONS.iter().enumerate() {
        let sx = swatch_start_x + i as f32 * (swatch_size + swatch_gap);
        let sy = y;
        let is_selected = i as u8 == settings.player_color_index;
        let is_hovered = mouse.x >= sx && mouse.x <= sx + swatch_size && mouse.y >= sy && mouse.y <= sy + swatch_size;

        draw_rectangle(sx, sy, swatch_size, swatch_size, Color::new(*r, *g, *b, 1.0));
        if is_selected {
            draw_rectangle_lines(sx - 2.0, sy - 2.0, swatch_size + 4.0, swatch_size + 4.0, 3.0, WHITE);
        } else if is_hovered {
            draw_rectangle_lines(sx - 1.0, sy - 1.0, swatch_size + 2.0, swatch_size + 2.0, 2.0, Color::new(0.7, 0.7, 0.7, 0.8));
        }

        // Color name below swatch
        let ndims = measure_text(name, None, 11, 1.0);
        draw_text(name, sx + swatch_size / 2.0 - ndims.width / 2.0, sy + swatch_size + 14.0, 11.0, LIGHTGRAY);

        if clicked && is_hovered {
            settings.player_color_index = i as u8;
        }
    }

    // Back button
    let back_w = 120.0;
    let back_h = 36.0;
    let back_x = px + panel_w / 2.0 - back_w / 2.0;
    let back_y = py + panel_h - 50.0;
    let back_hover = mouse.x >= back_x && mouse.x <= back_x + back_w && mouse.y >= back_y && mouse.y <= back_y + back_h;
    let back_bg = if back_hover { Color::new(0.3, 0.3, 0.35, 0.9) } else { Color::new(0.2, 0.2, 0.25, 0.8) };
    draw_rectangle(back_x, back_y, back_w, back_h, back_bg);
    draw_rectangle_lines(back_x, back_y, back_w, back_h, 1.0, GRAY);
    let bt = "Back";
    let bdims = measure_text(bt, None, 20, 1.0);
    draw_text(bt, back_x + back_w / 2.0 - bdims.width / 2.0, back_y + back_h / 2.0 + 6.0, 20.0, WHITE);

    if clicked && back_hover {
        back_clicked = true;
    }

    back_clicked
}
