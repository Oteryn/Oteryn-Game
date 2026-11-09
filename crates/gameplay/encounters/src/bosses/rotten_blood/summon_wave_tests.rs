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
    assert!(SummonWaveOwner::for_boss("oteryn:creature.bakragore").is_none());
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
