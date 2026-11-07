#![allow(
    clippy::expect_used,
    reason = "test-only assertions keep descriptive failures"
)]
use super::*;
use std::cell::Cell;

const BOSS: RoomPosition = RoomPosition {
    x: 33_008,
    y: 32_367,
    floor: 15,
};

fn facts() -> MurcionThinkFacts {
    MurcionThinkFacts {
        boss: BOSS,
        interval_ms: 1_000,
        nearby_mushrooms: 4,
        nearby_bloodjaws: 2,
        bloodjaw_target: 2,
        mechanics_disabled: false,
        bloodjaw_spawn_paused: false,
    }
}

fn wave_spawn() -> MurcionEffect {
    MurcionEffect::Spawn {
        creature_key: MUSHROOM_KEY,
        around: BOSS,
        placement: Placement::Random { offset_tiles: 1 },
    }
}

#[test]
fn first_think_fires_a_wave_of_three_random_mushrooms_and_draws_the_next_interval() {
    let mut owner = MurcionThinkOwner::default();
    let effects = owner.think(&facts(), || 8_500).expect("valid wave draw");

    assert_eq!(effects, vec![wave_spawn(), wave_spawn(), wave_spawn()]);
    owner.record_wave_summon(11);
    owner.record_wave_summon(12);
    owner.record_wave_summon(13);
    assert_eq!(owner.active_wave(), &[11, 12, 13]);
}

#[test]
fn wave_repeats_after_the_drawn_interval_and_removes_the_previous_wave() {
    let draws = Cell::new(0);
    let draw = || {
        draws.set(draws.get() + 1);
        8_500
    };
    let mut owner = MurcionThinkOwner::default();
    owner.think(&facts(), draw).expect("first valid wave");
    for actor in [11, 12, 13] {
        owner.record_wave_summon(actor);
    }

    for _ in 1..=8 {
        assert_eq!(owner.think(&facts(), draw).expect("valid think"), vec![]);
    }
    assert_eq!(
        owner.think(&facts(), draw).expect("second valid wave"),
        vec![
            MurcionEffect::RemoveSummon { actor: 11 },
            MurcionEffect::RemoveSummon { actor: 12 },
            MurcionEffect::RemoveSummon { actor: 13 },
            wave_spawn(),
            wave_spawn(),
            wave_spawn(),
        ]
    );
    assert!(owner.active_wave().is_empty());
    assert_eq!(draws.get(), 2);
}

#[test]
fn mushroom_floor_and_bloodjaws_are_topped_up_every_think() {
    let mut owner = MurcionThinkOwner::default();
    owner.think(&facts(), || 9_000).expect("initial wave");
    let mut sparse = facts();
    sparse.nearby_mushrooms = 1;
    sparse.nearby_bloodjaws = 0;
    sparse.bloodjaw_target = 3;
    let bloodjaw = MurcionEffect::Spawn {
        creature_key: ELDER_BLOODJAW_KEY,
        around: BOSS,
        placement: Placement::Close { offset_tiles: 2 },
    };
    let mushroom = MurcionEffect::Spawn {
        creature_key: MUSHROOM_KEY,
        around: BOSS,
        placement: Placement::Close { offset_tiles: 1 },
    };

    assert_eq!(
        owner.think(&sparse, || 9_000).expect("top-up think"),
        vec![bloodjaw, bloodjaw, bloodjaw, mushroom, mushroom, mushroom]
    );
    sparse.nearby_mushrooms = 5;
    sparse.nearby_bloodjaws = 4;
    assert_eq!(owner.think(&sparse, || 9_000).expect("full floor"), vec![]);
}

#[test]
fn disabled_mechanics_or_paused_spawn_stop_only_the_bloodjaws() {
    let mut owner = MurcionThinkOwner::default();
    owner.think(&facts(), || 9_000).expect("initial wave");
    let mushroom = MurcionEffect::Spawn {
        creature_key: MUSHROOM_KEY,
        around: BOSS,
        placement: Placement::Close { offset_tiles: 1 },
    };
    let mut disabled = facts();
    disabled.nearby_mushrooms = 3;
    disabled.nearby_bloodjaws = 0;
    disabled.bloodjaw_target = 3;
    disabled.mechanics_disabled = true;
    assert_eq!(
        owner.think(&disabled, || 9_000).expect("disabled"),
        vec![mushroom]
    );

    disabled.mechanics_disabled = false;
    disabled.bloodjaw_spawn_paused = true;
    assert_eq!(
        owner.think(&disabled, || 9_000).expect("paused"),
        vec![mushroom]
    );
}

#[test]
fn an_out_of_range_wave_interval_draw_refuses_without_changing_the_owner() {
    for invalid in [7_999, 9_001] {
        let mut owner = MurcionThinkOwner::default();
        assert_eq!(
            owner.think(&facts(), || invalid),
            Err(MurcionRefusal::InvalidWaveIntervalDraw)
        );
        assert_eq!(owner, MurcionThinkOwner::default());
    }
    assert!(
        MurcionThinkOwner::default()
            .think(&facts(), || 8_000)
            .is_ok()
    );
}

#[test]
fn summon_ranges_are_16_for_mushrooms_and_15_for_bloodjaws() {
    let at = |dx, floor| RoomPosition {
        x: BOSS.x + dx,
        y: BOSS.y,
        floor,
    };
    assert!(counts_mushroom(BOSS, at(16, BOSS.floor)));
    assert!(!counts_mushroom(BOSS, at(17, BOSS.floor)));
    assert!(!counts_mushroom(BOSS, at(0, BOSS.floor + 1)));
    assert!(counts_bloodjaw(BOSS, at(15, BOSS.floor)));
    assert!(!counts_bloodjaw(BOSS, at(16, BOSS.floor)));
}

#[test]
fn murcion_creature_key_is_exported_for_the_follow_up_hook() {
    assert_eq!(MURCION_KEY, "oteryn:creature.murcion");
}

#[test]
fn source_first_think_tops_up_four_then_attempts_three_wave_placements() {
    // Crystal 201189f murcion.lua:140-141 initializes the interval to zero;
    // :191-215 executes ensureSummons before the immediate wave.
    let mut initial = facts();
    initial.nearby_mushrooms = 0;
    initial.nearby_bloodjaws = 0;
    let effects = MurcionThinkOwner::default()
        .think(&initial, || 8_000)
        .expect("source first wave");
    assert_eq!(effects.len(), 9);
    assert_eq!(
        effects
            .iter()
            .filter(|effect| matches!(
                effect,
                MurcionEffect::Spawn {
                    creature_key: MUSHROOM_KEY,
                    placement: Placement::Close { offset_tiles: 1 },
                    ..
                }
            ))
            .count(),
        4
    );
    assert_eq!(
        effects
            .iter()
            .filter(|effect| matches!(
                effect,
                MurcionEffect::Spawn {
                    creature_key: MUSHROOM_KEY,
                    placement: Placement::Random { offset_tiles: 1 },
                    ..
                }
            ))
            .count(),
        3
    );
}
