# Extract Phase Logic from main.rs — Design Spec

## Goal

Extract Build phase (~410 lines) and Battle phase (~280 lines) update logic from `main.rs` into dedicated modules, reducing main.rs from ~1,158 to ~500 lines. Also extract the smaller RoundResult, WaitingForOpponent, and GameOver update blocks.

## Architecture

Introduce a `GameContext` struct that bundles the shared mutable state accessed across multiple phases. Phase-specific state lives in structs owned by each phase module. `main()` becomes a thin coordinator: init state, poll input, dispatch to phase modules, render, next_frame.

## State Design

### `GameContext` (shared across 2+ phases)

Lives in a new `src/context.rs` module. Contains all mutable state that multiple phases need:

```rust
pub struct GameContext {
    pub progress: MatchProgress,
    pub phase: GamePhase,
    pub build: BuildState,
    pub units: Vec<Unit>,
    pub projectiles: Vec<Projectile>,
    pub net: Option<NetState>,
    pub obstacles: Vec<terrain::Obstacle>,
    pub nav_grid: Option<terrain::NavGrid>,
    pub splash_effects: Vec<SplashEffect>,
    pub game_settings: settings::GameSettings,
    pub show_grid: bool,
    pub mp_player_name: String,
    pub mp_opponent_name: String,
    pub chat: chat::ChatState,
}
```

### `BattleState` (battle-phase-only)

Lives in `src/battle_phase.rs`. Contains state only meaningful during Battle:

```rust
pub struct BattleState {
    pub accumulator: f32,
    pub timer: f32,
    pub frame: u32,
    pub recent_hashes: VecDeque<(u32, u64)>,
    pub show_surrender_confirm: bool,
    pub waiting_for_round_end: bool,
    pub round_end_timeout: f32,
}
```

Created once in `main()`, reset when entering Battle phase via a `reset()` method.

### Variables that stay in `main()`

- `lobby` (LobbyState) — only used in Lobby phase, and the Lobby phase already self-contains via `lobby.update()` / `lobby.draw()`
- `main_settings` (MainSettings) — only used for `ui_scale` once per frame in the loop preamble
- Camera state (`camera_zoom`, `camera_target`, `pan_grab_world`, `is_fullscreen_mode`) — used in the loop preamble, not in phase logic

### Constants

`ROUND_TIMEOUT` (90.0) and `SYNC_INTERVAL` (4) move to `battle_phase.rs` as module-level constants.

## Module Design

### `src/context.rs`

- `GameContext` struct definition
- `GameContext::new(is_host: bool) -> Self` constructor
- `GameContext::reset_for_lobby(&mut self)` — full reset for returning to lobby
- `GameContext::reset_for_rematch(&mut self, draft_ban_enabled: bool)` — reset for rematch (skip lobby)

### `src/build_phase.rs`

- `pub fn update(ctx: &mut GameContext, battle: &mut BattleState, screen_mouse: Vec2, mouse: Vec2, left_click: bool, right_click: bool, middle_click: bool, dt: f32)`
- Contains: network poll, grid toggle, undo, timer, shop interaction, tech panel, sell, rotate, drag systems, multi-drag, begin-round button
- Returns nothing — mutates `ctx.phase` directly when transitioning

### `src/battle_phase.rs`

- `BattleState` struct + `new()` + `reset()`
- `pub fn update(ctx: &mut GameContext, battle: &mut BattleState, screen_mouse: Vec2, dt: f32)`
- Contains: surrender toggle, simulation loop (SP & MP), sync hashing, desync detection, state sync, round end logic (LP damage, timeout, guest waiting)
- Constants: `ROUND_TIMEOUT`, `SYNC_INTERVAL`, `FIXED_DT` (moved from main.rs)

### `src/waiting_phase.rs`

- `pub fn update(ctx: &mut GameContext, battle: &mut BattleState)`
- Small (~50 lines): poll network, apply opponent build, generate terrain, seed RNG, transition to Battle

### `src/round_result.rs`

- `pub fn update(ctx: &mut GameContext)`
- ~65 lines: on Space press, save gold, advance round, lock packs, respawn units, restore stats, transition to Build or GameOver

### `src/game_over.rs`

- `pub fn update(ctx: &mut GameContext, lobby: &mut LobbyState, screen_mouse: Vec2, left_click: bool, draft_ban_enabled: bool)`
- ~40 lines: R to return to lobby, rematch button click, state reset

## main.rs After Extraction

```rust
// ~80 lines of mod declarations and imports

fn main() {
    let mut ctx = context::GameContext::new(true);
    let mut battle = battle_phase::BattleState::new();
    let mut lobby = lobby::LobbyState::new();
    let mut main_settings = settings::MainSettings::default();
    // camera state ...

    loop {
        let dt = get_frame_time().min(0.05);
        // input polling, camera, color setup (~40 lines)

        match ctx.phase.clone() {
            GamePhase::Lobby => { /* lobby.update/draw, set ctx fields on transition */ }
            GamePhase::DraftBan { .. } => { /* already extracted */ }
            GamePhase::Build => build_phase::update(&mut ctx, &mut battle, ...),
            GamePhase::WaitingForOpponent => waiting_phase::update(&mut ctx, &mut battle),
            GamePhase::Battle => battle_phase::update(&mut ctx, &mut battle, ...),
            GamePhase::RoundResult { .. } => round_result::update(&mut ctx),
            GamePhase::GameOver(_) => game_over::update(&mut ctx, &mut lobby, ...),
        }

        // rendering (~15 lines of calls)
        // chat (~4 lines)
        next_frame().await;
    }
}
```

Estimated: ~350-400 lines total for main.rs.

## Phase Execution Order

1. Create `context.rs` with `GameContext` struct
2. Refactor `main()` to use `GameContext` (no logic changes yet — just restructure locals into the struct)
3. Extract `build_phase.rs`
4. Extract `battle_phase.rs` (with `BattleState`)
5. Extract `waiting_phase.rs`
6. Extract `round_result.rs` and `game_over.rs`

Each phase compiles independently. `cargo check` after each.

## Design Decisions

**Why clone phase in match?** `GamePhase` is `Clone` and the match needs to borrow enum fields while also mutating `ctx`. Cloning the phase avoids borrow conflicts. The alternative (splitting phase out of GameContext) adds complexity for no benefit since phase is small.

**Why not split `phase` out of GameContext?** Phase transitions happen inside phase update functions (e.g., Build sets `ctx.phase = GamePhase::WaitingForOpponent`). Having phase in the context is the natural place — phases own their transitions.

**Why keep lobby in main?** Lobby is self-contained (already has `update()` / `draw()` methods) and is the only consumer of `main_settings`. Pulling it into GameContext would add fields that 90% of phases never touch.

**Why separate BattleState from GameContext?** The 7 battle-specific variables are always reset together and only read/written during Battle. Keeping them in a separate struct makes the lifecycle explicit and avoids polluting GameContext with battle-specific concerns.
