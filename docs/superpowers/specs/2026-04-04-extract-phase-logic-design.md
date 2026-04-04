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

`obstacles` and `nav_grid` live here because they persist across rounds (obstacles are generated once per match, cover HP reset each round; nav_grid is rebuilt from obstacles on battle entry).

### Phase-specific state structs

Each phase that needs persistent local state gets its own struct. These structs live in `main()` and are passed `&mut` to the active phase's update function. This keeps state local to its phase while allowing it to persist between frames.

#### `BattleState` (in `src/battle_phase.rs`)

```rust
pub struct BattleState {
    pub accumulator: f32,
    pub timer: f32,
    pub frame: u32,
    pub recent_hashes: VecDeque<(u32, u64)>,
    pub show_surrender_confirm: bool,
    pub waiting_for_round_end: bool,
    pub round_end_timeout: f32,
    pub projectiles: Vec<Projectile>,
}
```

`projectiles` are here because they're created during battle simulation and cleared on every phase transition — true battle-only lifecycle. Reset via `reset()` method when entering battle.

#### Other phases

Build, WaitingForOpponent, RoundResult, and GameOver don't need their own state structs — their persistent state already lives in `GameContext` (e.g., `build: BuildState` carries build-phase state, `progress: MatchProgress` carries round results). They take `&mut GameContext` and operate directly.

### Variables that stay in `main()`

- `lobby` (LobbyState) — only used in Lobby phase, already self-contained via `lobby.update()` / `lobby.draw()`
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

        // rendering (~15 lines — obstacles from ctx, projectiles from battle)
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

**Why separate BattleState from GameContext?** The 8 battle-specific variables (7 simulation locals + projectiles) follow the battle lifecycle: reset on entry, used during simulation, cleared on exit. Keeping them separate makes the lifecycle explicit. `obstacles` and `nav_grid` stay in GameContext because obstacles persist across rounds (generated once per match, cover HP reset each round).

**Why not per-phase structs for every phase?** Only Battle needs its own state struct. Build-phase state already lives in `BuildState` (in GameContext). RoundResult, WaitingForOpponent, and GameOver are stateless update functions that just read/write GameContext fields.
