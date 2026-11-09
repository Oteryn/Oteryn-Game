use super::*;

const SOUL_SPOT: RoomPosition = RoomPosition {
    x: 33750,
    y: 31666,
    floor: 14,
};

#[test]
fn srx3_greed_source_type_counter_is_shared_and_requires_exact_eight_seconds() {
    // Crystal 201189f weak_soul.lua:92-104 stores one closure counter per type.
    let mut room = GoshnarsGreedRoom::default();
    assert_eq!(
        room.minion_think(1, GreedMinion::WeakSoul, 4_000, 0, &[], || 500),
        Ok(vec![])
    );
    assert_eq!(
        room.minion_think(2, GreedMinion::WeakSoul, 4_000, 0, &[], || 500),
        Ok(vec![
            GreedEffect::SpawnMonster {
                creature_key: STRONG_SOUL,
                position: SOUL_SPOT,
                forced: false,
            },
            GreedEffect::RemoveMonster { instance: 2 },
        ])
    );
    // ==8000 is deliberate in the source; an overshoot does not transform.
    assert_eq!(
        room.minion_think(3, GreedMinion::WeakSoul, 5_000, 0, &[], || 500),
        Ok(vec![])
    );
    assert_eq!(
        room.minion_think(3, GreedMinion::WeakSoul, 5_000, 0, &[], || 500),
        Ok(vec![])
    );
    room.minion_spawned(GreedMinion::WeakSoul);
    assert_eq!(
        room.minion_think(4, GreedMinion::WeakSoul, 8_000, 0, &[], || 500),
        Ok(vec![
            GreedEffect::SpawnMonster {
                creature_key: STRONG_SOUL,
                position: SOUL_SPOT,
                forced: false,
            },
            GreedEffect::RemoveMonster { instance: 4 },
        ])
    );
}

#[test]
fn srx3_greed_activity_instances_isolate_their_source_type_counters() {
    // Accepted BOSS_RAID0 D26 / §6: one independently owned activity per party.
    let mut first = GoshnarsGreedRoom::default();
    let mut second = GoshnarsGreedRoom::default();
    assert_eq!(
        first.minion_think(1, GreedMinion::WeakSoul, 4_000, 0, &[], || 500),
        Ok(vec![])
    );
    assert_eq!(
        second.minion_think(2, GreedMinion::WeakSoul, 4_000, 0, &[], || 500),
        Ok(vec![])
    );
    assert_eq!(
        first.minion_think(3, GreedMinion::WeakSoul, 4_000, 0, &[], || 500),
        Ok(vec![
            GreedEffect::SpawnMonster {
                creature_key: STRONG_SOUL,
                position: SOUL_SPOT,
                forced: false,
            },
            GreedEffect::RemoveMonster { instance: 3 },
        ])
    );
    second.minion_spawned(GreedMinion::WeakSoul);
    assert_eq!(
        second.minion_think(4, GreedMinion::WeakSoul, 4_000, 0, &[], || 500),
        Ok(vec![])
    );
}

fn summon_sphere(room: &mut GoshnarsGreedRoom) -> Vec<GreedEffect> {
    let _ = room.boss_spawned();
    for instance in 1..=5 {
        room.minion_died(instance, GreedMinion::Greedbeast, 0);
    }
    think(room, 1_000)
}

/// Execute only the source engine's spawn-result protocol, like the physical
/// driver. The test controls createMonster success; it does not invent HP rules.
fn think(room: &mut GoshnarsGreedRoom, interval: u64) -> Vec<GreedEffect> {
    think_with_result(room, interval, Some(1))
}
fn think_with_result(
    room: &mut GoshnarsGreedRoom,
    interval: u64,
    result: Option<u64>,
) -> Vec<GreedEffect> {
    let mut pending = std::collections::VecDeque::from(room.boss_think(interval));
    let mut emitted = Vec::new();
    while let Some(effect) = pending.pop_front() {
        match effect {
            GreedEffect::SpawnMonster {
                creature_key: SOUL_SPHERE,
                ..
            } => {
                if let Some(id) = result {
                    room.sphere_spawned(id);
                } else {
                    room.sphere_spawn_failed();
                }
                emitted.push(effect);
            }
            GreedEffect::FinishBossThink { interval_ms } => {
                for effect in room.finish_boss_think(interval_ms).into_iter().rev() {
                    pending.push_front(effect);
                }
            }
            _ => emitted.push(effect),
        }
    }
    emitted
}

#[test]
fn greed_minions_map_to_crystal_spawn_positions() {
    let expected = [
        (
            GreedMinion::Greedbeast,
            RoomPosition {
                x: 33744,
                y: 31666,
                floor: 14,
            },
        ),
        (
            GreedMinion::Soulsnatcher,
            RoomPosition {
                x: 33747,
                y: 31668,
                floor: 14,
            },
        ),
        (GreedMinion::WeakSoul, SOUL_SPOT),
        (GreedMinion::StrongSoul, SOUL_SPOT),
        (GreedMinion::PowerfulSoul, SOUL_SPOT),
    ];
    for (minion, position) in expected {
        assert_eq!(
            GreedMinion::from_creature_key(minion.creature_key()),
            Some(minion)
        );
        assert_eq!(minion.spawn_position(), position);
    }
    assert_eq!(GreedMinion::from_creature_key(SOUL_SPHERE), None);
    assert_eq!(GOSHNARS_GREED, "oteryn:creature.goshnar_s_greed");
}

#[test]
fn lever_schedules_three_greed_spawns_with_teleport_effects_at_seven_eight_nine_seconds() {
    let mut room = GoshnarsGreedRoom::default();
    room.lever_pulled(0);
    assert_eq!(room.advance(6_999), vec![]);
    assert_eq!(
        room.advance(7_000),
        [
            GreedMinion::Greedbeast,
            GreedMinion::Soulsnatcher,
            GreedMinion::WeakSoul,
        ]
        .map(|minion| GreedEffect::TeleportMagic {
            position: minion.spawn_position()
        })
    );
    let later_teleports = room.advance(9_999);
    assert_eq!(later_teleports.len(), 6);
    assert!(
        later_teleports
            .iter()
            .all(|effect| matches!(effect, GreedEffect::TeleportMagic { .. }))
    );
    assert_eq!(
        room.advance(10_000),
        [
            GreedMinion::Greedbeast,
            GreedMinion::Soulsnatcher,
            GreedMinion::WeakSoul,
        ]
        .map(|minion| GreedEffect::SpawnMonster {
            creature_key: minion.creature_key(),
            position: minion.spawn_position(),
            forced: false,
        })
    );
}

#[test]
fn weak_and_strong_souls_grow_after_eight_seconds_of_thinks() {
    let mut room = GoshnarsGreedRoom::default();
    for _ in 0..7 {
        assert_eq!(
            room.minion_think(1, GreedMinion::WeakSoul, 1_000, 0, &[], || 500),
            Ok(vec![])
        );
    }
    assert_eq!(
        room.minion_think(1, GreedMinion::WeakSoul, 1_000, 0, &[], || 500),
        Ok(vec![
            GreedEffect::SpawnMonster {
                creature_key: STRONG_SOUL,
                position: SOUL_SPOT,
                forced: false,
            },
            GreedEffect::RemoveMonster { instance: 1 },
        ])
    );

    for _ in 0..7 {
        assert_eq!(
            room.minion_think(2, GreedMinion::StrongSoul, 1_000, 0, &[], || 500),
            Ok(vec![])
        );
    }
    assert_eq!(
        room.minion_think(2, GreedMinion::StrongSoul, 1_000, 0, &[], || 500),
        Ok(vec![
            GreedEffect::SpawnMonster {
                creature_key: POWERFUL_SOUL,
                position: SOUL_SPOT,
                forced: false,
            },
            GreedEffect::RemoveMonster { instance: 2 },
        ])
    );
    assert_eq!(
        room.minion_think(3, GreedMinion::WeakSoul, 5_000, 0, &[], || 500),
        Ok(vec![])
    );
    assert_eq!(
        room.minion_think(3, GreedMinion::WeakSoul, 5_000, 0, &[], || 500),
        Ok(vec![])
    );
}

#[test]
fn powerful_soul_buffs_greed_and_respawns_a_weak_soul_ten_seconds_later() {
    let mut room = GoshnarsGreedRoom::default();
    assert_eq!(
        room.minion_think(4, GreedMinion::PowerfulSoul, 8_000, 20, &[], || 500),
        Ok(vec![
            GreedEffect::RemoveMonster { instance: 4 },
            GreedEffect::BossBuff {
                reflect_percent_per_element: 10,
                defense: 10,
                max_health: 10_000,
                health: 10_000,
            },
        ])
    );
    assert!(room.advance(10_020).contains(&GreedEffect::SpawnMonster {
        creature_key: WEAK_SOUL,
        position: SOUL_SPOT,
        forced: false,
    }));
}

#[test]
fn soulsnatcher_drains_zone_players_and_rejects_bad_draws_atomically() {
    let mut room = GoshnarsGreedRoom::default();
    assert_eq!(
        room.minion_think(5, GreedMinion::Soulsnatcher, 8_000, 50, &[7, 8], || 750),
        Ok(vec![
            GreedEffect::PlayerHealthLoss {
                actor: 7,
                amount: 750,
            },
            GreedEffect::PlayerHealthLoss {
                actor: 8,
                amount: 750,
            },
            GreedEffect::RemoveMonster { instance: 5 },
        ])
    );
    assert!(room.advance(10_050).contains(&GreedEffect::SpawnMonster {
        creature_key: SOULSNATCHER,
        position: GreedMinion::Soulsnatcher.spawn_position(),
        forced: false,
    }));

    let mut refused = GoshnarsGreedRoom::default();
    for _ in 0..7 {
        assert_eq!(
            refused.minion_think(6, GreedMinion::Soulsnatcher, 1_000, 0, &[7], || 750),
            Ok(vec![])
        );
    }
    let before = format!("{refused:?}");
    assert_eq!(
        refused.minion_think(6, GreedMinion::Soulsnatcher, 1_000, 0, &[7], || 1_001),
        Err(GreedRefusal::InvalidDamageDraw)
    );
    assert_eq!(format!("{refused:?}"), before);
}

#[test]
fn minion_deaths_respawn_and_five_greedbeast_kills_summon_the_sphere() {
    let mut room = GoshnarsGreedRoom::default();
    assert_eq!(
        room.boss_spawned(),
        vec![GreedEffect::BossImmune { immune: true }]
    );
    for instance in 1..=4 {
        room.minion_died(instance, GreedMinion::Greedbeast, 0);
    }
    assert_eq!(think(&mut room, 1_000), vec![]);
    room.minion_died(5, GreedMinion::Greedbeast, 0);
    assert_eq!(think(&mut room, 1_000), sphere_summon_effects());
    let spawns = room
        .advance(10_000)
        .into_iter()
        .filter(|effect| {
            matches!(
                effect,
                GreedEffect::SpawnMonster {
                    creature_key: GREEDBEAST,
                    ..
                }
            )
        })
        .count();
    assert_eq!(spawns, 5);
}

#[test]
fn killing_the_sphere_releases_greed_on_the_next_think() {
    let mut room = GoshnarsGreedRoom::default();
    assert_eq!(summon_sphere(&mut room), sphere_summon_effects());
    room.sphere_died(99);
    assert_eq!(think(&mut room, 1_000), release_effects());
}

#[test]
fn greed_is_released_after_thirty_four_seconds_of_sphere_immunity() {
    let mut room = GoshnarsGreedRoom::default();
    assert_eq!(summon_sphere(&mut room), sphere_summon_effects());
    for _ in 0..32 {
        assert_eq!(think(&mut room, 1_000), vec![]);
    }
    assert_eq!(think(&mut room, 1_000), release_effects());
}

#[test]
fn soul_sphere_walks_west_every_three_seconds_and_heals_greed_once_on_contact() {
    let mut room = GoshnarsGreedRoom::default();
    let start = SPHERE_SPAWN;
    assert_eq!(room.sphere_think(99, 1_000, start, false), vec![]);
    assert_eq!(room.sphere_think(99, 1_000, start, false), vec![]);
    let moved = RoomPosition {
        x: 33751,
        y: 31659,
        floor: 14,
    };
    assert_eq!(
        room.sphere_think(99, 1_000, start, false),
        vec![GreedEffect::SphereMove {
            instance: 99,
            position: moved,
        }]
    );
    assert_eq!(room.sphere_think(99, 2_000, moved, true), vec![]);
    assert_eq!(
        room.sphere_think(99, 1_000, moved, true),
        vec![GreedEffect::BossHealFull]
    );
    assert_eq!(room.sphere_think(99, 10_000, moved, false), vec![]);
}

fn sphere_summon_effects() -> Vec<GreedEffect> {
    vec![
        GreedEffect::BossImmune { immune: true },
        GreedEffect::BossTeleport {
            position: BOSS_SPHERE_SPOT,
        },
        GreedEffect::BossSpeed { speed: 0 },
        GreedEffect::SpawnMonster {
            creature_key: SOUL_SPHERE,
            position: SPHERE_SPAWN,
            forced: true,
        },
    ]
}

fn release_effects() -> Vec<GreedEffect> {
    vec![
        GreedEffect::RemoveSoulSphere { instance: Some(1) },
        GreedEffect::BossImmune { immune: false },
        GreedEffect::BossTeleport {
            position: BOSS_HOME,
        },
        GreedEffect::BossSpeed { speed: BOSS_SPEED },
    ]
}

#[test]
fn srx3_failed_sphere_creation_does_not_release_immunity_at_thirty_four_seconds() {
    // Crystal goshnars_greed.lua:161,165-175: timeout is inside `if createdSoulSphere`.
    let mut room = GoshnarsGreedRoom::default();
    assert_eq!(summon_sphere(&mut room), sphere_summon_effects());
    room.sphere_spawn_failed();
    assert!(think(&mut room, 100_000).is_empty());
    // The retained source counter makes a later successful creation expire
    // immediately in this SAME callback; it is not a fresh34second phase.
    for instance in 10..15 {
        room.minion_died(instance, GreedMinion::Greedbeast, 0);
    }
    let mut expected = sphere_summon_effects();
    expected.extend(release_effects());
    assert_eq!(think(&mut room, 1_000), expected);
    assert!(think(&mut room, 33_000).is_empty());
}

#[test]
fn srx3_greed_pending_callback_deadline_tracks_real_source_work_after_no_thinks() {
    let mut room = GoshnarsGreedRoom::default();
    assert_eq!(room.next_due_ms(), None);
    room.lever_pulled(123);
    // The helper warns at7/8/9s before spawning at10s, for all three children.
    assert_eq!(room.next_due_ms(), Some(7_123));
    assert!(room.advance(7_122).is_empty());
    assert_eq!(room.next_due_ms(), Some(7_123));
    assert_eq!(room.advance(7_123).len(), 3);
    assert_eq!(room.next_due_ms(), Some(8_123));
    assert_eq!(room.advance(10_123).len(), 9);
    assert_eq!(room.next_due_ms(), None);
    room.minion_died(1, GreedMinion::Soulsnatcher, 20_000);
    assert_eq!(room.next_due_ms(), Some(27_000));
    assert_eq!(room.advance(30_000).len(), 4);
    assert_eq!(room.next_due_ms(), None);
}

#[test]
fn srx3_greed_spheres_share_source_type_movement_counter() {
    // Crystal soul_sphere.lua:90-124 closes over ONE counter for the type.
    let mut room = GoshnarsGreedRoom::default();
    assert!(room.sphere_think(1, 1000, SPHERE_SPAWN, false).is_empty());
    assert!(room.sphere_think(2, 1000, SPHERE_SPAWN, false).is_empty());
    assert_eq!(
        room.sphere_think(1, 1000, SPHERE_SPAWN, false),
        vec![GreedEffect::SphereMove {
            instance: 1,
            position: RoomPosition {
                x: SPHERE_SPAWN.x - 1,
                ..SPHERE_SPAWN
            },
        }]
    );
}

#[test]
fn srx3_greed_one_sphere_reaching_boss_stops_every_source_type_actor() {
    let mut room = GoshnarsGreedRoom::default();
    assert_eq!(
        room.sphere_think(1, 3000, SPHERE_SPAWN, true),
        vec![GreedEffect::BossHealFull]
    );
    assert!(
        room.sphere_think(2, 3000, SPHERE_SPAWN, false).is_empty(),
        "shared source stop flag remains until a successful sphere spawn resets it"
    );
}

#[test]
fn srx3_greed_overlapping_sphere_creation_keeps_existing_immunity_counter() {
    let mut room = GoshnarsGreedRoom::default();
    for id in 0..5 {
        room.minion_died(id, GreedMinion::Greedbeast, 0);
    }
    let _ = think(&mut room, 1000);
    for _ in 0..32 {
        assert!(think(&mut room, 1000).is_empty());
    }
    // Wall-clock replacements can run while the boss's native think is idle.
    for id in 5..10 {
        room.minion_died(id, GreedMinion::Greedbeast, 100_000);
    }
    assert!(
        think(&mut room, 1000).ends_with(&release_effects()),
        "new creation must not reset donor immuneTimeCount"
    );
}

#[test]
fn srx3_greed_successful_spawn_resets_shared_sphere_stop_and_only_latest_is_removed() {
    let mut room = GoshnarsGreedRoom::default();
    room.sphere_spawned(7);
    assert_eq!(
        room.sphere_think(7, 3000, SPHERE_SPAWN, true),
        vec![GreedEffect::BossHealFull]
    );
    room.sphere_spawned(8);
    assert_eq!(
        room.sphere_think(7, 3000, SPHERE_SPAWN, false),
        vec![GreedEffect::SphereMove {
            instance: 7,
            position: RoomPosition {
                x: SPHERE_SPAWN.x - 1,
                ..SPHERE_SPAWN
            }
        }]
    );
    room.sphere_died(7);
    assert_eq!(
        think(&mut room, 1000).first(),
        Some(&GreedEffect::RemoveSoulSphere { instance: Some(8) })
    );
}

#[test]
fn srx3_greed_recovered_entry_schedules_only_residual_original_delays() {
    let mut room = GoshnarsGreedRoom::default();
    let _ = room.boss_spawned();
    assert!(room.advance(50_000).is_empty(), "birth is not lever use");
    let mut room = GoshnarsGreedRoom::default();
    let _ = room.boss_spawned();
    room.lever_pulled_with_elapsed(500, 8_000);
    assert_eq!(room.next_due_ms(), Some(500));
    assert_eq!(room.advance(500).len(), 6, "7s and8s warnings are overdue");
    assert_eq!(room.next_due_ms(), Some(1500));
    assert_eq!(room.advance(1500).len(), 3);
    let spawned = room.advance(2500);
    assert_eq!(spawned.len(), 3);
    assert!(
        spawned
            .iter()
            .all(|effect| matches!(effect, GreedEffect::SpawnMonster { .. }))
    );
    assert_eq!(room.next_due_ms(), None);
}

#[test]
fn srx3_greed_new_failed_sphere_cannot_release_an_already_elapsed_immunity_cycle() {
    let mut room = GoshnarsGreedRoom::default();
    let _ = summon_sphere(&mut room);
    room.sphere_spawn_failed();
    assert!(think(&mut room, 100_000).is_empty());
    for id in 20..25 {
        room.minion_died(id, GreedMinion::Greedbeast, 100_000);
    }
    let effects = think_with_result(&mut room, 1_000, None);
    // Real createMonster failure occurred before the same callback's timeout gate.
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, GreedEffect::BossImmune { immune: false })),
        "createdSoulSphere=nil must suppress timeout even after34s counted thinks"
    );
}

#[test]
fn srx3_greed_elapsed_immunity_removes_the_new_successful_sphere_reference_only() {
    let mut room = GoshnarsGreedRoom::default();
    let _ = summon_sphere(&mut room);
    for _ in 0..32 {
        assert!(think(&mut room, 1_000).is_empty());
    }
    for id in 30..35 {
        room.minion_died(id, GreedMinion::Greedbeast, 90_000);
    }
    let effects = think_with_result(&mut room, 1_000, Some(2));
    assert!(effects.contains(&GreedEffect::RemoveSoulSphere { instance: Some(2) }));
    assert!(
        !effects.contains(&GreedEffect::RemoveSoulSphere { instance: Some(1) }),
        "new creation overwrites createdSoulSphere before timeout, leaving older sphere untouched"
    );
    assert!(effects.contains(&GreedEffect::BossImmune { immune: false }));
}
