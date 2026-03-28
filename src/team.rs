use macroquad::prelude::*;
use std::sync::atomic::{AtomicU8, Ordering};

pub const TEAM_COLORS: &[Color] = &[
    Color::new(0.9, 0.2, 0.2, 1.0), // Red
    Color::new(0.2, 0.4, 0.9, 1.0), // Blue
    Color::new(0.2, 0.8, 0.3, 1.0), // Green
    Color::new(0.9, 0.8, 0.2, 1.0), // Yellow
];

/// Player color override index (255 = no override, use default).
static PLAYER_COLOR_OVERRIDE: AtomicU8 = AtomicU8::new(255);

/// Set the player's custom team color index (from settings).
pub fn set_player_color(index: u8) {
    PLAYER_COLOR_OVERRIDE.store(index, Ordering::Relaxed);
}

pub fn team_color(team_id: u8) -> Color {
    if team_id == 0 {
        let idx = PLAYER_COLOR_OVERRIDE.load(Ordering::Relaxed);
        if idx < crate::settings::TEAM_COLOR_OPTIONS.len() as u8 {
            let (_, (r, g, b)) = crate::settings::TEAM_COLOR_OPTIONS[idx as usize];
            return Color::new(r, g, b, 1.0);
        }
    }
    TEAM_COLORS
        .get(team_id as usize)
        .copied()
        .unwrap_or(WHITE)
}

pub fn team_projectile_color(team_id: u8) -> Color {
    let base = team_color(team_id);
    Color::new(
        (base.r + 0.3).min(1.0),
        (base.g + 0.3).min(1.0),
        (base.b + 0.3).min(1.0),
        1.0,
    )
}
