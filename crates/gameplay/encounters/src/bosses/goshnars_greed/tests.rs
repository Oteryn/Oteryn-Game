use super::*;

const SOUL_SPOT: RoomPosition = RoomPosition {
    x: 33750,
    y: 31666,
    floor: 14,
};

fn summon_sphere(room: &mut GoshnarsGreedRoom) -> Vec<GreedEffect> {
    let _ = room.boss_spawned();
    for instance in 1..=5 {
        room.minion_died(instance, GreedMinion::Greedbeast, 0);
    }
    room.boss_think(1_000)
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
    assert!(matches!(
        room.minion_think(3, GreedMinion::WeakSoul, 5_000, 0, &[], || 500),
        Ok(effects) if effects.len() == 2
    ));
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
    assert_eq!(room.boss_think(1_000), vec![]);
    room.minion_died(5, GreedMinion::Greedbeast, 0);
    assert_eq!(room.boss_think(1_000), sphere_summon_effects());
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
    assert_eq!(room.boss_think(1_000), release_effects());
}

#[test]
fn greed_is_released_after_thirty_four_seconds_of_sphere_immunity() {
    let mut room = GoshnarsGreedRoom::default();
    assert_eq!(summon_sphere(&mut room), sphere_summon_effects());
    for _ in 0..32 {
        assert_eq!(room.boss_think(1_000), vec![]);
    }
    assert_eq!(room.boss_think(1_000), release_effects());
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
        GreedEffect::RemoveSoulSphere,
        GreedEffect::BossImmune { immune: false },
        GreedEffect::BossTeleport {
            position: BOSS_HOME,
        },
        GreedEffect::BossSpeed { speed: BOSS_SPEED },
    ]
}
