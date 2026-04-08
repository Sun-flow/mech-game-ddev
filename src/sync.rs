use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Serialize, Deserialize};

use crate::unit::{Unit, UnitKind, ProjectileType};
use crate::projectile::Projectile;
use crate::terrain::Obstacle;

fn v2(t: (f32, f32)) -> macroquad::prelude::Vec2 {
    macroquad::prelude::vec2(t.0, t.1)
}

fn t2(v: macroquad::prelude::Vec2) -> (f32, f32) {
    (v.x, v.y)
}

// ---------------------------------------------------------------------------
// Lightweight serializable structs for state sync (avoids Vec2 serde issue)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncUnit {
    pub id: u64,
    pub kind: UnitKind,
    pub hp: f32,
    pub pos: (f32, f32),
    pub player_id: u16,
    pub target_id: Option<u64>,
    pub attack_cooldown: f32,
    pub alive: bool,
    pub death_timer: f32,
    pub path: Vec<(f32, f32)>,
    pub path_age: f32,
    pub slow_timer: f32,
    pub evasion_chance: f32,
    pub shield_hp: f32,
    pub damage_dealt_round: f32,
    pub damage_dealt_total: f32,
    pub damage_soaked_round: f32,
    pub damage_soaked_total: f32,
    pub kills_total: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncProjectile {
    pub pos: (f32, f32),
    pub vel: (f32, f32),
    pub origin: (f32, f32),
    pub max_range: f32,
    pub damage: f32,
    pub player_id: u16,
    pub splash_radius: f32,
    pub alive: bool,
    pub proj_type: ProjectileType,
    pub armor_pierce: bool,
    pub pierce_remaining: u8,
    pub applies_slow: bool,
    pub attacker_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncObstacle {
    pub pos: (f32, f32),
    pub half_size: (f32, f32),
    pub hp: f32,
    pub player_id: u16,
    pub alive: bool,
}

// ---------------------------------------------------------------------------
// Conversions: game structs -> sync structs
// ---------------------------------------------------------------------------

impl SyncUnit {
    pub fn from_unit(u: &Unit) -> Self {
        Self {
            id: u.id,
            kind: u.kind,
            hp: u.hp,
            pos: t2(u.pos),
            player_id: u.player_id,
            target_id: u.target_id,
            attack_cooldown: u.attack_cooldown,
            alive: u.alive,
            death_timer: u.death_timer,
            path: u.path.iter().copied().map(t2).collect(),
            path_age: u.path_age,
            slow_timer: u.slow_timer,
            evasion_chance: u.evasion_chance,
            shield_hp: u.shield_hp,
            damage_dealt_round: u.damage_dealt_round,
            damage_dealt_total: u.damage_dealt_total,
            damage_soaked_round: u.damage_soaked_round,
            damage_soaked_total: u.damage_soaked_total,
            kills_total: u.kills_total,
        }
    }

    /// Apply synced state onto an existing unit (preserves stats/shape/etc.)
    pub fn apply_to(&self, u: &mut Unit) {
        u.hp = self.hp;
        u.pos = v2(self.pos);
        u.target_id = self.target_id;
        u.attack_cooldown = self.attack_cooldown;
        u.alive = self.alive;
        u.death_timer = self.death_timer;
        u.path = self.path.iter().copied().map(v2).collect();
        u.path_age = self.path_age;
        u.slow_timer = self.slow_timer;
        u.evasion_chance = self.evasion_chance;
        u.shield_hp = self.shield_hp;
        u.damage_dealt_round = self.damage_dealt_round;
        u.damage_dealt_total = self.damage_dealt_total;
        u.damage_soaked_round = self.damage_soaked_round;
        u.damage_soaked_total = self.damage_soaked_total;
        u.kills_total = self.kills_total;
    }
}

impl SyncProjectile {
    pub fn from_projectile(p: &Projectile) -> Self {
        Self {
            pos: t2(p.pos),
            vel: t2(p.vel),
            origin: t2(p.origin),
            max_range: p.max_range,
            damage: p.damage,
            player_id: p.player_id,
            splash_radius: p.splash_radius,
            alive: p.alive,
            proj_type: p.proj_type,
            armor_pierce: p.armor_pierce,
            pierce_remaining: p.pierce_remaining,
            applies_slow: p.applies_slow,
            attacker_id: p.attacker_id,
        }
    }

    pub fn to_projectile(&self) -> Projectile {
        Projectile {
            pos: v2(self.pos),
            vel: v2(self.vel),
            origin: v2(self.origin),
            max_range: self.max_range,
            damage: self.damage,
            player_id: self.player_id,
            splash_radius: self.splash_radius,
            alive: self.alive,
            proj_type: self.proj_type,
            armor_pierce: self.armor_pierce,
            pierce_remaining: self.pierce_remaining,
            applies_slow: self.applies_slow,
            attacker_id: self.attacker_id,
        }
    }
}

impl SyncObstacle {
    pub fn from_obstacle(o: &Obstacle) -> Self {
        Self {
            pos: t2(o.pos),
            half_size: t2(o.half_size),
            hp: o.hp,
            player_id: o.player_id,
            alive: o.alive,
        }
    }

    pub fn apply_to(&self, o: &mut Obstacle) {
        o.pos = v2(self.pos);
        o.hp = self.hp;
        o.alive = self.alive;
    }
}

// ---------------------------------------------------------------------------
// State hashing — fast hash of gameplay-relevant fields only
// ---------------------------------------------------------------------------

/// Compute a hash of gameplay-relevant state.
/// Both clients hash identical canonical data — no mirroring needed.
pub fn compute_state_hash(
    units: &[Unit],
    projectiles: &[Projectile],
    obstacles: &[Obstacle],
) -> u64 {
    let mut hasher = DefaultHasher::new();

    // Sort unit indices by id so both sides hash in the same order
    let mut sorted_indices: Vec<usize> = (0..units.len()).collect();
    sorted_indices.sort_unstable_by_key(|&i| units[i].id);

    for &i in &sorted_indices {
        let u = &units[i];
        u.id.hash(&mut hasher);
        u.alive.hash(&mut hasher);
        u.hp.to_bits().hash(&mut hasher);
        u.pos.x.to_bits().hash(&mut hasher);
        u.pos.y.to_bits().hash(&mut hasher);
        u.player_id.hash(&mut hasher);
        u.attack_cooldown.to_bits().hash(&mut hasher);
        u.shield_hp.to_bits().hash(&mut hasher);
        u.slow_timer.to_bits().hash(&mut hasher);
    }

    for p in projectiles {
        p.pos.x.to_bits().hash(&mut hasher);
        p.pos.y.to_bits().hash(&mut hasher);
        p.vel.x.to_bits().hash(&mut hasher);
        p.vel.y.to_bits().hash(&mut hasher);
        p.alive.hash(&mut hasher);
        p.damage.to_bits().hash(&mut hasher);
        p.player_id.hash(&mut hasher);
    }

    for o in obstacles {
        o.pos.x.to_bits().hash(&mut hasher);
        o.pos.y.to_bits().hash(&mut hasher);
        o.hp.to_bits().hash(&mut hasher);
        o.alive.hash(&mut hasher);
    }

    hasher.finish()
}

// ---------------------------------------------------------------------------
// Serialize / deserialize full state for correction
// ---------------------------------------------------------------------------

pub fn serialize_state(
    units: &[Unit],
    projectiles: &[Projectile],
    obstacles: &[Obstacle],
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let sync_units: Vec<SyncUnit> = units.iter().map(SyncUnit::from_unit).collect();
    let sync_projs: Vec<SyncProjectile> = projectiles.iter().map(SyncProjectile::from_projectile).collect();
    let sync_obs: Vec<SyncObstacle> = obstacles.iter().map(SyncObstacle::from_obstacle).collect();

    (
        bincode::serialize(&sync_units).unwrap_or_default(),
        bincode::serialize(&sync_projs).unwrap_or_default(),
        bincode::serialize(&sync_obs).unwrap_or_default(),
    )
}

/// Apply host's authoritative state to local game state.
/// Canonical coordinates — no mirroring needed.
pub fn apply_state_sync(
    units: &mut [Unit],
    projectiles: &mut Vec<Projectile>,
    obstacles: &mut [Obstacle],
    units_data: &[u8],
    projectiles_data: &[u8],
    obstacles_data: &[u8],
) {
    // Deserialize sync structs
    if let Ok(sync_units) = bincode::deserialize::<Vec<SyncUnit>>(units_data) {
        for su in sync_units {
            if let Some(u) = units.iter_mut().find(|u| u.id == su.id) {
                su.apply_to(u);
            }
        }
    }

    if let Ok(sync_projs) = bincode::deserialize::<Vec<SyncProjectile>>(projectiles_data) {
        *projectiles = sync_projs.iter().map(|sp| sp.to_projectile()).collect();
    }

    if let Ok(sync_obs) = bincode::deserialize::<Vec<SyncObstacle>>(obstacles_data) {
        if sync_obs.len() != obstacles.len() {
            eprintln!("[SYNC] Obstacle count mismatch: received {} vs local {}", sync_obs.len(), obstacles.len());
        }
        for (so, o) in sync_obs.into_iter().zip(obstacles.iter_mut()) {
            so.apply_to(o);
        }
    }
}
