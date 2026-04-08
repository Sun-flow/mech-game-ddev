use macroquad::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnitShape {
    Triangle,
    Square,
    Diamond,
    Circle,
    Hexagon,
    Pentagon,
    Dot,
    Star,
    Cross,
    Octagon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectileType {
    Bullet,
    Laser,
    Rocket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitKind {
    // Original units
    Striker,
    Sentinel,
    Ranger,
    Scout,
    Bruiser,
    Artillery,
    Chaff,
    // New units
    Sniper,
    Skirmisher,
    Dragoon,
    Berserker,
    Shield,
    Interceptor,
}

#[derive(Clone, Debug)]
pub struct UnitStats {
    pub max_hp: f32,
    pub damage: f32,
    pub attack_range: f32,
    pub attack_speed: f32,
    pub projectile_speed: f32,
    pub projectile_type: ProjectileType,
    pub move_speed: f32,
    pub size: f32,
    pub armor: f32,
    pub splash_radius: f32,
    pub shield_radius: f32,
    pub shield_hp: f32,       // barrier durability (separate from unit HP)
    pub min_attack_range: f32,
    pub shape: UnitShape,
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub id: u64,
    pub kind: UnitKind,
    pub stats: UnitStats,
    pub hp: f32,
    pub pos: Vec2,
    pub player_id: u16,
    pub target_id: Option<u64>,
    pub attack_cooldown: f32,
    pub alive: bool,
    pub death_timer: f32,
    // Pathfinding
    pub path: Vec<Vec2>,
    pub path_age: f32,
    // Behavioral tech fields
    pub slow_timer: f32,
    pub evasion_chance: f32,
    pub shield_hp: f32,       // current barrier durability
    // Stat tracking
    pub damage_dealt_round: f32,
    pub damage_dealt_total: f32,
    pub damage_soaked_round: f32,
    pub damage_soaked_total: f32,
    pub kills_total: u32,
}

impl Unit {
    pub fn new(id: u64, kind: UnitKind, pos: Vec2, player_id: u16) -> Self {
        let stats = kind.stats();
        let hp = stats.max_hp;
        let shield_hp = stats.shield_hp;
        Self {
            id,
            kind,
            stats,
            hp,
            pos,
            player_id,
            target_id: None,
            attack_cooldown: 0.0,
            alive: true,
            death_timer: 0.0,
            path: Vec::new(),
            path_age: 0.0,
            slow_timer: 0.0,
            evasion_chance: 0.0,
            shield_hp,
            damage_dealt_round: 0.0,
            damage_dealt_total: 0.0,
            damage_soaked_round: 0.0,
            damage_soaked_total: 0.0,
            kills_total: 0,
        }
    }

    pub fn take_damage(&mut self, raw_damage: f32) {
        let effective = (raw_damage - self.stats.armor).max(0.0);
        self.hp -= effective;
        self.damage_soaked_round += effective;
        self.damage_soaked_total += effective;
        if self.hp <= 0.0 {
            self.hp = 0.0;
            if self.alive {
                self.alive = false;
                self.death_timer = 0.5;
            }
        }
    }

    /// Take raw damage bypassing armor (for armor-pierce and cleave).
    pub fn take_raw_damage(&mut self, damage: f32) {
        self.hp -= damage;
        self.damage_soaked_round += damage;
        self.damage_soaked_total += damage;
        if self.hp <= 0.0 {
            self.hp = 0.0;
            if self.alive {
                self.alive = false;
                self.death_timer = 0.5;
            }
        }
    }

    pub fn is_melee(&self) -> bool {
        self.stats.projectile_speed <= 0.0
    }

    pub fn can_attack(&self) -> bool {
        self.attack_cooldown <= 0.0
    }

    /// Effective attack speed accounting for berserker rage scaling.
    pub fn effective_attack_speed(&self) -> f32 {
        if self.kind == UnitKind::Berserker {
            let hp_frac = self.hp / self.stats.max_hp;
            // At full HP: 1x attack speed, at 0% HP: 3x attack speed
            let multiplier = 1.0 + 2.0 * (1.0 - hp_frac);
            self.stats.attack_speed * multiplier
        } else {
            self.stats.attack_speed
        }
    }

    pub fn reset_cooldown(&mut self) {
        let effective_speed = self.effective_attack_speed();
        if effective_speed > 0.0 {
            self.attack_cooldown = 1.0 / effective_speed;
        }
    }

    pub fn update_cooldown(&mut self, dt: f32) {
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
    }

    pub fn is_shield(&self) -> bool {
        self.kind == UnitKind::Shield && self.alive && self.stats.shield_radius > 0.0
    }

    pub fn is_interceptor(&self) -> bool {
        self.kind == UnitKind::Interceptor
    }
}

impl UnitKind {
    pub fn stats(self) -> UnitStats {
        match self {
            // === ORIGINAL UNITS ===
            UnitKind::Striker => UnitStats {
                max_hp: 600.0,
                damage: 250.0,
                attack_range: 200.0,
                attack_speed: 1.5,
                projectile_speed: 400.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 120.0,
                size: 10.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 0.0,
                shape: UnitShape::Triangle,
            },
            UnitKind::Sentinel => UnitStats {
                max_hp: 2000.0,
                damage: 80.0,
                attack_range: 80.0,
                attack_speed: 0.8,
                projectile_speed: 0.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 60.0,
                size: 20.0,
                armor: 80.0,
                splash_radius: 15.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 0.0,
                shape: UnitShape::Square,
            },
            UnitKind::Ranger => UnitStats {
                max_hp: 700.0,
                damage: 180.0,
                attack_range: 350.0,
                attack_speed: 0.7,
                projectile_speed: 500.0,
                projectile_type: ProjectileType::Laser,
                move_speed: 80.0,
                size: 10.0,
                armor: 10.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 0.0,
                shape: UnitShape::Diamond,
            },
            UnitKind::Scout => UnitStats {
                max_hp: 500.0,
                damage: 100.0,
                attack_range: 120.0,
                attack_speed: 2.0,
                projectile_speed: 300.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 180.0,
                size: 10.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 0.0,
                shape: UnitShape::Circle,
            },
            UnitKind::Bruiser => UnitStats {
                max_hp: 1700.0,
                damage: 150.0,
                attack_range: 100.0,
                attack_speed: 1.0,
                projectile_speed: 0.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 90.0,
                size: 15.0,
                armor: 20.0,
                splash_radius: 25.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 0.0,
                shape: UnitShape::Hexagon,
            },
            UnitKind::Artillery => UnitStats {
                max_hp: 700.0,
                damage: 500.0,
                attack_range: 450.0,
                attack_speed: 0.4,
                projectile_speed: 300.0,
                projectile_type: ProjectileType::Rocket,
                move_speed: 50.0,
                size: 15.0,
                armor: 0.0,
                splash_radius: 40.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 150.0,
                shape: UnitShape::Pentagon,
            },
            UnitKind::Chaff => UnitStats {
                max_hp: 120.0,
                damage: 30.0,
                attack_range: 30.0,
                attack_speed: 1.5,
                projectile_speed: 0.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 150.0,
                size: 5.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 0.0,
                shape: UnitShape::Dot,
            },
            UnitKind::Sniper => UnitStats {
                max_hp: 400.0,
                damage: 1200.0,
                attack_range: 500.0,
                attack_speed: 0.25,
                projectile_speed: 1100.0,
                projectile_type: ProjectileType::Laser,
                move_speed: 40.0,
                size: 10.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 150.0,
                shape: UnitShape::Cross,
            },

            // Skirmisher: nerfed damage (25 instead of naive 40)
            UnitKind::Skirmisher => UnitStats {
                max_hp: 70.0,
                damage: 25.0,
                attack_range: 180.0,
                attack_speed: 2.5,
                projectile_speed: 350.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 160.0,
                size: 5.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 0.0,
                shape: UnitShape::Dot,
            },

            UnitKind::Dragoon => UnitStats {
                max_hp: 1000.0,
                damage: 200.0,
                attack_range: 150.0,
                attack_speed: 0.5,
                projectile_speed: 350.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 85.0,
                size: 15.0,
                armor: 40.0,
                splash_radius: 4.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 0.0,
                shape: UnitShape::Octagon,
            },

            // Berserker: melee, attack speed scales up as HP drops
            UnitKind::Berserker => UnitStats {
                max_hp: 900.0,
                damage: 220.0,
                attack_range: 60.0,
                attack_speed: 1.0,
                projectile_speed: 0.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 130.0,
                size: 15.0,
                armor: 20.0,
                splash_radius: 10.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 0.0,
                shape: UnitShape::Star,
            },

            UnitKind::Shield => UnitStats {
                max_hp: 1500.0,
                damage: 50.0,
                attack_range: 100.0,
                attack_speed: 0.5,
                projectile_speed: 0.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 55.0,
                size: 15.0,
                armor: 50.0,
                splash_radius: 0.0,
                shield_radius: 80.0,
                shield_hp: 3000.0,
                min_attack_range: 0.0,
                shape: UnitShape::Square,
            },

            UnitKind::Interceptor => UnitStats {
                max_hp: 600.0,
                damage: 120.0,
                attack_range: 250.0,
                attack_speed: 1.2,
                projectile_speed: 450.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 100.0,
                size: 10.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shield_hp: 0.0,
                min_attack_range: 0.0,
                shape: UnitShape::Diamond,
            },
        }
    }
}

pub fn draw_unit_shape(pos: Vec2, size: f32, shape: UnitShape, color: Color) {
    match shape {
        UnitShape::Circle => draw_circle(pos.x, pos.y, size, color),
        UnitShape::Square => {
            draw_rectangle(pos.x - size, pos.y - size, size * 2.0, size * 2.0, color)
        }
        UnitShape::Triangle => {
            draw_triangle(
                vec2(pos.x, pos.y - size),
                vec2(pos.x - size, pos.y + size),
                vec2(pos.x + size, pos.y + size),
                color,
            );
        }
        UnitShape::Diamond => {
            let top = vec2(pos.x, pos.y - size * 1.3);
            let right = vec2(pos.x + size, pos.y);
            let bottom = vec2(pos.x, pos.y + size * 1.3);
            let left = vec2(pos.x - size, pos.y);
            draw_triangle(top, right, bottom, color);
            draw_triangle(top, left, bottom, color);
        }
        UnitShape::Hexagon => draw_poly(pos.x, pos.y, 6, size, 0.0, color),
        UnitShape::Pentagon => draw_poly(pos.x, pos.y, 5, size, 0.0, color),
        UnitShape::Dot => draw_circle(pos.x, pos.y, size, color),
        UnitShape::Star => {
            let s = size;
            draw_triangle(
                vec2(pos.x, pos.y - s),
                vec2(pos.x - s * 0.87, pos.y + s * 0.5),
                vec2(pos.x + s * 0.87, pos.y + s * 0.5),
                color,
            );
            draw_triangle(
                vec2(pos.x, pos.y + s),
                vec2(pos.x - s * 0.87, pos.y - s * 0.5),
                vec2(pos.x + s * 0.87, pos.y - s * 0.5),
                color,
            );
        }
        UnitShape::Cross => {
            let arm = size * 0.35;
            draw_rectangle(pos.x - arm, pos.y - size, arm * 2.0, size * 2.0, color);
            draw_rectangle(pos.x - size, pos.y - arm, size * 2.0, arm * 2.0, color);
        }
        UnitShape::Octagon => draw_poly(pos.x, pos.y, 8, size, 22.5, color),
    }
}
