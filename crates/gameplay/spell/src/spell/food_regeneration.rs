//! Always-on player health/mana regeneration. Food selects the faster clocks;
//! the existing vocation table supplies the amount credited per completed period.
//! Protection zones hold progress without credits for both hungry and fed actors.
#[cfg(test)]
use super::Vocation;
use super::cast::PlayerSpellState;

pub use oteryn_gameplay_progression::regeneration::{
    FOOD_TICK_MS, RegenerationRate, RegenerationState, effective, rate,
};

/// Every condition consumer advances the same clock before consuming due conditions.
pub(super) fn step_due(state: &mut PlayerSpellState, now: u64, in_protection_zone: bool) {
    if state.regeneration_next_tick_us.unwrap_or(now) > now {
        return;
    }
    let fed = state
        .conditions
        .active_at(
            oteryn_gameplay_domain::domain::condition::ConflictKey::FoodRegeneration,
            now.saturating_sub(1),
        )
        .is_some();
    if in_protection_zone {
        hold_fed_second(state, fed);
    } else {
        credit_second(state, fed);
    }
    state.regeneration_next_tick_us = Some(now.saturating_add(u64::from(FOOD_TICK_MS) * 1_000));
}

/// Credits one fed tick to a living actor. Dead actors neither accumulate nor gain.
pub fn credit_fed_second(state: &mut PlayerSpellState) {
    apply(state, |regeneration, rate, max_health, max_mana| {
        oteryn_gameplay_progression::regeneration::credit_fed_second(
            regeneration,
            rate,
            max_health,
            max_mana,
        );
    });
}

/// Credits the always-on native clock, using the saturation state selected by the condition
/// owner for this occurrence.
pub fn credit_second(state: &mut PlayerSpellState, fed: bool) {
    apply(state, |regeneration, rate, max_health, max_mana| {
        oteryn_gameplay_progression::regeneration::credit_second(
            regeneration,
            rate,
            max_health,
            max_mana,
            fed,
        );
    });
}

/// Holds one native tick in a protection zone. The historical helper also serves hungry actors.
pub fn hold_fed_second(state: &mut PlayerSpellState, fed: bool) {
    apply(state, |regeneration, mut rate, _, _| {
        use oteryn_gameplay_progression::regeneration::{
            FED_HEALTH_PERIOD_MS, FED_MANA_PERIOD_MS, UNFED_HEALTH_PERIOD_MS, UNFED_MANA_PERIOD_MS,
        };
        (rate.health_period_ms, rate.mana_period_ms) = if fed {
            (FED_HEALTH_PERIOD_MS, FED_MANA_PERIOD_MS)
        } else {
            (UNFED_HEALTH_PERIOD_MS, UNFED_MANA_PERIOD_MS)
        };
        oteryn_gameplay_progression::regeneration::hold_fed_second(regeneration, rate);
    });
}

fn apply(
    state: &mut PlayerSpellState,
    step: impl FnOnce(&mut RegenerationState, RegenerationRate, u32, u32),
) {
    let mut regeneration = RegenerationState {
        health: state.health,
        mana: state.mana,
        regen_health_ms: state.regen_health_ms,
        regen_mana_ms: state.regen_mana_ms,
    };
    step(
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
    fn fed_then_unfed_knight_uses_the_respective_native_periods() {
        let mut state = actor(Vocation::Knight);
        eat_food(&mut state, 10, 0).expect("eat");
        run(&mut state, 0, 5, TickFacts::default());
        assert_eq!((state.health, state.mana), (101, 102));
        run(&mut state, 5, 15, TickFacts::default());
        // Ten fed seconds followed by five unfed seconds retain one credit per due clock.
        assert_eq!((state.health, state.mana), (104, 104));
        assert!(state.conditions.instances().is_empty());
        run(&mut state, 15, 18, TickFacts::default());
        assert_eq!((state.health, state.mana), (105, 106));
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
        assert_eq!(lapsed.health, 102);
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
        assert_eq!(state.regen_health_ms, 2_000);
        super::super::actor_conditions::clear_on_lifecycle(&mut state);
        assert_eq!((state.regen_health_ms, state.regen_mana_ms), (0, 0));
        assert_eq!(state.regeneration_next_tick_us, None);
    }

    #[test]
    fn fed_and_unfed_actors_regenerate_on_their_native_clocks() {
        let mut druid = actor(Vocation::Druid);
        let mut unfed = actor(Vocation::Druid);
        eat_food(&mut druid, 12, 0).expect("eat");
        run(&mut druid, 0, 12, TickFacts::default());
        run(&mut unfed, 0, 12, TickFacts::default());
        assert_eq!((druid.health, druid.mana), (104, 104));
        assert_eq!((unfed.health, unfed.mana), (103, 104));
    }

    #[test]
    fn real_periodic_dispatch_path_ticks_fed_and_unfed_actors() {
        fn dispatch(state: &mut PlayerSpellState, seconds: u64) {
            for second in 1..=seconds {
                let cycle = super::super::actor_conditions::stage_source_owner_cycle(
                    state,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(
                        second * 1_000_000,
                    ),
                    Some(TickFacts::default()),
                    None,
                )
                .expect("periodic dispatch")
                .expect("clock successor");
                assert!(cycle.combat_ticks.is_empty());
                *state = cycle.next;
            }
        }

        let mut fed = actor(Vocation::Knight);
        let mut unfed = actor(Vocation::Knight);
        eat_food(&mut fed, 6, 0).expect("eat");
        dispatch(&mut fed, 6);
        dispatch(&mut unfed, 6);
        assert_eq!((fed.health, fed.mana), (102, 102));
        assert_eq!((unfed.health, unfed.mana), (101, 102));
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
    fn a_protection_zone_holds_the_due_period_until_the_first_tick_outside() {
        let mut state = actor(Vocation::Knight);
        eat_food(&mut state, 30, 0).expect("eat");
        let pz = TickFacts {
            in_protection_zone: true,
            standing_on_field: None,
        };
        // Fed clocks hold one second short of HP 3 s and MP 5 s.
        run(&mut state, 0, 10, pz);
        assert_eq!((state.health, state.mana), (100, 100));
        assert_eq!((state.regen_health_ms, state.regen_mana_ms), (2_000, 4_000));
        // The first tick outside credits once and starts the next period from zero.
        run(&mut state, 10, 11, TickFacts::default());
        assert_eq!((state.health, state.mana), (101, 102));
        assert_eq!((state.regen_health_ms, state.regen_mana_ms), (0, 0));
        run(&mut state, 11, 16, TickFacts::default());
        assert_eq!((state.health, state.mana), (102, 104));
        run(&mut state, 16, 17, TickFacts::default());
        assert_eq!((state.health, state.mana), (103, 104));
    }

    #[test]
    fn a_short_protection_zone_stay_keeps_the_fed_time() {
        let mut state = actor(Vocation::Knight);
        eat_food(&mut state, 30, 0).expect("eat");
        let pz = TickFacts {
            in_protection_zone: true,
            standing_on_field: None,
        };
        run(&mut state, 0, 2, TickFacts::default());
        run(&mut state, 2, 4, pz);
        assert_eq!(state.regen_health_ms, 2_000);
        run(&mut state, 4, 5, TickFacts::default());
        assert_eq!(state.health, 101);
        run(&mut state, 5, 6, TickFacts::default());
        assert_eq!(state.health, 101);
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
        hold_fed_second(&mut dead, true);
        assert_eq!((dead.health, dead.regen_health_ms), (0, 0));
    }

    #[test]
    fn hungry_protection_zone_ticks_hold_progress_without_credits() {
        let mut state = actor(Vocation::Knight);
        let pz = TickFacts {
            in_protection_zone: true,
            standing_on_field: None,
        };
        run(&mut state, 0, 10, pz);
        assert_eq!((state.health, state.mana), (100, 100));
        assert_eq!((state.regen_health_ms, state.regen_mana_ms), (3_000, 5_000));
        run(&mut state, 10, 11, TickFacts::default());
        assert_eq!((state.health, state.mana), (101, 102));
        assert_eq!((state.regen_health_ms, state.regen_mana_ms), (0, 0));
    }

    #[test]
    fn source_condition_clock_is_idempotent_at_the_same_now() {
        use oteryn_simulation_determinism::SemanticTimeMicros;

        for fed in [false, true] {
            for in_protection_zone in [false, true] {
                let mut state = actor(Vocation::Knight);
                state
                    .make_playable(SemanticTimeMicros::from_micros(0))
                    .expect("playable actor");
                if fed {
                    eat_food(&mut state, 120, 0).expect("eat");
                }
                let facts = TickFacts {
                    in_protection_zone,
                    standing_on_field: None,
                };
                for now in [1_000_000, 60_000_000] {
                    let _ = state.source_take_due(now, facts);
                    assert_eq!(state.regeneration_next_tick_us, Some(now + 1_000_000));
                    let expected_progress = if now == 1_000_000 { 1_000 } else { 2_000 };
                    assert_eq!(state.regen_health_ms, expected_progress);
                    assert_eq!(state.regen_mana_ms, expected_progress);
                    let before = state.clone();
                    assert!(state.source_take_due(now, facts).is_empty());
                    assert_eq!(state, before);
                    assert!(
                        super::super::actor_conditions::stage_source_owner_cycle(
                            &state,
                            SemanticTimeMicros::from_micros(now),
                            Some(facts),
                            None,
                        )
                        .expect("same-time environment cycle")
                        .is_none()
                    );
                    assert_eq!(state, before);
                }
            }
        }
    }

    #[test]
    fn delayed_clock_pass_cannot_be_replayed_to_credit_extra_seconds() {
        let mut state = actor(Vocation::Knight);
        run(&mut state, 0, 1, TickFacts::default());
        tick(&mut state, 60_000, TickFacts::default()).expect("delayed pass");
        let before = state.clone();
        for now_ms in [
            60_000, 60_000, 60_000, 60_100, 60_200, 60_300, 60_400, 60_500,
        ] {
            tick(&mut state, now_ms, TickFacts::default()).expect("repeated or subsecond pass");
            assert_eq!((state.health, state.mana), (before.health, before.mana));
            assert_eq!(state.regen_health_ms, before.regen_health_ms);
            assert_eq!(state.regen_mana_ms, before.regen_mana_ms);
        }
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
