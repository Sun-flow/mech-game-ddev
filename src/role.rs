use crate::arena::{ARENA_W, HALF_W};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Role {
    Host,
    Guest,
    Spectator,
}

impl Role {
    pub fn deploy_x_range(&self) -> (f32, f32) {
        match self {
            Role::Host => (0.0, HALF_W),
            Role::Guest => (HALF_W, ARENA_W),
            Role::Spectator => (0.0, 0.0),
        }
    }

    pub fn player_id(&self) -> u8 {
        match self {
            Role::Host => 0,
            Role::Guest => 1,
            Role::Spectator => 255,
        }
    }
}
