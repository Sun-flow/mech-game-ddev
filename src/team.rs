use macroquad::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

pub const TEAM_COLORS: &[Color] = &[
    Color::new(0.9, 0.2, 0.2, 1.0), // Red
    Color::new(0.2, 0.4, 0.9, 1.0), // Blue
    Color::new(0.2, 0.8, 0.3, 1.0), // Green
    Color::new(0.9, 0.8, 0.2, 1.0), // Yellow
];

thread_local! {
    static COLOR_OVERRIDES: RefCell<HashMap<u16, u8>> = RefCell::new(HashMap::new());
}

pub fn set_color(player_id: u16, index: u8) {
    COLOR_OVERRIDES.with(|c| c.borrow_mut().insert(player_id, index));
}

pub fn team_color(player_id: u16) -> Color {
    let options = crate::settings::TEAM_COLOR_OPTIONS;
    let override_idx = COLOR_OVERRIDES.with(|c| c.borrow().get(&player_id).copied());
    if let Some(idx) = override_idx {
        if (idx as usize) < options.len() {
            let (_, (r, g, b)) = options[idx as usize];
            return Color::new(r, g, b, 1.0);
        }
    }
    let fallback_idx = (player_id as usize) % TEAM_COLORS.len();
    TEAM_COLORS[fallback_idx]
}

pub fn color_index(player_id: u16) -> u8 {
    COLOR_OVERRIDES.with(|c| c.borrow().get(&player_id).copied().unwrap_or(255))
}

pub fn team_projectile_color(player_id: u16) -> Color {
    let base = team_color(player_id);
    Color::new(
        (base.r + 0.3).min(1.0),
        (base.g + 0.3).min(1.0),
        (base.b + 0.3).min(1.0),
        1.0,
    )
}
