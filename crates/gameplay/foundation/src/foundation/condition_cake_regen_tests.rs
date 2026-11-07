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
