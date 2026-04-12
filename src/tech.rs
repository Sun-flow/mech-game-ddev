use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use crate::unit::{Unit, UnitKind, UnitStats};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TechId {
    // Universal techs
    RangeBoost,
    ArmorBoost,
    SplashBoost,

    // Generic techs (apply to multiple units)
    HardenedFrame,
    Overdrive,
    HighCaliber,

    // Unique per-kind techs
    StrikerRapidFire,
    ChaffFrenzy,
    SniperStabilizer,
    SentinelBarrier,
    RangerPierce,
    ScoutEvasion,
    BruiserCleave,
    ArtillerySlow,
    ArtilleryBlastRadius,
    ChaffOverwhelm,
    SniperArmorPierce,
    DragoonFortify,
    BerserkerLifesteal,
    ShieldBarrierExpand,
    InterceptorDualWeapon,

    // New behavioral techs
    Entrench,
    SentinelTaunt,
    BerserkerUnstoppable,
    BerserkerDeathThroes,
    BruiserCharge,
    ChaffExpendable,
    ChaffScavenge,
    ShieldReflect,
    ShieldFortress,
    InterceptorFlak,
}

pub struct TechDef {
    pub id: TechId,
    pub name: &'static str,
    pub description: &'static str,
    pub applicable_to: &'static [UnitKind],
}

pub fn all_techs() -> &'static [TechDef] {
    &[
        // Universal techs
        TechDef {
            id: TechId::RangeBoost,
            name: "+Range",
            description: "+40 attack range",
            applicable_to: &[
                UnitKind::Striker, UnitKind::Ranger, UnitKind::Scout,
                UnitKind::Skirmisher, UnitKind::Dragoon, UnitKind::Interceptor,
            ],
        },
        TechDef {
            id: TechId::ArmorBoost,
            name: "+Armor",
            description: "+30 armor",
            applicable_to: &[
                UnitKind::Striker, UnitKind::Sentinel, UnitKind::Ranger,
                UnitKind::Scout, UnitKind::Bruiser, UnitKind::Artillery,
                UnitKind::Sniper, UnitKind::Skirmisher, UnitKind::Dragoon,
                UnitKind::Berserker, UnitKind::Shield, UnitKind::Interceptor,
            ],
        },
        TechDef {
            id: TechId::SplashBoost,
            name: "+Splash",
            description: "+15 splash radius",
            applicable_to: &[
                UnitKind::Sentinel, UnitKind::Bruiser, UnitKind::Artillery,
                UnitKind::Berserker,
            ],
        },
        // Generic techs
        TechDef {
            id: TechId::HardenedFrame,
            name: "Hardened Frame",
            description: "+20% max HP",
            applicable_to: &[
                UnitKind::Chaff, UnitKind::Scout, UnitKind::Striker,
                UnitKind::Ranger, UnitKind::Interceptor, UnitKind::Sniper,
                UnitKind::Artillery,
            ],
        },
        TechDef {
            id: TechId::Overdrive,
            name: "Overdrive",
            description: "+20% move speed",
            applicable_to: &[
                UnitKind::Chaff, UnitKind::Sentinel, UnitKind::Bruiser,
                UnitKind::Dragoon, UnitKind::Artillery, UnitKind::Shield,
            ],
        },
        TechDef {
            id: TechId::HighCaliber,
            name: "High-Caliber",
            description: "+15% damage",
            applicable_to: &[
                UnitKind::Striker, UnitKind::Ranger, UnitKind::Dragoon,
                UnitKind::Interceptor, UnitKind::Berserker, UnitKind::Bruiser,
            ],
        },
        // Unique techs
        TechDef {
            id: TechId::StrikerRapidFire,
            name: "Rapid Fire",
            description: "+0.4 attack speed",
            applicable_to: &[UnitKind::Striker],
        },
        TechDef {
            id: TechId::SentinelBarrier,
            name: "Barrier",
            description: "Projects shield (r=50)",
            applicable_to: &[UnitKind::Sentinel],
        },
        TechDef {
            id: TechId::RangerPierce,
            name: "Pierce",
            description: "Shots hit 2 targets",
            applicable_to: &[UnitKind::Ranger],
        },
        TechDef {
            id: TechId::ScoutEvasion,
            name: "Evasion",
            description: "25% dodge chance",
            applicable_to: &[UnitKind::Scout],
        },
        TechDef {
            id: TechId::BruiserCleave,
            name: "Cleave",
            description: "Splash ignores armor",
            applicable_to: &[UnitKind::Bruiser],
        },
        TechDef {
            id: TechId::ArtillerySlow,
            name: "Slow Shells",
            description: "Hits slow enemies 50% for 2s",
            applicable_to: &[UnitKind::Artillery],
        },
        TechDef {
            id: TechId::ArtilleryBlastRadius,
            name: "Blast Radius",
            description: "+25 splash radius",
            applicable_to: &[UnitKind::Artillery],
        },
        TechDef {
            id: TechId::ChaffOverwhelm,
            name: "Overwhelm",
            description: "+3 dmg per nearby chaff (max 10)",
            applicable_to: &[UnitKind::Chaff],
        },
        TechDef {
            id: TechId::ChaffFrenzy,
            name: "Frenzy",
            description: "+0.5 attack speed",
            applicable_to: &[UnitKind::Chaff],
        },
        TechDef {
            id: TechId::SniperArmorPierce,
            name: "Armor Pierce",
            description: "Shots ignore armor",
            applicable_to: &[UnitKind::Sniper, UnitKind::Striker],
        },
        TechDef {
            id: TechId::SniperStabilizer,
            name: "Stabilizer",
            description: "Min attack range 150 → 75",
            applicable_to: &[UnitKind::Sniper],
        },
        TechDef {
            id: TechId::DragoonFortify,
            name: "Fortify",
            description: "+250 HP, +15 armor",
            applicable_to: &[UnitKind::Dragoon],
        },
        TechDef {
            id: TechId::BerserkerLifesteal,
            name: "Lifesteal",
            description: "Heal on hit (scales w/ rage)",
            applicable_to: &[UnitKind::Berserker],
        },
        TechDef {
            id: TechId::ShieldBarrierExpand,
            name: "Expand Barrier",
            description: "+30 shield radius",
            applicable_to: &[UnitKind::Shield],
        },
        TechDef {
            id: TechId::InterceptorDualWeapon,
            name: "Dual Weapon",
            description: "Intercept + attack same frame",
            applicable_to: &[UnitKind::Interceptor],
        },
        // Behavioral techs
        TechDef {
            id: TechId::Entrench,
            name: "Entrench",
            description: "Stationary: +12% atk speed/sec (max 4)",
            applicable_to: &[UnitKind::Skirmisher, UnitKind::Ranger],
        },
        TechDef {
            id: TechId::SentinelTaunt,
            name: "Taunt Aura",
            description: "Enemies within 120 forced to target Sentinel",
            applicable_to: &[UnitKind::Sentinel],
        },
        TechDef {
            id: TechId::BerserkerUnstoppable,
            name: "Unstoppable",
            description: "Below 50% HP: slow immune, +20% move speed",
            applicable_to: &[UnitKind::Berserker],
        },
        TechDef {
            id: TechId::BerserkerDeathThroes,
            name: "Death Throes",
            description: "On death: 150 dmg splash (r=40)",
            applicable_to: &[UnitKind::Berserker],
        },
        TechDef {
            id: TechId::BruiserCharge,
            name: "Charge",
            description: "First attack after 100+ travel: 2x dmg",
            applicable_to: &[UnitKind::Bruiser],
        },
        TechDef {
            id: TechId::ChaffExpendable,
            name: "Expendable",
            description: "On death: nearby chaff +15% atk speed (3s, max 3)",
            applicable_to: &[UnitKind::Chaff],
        },
        TechDef {
            id: TechId::ChaffScavenge,
            name: "Scavenge",
            description: "On kill: spawn chaff (T1=1, T2=2, T3=3)",
            applicable_to: &[UnitKind::Chaff],
        },
        TechDef {
            id: TechId::ShieldReflect,
            name: "Reflective Barrier",
            description: "15% of absorbed dmg reflected to attacker",
            applicable_to: &[UnitKind::Shield],
        },
        TechDef {
            id: TechId::ShieldFortress,
            name: "Fortress Mode",
            description: "+1500 barrier HP, immobile",
            applicable_to: &[UnitKind::Shield],
        },
        TechDef {
            id: TechId::InterceptorFlak,
            name: "Flak Burst",
            description: "Intercepted rockets detonate (rocket's dmg + splash)",
            applicable_to: &[UnitKind::Interceptor],
        },
    ]
}

#[derive(Clone, Debug)]
pub struct TechState {
    pub purchased: HashMap<UnitKind, Vec<TechId>>,
}

impl TechState {
    pub fn new() -> Self {
        Self {
            purchased: HashMap::new(),
        }
    }

    pub fn tech_count(&self, kind: UnitKind) -> usize {
        self.purchased.get(&kind).map_or(0, |v| v.len())
    }

    /// Cost = 200 + (number of techs already bought for this kind) * 200
    pub fn effective_cost(&self, kind: UnitKind) -> u32 {
        200 + self.tech_count(kind) as u32 * 200
    }

    pub fn has_tech(&self, kind: UnitKind, tech_id: TechId) -> bool {
        self.purchased
            .get(&kind)
            .is_some_and(|v| v.contains(&tech_id))
    }

    pub fn purchase(&mut self, kind: UnitKind, tech_id: TechId) -> bool {
        if self.has_tech(kind, tech_id) {
            return false;
        }
        self.purchased
            .entry(kind)
            .or_default()
            .push(tech_id);
        true
    }

    /// Remove a purchased tech (for undo).
    pub fn unpurchase(&mut self, kind: UnitKind, tech_id: TechId) {
        if let Some(techs) = self.purchased.get_mut(&kind) {
            techs.retain(|t| *t != tech_id);
        }
    }

    /// Get available (not yet purchased) techs for a given unit kind.
    pub fn available_techs(&self, kind: UnitKind) -> Vec<&'static TechDef> {
        all_techs()
            .iter()
            .filter(|t| t.applicable_to.contains(&kind) && !self.has_tech(kind, t.id))
            .collect()
    }

    /// Apply all purchased stat-modifying techs to a UnitStats (in place).
    pub fn apply_to_stats(&self, kind: UnitKind, stats: &mut UnitStats) {
        let purchased = match self.purchased.get(&kind) {
            Some(v) => v,
            None => return,
        };

        for tech_id in purchased {
            match tech_id {
                TechId::RangeBoost => stats.attack_range += 40.0,
                TechId::ArmorBoost => stats.armor += 30.0,
                TechId::SplashBoost => {
                    if stats.splash_radius > 0.0 {
                        stats.splash_radius += 15.0;
                    }
                }
                TechId::HardenedFrame => stats.max_hp *= 1.2,
                TechId::Overdrive => stats.move_speed *= 1.2,
                TechId::HighCaliber => stats.damage *= 1.15,
                TechId::StrikerRapidFire => stats.attack_speed += 0.4,
                TechId::ChaffFrenzy => stats.attack_speed += 0.5,
                TechId::SniperStabilizer => stats.min_attack_range = 75.0,
                TechId::SentinelBarrier => {
                    if stats.shield_radius <= 0.0 {
                        stats.shield_radius = 50.0;
                    }
                }
                TechId::ShieldBarrierExpand => stats.shield_radius += 30.0,
                TechId::ArtilleryBlastRadius => stats.splash_radius += 25.0,
                TechId::DragoonFortify => {
                    stats.max_hp += 250.0;
                    stats.armor += 15.0;
                }
                TechId::ShieldFortress => {
                    stats.shield_hp += 1500.0;
                    stats.move_speed = 0.0;
                }
                // Behavioral techs don't modify stats directly
                TechId::RangerPierce
                | TechId::ScoutEvasion
                | TechId::BruiserCleave
                | TechId::ArtillerySlow
                | TechId::ChaffOverwhelm
                | TechId::SniperArmorPierce
                | TechId::BerserkerLifesteal
                | TechId::InterceptorDualWeapon
                | TechId::Entrench
                | TechId::SentinelTaunt
                | TechId::BerserkerUnstoppable
                | TechId::BerserkerDeathThroes
                | TechId::BruiserCharge
                | TechId::ChaffExpendable
                | TechId::ChaffScavenge
                | TechId::ShieldReflect
                | TechId::InterceptorFlak => {}
            }
        }
    }
}

/// Refresh all units of a given kind to have updated tech-modified stats.
pub fn refresh_units_of_kind(units: &mut [Unit], kind: UnitKind, tech_state: &TechState) {
    for unit in units.iter_mut() {
        if unit.kind != kind || !unit.alive {
            continue;
        }
        let hp_frac = unit.hp / unit.stats.max_hp;
        // Reset to base stats, then re-apply techs
        unit.stats = kind.stats();
        tech_state.apply_to_stats(kind, &mut unit.stats);
        unit.hp = unit.stats.max_hp * hp_frac;
        // Apply evasion
        if kind == UnitKind::Scout
            && tech_state.has_tech(UnitKind::Scout, TechId::ScoutEvasion)
        {
            unit.evasion_chance = 0.25;
        }
    }
}
