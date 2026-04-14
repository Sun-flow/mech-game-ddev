# Tech Panel UI Rework

## Summary

Reposition and reshape the tech panel from a tall-and-thin column (upper-right) to a wide-and-flat bar (lower-right, flush with screen edges). Single file change: `src/tech_ui.rs`.

## Layout

The panel anchors to the **bottom-right corner** of the screen, flush with both edges. It occupies roughly **44% of screen width** (was 210px fixed). It appears when a pack is selected, disappears when deselected — same trigger behavior as before.

### Structure

```
┌──────────────────────────────────────────────────────────────┐
│  Infantry                    Dmg:240  Soaked:85  Kills:3     │  <- Header: name left, combat stats right
├──────┬───────────────────────────────────────────────────────┤
│ HP  120 │ [Hardened Plates  100g] [Rapid Fire      100g] [Siege Rounds    100g] │
│ DMG  15 │ │+15 armor            │ │+0.3 AS              │ │+20 bldg dmg        │ │
│ RNG  80 ├───────────────────────┼──────────────────────────┼─────────────────────┤
│ AS  1.2 │ [Veteran Train.  100g] [✓ Forged Blades       ] [✓ Shield Wall       ] │
│ SPD  60 │ │+20 HP               │ │+10 damage            │ │+30 shield radius   │ │
│ ARM   5 │                                                                        │
└──────┴───────────────────────────────────────────────────────┘
```

### Sections

1. **Header row** — unit kind name (left-aligned), combat stats inline (right-aligned). Combat stats only shown when pack has engaged in combat.
2. **Stats sidebar** — vertical list of stat label/value pairs (HP, DMG, RNG, AS, SPD, ARM, plus conditional Splash/Shield). Separated from tech area by a vertical divider.
3. **Tech cards area** — all techs (available + purchased) in a single horizontal row. Wraps to additional rows when >3 techs. Each card shows:
   - Available: tech name (left) + cost in gold (right, same line), description below. Hover highlight, click to purchase.
   - Purchased: checkmark + tech name, description below. Green-tinted background.
   - Unaffordable: dimmed text colors.

### Sizing

All values go through the existing `s()` scaling function (reference 1400px width).

- Panel width: `s(616)` (44% of 1400)
- Panel position: bottom-right corner, flush (x = screen_width - panel_width, y = screen_height - panel_height)
- Panel height: dynamic based on number of tech rows
- Stats sidebar width: ~`s(77)`
- Tech cards: 3 per row, flex-wrapped
- Text sizes: header name ~14px, stat labels ~10px, tech names ~10px, descriptions ~9px (all pre-scale values)

### Interaction

- Left-click available tech card to purchase (unchanged)
- Click detection uses `point_in_rect` per tech card
- Panel area consumes mouse clicks to prevent pack dragging underneath (unchanged)
- `build_phase.rs` hit-test region updated to match new panel bounds

## Files Changed

- `src/tech_ui.rs` — full rewrite of `draw_tech_panel()` and position constants
- `src/build_phase.rs` — update panel hit-test bounds (click-consumption region)
