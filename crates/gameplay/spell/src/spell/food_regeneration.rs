//! FOOD-REGEN-1: fed-time regeneration of a player's health and mana. While the
//! `FoodRegeneration` condition runs, each one-second tick credits fed time toward the
//! vocation's health and mana periods; a completed period credits its amount once.
//! Protection-zone ticks are suppressed upstream and credit nothing.
#[cfg(test)]
use super::Vocation;
use super::cast::PlayerSpellState;

pub use oteryn_gameplay_progression::regeneration::{
    FOOD_TICK_MS, RegenerationRate, RegenerationState, effective, rate,
};

/// Credits one fed tick to a living actor. Dead actors accumulate and gain nothing.
pub fn credit_fed_second(state: &mut PlayerSpellState) {
    let mut regeneration = RegenerationState {
        health: state.health,
        mana: state.mana,
        regen_health_ms: state.regen_health_ms,
        regen_mana_ms: state.regen_mana_ms,
    };
    oteryn_gameplay_progression::regeneration::credit_fed_second(
        &mut regeneration,
        rate(effective(state.facts.vocation, state.premium_current)),
        state.facts.max_health,
        state.facts.max_mana,
    );
    state.health = regeneration.health;
    state.mana = regeneration.mana;
    state.regen_health_ms = regeneration.regen_health_ms;
    state.regen_mana_ms = regeneration.regen_mana_ms;
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::super::actor_conditions::{eat_food, tick};
    use super::super::cast::CharacterCastFacts;
    use super::*;
    use oteryn_gameplay_foundation::foundation::condition::TickFacts;

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
    fn fed_knight_regenerates_per_vocation_period_and_stops_when_the_food_ends() {
        let mut state = actor(Vocation::Knight);
        eat_food(&mut state, 10, 0).expect("eat");
        run(&mut state, 0, 5, TickFacts::default());
        assert_eq!((state.health, state.mana), (100, 100));
        run(&mut state, 5, 15, TickFacts::default());
        // 10 fed seconds: one health credit (6 s), one mana credit (6 s).
        assert_eq!((state.health, state.mana), (101, 102));
        assert!(state.conditions.instances().is_empty());
        let after = (state.health, state.mana, state.regen_health_ms);
        tick(&mut state, 60_000, TickFacts::default()).expect("tick");
        assert_eq!((state.health, state.mana, state.regen_health_ms), after);
    }

    #[test]
    fn expired_premium_regenerates_at_the_base_vocation_rate() {
        for (promoted, base) in [
            (Vocation::ElderDruid, Vocation::Druid),
            (Vocation::MasterSorcerer, Vocation::Sorcerer),
            (Vocation::EliteKnight, Vocation::Knight),
            (Vocation::RoyalPaladin, Vocation::Paladin),
            (Vocation::ExaltedMonk, Vocation::Monk),
        ] {
            assert_eq!(effective(promoted, false), base);
            assert_eq!(effective(promoted, true), promoted);
            assert_eq!(effective(base, false), base);
        }
        let mut lapsed = actor(Vocation::EliteKnight);
        lapsed.premium_current = false;
        eat_food(&mut lapsed, 6, 0).expect("eat");
        run(&mut lapsed, 0, 6, TickFacts::default());
        // Knight rate: one health credit in 6 s (Elite Knight would need 4 s).
        assert_eq!(lapsed.health, 101);
        assert_eq!(lapsed.regen_health_ms, 0);
        let mut current = actor(Vocation::EliteKnight);
        current.premium_current = true;
        eat_food(&mut current, 4, 0).expect("eat");
        run(&mut current, 0, 4, TickFacts::default());
        assert_eq!(current.health, 101);
    }

    #[test]
    fn lifecycle_clearing_drops_regeneration_progress() {
        let mut state = actor(Vocation::Knight);
        eat_food(&mut state, 10, 0).expect("eat");
        run(&mut state, 0, 5, TickFacts::default());
        assert_eq!(state.regen_health_ms, 5_000);
        super::super::actor_conditions::clear_on_lifecycle(&mut state);
        assert_eq!((state.regen_health_ms, state.regen_mana_ms), (0, 0));
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
        state.premium_current = true;
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
