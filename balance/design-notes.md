# Design Notes & Validation

## Matchup Validation

Quick checks that the proposed numbers produce healthy tier interactions.

### T1 vs T1 (100g vs 100g)

**Chaff vs Skirmisher:**
- 18 Chaff (1,800 HP, melee 30 range, 810 DPS) vs 12 Skirmisher (840 HP, 160 range, 600 DPS)
- Skirmishers fire at 160 range. Chaff must close 130 distance at 150 speed = 0.87 sec.
- In 0.87 sec: Skirmishers deal 600 × 0.87 = 522 damage. Chaff at 1,278 HP (~5 dead).
- Then melee begins. 13 Chaff × 45 DPS = 585 DPS vs 12 Skirmishers.
- Skirmishers continue at 600 DPS. Kill remaining Chaff in 1,278/600 = 2.13 sec.
- Chaff kill: 585 DPS × 2.13 = 1,246 damage to Skirmishers (all dead at 840 HP).
- **Result:** Chaff win. Correct — melee swarm overwhelms fragile ranged bodies at close range. Skirmishers need a frontline to hide behind, not a head-to-head fight with Chaff.

**Scout vs Chaff:**
- 6 Scouts (2,100 HP, 100 range, 900 DPS, speed 190) vs 18 Chaff (1,800 HP, melee, speed 150)
- Scouts fire at 100 range. Chaff close 70 distance at 150 speed = 0.47 sec.
- Scouts deal 900 × 0.47 = 423 damage (4 Chaff dead). Then melee.
- 14 Chaff × 45 = 630 DPS vs Scouts. Scouts at 900 DPS.
- Scouts outgun and out-HP the remaining Chaff.
- **Result:** Scouts win. Correct — Scouts are the "premium" T1 option.

### T1 vs T2 (100g vs 200g)

**1x Scout (100g) vs 1x Striker (200g):**
- 6 Scouts (2,100 HP, 900 DPS, 100 range, speed 190) vs 3 Strikers (1,800 HP, 990 DPS, 200 range)
- Strikers fire first at 200 range. Scouts close 100 distance at 190 speed = 0.53 sec.
- Strikers deal 990 × 0.53 = 525 damage. Scouts at 1,575 HP (~1.5 dead).
- Then both fire. Strikers at 990 DPS, Scouts at ~900 DPS (reduced slightly by losses).
- Strikers kill Scouts: 1,575 / 990 = 1.6 sec. Scouts deal 900 × 1.6 = ~1,440 damage.
- **Result:** Strikers win narrowly (~360 HP remaining). Correct — T2 should beat equal-cost T1, but Scouts trade efficiently for half the price.

**2x Scout (200g) vs 1x Striker (200g):**
- 12 Scouts (4,200 HP, 1,800 DPS) vs 3 Strikers (1,800 HP, 990 DPS)
- After closing (0.53 sec, Strikers deal ~525): Scouts at 3,675 HP.
- Scouts overwhelm: 1,800 vs 990 DPS, with 2x the HP.
- **Result:** Scouts win decisively. Correct — spending 200g on T1 spam SHOULD beat 200g of T2 in raw stats, but T2 units offer utility/specialization that compensates in army compositions.

### T2 vs T2 (200g vs 200g)

**Striker vs Sentinel:**
- 3 Strikers (1,800 HP, 220 dmg, 200 range) vs 2 Sentinels (4,400 HP, 60 armor, 80 range)
- Effective Striker damage: 220 - 60 = 160 per hit. Pack DPS: 3 × 160 × 1.5 = 720 eff. DPS.
- Sentinels close 120 range at 65 speed = 1.85 sec. Take 720 × 1.85 = 1,332 damage.
- Sentinels at 3,068 HP. Melee begins. Sentinel DPS: 160. Striker HP: 1,800.
- Time to kill Strikers: 1,800 / 160 = 11.25 sec. Time to kill Sentinels: 3,068 / 720 = 4.26 sec.
- **Result:** Strikers win by a lot. Correct — DPS beats pure tank. Sentinel needs support.

**Dragoon vs Ranger:**
- 4 Dragoons (4,400 HP, 35 armor, 150 range, 396 DPS) vs 3 Rangers (2,250 HP, 10 armor, 300 range, 382 DPS)
- Rangers fire at 300 range. Dragoons close 150 distance at 85 speed = 1.76 sec.
- Ranger effective DPS vs 35 armor: 3 × (170-35) × 0.75 = 303.75. Dragoons take 535 in 1.76 sec (3,865 HP).
- Dragoons fire at 150 range. Eff DPS vs 10 armor: 4 × (180-10) × 0.55 = 374.
- Rangers: 2,250 / 374 = 6.0 sec to die. Dragoons: 3,865 / 303.75 = 12.7 sec.
- **Result:** Dragoons win. Correct — armor tanks ranged fire. Ranger needs support or Pierce tech. Reduced range (300 vs 350) gives Dragoons less time under fire before closing.

### T1 Sniper Interactions

**Sniper (100g) vs Sentinel (200g):**
- 1 Sniper (500 HP, 800 dmg, 500 range) vs 2 Sentinels (4,400 HP, 60 armor, 65 speed)
- Eff damage: 800 - 60 = 740 per hit. 0.25 atk speed.
- Sentinels close 420 distance at 65 speed = 6.46 sec.
- Sniper fires 6.46 × 0.25 = 1.6 shots. ~1,184 damage. Sentinels at 3,216 HP.
- Then Sentinels melee the Sniper. Dead instantly.
- **Result:** Sniper chunks but dies. At 100g vs 200g, good value trade — Sniper dealt ~27% of Sentinel pack HP for half the cost.

**3x Sniper (300g) + 1x Chaff (100g) = 400g vs 2x Sentinel (400g):**
- 3 Snipers + 18 Chaff vs 4 Sentinels (8,800 HP, 60 armor, 65 speed)
- Sentinels wade through Chaff (0 damage to Sentinel). Chaff absorbs Sentinel attacks (320 DPS).
- 18 Chaff = 1,800 HP of meat shield. Sentinels kill all Chaff in 1,800/320 = 5.6 sec.
- Meanwhile 3 Snipers fire: 5.6 sec × 0.25 atk/sec × 3 × 740 = 3,108 damage. Sentinels at 5,692 HP.
- After Chaff die, Snipers get 1-2 more volleys before Sentinels close remaining distance.
- Total Sniper damage: ~4,440. Sentinels at 4,360 HP with all 4 alive but damaged.
- **Result:** Sniper+Chaff trades efficiently but can't solo kill the Sentinels. Needs additional DPS. Correct — Snipers are a support pick, not a standalone army.

**2x Sniper (200g) vs 1x Striker (200g):**
- 2 Snipers (1,000 HP, 400 DPS, 500 range) vs 3 Strikers (1,800 HP, 990 DPS, 200 range)
- Snipers fire first at 500 range. Strikers close 300 range at 120 speed = 2.5 sec.
- Snipers fire 2.5 × 0.25 × 2 = 1.25 shots total... wait, each Sniper fires 0.625 shots in 2.5 sec. So ~1 shot lands total. 800 damage, one-shotting a Striker. 2 Strikers remain.
- At 200 range, Strikers fire back: 2 × 330 = 660 DPS. Kill both Snipers in 1,000/660 = 1.5 sec.
- Snipers get maybe 1 more shot in 1.5 sec (unlikely at 0.25 atk speed).
- **Result:** Strikers win, but lose 1 unit. Even trade at equal cost. Correct — Snipers trade 1-for-1 against glass cannons.

**Artillery (300g) vs 2x Chaff (200g):**
- 2 Artillery (1,600 HP, splash 45, 450 dmg, 0.4 atk) vs 36 Chaff (3,600 HP, melee)
- Each rocket hits ~6-8 tightly packed Chaff (splash radius 45 with size-5 units).
- Effective DPS against swarm: 2 × 450 × 0.4 × ~6 = ~2,160 AoE DPS.
- Chaff close 420 range at 150 speed = 2.8 sec. Artillery deals ~6,048 AoE dmg (overkill, but many Chaff dead).
- **Result:** Artillery dominates swarm. Correct — splash is the anti-swarm answer.

### Cross-Tier Composition Test

**Army A (600g): 1x Sentinel + 1x Striker + 1x Chaff**
- 2 Sentinels (frontline, 4,400 HP), 3 Strikers (DPS, 990 DPS), 18 Chaff (screen)
- Total: 23 units, ~7,600 HP across army + Chaff screen

**Army B (600g): 1x Dragoon + 1x Ranger + 2x Sniper**
- 4 Dragoons (frontline, 4,400 HP), 3 Rangers (DPS, 382 DPS), 2 Snipers (assassins, 400 DPS combined)
- Total: 9 units, ~7,650 HP across army

Analysis: Army A has more bodies (Chaff absorbs hits), Sentinels tank, Strikers deal damage. Army B has Dragoon line, Rangers provide sustained fire, 2 Snipers threaten Sentinels (740 eff. dmg per shot each). This should be a close, interesting fight — both comps have clear strengths. The Snipers chunk Sentinels hard (1,480 per volley) but Army A's Strikers eat through Dragoons fast (220-35=185 per hit × 1.5 × 3 = 832.5 DPS).

**Result:** Close match. Both armies have distinct plans that partially counter each other. Healthy. Note that Army B spends 200g on Snipers (2 × 100g) which is very gold-efficient for the alpha damage they provide.

---

## Economy Impact

### Sniper Tier Change

Moving Sniper from T3 to T1 means:
- **T1 (100g):** 4 units — Chaff, Skirmisher, Scout, Sniper
- **T2 (200g):** 7 units — Striker, Bruiser, Sentinel, Ranger, Dragoon, Berserker, Interceptor
- **T3 (300g):** 2 units — Artillery, Shield

T1 now has 4 distinct spam options: melee bodies (Chaff), ranged bodies (Skirmisher), fast flankers (Scout), and backline assassins (Sniper). Each plays a fundamentally different role despite all being 100g. T2 remains the main army-building tier with 7 specialists. T3 has 2 force multipliers.

Sniper at 100g is spammable — buying 2-3 Snipers behind a frontline is a valid strategy. At 800 damage per shot and 500 range, they threaten heavy targets efficiently. The counterplay is clear: rush them with fast melee (Scout, Berserker) or splash them with Artillery.

### Tech Economy

With 24 total techs and escalating costs (200/400/600/800), a player in a typical game with ~800g total might spend:
- ~200-400g on techs (25-50% of budget, matching AI's 33% target)
- 1 tech per unit kind at most, occasionally 2 for a core unit

This means tech CHOICE matters more than tech QUANTITY. Having 4-5 options per unit ensures real decisions even when you're only buying 1.

---

## Open Questions

### 1. Chaff Scavenge Spawning Mechanics
When a Chaff kills an enemy, new Chaff spawn at the kill location. Implementation needs:
- On kill event: check if killer is Chaff with Scavenge tech
- Determine victim's tier (T1=100g cost, T2=200g, T3=300g) to set spawn count (1/2/3)
- Spawn new Chaff units at victim's death position with unique IDs
- Spawned units get full Chaff stats + all purchased Chaff techs (including Scavenge itself — recursive spawns are possible but self-limiting since each new Chaff still needs to land a killing blow)
- Spawned Chaff do NOT persist between rounds — they exist only for the current battle phase
- For determinism: spawned unit IDs must be derived deterministically (e.g., from killer ID + frame number)

### 2. Chaff Overwhelm + Frenzy Interaction
Chaff with both Overwhelm (+3 per nearby, max 10 stacks = +30) and Frenzy (+0.5 atk speed = 2.0 total) reach 60 damage × 2.0 atk speed = 120 DPS per unit, 2,160 pack DPS. This requires buying two techs (200g + 400g = 600g investment on top of the 100g pack). At 700g total for a fully teched Chaff swarm, 2,160 DPS is strong but not unreasonable — a single Artillery splash would still devastate them. **Monitor in testing.**

### 3. Entrench Stacking Mechanics
Entrench is the first tech that tracks per-unit state over time (stack count + timer). Applies to both Skirmisher and Ranger. Implementation needs:
- Per-unit `stationary_timer: f32` tracking time since last position change
- Stacks computed from `stationary_timer` (not stored separately): stacks = min(floor(stationary_timer), 4)
- Effective attack speed = base × (1.0 + 0.12 × stacks)
- "Moved" = position changed by more than ~2.0 units since last frame (to avoid jitter resets)
- DPS display should reflect current stacks for clarity

**Visual representation:** Reuse the existing Berserker rage tint system (`rendering.rs:106-111`) as a template. Instead of red shift based on HP fraction, apply a **yellow glow** based on Entrench stack count:
- 0 stacks: no tint (base team color)
- 1-4 stacks: progressively stronger yellow tint (e.g., `color.r += stacks * 0.1`, `color.g += stacks * 0.1`, `color.b *= 1.0 - stacks * 0.15`)
- Alternative: color-code by team color (make the tint the team color at increasing intensity) so Entrench'd units remain team-identifiable at a glance
- This gives players visible feedback: "these Skirmishers are fully stacked — don't let them lose position"

### 4. Taunt Aura Target Override
Taunt Aura overrides the closest-enemy targeting within 120 range. Implementation needs:
- During target selection: if any enemy Sentinel with Taunt Aura is within 120 range, force-target that Sentinel
- Multiple taunting Sentinels: target the closest one
- Units outside 120 range: normal targeting (closest enemy)
- This is a combat.rs change in the targeting logic

### 5. Charge One-Time Burst
Bruiser Charge tracks distance from spawn. Implementation needs:
- Per-unit `distance_from_spawn: f32` tracking total movement
- On first attack: if distance_from_spawn >= 100.0 and !has_charged, deal 2x damage, set has_charged = true
- `has_charged: bool` per-unit flag

### 6. Death Throes Damage Attribution
When a Berserker with Death Throes dies, the 150 splash damage should be attributed to... the dead Berserker? The killer? This affects damage tracking stats. Propose attributing to the Berserker (posthumous damage).

### 7. Reflective Barrier Source
When a Shield barrier reflects 15% damage, the reflected damage should bypass armor (it's reflected energy, not a physical hit). This prevents the degenerate case where armor-stacked enemies take 0 reflected damage.

### 8. Fortress Mode + Deploy Zone
Shield with Fortress Mode becomes immobile. This means its position is locked at battle start. Players must position it during build phase with full awareness that it won't move. The move_speed = 0 should be applied at spawn time when techs are applied to stats.
