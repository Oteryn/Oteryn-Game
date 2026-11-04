#![allow(
    clippy::expect_used,
    reason = "test-only assertions keep descriptive failures"
)]

use super::*;
use std::cell::Cell;

const BOSS: RoomPosition = RoomPosition {
    x: 33_008,
    y: 32_334,
    floor: 15,
};

fn facts() -> IchgahalThinkFacts {
    IchgahalThinkFacts {
        boss: BOSS,
        interval_ms: 1_000,
        nearby_mushrooms: 4,
        nearby_bloodjaws: 2,
        bloodjaw_target: 2,
        mechanics_disabled: false,
        bloodjaw_spawn_paused: false,
    }
}

fn spawn(creature_key: &'static str, placement: Placement) -> IchgahalEffect {
    IchgahalEffect::Spawn {
        creature_key,
        around: BOSS,
        placement,
    }
}

#[test]
fn first_think_fires_a_wave_of_three_random_mushrooms_and_draws_the_next_interval() {
    assert_eq!(Ichgahal_KEY, "oteryn:creature.ichgahal");
    let mut owner = IchgahalThinkOwner::default();
    let effects = owner
        .think(&facts(), || 8_500)
        .expect("the in-range interval draw should be accepted");
    let wave_spawn = spawn(
        MUSHROOM_KEY,
        Placement::Random {
            offset_tiles: SUMMON_2_TILES,
        },
    );
    assert_eq!(effects, vec![wave_spawn; 3]);

    for actor in [11, 12, 13] {
        owner.record_wave_summon(actor);
    }
    assert_eq!(owner.active_wave(), &[11, 12, 13]);
}

#[test]
fn wave_repeats_after_the_drawn_interval_and_removes_the_previous_wave() {
    let draws = Cell::new(0);
    let draw = || {
        draws.set(draws.get() + 1);
        8_500
    };
    let mut owner = IchgahalThinkOwner::default();
    owner
        .think(&facts(), draw)
        .expect("the first interval draw should be accepted");
    for actor in [11, 12, 13] {
        owner.record_wave_summon(actor);
    }

    for _ in 1..=8 {
        assert_eq!(owner.think(&facts(), draw), Ok(Vec::new()));
    }
    let effects = owner
        .think(&facts(), draw)
        .expect("the second interval draw should be accepted");
    let wave_spawn = spawn(
        MUSHROOM_KEY,
        Placement::Random {
            offset_tiles: SUMMON_2_TILES,
        },
    );
    assert_eq!(
        effects,
        vec![
            IchgahalEffect::RemoveSummon { actor: 11 },
            IchgahalEffect::RemoveSummon { actor: 12 },
            IchgahalEffect::RemoveSummon { actor: 13 },
            wave_spawn,
            wave_spawn,
            wave_spawn,
        ]
    );
    assert!(owner.active_wave().is_empty());
    assert_eq!(draws.get(), 2);
}

#[test]
fn mushroom_floor_and_bloodjaws_are_topped_up_every_think() {
    let mut owner = IchgahalThinkOwner::default();
    owner
        .think(&facts(), || 9_000)
        .expect("the initial interval draw should be accepted");
    let low_counts = IchgahalThinkFacts {
        nearby_mushrooms: 1,
        nearby_bloodjaws: 0,
        bloodjaw_target: 3,
        ..facts()
    };
    let bloodjaw_spawn = spawn(
        ELDER_BLOODJAW_KEY,
        Placement::Close {
            offset_tiles: BLOODJAW_2_TILES,
        },
    );
    let mushroom_spawn = spawn(
        MUSHROOM_KEY,
        Placement::Close {
            offset_tiles: SUMMON_2_TILES,
        },
    );
    assert_eq!(
        owner.think(&low_counts, || 9_000),
        Ok(vec![
            bloodjaw_spawn,
            bloodjaw_spawn,
            bloodjaw_spawn,
            mushroom_spawn,
            mushroom_spawn,
            mushroom_spawn,
        ])
    );

    let high_counts = IchgahalThinkFacts {
        nearby_mushrooms: 5,
        nearby_bloodjaws: 4,
        bloodjaw_target: 3,
        ..facts()
    };
    assert_eq!(owner.think(&high_counts, || 9_000), Ok(Vec::new()));
}

#[test]
fn disabled_mechanics_or_paused_spawn_stop_only_the_bloodjaws() {
    let mushroom_spawn = spawn(
        MUSHROOM_KEY,
        Placement::Close {
            offset_tiles: SUMMON_2_TILES,
        },
    );
    for facts in [
        IchgahalThinkFacts {
            nearby_mushrooms: 3,
            nearby_bloodjaws: 0,
            bloodjaw_target: 3,
            mechanics_disabled: true,
            ..facts()
        },
        IchgahalThinkFacts {
            nearby_mushrooms: 3,
            nearby_bloodjaws: 0,
            bloodjaw_target: 3,
            bloodjaw_spawn_paused: true,
            ..facts()
        },
    ] {
        let mut owner = IchgahalThinkOwner::default();
        owner
            .think(&facts, || 9_000)
            .expect("the initial interval draw should be accepted");
        assert_eq!(owner.think(&facts, || 9_000), Ok(vec![mushroom_spawn]));
    }
}

#[test]
fn an_out_of_range_wave_interval_draw_refuses_without_changing_the_owner() {
    for invalid_draw in [7_999, 9_001] {
        let mut owner = IchgahalThinkOwner::default();
        assert_eq!(
            owner.think(&facts(), || invalid_draw),
            Err(IchgahalRefusal::InvalidWaveIntervalDraw)
        );
        assert_eq!(owner, IchgahalThinkOwner::default());
    }

    assert!(
        IchgahalThinkOwner::default()
            .think(&facts(), || WAVE_INTERVAL_MIN_MS)
            .is_ok()
    );
}

#[test]
fn summon_ranges_are_14_for_mushrooms_and_15_for_bloodjaws() {
    let offset = |dx, floor| RoomPosition {
        x: BOSS.x + dx,
        y: BOSS.y,
        floor,
    };
    assert!(counts_mushroom(BOSS, offset(14, BOSS.floor)));
    assert!(!counts_mushroom(BOSS, offset(15, BOSS.floor)));
    assert!(!counts_mushroom(BOSS, offset(14, BOSS.floor + 1)));
    assert!(counts_bloodjaw(BOSS, offset(15, BOSS.floor)));
    assert!(!counts_bloodjaw(BOSS, offset(16, BOSS.floor)));
}
