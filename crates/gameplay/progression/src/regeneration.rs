//! Native regeneration clocks, vocation percentage credits and Premium eligibility.
use oteryn_gameplay_content::content::spell_data::Vocation;

/// The fixed tick of the food condition; every vocation period is a multiple of it.
pub const FOOD_TICK_MS: u32 = 1_000;
pub const FED_HEALTH_PERIOD_MS: u32 = 3_000;
pub const FED_MANA_PERIOD_MS: u32 = 3_000;
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

/// Percentage credited per completed native regeneration tick, in basis points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegenerationPercent {
    pub health_bp: u32,
    pub mana_bp: u32,
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

/// Owner-decided values (`vocation-vitals-candidate-2026-09-28.json`, `regeneration`): Canary
/// 15.30 (D5b), with Monk/Exalted Monk equal to Paladin/Royal Paladin (owner 2026-10-29,
/// WAVE_A_PLAN Q4). A test keeps this table equal to that file.
pub const fn rate(vocation: Vocation) -> RegenerationRate {
    let (health_period_ms, mana_period_ms) = match vocation {
        Vocation::None => (12_000, 6_000),
        Vocation::Druid | Vocation::Sorcerer => (12_000, 3_000),
        Vocation::ElderDruid | Vocation::MasterSorcerer => (12_000, 2_000),
        Vocation::Knight => (6_000, 6_000),
        Vocation::EliteKnight => (4_000, 6_000),
        Vocation::Paladin | Vocation::Monk => (8_000, 4_000),
        Vocation::RoyalPaladin | Vocation::ExaltedMonk => (6_000, 3_000),
    };
    RegenerationRate {
        health_amount: 1,
        health_period_ms,
        mana_amount: 2,
        mana_period_ms,
    }
}

/// Owner-decided percentage regeneration table (REGEN-PCT-4).
pub const fn percent(vocation: Vocation) -> RegenerationPercent {
    let (health_bp, mana_bp) = match vocation {
        Vocation::None => (0, 0),
        Vocation::Druid | Vocation::Sorcerer => (8, 15),
        Vocation::ElderDruid | Vocation::MasterSorcerer => (10, 20),
        Vocation::Knight => (10, 8),
        Vocation::EliteKnight => (15, 10),
        Vocation::Paladin | Vocation::Monk => (8, 10),
        Vocation::RoyalPaladin | Vocation::ExaltedMonk => (10, 15),
    };
    RegenerationPercent { health_bp, mana_bp }
}

/// Plain health, mana and fed-period progress, independent of the runtime actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegenerationState {
    pub health: u32,
    pub mana: u32,
    pub regen_health_ms: u32,
    pub regen_mana_ms: u32,
    pub regen_health_carry: u32,
    pub regen_mana_carry: u32,
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
    vocation: Vocation,
    rate: RegenerationRate,
    max_health: u32,
    max_mana: u32,
) {
    credit_second(state, vocation, rate, max_health, max_mana, true);
}

/// Credits one native regeneration tick. Food changes only the two clocks; each completed tick
/// uses the vocation percentage with the legacy Global speed as its floor.
pub fn credit_second(
    state: &mut RegenerationState,
    vocation: Vocation,
    rate: RegenerationRate,
    max_health: u32,
    max_mana: u32,
    fed: bool,
) {
    if state.health == 0 {
        return;
    }
    if state.health >= max_health {
        state.regen_health_carry = 0;
    }
    if state.mana >= max_mana {
        state.regen_mana_carry = 0;
    }
    let (health_period_ms, mana_period_ms) = if fed {
        (FED_HEALTH_PERIOD_MS, FED_MANA_PERIOD_MS)
    } else {
        (UNFED_HEALTH_PERIOD_MS, UNFED_MANA_PERIOD_MS)
    };
    state.regen_health_ms = state.regen_health_ms.saturating_add(FOOD_TICK_MS);
    if state.regen_health_ms >= health_period_ms {
        state.regen_health_ms -= health_period_ms;
        credit_pool(
            &mut state.health,
            &mut state.regen_health_carry,
            max_health,
            vocation,
            percent(vocation).health_bp,
            rate.health_amount,
            rate.health_period_ms,
        );
    }
    state.regen_mana_ms = state.regen_mana_ms.saturating_add(FOOD_TICK_MS);
    if state.regen_mana_ms >= mana_period_ms {
        state.regen_mana_ms -= mana_period_ms;
        credit_pool(
            &mut state.mana,
            &mut state.regen_mana_carry,
            max_mana,
            vocation,
            percent(vocation).mana_bp,
            rate.mana_amount,
            rate.mana_period_ms,
        );
    }
}

fn credit_pool(
    pool: &mut u32,
    carry: &mut u32,
    max_pool: u32,
    vocation: Vocation,
    basis_points: u32,
    global_amount: u32,
    global_period_ms: u32,
) {
    if *pool >= max_pool {
        *carry = 0;
        return;
    }
    if vocation == Vocation::None {
        *pool = pool.saturating_add(global_amount).min(max_pool);
        *carry = 0;
        return;
    }
    const POINT_UNITS: u64 = 10_000;
    const FED_TICK_PERIOD_MS: u64 = 3_000;
    let percent_units = u64::from(max_pool) * u64::from(basis_points);
    let global_units =
        u64::from(global_amount) * POINT_UNITS * FED_TICK_PERIOD_MS / u64::from(global_period_ms);
    let units = u64::from(*carry) + percent_units.max(global_units);
    let gained = units / POINT_UNITS;
    *carry = (units % POINT_UNITS) as u32;
    *pool = pool
        .saturating_add(gained.min(u64::from(u32::MAX)) as u32)
        .min(max_pool);
    if *pool == max_pool {
        *carry = 0;
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
    fn monk_regeneration_equals_paladin_including_premium_fallback() {
        for (monk, paladin) in [
            (Vocation::Monk, Vocation::Paladin),
            (Vocation::ExaltedMonk, Vocation::RoyalPaladin),
        ] {
            assert_eq!(rate(monk), rate(paladin), "{monk:?}");
            for premium_current in [false, true] {
                assert_eq!(
                    rate(effective(monk, premium_current)),
                    rate(effective(paladin, premium_current)),
                    "{monk:?} premium={premium_current}"
                );
            }
        }
    }

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

    fn state(health: u32, mana: u32) -> RegenerationState {
        RegenerationState {
            health,
            mana,
            regen_health_ms: 0,
            regen_mana_ms: 0,
            regen_health_carry: 0,
            regen_mana_carry: 0,
        }
    }

    fn tick(
        state: &mut RegenerationState,
        vocation: Vocation,
        max_health: u32,
        max_mana: u32,
        fed: bool,
        seconds: u32,
    ) {
        for _ in 0..seconds {
            credit_second(state, vocation, rate(vocation), max_health, max_mana, fed);
        }
    }

    #[test]
    fn percent_tick_uses_effective_vocation_basis_points() {
        let vocation = effective(Vocation::EliteKnight, true);
        let mut state = state(1, 0);
        tick(&mut state, vocation, 15_065, 5_050, true, 3);
        assert_eq!((state.health, state.mana), (23, 5));
        assert_eq!(
            (state.regen_health_carry, state.regen_mana_carry),
            (5_975, 500)
        );
        tick(&mut state, vocation, 15_065, 5_050, true, 3);
        assert_eq!((state.health, state.mana), (46, 10));
    }

    #[test]
    fn promoted_without_premium_uses_base_row() {
        let vocation = effective(Vocation::EliteKnight, false);
        let mut state = state(1, 0);
        tick(&mut state, vocation, 15_065, 5_050, true, 3);
        assert_eq!((state.health, state.mana), (16, 4));
    }

    #[test]
    fn fed_tick_is_three_seconds_for_every_vocation() {
        let vocation = effective(Vocation::MasterSorcerer, true);
        let mut state = state(1, 0);
        tick(&mut state, vocation, 1_000, 1_000, true, 2);
        assert_eq!((state.health, state.mana), (1, 0));
        tick(&mut state, vocation, 1_000, 1_000, true, 1);
        assert_eq!((state.health, state.mana), (2, 3));
    }

    #[test]
    fn hungry_uses_unfed_clocks_same_amounts() {
        let vocation = effective(Vocation::MasterSorcerer, true);
        let mut state = state(1, 0);
        tick(&mut state, vocation, 1_000, 29_850, false, 3);
        assert_eq!((state.health, state.mana), (1, 0));
        tick(&mut state, vocation, 1_000, 29_850, false, 1);
        assert_eq!((state.health, state.mana), (2, 0));
        tick(&mut state, vocation, 1_000, 29_850, false, 2);
        assert_eq!(state.mana, 59);
        assert_eq!(state.regen_mana_carry, 7_000);
    }

    #[test]
    fn low_level_gets_global_floor() {
        let mut state = state(1, 0);
        tick(&mut state, Vocation::Knight, 185, 90, true, 3);
        assert_eq!((state.health, state.mana), (1, 1));
        tick(&mut state, Vocation::Knight, 185, 90, true, 3);
        assert_eq!((state.health, state.mana), (2, 2));
    }

    #[test]
    fn vocation_none_is_flat() {
        let mut state = state(1, 0);
        tick(&mut state, Vocation::None, 100, 100, true, 3);
        assert_eq!((state.health, state.mana), (2, 2));
        assert_eq!((state.regen_health_carry, state.regen_mana_carry), (0, 0));
    }

    #[test]
    fn full_pool_clears_carry() {
        let mut state = state(999, 999);
        state.regen_health_carry = 9_000;
        state.regen_mana_carry = 9_000;
        tick(&mut state, Vocation::EliteKnight, 1_000, 1_000, true, 3);
        assert_eq!((state.health, state.mana), (1_000, 1_000));
        assert_eq!((state.regen_health_carry, state.regen_mana_carry), (0, 0));
    }

    #[test]
    fn pz_hold_then_step_out_credits_once() {
        let mut state = state(1, 0);
        state.regen_health_carry = 123;
        state.regen_mana_carry = 456;
        let mut fed_rate = rate(Vocation::Knight);
        fed_rate.health_period_ms = FED_HEALTH_PERIOD_MS;
        fed_rate.mana_period_ms = FED_MANA_PERIOD_MS;
        for _ in 0..4 {
            hold_fed_second(&mut state, fed_rate);
        }
        assert_eq!(
            (state.regen_health_carry, state.regen_mana_carry),
            (123, 456)
        );
        credit_second(
            &mut state,
            Vocation::Knight,
            rate(Vocation::Knight),
            185,
            90,
            true,
        );
        assert_eq!((state.health, state.mana), (1, 1));
        assert_eq!((state.regen_health_ms, state.regen_mana_ms), (0, 0));
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
            regen_health_carry: 0,
            regen_mana_carry: 0,
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
            regen_health_carry: 0,
            regen_mana_carry: 0,
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
