use macroquad::prelude::*;

use crate::arena::{ARENA_H, ARENA_W, HALF_W};

pub const GRID_CELL: f32 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObstacleType {
    Wall,  // Indestructible, blocks all projectiles and movement
    Cover, // Destructible, blocks enemy projectiles, allows movement through
}

#[derive(Clone, Debug)]
pub struct Obstacle {
    pub pos: Vec2,          // center position
    pub half_size: Vec2,    // half-width and half-height
    pub obstacle_type: ObstacleType,
    pub hp: f32,
    pub max_hp: f32,
    pub team_id: u8,        // 255 = neutral, 0 = player, 1 = opponent
    pub alive: bool,
}

impl Obstacle {
    pub fn wall(pos: Vec2, half_size: Vec2) -> Self {
        Self {
            pos,
            half_size,
            obstacle_type: ObstacleType::Wall,
            hp: f32::MAX,
            max_hp: f32::MAX,
            team_id: 255,
            alive: true,
        }
    }

    pub fn cover(pos: Vec2, half_size: Vec2, hp: f32, team_id: u8) -> Self {
        Self {
            pos,
            half_size,
            obstacle_type: ObstacleType::Cover,
            hp,
            max_hp: hp,
            team_id,
            alive: true,
        }
    }

    /// AABB contains check
    pub fn contains_point(&self, point: Vec2) -> bool {
        let min = self.pos - self.half_size;
        let max = self.pos + self.half_size;
        point.x >= min.x && point.x <= max.x && point.y >= min.y && point.y <= max.y
    }

    /// Check if a circle (unit/projectile) intersects this obstacle's AABB
    pub fn intersects_circle(&self, center: Vec2, radius: f32) -> bool {
        let min = self.pos - self.half_size;
        let max = self.pos + self.half_size;
        // Find closest point on AABB to circle center
        let closest = vec2(
            center.x.clamp(min.x, max.x),
            center.y.clamp(min.y, max.y),
        );
        closest.distance(center) < radius
    }

    /// Does this obstacle block a projectile from the given team?
    pub fn blocks_projectile(&self, proj_team_id: u8) -> bool {
        if !self.alive { return false; }
        match self.obstacle_type {
            ObstacleType::Wall => true, // Walls block everything
            ObstacleType::Cover => {
                // Cover blocks enemy projectiles, allows friendly through
                if self.team_id == 255 {
                    true // Neutral cover blocks all
                } else {
                    self.team_id != proj_team_id // Block enemy, allow friendly
                }
            }
        }
    }

    /// Does this obstacle block unit movement?
    pub fn blocks_movement(&self) -> bool {
        self.alive && self.obstacle_type == ObstacleType::Wall
    }

    pub fn take_damage(&mut self, damage: f32) {
        if self.obstacle_type != ObstacleType::Cover { return; }
        self.hp -= damage;
        if self.hp <= 0.0 {
            self.hp = 0.0;
            self.alive = false;
        }
    }
}

/// Generate terrain layout for a round, seeded deterministically.
/// Places obstacles symmetrically (mirrored across center line).
pub fn generate_terrain(round: u32, destructible: bool) -> Vec<Obstacle> {
    let mut obstacles = Vec::new();

    // Seed-based generation using round number
    // Use simple deterministic placement based on round
    let seed = round as usize;

    // Central wall patterns (always 1-3 walls in the middle zone)
    let wall_configs: &[&[(f32, f32, f32, f32)]] = &[
        // Round pattern 1: single central wall
        &[(HALF_W, ARENA_H * 0.5, 40.0, 80.0)],
        // Round pattern 2: two offset walls
        &[(HALF_W, ARENA_H * 0.3, 30.0, 60.0), (HALF_W, ARENA_H * 0.7, 30.0, 60.0)],
        // Round pattern 3: three walls forming corridors
        &[(HALF_W, ARENA_H * 0.25, 50.0, 40.0), (HALF_W, ARENA_H * 0.5, 50.0, 40.0), (HALF_W, ARENA_H * 0.75, 50.0, 40.0)],
        // Round pattern 4: L-shaped walls
        &[(HALF_W - 30.0, ARENA_H * 0.4, 20.0, 60.0), (HALF_W + 30.0, ARENA_H * 0.6, 20.0, 60.0)],
        // Round pattern 5: wide central barrier with gaps
        &[(HALF_W, ARENA_H * 0.2, 60.0, 30.0), (HALF_W, ARENA_H * 0.8, 60.0, 30.0)],
    ];

    let pattern = &wall_configs[seed % wall_configs.len()];
    for &(x, y, hw, hh) in *pattern {
        obstacles.push(Obstacle::wall(vec2(x, y), vec2(hw, hh)));
    }

    // Add destructible cover if enabled
    if destructible {
        let cover_configs: &[&[(f32, f32, f32, f32, u8)]] = &[
            // (x, y, hw, hh, team_id)
            &[(350.0, ARENA_H * 0.4, 25.0, 20.0, 0), (ARENA_W - 350.0, ARENA_H * 0.4, 25.0, 20.0, 1),
              (350.0, ARENA_H * 0.6, 25.0, 20.0, 0), (ARENA_W - 350.0, ARENA_H * 0.6, 25.0, 20.0, 1)],
            &[(400.0, ARENA_H * 0.3, 20.0, 30.0, 0), (ARENA_W - 400.0, ARENA_H * 0.3, 20.0, 30.0, 1),
              (400.0, ARENA_H * 0.7, 20.0, 30.0, 0), (ARENA_W - 400.0, ARENA_H * 0.7, 20.0, 30.0, 1)],
            &[(300.0, ARENA_H * 0.5, 30.0, 25.0, 0), (ARENA_W - 300.0, ARENA_H * 0.5, 30.0, 25.0, 1)],
        ];

        let cover_pattern = &cover_configs[seed % cover_configs.len()];
        for &(x, y, hw, hh, tid) in *cover_pattern {
            obstacles.push(Obstacle::cover(vec2(x, y), vec2(hw, hh), 2000.0, tid));
        }
    }

    obstacles
}

/// Draw all obstacles.
pub fn draw_obstacles(obstacles: &[Obstacle]) {
    for obs in obstacles {
        if !obs.alive { continue; }

        let min = obs.pos - obs.half_size;
        let w = obs.half_size.x * 2.0;
        let h = obs.half_size.y * 2.0;

        match obs.obstacle_type {
            ObstacleType::Wall => {
                draw_rectangle(min.x, min.y, w, h, Color::new(0.3, 0.3, 0.35, 0.9));
                draw_rectangle_lines(min.x, min.y, w, h, 2.0, Color::new(0.5, 0.5, 0.55, 1.0));
            }
            ObstacleType::Cover => {
                let hp_frac = obs.hp / obs.max_hp;
                let alpha = 0.4 + 0.4 * hp_frac;
                let color = if obs.team_id == 0 {
                    Color::new(0.6, 0.3, 0.2, alpha)
                } else if obs.team_id == 1 {
                    Color::new(0.2, 0.3, 0.6, alpha)
                } else {
                    Color::new(0.4, 0.4, 0.3, alpha)
                };
                draw_rectangle(min.x, min.y, w, h, color);
                draw_rectangle_lines(min.x, min.y, w, h, 1.5, Color::new(0.6, 0.6, 0.5, 0.6));

                // HP bar for cover
                if hp_frac < 1.0 {
                    let bar_w = w;
                    let bar_h = 3.0;
                    let bar_y = min.y - 6.0;
                    draw_rectangle(min.x, bar_y, bar_w, bar_h, DARKGRAY);
                    let hp_color = if hp_frac > 0.5 { GREEN } else if hp_frac > 0.25 { YELLOW } else { RED };
                    draw_rectangle(min.x, bar_y, bar_w * hp_frac, bar_h, hp_color);
                }
            }
        }
    }
}
