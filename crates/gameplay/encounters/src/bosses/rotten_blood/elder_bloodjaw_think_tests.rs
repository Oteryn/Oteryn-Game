#![allow(
    clippy::expect_used,
    reason = "test-only assertions keep descriptive failures"
)]

use super::*;

const OWN: RoomPosition = RoomPosition {
    x: 33_000,
    y: 32_000,
    floor: 15,
};

fn candidate(
    actor: u64,
    dx: i32,
    floor: i16,
    vocation: Vocation,
    health: u32,
) -> BloodjawCandidate {
    BloodjawCandidate {
        actor,
        position: RoomPosition {
            x: OWN.x + dx,
            y: OWN.y,
            floor,
        },
        vocation,
        health,
        max_health: 100,
    }
}

fn monster(creature_key: &str, dx: i32, floor: i16) -> NearbyMonster<'_> {
    NearbyMonster {
        creature_key,
        position: RoomPosition {
            x: OWN.x + dx,
            y: OWN.y,
            floor,
        },
    }
}

#[test]
fn bloodjaw_target_count_follows_the_crystal_taint_steps() {
    for (taint, expected) in [(0, 2), (4, 2), (5, 3), (7, 3), (8, 4), (9, 5), (15, 5)] {
        assert_eq!(bloodjaw_target_count(taint), expected);
    }
}

#[test]
fn vocation_priority_matches_the_monster_ai_table() {
    for vocation in [Vocation::Knight, Vocation::EliteKnight] {
        assert_eq!(vocation_priority(vocation), 100);
    }
    for vocation in [Vocation::Paladin, Vocation::RoyalPaladin] {
        assert_eq!(vocation_priority(vocation), 75);
    }
    for vocation in [
        Vocation::Sorcerer,
        Vocation::MasterSorcerer,
        Vocation::Druid,
        Vocation::ElderDruid,
        Vocation::Monk,
        Vocation::ExaltedMonk,
    ] {
        assert_eq!(vocation_priority(vocation), 70);
    }
    assert_eq!(vocation_priority(Vocation::None), 50);
    assert_eq!(health_percent(30, 100), 30);
    assert_eq!(health_percent(299, 1_000), 29);
    assert_eq!(health_percent(5, 0), 100);
}

#[test]
fn best_target_prefers_knights_then_low_health_within_ten_tiles() {
    let mut players = vec![
        candidate(1, 3, 15, Vocation::Druid, 100),
        candidate(2, 5, 15, Vocation::Knight, 100),
    ];
    assert_eq!(best_target(OWN, &players), Some(2));

    players.extend([
        candidate(3, 6, 15, Vocation::Paladin, 30),
        candidate(4, 11, 15, Vocation::Knight, 30),
        candidate(5, 1, 14, Vocation::Knight, 30),
    ]);
    assert_eq!(best_target(OWN, &players), Some(3));

    let tied = [
        candidate(6, 1, 15, Vocation::Knight, 100),
        candidate(7, 2, 15, Vocation::Knight, 100),
    ];
    assert_eq!(best_target(OWN, &tied), Some(6));
    assert_eq!(best_target(OWN, &[]), None);
}

#[test]
fn elder_bloodjaw_retargets_only_on_change_and_despawns_without_a_boss_in_fifteen_tiles() {
    let players = [
        candidate(1, 3, 15, Vocation::Druid, 100),
        candidate(2, 5, 15, Vocation::Knight, 100),
    ];
    let boss_in_range = [monster("oteryn:creature.ichgahal", 15, 15)];
    assert_eq!(
        elder_bloodjaw_think(OWN, Some(2), &players, &boss_in_range),
        ElderBloodjawThink {
            retarget: None,
            despawn: false,
        }
    );
    assert_eq!(
        elder_bloodjaw_think(OWN, Some(1), &players, &boss_in_range).retarget,
        Some(2)
    );

    for monsters in [
        vec![monster("oteryn:creature.ichgahal", 16, 15)],
        vec![monster("oteryn:creature.mushroom", 1, 15)],
        vec![monster("oteryn:creature.bakragore", 0, 14)],
        vec![],
    ] {
        assert!(elder_bloodjaw_think(OWN, None, &players, &monsters).despawn);
    }

    assert!(ROTTEN_BLOOD_BOSSES.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(is_rotten_blood_boss("oteryn:creature.vemiath"));
    assert!(!is_rotten_blood_boss(ELDER_BLOODJAW_KEY));
}
