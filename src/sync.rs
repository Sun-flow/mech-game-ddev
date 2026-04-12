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
    pub spawn_pos: (f32, f32),
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
    pub stationary_timer: f32,
    pub has_charged: bool,
    pub expendable_stacks: u8,
    pub expendable_timer: f32,
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
            spawn_pos: t2(u.spawn_pos),
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
            stationary_timer: u.stationary_timer,
            has_charged: u.has_charged,
            expendable_stacks: u.expendable_stacks,
            expendable_timer: u.expendable_timer,
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
        u.spawn_pos = v2(self.spawn_pos);
        u.target_id = self.target_id;
        u.attack_cooldown = self.attack_cooldown;
        u.alive = self.alive;
        u.death_timer = self.death_timer;
        u.path = self.path.iter().copied().map(v2).collect();
        u.path_age = self.path_age;
        u.slow_timer = self.slow_timer;
        u.evasion_chance = self.evasion_chance;
        u.shield_hp = self.shield_hp;
        u.stationary_timer = self.stationary_timer;
        u.has_charged = self.has_charged;
        u.expendable_stacks = self.expendable_stacks;
        u.expendable_timer = self.expendable_timer;
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
        u.stationary_timer.to_bits().hash(&mut hasher);
        u.has_charged.hash(&mut hasher);
        u.expendable_stacks.hash(&mut hasher);
        u.expendable_timer.to_bits().hash(&mut hasher);
    }

    // Sort projectiles by a deterministic key for hashing. Projectiles don't
    // have stable IDs, so we sort by (attacker_id, position bits) which is
    // unique enough to produce a consistent order regardless of Vec insertion order.
    let mut proj_indices: Vec<usize> = (0..projectiles.len()).collect();
    proj_indices.sort_unstable_by_key(|&i| {
        let p = &projectiles[i];
        (p.attacker_id, p.pos.x.to_bits(), p.pos.y.to_bits())
    });
    for &i in &proj_indices {
        let p = &projectiles[i];
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

/// Apply host's authoritative state snapshot, then fast-forward the simulation
/// to catch up to the guest's current frame.
///
/// Given a snapshot from host's frame `snapshot_frame`, this function:
///   1. Deserializes the snapshot
///   2. Replaces local unit/projectile/obstacle state with the snapshot
///      - Units matched by ID: existing ones updated, missing ones spawned,
///        units not in the snapshot are removed
///      - Projectiles fully replaced (they have no stable ID)
///      - Obstacles updated in place (same count assumed)
///   3. Sets `current_frame` to `snapshot_frame` (rollback)
///   4. Replays combat forward via `combat::run_one_frame` until `current_frame`
///      matches the ORIGINAL value (catch-up to target)
///
/// Because combat is deterministic, the post-catch-up state at the target frame
/// matches what host's state is at that same frame — the guest and host are now
/// in lockstep again. Splash effects are cleared (visual only).
///
/// Returns Ok(frames_replayed) on success, Err(reason) on deserialization failure.
#[allow(clippy::too_many_arguments)]
pub fn apply_and_fast_forward(
    snapshot_frame: u32,
    current_frame: &mut u32,
    units: &mut Vec<Unit>,
    projectiles: &mut Vec<Projectile>,
    obstacles: &mut [Obstacle],
    nav_grid: Option<&crate::terrain::NavGrid>,
    players: &mut [crate::match_progress::PlayerState],
    splash_effects: &mut Vec<crate::rendering::SplashEffect>,
    units_data: &[u8],
    projectiles_data: &[u8],
    obstacles_data: &[u8],
    dt: f32,
    arena_w: f32,
    arena_h: f32,
) -> Result<u32, String> {
    // Remember the frame we need to catch up to
    let target_frame = *current_frame;

    // Deserialize all three state blobs before touching anything mutable,
    // so a deserialization failure leaves local state untouched.
    let sync_units: Vec<SyncUnit> = bincode::deserialize(units_data)
        .map_err(|e| format!("units deserialize failed: {}", e))?;
    let sync_projs: Vec<SyncProjectile> = bincode::deserialize(projectiles_data)
        .map_err(|e| format!("projectiles deserialize failed: {}", e))?;
    let sync_obs: Vec<SyncObstacle> = bincode::deserialize(obstacles_data)
        .map_err(|e| format!("obstacles deserialize failed: {}", e))?;

    // ---- Replace units ----
    // Units are keyed by id. For incoming units that don't exist locally,
    // we reconstruct a fresh Unit with stats from kind + player's current tech
    // state, then overlay the synced dynamic fields.
    let incoming_ids: std::collections::HashSet<u64> = sync_units.iter().map(|u| u.id).collect();
    units.retain(|u| incoming_ids.contains(&u.id));
    for su in &sync_units {
        if let Some(u) = units.iter_mut().find(|u| u.id == su.id) {
            if u.kind != su.kind {
                eprintln!(
                    "[SYNC WARNING] Unit {} kind mismatch: local={:?} snapshot={:?} pid={}. \
                     Keeping local kind (owned by spawning client). \
                     This indicates a bug — unit IDs drifted between host and guest.",
                    u.id, u.kind, su.kind, u.player_id
                );
            }
            su.apply_to(u);
            // Validate stats match what we'd derive from kind + techs.
            // Stats are never transmitted — both sides derive them independently.
            // A mismatch here means tech state or kind drifted silently.
            if let Some(p) = players.iter().find(|p| p.player_id == u.player_id) {
                let mut expected = u.kind.stats();
                p.techs.apply_to_stats(u.kind, &mut expected);
                if (u.stats.max_hp - expected.max_hp).abs() > 0.01
                    || (u.stats.damage - expected.damage).abs() > 0.01
                    || (u.stats.armor - expected.armor).abs() > 0.01
                    || (u.stats.attack_speed - expected.attack_speed).abs() > 0.01
                {
                    eprintln!(
                        "[SYNC WARNING] Unit {} ({:?}) stats drift: hp={}/{} dmg={}/{} arm={}/{} spd={}/{}",
                        u.id, u.kind,
                        u.stats.max_hp, expected.max_hp,
                        u.stats.damage, expected.damage,
                        u.stats.armor, expected.armor,
                        u.stats.attack_speed, expected.attack_speed,
                    );
                }
            }
        } else {
            // Spawn new unit from snapshot data
            let mut new_unit = Unit::new(
                su.id,
                su.kind,
                macroquad::prelude::vec2(su.pos.0, su.pos.1),
                su.player_id,
            );
            if let Some(player) = players.iter().find(|p| p.player_id == su.player_id) {
                player.techs.apply_to_stats(su.kind, &mut new_unit.stats);
            }
            su.apply_to(&mut new_unit);
            units.push(new_unit);
        }
    }

    // ---- Replace projectiles ----
    *projectiles = sync_projs.iter().map(|sp| sp.to_projectile()).collect();

    // ---- Replace obstacles (in place; counts should match) ----
    if sync_obs.len() != obstacles.len() {
        eprintln!(
            "[SYNC] Obstacle count mismatch: snapshot {} vs local {}",
            sync_obs.len(),
            obstacles.len()
        );
    }
    for (so, o) in sync_obs.into_iter().zip(obstacles.iter_mut()) {
        so.apply_to(o);
    }

    // ---- Clear visual-only splash effects (they don't affect gameplay state) ----
    splash_effects.clear();

    // ---- Rollback + fast-forward ----
    *current_frame = snapshot_frame;
    let mut frames_replayed = 0u32;
    while *current_frame < target_frame {
        crate::combat::run_one_frame(
            units,
            projectiles,
            obstacles,
            nav_grid,
            players,
            splash_effects,
            dt,
            arena_w,
            arena_h,
        );
        *current_frame += 1;
        frames_replayed += 1;
    }

    // If target_frame < snapshot_frame (snapshot is ahead — should not happen
    // under normal lockstep), current_frame will be set to snapshot_frame and
    // no replay runs. Caller should see a jump in frame counter. Log it.
    if snapshot_frame > target_frame {
        eprintln!(
            "[SYNC] Snapshot frame {} is ahead of local frame {}, jumped forward",
            snapshot_frame, target_frame
        );
    }

    Ok(frames_replayed)
}
