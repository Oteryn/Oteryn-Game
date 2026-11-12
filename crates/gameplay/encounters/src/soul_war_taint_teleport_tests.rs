use super::*;
use oteryn_gameplay_quests::quest::soul_war::TAINT_DURATION_SECONDS;

const MONSTER: u64 = 90;
const PLAYER: u64 = 7;
const BRACHIODEMON: &str = "oteryn:creature.brachiodemon";

const fn at(x: i32, y: i32, floor: i16) -> RoomPosition {
    RoomPosition { x, y, floor }
}

fn candidate(actor: u64, position: RoomPosition) -> TeleportCandidate {
    TeleportCandidate {
        actor,
        position,
        has_teleport_taint: true,
        in_boss_zone: false,
    }
}

#[test]
fn six_soul_war_monsters_carry_the_crystal_teleport_lines() {
    assert!(
        TAINT_TELEPORT_MONSTERS
            .windows(2)
            .all(|pair| pair[0].0 < pair[1].0)
    );
    assert_eq!(teleport_line(BRACHIODEMON), Some("Burn in hell!"));
    assert_eq!(teleport_line("oteryn:creature.goshnar_s_greed"), None);
}

#[test]
fn teleport_taint_is_the_first_unexpired_soul_war_taint() {
    let state = SoulWarState {
        teleport_access: true,
        defeated_bosses: 1,
        first_taint_at: Some(0),
        ..SoulWarState::default()
    };
    assert!(has_teleport_taint(state, 10));
    assert!(!has_teleport_taint(state, TAINT_DURATION_SECONDS));
    assert!(!has_teleport_taint(SoulWarState::default(), 10));
}

#[test]
fn hunting_zones_match_crystal_boxes_minus_safe_areas() {
    for position in [
        at(33_990, 31_000, 10),
        at(33_933, 31_010, 9),
        at(33_854, 31_839, 4),
        at(33_962, 30_991, 12),
        at(33_934, 31_165, 12),
    ] {
        assert!(in_soul_war_hunting_zone(position));
    }
    for position in [
        at(34_010, 31_010, 9),
        at(33_933, 31_010, 7),
        at(33_744, 31_636, 14),
    ] {
        assert!(!in_soul_war_hunting_zone(position));
    }
}

#[test]
fn only_rolls_of_ten_or_less_attempt_a_teleport() {
    let monster_position = at(33_933, 31_010, 9);
    let player = candidate(PLAYER, at(33_938, 31_010, 9));
    let mut owner = TaintTeleportOwner::default();
    assert_eq!(
        owner.think(MONSTER, BRACHIODEMON, monster_position, 0, &[player], || 11),
        Ok(Vec::new())
    );
    assert!(!owner.cooldown_active(PLAYER));
    let effects = owner
        .think(MONSTER, BRACHIODEMON, monster_position, 0, &[player], || 10)
        .unwrap_or_default();
    assert_eq!(effects.len(), 2);
    assert!(effects.iter().all(|effect| matches!(
        effect,
        TaintTeleportEffect::Magic {
            effect: TaintTeleportMagic::MortArea,
            ..
        }
    )));
    assert!(owner.cooldown_active(PLAYER));

    for roll in [0, 101] {
        assert_eq!(
            TaintTeleportOwner::default().think(
                MONSTER,
                BRACHIODEMON,
                monster_position,
                0,
                &[player],
                || roll
            ),
            Err(TaintTeleportRefusal::InvalidRoll)
        );
    }
    assert_eq!(
        TaintTeleportOwner::default().think(
            MONSTER,
            "unknown",
            monster_position,
            0,
            &[player],
            || 0
        ),
        Ok(Vec::new())
    );
}

#[test]
fn farthest_qualifying_player_on_the_floor_is_chosen_ties_to_the_first() {
    let monster_position = at(33_933, 31_010, 9);
    let mut candidates = [
        candidate(1, at(33_936, 31_010, 9)),
        candidate(2, at(33_940, 31_010, 9)),
        candidate(3, at(33_933, 31_017, 9)),
        candidate(4, at(33_953, 31_010, 9)),
        candidate(5, at(33_964, 31_010, 9)),
        candidate(6, at(33_958, 31_010, 8)),
        candidate(7, at(33_933, 30_990, 9)),
    ];
    candidates[3].has_teleport_taint = false;
    let effects = TaintTeleportOwner::default()
        .think(
            MONSTER,
            BRACHIODEMON,
            monster_position,
            0,
            &candidates,
            || 1,
        )
        .unwrap_or_default();
    assert_eq!(
        effects.get(1),
        Some(&TaintTeleportEffect::Magic {
            position: candidates[1].position,
            effect: TaintTeleportMagic::MortArea,
        })
    );
    assert_eq!(
        TaintTeleportOwner::default().think(
            MONSTER,
            BRACHIODEMON,
            monster_position,
            0,
            &[candidate(1, monster_position)],
            || 1
        ),
        Ok(Vec::new())
    );
}

#[test]
fn the_teleport_lands_after_two_seconds_on_the_players_current_tile() {
    let old_monster = at(33_933, 31_010, 9);
    let initial_player = at(33_938, 31_010, 9);
    let current_player = at(33_940, 31_010, 9);
    let mut owner = TaintTeleportOwner::default();
    let _ = owner.think(
        MONSTER,
        BRACHIODEMON,
        old_monster,
        1_000,
        &[candidate(PLAYER, initial_player)],
        || 1,
    );
    assert!(owner.advance(2_999, |_, _| None, |_| true).is_empty());
    let facts = DueFacts {
        monster_position: old_monster,
        player_position: current_player,
        destination_allows_teleport: true,
    };
    assert_eq!(
        owner.advance(3_000, |_, _| Some(facts), |_| true),
        vec![
            TaintTeleportEffect::Say {
                monster: MONSTER,
                text: "Burn in hell!",
            },
            TaintTeleportEffect::Magic {
                position: old_monster,
                effect: TaintTeleportMagic::Teleport,
            },
            TaintTeleportEffect::Teleport {
                monster: MONSTER,
                destination: current_player,
            },
            TaintTeleportEffect::Magic {
                position: current_player,
                effect: TaintTeleportMagic::Teleport,
            },
        ]
    );

    for due_facts in [
        Some(DueFacts {
            destination_allows_teleport: false,
            ..facts
        }),
        None,
    ] {
        let mut blocked = TaintTeleportOwner::default();
        let _ = blocked.think(
            MONSTER,
            BRACHIODEMON,
            old_monster,
            1_000,
            &[candidate(PLAYER, initial_player)],
            || 1,
        );
        assert!(
            blocked
                .advance(3_000, |_, _| due_facts, |_| true)
                .is_empty()
        );
        assert!(
            blocked
                .advance(4_000, |_, _| Some(facts), |_| true)
                .is_empty()
        );
    }
}

#[test]
fn the_cooldown_blocks_a_second_monster_until_ten_seconds_and_survives_offline_players() {
    let monster_position = at(33_933, 31_010, 9);
    let player = candidate(PLAYER, at(33_938, 31_010, 9));
    let mut owner = TaintTeleportOwner::default();
    let _ = owner.think(
        MONSTER,
        BRACHIODEMON,
        monster_position,
        1_000,
        &[player],
        || 1,
    );
    assert_eq!(
        owner.think(91, BRACHIODEMON, monster_position, 5_000, &[player], || 1),
        Ok(Vec::new())
    );
    let _ = owner.advance(11_000, |_, _| None, |_| true);
    assert!(!owner.cooldown_active(PLAYER));
    assert_eq!(
        owner
            .think(91, BRACHIODEMON, monster_position, 11_000, &[player], || 1)
            .unwrap_or_default()
            .len(),
        2
    );

    let mut offline = TaintTeleportOwner::default();
    let _ = offline.think(
        MONSTER,
        BRACHIODEMON,
        monster_position,
        1_000,
        &[player],
        || 1,
    );
    let _ = offline.advance(20_000, |_, _| None, |_| false);
    assert!(offline.cooldown_active(PLAYER));
}
