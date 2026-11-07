//! PR #2782 Q9: Crystal CONDITION_ATTRIBUTES/COMBAT/subID0 is one mutation slot.
//! Source: swift_foot.lua:1,35-43; potions.lua:14-19; condition.cpp:436-447,552-592.
use super::*;

fn swift_attributes() -> ConditionDefinition {
    def(
        "spell.swift.attributes",
        ConditionValues::Attributes {
            duration_ms: 10_000,
            critical_chance_percent: 0,
            critical_extra_percentage_points: 0,
            damage_dealt_percent: 70,
            incoming_reduction_percent: 0,
        },
    )
}

fn bullseye() -> ConditionDefinition {
    def(
        "oteryn:item.tibia.i7443",
        ConditionValues::SpellSkills {
            duration_ms: 600_000,
            sub_id: 0,
            magic_level: 0,
            fist: 0,
            melee: 0,
            distance: 5,
            shielding: -10,
        },
    )
}

#[test]
fn condition_attributes_zero_slot_swift_then_potion_replaces_only_the_debuff() {
    let root = root();
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &speed(false, 1, 0),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    let retained_speed = store.get(ConflictKey::Speed).unwrap().clone();
    apply(
        &mut store,
        &swift_attributes(),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    let now = 1_000 * MS;
    let changed = apply(
        &mut store,
        &bullseye(),
        ConditionSourceKind::SelfUse,
        &facts(now, &root),
    )
    .unwrap();
    assert!(
        changed.replaced,
        "the potion must replace the attribute debuff"
    );
    assert_eq!(store.instances().len(), 2);
    assert_eq!(store.get(ConflictKey::Speed), Some(&retained_speed));
    assert!(
        store.attributes_at(now).is_none(),
        "typed attribute reads stay typed"
    );
    let expiry = now + 600_000 * MS;
    for at in [now, expiry - 1] {
        let skills = store.skill_adjustments(at).unwrap();
        assert_eq!((skills.distance, skills.shielding), (5, -10));
    }
    assert_eq!(store.skill_adjustments(expiry), Some(Default::default()));
}

#[test]
fn condition_attributes_zero_slot_potion_then_shorter_swift_keeps_the_potion() {
    let root = root();
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &bullseye(),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    let before = store.clone();
    assert_eq!(
        apply(
            &mut store,
            &swift_attributes(),
            ConditionSourceKind::SelfUse,
            &facts(1_000 * MS, &root),
        ),
        Err(ConditionRefusal::KeptCurrent)
    );
    assert_eq!(
        store, before,
        "refusal cannot change provenance, expiry or sequence"
    );
    assert!(store.attributes_at(1_000 * MS).is_none());
    assert_eq!(store.skill_adjustments(1_000 * MS).unwrap().distance, 5);

    // The source compares remaining duration, not a permanent priority between value types.
    let late = 591_000 * MS;
    assert!(
        apply(
            &mut store,
            &swift_attributes(),
            ConditionSourceKind::SelfUse,
            &facts(late, &root),
        )
        .unwrap()
        .replaced
    );
    assert_eq!(store.instances().len(), 1);
    assert_eq!(store.skill_adjustments(late), Some(Default::default()));
    assert!(matches!(
        store.attributes_at(late),
        Some(ConditionValues::Attributes {
            damage_dealt_percent: 70,
            ..
        })
    ));
    assert!(store.attributes_at(late + 10_000 * MS).is_none());
}

#[test]
fn condition_attributes_zero_slot_nonzero_subids_remain_independent() {
    let root = root();
    let mut store = ConditionStore::new();
    for sub_id in 1..=3 {
        let sibling = def(
            &format!("source.party.skills.{sub_id}"),
            ConditionValues::SpellSkills {
                duration_ms: 120_000,
                sub_id,
                magic_level: 1,
                fist: 0,
                melee: 0,
                distance: 0,
                shielding: 0,
            },
        );
        apply(
            &mut store,
            &sibling,
            ConditionSourceKind::SelfUse,
            &facts(0, &root),
        )
        .unwrap();
    }
    let siblings = store.instances().to_vec();
    apply(
        &mut store,
        &swift_attributes(),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    apply(
        &mut store,
        &bullseye(),
        ConditionSourceKind::SelfUse,
        &facts(MS, &root),
    )
    .unwrap();
    assert_eq!(store.instances().len(), 4, "only the zero slot is shared");
    assert_eq!(store.skill_adjustments(MS).unwrap().magic_level, 3);
    assert_eq!(store.dispel_key(ConflictKey::Attributes), 1);
    assert_eq!(store.instances(), siblings.as_slice());
    apply(
        &mut store,
        &swift_attributes(),
        ConditionSourceKind::SelfUse,
        &facts(MS, &root),
    )
    .unwrap();
    assert_eq!(store.dispel_key(ConflictKey::SpellSkills(0)), 1);
    assert_eq!(store.instances(), siblings.as_slice());

    apply(
        &mut store,
        &bullseye(),
        ConditionSourceKind::SelfUse,
        &facts(MS, &root),
    )
    .unwrap();
    assert_eq!(
        store.set_suppressed([ConditionType::Attributes].into_iter().collect()),
        1
    );
    assert_eq!(store.instances(), siblings.as_slice());
    assert_eq!(
        apply(
            &mut store,
            &bullseye(),
            ConditionSourceKind::SelfUse,
            &facts(2 * MS, &root)
        ),
        Err(ConditionRefusal::Immune)
    );
    assert_eq!(store.set_suppressed(Default::default()), 0);

    // The same slot identity also carries the existing bounded post-Cleanse immunity.
    use oteryn_gameplay_domain::domain::condition::attributes::{
        AttributeModifier, AttributeModifiers,
    };
    let negative = def(
        "source.attributes.negative",
        ConditionValues::SourceAttributes {
            duration_ms: 10_000,
            modifiers: AttributeModifiers {
                distance: Some(AttributeModifier::Add(-1)),
                ..Default::default()
            },
        },
    );
    apply(
        &mut store,
        &negative,
        ConditionSourceKind::SelfUse,
        &facts(2 * MS, &root),
    )
    .unwrap();
    let plan = store
        .prepare_cleanse(&facts(3 * MS, &root))
        .unwrap()
        .unwrap();
    assert!(store.commit_cleanse(plan));
    assert_eq!(store.instances(), siblings.as_slice());
    assert_eq!(
        store.cleanse_immunity_remaining(ConflictKey::SpellSkills(0), 3 * MS),
        store.cleanse_immunity_remaining(ConflictKey::Attributes, 3 * MS)
    );
    assert_eq!(
        apply(
            &mut store,
            &bullseye(),
            ConditionSourceKind::SelfUse,
            &facts(4 * MS, &root)
        ),
        Err(ConditionRefusal::Immune)
    );
    assert!(
        !store.commit_cleanse(plan),
        "the original sequence proof remains single-use"
    );
    assert_eq!(store.instances(), siblings.as_slice());
}
