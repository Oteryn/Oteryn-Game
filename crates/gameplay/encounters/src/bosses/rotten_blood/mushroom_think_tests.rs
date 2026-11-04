#![allow(
    clippy::expect_used,
    reason = "test-only assertions keep descriptive failures"
)]

use super::*;

const MUSHROOM: u64 = 9;
const OWN: RoomPosition = RoomPosition {
    x: 33_008,
    y: 32_334,
    floor: 15,
};
const BOSS: [(&str, RoomPosition); 1] = [("oteryn:creature.ichgahal", OWN)];

fn target(actor: u64, dx: i32, dy: i32, floor: i16) -> RoomTarget {
    RoomTarget {
        actor,
        position: RoomPosition {
            x: OWN.x + dx,
            y: OWN.y + dy,
            floor,
        },
    }
}

#[test]
fn an_unprimed_mushroom_explodes_eighteen_seconds_after_its_first_think() {
    let mut owner = MushroomThinkOwner::default();

    assert_eq!(owner.think(MUSHROOM, OWN, 1_000, &[], &BOSS), []);
    assert_eq!(owner.explode_at(MUSHROOM), Some(19_000));
    assert_eq!(owner.think(MUSHROOM, OWN, 2_000, &[], &BOSS), []);
    assert_eq!(owner.explode_at(MUSHROOM), Some(19_000));
    assert_eq!(
        owner.explode_due(MUSHROOM, OWN, 18_999, &[], || 2_700),
        Ok(vec![])
    );

    let targets = [
        target(1, 3, 0, 15),
        target(2, 4, 0, 15),
        target(3, 0, -3, 15),
        target(MUSHROOM, 0, 0, 15),
    ];
    assert_eq!(
        owner.explode_due(MUSHROOM, OWN, 19_000, &targets, || 2_700),
        Ok(vec![
            MushroomEffect::BlastVisual {
                mushroom: MUSHROOM,
                centre: OWN,
            },
            MushroomEffect::LifeDrain {
                mushroom: MUSHROOM,
                target: 1,
                amount: 2_700,
            },
            MushroomEffect::LifeDrain {
                mushroom: MUSHROOM,
                target: 3,
                amount: 2_700,
            },
            MushroomEffect::Despawn { mushroom: MUSHROOM },
        ])
    );
    assert_eq!(owner.explode_at(MUSHROOM), None);
}

#[test]
fn an_adjacent_player_primes_a_one_second_fuse() {
    let mut owner = MushroomThinkOwner::default();
    assert_eq!(
        owner.think(MUSHROOM, OWN, 5_000, &[target(1, 1, 0, 15)], &BOSS),
        [MushroomEffect::Primed { mushroom: MUSHROOM }]
    );
    assert_eq!(owner.explode_at(MUSHROOM), Some(6_000));

    let mut distant_owner = MushroomThinkOwner::default();
    assert_eq!(
        distant_owner.think(MUSHROOM, OWN, 5_000, &[target(1, 2, 0, 15)], &BOSS),
        []
    );
    assert_eq!(distant_owner.explode_at(MUSHROOM), Some(23_000));

    let mut other_floor_owner = MushroomThinkOwner::default();
    assert_eq!(
        other_floor_owner.think(MUSHROOM, OWN, 5_000, &[target(1, 1, 0, 14)], &BOSS),
        []
    );
    assert_eq!(other_floor_owner.explode_at(MUSHROOM), Some(23_000));
}

#[test]
fn a_mushroom_without_a_summoner_despawns_and_never_explodes() {
    let mut owner = MushroomThinkOwner::default();
    assert_eq!(
        owner.think(
            MUSHROOM,
            OWN,
            1_000,
            &[target(1, 1, 0, 15)],
            &[("oteryn:creature.chagorz", OWN)],
        ),
        [
            MushroomEffect::Primed { mushroom: MUSHROOM },
            MushroomEffect::Despawn { mushroom: MUSHROOM },
        ]
    );
    assert_eq!(owner.explode_at(MUSHROOM), None);
    assert_eq!(
        owner.explode_due(MUSHROOM, OWN, 2_000, &[target(1, 0, 0, 15)], || 2_700),
        Ok(vec![])
    );

    let mut at_edge = MushroomThinkOwner::default();
    assert_eq!(
        at_edge.think(
            MUSHROOM,
            OWN,
            1_000,
            &[],
            &[("oteryn:creature.murcion", target(1, 15, 0, 15).position)],
        ),
        []
    );
    assert_eq!(at_edge.explode_at(MUSHROOM), Some(19_000));

    let mut outside = MushroomThinkOwner::default();
    assert_eq!(
        outside.think(
            MUSHROOM,
            OWN,
            1_000,
            &[],
            &[("oteryn:creature.murcion", target(1, 16, 0, 15).position)],
        ),
        [MushroomEffect::Despawn { mushroom: MUSHROOM }]
    );
    assert_eq!(outside.explode_at(MUSHROOM), None);
}

#[test]
fn an_out_of_range_drain_draw_refuses_without_changing_the_owner() {
    let mut owner = MushroomThinkOwner::default();
    owner.think(MUSHROOM, OWN, 5_000, &[target(1, 1, 0, 15)], &BOSS);
    let before = owner.clone();
    let targets = [target(1, 0, 0, 15)];

    assert_eq!(
        owner.explode_due(MUSHROOM, OWN, 6_000, &targets, || 3_001),
        Err(MushroomRefusal::InvalidDrainDraw)
    );
    assert_eq!(owner, before);
    assert_eq!(
        owner.explode_due(MUSHROOM, OWN, 6_000, &targets, || 2_499),
        Err(MushroomRefusal::InvalidDrainDraw)
    );
    assert_eq!(owner, before);
    assert_eq!(
        owner.explode_due(MUSHROOM, OWN, 6_000, &targets, || 2_500),
        Ok(vec![
            MushroomEffect::BlastVisual {
                mushroom: MUSHROOM,
                centre: OWN,
            },
            MushroomEffect::LifeDrain {
                mushroom: MUSHROOM,
                target: 1,
                amount: 2_500,
            },
            MushroomEffect::Despawn { mushroom: MUSHROOM },
        ])
    );
}

#[test]
fn exported_helpers_and_manual_forget_are_covered_for_follow_up_wiring() {
    assert_eq!(MUSHROOM_KEY, "oteryn:creature.mushroom");
    assert!(is_mushroom_boss("oteryn:creature.bakragore"));
    assert!(!is_mushroom_boss("oteryn:creature.chagorz"));

    let mut owner = MushroomThinkOwner::default();
    owner.think(MUSHROOM, OWN, 1_000, &[], &BOSS);
    assert!(owner.forget(MUSHROOM));
    assert!(!owner.forget(MUSHROOM));
}
