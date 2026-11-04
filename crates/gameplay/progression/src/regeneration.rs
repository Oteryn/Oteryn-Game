//! Native regeneration clocks, legacy vocation amounts and Premium eligibility.
use oteryn_gameplay_content::content::spell_data::Vocation;

/// The fixed tick of the food condition; every vocation period is a multiple of it.
pub const FOOD_TICK_MS: u32 = 1_000;
pub const FED_HEALTH_PERIOD_MS: u32 = 3_000;
pub const FED_MANA_PERIOD_MS: u32 = 5_000;
pub const UNFED_HEALTH_PERIOD_MS: u32 = 4_000;
pub const UNFED_MANA_PERIOD_MS: u32 = 6_000;

/// One vocation's regeneration: `amount` points per `period_ms` for each pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegenerationRate {
    pub health_amount: u32,
    pub health_period_ms: u32,
    pub mana_amount: u32,
    pub mana_period_ms: u32,
}

/// A promoted vocation keeps its promotion benefit only while Premium is current (D76); without
/// it the base vocation's rate applies.
pub const fn effective(vocation: Vocation, premium_current: bool) -> Vocation {
    if premium_current {
        return vocation;
    }
    match vocation {
        Vocation::ElderDruid => Vocation::Druid,
        Vocation::MasterSorcerer => Vocation::Sorcerer,
        Vocation::EliteKnight => Vocation::Knight,
        Vocation::RoyalPaladin => Vocation::Paladin,
        Vocation::ExaltedMonk => Vocation::Monk,
        base => base,
    }
}

/// Owner-decided Canary 15.30 values (`vocation-vitals-candidate-2026-09-28.json`,
/// `regeneration`, D5b); a test keeps this table equal to that file.
pub const fn rate(vocation: Vocation) -> RegenerationRate {
    let (health_period_ms, mana_period_ms) = match vocation {
        Vocation::None => (12_000, 6_000),
        Vocation::Druid | Vocation::Sorcerer => (12_000, 3_000),
        Vocation::ElderDruid | Vocation::MasterSorcerer => (12_000, 2_000),
        Vocation::Knight | Vocation::Monk => (6_000, 6_000),
        Vocation::EliteKnight | Vocation::ExaltedMonk => (4_000, 6_000),
        Vocation::Paladin => (8_000, 4_000),
        Vocation::RoyalPaladin => (6_000, 3_000),
    };
    RegenerationRate {
        health_amount: 1,
        health_period_ms,
        mana_amount: 2,
        mana_period_ms,
    }
}

/// Plain health, mana and fed-period progress, independent of the runtime actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegenerationState {
    pub health: u32,
    pub mana: u32,
    pub regen_health_ms: u32,
    pub regen_mana_ms: u32,
}

/// One worn item's regeneration (Crystal CONDITION_REGENERATION params). A zero gain or zero
/// ticks disables that pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipmentRegeneration {
    pub health_gain: u32,
    pub health_ticks_ms: u32,
    pub mana_gain: u32,
    pub mana_ticks_ms: u32,
}

/// One slot's runtime condition: its params and its own internal tick counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipmentRegenerationClock {
    pub slot: u8,
    pub regeneration: EquipmentRegeneration,
    pub health_ms: u32,
    pub mana_ms: u32,
}

/// One 1000 ms tick of every worn regeneration condition, in slice order.
pub fn credit_equipment_second(
    state: &mut RegenerationState,
    clocks: &mut [EquipmentRegenerationClock],
    max_health: u32,
    max_mana: u32,
    in_protection_zone: bool,
    effective_streak: u32,
) {
    if state.health == 0 {
        return;
    }
    for clock in clocks {
        clock.health_ms = clock.health_ms.saturating_add(FOOD_TICK_MS);
        clock.mana_ms = clock.mana_ms.saturating_add(FOOD_TICK_MS);
        let regeneration = clock.regeneration;
        if (!in_protection_zone || effective_streak >= 2)
            && regeneration.health_gain > 0
            && regeneration.health_ticks_ms > 0
            && clock.health_ms >= regeneration.health_ticks_ms
        {
            clock.health_ms = 0;
            let multiplier = u32::from(in_protection_zone && effective_streak >= 5) + 1;
            state.health = state
                .health
                .saturating_add(regeneration.health_gain.saturating_mul(multiplier))
                .min(max_health);
        }
        if (!in_protection_zone || effective_streak >= 3)
            && regeneration.mana_gain > 0
            && regeneration.mana_ticks_ms > 0
            && clock.mana_ms >= regeneration.mana_ticks_ms
        {
            clock.mana_ms = 0;
            let multiplier = u32::from(in_protection_zone && effective_streak >= 6) + 1;
            state.mana = state
                .mana
                .saturating_add(regeneration.mana_gain.saturating_mul(multiplier))
                .min(max_mana);
        }
    }
}

/// Credits one fed tick to a living actor. Dead actors neither accumulate nor gain.
pub fn credit_fed_second(
    state: &mut RegenerationState,
    rate: RegenerationRate,
    max_health: u32,
    max_mana: u32,
) {
    credit_second(state, rate, max_health, max_mana, true);
}

/// Credits one native regeneration tick. Food changes only the two clocks; vocation amounts
/// remain the source of each completed tick's credit.
pub fn credit_second(
    state: &mut RegenerationState,
    rate: RegenerationRate,
    max_health: u32,
    max_mana: u32,
    fed: bool,
) {
    if state.health == 0 {
        return;
    }
    let (health_period_ms, mana_period_ms) = if fed {
        (FED_HEALTH_PERIOD_MS, FED_MANA_PERIOD_MS)
    } else {
        (UNFED_HEALTH_PERIOD_MS, UNFED_MANA_PERIOD_MS)
    };
    state.regen_health_ms = state.regen_health_ms.saturating_add(FOOD_TICK_MS);
    if state.regen_health_ms >= health_period_ms {
        state.regen_health_ms -= health_period_ms;
        state.health = state
            .health
            .saturating_add(rate.health_amount)
            .min(max_health);
    }
    state.regen_mana_ms = state.regen_mana_ms.saturating_add(FOOD_TICK_MS);
    if state.regen_mana_ms >= mana_period_ms {
        state.regen_mana_ms -= mana_period_ms;
        state.mana = state.mana.saturating_add(rate.mana_amount).min(max_mana);
    }
}

/// Holds one tick inside a protection zone under the existing native policy: progress
/// accumulates but credits nothing, and stops one tick short of each period so the first
/// tick outside credits the due period once and starts the next one from zero.
pub fn hold_fed_second(state: &mut RegenerationState, rate: RegenerationRate) {
    if state.health == 0 {
        return;
    }
    let hold = |progress: u32, period: u32| {
        progress
            .saturating_add(FOOD_TICK_MS)
            .min(period.saturating_sub(FOOD_TICK_MS))
            .max(progress)
    };
    state.regen_health_ms = hold(state.regen_health_ms, rate.health_period_ms);
    state.regen_mana_ms = hold(state.regen_mana_ms, rate.mana_period_ms);
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn rate_table_equals_the_owner_decided_vitals_file() {
        let table: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/vocation-vitals-candidate-2026-09-28.json"
        ))
        .expect("json");
        for (key, vocation) in [
            ("none", Vocation::None),
            ("druid", Vocation::Druid),
            ("elder_druid", Vocation::ElderDruid),
            ("sorcerer", Vocation::Sorcerer),
            ("master_sorcerer", Vocation::MasterSorcerer),
            ("knight", Vocation::Knight),
            ("elite_knight", Vocation::EliteKnight),
            ("paladin", Vocation::Paladin),
            ("royal_paladin", Vocation::RoyalPaladin),
            ("monk", Vocation::Monk),
            ("exalted_monk", Vocation::ExaltedMonk),
        ] {
            let regen = &table["vocations"][key]["regeneration"];
            let got = rate(vocation);
            assert_eq!(regen["gainhpamount"], got.health_amount, "{key}");
            assert_eq!(regen["gainhpticks"], got.health_period_ms, "{key}");
            assert_eq!(regen["gainmanaamount"], got.mana_amount, "{key}");
            assert_eq!(regen["gainmanaticks"], got.mana_period_ms, "{key}");
            assert_eq!(got.health_period_ms % FOOD_TICK_MS, 0, "{key}");
            assert_eq!(got.mana_period_ms % FOOD_TICK_MS, 0, "{key}");
        }
    }

    #[test]
    fn fed_and_unfed_clocks_credit_one_vocation_amount_per_period() {
        let rate = rate(Vocation::Knight);
        for (fed, seconds, expected) in [(true, 5, (11, 12)), (false, 6, (11, 12))] {
            let mut state = RegenerationState {
                health: 10,
                mana: 10,
                regen_health_ms: 0,
                regen_mana_ms: 0,
            };
            for _ in 0..seconds {
                credit_second(&mut state, rate, 100, 100, fed);
            }
            assert_eq!((state.health, state.mana), expected);
        }
    }

    #[test]
    fn equipment_regeneration_resets_to_zero_and_respects_zones_and_streaks() {
        let regeneration = EquipmentRegeneration {
            health_gain: 6,
            health_ticks_ms: 6_000,
            mana_gain: 24,
            mana_ticks_ms: 6_000,
        };
        let mut state = RegenerationState {
            health: 10,
            mana: 10,
            regen_health_ms: 0,
            regen_mana_ms: 0,
        };
        let mut clocks = [EquipmentRegenerationClock {
            slot: 8,
            regeneration,
            health_ms: 0,
            mana_ms: 0,
        }];
        for _ in 0..5 {
            credit_equipment_second(&mut state, &mut clocks, 1_000, 1_000, false, 0);
        }
        assert_eq!((state.health, state.mana), (10, 10));
        assert_eq!((clocks[0].health_ms, clocks[0].mana_ms), (5_000, 5_000));
        credit_equipment_second(&mut state, &mut clocks, 1_000, 1_000, false, 0);
        assert_eq!((state.health, state.mana), (16, 34));
        assert_eq!((clocks[0].health_ms, clocks[0].mana_ms), (0, 0));
        for _ in 0..6 {
            credit_equipment_second(&mut state, &mut clocks, 1_000, 1_000, false, 0);
        }
        assert_eq!((state.health, state.mana), (22, 58));

        for _ in 0..7 {
            credit_equipment_second(&mut state, &mut clocks, 1_000, 1_000, true, 0);
        }
        assert_eq!((state.health, state.mana), (22, 58));
        assert_eq!((clocks[0].health_ms, clocks[0].mana_ms), (7_000, 7_000));
        credit_equipment_second(&mut state, &mut clocks, 1_000, 1_000, false, 0);
        assert_eq!((state.health, state.mana), (28, 82));
        assert_eq!((clocks[0].health_ms, clocks[0].mana_ms), (0, 0));

        for (streak, expected) in [(2, (16, 10)), (5, (22, 34)), (6, (22, 58))] {
            let mut tier = RegenerationState {
                health: 10,
                mana: 10,
                ..state
            };
            let mut tier_clocks = [EquipmentRegenerationClock {
                slot: 8,
                regeneration,
                health_ms: 5_000,
                mana_ms: 5_000,
            }];
            credit_equipment_second(&mut tier, &mut tier_clocks, 1_000, 1_000, true, streak);
            assert_eq!((tier.health, tier.mana), expected, "streak {streak}");
        }

        state.health = 995;
        clocks[0].health_ms = 5_000;
        credit_equipment_second(&mut state, &mut clocks, 1_000, 1_000, false, 0);
        assert_eq!(state.health, 1_000);
        state.health = 0;
        let before = clocks;
        credit_equipment_second(&mut state, &mut clocks, 1_000, 1_000, false, 0);
        assert_eq!(clocks, before);
    }

    #[test]
    fn two_worn_regeneration_items_keep_separate_counters() {
        let mut state = RegenerationState {
            health: 10,
            mana: 10,
            regen_health_ms: 0,
            regen_mana_ms: 0,
        };
        let mut clocks = [
            EquipmentRegenerationClock {
                slot: 8,
                regeneration: EquipmentRegeneration {
                    health_gain: 2,
                    health_ticks_ms: 6_000,
                    mana_gain: 0,
                    mana_ticks_ms: 0,
                },
                health_ms: 0,
                mana_ms: 0,
            },
            EquipmentRegenerationClock {
                slot: 9,
                regeneration: EquipmentRegeneration {
                    health_gain: 0,
                    health_ticks_ms: 0,
                    mana_gain: 5,
                    mana_ticks_ms: 3_000,
                },
                health_ms: 0,
                mana_ms: 0,
            },
        ];
        for _ in 0..6 {
            credit_equipment_second(&mut state, &mut clocks, 100, 100, false, 0);
        }
        assert_eq!((state.health, state.mana), (12, 20));
    }
}
