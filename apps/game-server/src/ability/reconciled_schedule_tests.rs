#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::*;
fn facts(root: &GameplayDecisionRoot) -> ApplicationFacts<'_> {
    ApplicationFacts {
        now: 0,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: root,
        occurrence: DecisionOccurrenceId::from_bytes([44; 16]),
    }
}
#[test]
fn source_schedule_reconciles_with_native_cleanse_and_bounded_due_budget() {
    let root = GameplayDecisionRoot::from_bytes([8; 32]);
    let mut store = ConditionStore::<u32>::new();
    let schedule = DamageSchedule::Fixed {
        segments: vec![DamageSegment {
            count: 8,
            interval_ms: 1000,
            amount: 3,
        }],
        delayed: true,
    };
    let def =
        ConditionDefinition::new_damage_schedule("source.poison", 1, DotElement::Poison, schedule)
            .unwrap();
    store
        .apply(
            &def,
            Some(91),
            ConditionSourceKind::Creature,
            &[],
            &facts(&root),
        )
        .unwrap();
    let due = store.take_due(8000000, TickFacts::default());
    assert_eq!(due.len(), 4);
    assert!(due.iter().all(|tick| tick.provenance.source == Some(91)));
    assert!(store.take_due(8000000, TickFacts::default()).is_empty());
    let mut cleanse = facts(&root);
    cleanse.now = 8000000;
    let plan = store.prepare_cleanse(&cleanse).unwrap().unwrap();
    assert!(store.commit_cleanse(plan));
    assert!(store.instances().is_empty());
    assert!(
        store.cleanse_immunity_remaining(ConflictKey::Element(DotElement::Poison), 8000000) > 0
    );
}
#[test]
fn immediate_schedule_tick_on_own_field_consumes_the_cursor() {
    let root = GameplayDecisionRoot::from_bytes([9; 32]);
    let mut store = ConditionStore::<u32>::new();
    let schedule = DamageSchedule::Fixed {
        segments: vec![DamageSegment {
            count: 3,
            interval_ms: 1000,
            amount: 2,
        }],
        delayed: false,
    };
    let def =
        ConditionDefinition::new_damage_schedule("source.poison", 1, DotElement::Poison, schedule)
            .unwrap();
    store
        .apply(&def, None, ConditionSourceKind::Field, &[], &facts(&root))
        .unwrap();
    let field = TickFacts {
        in_protection_zone: false,
        standing_on_field: Some(DotElement::Poison),
    };
    assert_eq!(store.take_due(0, field).len(), 1);
    assert_eq!(
        store
            .get(ConflictKey::Element(DotElement::Poison))
            .unwrap()
            .remaining_total(),
        4
    );
    let mut later = 0;
    for second in 1..=10u64 {
        later += store
            .take_due(second * 1_000_000, TickFacts::default())
            .len();
    }
    assert_eq!(later, 2);
    assert!(store.instances().is_empty());
}
