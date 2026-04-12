//! Determinism tests for the combat simulation.
//!
//! These tests run the combat loop headlessly (no window, no rendering) to verify
//! that identical inputs produce identical outputs. Any divergence between two
//! runs of the same scenario indicates a source of non-determinism that would
//! cause multiplayer desyncs.
//!
//! Run with: `cargo test determinism`

#![cfg(test)]

use macroquad::prelude::*;

use std::collections::VecDeque;

use crate::arena::{ARENA_H, ARENA_W};
use crate::combat::{update_attacks, update_movement, update_projectiles, update_targeting};
use crate::match_progress::PlayerState;
use crate::pack::{all_packs, spawn_pack_units};
use crate::projectile::Projectile;
use crate::rendering::SplashEffect;
use crate::sync::compute_state_hash;
use crate::tech::TechId;
use crate::terrain::{NavGrid, Obstacle};
use crate::unit::{Unit, UnitKind};

const DT: f32 = 1.0 / 60.0;
const SEED: u64 = 12345;

// ============================================================================
// State builder
// ============================================================================

#[derive(Clone)]
struct BattleState {
    units: Vec<Unit>,
    projectiles: Vec<Projectile>,
    obstacles: Vec<Obstacle>,
    players: Vec<PlayerState>,
    splash_effects: Vec<SplashEffect>,
}

impl BattleState {
    /// Create an empty 2-player battle state with deploy zones set.
    fn new() -> Self {
        let mut p1 = PlayerState::new(1);
        p1.deploy_zone = (0.0, ARENA_W / 2.0);

        let mut p2 = PlayerState::new(2);
        p2.deploy_zone = (ARENA_W / 2.0, ARENA_W);

        Self {
            units: Vec::new(),
            projectiles: Vec::new(),
            obstacles: Vec::new(),
            players: vec![p1, p2],
            splash_effects: Vec::new(),
        }
    }

    /// Place a pack at a specific position for a given player.
    fn place_pack(&mut self, kind: UnitKind, center: Vec2, player_id: u16) {
        let pack = all_packs()
            .iter()
            .find(|p| p.kind == kind)
            .expect("pack kind not found");
        let player = self
            .players
            .iter_mut()
            .find(|p| p.player_id == player_id)
            .expect("player not found");
        let (spawned, _ids) = spawn_pack_units(
            pack,
            center,
            false,
            player_id,
            &player.techs,
            &mut player.next_id,
        );
        self.units.extend(spawned);
    }

    /// Purchase a tech for a player and refresh their units' stats.
    fn add_tech(&mut self, player_id: u16, kind: UnitKind, tech_id: TechId) {
        let player = self
            .players
            .iter_mut()
            .find(|p| p.player_id == player_id)
            .expect("player not found");
        player.techs.purchase(kind, tech_id);
        let techs = player.techs.clone();
        crate::tech::refresh_units_of_kind(&mut self.units, kind, &techs);
    }
}

// ============================================================================
// Frame runner
// ============================================================================

fn run_one_frame(state: &mut BattleState, nav_grid: &NavGrid) {
    update_targeting(&mut state.units, &state.obstacles, &state.players);
    update_movement(
        &mut state.units,
        DT,
        ARENA_W,
        ARENA_H,
        &state.obstacles,
        Some(nav_grid),
        &state.players,
    );
    update_attacks(
        &mut state.units,
        &mut state.projectiles,
        DT,
        &mut state.players,
        &mut state.splash_effects,
    );
    update_projectiles(
        &mut state.projectiles,
        &mut state.units,
        DT,
        &mut state.obstacles,
        &mut state.splash_effects,
        &state.players,
    );
    // Death animation timer decrement (matches battle_phase.rs)
    for u in state.units.iter_mut() {
        if !u.alive && u.death_timer > 0.0 {
            u.death_timer -= DT;
        }
    }
}

// ============================================================================
// Field-by-field state diffing
// ============================================================================

const F32_EPSILON: f32 = 1e-6;

fn diff_f32(label: &str, a: f32, b: f32, diffs: &mut Vec<String>) {
    if (a - b).abs() > F32_EPSILON || a.is_nan() != b.is_nan() {
        diffs.push(format!("  {}: {} vs {}", label, a, b));
    }
}

fn diff_unit(a: &Unit, b: &Unit, diffs: &mut Vec<String>) {
    if a.id != b.id {
        diffs.push(format!("  unit id: {} vs {}", a.id, b.id));
        return;
    }
    let prefix = format!("unit {} ({:?})", a.id, a.kind);
    if a.alive != b.alive {
        diffs.push(format!("{} alive: {} vs {}", prefix, a.alive, b.alive));
    }
    diff_f32(&format!("{} hp", prefix), a.hp, b.hp, diffs);
    diff_f32(&format!("{} pos.x", prefix), a.pos.x, b.pos.x, diffs);
    diff_f32(&format!("{} pos.y", prefix), a.pos.y, b.pos.y, diffs);
    diff_f32(&format!("{} spawn_pos.x", prefix), a.spawn_pos.x, b.spawn_pos.x, diffs);
    diff_f32(&format!("{} spawn_pos.y", prefix), a.spawn_pos.y, b.spawn_pos.y, diffs);
    if a.target_id != b.target_id {
        diffs.push(format!("{} target_id: {:?} vs {:?}", prefix, a.target_id, b.target_id));
    }
    diff_f32(&format!("{} attack_cooldown", prefix), a.attack_cooldown, b.attack_cooldown, diffs);
    diff_f32(&format!("{} death_timer", prefix), a.death_timer, b.death_timer, diffs);
    diff_f32(&format!("{} slow_timer", prefix), a.slow_timer, b.slow_timer, diffs);
    diff_f32(&format!("{} shield_hp", prefix), a.shield_hp, b.shield_hp, diffs);
    diff_f32(&format!("{} stationary_timer", prefix), a.stationary_timer, b.stationary_timer, diffs);
    if a.has_charged != b.has_charged {
        diffs.push(format!("{} has_charged: {} vs {}", prefix, a.has_charged, b.has_charged));
    }
    if a.expendable_stacks != b.expendable_stacks {
        diffs.push(format!("{} expendable_stacks: {} vs {}", prefix, a.expendable_stacks, b.expendable_stacks));
    }
    diff_f32(&format!("{} expendable_timer", prefix), a.expendable_timer, b.expendable_timer, diffs);
    // Path length only (comparing Vec2 waypoints frame-by-frame is usually just noise)
    if a.path.len() != b.path.len() {
        diffs.push(format!("{} path.len: {} vs {}", prefix, a.path.len(), b.path.len()));
    }
    diff_f32(&format!("{} damage_dealt_total", prefix), a.damage_dealt_total, b.damage_dealt_total, diffs);
    diff_f32(&format!("{} damage_soaked_total", prefix), a.damage_soaked_total, b.damage_soaked_total, diffs);
    if a.kills_total != b.kills_total {
        diffs.push(format!("{} kills_total: {} vs {}", prefix, a.kills_total, b.kills_total));
    }
}

fn diff_projectile(i: usize, a: &Projectile, b: &Projectile, diffs: &mut Vec<String>) {
    let prefix = format!("proj {}", i);
    if a.alive != b.alive {
        diffs.push(format!("{} alive: {} vs {}", prefix, a.alive, b.alive));
    }
    diff_f32(&format!("{} pos.x", prefix), a.pos.x, b.pos.x, diffs);
    diff_f32(&format!("{} pos.y", prefix), a.pos.y, b.pos.y, diffs);
    diff_f32(&format!("{} vel.x", prefix), a.vel.x, b.vel.x, diffs);
    diff_f32(&format!("{} vel.y", prefix), a.vel.y, b.vel.y, diffs);
    diff_f32(&format!("{} damage", prefix), a.damage, b.damage, diffs);
}

fn diff_states(a: &BattleState, b: &BattleState) -> Vec<String> {
    let mut diffs = Vec::new();

    if a.units.len() != b.units.len() {
        diffs.push(format!("unit count: {} vs {}", a.units.len(), b.units.len()));
    }
    // Sort by ID for matching
    let mut a_units: Vec<&Unit> = a.units.iter().collect();
    let mut b_units: Vec<&Unit> = b.units.iter().collect();
    a_units.sort_by_key(|u| u.id);
    b_units.sort_by_key(|u| u.id);

    for (ua, ub) in a_units.iter().zip(b_units.iter()) {
        diff_unit(ua, ub, &mut diffs);
    }

    if a.projectiles.len() != b.projectiles.len() {
        diffs.push(format!(
            "projectile count: {} vs {}",
            a.projectiles.len(),
            b.projectiles.len()
        ));
    }
    for (i, (pa, pb)) in a.projectiles.iter().zip(b.projectiles.iter()).enumerate() {
        diff_projectile(i, pa, pb, &mut diffs);
    }

    // Player next_id (changes when Scavenge spawns chaff)
    for (pa, pb) in a.players.iter().zip(b.players.iter()) {
        if pa.next_id != pb.next_id {
            diffs.push(format!(
                "player {} next_id: {} vs {}",
                pa.player_id, pa.next_id, pb.next_id
            ));
        }
    }

    diffs
}

/// Run two identical states and find the first frame of divergence.
/// Returns None if they remain identical for `max_frames`.
fn find_divergence(
    mut a: BattleState,
    mut b: BattleState,
    max_frames: usize,
    nav_grid: &NavGrid,
) -> Option<(usize, Vec<String>)> {
    // Snapshot initial state
    let check_initial = diff_states(&a, &b);
    if !check_initial.is_empty() {
        return Some((0, check_initial));
    }

    // Run A fully with seeded RNG, capturing snapshots
    macroquad::rand::srand(SEED);
    let mut a_snapshots = Vec::with_capacity(max_frames);
    for _ in 0..max_frames {
        run_one_frame(&mut a, nav_grid);
        a_snapshots.push(a.clone());
    }

    // Re-seed and run B identically, comparing against A's snapshots
    macroquad::rand::srand(SEED);
    #[allow(clippy::needless_range_loop)]
    for frame in 0..max_frames {
        run_one_frame(&mut b, nav_grid);
        let diffs = diff_states(&a_snapshots[frame], &b);
        if !diffs.is_empty() {
            return Some((frame + 1, diffs));
        }
    }

    None
}

fn fresh_nav_grid(obstacles: &[Obstacle]) -> NavGrid {
    NavGrid::from_obstacles(obstacles, ARENA_W, ARENA_H, 15.0)
}

fn assert_deterministic(state: BattleState, frames: usize, label: &str) {
    let nav_grid = fresh_nav_grid(&state.obstacles);
    let a = state.clone();
    let b = state.clone();
    if let Some((frame, diffs)) = find_divergence(a, b, frames, &nav_grid) {
        let mut msg = format!(
            "DETERMINISM FAILURE in `{}` at frame {}:\n",
            label, frame
        );
        for d in diffs.iter().take(20) {
            msg.push_str(d);
            msg.push('\n');
        }
        if diffs.len() > 20 {
            msg.push_str(&format!("  ... and {} more diffs\n", diffs.len() - 20));
        }
        panic!("{}", msg);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn determinism_chaff_mirror() {
    // 1v1 Chaff melee brawl
    let mut state = BattleState::new();
    state.place_pack(UnitKind::Chaff, vec2(400.0, 480.0), 1);
    state.place_pack(UnitKind::Chaff, vec2(1280.0, 480.0), 2);
    assert_deterministic(state, 300, "chaff_mirror");
}

#[test]
fn determinism_skirmisher_swarm() {
    // Ranged swarm with potential separation push jitter
    let mut state = BattleState::new();
    state.place_pack(UnitKind::Skirmisher, vec2(400.0, 480.0), 1);
    state.place_pack(UnitKind::Skirmisher, vec2(1280.0, 480.0), 2);
    assert_deterministic(state, 300, "skirmisher_swarm");
}

#[test]
fn determinism_mixed_arms() {
    // Frontline + DPS on both sides
    let mut state = BattleState::new();
    state.place_pack(UnitKind::Sentinel, vec2(500.0, 480.0), 1);
    state.place_pack(UnitKind::Striker, vec2(400.0, 480.0), 1);
    state.place_pack(UnitKind::Dragoon, vec2(1200.0, 480.0), 2);
    state.place_pack(UnitKind::Ranger, vec2(1350.0, 480.0), 2);
    assert_deterministic(state, 300, "mixed_arms");
}

#[test]
fn determinism_full_armies() {
    // 3 packs per side, varied compositions
    let mut state = BattleState::new();
    // Player 1: tank + dps + swarm
    state.place_pack(UnitKind::Sentinel, vec2(500.0, 300.0), 1);
    state.place_pack(UnitKind::Striker, vec2(400.0, 500.0), 1);
    state.place_pack(UnitKind::Chaff, vec2(500.0, 700.0), 1);
    // Player 2: mixed ranged + melee
    state.place_pack(UnitKind::Bruiser, vec2(1200.0, 300.0), 2);
    state.place_pack(UnitKind::Ranger, vec2(1350.0, 500.0), 2);
    state.place_pack(UnitKind::Berserker, vec2(1200.0, 700.0), 2);
    assert_deterministic(state, 600, "full_armies");
}

#[test]
fn determinism_with_techs() {
    // Same mirror but with techs applied to one side
    let mut state = BattleState::new();
    state.place_pack(UnitKind::Striker, vec2(400.0, 480.0), 1);
    state.place_pack(UnitKind::Striker, vec2(1280.0, 480.0), 2);
    state.add_tech(1, UnitKind::Striker, TechId::RangeBoost);
    state.add_tech(1, UnitKind::Striker, TechId::HighCaliber);
    state.add_tech(1, UnitKind::Striker, TechId::HardenedFrame);
    assert_deterministic(state, 300, "with_techs");
}

#[test]
fn determinism_long_battle() {
    // Run for 10 simulated seconds
    let mut state = BattleState::new();
    state.place_pack(UnitKind::Dragoon, vec2(500.0, 480.0), 1);
    state.place_pack(UnitKind::Ranger, vec2(350.0, 480.0), 1);
    state.place_pack(UnitKind::Bruiser, vec2(1200.0, 480.0), 2);
    state.place_pack(UnitKind::Skirmisher, vec2(1400.0, 480.0), 2);
    assert_deterministic(state, 600, "long_battle");
}

#[test]
fn determinism_scavenge_spawning() {
    // Chaff with Scavenge killing enemies triggers mid-battle spawns
    let mut state = BattleState::new();
    state.place_pack(UnitKind::Chaff, vec2(800.0, 480.0), 1);
    state.place_pack(UnitKind::Skirmisher, vec2(900.0, 480.0), 2);
    state.add_tech(1, UnitKind::Chaff, TechId::ChaffScavenge);
    assert_deterministic(state, 300, "scavenge_spawning");
}

// ============================================================================
// Entrench behavior verification (not a determinism test — functional test)
// ============================================================================

#[test]
fn entrench_lone_unit_accumulates() {
    // A single Skirmisher with no targets should accumulate stationary_timer
    // (no separation push, no movement).
    let mut state = BattleState::new();
    state.place_pack(UnitKind::Skirmisher, vec2(200.0, 480.0), 1);

    let nav_grid = fresh_nav_grid(&state.obstacles);
    macroquad::rand::srand(SEED);

    // Run for 2 seconds (120 frames)
    for _ in 0..120 {
        run_one_frame(&mut state, &nav_grid);
    }

    // Every unit in the pack should have stationary_timer ~= 2.0 seconds
    let min_timer = state
        .units
        .iter()
        .filter(|u| u.kind == UnitKind::Skirmisher)
        .map(|u| u.stationary_timer)
        .fold(f32::INFINITY, f32::min);

    assert!(
        min_timer >= 1.5,
        "Lone Skirmishers should accumulate stationary_timer, but min was {}",
        min_timer
    );
}

#[test]
fn entrench_packed_units_accumulate() {
    // Packed Skirmishers with no enemies — separation push should settle
    // and they should eventually accumulate stationary_timer.
    // NOTE: This test exposes a known issue where separation jitter prevents
    // the stationary_timer from accumulating in packed formations.
    let mut state = BattleState::new();
    state.place_pack(UnitKind::Skirmisher, vec2(400.0, 480.0), 1);

    let nav_grid = fresh_nav_grid(&state.obstacles);
    macroquad::rand::srand(SEED);

    // Run for 3 seconds
    for _ in 0..180 {
        run_one_frame(&mut state, &nav_grid);
    }

    let max_timer = state
        .units
        .iter()
        .filter(|u| u.kind == UnitKind::Skirmisher)
        .map(|u| u.stationary_timer)
        .fold(0.0f32, f32::max);

    println!("[entrench_packed] max stationary_timer after 3s = {}", max_timer);
    // Assert that at least ONE unit in the pack accumulates some time
    assert!(
        max_timer >= 1.0,
        "Packed Skirmishers max stationary_timer = {} — separation push likely blocks accumulation",
        max_timer
    );
}

#[test]
fn entrench_stacked_during_combat() {
    // Skirmishers in combat (stationary, firing at enemies in range)
    // should accumulate stationary_timer when they stop moving to shoot.
    let mut state = BattleState::new();
    // Skirmishers placed just in range of the enemy Chaff
    state.place_pack(UnitKind::Skirmisher, vec2(500.0, 480.0), 1);
    state.place_pack(UnitKind::Chaff, vec2(700.0, 480.0), 2);

    let nav_grid = fresh_nav_grid(&state.obstacles);
    macroquad::rand::srand(SEED);

    // Run for 2 seconds
    for _ in 0..120 {
        run_one_frame(&mut state, &nav_grid);
    }

    let max_timer = state
        .units
        .iter()
        .filter(|u| u.kind == UnitKind::Skirmisher && u.alive)
        .map(|u| u.stationary_timer)
        .fold(0.0f32, f32::max);

    println!("[entrench_combat] max stationary_timer after 2s = {}", max_timer);
    assert!(
        max_timer >= 0.5,
        "Skirmishers in combat should accumulate some stationary_timer, but max was {}",
        max_timer
    );
}

// ============================================================================
// Multiplayer simulation harness
// ============================================================================
//
// Mirrors a PROPOSED sync protocol for battle_phase.rs (Phase 1 — harness only):
//   1. Hash EVERY frame (no sync interval)
//   2. Bidirectional hash exchange (both sides send their hash to the other)
//   3. Host is authoritative: when host detects a mismatch from guest's hash,
//      host proactively pushes its full state to guest (no request/response)
//   4. Guest applies state with FAST-FORWARD: rollback to snapshot frame,
//      then simulate forward to catch up to guest's current frame
//   5. Debounce: host won't re-send state for (2 * latency) frames after
//      the last send, preventing flood during in-flight mismatches
//
// This exercises the REAL compute_state_hash function. State copy uses Clone
// (skipping bincode round-trip, which is tested elsewhere).

#[derive(Debug, Clone)]
#[allow(dead_code)]  // fields are read via Debug formatting in the event log
enum SyncEvent {
    FrameAdvanced { frame: u32 },
    DriftInjected { frame: u32, description: String },
    HashDeliveredToHost { snapshot_frame: u32, host_hash: u64, guest_hash: u64, match_: bool },
    HashDeliveredToGuest { snapshot_frame: u32, host_hash: u64, guest_hash: Option<u64>, match_: bool },
    HostDetectedMismatch { snapshot_frame: u32 },
    StateSent { snapshot_frame: u32 },
    StateApplied { snapshot_frame: u32, applied_at_frame: u32, frames_replayed: u32 },
    StatesConverged { at_frame: u32 },
    StatesDiverged { at_frame: u32, diff_count: usize },
}

#[derive(Clone)]
struct PendingHash {
    snapshot_frame: u32,
    hash: u64,
    arrives_at: u32,
}

#[derive(Clone)]
struct PendingStateSync {
    snapshot_frame: u32,
    state: BattleState,
    arrives_at: u32,
}

struct MultiplayerSim {
    host: BattleState,
    guest: BattleState,
    host_frame: u32,
    guest_frame: u32,
    latency: u32,

    // Bidirectional hash exchange
    h_to_g_hashes: VecDeque<PendingHash>,
    g_to_h_hashes: VecDeque<PendingHash>,

    // Host-authoritative state push (host → guest only in Phase 1)
    h_to_g_snapshots: VecDeque<PendingStateSync>,

    // Recent local hashes for comparison against incoming peer hashes
    host_recent_hashes: VecDeque<(u32, u64)>,
    guest_recent_hashes: VecDeque<(u32, u64)>,

    // Debounce: don't resend state within this window after the last send
    host_last_sent_snapshot_frame: Option<u32>,

    events: Vec<SyncEvent>,
    was_converged_last_frame: bool,
}

impl MultiplayerSim {
    fn new(initial: BattleState, latency: u32) -> Self {
        let host = initial.clone();
        let guest = initial;
        Self {
            host,
            guest,
            host_frame: 0,
            guest_frame: 0,
            latency,
            h_to_g_hashes: VecDeque::new(),
            g_to_h_hashes: VecDeque::new(),
            h_to_g_snapshots: VecDeque::new(),
            host_recent_hashes: VecDeque::new(),
            guest_recent_hashes: VecDeque::new(),
            host_last_sent_snapshot_frame: None,
            events: Vec::new(),
            was_converged_last_frame: true,
        }
    }

    /// Advance both host and guest by one frame, then process all pending
    /// sync messages that have arrived this frame.
    fn step(&mut self, nav_grid: &NavGrid) {
        // 1. Both sides simulate one frame
        run_one_frame(&mut self.host, nav_grid);
        run_one_frame(&mut self.guest, nav_grid);
        self.host_frame += 1;
        self.guest_frame += 1;
        self.events.push(SyncEvent::FrameAdvanced { frame: self.host_frame });

        let now = self.host_frame;

        // 2. Compute hashes EVERY frame, store locally, send to peer
        let host_hash = compute_state_hash(&self.host.units, &self.host.projectiles, &self.host.obstacles);
        let guest_hash = compute_state_hash(&self.guest.units, &self.guest.projectiles, &self.guest.obstacles);

        self.host_recent_hashes.push_back((now, host_hash));
        self.guest_recent_hashes.push_back((now, guest_hash));
        // Keep enough history to cover round-trip latency
        let window = (3 * self.latency as usize).max(16);
        while self.host_recent_hashes.len() > window {
            self.host_recent_hashes.pop_front();
        }
        while self.guest_recent_hashes.len() > window {
            self.guest_recent_hashes.pop_front();
        }

        self.h_to_g_hashes.push_back(PendingHash {
            snapshot_frame: now,
            hash: host_hash,
            arrives_at: now + self.latency,
        });
        self.g_to_h_hashes.push_back(PendingHash {
            snapshot_frame: now,
            hash: guest_hash,
            arrives_at: now + self.latency,
        });

        // 3. Deliver host→guest hashes (informational for the guest — it can't
        // initiate recovery in Phase 1, but this lets us log detection latency
        // from the guest's perspective too).
        while let Some(front) = self.h_to_g_hashes.front() {
            if front.arrives_at > now { break; }
            let ph = self.h_to_g_hashes.pop_front().unwrap();
            let local = self.guest_recent_hashes
                .iter()
                .find(|(f, _)| *f == ph.snapshot_frame)
                .map(|(_, h)| *h);
            let match_ = local.is_some_and(|l| l == ph.hash);
            self.events.push(SyncEvent::HashDeliveredToGuest {
                snapshot_frame: ph.snapshot_frame,
                host_hash: ph.hash,
                guest_hash: local,
                match_,
            });
        }

        // 4. Deliver guest→host hashes. Host compares each incoming hash to
        // its own hash for the same frame. If any mismatch, schedule a state
        // send (subject to debounce).
        let mut host_should_send = false;
        let mut detected_frame: Option<u32> = None;
        while let Some(front) = self.g_to_h_hashes.front() {
            if front.arrives_at > now { break; }
            let ph = self.g_to_h_hashes.pop_front().unwrap();
            let local = self.host_recent_hashes
                .iter()
                .find(|(f, _)| *f == ph.snapshot_frame)
                .map(|(_, h)| *h);
            if let Some(local_hash) = local {
                let match_ = local_hash == ph.hash;
                self.events.push(SyncEvent::HashDeliveredToHost {
                    snapshot_frame: ph.snapshot_frame,
                    host_hash: local_hash,
                    guest_hash: ph.hash,
                    match_,
                });
                if !match_ {
                    host_should_send = true;
                    detected_frame = Some(ph.snapshot_frame);
                }
            }
        }

        // 5. Host proactively pushes state on mismatch (with debounce)
        if host_should_send {
            let debounce_ok = self
                .host_last_sent_snapshot_frame
                .is_none_or(|last| now.saturating_sub(last) >= 2 * self.latency);
            if debounce_ok {
                self.events.push(SyncEvent::HostDetectedMismatch {
                    snapshot_frame: detected_frame.unwrap_or(now),
                });
                self.host_last_sent_snapshot_frame = Some(now);
                self.h_to_g_snapshots.push_back(PendingStateSync {
                    snapshot_frame: now,
                    state: self.host.clone(),
                    arrives_at: now + self.latency,
                });
                self.events.push(SyncEvent::StateSent { snapshot_frame: now });
            }
        }

        // 6. Apply arrived snapshots to guest WITH FAST-FORWARD
        //    - Replace guest state with snapshot
        //    - Reset guest_frame to snapshot_frame
        //    - Replay forward to the guest's previous current frame
        //    - Guest's recent hash cache is cleared (it's stale now)
        while let Some(front) = self.h_to_g_snapshots.front() {
            if front.arrives_at > now { break; }
            let ps = self.h_to_g_snapshots.pop_front().unwrap();

            let snapshot_frame = ps.snapshot_frame;
            let target_frame = self.guest_frame;

            // Rollback
            self.guest = ps.state;
            self.guest_frame = snapshot_frame;

            // Replay forward to catch up to target_frame
            let mut frames_replayed = 0u32;
            while self.guest_frame < target_frame {
                run_one_frame(&mut self.guest, nav_grid);
                self.guest_frame += 1;
                frames_replayed += 1;
            }

            // Stale hashes: the ones we stored pre-rollback were computed on
            // the drifted state. Drop them so future comparisons use fresh
            // post-correction hashes.
            self.guest_recent_hashes.clear();

            self.events.push(SyncEvent::StateApplied {
                snapshot_frame,
                applied_at_frame: target_frame,
                frames_replayed,
            });
        }

        // 7. Track convergence transitions
        let converged = diff_states(&self.host, &self.guest).is_empty();
        if converged != self.was_converged_last_frame {
            if converged {
                self.events.push(SyncEvent::StatesConverged { at_frame: now });
            } else {
                let diff_count = diff_states(&self.host, &self.guest).len();
                self.events.push(SyncEvent::StatesDiverged { at_frame: now, diff_count });
            }
            self.was_converged_last_frame = converged;
        }
    }

    /// Mutate guest state to simulate a sudden drift. Logs an event.
    fn inject_drift_on_guest(&mut self, description: &str, mutator: impl FnOnce(&mut BattleState)) {
        mutator(&mut self.guest);
        self.events.push(SyncEvent::DriftInjected {
            frame: self.guest_frame,
            description: description.to_string(),
        });
    }

    fn states_match(&self) -> bool {
        diff_states(&self.host, &self.guest).is_empty()
    }

    fn event_count<F: Fn(&SyncEvent) -> bool>(&self, pred: F) -> usize {
        self.events.iter().filter(|e| pred(e)).count()
    }

    /// Pretty-print the event log (used on test failure with --nocapture).
    fn print_events(&self, limit: usize) {
        let non_frame = self.events.iter().filter(|e| !matches!(e, SyncEvent::FrameAdvanced { .. })).count();
        println!("--- Event log ({} total, {} non-frame-advance) ---", self.events.len(), non_frame);
        let mut shown = 0;
        for e in &self.events {
            if matches!(e, SyncEvent::FrameAdvanced { .. }) { continue; }
            println!("  {:?}", e);
            shown += 1;
            if shown >= limit {
                println!("  ... (truncated at {} events)", limit);
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Multiplayer tests
// ---------------------------------------------------------------------------

#[test]
fn mp_no_drift_stays_synced() {
    // Baseline: with no injected drift, host and guest never diverge and
    // no state syncs are ever triggered.
    let mut initial = BattleState::new();
    initial.place_pack(UnitKind::Striker, vec2(400.0, 480.0), 1);
    initial.place_pack(UnitKind::Bruiser, vec2(500.0, 480.0), 1);
    initial.place_pack(UnitKind::Dragoon, vec2(1200.0, 480.0), 2);
    initial.place_pack(UnitKind::Ranger, vec2(1350.0, 480.0), 2);

    let nav_grid = fresh_nav_grid(&initial.obstacles);
    macroquad::rand::srand(SEED);

    let mut sim = MultiplayerSim::new(initial, 4);
    for _ in 0..300 {
        sim.step(&nav_grid);
    }

    let host_mismatches = sim.event_count(|e| matches!(e, SyncEvent::HashDeliveredToHost { match_: false, .. }));
    let guest_mismatches = sim.event_count(|e| matches!(e, SyncEvent::HashDeliveredToGuest { match_: false, .. }));
    let sends = sim.event_count(|e| matches!(e, SyncEvent::StateSent { .. }));
    let applies = sim.event_count(|e| matches!(e, SyncEvent::StateApplied { .. }));
    let diverges = sim.event_count(|e| matches!(e, SyncEvent::StatesDiverged { .. }));

    println!("[mp_no_drift] host_mm={} guest_mm={} sends={} applies={} diverges={}",
        host_mismatches, guest_mismatches, sends, applies, diverges);

    if !sim.states_match() {
        sim.print_events(50);
        panic!("Host and guest diverged WITHOUT injected drift (non-determinism?)");
    }
    assert_eq!(host_mismatches, 0, "Host should see 0 mismatches without drift, got {}", host_mismatches);
    assert_eq!(guest_mismatches, 0, "Guest should see 0 mismatches without drift, got {}", guest_mismatches);
    assert_eq!(sends, 0, "Expected 0 state sends without drift, got {}", sends);
    assert_eq!(applies, 0, "Expected 0 state applies without drift, got {}", applies);
}

#[test]
fn mp_injected_drift_is_detected() {
    // Inject drift on guest at frame 20; verify host detects via guest's
    // incoming hash within `latency` frames.
    let mut initial = BattleState::new();
    initial.place_pack(UnitKind::Striker, vec2(400.0, 480.0), 1);
    initial.place_pack(UnitKind::Dragoon, vec2(1200.0, 480.0), 2);

    let nav_grid = fresh_nav_grid(&initial.obstacles);
    macroquad::rand::srand(SEED);

    let mut sim = MultiplayerSim::new(initial, 4);
    for _ in 0..20 {
        sim.step(&nav_grid);
    }
    sim.inject_drift_on_guest("shaved 50 HP off guest's first Striker", |s| {
        if let Some(u) = s.units.iter_mut().find(|u| u.kind == UnitKind::Striker) {
            u.hp -= 50.0;
        }
    });
    // Run 20 more frames — plenty of time for detection (need latency + 1 = 5)
    for _ in 0..20 {
        sim.step(&nav_grid);
    }

    let host_mismatches = sim.event_count(|e| matches!(e, SyncEvent::HashDeliveredToHost { match_: false, .. }));
    let detected = sim.event_count(|e| matches!(e, SyncEvent::HostDetectedMismatch { .. }));

    println!("[mp_detect] host_mismatches={} detected={}", host_mismatches, detected);

    assert!(host_mismatches > 0, "Host should have observed at least one mismatch");
    assert!(detected > 0, "Host should have declared at least one mismatch event");
}

#[test]
fn mp_sync_mechanism_recovery() {
    // Inject drift on guest, run long enough for the sync protocol to apply
    // recovery, and ASSERT convergence.
    let mut initial = BattleState::new();
    initial.place_pack(UnitKind::Striker, vec2(400.0, 480.0), 1);
    initial.place_pack(UnitKind::Sentinel, vec2(500.0, 480.0), 1);
    initial.place_pack(UnitKind::Ranger, vec2(1300.0, 480.0), 2);
    initial.place_pack(UnitKind::Chaff, vec2(1200.0, 480.0), 2);

    let nav_grid = fresh_nav_grid(&initial.obstacles);
    macroquad::rand::srand(SEED);

    let mut sim = MultiplayerSim::new(initial, 4);
    for _ in 0..30 {
        sim.step(&nav_grid);
    }
    sim.inject_drift_on_guest("shaved 100 HP off guest's first Striker", |s| {
        if let Some(u) = s.units.iter_mut().find(|u| u.kind == UnitKind::Striker) {
            u.hp -= 100.0;
        }
    });
    for _ in 0..200 {
        sim.step(&nav_grid);
    }

    let host_mismatches = sim.event_count(|e| matches!(e, SyncEvent::HashDeliveredToHost { match_: false, .. }));
    let sends = sim.event_count(|e| matches!(e, SyncEvent::StateSent { .. }));
    let applies = sim.event_count(|e| matches!(e, SyncEvent::StateApplied { .. }));
    let converges = sim.event_count(|e| matches!(e, SyncEvent::StatesConverged { .. }));
    let diverges = sim.event_count(|e| matches!(e, SyncEvent::StatesDiverged { .. }));

    println!("[mp_recovery] mismatches={} sends={} applies={} converges={} diverges={} final_match={}",
        host_mismatches, sends, applies, converges, diverges, sim.states_match());

    if !sim.states_match() {
        sim.print_events(60);
    }
    assert!(sends >= 1, "Host should send at least one state correction after drift");
    assert!(applies >= 1, "Guest should apply at least one state correction");
    assert!(sim.states_match(), "Host and guest must converge after recovery");
}

#[test]
fn mp_high_latency_stress() {
    // Same drift injection but with 20-frame (333ms) simulated network latency.
    // Must still converge.
    let mut initial = BattleState::new();
    initial.place_pack(UnitKind::Striker, vec2(400.0, 480.0), 1);
    initial.place_pack(UnitKind::Bruiser, vec2(1250.0, 480.0), 2);

    let nav_grid = fresh_nav_grid(&initial.obstacles);
    macroquad::rand::srand(SEED);

    let mut sim = MultiplayerSim::new(initial, 20);
    for _ in 0..30 {
        sim.step(&nav_grid);
    }
    sim.inject_drift_on_guest("removed 150 HP from a Striker", |s| {
        if let Some(u) = s.units.iter_mut().find(|u| u.kind == UnitKind::Striker) {
            u.hp -= 150.0;
        }
    });
    // Need plenty of time for round-trip at 20-frame latency
    for _ in 0..400 {
        sim.step(&nav_grid);
    }

    let host_mismatches = sim.event_count(|e| matches!(e, SyncEvent::HashDeliveredToHost { match_: false, .. }));
    let applies = sim.event_count(|e| matches!(e, SyncEvent::StateApplied { .. }));
    let converges = sim.event_count(|e| matches!(e, SyncEvent::StatesConverged { .. }));

    println!("[mp_high_latency] mismatches={} applies={} converges={} final_match={}",
        host_mismatches, applies, converges, sim.states_match());

    if !sim.states_match() {
        sim.print_events(80);
    }
    assert!(sim.states_match(), "High-latency recovery must converge");
}

/// Shared setup for the complex battle tests:
/// Player 1: Striker (3) + Ranger (3) + Bruiser (2) = 8 units, 3 types
///   Techs: Striker RangeBoost + HighCaliber, Ranger Entrench, Bruiser Cleave
/// Player 2: Dragoon (4) + Berserker (3) + Interceptor (3) = 10 units, 3 types
///   Techs: Dragoon ArmorBoost, Berserker Lifesteal, Interceptor DualWeapon
fn build_complex_battle() -> BattleState {
    let mut s = BattleState::new();
    // Player 1
    s.place_pack(UnitKind::Striker, vec2(450.0, 300.0), 1);
    s.place_pack(UnitKind::Ranger, vec2(350.0, 500.0), 1);
    s.place_pack(UnitKind::Bruiser, vec2(550.0, 650.0), 1);
    s.add_tech(1, UnitKind::Striker, TechId::RangeBoost);
    s.add_tech(1, UnitKind::Striker, TechId::HighCaliber);
    s.add_tech(1, UnitKind::Ranger, TechId::Entrench);
    s.add_tech(1, UnitKind::Bruiser, TechId::BruiserCleave);
    // Player 2
    s.place_pack(UnitKind::Dragoon, vec2(1200.0, 300.0), 2);
    s.place_pack(UnitKind::Berserker, vec2(1250.0, 500.0), 2);
    s.place_pack(UnitKind::Interceptor, vec2(1350.0, 650.0), 2);
    s.add_tech(2, UnitKind::Dragoon, TechId::ArmorBoost);
    s.add_tech(2, UnitKind::Berserker, TechId::BerserkerLifesteal);
    s.add_tech(2, UnitKind::Interceptor, TechId::InterceptorDualWeapon);
    s
}

#[test]
fn mp_complex_battle_early_drift() {
    // Drift at frame 20, before major combat kicks in.
    let initial = build_complex_battle();
    let nav_grid = fresh_nav_grid(&initial.obstacles);
    macroquad::rand::srand(SEED);

    let mut sim = MultiplayerSim::new(initial, 4);
    // Verify initial setup has 18 units total
    assert_eq!(sim.host.units.len(), 18, "Expected 18 total units, got {}", sim.host.units.len());

    for _ in 0..20 { sim.step(&nav_grid); }
    sim.inject_drift_on_guest("drifted a Dragoon HP by -50", |s| {
        if let Some(u) = s.units.iter_mut().find(|u| u.kind == UnitKind::Dragoon) {
            u.hp -= 50.0;
        }
    });
    for _ in 0..400 { sim.step(&nav_grid); }

    let applies = sim.event_count(|e| matches!(e, SyncEvent::StateApplied { .. }));
    let converges = sim.event_count(|e| matches!(e, SyncEvent::StatesConverged { .. }));
    let diverges = sim.event_count(|e| matches!(e, SyncEvent::StatesDiverged { .. }));
    println!("[mp_complex_early] applies={} converges={} diverges={} final_match={}",
        applies, converges, diverges, sim.states_match());

    if !sim.states_match() {
        sim.print_events(100);
    }
    assert!(applies >= 1, "Expected at least one state correction");
    assert!(sim.states_match(), "Complex battle with early drift must converge");
}

#[test]
fn mp_complex_battle_midcombat_drift() {
    // Let combat run until units are actually fighting and taking damage.
    // Then inject drift and verify recovery still works under active combat state.
    let initial = build_complex_battle();
    let nav_grid = fresh_nav_grid(&initial.obstacles);
    macroquad::rand::srand(SEED);

    let mut sim = MultiplayerSim::new(initial, 4);
    // Run long enough for units to close distance and engage.
    // At ~100 move speed, closing a ~700-unit gap takes ~7 seconds = 420 frames.
    for _ in 0..420 { sim.step(&nav_grid); }

    let alive_before = sim.host.units.iter().filter(|u| u.alive).count();
    let dead_before = sim.host.units.iter().filter(|u| !u.alive).count();
    let damaged_before = sim.host.units.iter().filter(|u| u.alive && u.hp < u.stats.max_hp).count();
    println!("[mp_complex_mid] before drift: {} alive, {} dead, {} damaged",
        alive_before, dead_before, damaged_before);

    sim.inject_drift_on_guest("mid-combat: removed 75 HP from a live Berserker", |s| {
        if let Some(u) = s.units.iter_mut().find(|u| u.kind == UnitKind::Berserker && u.alive) {
            u.hp -= 75.0;
        }
    });
    for _ in 0..400 { sim.step(&nav_grid); }

    let applies = sim.event_count(|e| matches!(e, SyncEvent::StateApplied { .. }));
    let alive_after = sim.host.units.iter().filter(|u| u.alive).count();
    println!("[mp_complex_mid] after drift+recovery: applies={} alive={} final_match={}",
        applies, alive_after, sim.states_match());

    if !sim.states_match() {
        sim.print_events(100);
    }
    // Sanity check that the "mid-combat" scenario is actually mid-combat
    assert!(damaged_before > 0, "mid-combat setup is broken: no damaged units at drift time");
    assert!(applies >= 1);
    assert!(sim.states_match(), "Mid-combat drift recovery must converge");
}

#[test]
fn mp_complex_battle_critical_unit_drift() {
    // Inject drift on a unit that will influence the battle's outcome significantly:
    // the Sentinel in a Sentinel-included build. Its damage/survival affects everything.
    let mut initial = BattleState::new();
    initial.place_pack(UnitKind::Sentinel, vec2(500.0, 480.0), 1);
    initial.place_pack(UnitKind::Striker, vec2(400.0, 300.0), 1);
    initial.place_pack(UnitKind::Ranger, vec2(350.0, 660.0), 1);
    initial.add_tech(1, UnitKind::Sentinel, TechId::ArmorBoost);
    initial.add_tech(1, UnitKind::Striker, TechId::HighCaliber);
    initial.place_pack(UnitKind::Berserker, vec2(1200.0, 480.0), 2);
    initial.place_pack(UnitKind::Skirmisher, vec2(1350.0, 300.0), 2);
    initial.add_tech(2, UnitKind::Berserker, TechId::BerserkerLifesteal);

    let nav_grid = fresh_nav_grid(&initial.obstacles);
    macroquad::rand::srand(SEED);

    let mut sim = MultiplayerSim::new(initial, 4);
    // Wait for combat to engage
    for _ in 0..80 { sim.step(&nav_grid); }

    // Sentinel has max armor + boost — drift it to test recovery on a heavy unit
    sim.inject_drift_on_guest("chunked Sentinel HP by -200", |s| {
        if let Some(u) = s.units.iter_mut().find(|u| u.kind == UnitKind::Sentinel && u.alive) {
            u.hp -= 200.0;
        }
    });
    for _ in 0..400 { sim.step(&nav_grid); }

    let applies = sim.event_count(|e| matches!(e, SyncEvent::StateApplied { .. }));
    let sentinel_alive_count = sim.host.units.iter()
        .filter(|u| u.kind == UnitKind::Sentinel && u.alive)
        .count();
    println!("[mp_complex_crit] applies={} sentinels_alive={} final_match={}",
        applies, sentinel_alive_count, sim.states_match());

    if !sim.states_match() {
        sim.print_events(100);
    }
    assert!(applies >= 1);
    assert!(sim.states_match(), "Critical unit drift recovery must converge");
}

#[test]
fn mp_multiple_drifts_recover() {
    // Inject drift multiple times throughout the battle. Each injection
    // should trigger a recovery; final state must be converged.
    let mut initial = BattleState::new();
    initial.place_pack(UnitKind::Chaff, vec2(500.0, 480.0), 1);
    initial.place_pack(UnitKind::Skirmisher, vec2(1250.0, 480.0), 2);

    let nav_grid = fresh_nav_grid(&initial.obstacles);
    macroquad::rand::srand(SEED);

    let mut sim = MultiplayerSim::new(initial, 4);
    for _ in 0..20 { sim.step(&nav_grid); }
    sim.inject_drift_on_guest("drift 1: HP -10 on chaff[0]", |s| {
        if let Some(u) = s.units.iter_mut().find(|u| u.kind == UnitKind::Chaff) { u.hp -= 10.0; }
    });
    for _ in 0..40 { sim.step(&nav_grid); }
    sim.inject_drift_on_guest("drift 2: move a chaff 5px", |s| {
        if let Some(u) = s.units.iter_mut().find(|u| u.kind == UnitKind::Chaff && u.alive) { u.pos.x += 5.0; }
    });
    for _ in 0..40 { sim.step(&nav_grid); }
    sim.inject_drift_on_guest("drift 3: HP -20 on skirmisher", |s| {
        if let Some(u) = s.units.iter_mut().find(|u| u.kind == UnitKind::Skirmisher && u.alive) { u.hp -= 20.0; }
    });
    for _ in 0..80 { sim.step(&nav_grid); }

    let applies = sim.event_count(|e| matches!(e, SyncEvent::StateApplied { .. }));
    println!("[mp_multiple_drifts] applies={} final_match={}", applies, sim.states_match());

    if !sim.states_match() {
        sim.print_events(80);
    }
    assert!(applies >= 3, "Expected at least 3 state corrections for 3 drift injections, got {}", applies);
    assert!(sim.states_match(), "Must converge after multiple drifts");
}
