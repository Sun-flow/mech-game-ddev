# Unit Balance Proposal

## Tier Philosophy

| Tier | Cost | Role | Design Intent |
|------|------|------|---------------|
| T1 | 100g | Spam / Overkill Absorbers | Many cheap bodies (or one glass cannon) that waste enemy alpha damage and pad your army. Expendable by design. |
| T2 | 200g | Generalists | Well-rounded, middle-strength units with 1-2 clear design goals. The backbone of any army — each one does its job reliably without being hyper-specialized. |
| T3 | 300g | Specialists | Expensive units with very specific goals and counters. High-impact when their niche is relevant, poor value when it isn't. Force multipliers that shape how battles play out. |
| T4 | TBD | Giants (future) | Single units with extremely clear purposes and high HP. Not yet implemented. |

### T2 Archetypes

Every T2 unit should be a solid, well-rounded pick that excels in 1-2 roles without being dead weight outside of them.

| Unit | Primary Role | Secondary Role | One-liner |
|------|-------------|----------------|-----------|
| **Striker** | Ranged DPS | Anti-armor (high per-hit damage) | Highest sustained ranged damage in the game. Fragile but lethal. |
| **Bruiser** | Chaff Clear | Off-tank | Melee splash that shreds swarms. Tanky enough to hold a line. |
| **Sentinel** | HP Tank | Frontline Anchor | Immovable wall. Absorbs punishment so the rest of the army doesn't. |
| **Ranger** | Sustained Ranged | Backline Safety | Long-range laser fire from maximum distance. Never needs to be in danger. |
| **Dragoon** | Armor Tank | Mid-range Line Holder | Many armored bodies that shoot while tanking. The durable frontline. |
| **Berserker** | Melee DPS | Lifesteal Tank | Escalating damage as HP drops. Self-sustains through aggression. |
| **Interceptor** | Anti-Projectile | Defensive Ranged | Neutralizes rockets. Moderate combat contribution when rockets aren't present. |

**Design rule:** A T2 unit should never be "dead weight" in any matchup. Even Interceptor — the most niche T2 — contributes 432 pack DPS at 250 range. It's just not *optimal* without rockets to intercept.

### T3 Specialist Roles

T3 units, by contrast, ARE allowed to be narrow. Their value comes from being the best at one specific thing.

| Unit | Specialism | Counter | When to buy |
|------|-----------|---------|-------------|
| **Artillery** | Anti-swarm / Area Denial | Fast melee rush, Interceptors | Enemy is clumping T1 spam or slow deathballs |
| **Shield** | Projectile Denial | Melee units that walk through barriers | Enemy relies on ranged DPS (Strikers, Rangers, Snipers) |

---

## T1 Units (100g)

### Chaff (18 units, 3x6 pack)

Melee swarm. Maximum overkill absorption. Useless against armor, melts to splash.

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| HP | 120 | **100** | -20 |
| Damage | 30 | 30 | -- |
| Attack Speed | 1.5 | 1.5 | -- |
| Attack Range | 30 | 30 | -- |
| Move Speed | 150 | 150 | -- |
| Armor | 0 | 0 | -- |
| Size | 5 | 5 | -- |
| Splash Radius | 0 | 0 | -- |

| Metric | Current | Proposed |
|--------|---------|----------|
| Pack HP | 2,160 | **1,800** |
| DPS/unit | 45 | 45 |
| Pack DPS | 810 | 810 |

**Rationale:** Slight HP reduction keeps Chaff disposable. 100 HP means a Sniper shot (800 dmg) wastes 700 damage — excellent overkill absorption. Skirmishers (25 dmg × 2.0 speed = 50 DPS) kill a Chaff in 2 seconds — efficient, as intended. Skirmishers at 70 HP die to 2 Chaff hits (30 dmg × 2 = 60... survives, 3rd hit kills). Both are disposable.

---

### Skirmisher (12 units, 3x4 pack)

Ranged screen. Sits behind frontline Dragoons/Sentinels as a second wall of bodies with modest chip damage. Not a DPS threat — a ranged meat shield.

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| HP | 70 | 70 | -- |
| Damage | 25 | 25 | -- |
| Attack Speed | 2.5 | **2.0** | -0.5 |
| Attack Range | 180 | **160** | -20 |
| Projectile Speed | 350 | 350 | -- |
| Move Speed | 160 | **150** | -10 |
| Armor | 0 | 0 | -- |
| Size | 5 | 5 | -- |
| Pack Layout | 2x6 | **3x4** | formation |

| Metric | Current | Proposed |
|--------|---------|----------|
| Pack HP | 840 | 840 |
| DPS/unit | 62.5 | **50** |
| Pack DPS | 750 | **600** |

**Rationale:** Skirmishers are disposable ranged bodies — they're supposed to be fragile. DPS trimmed (600 vs 750) because their job is to be a wall of bodies, not damage dealers. Range reduced to 160 so they don't outrange T2 units; they need to sit closer to the frontline to contribute, making positioning matter more.

---

### Scout (6 units, 2x3 pack)

Fast flanker. Mobile, short-range harasser. Not a frontline unit.

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| HP | 500 | **350** | -150 |
| Damage | 100 | 100 | -- |
| Attack Speed | 2.0 | **1.5** | -0.5 |
| Attack Range | 120 | **100** | -20 |
| Projectile Speed | 300 | 300 | -- |
| Move Speed | 180 | **190** | +10 |
| Armor | 0 | 0 | -- |
| Size | 10 | **8** | -2 |
| Pack Layout | 2x3 | 2x3 | -- |

| Metric | Current | Proposed |
|--------|---------|----------|
| Pack HP | 3,000 | **2,100** |
| DPS/unit | 200 | **150** |
| Pack DPS | 1,200 | **900** |

**Rationale:** Current Scouts beat most T2 packs head-to-head for half the cost. HP and attack speed nerfs bring total pack power down significantly (2,100 HP / 900 DPS, down from 3,000 / 1,200) while keeping 100 damage per hit so each shot still stings. Speed fantasy preserved (190, fastest in game). Scouts hit hard per-shot but fire slower and die faster.

---

### Sniper (1 unit, 1x1 pack) — MOVED FROM T3 TO T1

Anti-armor assassin. Extreme range, extreme per-hit alpha, extreme fragility. Spammable — buy several behind a frontline to threaten heavy targets.

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| **Cost** | **300** | **100** | **-200** |
| HP | 400 | **500** | +100 |
| Damage | 1,200 | **800** | -400 |
| Attack Speed | 0.25 | 0.25 | -- |
| Attack Range | 500 | 500 | -- |
| Projectile Speed | 1,100 | 1,100 | -- |
| Projectile Type | Laser | Laser | -- |
| Move Speed | 40 | **45** | +5 |
| Armor | 0 | 0 | -- |
| Min Attack Range | 150 | 150 | -- |
| Size | 10 | 10 | -- |
| Pack | 1x1 | 1x1 | -- |

| Metric | Current (300g) | Proposed (100g) |
|--------|----------------|-----------------|
| Pack HP | 400 | **500** |
| DPS/unit | 300 | **200** |
| Pack DPS | 300 | **200** |

**Rationale:** A single 400 HP unit for 300g was the worst cost-to-value ratio in the game. At 100g, Sniper becomes a spammable specialist: cheap enough to buy 2-3 behind your frontline. Each one threatens heavy targets from 500 range with 800-damage alpha strikes. The risk/reward is clear — incredible output if protected, instant loss if rushed. 1 unit per pack preserves the "anti-armor assassin" identity.

At 100g, Sniper is by far the weakest T1 in raw pack stats (500 HP, 200 DPS vs Chaff's 1,800 HP / 810 DPS). The value is entirely in the per-hit alpha and extreme range — a fundamentally different tool than the other T1 spam units.

**Key interactions at 800 damage:**
- vs Sentinel (2,200 HP, 60 armor): 740 eff. dmg, kills in 3 shots (12 sec)
- vs Dragoon (1,100 HP, 35 armor): 765 eff. dmg, kills in 2 shots (8 sec)
- vs Berserker (1,000 HP, 15 armor): 785 eff. dmg, survives with 215 HP (triggers rage at ~22% HP)
- vs Bruiser (1,500 HP, 25 armor): 775 eff. dmg, kills in 2 shots (8 sec)
- vs Striker (600 HP, 0 armor): one-shot
- vs Chaff (100 HP): one-shot, wastes 700 damage (overkill absorber working as intended)
- vs other Sniper (500 HP): one-shot

**Spam scenario — 3x Sniper (300g) + 1x Chaff (100g) = 400g:**
- 3 Snipers behind 18 Chaff. Each volley = 3 × 800 = 2,400 damage from 500 range.
- Chaff absorbs incoming fire while Snipers delete targets one by one.
- Countered by: fast melee (Scouts, Berserkers) that bypass the Chaff screen, or splash that kills Snipers through Chaff.

---

## T2 Units (200g)

### Striker (3 units, 1x3 pack) — Ranged DPS / Anti-Armor

The army's primary damage engine. Highest sustained ranged DPS at T2. No armor, no splash — pure single-target output. Also the best non-Sniper answer to armor: 220 per hit cuts through even Sentinel's 60 armor for 160 effective damage.

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| HP | 600 | 600 | -- |
| Damage | 250 | **220** | -30 |
| Attack Speed | 1.5 | 1.5 | -- |
| Attack Range | 200 | 200 | -- |
| Projectile Speed | 400 | 400 | -- |
| Move Speed | 120 | 120 | -- |
| Armor | 0 | 0 | -- |

| Metric | Current | Proposed |
|--------|---------|----------|
| Pack HP | 1,800 | 1,800 |
| DPS/unit | 375 | **330** |
| Pack DPS | 1,125 | **990** |

**Rationale:** Slight damage trim. 990 pack DPS is still the highest at T2, preserving the glass cannon identity. The damage-per-hit reduction (250 → 220) also means Strikers are slightly worse at punching through armor, creating more room for Sniper/ArmorPierce as a complement.

---

### Bruiser (2 units, 1x2 pack) — Chaff Clear / Off-Tank

Melee splash that shreds swarms. Splash radius 25 hits multiple T1 bodies per swing. Tanky enough (1,500 HP, 25 armor) to hold a line while clearing, but not a dedicated wall like Sentinel or Dragoon.

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| HP | 1,700 | **1,500** | -200 |
| Damage | 150 | **160** | +10 |
| Attack Speed | 1.0 | 1.0 | -- |
| Attack Range | 100 | 100 | -- |
| Move Speed | 90 | 90 | -- |
| Armor | 20 | **25** | +5 |
| Splash Radius | 25 | 25 | -- |

| Metric | Current | Proposed |
|--------|---------|----------|
| Pack HP | 3,400 | **3,000** |
| DPS/unit | 150 | **160** |
| Pack DPS | 300 | **320** |

**Rationale:** Slight HP/armor rebalance. Less raw HP but more armor per unit creates a cleaner identity split from Sentinel (wall) and Dragoon (ranged tank). Damage bump to 160 differentiates from Sentinel (100 dmg) — Bruiser hits harder, Sentinel tanks harder.

---

### Sentinel (2 units, 1x2 pack) — HP Tank / Frontline Anchor

The immovable wall. Highest raw HP (4,400 pack) and highest armor (60) in the game. Minimal damage output — exists purely to absorb punishment so everything behind it stays alive. The unit your army forms around.

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| HP | 2,000 | **2,200** | +200 |
| Damage | 80 | **100** | +20 |
| Attack Speed | 0.8 | 0.8 | -- |
| Attack Range | 80 | 80 | -- |
| Move Speed | 60 | **65** | +5 |
| Armor | 80 | **60** | -20 |
| Splash Radius | 15 | **20** | +5 |
| Size | 20 | 20 | -- |

| Metric | Current | Proposed |
|--------|---------|----------|
| Pack HP | 4,000 | **4,400** |
| DPS/unit | 64 | **80** |
| Pack DPS | 128 | **160** |

**Rationale:** 80 armor creates a binary dynamic: many units deal literally 0 damage (Chaff 30, Skirmisher 25, Shield 60). At 60 armor, Sentinel is still nearly invulnerable to T1 but T2 units chip through faster. HP increase compensates — total effective HP against mid-damage threats is similar. Splash radius bump (15 → 20) helps Sentinel clear Chaff swarms.

**Damage against Sentinel (current → proposed):**

| Attacker (dmg) | vs 80 armor | vs 60 armor |
|----------------|-------------|-------------|
| Chaff (30) | 0 | 0 |
| Skirmisher (25) | 0 | 0 |
| Scout (100) | 20 | 40 |
| Interceptor (120) | 40 | 60 |
| Bruiser (160) | 80 | 100 |
| Ranger (170) | 100 | 110 |
| Dragoon (180) | 120 | 120 |
| Berserker (200) | 140 | 140 |
| Striker (220) | 170 | 160 |
| Artillery (450) | 420 | 390 |
| Sniper (1,000) | 920 | 940 |

Key change: Scout goes from 0 to 20 effective damage. Interceptor goes from 40 to 60. Everything else stays close.

---

### Ranger (3 units, 1x3 pack) — Sustained Ranged / Backline Safety

Long-range laser fire from maximum distance. 350 range means Rangers never need to be in danger — they fire over the frontline into whatever the tanks are holding. Moderate DPS that adds up over long fights, especially with Entrench stacking.

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| HP | 700 | **750** | +50 |
| Damage | 180 | **170** | -10 |
| Attack Speed | 0.7 | **0.75** | +0.05 |
| Attack Range | 350 | **300** | -50 |
| Projectile Speed | 500 | 500 | -- |
| Projectile Type | Laser | Laser | -- |
| Move Speed | 80 | 80 | -- |
| Armor | 10 | 10 | -- |

| Metric | Current | Proposed |
|--------|---------|----------|
| Pack HP | 2,100 | **2,250** |
| DPS/unit | 126 | **127.5** |
| Pack DPS | 378 | **382.5** |

**Rationale:** Ranger is close to balanced. Tiny durability bump, net DPS roughly flat. 350 range justifies moderate DPS. The real power comes from positioning behind tanks.

---

### Dragoon (4 units, 1x4 pack) — Armor Tank / Mid-Range Line Holder

The durable frontline that shoots back. Four armored bodies (35 armor each) that soak hits while dealing 150-range return fire. Where Sentinel is a pure wall, Dragoon is a wall that contributes meaningful damage. The unit Skirmishers and Rangers hide behind.

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| **Pack** | **1x5 (5)** | **1x4 (4)** | **-1 unit** |
| HP | 1,000 | **1,100** | +100 |
| Damage | 200 | **180** | -20 |
| Attack Speed | 0.5 | **0.55** | +0.05 |
| Attack Range | 150 | 150 | -- |
| Projectile Speed | 350 | 350 | -- |
| Move Speed | 85 | 85 | -- |
| Armor | 40 | **35** | -5 |
| Splash Radius | 4 | **0** | removed |

| Metric | Current | Proposed |
|--------|---------|----------|
| Pack HP | 5,000 | **4,400** |
| DPS/unit | 100 | **99** |
| Pack DPS | 500 | **396** |

**Rationale:** 5 units at 40 armor / 1,000 HP was the tankiest pack in the game, outclassing Sentinel. Dropping to 4 units and reducing armor to 35 brings total pack durability in line with other T2 tanks. Splash removal (was 4, negligible) cleans up the identity: Dragoon is a mid-range fighter, not a splasher. Per-unit stats are slightly buffed (+100 HP) to compensate for losing a body.

---

### Berserker (3 units, 1x3 pack) — Melee DPS / Lifesteal Tank

Escalating melee threat that gets more dangerous as it takes damage. Rage scaling (up to 2.5x attack speed at low HP) makes Berserkers the highest potential DPS unit in the game. Lifesteal tech lets them sustain through aggression — the lower their HP, the faster they attack, the more they heal. High-risk, high-reward frontline aggressor.

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| HP | 900 | **1,000** | +100 |
| Damage | 220 | **200** | -20 |
| Attack Speed | 1.0 (→3.0x) | **1.0 (→2.5x)** | cap reduced |
| Attack Range | 60 | 60 | -- |
| Move Speed | 130 | 130 | -- |
| Armor | 20 | **15** | -5 |
| Splash Radius | 10 | **12** | +2 |

| Metric | Current | Proposed |
|--------|---------|----------|
| Pack HP | 2,700 | **3,000** |
| DPS/unit (base) | 220 | **200** |
| DPS/unit (max) | 660 | **500** |
| Pack DPS (base) | 660 | **600** |
| Pack DPS (max) | 1,980 | **1,500** |

**Rage formula change:** `1.0 + 2.0 * (1.0 - hp_frac)` → `1.0 + 1.5 * (1.0 - hp_frac)`

Peak multiplier drops from 3.0x to 2.5x. Still rewards the berserker fantasy, but 1,500 max pack DPS is less degenerate than 1,980.

---

### Interceptor (3 units, 1x3 pack) — Anti-Projectile / Defensive Ranged — NO CHANGES

Neutralizes rockets and provides moderate ranged support. 250 range and 432 pack DPS means Interceptor is never dead weight — it's a functional ranged unit that also happens to hard-counter Artillery. Intentionally not the best at pure combat (that's Striker/Ranger's job).

| Stat | Value |
|------|-------|
| HP | 600 |
| Damage | 120 |
| Attack Speed | 1.2 |
| Attack Range | 250 |
| Projectile Speed | 450 |
| Move Speed | 100 |
| Armor | 0 |

| Metric | Value |
|--------|-------|
| Pack HP | 1,800 |
| DPS/unit | 144 |
| Pack DPS | 432 |

**Rationale:** This unit SHOULD underperform without rockets present. The point is to counter Artillery, not to be a general-purpose damage dealer. No changes.

---

---

## T3 Units (300g)

### Artillery (2 units, 1x2 pack) — SPECIALIST: Anti-Swarm / Area Denial

Slow-firing rockets with massive splash. Devastating against clumps, helpless up close. The hard counter to T1 spam — a single rocket into a Chaff blob can wipe half the pack. Countered by: fast melee rush (Scouts, Berserkers close the min-range gap), Interceptors (neutralize rockets mid-flight).

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| HP | 700 | **800** | +100 |
| Damage | 500 | **450** | -50 |
| Attack Speed | 0.4 | 0.4 | -- |
| Attack Range | 450 | 450 | -- |
| Projectile Speed | 300 | 300 | -- |
| Projectile Type | Rocket | Rocket | -- |
| Move Speed | 50 | 50 | -- |
| Armor | 0 | 0 | -- |
| Splash Radius | 40 | **45** | +5 |
| Min Attack Range | 150 | 150 | -- |

| Metric | Current | Proposed |
|--------|---------|----------|
| Pack HP | 1,400 | **1,600** |
| DPS/unit | 200 | **180** |
| Pack DPS | 400 | **360** |

**Rationale:** Slight shift from per-hit damage toward splash coverage. Lower damage per rocket but bigger splash means Artillery is better at clearing swarms (its job) and slightly worse at single-target damage (Sniper's job). HP bump makes it less trivially sniped.

---

### Shield (2 units, 1x2 pack) — SPECIALIST: Projectile Denial

Purely defensive force multiplier. Projects a barrier that intercepts incoming projectiles, protecting everything behind it from ranged fire. The hard counter to ranged-heavy armies (Strikers, Rangers, Snipers). Countered by: melee units that walk through the barrier, or sustained fire that depletes the barrier HP pool.

| Stat | Current | Proposed | Delta |
|------|---------|----------|-------|
| HP | 1,500 | **1,300** | -200 |
| Damage | 50 | **60** | +10 |
| Attack Speed | 0.5 | 0.5 | -- |
| Attack Range | 100 | 100 | -- |
| Move Speed | 55 | 55 | -- |
| Armor | 50 | **40** | -10 |
| Shield Radius | 80 | 80 | -- |
| Shield HP | 3,000 | **2,500** | -500 |

| Metric | Current | Proposed |
|--------|---------|----------|
| Pack HP | 3,000 | **2,600** |
| Pack Barrier HP | 6,000 | **5,000** |
| Pack DPS | 50 | **60** |

**Rationale:** Shield's value is the barrier, not personal tankiness. Reducing unit HP, armor, and barrier HP brings the total defensive package down from overwhelming (9,000 HP+barrier combined) to strong but not oppressive (7,600). Still a huge force multiplier when positioned correctly.

---

## Full Comparison Table

| Unit | Tier | Pack | HP | Dmg | AtkSpd | Range | Proj | Move | Armor | Splash | Special |
|------|------|------|----|-----|--------|-------|------|------|-------|--------|---------|
| Chaff | T1 | 18 | 100 | 30 | 1.5 | 30 | melee | 150 | 0 | 0 | -- |
| Skirmisher | T1 | 12 | 70 | 25 | 2.0 | 160 | 350 | 150 | 0 | 0 | -- |
| Scout | T1 | 6 | 350 | 100 | 1.5 | 100 | 300 | 190 | 0 | 0 | -- |
| Sniper | T1 | 1 | 500 | 800 | 0.25 | 500 | 1100 | 45 | 0 | 0 | Laser, min range 150 |
| Striker | T2 | 3 | 600 | 220 | 1.5 | 200 | 400 | 120 | 0 | 0 | -- |
| Bruiser | T2 | 2 | 1500 | 160 | 1.0 | 100 | melee | 90 | 25 | 25 | -- |
| Sentinel | T2 | 2 | 2200 | 100 | 0.8 | 80 | melee | 65 | 60 | 20 | -- |
| Ranger | T2 | 3 | 750 | 170 | 0.75 | 300 | 500 | 80 | 10 | 0 | Laser |
| Dragoon | T2 | 4 | 1100 | 180 | 0.55 | 150 | 350 | 85 | 35 | 0 | -- |
| Berserker | T2 | 3 | 1000 | 200 | 1.0-2.5x | 60 | melee | 130 | 15 | 12 | Rage scaling |
| Interceptor | T2 | 3 | 600 | 120 | 1.2 | 250 | 450 | 100 | 0 | 0 | Intercepts rockets |
| Artillery | T3 | 2 | 800 | 450 | 0.4 | 450 | 300 | 50 | 0 | 45 | Rocket, min range 150 |
| Shield | T3 | 2 | 1300 | 60 | 0.5 | 100 | melee | 55 | 40 | 0 | 2500 barrier HP |

## Pack Power Summary

| Unit | Cost | Pack HP | Pack DPS | HP/gold | DPS/gold |
|------|------|---------|----------|---------|----------|
| Chaff | 100 | 1,800 | 810 | 18.0 | 8.1 |
| Skirmisher | 100 | 840 | 600 | 8.4 | 6.0 |
| Scout | 100 | 2,100 | 900 | 21.0 | 9.0 |
| Sniper | 100 | 500 | 200 | 5.0 | 2.0 |
| Striker | 200 | 1,800 | 990 | 9.0 | 4.95 |
| Bruiser | 200 | 3,000 | 320 | 15.0 | 1.6 |
| Sentinel | 200 | 4,400 | 160 | 22.0 | 0.8 |
| Ranger | 200 | 2,250 | 382 | 11.3 | 1.9 |
| Dragoon | 200 | 4,400 | 396 | 22.0 | 2.0 |
| Berserker | 200 | 3,000 | 600-1500 | 15.0 | 3.0-7.5 |
| Interceptor | 200 | 1,800 | 432 | 9.0 | 2.2 |
| Artillery | 300 | 1,600 | 360 | 5.3 | 1.2 |
| Shield | 300 | 7,600* | 60 | 25.3* | 0.2 |

*Shield HP includes barrier HP (2,600 unit + 5,000 barrier).

Note: Raw HP/gold and DPS/gold don't capture the full picture. Sniper's value is in 800-damage alpha strikes from 500 range. Shield's value is in barrier coverage. Artillery's value is in splash. Sniper has the lowest raw stats of any T1 pack — the value is entirely in the per-hit alpha and extreme range.
