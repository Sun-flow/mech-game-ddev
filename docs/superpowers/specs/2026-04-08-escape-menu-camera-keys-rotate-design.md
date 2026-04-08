# Escape Menu, Camera Keys, R-to-Rotate

**Date:** 2026-04-08
**Status:** Approved

## Goal

Add an in-match escape menu with resume/settings/surrender, WASD/arrow camera panning that respects camera rotation, and R key to rotate packs during build phase.

## Non-Goals

- Keybinding remapping UI (future)
- Multiplayer pause synchronization

## Part 1: Escape Menu

### State

Add `pub show_escape_menu: bool` to `GameContext`. Default `false`.

### Escape Key Behavior

During match phases (Build, WaitingForOpponent, Battle, RoundResult):
1. If chat is open → close chat (existing behavior, takes priority)
2. Else if escape menu is open → close escape menu
3. Else → open escape menu

NOT available during Lobby, DraftBan, or GameOver. Escape in those phases keeps existing behavior (back-navigation in lobby, close chat).

### Menu Rendering

Centered overlay with semi-transparent dark background (same style as existing surrender confirm overlay). Drawn in screen-space after all other UI.

Buttons (vertically stacked, centered):
- **Resume** — closes menu
- **Settings** — switches to settings sub-view
- **Surrender** — sets local player LP to 0, triggers GameOver. Visible during all match phases.

### Settings Sub-View

When "Settings" is clicked, the escape menu renders the existing settings panel (`settings::draw_settings_panel` for game settings + `settings::draw_ui_scale_slider` for UI scale). A "Back" button or Escape returns to the main escape menu.

### Pausing

- **Single-player:** Game pauses while escape menu is open. `dt` is not applied to combat simulation or build timer. Everything resumes exactly where it left off when menu closes.
- **Multiplayer:** Simulation continues (cannot pause the peer).

### Replaces Surrender Confirm

The existing `show_surrender_confirm` field in `BattleState` and its dedicated overlay in `battle_phase.rs` / `phase_ui.rs` are removed. Surrender now lives in the escape menu.

### Input Blocking

While the escape menu is open:
- All game input is blocked (clicks don't place/select packs, no drag, no shop interaction)
- Camera controls still work (zoom, pan, Q/E rotation) — the player can look around while paused
- Chat input is blocked

## Part 2: Camera Panning with WASD / Arrow Keys

### Controls

WASD and arrow keys pan the camera relative to the **screen**, accounting for camera rotation angle:

```
let pan_speed = 400.0 * dt;
let angle_rad = camera_angle.to_radians();
let screen_right = vec2(angle_rad.cos(), angle_rad.sin());
let screen_up = vec2(-angle_rad.sin(), angle_rad.cos());

// A/Left = screen-left, D/Right = screen-right
// W/Up = screen-up, S/Down = screen-down
```

### When Active

Available in all non-lobby phases (same scope as zoom/pan/Q/E). Disabled when:
- Chat is open (WASD would type into chat)
- Escape menu is open with settings sub-view (if text input is needed — currently settings has no text input, so this is a future concern)

### Coexistence with Existing Pan

Middle-click "grab the ground" pan continues to work alongside WASD. Both methods move `camera_target`. The existing camera clamp (140% of arena) applies to both.

## Part 3: R to Rotate Packs

### Controls

`KeyCode::R` rotates packs during build phase, same as middle-click rotation. Logic:
1. If a pack is being single-dragged → rotate it
2. Else if a pack is under the cursor → rotate it

Same rotation logic already exists for middle-click in `build_phase.rs`. R key is added as an alternative trigger alongside `middle_click`.

### When Active

Only during Build phase, only when not in escape menu, only when chat is not open.

## Files Affected

| File | Changes |
|------|---------|
| `context.rs` | Add `show_escape_menu: bool` field |
| `main.rs` | Escape key routing, WASD/arrow camera pan, render escape menu overlay, pause logic |
| `battle_phase.rs` | Remove `show_surrender_confirm` and its toggle, skip simulation when paused in single-player |
| `phase_ui.rs` | Remove `draw_battle_ui` surrender overlay, remove `show_surrender_confirm` parameter |
| `build_phase.rs` | Add R key rotation, skip input when escape menu open |

## Testing

Manual verification:
- Escape opens/closes menu during Build, Battle, WaitingForOpponent, RoundResult
- Escape does NOT open menu during Lobby, DraftBan, GameOver
- Resume button closes menu
- Settings button shows settings panel, Back/Escape returns to menu
- Surrender works from all match phases
- WASD/arrows pan camera relative to screen at any rotation angle
- Camera pan disabled when chat is open
- R rotates packs (dragged or under cursor)
- Single-player: simulation pauses when menu open, resumes correctly
- Multiplayer: simulation continues when menu open
