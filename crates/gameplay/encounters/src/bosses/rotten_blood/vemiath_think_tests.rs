#![allow(
    clippy::expect_used,
    reason = "test-only assertions keep descriptive failures"
)]

use super::*;

fn fixture() -> VemiathThinkFacts {
    VemiathThinkFacts {
        boss: VEMIATH_ROOM_CENTRE,
        mechanics_disabled: false,
        bloodjaw_spawn_paused: false,
        bloodjaw_target: 2,
        nearby_bloodjaws: 0,
        nearby_pillars: 0,
    }
}

#[test]
fn first_vemiath_think_starts_the_cycle_and_tops_up_bloodjaws_and_pillars_with_one_tile_offsets() {
    let boss = VEMIATH_ROOM_CENTRE;
    let mut owner = VemiathThinkOwner::default();
    let expected = vec![
        VemiathEffect::StartTeleportCycle,
        VemiathEffect::Spawn {
            creature_key: ELDER_BLOODJAW_KEY,
            around: boss,
            offset_tiles: 1,
        },
        VemiathEffect::Spawn {
            creature_key: ELDER_BLOODJAW_KEY,
            around: boss,
            offset_tiles: 1,
        },
        VemiathEffect::Spawn {
            creature_key: PILLAR_KEY,
            around: boss,
            offset_tiles: 1,
        },
        VemiathEffect::Spawn {
            creature_key: PILLAR_KEY,
            around: boss,
            offset_tiles: 1,
        },
        VemiathEffect::Spawn {
            creature_key: PILLAR_KEY,
            around: boss,
            offset_tiles: 1,
        },
        VemiathEffect::Spawn {
            creature_key: PILLAR_KEY,
            around: boss,
            offset_tiles: 1,
        },
        VemiathEffect::RunAgonyCheck,
    ];

    assert_eq!(owner.think(&fixture()), expected);
    assert!(
        !owner
            .think(&fixture())
            .contains(&VemiathEffect::StartTeleportCycle)
    );
}

#[test]
fn disabled_mechanics_still_summon_pillars_but_no_bloodjaws_or_agony() {
    let mut owner = VemiathThinkOwner::default();
    let _ = owner.think(&fixture());
    let facts = VemiathThinkFacts {
        mechanics_disabled: true,
        nearby_pillars: 3,
        ..fixture()
    };

    assert_eq!(
        owner.think(&facts),
        vec![VemiathEffect::Spawn {
            creature_key: PILLAR_KEY,
            around: VEMIATH_ROOM_CENTRE,
            offset_tiles: 1,
        }]
    );
}

#[test]
fn paused_bloodjaw_spawn_keeps_pillars_and_agony() {
    let mut owner = VemiathThinkOwner::default();
    let _ = owner.think(&fixture());
    let facts = VemiathThinkFacts {
        bloodjaw_spawn_paused: true,
        ..fixture()
    };
    let effects = owner.think(&facts);

    assert_eq!(effects.len(), usize::from(PILLAR_FLOOR) + 1);
    assert!(effects[..usize::from(PILLAR_FLOOR)].iter().all(|effect| {
        *effect
            == VemiathEffect::Spawn {
                creature_key: PILLAR_KEY,
                around: VEMIATH_ROOM_CENTRE,
                offset_tiles: 1,
            }
    }));
    assert_eq!(effects.last(), Some(&VemiathEffect::RunAgonyCheck));
}

#[test]
fn summons_are_counted_within_seventeen_tiles() {
    let boss = VEMIATH_ROOM_CENTRE;

    assert!(counts_summon(
        boss,
        RoomPosition {
            x: boss.x + 17,
            y: boss.y - 17,
            floor: boss.floor,
        }
    ));
    assert!(!counts_summon(
        boss,
        RoomPosition {
            x: boss.x + 18,
            ..boss
        }
    ));
    assert!(!counts_summon(boss, RoomPosition { floor: 14, ..boss }));
}
