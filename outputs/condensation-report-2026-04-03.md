# Condensation Report

**Generated:** 2026-04-03
**Scope:** Full repository
**Files Analyzed:** 19
**Languages Detected:** Rust
**Total Lines:** ~6,954

## Summary

| Category | Count | Priority |
|----------|-------|----------|
| Monolithic file (main.rs) | 1 | Critical |
| Duplicate code patterns | 6 | High |
| Redundant patterns | 3 | Medium |
| Unused files/dead code | 0 | — |

## Critical: main.rs is 2,249 Lines

`main.rs` contains the entire game loop, all phase logic, all rendering, chat, and helper functions. It should be ~300-400 lines as a coordinator.

### Extraction Targets

| Code Block | Lines | Target Module | Priority |
|------------|-------|---------------|----------|
| Build phase logic | 352–764 (~410 lines) | `build_phase.rs` | High |
| Battle phase logic | 815–1097 (~280 lines) | `battle_phase.rs` | High |
| Phase-specific UI rendering | 1463–1835 (~370 lines) | `phase_ui.rs` | High |
| World-space rendering | 1208–1459 (~250 lines) | `rendering.rs` | Medium |
| DraftBan phase | 222–349 (~125 lines) | `draft_ban.rs` | Medium |
| Chat system | 1873–1972 (~100 lines) | `chat.rs` | Medium |
| `draw_hud()` | 2050–2127 | `ui.rs` | High |
| `draw_unit_shape()` | 2141–2188 | `unit.rs` | High |
| `respawn_player_units()` | 2191–2228 | `game_state.rs` | High |
| `refresh_units_of_kind()` | 2231–2248 | `tech.rs` | High |
| `send_build_complete()` | 1978–1999 | `net.rs` | Medium |
| `start_battle_ai()` | 2002–2048 | `economy.rs` | Medium |
| `draw_center_divider()` | 2129–2139 | `arena.rs` | Low |

### Minor Issue
- `mod settings; mod terrain;` declared at line 2249 instead of with other `mod` declarations at top of file.

## Duplicate Code

### 1. Pack Spawn Grid Layout (Critical — 4 locations)

**Files:** `pack.rs:28-68`, `pack.rs:71-112`, `game_state.rs:274-302`, `main.rs:2195-2230`
**~50+ duplicated lines** of grid calculation, unit positioning, and tech application.

**Recommendation:** Extract to a single `spawn_units_on_grid()` helper in `pack.rs`.

### 2. Damage Tracking Pattern (High — 3 locations)

**File:** `combat.rs` at lines 399-407, 536-544, 578-586

Identical 8-line block: save HP, check alive, apply damage (with armor-pierce branch), track kills.

**Recommendation:** Extract `fn apply_damage_tracked(unit, damage, armor_pierce) -> (f32, bool)` returning (damage_dealt, killed).

### 3. Ray-AABB Intersection (Medium — 2 locations)

**File:** `terrain.rs` at lines 119-136 and 174-189

14 lines of slab-method ray intersection copied verbatim.

**Recommendation:** Extract `fn ray_intersects_aabb(from, dir, min, max) -> bool`.

### 4. Scout Evasion Initialization (Medium — 3 locations)

**Files:** `pack.rs:56-60`, `pack.rs:100-104`, `main.rs:2214-2218`

Same 4-line evasion check block.

**Recommendation:** Fold into the grid spawn helper from item #1.

### 5. Distance-Based Selection with Tiebreaking (Medium — 2 locations)

**File:** `combat.rs` at lines 22-48 and 492-503

Deterministic closest-entity selection with ID-based tiebreaking.

**Recommendation:** Extract `fn find_closest_entity()` with tiebreaker.

### 6. Tech Application Boilerplate (Low — 5 locations)

**Files:** `pack.rs` (4×), `main.rs` (1×)

`techs.apply_to_stats()` + `unit.hp = unit.stats.max_hp` repeated.

**Recommendation:** Wrap in spawn helper from item #1.

## Redundant Patterns

### 1. Sync Struct Vec2 Conversions

**File:** `sync.rs` lines 68-167 — Three sync structs each manually convert `Vec2 ↔ (f32, f32)`.

**Recommendation:** Implement a conversion trait or macro.

### 2. Inconsistent Network State Access

**Files:** `combat.rs`, `lobby.rs`, `main.rs` — Different patterns for accessing opponent tech/team state (closures, direct field access, hardcoded team IDs).

**Recommendation:** Standardize via helper methods on `MatchProgress`.

### 3. AABB Overlap Variants

**File:** `game_state.rs` — Multiple slightly different bounding-box overlap checks.

**Recommendation:** Centralize in a `fn aabb_overlaps()` utility.

## Unused Files / Dead Code

**None found.** All modules, exports, functions, and imports are actively used. The codebase is clean in this regard.

## Proposed Actions

### 1. High Priority

- **Refactor main.rs** — Extract phase logic and rendering into dedicated modules. Target: reduce from 2,249 to ~300-400 lines.
- **Consolidate pack spawning** — Single `spawn_units_on_grid()` replacing 4 duplicate implementations.
- **Extract damage tracking** — `apply_damage_tracked()` helper in `combat.rs`.

### 2. Medium Priority

- **Extract ray-AABB intersection** — Single function in `terrain.rs`.
- **Standardize network state access** — Helper methods on `MatchProgress`.
- **Extract chat system** — New `chat.rs` module.

### 3. Low Priority

- **Sync struct conversions** — Trait or macro for Vec2 serialization.
- **AABB overlap utility** — Centralize bounding-box checks.
- **Move misplaced mod declarations** to top of `main.rs`.

## Metrics

- **Estimated lines saved:** ~200-300 (from deduplication alone)
- **Estimated lines moved:** ~1,800 (from main.rs extraction)
- **Files to create:** 4-6 new modules
- **Files to remove:** 0
- **New shared helpers:** 4-5 functions
