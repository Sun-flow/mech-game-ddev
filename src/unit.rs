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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnitKind {
    Striker,
    Sentinel,
    Ranger,
    Scout,
    Bruiser,
    Artillery,
    Chaff,
}

#[derive(Clone, Debug)]
pub struct UnitStats {
    pub max_hp: f32,
    pub damage: f32,
    pub attack_range: f32,
    pub attack_speed: f32,
    pub projectile_speed: f32,
    pub move_speed: f32,
    pub size: f32,
    pub armor: f32,
    pub splash_radius: f32,
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

    pub fn reset_cooldown(&mut self) {
        if self.stats.attack_speed > 0.0 {
            self.attack_cooldown = 1.0 / self.stats.attack_speed;
        }
    }

    pub fn update_cooldown(&mut self, dt: f32) {
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
    }
}

impl UnitKind {
    pub fn stats(self) -> UnitStats {
        match self {
            UnitKind::Striker => UnitStats {
                max_hp: 60.0,
                damage: 25.0,
                attack_range: 200.0,
                attack_speed: 1.5,
                projectile_speed: 400.0,
                move_speed: 120.0,
                size: 12.0,
                armor: 0.0,
                splash_radius: 0.0,
                shape: UnitShape::Triangle,
            },
            UnitKind::Sentinel => UnitStats {
                max_hp: 200.0,
                damage: 8.0,
                attack_range: 80.0,
                attack_speed: 0.8,
                projectile_speed: 0.0,
                move_speed: 60.0,
                size: 18.0,
                armor: 8.0,
                splash_radius: 15.0,
                shape: UnitShape::Square,
            },
            UnitKind::Ranger => UnitStats {
                max_hp: 70.0,
                damage: 18.0,
                attack_range: 350.0,
                attack_speed: 0.7,
                projectile_speed: 500.0,
                move_speed: 80.0,
                size: 10.0,
                armor: 1.0,
                splash_radius: 0.0,
                shape: UnitShape::Diamond,
            },
            UnitKind::Scout => UnitStats {
                max_hp: 50.0,
                damage: 10.0,
                attack_range: 120.0,
                attack_speed: 2.0,
                projectile_speed: 300.0,
                move_speed: 180.0,
                size: 8.0,
                armor: 0.0,
                splash_radius: 0.0,
                shape: UnitShape::Circle,
            },
            UnitKind::Bruiser => UnitStats {
                max_hp: 140.0,
                damage: 15.0,
                attack_range: 100.0,
                attack_speed: 1.0,
                projectile_speed: 0.0,
                move_speed: 90.0,
                size: 15.0,
                armor: 4.0,
                splash_radius: 25.0,
                shape: UnitShape::Hexagon,
            },
            UnitKind::Artillery => UnitStats {
                max_hp: 55.0,
                damage: 35.0,
                attack_range: 450.0,
                attack_speed: 0.4,
                projectile_speed: 250.0,
                move_speed: 50.0,
                size: 14.0,
                armor: 0.0,
                splash_radius: 40.0,
                shape: UnitShape::Pentagon,
            },
            UnitKind::Chaff => UnitStats {
                max_hp: 15.0,
                damage: 3.0,
                attack_range: 30.0,
                attack_speed: 1.5,
                projectile_speed: 0.0,
                move_speed: 150.0,
                size: 4.0,
                armor: 0.0,
                splash_radius: 0.0,
                shape: UnitShape::Dot,
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
        ]
    }
}
