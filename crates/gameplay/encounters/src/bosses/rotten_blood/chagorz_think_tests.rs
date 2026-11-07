#![allow(
    clippy::expect_used,
    reason = "test-only assertions keep descriptive failures"
)]

use super::*;

fn player(actor: u64, position: RoomPosition) -> AgonyPlayer {
    AgonyPlayer {
        actor,
        position,
        max_health: 1_000,
    }
}

fn offset(position: RoomPosition, x: i32, floor: i16) -> RoomPosition {
    RoomPosition {
        x: position.x + x,
        y: position.y,
        floor,
    }
}

#[test]
fn standing_still_builds_agony_stacks_and_damage_tiers_up_to_twenty_one() {
    let mut agony = AgonyStacks::default();
    let player = player(1, offset(CHAGORZ_ROOM_CENTRE, 2, 15));

    for check in 1_u8..=22 {
        let count = check.min(AGONY_STACK_CAP);
        let mut expected = Vec::new();
        if check == 1 {
            expected.push(AgonyEffect::SetStacks { actor: 1, count: 0 });
        }
        expected.push(AgonyEffect::SetStacks { actor: 1, count });
        expected.push(AgonyEffect::Damage(AgonyDamage {
            actor: 1,
            amount: agony_percent(count) * 10,
        }));
        assert_eq!(
            agony.check(CHAGORZ_ROOM_CENTRE, AGONY_RANGE_TILES, &[player]),
            expected
        );
    }
}

#[test]
fn moving_lowers_stacks_by_one_without_damage() {
    let mut agony = AgonyStacks::default();
    let first = offset(CHAGORZ_ROOM_CENTRE, 2, 15);
    for _ in 0..3 {
        agony.check(CHAGORZ_ROOM_CENTRE, AGONY_RANGE_TILES, &[player(1, first)]);
    }
    let second = offset(CHAGORZ_ROOM_CENTRE, 3, 15);
    assert_eq!(
        agony.check(CHAGORZ_ROOM_CENTRE, AGONY_RANGE_TILES, &[player(1, second)]),
        vec![AgonyEffect::SetStacks { actor: 1, count: 2 }]
    );
    let third = offset(CHAGORZ_ROOM_CENTRE, 4, 15);
    assert_eq!(
        agony.check(CHAGORZ_ROOM_CENTRE, AGONY_RANGE_TILES, &[player(1, third)]),
        vec![AgonyEffect::SetStacks { actor: 1, count: 1 }]
    );
    assert_eq!(
        agony.check(CHAGORZ_ROOM_CENTRE, AGONY_RANGE_TILES, &[player(1, third)]),
        vec![
            AgonyEffect::SetStacks { actor: 1, count: 2 },
            AgonyEffect::Damage(AgonyDamage {
                actor: 1,
                amount: 30,
            }),
        ]
    );

    let mut zero = AgonyStacks::default();
    zero.players.insert(1, (first, 0));
    assert_eq!(
        zero.check(CHAGORZ_ROOM_CENTRE, AGONY_RANGE_TILES, &[player(1, second)]),
        vec![AgonyEffect::SetStacks { actor: 1, count: 0 }]
    );
}

#[test]
fn agony_ignores_players_beyond_sixteen_tiles_or_on_another_floor() {
    let mut agony = AgonyStacks::default();
    let players = [
        player(1, offset(CHAGORZ_ROOM_CENTRE, 16, 15)),
        player(2, offset(CHAGORZ_ROOM_CENTRE, 17, 15)),
        player(3, offset(CHAGORZ_ROOM_CENTRE, 0, 14)),
    ];
    let effects = agony.check(CHAGORZ_ROOM_CENTRE, AGONY_RANGE_TILES, &players);

    assert_eq!(effects.len(), 3);
    assert_eq!(agony.stacks(1), Some(1));
    assert_eq!(agony.stacks(2), None);
    assert_eq!(agony.stacks(3), None);
}

#[test]
fn agony_percent_tiers() {
    assert_eq!(
        [0, 1, 14, 15, 19, 20, 21].map(agony_percent),
        [0, 3, 3, 7, 7, 15, 15]
    );
    let mut agony = AgonyStacks::default();
    agony.players.insert(1, (CHAGORZ_ROOM_CENTRE, 1));
    agony.players.insert(2, (CHAGORZ_ROOM_CENTRE, 1));
    agony.retain_present(&[1]);
    assert_eq!(agony.stacks(1), Some(1));
    assert_eq!(agony.stacks(2), None);
}

#[test]
fn teleport_cycle_runs_every_ninety_seconds_only_with_a_bloodjaw_and_stops_without_the_boss() {
    let mut cycle = TeleportCycle::new(CHAGORZ_ROOM_CENTRE);
    cycle.start(0);
    assert_eq!(cycle.due(89_999, true, false, true), None);
    assert_eq!(
        cycle.due(90_000, true, false, true),
        Some(CentreTeleport {
            centre: CHAGORZ_ROOM_CENTRE,
            item_id: TELEPORT_ITEM_ID,
            action_id: TELEPORT_ACTION_ID,
            remove_at_ms: 105_000,
        })
    );
    assert_eq!(cycle.next_at_ms(), Some(180_000));
    assert_eq!(cycle.due(180_000, true, false, false), None);
    assert_eq!(cycle.next_at_ms(), Some(270_000));
    assert_eq!(cycle.due(270_000, true, true, true), None);
    assert_eq!(cycle.next_at_ms(), Some(360_000));
    assert_eq!(cycle.due(360_000, false, false, true), None);
    assert_eq!(cycle.next_at_ms(), None);
    assert_eq!(cycle.due(450_000, true, false, true), None);
}

#[test]
fn chagorz_think_starts_the_cycle_once_and_tops_up_summons_only_while_enabled() {
    let mut owner = ChagorzThinkOwner::default();
    let facts = ChagorzThinkFacts {
        boss: CHAGORZ_ROOM_CENTRE,
        now_ms: 10,
        mechanics_disabled: false,
        bloodjaw_spawn_paused: false,
        bloodjaw_target: 2,
        nearby_bloodjaws: 0,
        nearby_pillars: 1,
        players: &[],
    };
    let bloodjaw = ChagorzEffect::Spawn {
        creature_key: ELDER_BLOODJAW_KEY,
        around: CHAGORZ_ROOM_CENTRE,
        placement: Placement::Close {
            offset_tiles: BLOODJAW_OFFSET_TILES,
        },
    };
    let pillar = ChagorzEffect::Spawn {
        creature_key: PILLAR_KEY,
        around: CHAGORZ_ROOM_CENTRE,
        placement: Placement::Close {
            offset_tiles: PILLAR_OFFSET_TILES,
        },
    };
    assert_eq!(
        owner.think(&facts),
        vec![
            ChagorzEffect::StartTeleportCycle,
            bloodjaw,
            bloodjaw,
            pillar,
            pillar,
            pillar,
        ]
    );
    assert_eq!(owner.teleport_mut().next_at_ms(), Some(90_010));
    assert_eq!(
        owner.think(&facts),
        vec![bloodjaw, bloodjaw, pillar, pillar, pillar]
    );
    let paused = ChagorzThinkFacts {
        bloodjaw_spawn_paused: true,
        ..facts
    };
    assert_eq!(owner.think(&paused), vec![pillar, pillar, pillar]);

    let player = player(1, CHAGORZ_ROOM_CENTRE);
    let disabled = ChagorzThinkFacts {
        mechanics_disabled: true,
        players: &[player],
        ..facts
    };
    assert!(owner.think(&disabled).is_empty());
    assert_eq!(owner.agony().stacks(1), None);
    assert_eq!(CHAGORZ_KEY, "oteryn:creature.chagorz");
    assert_eq!(PILLAR_COUNT_RANGE_TILES, 16);
}

#[test]
fn paused_global_portal_reschedules_before_the_current_boss_presence_check() {
    // Crystal chagorz.lua:224-229 (and Vemiath/Bakragore equivalents):
    // the Lua paused branch returns after rescheduling, before spectator lookup.
    let mut cycle = TeleportCycle::new(CHAGORZ_ROOM_CENTRE);
    cycle.start(0);
    assert_eq!(cycle.due(90_000, false, true, false), None);
    assert_eq!(cycle.next_at_ms(), Some(180_000));
    assert_eq!(
        cycle.due(180_000, true, false, true).map(|p| p.item_id),
        Some(37000)
    );
}
