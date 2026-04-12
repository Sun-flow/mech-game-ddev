use macroquad::prelude::*;

use crate::projectile::Projectile;
use crate::tech::{TechId, TechState};
use crate::terrain::Obstacle;
use crate::unit::{Unit, UnitKind};

/// Run one fixed-timestep frame of combat. Used by both the main battle loop
/// and by sync::apply_and_fast_forward during state-correction catch-up.
/// Does NOT increment any frame counter — the caller owns that.
#[allow(clippy::too_many_arguments)]
pub fn run_one_frame(
    units: &mut Vec<Unit>,
    projectiles: &mut Vec<Projectile>,
    obstacles: &mut [Obstacle],
    nav_grid: Option<&crate::terrain::NavGrid>,
    players: &mut [crate::match_progress::PlayerState],
    splash_effects: &mut Vec<crate::rendering::SplashEffect>,
    dt: f32,
    arena_w: f32,
    arena_h: f32,
) {
    // Canonical unit ordering: sort by ID so both host and guest iterate
    // in the same order. Without this, host=[host_units, guest_units] and
    // guest=[guest_units, host_units], causing projectile creation order
    // and splash application order to diverge. pdqsort is O(n) on
    // already-sorted input, so this is effectively free after the first frame.
    units.sort_unstable_by_key(|u| u.id);

    update_targeting(units, obstacles, players);
    update_movement(units, dt, arena_w, arena_h, obstacles, nav_grid, players);
    update_attacks(units, projectiles, dt, players, splash_effects);
    update_projectiles(projectiles, units, dt, obstacles, splash_effects, players);
    // Death animation timer (kept identical to the old battle_phase inline code)
    for unit in units.iter_mut() {
        if !unit.alive && unit.death_timer > 0.0 {
            unit.death_timer -= dt;
        }
    }
}

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

/// Compute the attack cooldown for a unit accounting for:
/// - Berserker rage scaling (from Unit::effective_attack_speed)
/// - Entrench stacks (Skirmisher, Ranger): +12% atk speed per stationary second, max 4 stacks
/// - Chaff Expendable stacks: +15% atk speed per stack, max 3
fn cooldown_from_techs(unit: &Unit, techs: &TechState) -> f32 {
    let mut speed = unit.effective_attack_speed();

    // Entrench: stationary stacking attack speed
    if (unit.kind == UnitKind::Skirmisher || unit.kind == UnitKind::Ranger)
        && techs.has_tech(unit.kind, TechId::Entrench)
    {
        let stacks = (unit.stationary_timer.floor() as u32).min(4);
        speed *= 1.0 + 0.12 * stacks as f32;
    }

    // Expendable: Chaff attack speed buff from nearby deaths
    if unit.kind == UnitKind::Chaff && unit.expendable_stacks > 0 {
        speed *= 1.0 + 0.15 * unit.expendable_stacks as f32;
    }

    if speed > 0.0 { 1.0 / speed } else { 0.0 }
}

/// Apply a Berserker Death Throes splash centered at `pos`, damaging enemies of `attacker_team`.
/// Does NOT re-trigger Death Throes if a Berserker killed by this splash also had Death Throes
/// (prevents infinite chains).
fn apply_death_throes(
    units: &mut [Unit],
    pos: Vec2,
    attacker_team: u16,
    splash_effects: &mut Vec<crate::rendering::SplashEffect>,
) {
    const DEATH_THROES_DAMAGE: f32 = 150.0;
    const DEATH_THROES_RADIUS: f32 = 40.0;

    splash_effects.push(crate::rendering::SplashEffect {
        pos,
        radius: DEATH_THROES_RADIUS,
        timer: 0.3,
        max_timer: 0.3,
        player_id: attacker_team,
    });

    for unit in units.iter_mut() {
        if !unit.alive || unit.player_id == attacker_team {
            continue;
        }
        if unit.pos.distance(pos) < DEATH_THROES_RADIUS {
            unit.take_damage(DEATH_THROES_DAMAGE);
        }
    }
}

/// Deterministic distance tiebreaker: prefer closer, then lower ID.
fn is_closer(dist: f32, id: u64, best_dist: f32, best_id: Option<u64>) -> bool {
    dist < best_dist
        || (dist - best_dist).abs() < 0.01 && best_id.is_none_or(|bid| id < bid)
}

/// Find the nearest alive enemy for each unit and assign as target.
/// Prefers targets with line of sight, but falls back to nearest enemy
/// without LOS so units will path toward hidden enemies.
/// Sentinel Taunt overrides targeting within 120 range.
pub fn update_targeting(
    units: &mut [Unit],
    obstacles: &[Obstacle],
    players: &[crate::match_progress::PlayerState],
) {
    let positions: Vec<(u64, u16, Vec2, bool)> = units
        .iter()
        .map(|u| (u.id, u.player_id, u.pos, u.alive))
        .collect();

    // Pre-compute taunting Sentinels: (id, team, pos)
    let taunters: Vec<(u64, u16, Vec2)> = units
        .iter()
        .filter(|u| u.alive && u.kind == UnitKind::Sentinel)
        .filter(|u| {
            players
                .iter()
                .find(|p| p.player_id == u.player_id)
                .is_some_and(|p| p.techs.has_tech(UnitKind::Sentinel, TechId::SentinelTaunt))
        })
        .map(|u| (u.id, u.player_id, u.pos))
        .collect();

    const TAUNT_RANGE: f32 = 120.0;

    for unit in units.iter_mut() {
        if !unit.alive {
            continue;
        }

        // Taunt override: if any enemy taunting Sentinel is within range, target it.
        let mut taunt_target: Option<(u64, f32)> = None;
        for &(tid, tteam, tpos) in &taunters {
            if tteam == unit.player_id {
                continue;
            }
            let d = unit.pos.distance(tpos);
            if d < TAUNT_RANGE && (taunt_target.is_none() || d < taunt_target.unwrap().1) {
                taunt_target = Some((tid, d));
            }
        }

        if let Some((tid, _)) = taunt_target {
            unit.target_id = Some(tid);
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
/// Also handles Entrench stationary tracking and Berserker Unstoppable slow immunity.
pub fn update_movement(
    units: &mut [Unit],
    dt: f32,
    arena_w: f32,
    arena_h: f32,
    obstacles: &[Obstacle],
    nav_grid: Option<&crate::terrain::NavGrid>,
    players: &[crate::match_progress::PlayerState],
) {
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

        // Track position for Entrench stationary detection (compare at end of iteration)
        let pre_move_pos = unit.pos;

        // Decrement slow timer
        if unit.slow_timer > 0.0 {
            unit.slow_timer = (unit.slow_timer - dt).max(0.0);
        }

        // Berserker Unstoppable: below 50% HP, immune to slow and +20% move speed
        let techs = players
            .iter()
            .find(|p| p.player_id == unit.player_id)
            .map(|p| &p.techs);
        let is_unstoppable = unit.kind == UnitKind::Berserker
            && (unit.hp / unit.stats.max_hp) < 0.5
            && techs
                .is_some_and(|t| t.has_tech(UnitKind::Berserker, TechId::BerserkerUnstoppable));

        // Effective move speed (halved if slowed; Unstoppable ignores slow and gets +20%)
        let effective_speed = if is_unstoppable {
            unit.stats.move_speed * 1.2
        } else if unit.slow_timer > 0.0 {
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

        // Entrench: track how long this unit has been stationary.
        // Threshold of 2.0 to ignore jitter from separation push.
        if unit.pos.distance(pre_move_pos) > 2.0 {
            unit.stationary_timer = 0.0;
        } else {
            unit.stationary_timer += dt;
        }
    }
}

/// Process attacks with tech effects.
pub fn update_attacks(
    units: &mut Vec<Unit>,
    projectiles: &mut Vec<Projectile>,
    dt: f32,
    players: &mut [crate::match_progress::PlayerState],
    splash_effects: &mut Vec<crate::rendering::SplashEffect>,
) {
    // Update cooldowns and decay Expendable buff timers
    for unit in units.iter_mut() {
        unit.update_cooldown(dt);
        if unit.expendable_timer > 0.0 {
            unit.expendable_timer = (unit.expendable_timer - dt).max(0.0);
            if unit.expendable_timer == 0.0 {
                unit.expendable_stacks = 0;
            }
        }
    }

    // Helper to get the right tech state for a player (immutable read)
    let tech_for_player = |pls: &[crate::match_progress::PlayerState], pid: u16| -> TechState {
        pls.iter().find(|p| p.player_id == pid).unwrap().techs.clone()
    };

    // === Interceptor rocket interception ===
    // interceptor_actions: (interceptor_id, projectile_index, team, interception_pos)
    let interceptor_actions: Vec<(u64, usize, u16, Vec2)> = {
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
                let ipos = projectiles[pi].pos;
                actions.push((unit.id, pi, unit.player_id, ipos));
                unit.reset_cooldown();
            }
        }
        actions
    };

    // Kill intercepted rockets and apply Flak Burst if tech is present.
    // Collect flak events first to avoid borrow issues with units.
    let mut flak_events: Vec<(Vec2, f32, f32, u16)> = Vec::new();  // (pos, dmg, radius, team)
    for &(_uid, proj_idx, team, ipos) in &interceptor_actions {
        if proj_idx >= projectiles.len() { continue; }
        let proj = &projectiles[proj_idx];
        if !proj.alive { continue; }
        let has_flak = players
            .iter()
            .find(|p| p.player_id == team)
            .is_some_and(|p| p.techs.has_tech(UnitKind::Interceptor, TechId::InterceptorFlak));
        if has_flak {
            flak_events.push((ipos, proj.damage, proj.splash_radius, team));
        }
        projectiles[proj_idx].alive = false;
    }
    // Apply flak bursts: splash damage at interception point using rocket's own damage + radius.
    // Enemies of the interceptor only (no friendly fire).
    for (pos, damage, radius, team) in flak_events {
        if radius <= 0.0 { continue; }
        splash_effects.push(crate::rendering::SplashEffect {
            pos,
            radius,
            timer: 0.3,
            max_timer: 0.3,
            player_id: team,
        });
        for unit in units.iter_mut() {
            if !unit.alive || unit.player_id == team {
                continue;
            }
            if unit.pos.distance(pos) < radius {
                unit.take_damage(damage);
            }
        }
    }

    // Interceptors that intercepted a rocket this frame are blocked from also
    // attacking units — UNLESS they have the DualWeapon tech.
    let intercepted_unit_ids: Vec<u64> = interceptor_actions
        .iter()
        .filter(|(_uid, _, team, _pos)| {
            let techs = tech_for_player(players, *team);
            !techs.has_tech(UnitKind::Interceptor, TechId::InterceptorDualWeapon)
        })
        .map(|(uid, _, _, _)| *uid)
        .collect();

    // === Chaff Overwhelm: precompute nearby-chaff counts ===
    let chaff_positions: Vec<(Vec2, u16)> = units
        .iter()
        .filter(|u| u.alive && u.kind == UnitKind::Chaff)
        .map(|u| (u.pos, u.player_id))
        .collect();

    // === Normal attacks ===
    let mut events: Vec<AttackEvent> = Vec::new();

    {
        let snapshot: Vec<(u64, Vec2, f32, bool, u16)> = units
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

            // Reset cooldown using tech-aware calculation (Entrench, Expendable)
            let techs = tech_for_player(players, unit.player_id);
            unit.attack_cooldown = cooldown_from_techs(unit, &techs);

            // Calculate bonus damage from Chaff Overwhelm tech (+3 per stack, max 10 stacks)
            let mut bonus_damage = 0.0;
            if unit.kind == UnitKind::Chaff && techs.has_tech(UnitKind::Chaff, TechId::ChaffOverwhelm) {
                let mut stacks: u32 = 0;
                for &(cpos, cteam) in &chaff_positions {
                    if cteam == unit.player_id && cpos.distance(unit.pos) < 50.0 && cpos != unit.pos {
                        stacks += 1;
                    }
                }
                bonus_damage += 3.0 * stacks.min(10) as f32;
            }

            let mut total_damage = unit.stats.damage + bonus_damage;

            // Bruiser Charge: first attack after 100+ distance from spawn deals 2x damage
            if unit.kind == UnitKind::Bruiser
                && !unit.has_charged
                && techs.has_tech(UnitKind::Bruiser, TechId::BruiserCharge)
                && unit.pos.distance(unit.spawn_pos) >= 100.0
            {
                total_damage *= 2.0;
                unit.has_charged = true;
            }

            if unit.is_melee() {
                let has_lifesteal = unit.kind == UnitKind::Berserker
                    && techs.has_tech(UnitKind::Berserker, TechId::BerserkerLifesteal);
                let cleave_ignores_armor = unit.kind == UnitKind::Bruiser
                    && techs.has_tech(UnitKind::Bruiser, TechId::BruiserCleave);

                events.push(AttackEvent::Melee {
                    attacker_id: unit.id,
                    attacker_kind: unit.kind,
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
    // Collect kill events for post-processing (Scavenge, Expendable, Death Throes)
    // KillEvent: (killer_id, killer_team, victim_id, victim_kind, victim_pos, victim_had_death_throes)
    let mut kill_events: Vec<(u64, u16, u64, UnitKind, Vec2, bool)> = Vec::new();

    for event in events {
        match event {
            AttackEvent::Melee {
                attacker_id,
                attacker_kind,
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
                    let target_kind = target.kind;
                    let target_pos_now = target.pos;
                    let target_team = target.player_id;
                    let (dealt, killed) = apply_damage(target, damage, false);
                    total_damage_dealt += dealt;
                    if killed {
                        kills += 1;
                        let victim_techs = tech_for_player(players, target_team);
                        let had_dt = target_kind == UnitKind::Berserker
                            && victim_techs.has_tech(UnitKind::Berserker, TechId::BerserkerDeathThroes);
                        kill_events.push((attacker_id, attacker_team, target_id, target_kind, target_pos_now, had_dt));
                    }
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
                    // Collect splash kill info before mutating (to avoid double-borrow of kill_events)
                    let mut splash_kills: Vec<(u64, UnitKind, Vec2, u16)> = Vec::new();
                    for unit in units.iter_mut() {
                        if !unit.alive || unit.id == target_id || unit.player_id == attacker_team {
                            continue;
                        }
                        if unit.pos.distance(target_pos) < splash_radius {
                            let victim_kind = unit.kind;
                            let victim_pos = unit.pos;
                            let victim_team = unit.player_id;
                            let victim_id = unit.id;
                            let (dealt, killed) = apply_damage(unit, damage, cleave_ignores_armor);
                            total_damage_dealt += dealt;
                            if killed {
                                kills += 1;
                                splash_kills.push((victim_id, victim_kind, victim_pos, victim_team));
                            }
                        }
                    }
                    for (vid, vkind, vpos, vteam) in splash_kills {
                        let victim_techs = tech_for_player(players, vteam);
                        let had_dt = vkind == UnitKind::Berserker
                            && victim_techs.has_tech(UnitKind::Berserker, TechId::BerserkerDeathThroes);
                        kill_events.push((attacker_id, attacker_team, vid, vkind, vpos, had_dt));
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
                // Suppress unused warning
                let _ = attacker_kind;
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

    // === Post-kill effects: Death Throes, Chaff Expendable, Chaff Scavenge ===
    for (killer_id, _killer_team, _victim_id, victim_kind, victim_pos, victim_had_death_throes) in kill_events {
        // Snapshot killer info (the killer might have died from a simultaneous effect, so check alive)
        let (killer_alive, killer_kind, killer_team) = units
            .iter()
            .find(|u| u.id == killer_id)
            .map(|u| (u.alive, u.kind, u.player_id))
            .unwrap_or((false, UnitKind::Chaff, 0));

        // Berserker Death Throes (victim's ability)
        if victim_had_death_throes {
            // Enemies of the victim's team take damage
            // Victim team is inferred from where the kill came from: victim was on the OTHER team from killer_team
            // Actually we stored killer_team, and victim was enemy, so victim_team != killer_team.
            // Apply damage to enemies of victim = allies of killer_team... wait that's wrong.
            // Death Throes hits enemies of the Berserker. The Berserker's enemies are units on killer_team.
            // So attacker_team (for the splash) should be the Berserker's team, NOT the killer's team.
            // But we need the victim's team. Since victim died to killer_team, victim's team != killer_team.
            // We need to look up the victim from kill_events... but victim is already dead.
            // Simpler: we know the victim was killed by the killer, and victim_team != killer_team.
            // The death splash damages enemies of victim (= killer_team allies),
            // but since the killer dealt the killing blow, there might be no other killer_team units nearby.
            // Since we don't have victim_team stored separately, and all enemies of victim are on killer_team,
            // we pass killer_team as the "attacker_team" for damage purposes (which skips damaging itself).
            apply_death_throes(units, victim_pos, killer_team, splash_effects);
        }

        // Chaff Expendable: when a Chaff dies, nearby allied chaff get attack speed buff
        if victim_kind == UnitKind::Chaff {
            // Find the victim's team (it's the OPPOSITE of killer_team if killer != victim)
            // Since the killer is an enemy, victim_team = NOT killer_team.
            // But we need to broadcast the buff to victim's ALLIES (same team as victim).
            // The victim is dead now — we need its team. Find any other chaff to check.
            // Simplest: the victim was killed by killer_team, so victim_team is any player_id != killer_team.
            // Since there are only 2 players in practice, find any player != killer_team.
            let victim_team = players
                .iter()
                .map(|p| p.player_id)
                .find(|&pid| pid != killer_team);
            if let Some(vteam) = victim_team {
                let vhas_expendable = tech_for_player(players, vteam)
                    .has_tech(UnitKind::Chaff, TechId::ChaffExpendable);
                if vhas_expendable {
                    for other in units.iter_mut() {
                        if !other.alive || other.kind != UnitKind::Chaff || other.player_id != vteam {
                            continue;
                        }
                        if other.pos.distance(victim_pos) < 40.0 {
                            other.expendable_stacks = (other.expendable_stacks + 1).min(3);
                            other.expendable_timer = 3.0;
                        }
                    }
                }
            }
        }

        // Chaff Scavenge: when a Chaff kills an enemy, spawn new chaff based on victim tier
        if killer_alive && killer_kind == UnitKind::Chaff {
            let killer_techs = tech_for_player(players, killer_team);
            if killer_techs.has_tech(UnitKind::Chaff, TechId::ChaffScavenge) {
                let spawn_count = crate::pack::unit_tier(victim_kind);
                if let Some(player) = players.iter_mut().find(|p| p.player_id == killer_team) {
                    for i in 0..spawn_count {
                        let angle = (i as f32) * std::f32::consts::TAU / spawn_count as f32;
                        let offset = vec2(angle.cos(), angle.sin()) * 10.0;
                        let new_unit = crate::pack::spawn_chaff_unit(
                            victim_pos + offset,
                            killer_team,
                            &player.techs,
                            &mut player.next_id,
                        );
                        units.push(new_unit);
                    }
                }
            }
        }
    }
}

/// Update projectiles with shield interception, evasion, pierce, and slow.
pub fn update_projectiles(
    projectiles: &mut Vec<Projectile>,
    units: &mut [Unit],
    dt: f32,
    obstacles: &mut [Obstacle],
    splash_effects: &mut Vec<crate::rendering::SplashEffect>,
    players: &[crate::match_progress::PlayerState],
) {
    let shields: Vec<(u64, u16, Vec2, f32, bool)> = units
        .iter()
        .filter(|u| u.is_shield() && u.alive)
        .map(|u| (u.id, u.player_id, u.pos, u.stats.shield_radius, u.shield_hp > 0.0))
        .collect();

    // Post-frame effects: Death Throes positions triggered by projectile kills
    let mut pending_death_throes: Vec<(Vec2, u16)> = Vec::new();

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
            let mut reflect_to_attacker: Option<f32> = None;
            if let Some(shield_unit) = units.iter_mut().find(|u| u.id == shield_id && u.alive) {
                // Damage the barrier's separate HP pool
                shield_unit.shield_hp = (shield_unit.shield_hp - proj.damage).max(0.0);
                // Reflective Barrier tech: reflect 15% of damage back to attacker (bypasses armor)
                let shield_team = shield_unit.player_id;
                let has_reflect = players
                    .iter()
                    .find(|p| p.player_id == shield_team)
                    .is_some_and(|p| p.techs.has_tech(UnitKind::Shield, TechId::ShieldReflect));
                if has_reflect {
                    reflect_to_attacker = Some(proj.damage * 0.15);
                }
            }
            if let Some(reflect_dmg) = reflect_to_attacker {
                if let Some(attacker) = units.iter_mut().find(|u| u.id == proj.attacker_id && u.alive) {
                    attacker.take_raw_damage(reflect_dmg);
                }
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

                let victim_kind = unit.kind;
                let victim_pos = unit.pos;
                let victim_team = unit.player_id;
                let (dealt, killed) = apply_damage(unit, proj.damage, proj.armor_pierce);
                proj_damage_dealt += dealt;
                if killed {
                    proj_kills += 1;
                    // Berserker Death Throes on ranged kill
                    if victim_kind == UnitKind::Berserker {
                        let had_dt = players
                            .iter()
                            .find(|p| p.player_id == victim_team)
                            .is_some_and(|p| p.techs.has_tech(UnitKind::Berserker, TechId::BerserkerDeathThroes));
                        if had_dt {
                            pending_death_throes.push((victim_pos, proj.player_id));
                        }
                    }
                }

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
                    let victim_kind = unit.kind;
                    let victim_pos = unit.pos;
                    let victim_team = unit.player_id;
                    let (dealt, killed) = apply_damage(unit, proj.damage, proj.armor_pierce);
                    proj_damage_dealt += dealt;
                    if killed {
                        proj_kills += 1;
                        if victim_kind == UnitKind::Berserker {
                            let had_dt = players
                                .iter()
                                .find(|p| p.player_id == victim_team)
                                .is_some_and(|p| p.techs.has_tech(UnitKind::Berserker, TechId::BerserkerDeathThroes));
                            if had_dt {
                                pending_death_throes.push((victim_pos, proj.player_id));
                            }
                        }
                    }
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

    // Apply Berserker Death Throes triggered by projectile kills
    for (pos, attacker_team) in pending_death_throes {
        apply_death_throes(units, pos, attacker_team, splash_effects);
    }
}

enum AttackEvent {
    Melee {
        attacker_id: u64,
        attacker_kind: UnitKind,
        target_id: u64,
        target_pos: Vec2,
        damage: f32,
        splash_radius: f32,
        attacker_team: u16,
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
        player_id: u16,
        splash_radius: f32,
        proj_type: crate::unit::ProjectileType,
        armor_pierce: bool,
        pierce_count: u8,
        applies_slow: bool,
    },
}
