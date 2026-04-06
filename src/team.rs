use macroquad::prelude::*;
use std::sync::atomic::{AtomicU8, Ordering};

pub const TEAM_COLORS: &[Color] = &[
    Color::new(0.9, 0.2, 0.2, 1.0), // Red
    Color::new(0.2, 0.4, 0.9, 1.0), // Blue
    Color::new(0.2, 0.8, 0.3, 1.0), // Green
    Color::new(0.9, 0.8, 0.2, 1.0), // Yellow
];

/// Canonical color override per player_id (255 = no override, use default).
static COLOR_OVERRIDE_0: AtomicU8 = AtomicU8::new(255);
static COLOR_OVERRIDE_1: AtomicU8 = AtomicU8::new(255);

/// Set the color override for a given player_id.
pub fn set_color(player_id: u8, index: u8) {
    match player_id {
        0 => COLOR_OVERRIDE_0.store(index, Ordering::Relaxed),
        1 => COLOR_OVERRIDE_1.store(index, Ordering::Relaxed),
        _ => {}
    }
}

pub fn team_color(player_id: u8) -> Color {
    let options = crate::settings::TEAM_COLOR_OPTIONS;
    let override_idx = match player_id {
        0 => COLOR_OVERRIDE_0.load(Ordering::Relaxed),
        1 => COLOR_OVERRIDE_1.load(Ordering::Relaxed),
        _ => 255,
    };
    if (override_idx as usize) < options.len() {
        let (_, (r, g, b)) = options[override_idx as usize];
        return Color::new(r, g, b, 1.0);
    }
    TEAM_COLORS
        .get(player_id as usize)
        .copied()
        .unwrap_or(WHITE)
}

/// Get the color override index for a player_id (255 if no override).
pub fn color_index(player_id: u8) -> u8 {
    match player_id {
        0 => COLOR_OVERRIDE_0.load(Ordering::Relaxed),
        1 => COLOR_OVERRIDE_1.load(Ordering::Relaxed),
        _ => 255,
    }
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
