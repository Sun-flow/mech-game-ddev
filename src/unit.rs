use macroquad::prelude::*;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProjectileType {
    Bullet,
    Laser,
    Rocket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    pub shape: UnitShape,
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub id: u64,
    pub kind: UnitKind,
    pub stats: UnitStats,
    pub hp: f32,
    pub pos: Vec2,
    pub team_id: u8,
    pub target_id: Option<u64>,
    pub attack_cooldown: f32,
    pub alive: bool,
}

impl Unit {
    pub fn new(id: u64, kind: UnitKind, pos: Vec2, team_id: u8) -> Self {
        let stats = kind.stats();
        let hp = stats.max_hp;
        Self {
            id,
            kind,
            stats,
            hp,
            pos,
            team_id,
            target_id: None,
            attack_cooldown: 0.0,
            alive: true,
        }
    }

    pub fn take_damage(&mut self, raw_damage: f32) {
        let effective = (raw_damage - self.stats.armor).max(1.0);
        self.hp -= effective;
        if self.hp <= 0.0 {
            self.hp = 0.0;
            self.alive = false;
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
                max_hp: 60.0,
                damage: 25.0,
                attack_range: 200.0,
                attack_speed: 1.5,
                projectile_speed: 400.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 120.0,
                size: 12.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shape: UnitShape::Triangle,
            },
            UnitKind::Sentinel => UnitStats {
                max_hp: 200.0,
                damage: 8.0,
                attack_range: 80.0,
                attack_speed: 0.8,
                projectile_speed: 0.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 60.0,
                size: 18.0,
                armor: 8.0,
                splash_radius: 15.0,
                shield_radius: 0.0,
                shape: UnitShape::Square,
            },
            UnitKind::Ranger => UnitStats {
                max_hp: 70.0,
                damage: 18.0,
                attack_range: 350.0,
                attack_speed: 0.7,
                projectile_speed: 500.0,
                projectile_type: ProjectileType::Laser,
                move_speed: 80.0,
                size: 10.0,
                armor: 1.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shape: UnitShape::Diamond,
            },
            UnitKind::Scout => UnitStats {
                max_hp: 50.0,
                damage: 10.0,
                attack_range: 120.0,
                attack_speed: 2.0,
                projectile_speed: 300.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 180.0,
                size: 8.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shape: UnitShape::Circle,
            },
            UnitKind::Bruiser => UnitStats {
                max_hp: 140.0,
                damage: 15.0,
                attack_range: 100.0,
                attack_speed: 1.0,
                projectile_speed: 0.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 90.0,
                size: 15.0,
                armor: 4.0,
                splash_radius: 25.0,
                shield_radius: 0.0,
                shape: UnitShape::Hexagon,
            },
            UnitKind::Artillery => UnitStats {
                max_hp: 55.0,
                damage: 35.0,
                attack_range: 450.0,
                attack_speed: 0.4,
                projectile_speed: 250.0,
                projectile_type: ProjectileType::Rocket,
                move_speed: 50.0,
                size: 14.0,
                armor: 0.0,
                splash_radius: 40.0,
                shield_radius: 0.0,
                shape: UnitShape::Pentagon,
            },
            UnitKind::Chaff => UnitStats {
                max_hp: 15.0,
                damage: 3.0,
                attack_range: 30.0,
                attack_speed: 1.5,
                projectile_speed: 0.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 150.0,
                size: 4.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shape: UnitShape::Dot,
            },

            // === NEW UNITS ===

            // Sniper: single high-damage shot, very long range, very slow ROF
            UnitKind::Sniper => UnitStats {
                max_hp: 40.0,
                damage: 80.0,
                attack_range: 500.0,
                attack_speed: 0.25,
                projectile_speed: 900.0,
                projectile_type: ProjectileType::Laser,
                move_speed: 40.0,
                size: 11.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shape: UnitShape::Cross,
            },

            // Skirmisher: ranged chaff, fast swarm with low HP
            UnitKind::Skirmisher => UnitStats {
                max_hp: 10.0,
                damage: 4.0,
                attack_range: 180.0,
                attack_speed: 2.5,
                projectile_speed: 350.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 160.0,
                size: 4.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shape: UnitShape::Dot,
            },

            // Dragoon: ranged bruiser, medium everything
            UnitKind::Dragoon => UnitStats {
                max_hp: 100.0,
                damage: 20.0,
                attack_range: 200.0,
                attack_speed: 0.6,
                projectile_speed: 350.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 85.0,
                size: 13.0,
                armor: 3.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shape: UnitShape::Octagon,
            },

            // Berserker: melee, attack speed scales up as HP drops
            UnitKind::Berserker => UnitStats {
                max_hp: 90.0,
                damage: 22.0,
                attack_range: 60.0,
                attack_speed: 1.0, // base; scales to 3x at low HP
                projectile_speed: 0.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 130.0,
                size: 13.0,
                armor: 2.0,
                splash_radius: 10.0,
                shield_radius: 0.0,
                shape: UnitShape::Star,
            },

            // Shield: projects a barrier that intercepts enemy projectiles
            UnitKind::Shield => UnitStats {
                max_hp: 150.0,
                damage: 5.0,
                attack_range: 100.0,
                attack_speed: 0.5,
                projectile_speed: 0.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 55.0,
                size: 16.0,
                armor: 5.0,
                splash_radius: 0.0,
                shield_radius: 80.0,
                shape: UnitShape::Square,
            },

            // Interceptor: prioritizes shooting down enemy rockets, otherwise attacks units
            UnitKind::Interceptor => UnitStats {
                max_hp: 60.0,
                damage: 12.0,
                attack_range: 250.0,
                attack_speed: 1.2,
                projectile_speed: 450.0,
                projectile_type: ProjectileType::Bullet,
                move_speed: 100.0,
                size: 10.0,
                armor: 0.0,
                splash_radius: 0.0,
                shield_radius: 0.0,
                shape: UnitShape::Diamond,
            },
        }
    }

    pub fn all() -> &'static [UnitKind] {
        &[
            UnitKind::Striker,
            UnitKind::Sentinel,
            UnitKind::Ranger,
            UnitKind::Scout,
            UnitKind::Bruiser,
            UnitKind::Artillery,
            UnitKind::Chaff,
            UnitKind::Sniper,
            UnitKind::Skirmisher,
            UnitKind::Dragoon,
            UnitKind::Berserker,
            UnitKind::Shield,
            UnitKind::Interceptor,
        ]
    }
}
