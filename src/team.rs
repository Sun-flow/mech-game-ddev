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
/// Opponent color override index (255 = no override, use default).
static OPPONENT_COLOR_OVERRIDE: AtomicU8 = AtomicU8::new(255);

/// Set the player's custom team color index (from settings).
pub fn set_player_color(index: u8) {
    PLAYER_COLOR_OVERRIDE.store(index, Ordering::Relaxed);
}

/// Set the opponent's color index (received over network).
pub fn set_opponent_color(index: u8) {
    OPPONENT_COLOR_OVERRIDE.store(index, Ordering::Relaxed);
}

pub fn team_color(player_id: u8) -> Color {
    let options = crate::settings::TEAM_COLOR_OPTIONS;
    if player_id == 0 {
        let idx = PLAYER_COLOR_OVERRIDE.load(Ordering::Relaxed);
        if (idx as usize) < options.len() {
            let (_, (r, g, b)) = options[idx as usize];
            return Color::new(r, g, b, 1.0);
        }
    }
    if player_id == 1 {
        let idx = OPPONENT_COLOR_OVERRIDE.load(Ordering::Relaxed);
        if (idx as usize) < options.len() {
            let (_, (r, g, b)) = options[idx as usize];
            return Color::new(r, g, b, 1.0);
        }
    }
    TEAM_COLORS
        .get(player_id as usize)
        .copied()
        .unwrap_or(WHITE)
}

pub fn team_projectile_color(player_id: u8) -> Color {
    let base = team_color(player_id);
    Color::new(
        (base.r + 0.3).min(1.0),
        (base.g + 0.3).min(1.0),
        (base.b + 0.3).min(1.0),
        1.0,
    )
}
