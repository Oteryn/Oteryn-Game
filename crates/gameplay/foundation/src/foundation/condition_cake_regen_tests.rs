//! Crystal201189f world_bonus.lua:25-42, recovery.lua:5-9, heal_party.lua:6-11;
//! condition.cpp:436-447,1287-1298 preserves the real COMBAT/sub0 mutation slot.
use super::*;
fn regen(
    key: &str,
    sub_id: u8,
    duration_ms: u32,
    h: u32,
    ht: u32,
    m: u32,
    mt: u32,
) -> ConditionDefinition {
    def(
        key,
        ConditionValues::SpellRegeneration {
            sub_id,
            duration_ms,
            health_gain: h,
            health_interval_ms: ht,
            mana_gain: m,
            mana_interval_ms: mt,
        },
    )
}
#[test]
fn cake_regeneration_reuses_recovery_slot_and_preserves_separate_partial_ticks() {
    let root = root();
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &regen("source.recovery", 0, 60_000, 20, 3_000, 0, 3_000),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    apply(
        &mut store,
        &regen("source.cake", 0, 604_800_000, 1, 6_000, 2, 2_000),
        ConditionSourceKind::SelfUse,
        &facts(1_000 * MS, &root),
    )
    .unwrap();
    assert_eq!(store.instances().len(), 1);
    let due = store.take_due(2_000 * MS, TickFacts::default());
    assert_eq!(
        due.len(),
        1,
        "the old1s mana accumulator survives the changed2s interval"
    );
    assert!(matches!(
        due[0].kind,
        TickKind::SpellRegeneration {
            health_gain: 0,
            mana_gain: 2,
            suppressed: false
        }
    ));
    let due = store.take_due(6_000 * MS, TickFacts::default());
    assert!(due.iter().any(|tick| matches!(
        tick.kind,
        TickKind::SpellRegeneration {
            health_gain: 1,
            mana_gain: 2,
            suppressed: false
        }
    )));
}
#[test]
fn cake_long_regeneration_refuses_shorter_recovery_while_food_and_party_stack() {
    let root = root();
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &regen("source.cake", 0, 604_800_000, 1, 6_000, 2, 2_000),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    apply(
        &mut store,
        &def(
            "source.food",
            ConditionValues::FoodRegeneration {
                added_ms: 60_000,
                interval_ms: 6_000,
            },
        ),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    apply(
        &mut store,
        &regen("source.party", 1, 120_000, 20, 2_000, 0, 2_000),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    assert_eq!(
        apply(
            &mut store,
            &regen("source.recovery", 0, 60_000, 20, 3_000, 0, 3_000),
            ConditionSourceKind::SelfUse,
            &facts(1_000 * MS, &root)
        ),
        Err(ConditionRefusal::KeptCurrent)
    );
    assert_eq!(store.instances().len(), 3);
    assert_eq!(
        store.get(ConflictKey::Recovery).unwrap().definition().key(),
        "source.cake"
    );
}
#[test]
fn regeneration_interval_change_keeps_each_previous_resource_tick() {
    let root = root();
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &regen("source.before", 0, 60_000, 20, 3_000, 7, 2_000),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    let _ = store.take_due(4_000 * MS, TickFacts::default());
    apply(
        &mut store,
        &regen("source.cake", 0, 604_800_000, 1, 6_000, 2, 3_000),
        ConditionSourceKind::SelfUse,
        &facts(4_500 * MS, &root),
    )
    .unwrap();
    let due = store.take_due(7_000 * MS, TickFacts::default());
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].due, 7_000 * MS);
    assert!(matches!(
        due[0].kind,
        TickKind::SpellRegeneration {
            health_gain: 0,
            mana_gain: 2,
            suppressed: false
        }
    ));
    let due = store.take_due(9_000 * MS, TickFacts::default());
    assert!(due.iter().any(|tick| tick.due == 9_000 * MS
        && matches!(
            tick.kind,
            TickKind::SpellRegeneration {
                health_gain: 1,
                mana_gain: 0,
                suppressed: false
            }
        )));
}

#[test]
fn cake_regeneration_partial_resources_survive_snapshot_rebase() {
    let root = root();
    let mut store = ConditionStore::<String>::new();
    store
        .apply(
            &regen("source.before", 0, 60_000, 20, 3_000, 7, 2_000),
            None,
            ConditionSourceKind::SelfUse,
            &[],
            &facts(0, &root),
        )
        .unwrap();
    let _ = store.take_due(4_000 * MS, TickFacts::default());
    store
        .apply(
            &regen("source.cake", 0, 604_800_000, 1, 6_000, 2, 3_000),
            None,
            ConditionSourceKind::SelfUse,
            &[],
            &facts(4_500 * MS, &root),
        )
        .unwrap();
    let saved = store.persistent_snapshot(5_000 * MS).unwrap();
    let mut restored = ConditionStore::from_persistent_snapshot(&saved, 100_000 * MS).unwrap();
    let due = restored.take_due(102_000 * MS, TickFacts::default());
    assert_eq!(due.len(), 1);
    assert!(matches!(
        due[0].kind,
        TickKind::SpellRegeneration {
            health_gain: 0,
            mana_gain: 2,
            suppressed: false
        }
    ));
    let due = restored.take_due(104_000 * MS, TickFacts::default());
    assert!(due.iter().any(|tick| matches!(
        tick.kind,
        TickKind::SpellRegeneration {
            health_gain: 1,
            mana_gain: 0,
            suppressed: false
        }
    )));
}
#[test]
fn legacy_regeneration_snapshot_recovers_separate_next_pulses() {
    let root = root();
    let mut store = ConditionStore::<String>::new();
    store
        .apply(
            &regen("source.cake", 0, 604_800_000, 1, 6_000, 2, 2_000),
            None,
            ConditionSourceKind::SelfUse,
            &[],
            &facts(0, &root),
        )
        .unwrap();
    let _ = store.take_due(2_000 * MS, TickFacts::default());
    let mut saved = store.persistent_snapshot(2_500 * MS).unwrap();
    saved["conditions"][0]
        .as_object_mut()
        .unwrap()
        .remove("regeneration_phase");
    let mut restored = ConditionStore::from_persistent_snapshot(&saved, 0).unwrap();
    let due = restored.take_due(1_500 * MS, TickFacts::default());
    assert_eq!(due.len(), 1);
    assert!(matches!(
        due[0].kind,
        TickKind::SpellRegeneration {
            health_gain: 0,
            mana_gain: 2,
            suppressed: false
        }
    ));
}

#[test]
fn cake_pz_suppression_preserves_accumulated_pulse_until_exit() {
    let root = root();
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &regen("source.cake", 0, 60000, 1, 6000, 2, 2000),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    let suppressed = store.take_due(
        2000 * MS,
        TickFacts {
            in_protection_zone: true,
            standing_on_field: None,
        },
    );
    assert!(suppressed.iter().all(|tick| matches!(
        tick.kind,
        TickKind::SpellRegeneration {
            suppressed: true,
            ..
        }
    )));
    let outside = store.take_due(3000 * MS, TickFacts::default());
    assert!(
        outside.iter().any(|tick| matches!(
            tick.kind,
            TickKind::SpellRegeneration {
                health_gain: 0,
                mana_gain: 2,
                suppressed: false
            }
        )),
        "Crystal condition.cpp1331-1350/1385-1396 retains the PZ accumulator; the first outside tick is ready"
    );
}

#[test]
fn cake_regeneration_honors_independent_native_daily_reward_pz_tiers() {
    for (streak, health, mana) in [(0, 0, 0), (2, 1, 0), (3, 1, 6), (5, 2, 6), (6, 2, 12)] {
        let root = root();
        let mut store = ConditionStore::new();
        apply(
            &mut store,
            &regen("source.cake", 0, 60000, 1, 6000, 2, 2000),
            ConditionSourceKind::SelfUse,
            &facts(0, &root),
        )
        .unwrap();
        let ticks = store.take_due_with_regeneration(
            6000 * MS,
            TickFacts {
                in_protection_zone: true,
                standing_on_field: None,
            },
            RegenerationPzRewards::from_qualified_streak(streak),
        );
        let gains = ticks.into_iter().fold((0, 0), |sum, tick| match tick.kind {
            TickKind::SpellRegeneration {
                health_gain,
                mana_gain,
                suppressed: false,
            } => (sum.0 + health_gain, sum.1 + mana_gain),
            _ => (sum.0, sum.1),
        });
        assert_eq!(gains, (health, mana), "source PZ tier{streak}");
    }
}
#[test]
fn cake_pz_exit_does_not_replay_blocked_mana_while_health_backlog_rebases() {
    let root = root();
    let mut store = ConditionStore::<String>::new();
    store
        .apply(
            &regen("source.cake", 0, 600000, 1, 6000, 2, 2000),
            Some("source.cake".to_owned()),
            ConditionSourceKind::SelfUse,
            &[],
            &facts(0, &root),
        )
        .unwrap();
    let _ = store.take_due_with_regeneration(
        400000 * MS,
        TickFacts {
            in_protection_zone: true,
            standing_on_field: None,
        },
        RegenerationPzRewards::from_qualified_streak(2),
    );
    let outside = store.take_due(400001 * MS, TickFacts::default());
    let mana = outside
        .into_iter()
        .map(|tick| match tick.kind {
            TickKind::SpellRegeneration {
                mana_gain,
                suppressed: false,
                ..
            } => mana_gain,
            _ => 0,
        })
        .sum::<u32>();
    assert_eq!(
        mana, 2,
        "one current pulse after leaving PZ; no retroactive mana for PZ time"
    );
    let snapshot = store.persistent_snapshot(400001 * MS).unwrap();
    let mut restored = ConditionStore::from_persistent_snapshot(&snapshot, 50 * MS).unwrap();
    let resumed = restored.take_due(50 * MS, TickFacts::default());
    assert!(
        resumed
            .iter()
            .all(|tick| matches!(tick.kind, TickKind::SpellRegeneration { mana_gain: 0, .. })),
        "old eligible HP backlog cannot repeat the newly unblocked mana pulse"
    );
}
