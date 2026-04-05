use macroquad::prelude::*;

use crate::arena::{draw_center_divider, ARENA_H, ARENA_W, HALF_W};
use crate::game_state::{BuildState, PlacedPack};
use crate::match_progress::MatchProgress;
use crate::pack::all_packs;
use crate::projectile::{projectile_visual_radius, Projectile};
use crate::team::{team_color, team_projectile_color};
use crate::terrain;
use crate::unit::{draw_unit_shape, ProjectileType, Unit, UnitKind};

/// Visual effect for AOE splash damage (expanding, fading circle).
pub struct SplashEffect {
    pub pos: Vec2,
    pub radius: f32,
    pub timer: f32,
    pub max_timer: f32,
    pub player_id: u8,
}

pub fn update_splash_effects(effects: &mut Vec<SplashEffect>, dt: f32) {
    for effect in effects.iter_mut() {
        effect.timer -= dt;
    }
    effects.retain(|e| e.timer > 0.0);
}

pub fn draw_world(
    units: &[Unit],
    projectiles: &[Projectile],
    obstacles: &[crate::terrain::Obstacle],
    splash_effects: &[SplashEffect],
    show_grid: bool,
) {
    draw_rectangle_lines(0.0, 0.0, ARENA_W, ARENA_H, 2.0, GRAY);
    draw_center_divider();
    terrain::draw_obstacles(obstacles);

    if show_grid {
        draw_grid();
    }

    draw_shields(units);
    draw_units(units);
    draw_projectiles(projectiles);
    draw_splash_effects(splash_effects);
}

fn draw_grid() {
    let grid = terrain::GRID_CELL;
    let line_color = Color::new(0.3, 0.3, 0.35, 0.15);
    let mut gx = 0.0;
    while gx <= ARENA_W {
        draw_line(gx, 0.0, gx, ARENA_H, 1.0, line_color);
        gx += grid;
    }
    let mut gy = 0.0;
    while gy <= ARENA_H {
        draw_line(0.0, gy, ARENA_W, gy, 1.0, line_color);
        gy += grid;
    }
}

fn draw_shields(units: &[Unit]) {
    for unit in units {
        if !unit.alive || !unit.is_shield() || unit.shield_hp <= 0.0 {
            continue;
        }
        let tc = team_color(unit.player_id);
        let shield_frac = if unit.stats.shield_hp > 0.0 { unit.shield_hp / unit.stats.shield_hp } else { 0.0 };
        let alpha = 0.12 + 0.12 * shield_frac;
        draw_circle(
            unit.pos.x,
            unit.pos.y,
            unit.stats.shield_radius,
            Color::new(tc.r, tc.g, tc.b, alpha),
        );
        draw_circle_lines(
            unit.pos.x,
            unit.pos.y,
            unit.stats.shield_radius,
            1.5,
            Color::new(tc.r, tc.g, tc.b, 0.4 * shield_frac + 0.1),
        );
    }
}

fn draw_units(units: &[Unit]) {
    for unit in units {
        if !unit.alive && unit.death_timer <= 0.0 {
            continue;
        }

        // Death animation: shrink and fade
        if !unit.alive && unit.death_timer > 0.0 {
            let frac = unit.death_timer / 0.5;
            let alpha = frac * 0.8;
            let draw_size = unit.stats.size * frac;
            let mut color = team_color(unit.player_id);
            color.a = alpha;
            draw_unit_shape(unit.pos, draw_size, unit.stats.shape, color);
            continue;
        }

        let mut color = team_color(unit.player_id);
        if unit.kind == UnitKind::Berserker {
            let hp_frac = unit.hp / unit.stats.max_hp;
            let rage = 1.0 - hp_frac;
            color.r = (color.r + rage * 0.5).min(1.0);
            color.g = (color.g * (1.0 - rage * 0.5)).max(0.1);
        }
        // Slow visual indicator
        if unit.slow_timer > 0.0 {
            draw_circle_lines(
                unit.pos.x,
                unit.pos.y,
                unit.stats.size + 3.0,
                1.0,
                Color::new(0.2, 0.5, 1.0, 0.5),
            );
        }
        draw_unit_shape(unit.pos, unit.stats.size, unit.stats.shape, color);
        // HP bar (only show when damaged)
        let hp_frac = unit.hp / unit.stats.max_hp;
        if hp_frac < 1.0 {
            let bar_w = unit.stats.size * 2.0;
            let bar_h = 3.0;
            let bar_x = unit.pos.x - bar_w / 2.0;
            let bar_y = unit.pos.y - unit.stats.size - 8.0;
            draw_rectangle(bar_x, bar_y, bar_w, bar_h, DARKGRAY);
            let hp_color = if hp_frac > 0.5 {
                GREEN
            } else if hp_frac > 0.25 {
                YELLOW
            } else {
                RED
            };
            draw_rectangle(bar_x, bar_y, bar_w * hp_frac, bar_h, hp_color);
        }
    }
}

fn draw_projectiles(projectiles: &[Projectile]) {
    for proj in projectiles {
        if !proj.alive {
            continue;
        }
        let color = team_projectile_color(proj.player_id);
        let r = projectile_visual_radius(proj.proj_type);
        match proj.proj_type {
            ProjectileType::Laser => {
                let dir = proj.vel.normalize_or_zero();
                let tail = proj.pos - dir * 8.0;
                draw_line(tail.x, tail.y, proj.pos.x, proj.pos.y, 2.0, color);
                draw_circle(proj.pos.x, proj.pos.y, r, WHITE);
            }
            ProjectileType::Bullet => {
                draw_circle(proj.pos.x, proj.pos.y, r, color);
            }
            ProjectileType::Rocket => {
                let dir = proj.vel.normalize_or_zero();
                let tail = proj.pos - dir * 6.0;
                draw_line(
                    tail.x,
                    tail.y,
                    proj.pos.x,
                    proj.pos.y,
                    3.0,
                    Color::new(1.0, 0.5, 0.2, 0.4),
                );
                draw_circle(proj.pos.x, proj.pos.y, r, color);
            }
        }
    }
}

fn draw_splash_effects(effects: &[SplashEffect]) {
    for effect in effects {
        let progress = 1.0 - (effect.timer / effect.max_timer);
        let current_radius = effect.radius * (0.3 + 0.7 * progress);
        let alpha = 0.4 * (effect.timer / effect.max_timer);
        let tc = team_color(effect.player_id);
        draw_circle(effect.pos.x, effect.pos.y, current_radius,
            Color::new(tc.r, tc.g, tc.b, alpha * 0.3));
        draw_circle_lines(effect.pos.x, effect.pos.y, current_radius, 2.0,
            Color::new(tc.r, tc.g, tc.b, alpha));
    }
}

pub fn draw_build_overlays(build: &BuildState, progress: &MatchProgress, world_mouse: Vec2) {
    // Placement zone overlay
    draw_rectangle(0.0, 0.0, HALF_W, ARENA_H, Color::new(0.2, 0.3, 0.5, 0.05));
    draw_rectangle(HALF_W, 0.0, HALF_W, ARENA_H, Color::new(0.5, 0.2, 0.2, 0.05));

    // Drag-box selection rectangle
    if let Some(box_start) = build.drag_box_start {
        let box_end = world_mouse;
        let min_x = box_start.x.min(box_end.x);
        let min_y = box_start.y.min(box_end.y);
        let w = (box_start.x - box_end.x).abs();
        let h = (box_start.y - box_end.y).abs();
        draw_rectangle(min_x, min_y, w, h, Color::new(0.2, 0.5, 1.0, 0.15));
        draw_rectangle_lines(min_x, min_y, w, h, 1.5, Color::new(0.3, 0.6, 1.0, 0.8));
    }

    // Pack bounding boxes
    let packs = all_packs();
    for (i, placed) in build.placed_packs.iter().enumerate() {
        let pack = &packs[placed.pack_index];
        let half = placed.bbox_half_size_for(pack);
        let min = placed.center - half;

        let is_multi_dragged = build.multi_dragging.contains(&i);
        let bbox_color = if is_multi_dragged {
            // Check overlap against non-dragged packs
            let mut overlap = false;
            for (j, other) in build.placed_packs.iter().enumerate() {
                if build.multi_dragging.contains(&j) { continue; }
                let p1 = &packs[placed.pack_index];
                let p2 = &packs[other.pack_index];
                if placed.overlaps(other, p1, p2) { overlap = true; break; }
            }
            if overlap { Color::new(1.0, 0.2, 0.2, 0.6) } else { Color::new(0.2, 1.0, 0.3, 0.5) }
        } else if build.dragging == Some(i)
            && build.would_overlap(
                placed.center,
                placed.pack_index,
                Some(i),
                placed.rotated,
            )
        {
            Color::new(1.0, 0.2, 0.2, 0.6)
        } else if build.dragging == Some(i) {
            Color::new(0.2, 1.0, 0.3, 0.5)
        } else if build.selected_pack == Some(i) {
            Color::new(0.2, 0.8, 1.0, 0.8) // cyan highlight for selected
        } else if placed.locked {
            Color::new(0.3, 0.3, 0.4, 0.25) // dimmer for locked
        } else {
            Color::new(0.5, 0.5, 0.5, 0.3)
        };

        let thickness = if build.selected_pack == Some(i) {
            2.5
        } else {
            1.5
        };
        draw_rectangle_lines(
            min.x,
            min.y,
            half.x * 2.0,
            half.y * 2.0,
            thickness,
            bbox_color,
        );

        // Pack label — collected for screen-space drawing below
    }

    // Opponent pack bounding boxes (from previous rounds, visible during build)
    let packs = all_packs();
    for opponent_pack in &progress.opponent_packs {
        let pack = &packs[opponent_pack.pack_index];
        let half = PlacedPack::bbox_half_size_rotated(
            pack,
            opponent_pack.rotated,
        );
        let min = opponent_pack.center - half;
        let bbox_color = Color::new(0.3, 0.3, 0.5, 0.2);
        draw_rectangle_lines(
            min.x,
            min.y,
            half.x * 2.0,
            half.y * 2.0,
            1.0,
            bbox_color,
        );
    }
}
