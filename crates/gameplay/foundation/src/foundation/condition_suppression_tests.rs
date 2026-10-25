use super::*;

#[test]
fn equipment_suppression_removes_current_drown_and_drunk_and_refuses_reapplication() {
    let root = root();
    let facts = facts(0, &root);
    let mut store = ConditionStore::new();
    let drowned = dot(DotElement::Drown, 30, 30, false);
    let drunk = def(
        "cond.drunk",
        ConditionValues::TimedStatus {
            kind: StatusKind::Drunk,
            duration_ms: 10_000,
        },
    );
    let poison = dot(DotElement::Poison, 30, 30, true);
    for definition in [&drowned, &drunk, &poison] {
        apply(
            &mut store,
            definition,
            ConditionSourceKind::Creature,
            &facts,
        )
        .unwrap();
    }
    let suppressed = [drowned.condition_type(), drunk.condition_type()]
        .into_iter()
        .collect();
    assert_eq!(store.set_suppressed(suppressed), 2);
    assert_eq!(store.instances().len(), 1);
    assert_eq!(store.instances()[0].definition(), &poison);
    for definition in [&drowned, &drunk] {
        assert_eq!(
            apply(
                &mut store,
                definition,
                ConditionSourceKind::Creature,
                &facts
            ),
            Err(ConditionRefusal::Immune)
        );
    }
    assert!(
        store
            .take_due(
                0,
                TickFacts {
                    in_protection_zone: false,
                    standing_on_field: None
                }
            )
            .is_empty()
    );
    store.clear_on_death();
    assert_eq!(
        apply(&mut store, &drunk, ConditionSourceKind::Creature, &facts),
        Err(ConditionRefusal::Immune)
    );
    assert_eq!(store.set_suppressed(Default::default()), 0);
    assert!(apply(&mut store, &drunk, ConditionSourceKind::Creature, &facts).is_ok());
}

#[test]
fn equipment_suppression_replacement_keeps_only_the_new_set() {
    let root = root();
    let facts = facts(0, &root);
    let mut store = ConditionStore::new();
    let drowned = dot(DotElement::Drown, 30, 30, true);
    let drunk = def(
        "cond.drunk",
        ConditionValues::TimedStatus {
            kind: StatusKind::Drunk,
            duration_ms: 10_000,
        },
    );
    assert_eq!(
        store.set_suppressed([drowned.condition_type()].into_iter().collect()),
        0
    );
    apply(&mut store, &drunk, ConditionSourceKind::Creature, &facts).unwrap();
    assert_eq!(
        store.set_suppressed([drunk.condition_type()].into_iter().collect()),
        1
    );
    assert_eq!(
        store.set_suppressed([drunk.condition_type()].into_iter().collect()),
        0
    );
    assert!(apply(&mut store, &drowned, ConditionSourceKind::Creature, &facts).is_ok());
    assert_eq!(
        apply(&mut store, &drunk, ConditionSourceKind::Creature, &facts),
        Err(ConditionRefusal::Immune)
    );
}
