//! Pure food regeneration rates and Premium vocation eligibility.
use oteryn_gameplay_content::content::spell_data::Vocation;

/// The fixed tick of the food condition; every vocation period is a multiple of it.
pub const FOOD_TICK_MS: u32 = 1_000;

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

/// Credits one fed tick to a living actor. Dead actors accumulate and gain nothing.
pub fn credit_fed_second(
    state: &mut RegenerationState,
    rate: RegenerationRate,
    max_health: u32,
    max_mana: u32,
) {
    if state.health == 0 {
        return;
    }
    state.regen_health_ms = state.regen_health_ms.saturating_add(FOOD_TICK_MS);
    if state.regen_health_ms >= rate.health_period_ms {
        state.regen_health_ms -= rate.health_period_ms;
        state.health = state
            .health
            .saturating_add(rate.health_amount)
            .min(max_health);
    }
    state.regen_mana_ms = state.regen_mana_ms.saturating_add(FOOD_TICK_MS);
    if state.regen_mana_ms >= rate.mana_period_ms {
        state.regen_mana_ms -= rate.mana_period_ms;
        state.mana = state.mana.saturating_add(rate.mana_amount).min(max_mana);
    }
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
}
