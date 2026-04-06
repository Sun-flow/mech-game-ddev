use crate::arena::{ARENA_W, HALF_W};

/// Canonical player identity within a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Player 0 — always builds on the left half.
    Host,
    /// Player 1 — always builds on the right half (camera flipped later).
    Guest,
    /// Observer — no build zone, cannot act.
    Spectator,
}

impl Role {
    /// The canonical player_id (0 for Host, 1 for Guest).
    pub fn player_id(self) -> u8 {
        match self {
            Role::Host => 0,
            Role::Guest => 1,
            Role::Spectator => 255,
        }
    }

    /// Deploy zone x-range for this role.
    pub fn deploy_x_range(self) -> (f32, f32) {
        match self {
            Role::Host => (0.0, HALF_W),
            Role::Guest => (HALF_W, ARENA_W),
            Role::Spectator => (0.0, 0.0),
        }
    }

    /// The opponent's player_id.
    pub fn opponent_id(self) -> u8 {
        match self {
            Role::Host => 1,
            Role::Guest => 0,
            Role::Spectator => 255,
        }
    }
}
