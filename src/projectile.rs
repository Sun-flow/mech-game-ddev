use macroquad::prelude::*;

use crate::unit::ProjectileType;

#[derive(Clone, Debug)]
pub struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub damage: f32,
    pub team_id: u8,
    pub splash_radius: f32,
    pub lifetime: f32,
    pub alive: bool,
    pub proj_type: ProjectileType,
}

const MAX_LIFETIME: f32 = 3.0;
pub const PROJECTILE_RADIUS: f32 = 3.0;

/// Visual radius varies by projectile type.
pub fn projectile_visual_radius(proj_type: ProjectileType) -> f32 {
    match proj_type {
        ProjectileType::Bullet => 3.0,
        ProjectileType::Laser => 2.0,
        ProjectileType::Rocket => 5.0,
    }
}

impl Projectile {
    pub fn new(
        origin: Vec2,
        target_pos: Vec2,
        speed: f32,
        damage: f32,
        team_id: u8,
        splash_radius: f32,
        proj_type: ProjectileType,
    ) -> Self {
        let dir = (target_pos - origin).normalize_or_zero();
        Self {
            pos: origin,
            vel: dir * speed,
            damage,
            team_id,
            splash_radius,
            lifetime: MAX_LIFETIME,
            alive: true,
            proj_type,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.pos += self.vel * dt;
        self.lifetime -= dt;
        if self.lifetime <= 0.0 {
            self.alive = false;
        }
    }

    pub fn is_rocket(&self) -> bool {
        self.proj_type == ProjectileType::Rocket
    }
}
