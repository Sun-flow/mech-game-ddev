use crate::unit::Unit;

pub const ARENA_W: f32 = 1400.0;
pub const ARENA_H: f32 = 800.0;
pub const HALF_W: f32 = ARENA_W / 2.0;
pub const SHOP_W: f32 = 180.0;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum MatchState {
    InProgress,
    Winner(u8),
    Draw,
}

/// Check if the match is over.
pub fn check_match_state(units: &[Unit]) -> MatchState {
    let mut team_alive = [false; 4];
    for u in units {
        if u.alive {
            team_alive[u.team_id as usize] = true;
        }
    }

    let alive_count = team_alive.iter().filter(|&&a| a).count();
    match alive_count {
        0 => MatchState::Draw,
        1 => {
            let winner = team_alive.iter().position(|&a| a).unwrap() as u8;
            MatchState::Winner(winner)
        }
        _ => MatchState::InProgress,
    }
}
