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
/// Interceptors prioritize destroying enemy rockets in range.
pub fn update_attacks(units: &mut [Unit], projectiles: &mut Vec<Projectile>, dt: f32) {
    // Update cooldowns
    for unit in units.iter_mut() {
        unit.update_cooldown(dt);
    }

    // === Interceptor rocket interception ===
    // Interceptors that can attack will try to destroy the nearest enemy rocket in range first.
    let interceptor_actions: Vec<(u64, usize)> = {
        let mut actions = Vec::new();
        for unit in units.iter_mut() {
            if !unit.alive || !unit.can_attack() || !unit.is_interceptor() {
                continue;
            }
            // Find nearest enemy rocket in attack range
            let mut best_rocket: Option<(usize, f32)> = None;
            for (pi, proj) in projectiles.iter().enumerate() {
                if !proj.alive || proj.team_id == unit.team_id || !proj.is_rocket() {
                    continue;
                }
                let dist = unit.pos.distance(proj.pos);
                if dist <= unit.stats.attack_range {
                    if best_rocket.is_none() || dist < best_rocket.unwrap().1 {
                        best_rocket = Some((pi, dist));
                    }
                }
            }
            if let Some((pi, _)) = best_rocket {
                actions.push((unit.id, pi));
                unit.reset_cooldown();
            }
        }
        actions
    };

    // Apply interceptions (destroy rockets)
    for (_unit_id, proj_idx) in &interceptor_actions {
        if *proj_idx < projectiles.len() {
            projectiles[*proj_idx].alive = false;
        }
    }

    // Track which interceptors already acted (so they don't also fire at units)
    let intercepted_unit_ids: Vec<u64> = interceptor_actions.iter().map(|(uid, _)| *uid).collect();

    // === Normal attacks ===
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
            // Skip interceptors that already intercepted a rocket this frame
            if intercepted_unit_ids.contains(&unit.id) {
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
                    proj_type: unit.stats.projectile_type,
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
                proj_type,
            } => {
                projectiles.push(Projectile::new(
                    origin,
                    target_pos,
                    speed,
                    damage,
                    team_id,
                    splash_radius,
                    proj_type,
                ));
            }
        }
    }
}

/// Update projectiles and check collisions with enemy units.
/// Shield units intercept enemy projectiles that enter their barrier radius.
pub fn update_projectiles(projectiles: &mut Vec<Projectile>, units: &mut [Unit], dt: f32) {
    // Collect shield unit info for interception checks
    let shields: Vec<(u64, u8, Vec2, f32)> = units
        .iter()
        .filter(|u| u.is_shield())
        .map(|u| (u.id, u.team_id, u.pos, u.stats.shield_radius))
        .collect();

    for proj in projectiles.iter_mut() {
        if !proj.alive {
            continue;
        }
        proj.update(dt);

        // === Shield barrier interception ===
        // Check if this projectile has entered any enemy shield's barrier radius.
        // Shield units on a different team from the projectile can intercept it.
        let mut intercepted_by_shield: Option<u64> = None;
        for &(shield_id, shield_team, shield_pos, shield_radius) in &shields {
            if shield_team == proj.team_id {
                continue; // Shield doesn't block friendly projectiles
            }
            let dist = proj.pos.distance(shield_pos);
            if dist < shield_radius {
                intercepted_by_shield = Some(shield_id);
                break;
            }
        }

        if let Some(shield_id) = intercepted_by_shield {
            // Deal projectile damage to the shield unit and destroy the projectile
            if let Some(shield_unit) = units.iter_mut().find(|u| u.id == shield_id && u.alive) {
                shield_unit.take_damage(proj.damage);
            }
            proj.alive = false;
            continue;
        }

        // === Normal collision with enemy units ===
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
        proj_type: crate::unit::ProjectileType,
    },
}
