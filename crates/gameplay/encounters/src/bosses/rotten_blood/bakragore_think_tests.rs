#![allow(
    clippy::expect_used,
    reason = "test-only assertions keep descriptive failures"
)]

use super::*;

fn position(x: i32, y: i32, floor: i16) -> RoomPosition {
    RoomPosition { x, y, floor }
}

fn fixture<'a>(players: &'a [EchoPlayer]) -> BakragoreThinkFacts<'a> {
    BakragoreThinkFacts {
        boss: BAKRAGORE_ROOM_CENTRE,
        mechanics_disabled: false,
        bloodjaw_spawn_paused: false,
        bloodjaw_target: 2,
        nearby_bloodjaws: 2,
        nearby_mushrooms: 4,
        nearby_pillars: 4,
        active_essences: [false; 4],
        players,
    }
}

#[test]
fn first_think_starts_the_cycle_and_writes_a_zero_essence_count_once() {
    let mut owner = BakragoreThinkOwner::default();
    let facts = fixture(&[]);
    assert_eq!(
        owner.think(&facts),
        vec![
            BakragoreEffect::StartTeleportCycle,
            BakragoreEffect::RunAgonyCheck {
                room_players: vec![]
            },
            BakragoreEffect::SetEssenceCount { count: 0 },
        ]
    );
    assert_eq!(
        owner.think(&facts),
        vec![BakragoreEffect::RunAgonyCheck {
            room_players: vec![]
        }]
    );
}

#[test]
fn summons_top_up_bloodjaws_mushrooms_and_pillars_within_twenty_tiles_only_while_enabled() {
    let mut owner = BakragoreThinkOwner::default();
    owner.think(&fixture(&[]));
    let mut facts = fixture(&[]);
    facts.bloodjaw_target = 3;
    facts.nearby_bloodjaws = 0;
    facts.nearby_mushrooms = 1;
    facts.nearby_pillars = 2;
    let effects = owner.think(&facts);
    let keys: Vec<_> = effects
        .iter()
        .filter_map(|effect| match effect {
            BakragoreEffect::Spawn {
                creature_key,
                around,
                offset_tiles,
            } => {
                assert_eq!(*around, BAKRAGORE_ROOM_CENTRE);
                assert_eq!(*offset_tiles, 1);
                Some(*creature_key)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        keys,
        vec![
            ELDER_BLOODJAW_KEY,
            ELDER_BLOODJAW_KEY,
            ELDER_BLOODJAW_KEY,
            MUSHROOM_KEY,
            MUSHROOM_KEY,
            MUSHROOM_KEY,
            PILLAR_KEY,
            PILLAR_KEY,
        ]
    );
    assert!(matches!(
        effects.last(),
        Some(BakragoreEffect::RunAgonyCheck { .. })
    ));

    facts.bloodjaw_spawn_paused = true;
    assert!(!owner.think(&facts).iter().any(|effect| matches!(
        effect,
        BakragoreEffect::Spawn {
            creature_key: ELDER_BLOODJAW_KEY,
            ..
        }
    )));
    facts.mechanics_disabled = true;
    assert!(owner.think(&facts).is_empty());

    assert!(counts_summon(
        BAKRAGORE_ROOM_CENTRE,
        position(33_063, 32_397, 15)
    ));
    assert!(!counts_summon(
        BAKRAGORE_ROOM_CENTRE,
        position(33_064, 32_397, 15)
    ));
    assert!(!counts_summon(
        BAKRAGORE_ROOM_CENTRE,
        position(33_043, 32_397, 14)
    ));
}

#[test]
fn the_agony_room_is_the_full_square_not_the_donor_strip() {
    for inside in [
        position(33_044, 32_398, 15),
        position(33_024, 32_378, 15),
        position(33_064, 32_418, 15),
        position(33_044, 32_410, 15),
    ] {
        assert!(in_agony_room(inside));
    }
    for outside in [
        position(33_065, 32_398, 15),
        position(33_044, 32_419, 15),
        position(33_044, 32_398, 14),
    ] {
        assert!(!in_agony_room(outside));
    }
    let players = [
        EchoPlayer {
            actor: 1,
            position: position(33_044, 32_410, 15),
            taint_level: 0,
        },
        EchoPlayer {
            actor: 2,
            position: position(33_070, 32_398, 15),
            taint_level: 0,
        },
    ];
    let mut owner = BakragoreThinkOwner::default();
    let effects = owner.think(&fixture(&players));
    assert!(effects.contains(&BakragoreEffect::RunAgonyCheck {
        room_players: vec![1]
    }));
}

#[test]
fn echoes_spawn_only_when_every_nearby_player_has_final_taint() {
    let players = [
        EchoPlayer {
            actor: 1,
            position: BAKRAGORE_ROOM_CENTRE,
            taint_level: 5,
        },
        EchoPlayer {
            actor: 2,
            position: position(33_058, 32_412, 15),
            taint_level: 9,
        },
    ];
    let mut facts = fixture(&players);
    facts.active_essences = [true, false, true, false];
    let mut owner = BakragoreThinkOwner::default();
    let effects = owner.think(&facts);
    let echo_start = effects
        .iter()
        .position(|effect| *effect == BakragoreEffect::SetEssenceCount { count: 2 })
        .expect("essence count effect");
    assert_eq!(
        &effects[echo_start + 1..],
        &[
            BakragoreEffect::SpawnEcho {
                creature_key: ESSENCES[0].echo_key,
                at: ESSENCES[0].echo_at
            },
            BakragoreEffect::ResetEssence {
                storage: ESSENCES[0].storage
            },
            BakragoreEffect::TransformItem {
                at: ESSENCES[0].flame_at,
                from: 43_984,
                to: 43_983
            },
            BakragoreEffect::TransformItem {
                at: ESSENCES[0].pillar_at,
                from: 43_508,
                to: 43_507
            },
            BakragoreEffect::SpawnEcho {
                creature_key: ESSENCES[2].echo_key,
                at: ESSENCES[2].echo_at
            },
            BakragoreEffect::ResetEssence {
                storage: ESSENCES[2].storage
            },
            BakragoreEffect::TransformItem {
                at: ESSENCES[2].flame_at,
                from: 43_988,
                to: 43_987
            },
            BakragoreEffect::TransformItem {
                at: ESSENCES[2].pillar_at,
                from: 43_508,
                to: 43_507
            },
        ]
    );
}

#[test]
fn echoes_are_withheld_but_essences_still_reset_without_final_taint_or_players() {
    let assert_reset_without_echo = |players: &[EchoPlayer]| {
        let mut facts = fixture(players);
        facts.active_essences = [true, false, true, false];
        let effects = BakragoreThinkOwner::default().think(&facts);
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, BakragoreEffect::SpawnEcho { .. }))
        );
        assert_eq!(
            effects
                .iter()
                .filter(|effect| matches!(effect, BakragoreEffect::ResetEssence { .. }))
                .count(),
            2
        );
        assert_eq!(
            effects
                .iter()
                .filter(|effect| matches!(effect, BakragoreEffect::TransformItem { .. }))
                .count(),
            4
        );
    };
    assert_reset_without_echo(&[EchoPlayer {
        actor: 1,
        position: BAKRAGORE_ROOM_CENTRE,
        taint_level: 4,
    }]);
    assert_reset_without_echo(&[]);

    let players = [
        EchoPlayer {
            actor: 1,
            position: position(33_059, 32_397, 15),
            taint_level: 4,
        },
        EchoPlayer {
            actor: 2,
            position: BAKRAGORE_ROOM_CENTRE,
            taint_level: 6,
        },
    ];
    let mut facts = fixture(&players);
    facts.active_essences = [true, false, true, false];
    let effects = BakragoreThinkOwner::default().think(&facts);
    assert_eq!(
        effects
            .iter()
            .filter(|effect| matches!(effect, BakragoreEffect::SpawnEcho { .. }))
            .count(),
        2
    );
}

#[test]
fn essence_table_matches_the_donor() {
    assert_eq!(
        ESSENCES[1],
        Essence {
            storage: "essenceOfIchgahal",
            echo_key: "oteryn:creature.echo_of_ichgahal",
            echo_at: position(33_044, 32_393, 15),
            flame_at: position(33_076, 32_389, 15),
            flame_item: 43_498,
            flame_reset_item: 43_497,
            pillar_at: position(33_076, 32_395, 15),
            pillar_item: 43_508,
            pillar_reset_item: 43_507,
        }
    );
    assert_eq!(
        ESSENCES[3],
        Essence {
            storage: "essenceOfVemiath",
            echo_key: "oteryn:creature.echo_of_vemiath",
            echo_at: position(33_039, 32_396, 15),
            flame_at: position(33_076, 32_407, 15),
            flame_item: 43_986,
            flame_reset_item: 43_985,
            pillar_at: position(33_076, 32_401, 15),
            pillar_item: 43_508,
            pillar_reset_item: 43_507,
        }
    );
}
