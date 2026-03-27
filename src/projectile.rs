use macroquad::prelude::*;

#[derive(Clone, Debug)]
pub struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub damage: f32,
    pub team_id: u8,
    pub splash_radius: f32,
    pub lifetime: f32,
    pub alive: bool,
}

const MAX_LIFETIME: f32 = 3.0;
pub const PROJECTILE_RADIUS: f32 = 3.0;

impl Projectile {
    pub fn new(origin: Vec2, target_pos: Vec2, speed: f32, damage: f32, team_id: u8, splash_radius: f32) -> Self {
        let dir = (target_pos - origin).normalize_or_zero();
        Self {
            pos: origin,
            vel: dir * speed,
            damage,
            team_id,
            splash_radius,
            lifetime: MAX_LIFETIME,
            alive: true,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.pos += self.vel * dt;
        self.lifetime -= dt;
        if self.lifetime <= 0.0 {
            self.alive = false;
        }
    }
}
