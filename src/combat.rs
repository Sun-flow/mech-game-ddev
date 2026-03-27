use macroquad::prelude::*;

use crate::projectile::Projectile;
use crate::unit::Unit;

/// Find the nearest alive enemy for each unit and assign as target.
pub fn update_targeting(units: &mut [Unit]) {
    let positions: Vec<(u64, u8, Vec2, bool)> = units
        .iter()
        .map(|u| (u.id, u.team_id, u.pos, u.alive))
        .collect();

    for unit in units.iter_mut() {
        if !unit.alive {
            continue;
        }

        let mut best_dist = f32::MAX;
        let mut best_id = None;

        for &(eid, eteam, epos, ealive) in &positions {
            if !ealive || eteam == unit.team_id {
                continue;
            }
            let d = unit.pos.distance(epos);
            if d < best_dist {
                best_dist = d;
                best_id = Some(eid);
            }
        }

        unit.target_id = best_id;
    }
}

/// Move units toward their targets. Apply separation to avoid stacking.
pub fn update_movement(units: &mut [Unit], dt: f32, arena_w: f32, arena_h: f32) {
    let snapshot: Vec<(u64, Vec2, f32, bool)> = units
        .iter()
        .map(|u| (u.id, u.pos, u.stats.size, u.alive))
        .collect();

    // Build a target position lookup
    let target_positions: Vec<(u64, Option<Vec2>)> = units
        .iter()
        .map(|u| {
            let tpos = u.target_id.and_then(|tid| {
                snapshot
                    .iter()
                    .find(|(id, _, _, alive)| *id == tid && *alive)
                    .map(|(_, pos, _, _)| *pos)
            });
            (u.id, tpos)
        })
        .collect();

    for (i, unit) in units.iter_mut().enumerate() {
        if !unit.alive {
            continue;
        }

        // Move toward target if out of range
        if let Some(target_pos) = target_positions[i].1 {
            let dist = unit.pos.distance(target_pos);
            if dist > unit.stats.attack_range * 0.9 {
                let dir = (target_pos - unit.pos).normalize_or_zero();
                unit.pos += dir * unit.stats.move_speed * dt;
            }
        }

        // Separation from other units
        let mut push = Vec2::ZERO;
        for &(oid, opos, osize, oalive) in &snapshot {
            if !oalive || oid == unit.id {
                continue;
            }
            let diff = unit.pos - opos;
            let dist = diff.length();
            let min_dist = (unit.stats.size + osize) * 0.5;
            if dist < min_dist && dist > 0.001 {
                push += diff.normalize() * (min_dist - dist) * 0.5;
            }
        }
        unit.pos += push;

        // Clamp to arena bounds
        let s = unit.stats.size;
        unit.pos.x = unit.pos.x.clamp(s, arena_w - s);
        unit.pos.y = unit.pos.y.clamp(s, arena_h - s);
    }
}

/// Process attacks: melee does instant damage, ranged spawns projectiles.
pub fn update_attacks(units: &mut [Unit], projectiles: &mut Vec<Projectile>, dt: f32) {
    // Update cooldowns
    for unit in units.iter_mut() {
        unit.update_cooldown(dt);
    }

    // Collect attack events
    let mut events: Vec<AttackEvent> = Vec::new();

    {
        let snapshot: Vec<(u64, Vec2, f32, bool, u8)> = units
            .iter()
            .map(|u| (u.id, u.pos, u.stats.size, u.alive, u.team_id))
            .collect();

        for unit in units.iter_mut() {
            if !unit.alive || !unit.can_attack() {
                continue;
            }

            let target_id = match unit.target_id {
                Some(tid) => tid,
                None => continue,
            };

            let target = match snapshot.iter().find(|(id, _, _, alive, _)| *id == target_id && *alive) {
                Some(t) => t,
                None => continue,
            };

            let dist = unit.pos.distance(target.1);
            if dist > unit.stats.attack_range {
                continue;
            }

            unit.reset_cooldown();

            if unit.is_melee() {
                events.push(AttackEvent::Melee {
                    target_id,
                    target_pos: target.1,
                    damage: unit.stats.damage,
                    splash_radius: unit.stats.splash_radius,
                    attacker_team: unit.team_id,
                });
            } else {
                events.push(AttackEvent::Ranged {
                    origin: unit.pos,
                    target_pos: target.1,
                    speed: unit.stats.projectile_speed,
                    damage: unit.stats.damage,
                    team_id: unit.team_id,
                    splash_radius: unit.stats.splash_radius,
                });
            }
        }
    }

    // Apply events
    for event in events {
        match event {
            AttackEvent::Melee {
                target_id,
                target_pos,
                damage,
                splash_radius,
                attacker_team,
            } => {
                if let Some(target) = units.iter_mut().find(|u| u.id == target_id && u.alive) {
                    target.take_damage(damage);
                }
                // Splash damage to nearby enemies
                if splash_radius > 0.0 {
                    for unit in units.iter_mut() {
                        if !unit.alive || unit.id == target_id || unit.team_id == attacker_team {
                            continue;
                        }
                        if unit.pos.distance(target_pos) < splash_radius {
                            unit.take_damage(damage);
                        }
                    }
                }
            }
            AttackEvent::Ranged {
                origin,
                target_pos,
                speed,
                damage,
                team_id,
                splash_radius,
            } => {
                projectiles.push(Projectile::new(
                    origin,
                    target_pos,
                    speed,
                    damage,
                    team_id,
                    splash_radius,
                ));
            }
        }
    }
}

/// Update projectiles and check collisions with enemy units.
pub fn update_projectiles(projectiles: &mut Vec<Projectile>, units: &mut [Unit], dt: f32) {
    for proj in projectiles.iter_mut() {
        if !proj.alive {
            continue;
        }
        proj.update(dt);

        // Check collision with first enemy unit hit
        let mut hit = false;
        let mut impact_pos = proj.pos;
        for unit in units.iter_mut() {
            if !unit.alive || unit.team_id == proj.team_id {
                continue;
            }
            let dist = proj.pos.distance(unit.pos);
            if dist < unit.stats.size + crate::projectile::PROJECTILE_RADIUS {
                unit.take_damage(proj.damage);
                impact_pos = proj.pos;
                hit = true;
                proj.alive = false;
                break;
            }
        }

        // Apply splash damage to nearby enemies
        if hit && proj.splash_radius > 0.0 {
            for unit in units.iter_mut() {
                if !unit.alive || unit.team_id == proj.team_id {
                    continue;
                }
                let dist = unit.pos.distance(impact_pos);
                // Skip the primary target (already damaged), hit others in splash range
                if dist < proj.splash_radius && dist > 0.001 {
                    unit.take_damage(proj.damage);
                }
            }
        }
    }

    projectiles.retain(|p| p.alive);
}

enum AttackEvent {
    Melee {
        target_id: u64,
        target_pos: Vec2,
        damage: f32,
        splash_radius: f32,
        attacker_team: u8,
    },
    Ranged {
        origin: Vec2,
        target_pos: Vec2,
        speed: f32,
        damage: f32,
        team_id: u8,
        splash_radius: f32,
    },
}
