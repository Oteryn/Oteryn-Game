#![allow(
    clippy::unwrap_used,
    reason = "focused assertions over explicitly constructed valid state-machine inputs"
)]
use oteryn_gameplay_world_change::counter::{CounterKind as K, CounterState, Credit, Parameters};

#[test]
fn world_counter_demon_save_boundaries_and_strict_prince_threshold() {
    let parameters = Parameters::default();
    let initial = CounterState::new(K::DemonWars, 41, false);
    assert_eq!(initial.stage, 3);
    let count = |n| {
        let mut state = initial;
        for _ in 0..n {
            state = state
                .credit(K::DemonWars, Credit::Kill(0), &parameters, 0)
                .unwrap();
        }
        state
            .at_save(K::DemonWars, 42, false, &parameters, 0)
            .unwrap()
    };
    assert_eq!(count(99).stage, 3);
    assert_eq!(count(100).stage, 4);
    assert_eq!(count(400).stage, 4);
    assert_eq!(count(401).stage, 5);
    let saved = count(401);
    assert_eq!(
        saved,
        saved
            .at_save(K::DemonWars, 42, false, &parameters, 0)
            .unwrap()
    );
    assert!(
        saved
            .at_save(K::DemonWars, 40, false, &parameters, 0)
            .is_err()
    );
}
#[test]
fn world_counter_serpent_is_immediate_and_requires_both_reset_conditions() {
    let p = Parameters::default();
    let mut state = CounterState::new(K::FireFeathered, 1, false);
    for _ in 0..1000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(0), &p, 0)
            .unwrap();
    }
    assert_eq!(state.stage, 2);
    assert!(!state.is_active(K::FireFeathered));
    for _ in 0..1000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(0), &p, 0)
            .unwrap();
    }
    assert_eq!(state.stage, 3);
    assert!(state.is_active(K::FireFeathered));
    for _ in 0..2000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(1), &p, 0)
            .unwrap();
    }
    assert_eq!(
        state.stage, 3,
        "kills alone cannot reset mission-dependent state"
    );
    assert_eq!(
        state
            .credit(K::FireFeathered, Credit::MissionComplete, &p, 0)
            .unwrap()
            .stage,
        1
    );
}
#[test]
fn world_counter_mage_requires_energized_death_and_restarts_its_collapse_clock() {
    let p = Parameters::default();
    let mut state = CounterState::new(K::MagesTower, 1, false);
    for _ in 0..2000 {
        state = state.credit(K::MagesTower, Credit::Kill(0), &p, 0).unwrap();
    }
    assert_eq!(
        state.stage, 1,
        "world kills alone do not kill the energized boss"
    );
    state = state
        .credit(K::MagesTower, Credit::Kill(1), &p, 100)
        .unwrap();
    assert_eq!(state.stage, 2);
    state = state
        .credit(K::MagesTower, Credit::Kill(2), &p, 200)
        .unwrap();
    assert_eq!(state.tick(K::MagesTower, 300199).unwrap().stage, 3);
    let closed = state.tick(K::MagesTower, 300200).unwrap();
    assert_eq!(closed.stage, 4);
    assert_eq!(
        closed
            .at_save(K::MagesTower, 2, false, &p, 300201)
            .unwrap()
            .stage,
        1
    );
}
#[test]
fn world_counter_hive_actions_do_not_confuse_points_or_individual_kills() {
    let p = Parameters::default();
    let mut state = CounterState::new(K::HiveBorn, 1, false);
    for _ in 0..200 {
        state = state.credit(K::HiveBorn, Credit::Task(0), &p, 0).unwrap();
    }
    state = state.at_save(K::HiveBorn, 2, false, &p, 0).unwrap();
    assert_eq!(state.stage, 2);
    for _ in 0..100 {
        state = state.credit(K::HiveBorn, Credit::Task(4), &p, 0).unwrap();
    }
    assert_eq!(
        state.at_save(K::HiveBorn, 3, false, &p, 0).unwrap().stage,
        2,
        "200 points are only100 actions"
    );
    for _ in 0..100 {
        state = state.credit(K::HiveBorn, Credit::Task(5), &p, 0).unwrap();
    }
    state = state.at_save(K::HiveBorn, 3, false, &p, 0).unwrap();
    assert_eq!(state.stage, 3);
    assert_eq!(
        state.at_save(K::HiveBorn, 7, false, &p, 0).unwrap().stage,
        3
    );
    assert_eq!(
        state.at_save(K::HiveBorn, 8, false, &p, 0).unwrap().stage,
        1
    );
}
#[test]
fn world_counter_fire_earth_fifty_world_kills_do_not_activate_or_complete_it() {
    let p = Parameters::default();
    let mut state = CounterState::new(K::FireEarth, 1, false);
    for _ in 0..50 {
        state = state.credit(K::FireEarth, Credit::Kill(0), &p, 0).unwrap();
    }
    assert!(!state.is_active(K::FireEarth));
    state = state.at_save(K::FireEarth, 2, true, &p, 0).unwrap();
    for _ in 0..50 {
        state = state.credit(K::FireEarth, Credit::Kill(0), &p, 0).unwrap();
    }
    assert_eq!(
        state.stage, 1,
        "50 is personal achievement progress, not a world phase"
    );
}

#[test]
fn world_counter_projection_milestones_keep_the_credit_cycle_until_reset() {
    let p = Parameters::default();
    let mut state = CounterState::new(K::FireFeathered, 1, false);
    for _ in 0..1000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(0), &p, 0)
            .unwrap();
    }
    assert_eq!(state.stage, 2);
    assert_eq!(state.credit_generation, 1);
    for _ in 0..1000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(0), &p, 0)
            .unwrap();
    }
    for _ in 0..2000 {
        state = state
            .credit(K::FireFeathered, Credit::Kill(1), &p, 0)
            .unwrap();
    }
    state = state
        .credit(K::FireFeathered, Credit::MissionComplete, &p, 0)
        .unwrap();
    assert_eq!(state.stage, 1);
    assert_eq!(state.credit_generation, 2);
}
#[test]
fn world_counter_membership_excludes_fury_serpents_and_other_fire_caves() {
    let bindings: oteryn_gameplay_world_change::counter::Bindings =
        serde_json::from_str(oteryn_gameplay_world_change::counter::BINDINGS_JSON).unwrap();
    bindings.validate().unwrap();
    assert_eq!(
        bindings.credits("oteryn:creature.seacrest_serpent", (33530, 31630, 14)),
        vec![(K::FireFeathered, Credit::Kill(0))]
    );
    assert!(
        bindings
            .credits("oteryn:creature.seacrest_serpent", (33393, 31855, 14))
            .is_empty()
    );
    assert!(
        bindings
            .credits("oteryn:creature.fire_elemental", (100, 100, 7))
            .is_empty()
    );
}
#[test]
fn world_counter_configured_population_and_deepling_weights_are_distinct() {
    let p = Parameters {
        deepling_points: 7,
        deer_kills: 5,
        wolf_recovery: 20,
    };
    let initial = CounterState::new(K::Deeplings, 1, false);
    assert!(
        initial
            .at_save(K::Deeplings, 2, false, &Parameters::default(), 0)
            .is_err()
    );
    assert!(
        initial
            .credit(K::Deeplings, Credit::Kill(0), &p, 0)
            .is_err()
    );
    let state = initial
        .credit(K::Deeplings, Credit::HeartDestroyed, &p, 0)
        .unwrap()
        .credit(K::Deeplings, Credit::SoulNetDestroyed, &p, 0)
        .unwrap();
    assert_eq!(
        state.at_save(K::Deeplings, 2, false, &p, 0).unwrap().stage,
        2
    );
    let mut state = CounterState::new(K::Overhunting, 1, false);
    for _ in 0..5 {
        state = state
            .credit(K::Overhunting, Credit::Kill(0), &p, 0)
            .unwrap();
    }
    state = state.at_save(K::Overhunting, 2, false, &p, 0).unwrap();
    assert_eq!(state.stage, 2);
    state = state
        .credit(K::Overhunting, Credit::TrappedWolves(2), &p, 0)
        .unwrap();
    assert_eq!(
        state
            .at_save(K::Overhunting, 3, false, &p, 0)
            .unwrap()
            .stage,
        1
    );
}
