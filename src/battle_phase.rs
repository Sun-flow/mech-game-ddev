use crate::projectile::Projectile;
use crate::rendering::SplashEffect;

pub const FIXED_DT: f32 = 1.0 / 60.0;
pub const ROUND_TIMEOUT: f32 = 90.0;
pub const SYNC_INTERVAL: u32 = 4;

pub struct BattleState {
    pub accumulator: f32,
    pub timer: f32,
    pub frame: u32,
    pub recent_hashes: std::collections::VecDeque<(u32, u64)>,
    pub show_surrender_confirm: bool,
    pub waiting_for_round_end: bool,
    pub round_end_timeout: f32,
    pub projectiles: Vec<Projectile>,
    pub splash_effects: Vec<SplashEffect>,
}

impl BattleState {
    pub fn new() -> Self {
        Self {
            accumulator: 0.0,
            timer: 0.0,
            frame: 0,
            recent_hashes: std::collections::VecDeque::with_capacity(5),
            show_surrender_confirm: false,
            waiting_for_round_end: false,
            round_end_timeout: 0.0,
            projectiles: Vec::new(),
            splash_effects: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.accumulator = 0.0;
        self.timer = 0.0;
        self.frame = 0;
        self.recent_hashes.clear();
        self.show_surrender_confirm = false;
        self.waiting_for_round_end = false;
        self.round_end_timeout = 0.0;
        self.projectiles.clear();
        self.splash_effects.clear();
    }
}
