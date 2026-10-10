//! FOOD-REGEN-1: fed-time regeneration of a player's health and mana. While the
//! `FoodRegeneration` condition runs, each one-second tick credits fed time toward the
//! vocation's health and mana periods; a completed period credits its amount once.
//! Protection-zone ticks are suppressed upstream and credit nothing.
use super::Vocation;
use super::cast::PlayerSpellState;

/// The fixed tick of the food condition; every vocation period is a multiple of it.
pub(crate) const FOOD_TICK_MS: u32 = 1_000;

/// One vocation's regeneration: `amount` points per `period_ms` for each pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RegenerationRate {
    pub(crate) health_amount: u32,
    pub(crate) health_period_ms: u32,
    pub(crate) mana_amount: u32,
    pub(crate) mana_period_ms: u32,
}

/// Owner-decided Canary 15.30 values (`vocation-vitals-candidate-2026-09-28.json`,
/// `regeneration`, D5b); a test keeps this table equal to that file.
pub(crate) const fn rate(vocation: Vocation) -> RegenerationRate {
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

/// Credits one fed tick to a living actor. Dead actors accumulate and gain nothing.
pub(crate) fn credit_fed_second(state: &mut PlayerSpellState) {
    if state.health == 0 {
        return;
    }
    let rate = rate(state.facts.vocation);
    state.regen_health_ms = state.regen_health_ms.saturating_add(FOOD_TICK_MS);
    if state.regen_health_ms >= rate.health_period_ms {
        state.regen_health_ms -= rate.health_period_ms;
        state.health = state
            .health
            .saturating_add(rate.health_amount)
            .min(state.facts.max_health);
    }
    state.regen_mana_ms = state.regen_mana_ms.saturating_add(FOOD_TICK_MS);
    if state.regen_mana_ms >= rate.mana_period_ms {
        state.regen_mana_ms -= rate.mana_period_ms;
        state.mana = state
            .mana
            .saturating_add(rate.mana_amount)
            .min(state.facts.max_mana);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::super::actor_conditions::{eat_food, tick};
    use super::super::cast::CharacterCastFacts;
    use super::*;
    use crate::ability::condition::TickFacts;

    fn actor(vocation: Vocation) -> PlayerSpellState {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation,
                level: 100,
                magic_level: 10,
                max_health: 1000,
                max_mana: 500,
                max_soul: 100,
            },
            0,
            0,
        )
        .expect("actor");
        state.health = 100;
        state.mana = 100;
        state
    }

    /// Ticks seconds `from + 1 ..= to` (time stays monotonic across calls).
    fn run(state: &mut PlayerSpellState, from: u64, to: u64, facts: TickFacts) {
        for second in from + 1..=to {
            tick(state, second * 1_000, facts).expect("tick");
        }
    }

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

    #[test]
    fn fed_knight_regenerates_per_vocation_period_and_stops_when_the_food_ends() {
        let mut state = actor(Vocation::Knight);
        eat_food(&mut state, 10, 0).expect("eat");
        run(&mut state, 0, 5, TickFacts::default());
        assert_eq!((state.health, state.mana), (100, 100));
        run(&mut state, 5, 15, TickFacts::default());
        // 10 fed seconds: one health credit (6 s), one mana credit (6 s).
        assert_eq!((state.health, state.mana), (101, 102));
        assert!(state.conditions.instances().is_empty());
        let after = state.clone();
        tick(&mut state, 60_000, TickFacts::default()).expect("tick");
        assert_eq!(state, after);
    }

    #[test]
    fn vocations_differ_and_unfed_actors_do_not_regenerate() {
        let mut druid = actor(Vocation::Druid);
        let mut unfed = actor(Vocation::Druid);
        eat_food(&mut druid, 12, 0).expect("eat");
        run(&mut druid, 0, 12, TickFacts::default());
        run(&mut unfed, 0, 12, TickFacts::default());
        assert_eq!((druid.health, druid.mana), (101, 108));
        assert_eq!((unfed.health, unfed.mana), (100, 100));
    }

    #[test]
    fn protection_zone_suppresses_regeneration_but_food_is_consumed() {
        let mut state = actor(Vocation::EliteKnight);
        eat_food(&mut state, 6, 0).expect("eat");
        let pz = TickFacts {
            in_protection_zone: true,
            standing_on_field: None,
        };
        run(&mut state, 0, 6, pz);
        assert_eq!((state.health, state.mana), (100, 100));
        assert!(state.conditions.instances().is_empty());
    }

    #[test]
    fn credits_clamp_at_the_maxima_and_dead_actors_gain_nothing() {
        let mut full = actor(Vocation::Sorcerer);
        full.health = full.facts.max_health;
        full.mana = full.facts.max_mana;
        eat_food(&mut full, 30, 0).expect("eat");
        run(&mut full, 0, 30, TickFacts::default());
        assert_eq!((full.health, full.mana), (1000, 500));
        let mut dead = actor(Vocation::Sorcerer);
        dead.health = 0;
        credit_fed_second(&mut dead);
        assert_eq!((dead.health, dead.regen_health_ms), (0, 0));
    }

    #[test]
    fn food_time_adds_up_to_the_cap_then_refuses_without_change() {
        let mut state = actor(Vocation::Knight);
        eat_food(&mut state, 1_000, 0).expect("eat");
        let before = state.clone();
        assert!(eat_food(&mut state, 200, 0).is_err());
        assert_eq!(state, before);
        assert!(eat_food(&mut state, 0, 0).is_err());
        assert_eq!(state, before);
    }
}
