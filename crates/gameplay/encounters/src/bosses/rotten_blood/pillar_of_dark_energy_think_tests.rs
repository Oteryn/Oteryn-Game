#![allow(
    clippy::expect_used,
    reason = "test-only assertions keep descriptive failures"
)]

use super::*;

fn own_position() -> RoomPosition {
    RoomPosition {
        x: 33_043,
        y: 32_366,
        floor: 15,
    }
}

#[test]
fn a_pillar_expires_on_its_tenth_think() {
    let own = own_position();
    let chagorz = [("oteryn:creature.chagorz", own)];
    let mut owner = PillarThinkOwner::default();

    for _ in 1..=9 {
        assert_eq!(owner.think(7, own, &chagorz), PillarThink::Stay);
    }
    assert_eq!(owner.thinks(7), Some(9));

    assert_eq!(owner.think(7, own, &chagorz), PillarThink::Expired);
    assert_eq!(owner.thinks(7), None);

    assert_eq!(owner.think(7, own, &chagorz), PillarThink::Stay);
    assert_eq!(owner.thinks(7), Some(1));
}

#[test]
fn a_pillar_without_a_summoner_in_fifteen_tiles_is_removed() {
    let own = own_position();
    let mut owner = PillarThinkOwner::default();
    let at_fifteen = [(
        "oteryn:creature.vemiath",
        RoomPosition {
            x: own.x + 15,
            ..own
        },
    )];
    let at_sixteen = [(
        "oteryn:creature.vemiath",
        RoomPosition {
            x: own.x + 16,
            ..own
        },
    )];
    let ichgahal = [("oteryn:creature.ichgahal", own)];
    let other_floor = [(
        "oteryn:creature.bakragore",
        RoomPosition { floor: 14, ..own },
    )];

    assert_eq!(owner.think(7, own, &at_fifteen), PillarThink::Stay);
    assert_eq!(owner.think(7, own, &at_sixteen), PillarThink::BossGone);
    assert_eq!(owner.think(7, own, &ichgahal), PillarThink::BossGone);
    assert_eq!(owner.think(7, own, &other_floor), PillarThink::BossGone);
    assert_eq!(owner.think(7, own, &[]), PillarThink::BossGone);
    assert_eq!(owner.thinks(7), None);
}

#[test]
fn forget_drops_the_count_and_boss_list_is_sorted() {
    let own = own_position();
    let chagorz = [("oteryn:creature.chagorz", own)];
    let mut owner = PillarThinkOwner::default();

    for _ in 0..3 {
        assert_eq!(owner.think(7, own, &chagorz), PillarThink::Stay);
    }

    assert!(owner.forget(7));
    assert!(!owner.forget(7));
    assert!(PILLAR_BOSSES.is_sorted());
    assert!(!is_pillar_boss("oteryn:creature.murcion"));
}
