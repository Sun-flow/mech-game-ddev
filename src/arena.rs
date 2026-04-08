use macroquad::prelude::*;
use crate::unit::Unit;

pub const ARENA_W: f32 = 1680.0;
pub const ARENA_H: f32 = 960.0;
pub const HALF_W: f32 = ARENA_W / 2.0;
pub const SHOP_W_BASE: f32 = 180.0;
/// Dynamic shop width that scales with window size.
pub fn shop_w() -> f32 { SHOP_W_BASE * crate::ui::ui_scale() }

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum MatchState {
    InProgress,
    Winner(u16),
    Draw,
}

/// Check if the match is over.
pub fn check_match_state(units: &[Unit]) -> MatchState {
    let mut alive_players: std::collections::HashSet<u16> = std::collections::HashSet::new();
    for u in units {
        if u.alive {
            alive_players.insert(u.player_id);
        }
    }

    match alive_players.len() {
        0 => MatchState::Draw,
        1 => {
            let winner = *alive_players.iter().next().unwrap();
            MatchState::Winner(winner)
        }
        _ => MatchState::InProgress,
    }
}

pub fn draw_center_divider() {
    let dash_len = 10.0;
    let gap_len = 8.0;
    let color = Color::new(0.3, 0.3, 0.35, 0.4);
    let mut y = 0.0;
    while y < ARENA_H {
        let end = (y + dash_len).min(ARENA_H);
        draw_line(HALF_W, y, HALF_W, end, 1.0, color);
        y += dash_len + gap_len;
    }
}
