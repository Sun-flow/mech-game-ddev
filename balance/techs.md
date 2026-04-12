# Technology Balance Proposal

## Tech Economy Recap

- First tech per unit kind: 200g
- Each additional: +200g (2nd = 400g, 3rd = 600g, 4th = 800g)
- In practice, players buy 1-2 techs per unit kind per game
- Having 4-5 options per unit creates meaningful choices, not an expectation to buy all

---

## Existing Tech Changes

### Universal Techs

#### ArmorBoost — no change
| | Current | Proposed |
|-|---------|----------|
| Effect | +30 armor | +30 armor |
| Applies to | All 13 units | All 13 units |

#### RangeBoost — slight buff
| | Current | Proposed |
|-|---------|----------|
| Effect | +30 range | **+40 range** |
| Applies to | Striker, Ranger, Scout, Artillery, Sniper, Skirmisher, Dragoon, Interceptor | **Striker, Ranger, Scout, Skirmisher, Dragoon, Interceptor** |

**Rationale:** Remove from Artillery (450 range is already extreme) and Sniper (500 range doesn't need +25). Slight value reduction (+25 vs +30) keeps it meaningful without being auto-pick.

#### SplashBoost — no change
| | Current | Proposed |
|-|---------|----------|
| Effect | +15 splash radius | +15 splash radius |
| Applies to | Sentinel, Bruiser, Artillery, Berserker | Sentinel, Bruiser, Artillery, Berserker |

### Unit-Specific Tech Changes

#### ArtilleryBlastRadius — KEPT (no change)
Double-teching Artillery for splash (SplashBoost + BlastRadius = +40 splash radius, bringing base 45 to 85) is an expensive but powerful investment (200g + 400g = 600g on top of the 300g pack). Creates a distinct "mega-splash" build path for Artillery that rewards heavy commitment.

#### StrikerRapidFire — slight nerf
| | Current | Proposed |
|-|---------|----------|
| Effect | +0.5 attack speed | **+0.4 attack speed** |

**Rationale:** At +0.5, Striker reaches 1.9 attack speed × 220 damage = 418 DPS/unit (1,254 pack). At +0.4, it's 1.9 × 220 = still 418... actually let me recalc. 1.5 + 0.4 = 1.9. 220 × 1.9 = 418. Hmm. The difference between +0.5 (2.0 × 220 = 440) and +0.4 (1.9 × 220 = 418) is small but meaningful at pack level: 1,320 vs 1,254. A minor trim.

#### DragoonFortify — scaled down
| | Current | Proposed |
|-|---------|----------|
| Effect | +300 HP, +20 armor | **+250 HP, +15 armor** |

**Rationale:** With pack reduced to 4 units, +300 HP each = +1,200 pack HP. At +250, it's +1,000. Armor going from 35 → 50 is still very strong; 35 → 55 was too much.

#### ChaffOverwhelm — capped
| | Current | Proposed |
|-|---------|----------|
| Effect | +2 dmg per nearby chaff (uncapped) | **+3 dmg per nearby chaff (max 10 stacks)** |

**Rationale:** Uncapped scaling is dangerous — 18 tightly packed Chaff could theoretically hit +34 each. Capping at 10 stacks with +3 per stack = +30 max bonus damage (total 60 per hit). This is strong but bounded.

#### SkirmisherSwarm — REMOVED
**Rationale:** Redundant with the new Overdrive generic tech (+20% move speed). Skirmishers don't need a dedicated speed tech when a generic option exists. Overdrive is instead made available to Chaff.

#### BerserkerLifesteal — KEPT (no change)

Self-healing is acceptable (the unit heals itself through its own actions). The "no healing" principle applies to external healing sources (dedicated healers, aura heals, etc.), not self-sustain mechanics. Lifesteal synergizes with rage scaling — low HP = fast attacks = more lifesteal, creating an interesting tension between "dying makes me stronger" and "dying makes me heal more."

---

## New Generic Techs

### Hardened Frame — +20% max HP

| | |
|-|-|
| Effect | +20% max HP |
| Applies to | Chaff, Scout, Striker, Ranger, Interceptor, Sniper, Artillery |
| Type | Stat modifier |

**Rationale:** Defensive option for fragile units (those with ≤800 base HP). Competes with ArmorBoost as a survivability choice: ArmorBoost is better against many small hits, Hardened Frame is better against few large hits. Doesn't apply to swarm units (Chaff/Skirmisher — +20% of 90 HP is negligible) or already-tanky units.

| Unit | Base HP | With Hardened Frame |
|------|---------|---------------------|
| Scout | 350 | 420 |
| Striker | 600 | 720 |
| Ranger | 750 | 900 |
| Interceptor | 600 | 720 |
| Sniper | 500 | 600 |
| Artillery | 800 | 960 |

### Overdrive — +20% move speed

| | |
|-|-|
| Effect | +20% move speed |
| Applies to | Chaff, Sentinel, Bruiser, Dragoon, Artillery, Shield |
| Type | Stat modifier |

**Rationale:** Speed boost for units that benefit from closing distance faster or repositioning. Strategic choice: make your Sentinel tougher (ArmorBoost) or make it actually arrive? Chaff with Overdrive close melee range faster, reducing time under fire.

| Unit | Base Speed | With Overdrive |
|------|------------|----------------|
| Chaff | 150 | 180 |
| Sentinel | 65 | 78 |
| Bruiser | 90 | 108 |
| Dragoon | 85 | 102 |
| Artillery | 50 | 60 |
| Shield | 55 | 66 |

### High-Caliber Rounds — +15% damage

| | |
|-|-|
| Effect | +15% base damage |
| Applies to | Striker, Ranger, Dragoon, Interceptor, Berserker, Bruiser |
| Type | Stat modifier |

**Rationale:** Generic damage boost for combat-oriented T2 units. Creates a damage vs. survivability vs. range three-way tech choice. Doesn't apply to T1 (too cheap) or T3 (already extreme).

| Unit | Base Dmg | With High-Caliber |
|------|----------|-------------------|
| Striker | 220 | 253 |
| Ranger | 170 | 195 |
| Dragoon | 180 | 207 |
| Interceptor | 120 | 138 |
| Berserker | 200 | 230 |
| Bruiser | 160 | 184 |

---

## New Unit-Specific Techs

### Chaff: Frenzy
| | |
|-|-|
| Effect | +0.5 attack speed (1.5 → 2.0) |
| Type | Stat modifier |

**Rationale:** Simple DPS increase. Competes with Overwhelm — Frenzy is better when Chaff are spread out (each one hits faster), Overwhelm is better when packed tight (damage scales with density). Meaningful choice.

### Chaff: Expendable
| | |
|-|-|
| Effect | When a Chaff dies, all allied Chaff within 40 range gain +15% attack speed for 3 seconds (stacks up to 3 times, +45%) |
| Type | Behavioral |

**Rationale:** As Chaff die, survivors fight harder. Creates a snowball dynamic — once the swarm starts winning, it accelerates. But if they're dying too fast (to splash), the buff can't keep up. Rewards spreading Chaff to survive longer.

### Chaff: Scavenge
| | |
|-|-|
| Effect | When a Chaff lands a killing blow on an enemy unit, spawn new Chaff at the kill location. Spawn count based on victim's tier: T1 kill = 1 Chaff, T2 kill = 2 Chaff, T3 kill = 3 Chaff. |
| Type | Behavioral |

Spawned Chaff inherit all purchased Chaff techs, have full HP, and exist only for the current battle phase. They are not carried between rounds.

**Rationale:** Rewards Chaff for actually finishing off targets rather than just chip-damaging. The swarm grows by consuming the enemy. Against other T1 swarms (Chaff vs Chaff), each kill replaces a body — sustaining the fight. Against T2/T3, kills are harder to get (high HP, armor) but reward more bodies. Creates a snowball dynamic: if Chaff start winning, the swarm grows, but if the enemy has splash or armor, Chaff never get kills and the tech does nothing.

**Balance notes:**
- Chaff do 30 damage at 1.5 atk speed. Against a 90 HP Skirmisher, that's ~2 seconds per kill — manageable spawning rate.
- Against a 1,500 HP Bruiser with 25 armor, Chaff deal 5 effective damage per hit. It takes ~200 hits to kill one, and only the killing blow spawns 2 Chaff. This is self-limiting against heavy targets.
- Multiple Chaff attacking the same target: only the one that lands the killing blow triggers the spawn. No double-counting.
- The tech costs 200g+ (on top of 100g pack cost). At 300g+ total investment for a swarm that still melts to splash, this is a high-risk/high-reward pick.

### Shield: Reflective Barrier
| | |
|-|-|
| Effect | 15% of damage absorbed by the barrier is dealt back to the attacker |
| Type | Behavioral |

**Rationale:** Punishes enemies for shooting into the barrier. Creates a decision: power through and take reflected damage, or spend time flanking around? At 15%, a Striker dealing 220 damage into the barrier takes 33 back — noticeable but not lethal.

### Shield: Fortress Mode
| | |
|-|-|
| Effect | +1,500 barrier HP; Shield unit becomes immobile (move speed → 0) |
| Type | Stat modifier + behavioral |

**Rationale:** Massive defensive anchor that must be positioned perfectly in build phase. 2,500 + 1,500 = 4,000 barrier HP per Shield (8,000 total for 2 Shields). Incredibly strong if positioned well, useless if flanked. Creates a meaningful positioning puzzle during build phase.

### Entrench (Skirmisher, Ranger)
| | |
|-|-|
| Effect | While stationary, gain +12% attack speed every second, up to 4 stacks (+48%). Moving resets all stacks. |
| Applies to | Skirmisher, Ranger |
| Type | Behavioral |

**Rationale:** Rewards ranged units for holding position behind a frontline. Skirmishers entrenched for 4 seconds go from 2.0 to 2.96 attack speed (50 → 74 DPS per unit, 600 → 888 pack DPS). Rangers go from 0.75 to 1.11 attack speed (127.5 → 189 DPS per unit, 382 → 567 pack DPS). Creates tension: stay and stack damage, or reposition when the frontline breaks?

### Sentinel: Taunt Aura
| | |
|-|-|
| Effect | Enemy units within 120 range are forced to target the Sentinel (overrides closest-enemy targeting) |
| Type | Behavioral |

**Rationale:** Pure tank fantasy. Sentinel draws fire away from squishier allies, becoming even more of a wall. Synergizes with high HP/armor. Counter: ranged units outside 120 range ignore the taunt entirely, so it's not a universal "I win" button — it specifically protects nearby allies from melee threats and short-range attackers.

### Berserker: Unstoppable
| | |
|-|-|
| Effect | Below 50% HP: immune to slow effects, +20% move speed |
| Type | Behavioral |

**Rationale:** Leans into the berserker fantasy — as HP drops, the berserker becomes harder to kite or control. Synergizes with rage scaling (low HP = fast attacks + fast movement + slow immunity). At 130 base speed, Unstoppable brings a wounded Berserker to 156 speed — nearly as fast as Chaff (150), making them extremely hard to escape. Competes with Lifesteal for the "low HP payoff" tech slot: Lifesteal sustains, Unstoppable closes distance. Interesting choice depending on whether the enemy is kiting or standing and fighting.

### Berserker: Death Throes
| | |
|-|-|
| Effect | On death, explode for 150 damage in a 40-radius splash |
| Type | Behavioral |

**Rationale:** Punishes enemies for killing Berserkers in melee. Creates tension: focus Berserkers (take explosions) or ignore them (let them ramp up with rage). 150 damage at radius 40 is significant but not devastating — comparable to one Bruiser hit. The real value is in clustered fights where multiple Berserker deaths chain splash damage.

### Bruiser: Charge
| | |
|-|-|
| Effect | First attack after closing 100+ distance from spawn deals 2x damage (one-time burst) |
| Type | Behavioral |

**Rationale:** Rewards aggressive Bruiser positioning. Opening hit does 320 damage instead of 160 — a significant punch. Synergizes with Overdrive (faster closing = less time under fire before the charge lands). One-time only; after the first hit, damage returns to normal.

### Interceptor: Flak Burst
| | |
|-|-|
| Effect | Intercepted rockets detonate at the interception point, using the rocket's own damage and splash radius. Only damages enemies of the Interceptor (no friendly fire). |
| Type | Behavioral |

**Rationale:** Turns Interceptor from "I prevent damage" to "I redirect damage against the sender." Creates a nasty feedback loop: the more the opponent invests in Artillery splash tech, the more their own rockets punish them when intercepted.

**Scaling against opponent Artillery investment:**
| Opponent Artillery Build | Rocket Splash | Flak Burst Radius |
|--------------------------|---------------|-------------------|
| Base Artillery | 45 | 45 |
| + SplashBoost | 60 | 60 |
| + BlastRadius | 70 | 70 |
| + Both (mega-splash) | 85 | 85 |

Each flak burst deals the full rocket damage (450 base) in the full splash radius — effectively turning every intercepted rocket into a rocket launched *at the enemy* from wherever the Interceptor is standing. Forward-positioned Interceptors become a devastating counter to heavy Artillery investment: the opponent's own tech gold gets weaponized against them.

**Counterplay:** Opponent can avoid investing in Artillery splash, or position Artillery such that intercepted rockets don't land near their own units. Creates meaningful strategic tension: do you commit to splash tech knowing Interceptors will mirror that investment back at you?

### Sniper: Stabilizer
| | |
|-|-|
| Effect | Minimum attack range reduced from 150 to 75 |
| Type | Stat modifier |

**Rationale:** Addresses Sniper's core vulnerability (enemies inside min range). At 75 min range, Sniper can fire on enemies that are moderately close but not point-blank. Still vulnerable to fast melee (Chaff, Berserker) but less helpless when the frontline collapses. Competes with ArmorPierce (damage) and Hardened Frame (survivability) as a defensive tech choice.

---

## Per-Unit Tech Availability (After All Changes)

### T1

| Unit | Techs | Count |
|------|-------|-------|
| **Chaff** | ArmorBoost, ChaffOverwhelm, Frenzy, Expendable, Scavenge, Hardened Frame, Overdrive | **7** |
| **Skirmisher** | RangeBoost, ArmorBoost, Entrench | **3** |
| **Scout** | RangeBoost, ArmorBoost, ScoutEvasion, Hardened Frame | **4** |
| **Sniper** | ArmorBoost, SniperArmorPierce, Hardened Frame, Stabilizer | **4** |

### T2

| Unit | Techs | Count |
|------|-------|-------|
| **Striker** | RangeBoost, ArmorBoost, StrikerRapidFire, SniperArmorPierce, High-Caliber Rounds | **5** |
| **Bruiser** | ArmorBoost, SplashBoost, BruiserCleave, High-Caliber Rounds, Charge | **5** |
| **Sentinel** | ArmorBoost, SplashBoost, SentinelBarrier, Taunt Aura, Overdrive | **5** |
| **Ranger** | RangeBoost, ArmorBoost, RangerPierce, Entrench, High-Caliber Rounds | **5** |

| **Dragoon** | RangeBoost, ArmorBoost, DragoonFortify, Overdrive, High-Caliber Rounds | **5** |
| **Berserker** | ArmorBoost, SplashBoost, BerserkerLifesteal, Unstoppable, Death Throes, High-Caliber Rounds | **6** |
| **Interceptor** | RangeBoost, ArmorBoost, InterceptorDualWeapon, Flak Burst | **4** |

### T3

| Unit | Techs | Count |
|------|-------|-------|
| **Artillery** | ArmorBoost, SplashBoost, ArtilleryBlastRadius, ArtillerySlow, Overdrive, Hardened Frame | **6** |
| **Shield** | ArmorBoost, ShieldBarrierExpand, Overdrive, Reflective Barrier, Fortress Mode | **5** |

**Range:** 4-5 techs per unit. With escalating costs (200/400/600/800), players typically buy 1-2 per unit, making every choice meaningful.

---

## Complete Tech Registry

### Universal Techs (3)

| ID | Name | Effect | Applicable Units |
|----|------|--------|-----------------|
| RangeBoost | +Range | +40 attack range | Striker, Ranger, Scout, Skirmisher, Dragoon, Interceptor |
| ArmorBoost | +Armor | +30 armor | All 13 units |
| SplashBoost | +Splash | +15 splash radius | Sentinel, Bruiser, Artillery, Berserker |

### Generic Techs (3) — NEW

| ID | Name | Effect | Applicable Units |
|----|------|--------|-----------------|
| HardenedFrame | Hardened Frame | +20% max HP | Chaff, Scout, Striker, Ranger, Interceptor, Sniper, Artillery |
| Overdrive | Overdrive | +20% move speed | Chaff, Sentinel, Bruiser, Dragoon, Artillery, Shield |
| HighCaliber | High-Caliber | +15% damage | Striker, Ranger, Dragoon, Interceptor, Berserker, Bruiser |

### Unit-Specific Techs (17)

| ID | Name | Effect | Unit(s) |
|----|------|--------|---------|
| StrikerRapidFire | Rapid Fire | +0.4 attack speed | Striker |
| SentinelBarrier | Barrier | Shield r=40, 1500 barrier HP | Sentinel |
| RangerPierce | Pierce | Shots hit 2 targets | Ranger |
| ScoutEvasion | Evasion | 25% dodge chance | Scout |
| BruiserCleave | Cleave | Splash ignores armor | Bruiser |
| ArtilleryBlastRadius | Blast Radius | +25 splash radius | Artillery |
| ArtillerySlow | Slow Shells | Hits slow enemies 50% for 2s | Artillery |
| ChaffOverwhelm | Overwhelm | +3 dmg per nearby chaff (max 10) | Chaff |
| SniperArmorPierce | Armor Pierce | Shots ignore armor | Sniper, Striker |
| DragoonFortify | Fortify | +250 HP, +15 armor | Dragoon |
| ShieldBarrierExpand | Expand Barrier | +30 shield radius | Shield |
| InterceptorDualWeapon | Dual Weapon | Intercept + attack same frame | Interceptor |
| ChaffFrenzy | Frenzy | +0.5 attack speed | Chaff |
| ChaffExpendable | Expendable | On death: nearby chaff +15% atk speed 3s (max 3 stacks) | Chaff |
| ChaffScavenge | Scavenge | On kill: spawn Chaff (T1=1, T2=2, T3=3) at kill site | Chaff |
| ShieldReflect | Reflective Barrier | 15% barrier damage reflected to attacker | Shield |
| ShieldFortress | Fortress Mode | +1500 barrier HP, immobile | Shield |
| Entrench | Entrench | Stationary: +12% atk speed/sec, max 4 stacks. Move resets | Skirmisher, Ranger |
| SentinelTaunt | Taunt Aura | Enemies within 120 range forced to target Sentinel | Sentinel |
| BerserkerUnstoppable | Unstoppable | Below 50% HP: slow immune, +20% move speed | Berserker |
| BerserkerDeathThroes | Death Throes | On death: 150 dmg in 40-radius splash | Berserker |
| BruiserCharge | Charge | First attack after 100+ distance: 2x damage | Bruiser |
| InterceptorFlak | Flak Burst | Intercepted rockets detonate using rocket's own dmg + splash radius | Interceptor |
| SniperStabilizer | Stabilizer | Min attack range 150 → 75 | Sniper |

### Removed Techs

| ID | Reason |
|----|--------|
| SkirmisherSwarm | Redundant with Overdrive generic tech |

**Total tech count:** 3 universal + 3 generic + 18 unit-specific = **24 techs** (up from 17)
