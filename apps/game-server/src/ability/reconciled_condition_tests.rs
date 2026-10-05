#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::*;
fn facts(root: &GameplayDecisionRoot, now: u64) -> ApplicationFacts<'_> {
    ApplicationFacts {
        now,
        base_speed: 2040,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: root,
        occurrence: DecisionOccurrenceId::from_bytes([41; 16]),
    }
}
#[test]
fn reconciled_status_exact_expiry_and_main_cleanse_immunity_survive() {
    let root = GameplayDecisionRoot::from_bytes([8; 32]);
    let mut store = ConditionStore::<u32>::new();
    let status = ConditionDefinition::new(
        "source.drunk",
        1,
        ConditionValues::TimedStatus {
            kind: StatusKind::Drunk,
            duration_ms: 1000,
        },
    )
    .unwrap();
    store
        .apply(
            &status,
            Some(9),
            ConditionSourceKind::Creature,
            &[],
            &facts(&root, 0),
        )
        .unwrap();
    assert!(store.has_status(StatusKind::Drunk, 999999));
    assert!(!store.has_status(StatusKind::Drunk, 1000000));
    let plan = store.prepare_cleanse(&facts(&root, 10)).unwrap().unwrap();
    assert!(store.commit_cleanse(plan));
    assert_eq!(
        store.apply(
            &status,
            Some(9),
            ConditionSourceKind::Creature,
            &[],
            &facts(&root, 11)
        ),
        Err(ConditionRefusal::Immune)
    );
    store.clear_on_death();
    assert_eq!(
        store.cleanse_immunity_remaining(ConflictKey::Status(StatusKind::Drunk), 11),
        0
    );
}
#[test]
fn reconciled_rational_speed_retains_main_exact_deadline_and_overflow_refuses() {
    let root = GameplayDecisionRoot::from_bytes([8; 32]);
    let mut store = ConditionStore::<u32>::new();
    let ratio = ExactSpeedRatio {
        numerator: 2901,
        denominator: 2000,
    };
    let def = ConditionDefinition::new(
        "source.speed",
        1,
        ConditionValues::RationalSpeed {
            paralysis: false,
            range: RationalSpeedRange {
                a_min: ratio,
                a_max: ratio,
                b_min: 40,
                b_max: 40,
            },
            duration_ms: 1000,
        },
    )
    .unwrap();
    store
        .apply(
            &def,
            Some(9),
            ConditionSourceKind::Creature,
            &[],
            &facts(&root, 0),
        )
        .unwrap();
    assert_eq!(store.active_speed_delta(999999), 901);
    assert_eq!(store.active_speed_delta(1000000), 0);
    assert!(store.expire_non_ticking(1000000));
    assert!(store.instances().is_empty());
    assert!(
        store
            .apply(
                &def,
                Some(9),
                ConditionSourceKind::Creature,
                &[],
                &facts(&root, u64::MAX)
            )
            .is_err()
    );
    assert!(store.instances().is_empty());
}
