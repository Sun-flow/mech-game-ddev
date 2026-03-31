use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Serialize, Deserialize};

use crate::unit::{Unit, UnitKind, ProjectileType};
use crate::projectile::Projectile;
use crate::terrain::Obstacle;

// ---------------------------------------------------------------------------
// Lightweight serializable structs for state sync (avoids Vec2 serde issue)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncUnit {
    pub id: u64,
    pub kind: UnitKind,
    pub hp: f32,
    pub pos: (f32, f32),
    pub team_id: u8,
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
    pub team_id: u8,
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
    pub team_id: u8,
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
            pos: (u.pos.x, u.pos.y),
            team_id: u.team_id,
            target_id: u.target_id,
            attack_cooldown: u.attack_cooldown,
            alive: u.alive,
            death_timer: u.death_timer,
            path: u.path.iter().map(|v| (v.x, v.y)).collect(),
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
        u.pos = macroquad::prelude::vec2(self.pos.0, self.pos.1);
        u.target_id = self.target_id;
        u.attack_cooldown = self.attack_cooldown;
        u.alive = self.alive;
        u.death_timer = self.death_timer;
        u.path = self.path.iter().map(|&(x, y)| macroquad::prelude::vec2(x, y)).collect();
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
            pos: (p.pos.x, p.pos.y),
            vel: (p.vel.x, p.vel.y),
            origin: (p.origin.x, p.origin.y),
            max_range: p.max_range,
            damage: p.damage,
            team_id: p.team_id,
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
            pos: macroquad::prelude::vec2(self.pos.0, self.pos.1),
            vel: macroquad::prelude::vec2(self.vel.0, self.vel.1),
            origin: macroquad::prelude::vec2(self.origin.0, self.origin.1),
            max_range: self.max_range,
            damage: self.damage,
            team_id: self.team_id,
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
            pos: (o.pos.x, o.pos.y),
            half_size: (o.half_size.x, o.half_size.y),
            hp: o.hp,
            team_id: o.team_id,
            alive: o.alive,
        }
    }

    pub fn apply_to(&self, o: &mut Obstacle) {
        o.pos = macroquad::prelude::vec2(self.pos.0, self.pos.1);
        o.hp = self.hp;
        o.alive = self.alive;
    }
}

// ---------------------------------------------------------------------------
// State hashing — fast hash of gameplay-relevant fields only
// ---------------------------------------------------------------------------

/// Compute a hash of gameplay-relevant state.
/// When `mirror` is true (guest), hashes with mirrored x positions and swapped
/// team_ids so the result matches the host's hash for the same physical state.
pub fn compute_state_hash(
    units: &[Unit],
    projectiles: &[Projectile],
    obstacles: &[Obstacle],
    mirror: bool,
) -> u64 {
    let arena_w = crate::arena::ARENA_W;
    let mut hasher = DefaultHasher::new();

    // Sort units by id so both sides hash in the same order
    let mut sorted_units: Vec<&Unit> = units.iter().collect();
    sorted_units.sort_by_key(|u| u.id);

    for u in &sorted_units {
        u.id.hash(&mut hasher);
        u.alive.hash(&mut hasher);
        u.hp.to_bits().hash(&mut hasher);
        let x = if mirror { arena_w - u.pos.x } else { u.pos.x };
        x.to_bits().hash(&mut hasher);
        u.pos.y.to_bits().hash(&mut hasher);
        let team = if mirror { 1 - u.team_id } else { u.team_id };
        team.hash(&mut hasher);
        // NOTE: target_id excluded — targeting can diverge due to tie-breaking without
        // meaning the simulation is desynced (positions/hp are what matter)
        u.attack_cooldown.to_bits().hash(&mut hasher);
        u.shield_hp.to_bits().hash(&mut hasher);
        u.slow_timer.to_bits().hash(&mut hasher);
    }

    for p in projectiles {
        let px = if mirror { arena_w - p.pos.x } else { p.pos.x };
        px.to_bits().hash(&mut hasher);
        p.pos.y.to_bits().hash(&mut hasher);
        let vx = if mirror { -p.vel.x } else { p.vel.x };
        vx.to_bits().hash(&mut hasher);
        p.vel.y.to_bits().hash(&mut hasher);
        p.alive.hash(&mut hasher);
        p.damage.to_bits().hash(&mut hasher);
        let team = if mirror { 1 - p.team_id } else { p.team_id };
        team.hash(&mut hasher);
    }

    for o in obstacles {
        let ox = if mirror { arena_w - o.pos.x } else { o.pos.x };
        ox.to_bits().hash(&mut hasher);
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
/// When `mirror` is true (guest), translates host positions via (-1)*x
/// (ARENA_W - x) and swaps team_ids to maintain the guest's perspective.
pub fn apply_state_sync(
    units: &mut Vec<Unit>,
    projectiles: &mut Vec<Projectile>,
    obstacles: &mut Vec<Obstacle>,
    units_data: &[u8],
    projectiles_data: &[u8],
    obstacles_data: &[u8],
    mirror: bool,
) {
    let arena_w = crate::arena::ARENA_W;

    // Deserialize sync structs
    if let Ok(sync_units) = bincode::deserialize::<Vec<SyncUnit>>(units_data) {
        // Match by id and apply; units that exist in sync but not locally are skipped
        // (stats/kind are preserved from local since they don't change mid-round)
        for mut su in sync_units {
            if mirror {
                su.pos.0 = arena_w - su.pos.0;
                su.team_id = 1 - su.team_id;
                su.path = su.path.iter().map(|&(px, py)| (arena_w - px, py)).collect();
            }
            if let Some(u) = units.iter_mut().find(|u| u.id == su.id) {
                su.apply_to(u);
            }
        }
    }

    if let Ok(sync_projs) = bincode::deserialize::<Vec<SyncProjectile>>(projectiles_data) {
        // Replace projectiles entirely — they're ephemeral and have no persistent identity
        *projectiles = sync_projs.iter().map(|sp| {
            let mut p = sp.to_projectile();
            if mirror {
                p.pos.x = arena_w - p.pos.x;
                p.vel.x = -p.vel.x;
                p.origin.x = arena_w - p.origin.x;
                p.team_id = 1 - p.team_id;
            }
            p
        }).collect();
    }

    if let Ok(sync_obs) = bincode::deserialize::<Vec<SyncObstacle>>(obstacles_data) {
        // Apply to existing obstacles by index (obstacles don't change count mid-round)
        for (mut so, o) in sync_obs.into_iter().zip(obstacles.iter_mut()) {
            if mirror {
                so.pos.0 = arena_w - so.pos.0;
                if so.team_id != 255 {
                    so.team_id = 1 - so.team_id;
                }
            }
            so.apply_to(o);
        }
    }
}
