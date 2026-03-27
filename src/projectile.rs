use macroquad::prelude::*;

use crate::unit::ProjectileType;

#[derive(Clone, Debug)]
pub struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub origin: Vec2,
    pub max_range: f32,
    pub damage: f32,
    pub team_id: u8,
    pub splash_radius: f32,
    pub alive: bool,
    pub proj_type: ProjectileType,
    // Tech effect flags
    pub armor_pierce: bool,
    pub pierce_remaining: u8,
    pub applies_slow: bool,
    pub attacker_id: u64,
}

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
        attack_range: f32,
    ) -> Self {
        let dir = (target_pos - origin).normalize_or_zero();
        Self {
            pos: origin,
            vel: dir * speed,
            origin,
            max_range: attack_range * 1.1, // expire at 110% of attack range
            damage,
            team_id,
            splash_radius,
            alive: true,
            proj_type,
            armor_pierce: false,
            pierce_remaining: 0,
            applies_slow: false,
            attacker_id: 0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.pos += self.vel * dt;
        // Kill projectile if it has traveled beyond max range
        if self.pos.distance(self.origin) > self.max_range {
            self.alive = false;
        }
    }

    pub fn is_rocket(&self) -> bool {
        self.proj_type == ProjectileType::Rocket
    }
}
