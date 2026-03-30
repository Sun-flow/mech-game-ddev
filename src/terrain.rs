use macroquad::prelude::*;
use std::collections::BinaryHeap;
use std::cmp::Ordering;

use crate::arena::{ARENA_H, ARENA_W, HALF_W};

pub const GRID_CELL: f32 = 10.0;

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

/// Check if a straight line from `from` to `to` is unobstructed by walls.
/// Only Wall-type obstacles block line of sight (Cover does not).
pub fn has_line_of_sight(from: Vec2, to: Vec2, obstacles: &[Obstacle]) -> bool {
    let dir = to - from;
    let len = dir.length();
    if len < 0.001 {
        return true;
    }

    for obs in obstacles {
        if !obs.alive || obs.obstacle_type != ObstacleType::Wall {
            continue;
        }

        let obs_min = obs.pos - obs.half_size;
        let obs_max = obs.pos + obs.half_size;

        // Slab method for ray-AABB intersection
        let inv_dir = vec2(
            if dir.x.abs() < 0.0001 { f32::MAX.copysign(dir.x) } else { 1.0 / dir.x },
            if dir.y.abs() < 0.0001 { f32::MAX.copysign(dir.y) } else { 1.0 / dir.y },
        );

        let t1x = (obs_min.x - from.x) * inv_dir.x;
        let t2x = (obs_max.x - from.x) * inv_dir.x;
        let t1y = (obs_min.y - from.y) * inv_dir.y;
        let t2y = (obs_max.y - from.y) * inv_dir.y;

        let t_enter = t1x.min(t2x).max(t1y.min(t2y));
        let t_exit = t1x.max(t2x).min(t1y.max(t2y));

        // Intersection if t_enter <= t_exit and the interval overlaps [0, 1]
        if t_enter <= t_exit && t_enter < 1.0 && t_exit > 0.0 {
            return false;
        }
    }

    true
}

/// Check LOS using two parallel rays offset by `half_width` perpendicular to the aim direction.
/// Both rays must be clear for LOS to return true, preventing bullets from clipping wall corners.
pub fn has_line_of_sight_wide(from: Vec2, to: Vec2, half_width: f32, obstacles: &[Obstacle]) -> bool {
    let dir = to - from;
    let len = dir.length();
    if len < 0.001 {
        return true;
    }
    let norm = dir / len;
    let perp = vec2(-norm.y, norm.x);
    let offset = perp * half_width;

    has_line_of_sight(from + offset, to + offset, obstacles)
        && has_line_of_sight(from - offset, to - offset, obstacles)
}

/// Check if a ray segment from `from` to `to` hits any obstacle that blocks projectiles for the given team.
/// Used for swept projectile collision to prevent tunneling.
pub fn ray_hits_blocking_obstacle(from: Vec2, to: Vec2, team_id: u8, obstacles: &[Obstacle]) -> bool {
    let dir = to - from;
    let len = dir.length();
    if len < 0.001 {
        return false;
    }

    for obs in obstacles {
        if !obs.alive || !obs.blocks_projectile(team_id) {
            continue;
        }

        let obs_min = obs.pos - obs.half_size;
        let obs_max = obs.pos + obs.half_size;

        let inv_dir = vec2(
            if dir.x.abs() < 0.0001 { f32::MAX.copysign(dir.x) } else { 1.0 / dir.x },
            if dir.y.abs() < 0.0001 { f32::MAX.copysign(dir.y) } else { 1.0 / dir.y },
        );

        let t1x = (obs_min.x - from.x) * inv_dir.x;
        let t2x = (obs_max.x - from.x) * inv_dir.x;
        let t1y = (obs_min.y - from.y) * inv_dir.y;
        let t2y = (obs_max.y - from.y) * inv_dir.y;

        let t_enter = t1x.min(t2x).max(t1y.min(t2y));
        let t_exit = t1x.max(t2x).min(t1y.max(t2y));

        if t_enter <= t_exit && t_enter < 1.0 && t_exit > 0.0 {
            return true;
        }
    }

    false
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

/// Reset destructible cover to full HP (call between rounds to preserve layout).
pub fn reset_cover_hp(obstacles: &mut [Obstacle]) {
    for obs in obstacles.iter_mut() {
        if obs.obstacle_type == ObstacleType::Cover {
            obs.hp = obs.max_hp;
            obs.alive = true;
        }
    }
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

// === A* Pathfinding ===

/// Navigation grid for A* pathfinding. true = passable.
pub struct NavGrid {
    pub cells: Vec<bool>,
    pub width: usize,
    pub height: usize,
}

impl NavGrid {
    /// Build a nav grid from obstacles. Cells overlapping Wall obstacles (inflated by padding) are impassable.
    pub fn from_obstacles(obstacles: &[Obstacle], arena_w: f32, arena_h: f32, padding: f32) -> Self {
        let width = (arena_w / GRID_CELL) as usize;
        let height = (arena_h / GRID_CELL) as usize;
        let mut cells = vec![true; width * height];

        for gy in 0..height {
            for gx in 0..width {
                let world = grid_to_world(gx, gy);
                for obs in obstacles {
                    if !obs.alive || !obs.blocks_movement() { continue; }
                    let obs_min = obs.pos - obs.half_size - vec2(padding, padding);
                    let obs_max = obs.pos + obs.half_size + vec2(padding, padding);
                    if world.x >= obs_min.x && world.x <= obs_max.x
                        && world.y >= obs_min.y && world.y <= obs_max.y
                    {
                        cells[gy * width + gx] = false;
                        break;
                    }
                }
            }
        }

        NavGrid { cells, width, height }
    }

    fn passable(&self, gx: usize, gy: usize) -> bool {
        if gx >= self.width || gy >= self.height { return false; }
        self.cells[gy * self.width + gx]
    }
}

pub fn world_to_grid(pos: Vec2) -> (usize, usize) {
    let gx = ((pos.x / GRID_CELL) as usize).min(((ARENA_W / GRID_CELL) as usize).saturating_sub(1));
    let gy = ((pos.y / GRID_CELL) as usize).min(((ARENA_H / GRID_CELL) as usize).saturating_sub(1));
    (gx, gy)
}

pub fn grid_to_world(gx: usize, gy: usize) -> Vec2 {
    vec2(gx as f32 * GRID_CELL + GRID_CELL * 0.5, gy as f32 * GRID_CELL + GRID_CELL * 0.5)
}

#[derive(Clone, Copy)]
struct AStarNode {
    f: f32,
    g: f32,
    pos: (usize, usize),
}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool { self.f == other.f && self.pos == other.pos }
}
impl Eq for AStarNode {}
impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Primary: lower f-score is better (reversed for max-heap)
        // Tiebreak: deterministic ordering by grid position (y then x)
        match other.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal) {
            Ordering::Equal => {
                match self.pos.1.cmp(&other.pos.1) {
                    Ordering::Equal => self.pos.0.cmp(&other.pos.0),
                    ord => ord,
                }
            }
            ord => ord,
        }
    }
}

fn octile_dist(a: (usize, usize), b: (usize, usize)) -> f32 {
    let dx = (a.0 as f32 - b.0 as f32).abs();
    let dy = (a.1 as f32 - b.1 as f32).abs();
    let (min, max) = if dx < dy { (dx, dy) } else { (dy, dx) };
    max + min * (std::f32::consts::SQRT_2 - 1.0)
}

const DIRS: [(i32, i32); 8] = [
    (1, 0), (-1, 0), (0, 1), (0, -1),
    (1, 1), (1, -1), (-1, 1), (-1, -1),
];

/// Find a path from `from` to `to` on the nav grid. Returns waypoints in world coords.
pub fn find_path(grid: &NavGrid, from: Vec2, to: Vec2) -> Option<Vec<Vec2>> {
    let start = world_to_grid(from);
    let goal = world_to_grid(to);

    let start = nearest_passable(grid, start);
    let goal = nearest_passable(grid, goal);

    if start == goal {
        return Some(vec![to]);
    }

    let size = grid.width * grid.height;
    let mut g_scores = vec![f32::MAX; size];
    let mut came_from: Vec<Option<(usize, usize)>> = vec![None; size];
    let mut open = BinaryHeap::new();

    let si = start.1 * grid.width + start.0;
    g_scores[si] = 0.0;
    open.push(AStarNode { f: octile_dist(start, goal), g: 0.0, pos: start });

    while let Some(current) = open.pop() {
        let (cx, cy) = current.pos;
        if (cx, cy) == goal {
            let mut path = Vec::new();
            let mut node = goal;
            while node != start {
                path.push(grid_to_world(node.0, node.1));
                match came_from[node.1 * grid.width + node.0] {
                    Some(prev) => node = prev,
                    None => break,
                }
            }
            path.reverse();
            if let Some(last) = path.last_mut() {
                *last = to;
            }
            return Some(smooth_path(grid, from, &path));
        }

        let ci = cy * grid.width + cx;
        if current.g > g_scores[ci] { continue; }

        for &(dx, dy) in &DIRS {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx < 0 || ny < 0 { continue; }
            let (nx, ny) = (nx as usize, ny as usize);
            if !grid.passable(nx, ny) { continue; }

            if dx != 0 && dy != 0 {
                if !grid.passable(cx, ny) || !grid.passable(nx, cy) { continue; }
            }

            let cost = if dx != 0 && dy != 0 { std::f32::consts::SQRT_2 } else { 1.0 };
            let new_g = current.g + cost;
            let ni = ny * grid.width + nx;
            if new_g < g_scores[ni] {
                g_scores[ni] = new_g;
                came_from[ni] = Some((cx, cy));
                open.push(AStarNode { f: new_g + octile_dist((nx, ny), goal), g: new_g, pos: (nx, ny) });
            }
        }
    }

    None
}

fn nearest_passable(grid: &NavGrid, pos: (usize, usize)) -> (usize, usize) {
    if grid.passable(pos.0, pos.1) { return pos; }
    for r in 1..20 {
        for dx in -(r as i32)..=(r as i32) {
            for dy in -(r as i32)..=(r as i32) {
                if dx.abs() != r && dy.abs() != r { continue; }
                let nx = pos.0 as i32 + dx;
                let ny = pos.1 as i32 + dy;
                if nx < 0 || ny < 0 { continue; }
                let (nx, ny) = (nx as usize, ny as usize);
                if grid.passable(nx, ny) { return (nx, ny); }
            }
        }
    }
    pos
}

fn smooth_path(grid: &NavGrid, start: Vec2, path: &[Vec2]) -> Vec<Vec2> {
    if path.len() <= 1 { return path.to_vec(); }
    let mut result = Vec::new();
    let mut current = start;
    let mut i = 0;
    while i < path.len() {
        let mut farthest = i;
        for j in (i + 1)..path.len() {
            if grid_line_clear(grid, current, path[j]) {
                farthest = j;
            } else {
                break;
            }
        }
        result.push(path[farthest]);
        current = path[farthest];
        i = farthest + 1;
    }
    result
}

fn grid_line_clear(grid: &NavGrid, from: Vec2, to: Vec2) -> bool {
    let dist = from.distance(to);
    let steps = (dist / (GRID_CELL * 0.5)) as usize + 1;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let p = from.lerp(to, t);
        let (gx, gy) = world_to_grid(p);
        if !grid.passable(gx, gy) { return false; }
    }
    true
}
