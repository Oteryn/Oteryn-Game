#![allow(clippy::expect_used, reason = "descriptive test failures")]
use super::*;
use std::cell::Cell;

const BOSS: RoomPosition = RoomPosition {
    x: 33_008,
    y: 32_334,
    floor: 15,
};
const BOSSES: [&str; 2] = [ichgahal_think::Ichgahal_KEY, murcion_think::Murcion_KEY];

fn facts<'a>(neighbours: &'a [SummonNeighbour<'a>]) -> SummonWaveFacts<'a> {
    SummonWaveFacts {
        boss: BOSS,
        interval_ms: 1_000,
        neighbours,
        max_room_taint: 9,
        mechanics_disabled: false,
        bloodjaw_spawn_paused: false,
    }
}

fn count_spawn(effects: &[SummonWaveEffect], key: &str, placement: SummonPlacement) -> usize {
    effects
        .iter()
        .filter(|effect| {
            matches!(effect, SummonWaveEffect::Spawn {
        creature_key, around, placement: actual
    } if *creature_key == key && *around == BOSS && *actual == placement)
        })
        .count()
}

#[test]
fn source_bosses_fill_taint_population_and_first_wave() {
    for (key, offset) in BOSSES.into_iter().zip([2, 1]) {
        let mut owner = SummonWaveOwner::for_boss(key).expect("source boss");
        let effects = owner.think(&facts(&[]), || 8_000).expect("source interval");
        assert_eq!(
            count_spawn(
                &effects,
                ichgahal_think::ELDER_BLOODJAW_KEY,
                SummonPlacement::Close { offset_tiles: 2 }
            ),
            5,
            "{key} bloodjaws"
        );
        assert_eq!(
            count_spawn(
                &effects,
                ichgahal_think::MUSHROOM_KEY,
                SummonPlacement::Close {
                    offset_tiles: offset
                }
            ),
            4,
            "{key} minimum mushrooms"
        );
        assert_eq!(
            count_spawn(
                &effects,
                ichgahal_think::MUSHROOM_KEY,
                SummonPlacement::Random {
                    offset_tiles: offset
                }
            ),
            3,
            "{key} wave"
        );
    }
    assert!(SummonWaveOwner::for_boss("oteryn:creature.bakragore").is_some());
    assert!(SummonWaveOwner::for_boss("oteryn:creature.chagorz").is_none());
    assert!(SummonWaveOwner::for_boss("unqualified:boss").is_none());
}

#[test]
fn census_respects_each_source_radius_and_floor() {
    let neighbour = SummonNeighbour {
        creature_key: ichgahal_think::MUSHROOM_KEY,
        position: RoomPosition {
            x: BOSS.x + 15,
            ..BOSS
        },
    };
    let neighbours = [neighbour; 4];
    for (key, missing) in BOSSES.into_iter().zip([4, 0]) {
        let mut owner = SummonWaveOwner::for_boss(key).expect("source boss");
        let effects = owner
            .think(&facts(&neighbours), || 8_000)
            .expect("source interval");
        let close = effects
            .iter()
            .filter(|effect| {
                matches!(effect,
            SummonWaveEffect::Spawn { creature_key, placement: SummonPlacement::Close { .. }, .. }
            if *creature_key == ichgahal_think::MUSHROOM_KEY)
            })
            .count();
        assert_eq!(close, missing, "{key} uses its own source census radius");
        let other_floor = neighbours.map(|mut row| {
            row.position.floor += 1;
            row
        });
        let draws = Cell::new(0);
        let effects = owner
            .think(&facts(&other_floor), || {
                draws.set(draws.get() + 1);
                8_000
            })
            .expect("same floor census");
        assert_eq!(draws.get(), 0, "no wave due");
        assert_eq!(
            effects
                .iter()
                .filter(|effect| matches!(effect,
            SummonWaveEffect::Spawn { creature_key, placement: SummonPlacement::Close { .. }, .. }
            if *creature_key == ichgahal_think::MUSHROOM_KEY))
                .count(),
            4
        );
    }
}

#[test]
fn invalid_draw_or_taint_preserves_attempt_and_does_not_borrow_another_room() {
    for key in BOSSES {
        let mut owner = SummonWaveOwner::for_boss(key).expect("source boss");
        let before = owner.clone();
        assert_eq!(
            owner.think(&facts(&[]), || 7_999),
            Err(SummonWaveRefusal::InvalidIntervalDraw)
        );
        assert_eq!(owner, before);
        let mut invalid = facts(&[]);
        invalid.max_room_taint = 10;
        let draws = Cell::new(0);
        assert_eq!(
            owner.think(&invalid, || {
                draws.set(draws.get() + 1);
                8_000
            }),
            Err(SummonWaveRefusal::InvalidTaint)
        );
        assert_eq!(draws.get(), 0, "taint rejected before draw");
        assert_eq!(owner, before);
        owner.think(&facts(&[]), || 8_000).expect("first wave");
        owner.record_wave_summon(42);
        let mut next = facts(&[]);
        next.interval_ms = 8_000;
        let effects = owner.think(&next, || 9_000).expect("second wave");
        assert!(effects.contains(&SummonWaveEffect::RemoveSummon { actor: 42 }));
        let fresh = SummonWaveOwner::for_boss(key)
            .expect("independent attempt")
            .think(&facts(&[]), || 8_000)
            .expect("fresh wave");
        assert!(
            !fresh
                .iter()
                .any(|effect| matches!(effect, SummonWaveEffect::RemoveSummon { .. }))
        );
    }
}

#[test]
fn vemiath_population_uses_its_own_close_offset_and_disabled_gate() {
    const KEY: &str = "oteryn:creature.vemiath";
    for (disabled, paused, expected_bloodjaws) in
        [(false, false, 5), (true, false, 0), (false, true, 0)]
    {
        let mut owner = SummonWaveOwner::for_boss(KEY).expect("Vemiath native source population");
        let mut input = facts(&[]);
        input.mechanics_disabled = disabled;
        input.bloodjaw_spawn_paused = paused;
        let draws = Cell::new(0_u32);
        let effects = owner
            .think(&input, || {
                draws.set(draws.get() + 1);
                8_000
            })
            .expect("source population");
        assert_eq!(draws.get(), 0, "Vemiath population has no interval draw");
        assert_eq!(
            count_spawn(
                &effects,
                "oteryn:creature.elder_bloodjaw",
                SummonPlacement::Close { offset_tiles: 1 }
            ),
            expected_bloodjaws
        );
        assert_eq!(
            count_spawn(
                &effects,
                "oteryn:creature.pillar_of_dark_energy",
                SummonPlacement::Close { offset_tiles: 1 }
            ),
            4
        );
        assert_eq!(effects.len(), expected_bloodjaws + 4);
    }
}

#[test]
fn vemiath_population_counts_both_minions_at_seventeen_tiles_on_its_floor() {
    for (distance, floor, bloodjaws, pillars) in [(17, 15, 4, 3), (18, 15, 5, 4), (17, 14, 5, 4)] {
        let neighbours = [
            SummonNeighbour {
                creature_key: "oteryn:creature.elder_bloodjaw",
                position: RoomPosition {
                    x: BOSS.x + distance,
                    y: BOSS.y,
                    floor,
                },
            },
            SummonNeighbour {
                creature_key: "oteryn:creature.pillar_of_dark_energy",
                position: RoomPosition {
                    x: BOSS.x + distance,
                    y: BOSS.y,
                    floor,
                },
            },
        ];
        let draws = Cell::new(0_u32);
        let effects = SummonWaveOwner::for_boss("oteryn:creature.vemiath")
            .expect("Vemiath")
            .think(&facts(&neighbours), || {
                draws.set(draws.get() + 1);
                8_000
            })
            .expect("population");
        assert_eq!(draws.get(), 0, "Vemiath does not draw a wave interval");
        assert_eq!(
            count_spawn(
                &effects,
                "oteryn:creature.elder_bloodjaw",
                SummonPlacement::Close { offset_tiles: 1 }
            ),
            bloodjaws
        );
        assert_eq!(
            count_spawn(
                &effects,
                "oteryn:creature.pillar_of_dark_energy",
                SummonPlacement::Close { offset_tiles: 1 }
            ),
            pillars
        );
    }
}

#[test]
fn bakragore_population_has_three_source_kinds_and_its_own_disabled_gate() {
    for (disabled, paused, expected_bloodjaws, expected_others) in [
        (false, false, 5, 4),
        (false, true, 0, 4),
        (true, false, 0, 0),
    ] {
        let mut owner = SummonWaveOwner::for_boss("oteryn:creature.bakragore")
            .expect("Bakragore native population");
        let mut input = facts(&[]);
        input.mechanics_disabled = disabled;
        input.bloodjaw_spawn_paused = paused;
        let draws = Cell::new(0_u32);
        let effects = owner
            .think(&input, || {
                draws.set(draws.get() + 1);
                8_000
            })
            .expect("population");
        assert_eq!(
            draws.get(),
            0,
            "population does not consume a wave timer draw"
        );
        for (key, count) in [
            ("oteryn:creature.elder_bloodjaw", expected_bloodjaws),
            ("oteryn:creature.mushroom", expected_others),
            ("oteryn:creature.pillar_of_dark_energy", expected_others),
        ] {
            assert_eq!(
                count_spawn(&effects, key, SummonPlacement::Close { offset_tiles: 1 }),
                count
            );
        }
        assert_eq!(effects.len(), expected_bloodjaws + 2 * expected_others);
    }
}

#[test]
fn bakragore_population_counts_three_kinds_on_its_twenty_tile_floor() {
    for (distance, floor, expected) in [
        (20, 15, [4, 3, 3]),
        (21, 15, [5, 4, 4]),
        (20, 14, [5, 4, 4]),
    ] {
        let keys = [
            "oteryn:creature.elder_bloodjaw",
            "oteryn:creature.mushroom",
            "oteryn:creature.pillar_of_dark_energy",
        ];
        let neighbours = keys.map(|key| SummonNeighbour {
            creature_key: key,
            position: RoomPosition {
                x: BOSS.x + distance,
                y: BOSS.y,
                floor,
            },
        });
        let mut owner = SummonWaveOwner::for_boss("oteryn:creature.bakragore").expect("Bakragore");
        let effects = owner
            .think(&facts(&neighbours), || 8_000)
            .expect("source population");
        for (key, count) in keys.into_iter().zip(expected) {
            assert_eq!(
                count_spawn(&effects, key, SummonPlacement::Close { offset_tiles: 1 }),
                count
            );
        }
    }
}
