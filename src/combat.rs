use macroquad::prelude::*;

use crate::projectile::Projectile;
use crate::tech::{TechId, TechState};
use crate::terrain::Obstacle;
use crate::unit::{Unit, UnitKind};

/// Apply damage to a unit, returning (damage_dealt, was_killed).
fn apply_damage(unit: &mut Unit, damage: f32, armor_pierce: bool) -> (f32, bool) {
    let before_hp = unit.hp;
    let was_alive = unit.alive;
    if armor_pierce {
        unit.take_raw_damage(damage);
    } else {
        unit.take_damage(damage);
    }
    (before_hp - unit.hp, was_alive && !unit.alive)
}

/// Deterministic distance tiebreaker: prefer closer, then lower ID.
fn is_closer(dist: f32, id: u64, best_dist: f32, best_id: Option<u64>) -> bool {
    dist < best_dist
        || (dist - best_dist).abs() < 0.01 && best_id.is_none_or(|bid| id < bid)
}

/// Find the nearest alive enemy for each unit and assign as target.
/// Prefers targets with line of sight, but falls back to nearest enemy
/// without LOS so units will path toward hidden enemies.
pub fn update_targeting(units: &mut [Unit], obstacles: &[Obstacle]) {
    let positions: Vec<(u64, u8, Vec2, bool)> = units
        .iter()
        .map(|u| (u.id, u.player_id, u.pos, u.alive))
        .collect();

    for unit in units.iter_mut() {
        if !unit.alive {
            continue;
        }

        let mut best_los_dist = f32::MAX;
        let mut best_los_id = None;
        let mut best_any_dist = f32::MAX;
        let mut best_any_id = None;

        for &(eid, eteam, epos, ealive) in &positions {
            if !ealive || eteam == unit.player_id {
                continue;
            }
            let d = unit.pos.distance(epos);
            // Track nearest enemy regardless of LOS (for pathfinding)
            // Tiebreak on unit ID for determinism
            if is_closer(d, eid, best_any_dist, best_any_id) {
                best_any_dist = d;
                best_any_id = Some(eid);
            }
            // Track nearest enemy with LOS (preferred for attacking)
            if is_closer(d, eid, best_los_dist, best_los_id)
                && crate::terrain::has_line_of_sight_wide(unit.pos, epos, crate::projectile::PROJECTILE_RADIUS, obstacles)
            {
                best_los_dist = d;
                best_los_id = Some(eid);
            }
        }

        // Prefer LOS target, fall back to nearest enemy for pathing
        unit.target_id = best_los_id.or(best_any_id);
    }
}

/// Move units toward their targets using A* pathfinding.
/// Falls back to direct movement when no nav grid is provided or no path is found.
pub fn update_movement(units: &mut [Unit], dt: f32, arena_w: f32, arena_h: f32, obstacles: &[Obstacle], nav_grid: Option<&crate::terrain::NavGrid>) {
    let snapshot: Vec<(u64, Vec2, f32, bool)> = units
        .iter()
        .map(|u| (u.id, u.pos, u.stats.size, u.alive))
        .collect();

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

        // Decrement slow timer
        if unit.slow_timer > 0.0 {
            unit.slow_timer = (unit.slow_timer - dt).max(0.0);
        }

        // Effective move speed (halved if slowed)
        let effective_speed = if unit.slow_timer > 0.0 {
            unit.stats.move_speed * 0.5
        } else {
            unit.stats.move_speed
        };

        // Increment path age
        unit.path_age += dt;

        if let Some(target_pos) = target_positions[i].1 {
            let dist = unit.pos.distance(target_pos);
            let has_los = crate::terrain::has_line_of_sight_wide(unit.pos, target_pos, crate::projectile::PROJECTILE_RADIUS, obstacles);

            let needs_retreat = unit.stats.min_attack_range > 0.0 && dist < unit.stats.min_attack_range * 0.8;
            // Advance if out of range OR in range but can't see target (path around wall)
            let needs_advance = dist > unit.stats.attack_range * 0.9 || !has_los;

            if needs_retreat {
                // Retreat: move directly away from target (simple, no pathfinding needed)
                let dir = (unit.pos - target_pos).normalize_or_zero();
                unit.pos += dir * effective_speed * dt;
                unit.path.clear();
            } else if needs_advance {
                // Advance: use A* pathfinding if available
                if let Some(grid) = nav_grid {
                    // Repath if path is stale, empty, or target moved significantly
                    if unit.path.is_empty() || unit.path_age > 0.5 {
                        if let Some(new_path) = crate::terrain::find_path(grid, unit.pos, target_pos) {
                            unit.path = new_path;
                        } else {
                            // No path found — direct movement fallback
                            unit.path = vec![target_pos];
                        }
                        unit.path_age = 0.0;
                    }

                    // Follow waypoints
                    if !unit.path.is_empty() {
                        let waypoint = unit.path[0];
                        let to_waypoint = waypoint - unit.pos;
                        let wp_dist = to_waypoint.length();

                        if wp_dist < crate::terrain::GRID_CELL {
                            // Reached waypoint, advance to next
                            unit.path.remove(0);
                        } else {
                            // Move toward waypoint
                            let dir = to_waypoint.normalize_or_zero();
                            unit.pos += dir * effective_speed * dt;
                        }
                    }
                } else {
                    // No nav grid — direct movement
                    let dir = (target_pos - unit.pos).normalize_or_zero();
                    unit.pos += dir * effective_speed * dt;
                }
            } else {
                // In range — clear path, hold position
                unit.path.clear();
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
            let min_dist = (unit.stats.size + osize) * 1.05;
            if dist < min_dist && dist > 0.001 {
                push += diff.normalize() * (min_dist - dist) * 0.5;
            }
        }
        unit.pos += push;

        // Wall collision — push units out of wall obstacles (safety net)
        for obs in obstacles {
            if !obs.blocks_movement() { continue; }
            if obs.intersects_circle(unit.pos, unit.stats.size) {
                let obs_min = obs.pos - obs.half_size;
                let obs_max = obs.pos + obs.half_size;
                let closest = vec2(
                    unit.pos.x.clamp(obs_min.x, obs_max.x),
                    unit.pos.y.clamp(obs_min.y, obs_max.y),
                );
                let diff = unit.pos - closest;
                let dist = diff.length();
                if dist > 0.001 && dist < unit.stats.size {
                    unit.pos += diff.normalize() * (unit.stats.size - dist);
                } else if dist <= 0.001 {
                    let dx_left = unit.pos.x - obs_min.x;
                    let dx_right = obs_max.x - unit.pos.x;
                    let dy_top = unit.pos.y - obs_min.y;
                    let dy_bot = obs_max.y - unit.pos.y;
                    let min_d = dx_left.min(dx_right).min(dy_top).min(dy_bot);
                    const EPS: f32 = 0.001;
                    if (min_d - dx_left).abs() < EPS { unit.pos.x = obs_min.x - unit.stats.size; }
                    else if (min_d - dx_right).abs() < EPS { unit.pos.x = obs_max.x + unit.stats.size; }
                    else if (min_d - dy_top).abs() < EPS { unit.pos.y = obs_min.y - unit.stats.size; }
                    else { unit.pos.y = obs_max.y + unit.stats.size; }
                }
            }
        }

        let s = unit.stats.size;
        unit.pos.x = unit.pos.x.clamp(s, arena_w - s);
        unit.pos.y = unit.pos.y.clamp(s, arena_h - s);
    }
}

/// Process attacks with tech effects.
pub fn update_attacks(
    units: &mut [Unit],
    projectiles: &mut Vec<Projectile>,
    dt: f32,
    player_techs: &TechState,
    ai_techs: &TechState,
    splash_effects: &mut Vec<crate::rendering::SplashEffect>,
) {
    // Update cooldowns
    for unit in units.iter_mut() {
        unit.update_cooldown(dt);
    }

    // Helper to get the right tech state for a team
    let tech_for_team = |player_id: u8| -> &TechState {
        if player_id == 0 { player_techs } else { ai_techs }
    };

    // === Interceptor rocket interception ===
    let interceptor_actions: Vec<(u64, usize, u8)> = {
        let mut actions = Vec::new();
        for unit in units.iter_mut() {
            if !unit.alive || !unit.can_attack() || !unit.is_interceptor() {
                continue;
            }
            let mut best_rocket: Option<(usize, f32)> = None;
            for (pi, proj) in projectiles.iter().enumerate() {
                if !proj.alive || proj.player_id == unit.player_id || !proj.is_rocket() {
                    continue;
                }
                let dist = unit.pos.distance(proj.pos);
                if dist <= unit.stats.attack_range {
                    // Tiebreak on index for determinism when distances are equal
                    if best_rocket.is_none() || dist < best_rocket.unwrap().1
                        || ((dist - best_rocket.unwrap().1).abs() < 0.01 && pi < best_rocket.unwrap().0)
                    {
                        best_rocket = Some((pi, dist));
                    }
                }
            }
            if let Some((pi, _)) = best_rocket {
                actions.push((unit.id, pi, unit.player_id));
                unit.reset_cooldown();
            }
        }
        actions
    };

    for (_unit_id, proj_idx, _team) in &interceptor_actions {
        if *proj_idx < projectiles.len() {
            projectiles[*proj_idx].alive = false;
        }
    }

    // Interceptors that intercepted a rocket this frame are blocked from also
    // attacking units — UNLESS they have the DualWeapon tech.
    let intercepted_unit_ids: Vec<u64> = interceptor_actions
        .iter()
        .filter(|(_uid, _, team)| {
            let techs = tech_for_team(*team);
            !techs.has_tech(UnitKind::Interceptor, TechId::InterceptorDualWeapon)
        })
        .map(|(uid, _, _)| *uid)
        .collect();

    // === Chaff Overwhelm: precompute bonus damage ===
    let chaff_positions: Vec<(Vec2, u8)> = units
        .iter()
        .filter(|u| u.alive && u.kind == UnitKind::Chaff)
        .map(|u| (u.pos, u.player_id))
        .collect();

    // === Normal attacks ===
    let mut events: Vec<AttackEvent> = Vec::new();

    {
        let snapshot: Vec<(u64, Vec2, f32, bool, u8)> = units
            .iter()
            .map(|u| (u.id, u.pos, u.stats.size, u.alive, u.player_id))
            .collect();

        for unit in units.iter_mut() {
            if !unit.alive || !unit.can_attack() {
                continue;
            }
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
            // Min range check — can't fire at targets too close
            if unit.stats.min_attack_range > 0.0 && dist < unit.stats.min_attack_range {
                continue;
            }

            unit.reset_cooldown();
            let techs = tech_for_team(unit.player_id);

            // Calculate bonus damage from Chaff Overwhelm tech
            let mut bonus_damage = 0.0;
            if unit.kind == UnitKind::Chaff && techs.has_tech(UnitKind::Chaff, TechId::ChaffOverwhelm) {
                for &(cpos, cteam) in &chaff_positions {
                    if cteam == unit.player_id && cpos.distance(unit.pos) < 50.0 && cpos != unit.pos {
                        bonus_damage += 2.0;
                    }
                }
            }

            let total_damage = unit.stats.damage + bonus_damage;

            if unit.is_melee() {
                let has_lifesteal = unit.kind == UnitKind::Berserker
                    && techs.has_tech(UnitKind::Berserker, TechId::BerserkerLifesteal);
                let cleave_ignores_armor = unit.kind == UnitKind::Bruiser
                    && techs.has_tech(UnitKind::Bruiser, TechId::BruiserCleave);

                events.push(AttackEvent::Melee {
                    attacker_id: unit.id,
                    target_id,
                    target_pos: target.1,
                    damage: total_damage,
                    splash_radius: unit.stats.splash_radius,
                    attacker_team: unit.player_id,
                    lifesteal: has_lifesteal,
                    attacker_hp_frac: unit.hp / unit.stats.max_hp,
                    cleave_ignores_armor,
                });
            } else {
                let armor_pierce = techs.has_tech(unit.kind, TechId::SniperArmorPierce);
                let pierce = unit.kind == UnitKind::Ranger
                    && techs.has_tech(UnitKind::Ranger, TechId::RangerPierce);
                let slow = unit.kind == UnitKind::Artillery
                    && techs.has_tech(UnitKind::Artillery, TechId::ArtillerySlow);

                events.push(AttackEvent::Ranged {
                    attacker_id: unit.id,
                    origin: unit.pos,
                    target_pos: target.1,
                    speed: unit.stats.projectile_speed,
                    damage: total_damage,
                    player_id: unit.player_id,
                    splash_radius: unit.stats.splash_radius,
                    proj_type: unit.stats.projectile_type,
                    armor_pierce,
                    pierce_count: if pierce { 1 } else { 0 },
                    applies_slow: slow,
                });
            }
        }
    }

    // Apply events
    for event in events {
        match event {
            AttackEvent::Melee {
                attacker_id,
                target_id,
                target_pos,
                damage,
                splash_radius,
                attacker_team,
                lifesteal,
                attacker_hp_frac,
                cleave_ignores_armor,
            } => {
                let mut total_damage_dealt = 0.0;
                let mut kills = 0u32;
                // Primary target
                if let Some(target) = units.iter_mut().find(|u| u.id == target_id && u.alive) {
                    let (dealt, killed) = apply_damage(target, damage, false);
                    total_damage_dealt += dealt;
                    if killed { kills += 1; }
                }
                // Splash damage
                if splash_radius > 0.0 {
                    splash_effects.push(crate::rendering::SplashEffect {
                        pos: target_pos,
                        radius: splash_radius,
                        timer: 0.3,
                        max_timer: 0.3,
                        player_id: attacker_team,
                    });
                    for unit in units.iter_mut() {
                        if !unit.alive || unit.id == target_id || unit.player_id == attacker_team {
                            continue;
                        }
                        if unit.pos.distance(target_pos) < splash_radius {
                            let (dealt, killed) = apply_damage(unit, damage, cleave_ignores_armor);
                            total_damage_dealt += dealt;
                            if killed { kills += 1; }
                        }
                    }
                }
                // Record stats on attacker (and apply lifesteal if applicable)
                if let Some(attacker) = units.iter_mut().find(|u| u.id == attacker_id) {
                    attacker.damage_dealt_round += total_damage_dealt;
                    attacker.damage_dealt_total += total_damage_dealt;
                    attacker.kills_total += kills;
                    if lifesteal && total_damage_dealt > 0.0 && attacker.alive {
                        let heal = total_damage_dealt * 0.3 * (1.0 - attacker_hp_frac);
                        attacker.hp = (attacker.hp + heal).min(attacker.stats.max_hp);
                    }
                }
            }
            AttackEvent::Ranged {
                attacker_id,
                origin,
                target_pos,
                speed,
                damage,
                player_id,
                splash_radius,
                proj_type,
                armor_pierce,
                pierce_count,
                applies_slow,
            } => {
                let mut proj = Projectile::new(
                    origin,
                    target_pos,
                    speed,
                    damage,
                    player_id,
                    splash_radius,
                    proj_type,
                );
                proj.armor_pierce = armor_pierce;
                proj.pierce_remaining = pierce_count;
                proj.applies_slow = applies_slow;
                proj.attacker_id = attacker_id;
                projectiles.push(proj);
            }
        }
    }
}

/// Update projectiles with shield interception, evasion, pierce, and slow.
pub fn update_projectiles(projectiles: &mut Vec<Projectile>, units: &mut [Unit], dt: f32, obstacles: &mut [Obstacle], splash_effects: &mut Vec<crate::rendering::SplashEffect>) {
    let shields: Vec<(u64, u8, Vec2, f32, bool)> = units
        .iter()
        .filter(|u| u.is_shield() && u.alive)
        .map(|u| (u.id, u.player_id, u.pos, u.stats.shield_radius, u.shield_hp > 0.0))
        .collect();

    for proj in projectiles.iter_mut() {
        if !proj.alive {
            continue;
        }
        let old_pos = proj.pos;
        proj.update(dt);

        // Swept collision — check if the ray from old_pos to new pos crosses any blocking obstacle
        if crate::terrain::ray_hits_blocking_obstacle(old_pos, proj.pos, proj.player_id, obstacles) {
            proj.alive = false;
            continue;
        }

        // Obstacle collision — check if projectile currently overlaps a wall or enemy cover
        let mut hit_obstacle = false;
        for obs in obstacles.iter_mut() {
            if !obs.alive { continue; }
            if !obs.blocks_projectile(proj.player_id) { continue; }
            if obs.intersects_circle(proj.pos, crate::projectile::PROJECTILE_RADIUS) {
                // Destructible cover takes damage
                obs.take_damage(proj.damage);
                proj.alive = false;
                hit_obstacle = true;
                break;
            }
        }
        if hit_obstacle { continue; }

        // Shield barrier interception — pick closest shield, tiebreak on ID
        let mut intercepted_by_shield: Option<u64> = None;
        let mut best_shield_dist = f32::MAX;
        for &(shield_id, shield_team, shield_pos, shield_radius, has_shield_hp) in &shields {
            if shield_team == proj.player_id || !has_shield_hp {
                continue;
            }
            let dist = proj.pos.distance(shield_pos);
            if dist < shield_radius
                && is_closer(dist, shield_id, best_shield_dist, intercepted_by_shield)
            {
                intercepted_by_shield = Some(shield_id);
                best_shield_dist = dist;
            }
        }

        if let Some(shield_id) = intercepted_by_shield {
            if let Some(shield_unit) = units.iter_mut().find(|u| u.id == shield_id && u.alive) {
                // Damage the barrier's separate HP pool
                shield_unit.shield_hp = (shield_unit.shield_hp - proj.damage).max(0.0);
            }
            proj.alive = false;
            continue;
        }

        // Normal collision with enemy units
        let mut hit = false;
        let mut impact_pos = proj.pos;
        let mut proj_damage_dealt = 0.0f32;
        let mut proj_kills = 0u32;
        let attacker_id = proj.attacker_id;

        for unit in units.iter_mut() {
            if !unit.alive || unit.player_id == proj.player_id {
                continue;
            }
            let dist = proj.pos.distance(unit.pos);
            if dist < unit.stats.size + crate::projectile::PROJECTILE_RADIUS {
                // Evasion check
                if unit.evasion_chance > 0.0 {
                    let roll = macroquad::rand::gen_range(0.0f32, 1.0);
                    if roll < unit.evasion_chance {
                        continue;
                    }
                }

                let (dealt, killed) = apply_damage(unit, proj.damage, proj.armor_pierce);
                proj_damage_dealt += dealt;
                if killed { proj_kills += 1; }

                if proj.applies_slow {
                    unit.slow_timer = 2.0;
                }

                impact_pos = proj.pos;
                hit = true;

                if proj.pierce_remaining > 0 {
                    proj.pierce_remaining -= 1;
                    continue;
                }

                proj.alive = false;
                break;
            }
        }

        // Splash damage
        if hit && proj.splash_radius > 0.0 {
            splash_effects.push(crate::rendering::SplashEffect {
                pos: impact_pos,
                radius: proj.splash_radius,
                timer: 0.3,
                max_timer: 0.3,
                player_id: proj.player_id,
            });
            for unit in units.iter_mut() {
                if !unit.alive || unit.player_id == proj.player_id {
                    continue;
                }
                let dist = unit.pos.distance(impact_pos);
                if dist < proj.splash_radius && dist > 0.001 {
                    let (dealt, killed) = apply_damage(unit, proj.damage, proj.armor_pierce);
                    proj_damage_dealt += dealt;
                    if killed { proj_kills += 1; }
                    if proj.applies_slow {
                        unit.slow_timer = 2.0;
                    }
                }
            }
        }

        // Record stats on the attacker
        if proj_damage_dealt > 0.0 || proj_kills > 0 {
            if let Some(attacker) = units.iter_mut().find(|u| u.id == attacker_id) {
                attacker.damage_dealt_round += proj_damage_dealt;
                attacker.damage_dealt_total += proj_damage_dealt;
                attacker.kills_total += proj_kills;
            }
        }
    }

    projectiles.retain(|p| p.alive);
}

enum AttackEvent {
    Melee {
        attacker_id: u64,
        target_id: u64,
        target_pos: Vec2,
        damage: f32,
        splash_radius: f32,
        attacker_team: u8,
        lifesteal: bool,
        attacker_hp_frac: f32,
        cleave_ignores_armor: bool,
    },
    Ranged {
        attacker_id: u64,
        origin: Vec2,
        target_pos: Vec2,
        speed: f32,
        damage: f32,
        player_id: u8,
        splash_radius: f32,
        proj_type: crate::unit::ProjectileType,
        armor_pierce: bool,
        pierce_count: u8,
        applies_slow: bool,
    },
}
