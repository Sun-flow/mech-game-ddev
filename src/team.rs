use macroquad::prelude::*;

pub const TEAM_COLORS: &[Color] = &[
    Color::new(0.9, 0.2, 0.2, 1.0), // Red
    Color::new(0.2, 0.4, 0.9, 1.0), // Blue
    Color::new(0.2, 0.8, 0.3, 1.0), // Green
    Color::new(0.9, 0.8, 0.2, 1.0), // Yellow
];

pub fn team_color(team_id: u8) -> Color {
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
