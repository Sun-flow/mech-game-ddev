# PlayerState & Host/Guest Architecture

**Date:** 2026-04-05
**Status:** Approved
**Scope:** Replace player/opponent model with canonical host/guest state, introduce PlayerState struct, camera flip for guest perspective, simplify state sync.

## Motivation

The current architecture tracks per-player state as paired fields (`player_lp`/`opponent_lp`, `player_techs`/`opponent_techs`, etc.) scattered across `MatchProgress` and `GameContext`. Each client stores state from its own perspective, requiring coordinate mirroring and team_id swapping during network sync. This creates complexity in sync code and makes it harder to reason about shared state.

The refactor introduces a canonical host/guest state model where both clients store identical data. The guest client uses a camera flip for visual perspective rather than transforming state.

## Role Enum

```rust
pub enum Role {
    Host,
    Guest,
    Spectator,
}
```

Stored once on `GameContext` at game start. Single source of truth for "who am I." AI is always `Role::Guest`.

- `team_id=0` always means host on both machines.
- `team_id=1` always means guest on both machines.
- Spectator is reserved for future use.

## PlayerState Struct

```rust
pub struct PlayerState {
    pub lp: i32,
    pub techs: TechState,
    pub name: String,
    pub next_id: u64,
    pub gold: u32,
    pub packs: Vec<PlacedPack>,
    pub ai_memory: AiMemory,
}
```

- `gold` is a live balance. Mutated during build phase (purchases, sells, tech buys). Carried between rounds. Round allowance added at the start of each build phase.
- `packs` uses a unified `PlacedPack` type (see below). Ownership is structural — `progress.host.packs` vs `progress.guest.packs`.
- `ai_memory` defaults to empty for human players. Only meaningful for AI guest.

## MatchProgress

```rust
pub struct MatchProgress {
    pub round: u32,
    pub host: PlayerState,
    pub guest: PlayerState,
    pub banned_kinds: Vec<UnitKind>,
}
```

Replaces all paired `player_*`/`opponent_*` fields. Access pattern:

```rust
let me = match ctx.role {
    Role::Host => &progress.host,
    Role::Guest => &progress.guest,
    Role::Spectator => &progress.host, // spectators observe host perspective
};
```

`round_allowance()` moves to a standalone function or `MatchProgress` method since it depends on `round`, not on a specific player.

## Unified PlacedPack

Merges `PlacedPack` (game_state.rs) and `OpponentPlacedPack` (match_progress.rs) into one type:

```rust
pub struct PlacedPack {
    pub pack_index: usize,
    pub center: Vec2,
    pub unit_ids: Vec<u64>,
    pub rotated: bool,
    pub locked: bool,
    pub round_placed: u32,
}
```

- `locked` controls whether a pack can be sold/moved in the build phase. Packs from previous rounds and packs received from the network are `locked: true`.
- Pack identity does not include owner — ownership is determined by which `PlayerState` contains it.

## BuildState Slimdown

`BuildState` loses packs, gold, and next_id. It becomes the local build session UI state:

```rust
pub struct BuildState {
    pub timer: f32,
    pub selected_pack: Option<usize>,
    pub dragging: Option<usize>,
    pub multi_dragging: Vec<usize>,
    pub drag_box_start: Option<Vec2>,
    pub drag_offset: Vec2,
    pub multi_drag_offsets: Vec<Vec2>,
    pub round_tech_purchases: Vec<(UnitKind, TechId)>,
    pub undo_history: Vec<UndoEntry>,
    pub packs_bought_this_round: u32,
}
```

Methods like `purchase_pack()`, `sell_pack()`, `rotate_pack()` take `&mut PlayerState` as a parameter to mutate packs, gold, and next_id.

`ArmyBuilder` in economy.rs is removed — its only purpose was tracking `gold_remaining`, which is now `PlayerState.gold`.

## Camera Flip for Guest

The guest client sees the world mirrored horizontally via negative x-zoom:

```rust
let arena_camera = Camera2D {
    target: camera_target,
    zoom: vec2(
        camera_zoom * 2.0 / screen_width() * if ctx.role == Role::Guest { -1.0 } else { 1.0 },
        camera_zoom * 2.0 / screen_height(),
    ),
    ..Default::default()
};
```

Effects:
- Guest's units (canonical right side, `HALF_W..ARENA_W`) appear on the left of their screen.
- `screen_to_world` and `world_to_screen` automatically account for the flip — mouse input and pack placement work in canonical coordinates.
- Screen-space UI (HUD, shop, tech panel) is unaffected — drawn after `set_default_camera()`.
- All world-space text in this game is drawn in screen-space via `world_to_screen` conversion, so no mirrored text issues.

## Deploy Zone Parameterization

Hardcoded `0..HALF_W` clamps are replaced with role-derived bounds:

```rust
impl Role {
    pub fn deploy_x_range(&self) -> (f32, f32) {
        match self {
            Role::Host => (0.0, HALF_W),
            Role::Guest => (HALF_W, ARENA_W),
            Role::Spectator => (0.0, 0.0),
        }
    }
}
```

All `HALF_W` references in build_phase.rs and game_state.rs for pack placement clamping are replaced with `ctx.role.deploy_x_range()`.

The shop UI is drawn in screen-space and anchored to the left side of the screen. Since the guest's camera is flipped, the shop visually appears on the left (their side) even though it occupies the same screen coordinates.

## State Sync Simplification

With canonical state, all mirroring logic is removed:

- **Build sync:** Players send canonical coordinates. No mirroring on send or receive. Receiving client stores packs in the opponent's `PlayerState.packs` as-is.
- **State hash:** `compute_state_hash` drops the `mirror` parameter. Both clients hash identical canonical data.
- **`apply_state_sync`:** Drops the `mirror` parameter. Host sends canonical positions, guest applies directly. No coordinate flipping, no team_id swapping.
- **Round end:** Messages use host/guest LP rather than perspective-relative team IDs.

This removes ~40 lines of mirroring logic from sync.rs and eliminates the `mirror` parameter from battle_phase.rs call sites.

## AI Integration

AI is always `Role::Guest` (team_id=1). No changes to AI placement logic — it already spawns units in `HALF_W..ARENA_W`, which is the guest's canonical deploy zone.

- `ai_memory` lives on `progress.guest.ai_memory`.
- Tech purchases go to `progress.guest.techs`.
- Gold comes from `progress.guest.gold`.
- AI builder functions operate on `&mut PlayerState` directly instead of `ArmyBuilder.gold_remaining`.

## Units Vec

The mixed `Vec<Unit>` in `GameContext.units` is unchanged. Both teams' units share one Vec, filtered by `team_id`. This is intentional — combat targeting iterates all units and selects enemies by `team_id != self.team_id`. Splitting would add complexity with no benefit.

## Files Impacted

**Heavy changes (>10 references):**
- `match_progress.rs` — Core struct rewrite, all paired field access
- `build_phase.rs` — BuildState methods take `&mut PlayerState`, deploy zone bounds
- `game_state.rs` — PlacedPack unification, BuildState slimdown, deploy zone bounds
- `phase_ui.rs` — All UI drawing reads from PlayerState
- `battle_phase.rs` — LP damage writes to PlayerState, remove mirror from sync
- `main.rs` — Role on GameContext, camera flip, deploy zone

**Moderate changes (3-10 references):**
- `context.rs` — Add Role, remove mp_player_name/mp_opponent_name
- `economy.rs` — Remove ArmyBuilder, operate on PlayerState.gold
- `sync.rs` — Remove mirror parameter and mirroring logic
- `ui.rs` — Read LP/names from PlayerState
- `round_result.rs` — Gold carry-over via PlayerState.gold
- `rendering.rs` — Deploy zone overlay colors from role
- `net.rs` — Remove coordinate mirroring from build sync

**Light changes (<3 references):**
- `game_over.rs`, `waiting_phase.rs`, `lobby.rs`, `combat.rs`, `tech.rs`
