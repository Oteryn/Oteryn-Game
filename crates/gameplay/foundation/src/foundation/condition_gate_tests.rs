//! CONDITIONS-0 §3.5 (MAGIC-COND-1): the `muted`, `pacified`, `powerless` and `agony`
//! families, their gates and `COND0-RL-01`.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

const MS: u64 = 1_000;
const GATES: [ConditionGate; 3] = [
    ConditionGate::Muted,
    ConditionGate::Pacified,
    ConditionGate::Powerless,
];

fn root() -> GameplayDecisionRoot {
    GameplayDecisionRoot::from_bytes([7; 32])
}

fn facts(now: u64, root: &GameplayDecisionRoot) -> ApplicationFacts<'_> {
    ApplicationFacts {
        now,
        base_speed: 220,
        mana_shield_capacity: 500,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: root,
        occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
    }
}

fn def(key: &str, values: ConditionValues) -> ConditionDefinition {
    ConditionDefinition::new(key, 1, values).expect("valid definition")
}

fn gate(gate: ConditionGate, duration_ms: u32) -> ConditionDefinition {
    def("cond.gate", ConditionValues::Gate { gate, duration_ms })
}

fn agony(total_min: u32, total_max: u32, delayed: bool) -> ConditionDefinition {
    def(
        "cond.agony",
        ConditionValues::Agony {
            total_min,
            total_max,
            per_tick: 10,
            interval_ms: 2_000,
            delayed,
        },
    )
}

fn apply_at(
    store: &mut ConditionStore<u32>,
    definition: &ConditionDefinition,
    source: u32,
    now: u64,
) -> Result<Applied, ConditionRefusal> {
    let root = root();
    store.apply(
        definition,
        Some(source),
        ConditionSourceKind::Creature,
        &[],
        &facts(now, &root),
    )
}

fn gated(gates: &[ConditionGate], duration_ms: u32) -> ConditionStore<u32> {
    let mut store = ConditionStore::new();
    for &kind in gates {
        apply_at(&mut store, &gate(kind, duration_ms), 1, 0).unwrap();
    }
    store
}

#[test]
fn gate_and_agony_families_have_their_own_keys_and_are_not_negative() {
    for kind in GATES {
        let definition = gate(kind, 1_000);
        assert_eq!(definition.condition_type(), ConditionType::Gate(kind));
        assert_eq!(
            definition.condition_type().conflict_key(),
            ConflictKey::Gate(kind)
        );
        assert!(!definition.is_negative());
    }
    let definition = agony(100, 100, false);
    assert_eq!(definition.condition_type(), ConditionType::Agony);
    assert_eq!(
        definition.condition_type().conflict_key(),
        ConflictKey::Agony
    );
    assert!(!definition.is_negative());
    assert_eq!(
        ConditionDefinition::new(
            "cond.gate",
            1,
            ConditionValues::Gate {
                gate: ConditionGate::Muted,
                duration_ms: 0,
            },
        ),
        None
    );
    assert_eq!(
        ConditionDefinition::new(
            "cond.agony",
            1,
            ConditionValues::Agony {
                total_min: 10,
                total_max: 10,
                per_tick: 10,
                interval_ms: COND0_RL_02_MIN_TICK_INTERVAL_MS - 1,
                delayed: false,
            },
        ),
        None
    );

    let store = gated(&GATES, 1_000);
    assert_eq!(store.instances().len(), 3);
    for kind in GATES {
        assert!(store.has_gate(kind, 0));
    }
}

#[test]
fn gate_application_refreshes_the_duration_even_when_shorter() {
    for kind in GATES {
        let mut store = gated(&[kind], 10_000);
        assert_eq!(
            apply_at(&mut store, &gate(kind, 2_000), 2, 1_000 * MS),
            Ok(Applied {
                sequence: 1,
                replaced: true,
            })
        );
        assert_eq!(store.instances().len(), 1);
        assert_eq!(store.instances()[0].provenance().source, Some(2));
        assert!(store.has_gate(kind, 3_000 * MS - 1));
        assert!(!store.has_gate(kind, 3_000 * MS));
        assert!(store.expire_non_ticking(3_000 * MS));
        assert!(store.instances().is_empty());
    }
}

#[test]
fn spell_cast_gate_answers_with_the_first_of_muted_pacified_powerless() {
    let all = gated(&GATES, 1_000);
    assert_eq!(all.spell_cast_gate(true, 0), Err(ConditionGate::Muted));
    assert_eq!(all.spell_cast_gate(false, 0), Err(ConditionGate::Muted));

    let restricted = gated(&[ConditionGate::Powerless, ConditionGate::Pacified], 1_000);
    assert_eq!(
        restricted.spell_cast_gate(true, 0),
        Err(ConditionGate::Pacified)
    );
    assert_eq!(restricted.spell_cast_gate(false, 0), Ok(()));

    let powerless = gated(&[ConditionGate::Powerless], 1_000);
    assert_eq!(
        powerless.spell_cast_gate(true, 0),
        Err(ConditionGate::Powerless)
    );
    assert_eq!(powerless.spell_cast_gate(false, 0), Ok(()));
    assert_eq!(
        ConditionStore::<u32>::new().spell_cast_gate(true, 0),
        Ok(())
    );
}

#[test]
fn rune_gate_ignores_muted_and_refuses_aggressive_runes_while_pacified_or_powerless() {
    let muted = gated(&[ConditionGate::Muted], 1_000);
    assert_eq!(muted.rune_gate(true, 0), Ok(()));
    assert_eq!(muted.rune_gate(false, 0), Ok(()));

    let all = gated(&GATES, 1_000);
    assert_eq!(all.rune_gate(true, 0), Err(ConditionGate::Pacified));
    assert_eq!(all.rune_gate(false, 0), Ok(()));

    let powerless = gated(&[ConditionGate::Powerless], 1_000);
    assert_eq!(powerless.rune_gate(true, 0), Err(ConditionGate::Powerless));
    assert_eq!(powerless.rune_gate(false, 0), Ok(()));
}

#[test]
fn attack_target_gate_refuses_only_while_pacified() {
    assert_eq!(
        gated(&[ConditionGate::Pacified], 1_000).attack_target_gate(0),
        Err(ConditionGate::Pacified)
    );
    assert_eq!(
        gated(&[ConditionGate::Muted, ConditionGate::Powerless], 1_000).attack_target_gate(0),
        Ok(())
    );
}

#[test]
fn gates_end_at_their_exact_deadline_before_cleanup() {
    let store = gated(&GATES, 1_000);
    let end = 1_000 * MS;
    assert_eq!(
        store.spell_cast_gate(true, end - 1),
        Err(ConditionGate::Muted)
    );
    assert_eq!(store.rune_gate(true, end - 1), Err(ConditionGate::Pacified));
    assert_eq!(
        store.attack_target_gate(end - 1),
        Err(ConditionGate::Pacified)
    );
    assert_eq!(store.instances().len(), 3);
    assert_eq!(store.spell_cast_gate(true, end), Ok(()));
    assert_eq!(store.rune_gate(true, end), Ok(()));
    assert_eq!(store.attack_target_gate(end), Ok(()));
}

#[test]
fn reentry_protection_refuses_gate_and_agony_applications_by_creatures() {
    let root = root();
    let mut protected = facts(0, &root);
    protected.target_reentry_protected = true;
    for definition in [
        gate(ConditionGate::Muted, 1_000),
        gate(ConditionGate::Pacified, 1_000),
        gate(ConditionGate::Powerless, 1_000),
        agony(100, 100, false),
    ] {
        let mut store = ConditionStore::new();
        assert_eq!(
            store.apply(
                &definition,
                Some(1),
                ConditionSourceKind::Creature,
                &[],
                &protected
            ),
            Err(ConditionRefusal::ReentryProtected)
        );
        assert!(store.instances().is_empty());
    }
}

#[test]
fn agony_replaces_when_the_new_total_is_at_least_the_remaining_one() {
    let mut store = ConditionStore::new();
    apply_at(&mut store, &agony(100, 100, true), 1, 0).unwrap();
    let first = store.get(ConflictKey::Agony).unwrap().clone();
    assert_eq!(first.remaining_total(), 100);
    assert_eq!(first.next_tick_at, Some(2_000 * MS));
    assert_eq!(first.immediate_due, None);

    // Strictly smaller is kept, also as the midpoint of an undrawn range (50 + 148) / 2.
    let before = store.clone();
    for smaller in [agony(99, 99, false), agony(50, 148, false)] {
        assert_eq!(
            apply_at(&mut store, &smaller, 2, 500 * MS),
            Err(ConditionRefusal::KeptCurrent)
        );
        assert_eq!(store, before);
    }

    // Equal replaces: the tick timing is kept, one tick is due at once, the source is new.
    assert_eq!(
        apply_at(&mut store, &agony(100, 100, false), 2, 500 * MS),
        Ok(Applied {
            sequence: 1,
            replaced: true,
        })
    );
    let replaced = store.get(ConflictKey::Agony).unwrap();
    assert_eq!(store.instances().len(), 1);
    assert_eq!(replaced.remaining_total(), 100);
    assert_eq!(replaced.next_tick_at, Some(2_000 * MS));
    assert_eq!(replaced.immediate_due, Some(500 * MS));
    assert_eq!(replaced.provenance().source, Some(2));

    // An undrawn range whose midpoint equals the remaining total replaces too; a `delayed`
    // replacement deals no tick at once.
    assert_eq!(
        apply_at(&mut store, &agony(100, 100, true), 3, 600 * MS).map(|a| a.replaced),
        Ok(true)
    );
    let delayed = store.get(ConflictKey::Agony).unwrap();
    assert_eq!(delayed.immediate_due, None);
    assert_eq!(delayed.next_tick_at, Some(2_000 * MS));
    assert_eq!(delayed.provenance().source, Some(3));
    assert_eq!(
        apply_at(&mut store, &agony(50, 150, true), 4, 700 * MS).map(|a| a.replaced),
        Ok(true)
    );
}

#[test]
fn agony_is_never_selected_by_cleanse_or_dispel_and_immunity_does_not_refuse_it() {
    let root = root();
    let poison = def(
        "cond.dot",
        ConditionValues::DamageOverTime {
            element: DotElement::Poison,
            total_min: 50,
            total_max: 50,
            per_tick: 10,
            interval_ms: 2_000,
            delayed: true,
        },
    );
    let mut store = ConditionStore::new();
    apply_at(&mut store, &agony(100, 100, true), 1, 0).unwrap();
    assert_eq!(store.prepare_cleanse(&facts(0, &root)), Ok(None));
    apply_at(&mut store, &poison, 1, 0).unwrap();
    let plan = store.prepare_cleanse(&facts(0, &root)).unwrap().unwrap();
    assert_eq!(
        plan.conflict_key(),
        ConflictKey::Element(DotElement::Poison)
    );
    assert!(store.commit_cleanse(plan));
    assert_eq!(store.cleanse_negative(), 0);
    assert_eq!(store.dispel_key(ConflictKey::Agony), 0);
    assert_eq!(store.instances().len(), 1);
    assert!(store.get(ConflictKey::Agony).is_some());

    // Creature content immunity and a Cleanse immunity on the key do not refuse `agony`.
    let mut immune = ConditionStore::new();
    immune.cleanse_immunities.push(CleanseImmunity {
        key: ConflictKey::Agony,
        until: 11_000 * MS,
    });
    assert_eq!(
        immune
            .apply(
                &agony(100, 100, true),
                Some(1),
                ConditionSourceKind::Creature,
                &[ConditionType::Agony],
                &facts(0, &root),
            )
            .map(|a| a.replaced),
        Ok(false)
    );

    // Death ends it, as every other instance.
    store.clear_on_death();
    assert!(store.instances().is_empty());
}

fn agony_kinds(ticks: &[ConditionTick<u32>]) -> Vec<TickKind> {
    ticks.iter().map(|tick| tick.kind).collect()
}

const AGONY_10: TickKind = TickKind::Agony {
    amount: 10,
    refused: false,
};

#[test]
fn agony_ticks_at_once_then_every_interval_until_its_total_is_spent() {
    let mut store = ConditionStore::new();
    apply_at(&mut store, &agony(30, 30, false), 1, 0).unwrap();
    assert_eq!(
        agony_kinds(&store.take_due(0, TickFacts::default())),
        [AGONY_10]
    );
    assert!(
        store
            .take_due(2_000 * MS - 1, TickFacts::default())
            .is_empty()
    );
    assert_eq!(
        agony_kinds(&store.take_due(2_000 * MS, TickFacts::default())),
        [AGONY_10]
    );
    assert_eq!(store.get(ConflictKey::Agony).unwrap().remaining_total(), 10);
    assert_eq!(
        agony_kinds(&store.take_due(4_000 * MS, TickFacts::default())),
        [AGONY_10]
    );
    assert!(store.get(ConflictKey::Agony).is_none());
}

#[test]
fn delayed_agony_waits_one_interval() {
    let mut store = ConditionStore::new();
    apply_at(&mut store, &agony(20, 20, true), 1, 0).unwrap();
    assert!(
        store
            .take_due(2_000 * MS - 1, TickFacts::default())
            .is_empty()
    );
    assert_eq!(
        agony_kinds(&store.take_due(2_000 * MS, TickFacts::default())),
        [AGONY_10]
    );
}

#[test]
fn agony_deals_damage_and_is_used_up_in_a_protection_zone_and_no_field_keeps_it() {
    let mut store = ConditionStore::new();
    apply_at(&mut store, &agony(30, 30, false), 1, 0).unwrap();
    let pz = TickFacts {
        in_protection_zone: true,
        standing_on_field: None,
    };
    assert_eq!(
        agony_kinds(&store.take_due(0, pz)),
        // Amended by NIGHT-W1 #1: agony is always permitted in PZ.
        [AGONY_10]
    );
    assert_eq!(store.get(ConflictKey::Agony).unwrap().remaining_total(), 20);
    // An element field keeps only its own damage over time; agony has no element.
    let field = TickFacts {
        in_protection_zone: false,
        standing_on_field: Some(DotElement::Fire),
    };
    assert_eq!(agony_kinds(&store.take_due(2_000 * MS, field)), [AGONY_10]);
    assert_eq!(store.get(ConflictKey::Agony).unwrap().remaining_total(), 10);
}

#[test]
fn a_regeneration_pass_never_takes_an_agony_tick() {
    let mut store = ConditionStore::new();
    apply_at(&mut store, &agony(30, 30, false), 1, 0).unwrap();
    assert!(
        store
            .take_due_non_damage(10_000 * MS, TickFacts::default())
            .is_empty()
    );
    let kept = store.get(ConflictKey::Agony).unwrap();
    assert_eq!(kept.remaining_total(), 30);
    assert_eq!(kept.immediate_due, Some(0));
}

#[test]
fn gates_are_not_negative_and_end_only_by_timeout_or_death() {
    let root = root();
    let mut store = gated(&GATES, 1_000);
    for kind in GATES {
        assert!(
            !store
                .get(ConflictKey::Gate(kind))
                .unwrap()
                .definition()
                .is_negative()
        );
    }
    assert_eq!(store.prepare_cleanse(&facts(0, &root)), Ok(None));
    assert_eq!(store.cleanse_negative(), 0);
    for kind in GATES {
        assert_eq!(store.dispel_key(ConflictKey::Gate(kind)), 0);
    }
    assert_eq!(store.instances().len(), 3);
    assert!(!store.expire_non_ticking(1_000 * MS - 1));
    assert!(store.expire_non_ticking(1_000 * MS));
    assert!(store.instances().is_empty());

    let mut dying = gated(&GATES, 1_000);
    assert!(dying.pacified_applied_since(0));
    dying.clear_on_death();
    assert!(dying.instances().is_empty());
    // A target set after the death in the same tick is not measured against that pacified.
    assert!(!dying.pacified_applied_since(0));
}

#[test]
fn rl_01_admits_a_17th_distinct_key_past_the_old_bound_of_16() {
    let mut values: Vec<_> = [
        StatusKind::Invisible,
        StatusKind::Drunk,
        StatusKind::Rooted,
        StatusKind::Feared,
    ]
    .map(|kind| ConditionValues::TimedStatus {
        kind,
        duration_ms: 1_000,
    })
    .into_iter()
    .collect();
    values.extend(
        [
            DotElement::Poison,
            DotElement::Fire,
            DotElement::Energy,
            DotElement::Bleeding,
            DotElement::Drown,
            DotElement::Freezing,
            DotElement::Dazzled,
            DotElement::Cursed,
        ]
        .map(|element| ConditionValues::DamageOverTime {
            element,
            total_min: 10,
            total_max: 10,
            per_tick: 10,
            interval_ms: 1_000,
            delayed: true,
        }),
    );
    values.extend((0..=3).map(|sub_id| ConditionValues::SpellSkills {
        duration_ms: 1_000,
        sub_id,
        magic_level: 1,
        fist: 0,
        melee: 0,
        distance: 0,
        shielding: 0,
    }));
    let mut store = ConditionStore::new();
    for value in values {
        apply_at(&mut store, &def("cond.any", value), 1, 0).unwrap();
    }
    assert_eq!(store.instances().len(), 16);

    // The 17th distinct key, which the old bound of 16 refused, is admitted under 32.
    assert_eq!(
        apply_at(&mut store, &gate(ConditionGate::Muted, 1_000), 1, 0).map(|a| a.replaced),
        Ok(false)
    );
    assert_eq!(store.instances().len(), 17);
    assert!(store.get(ConflictKey::Gate(ConditionGate::Muted)).is_some());
}

#[test]
fn rl_01_admits_every_exhaustive_key_at_once_and_refuses_past_32() {
    assert_eq!(COND0_RL_01_INSTANCES_PER_ACTOR, 32);
    let timed = |kind| ConditionValues::TimedStatus {
        kind,
        duration_ms: 1_000,
    };
    let mut values = vec![
        timed(StatusKind::Invisible),
        timed(StatusKind::Drunk),
        timed(StatusKind::Rooted),
        timed(StatusKind::Feared),
        ConditionValues::Speed {
            paralysis: false,
            range: SpeedRange {
                a_min: 1_000,
                b_min: 0,
                a_max: 1_000,
                b_max: 0,
            },
            duration_ms: 1_000,
        },
        ConditionValues::FoodRegeneration {
            added_ms: 1_000,
            interval_ms: 1_000,
        },
        ConditionValues::Recovery {
            duration_ms: 1_000,
            interval_ms: 1_000,
        },
        ConditionValues::ManaShield { duration_ms: 1_000 },
        ConditionValues::Light {
            level: 6,
            duration_ms: 1_000,
        },
        ConditionValues::Attributes {
            duration_ms: 1_000,
            critical_chance_percent: 0,
            critical_extra_percentage_points: 0,
            damage_dealt_percent: 0,
            incoming_reduction_percent: 0,
        },
        ConditionValues::SpellRegeneration {
            duration_ms: 1_000,
            sub_id: 1,
            health_gain: 1,
            health_interval_ms: 1_000,
            mana_gain: 0,
            mana_interval_ms: 1_000,
        },
        ConditionValues::Invisible { duration_ms: 1_000 },
        ConditionValues::Outfit {
            duration_ms: 1_000,
            look_type: 1,
        },
        ConditionValues::Agony {
            total_min: 10,
            total_max: 10,
            per_tick: 10,
            interval_ms: 1_000,
            delayed: true,
        },
    ];
    values.extend(
        [
            DotElement::Poison,
            DotElement::Fire,
            DotElement::Energy,
            DotElement::Bleeding,
            DotElement::Drown,
            DotElement::Freezing,
            DotElement::Dazzled,
            DotElement::Cursed,
        ]
        .map(|element| ConditionValues::DamageOverTime {
            element,
            total_min: 10,
            total_max: 10,
            per_tick: 10,
            interval_ms: 1_000,
            delayed: true,
        }),
    );
    values.extend((0..=3).map(|sub_id| ConditionValues::SpellSkills {
        duration_ms: 1_000,
        sub_id,
        magic_level: 1,
        fist: 0,
        melee: 0,
        distance: 0,
        shielding: 0,
    }));
    values.extend(GATES.map(|gate| ConditionValues::Gate {
        gate,
        duration_ms: 1_000,
    }));
    let mut store = ConditionStore::new();
    for value in values {
        assert_eq!(
            apply_at(&mut store, &def("cond.any", value), 1, 0).map(|a| a.replaced),
            Ok(false)
        );
    }
    // The §3.5 exhaustive count: 29 distinct keys, all live at once.
    let mut keys: Vec<_> = store
        .instances()
        .iter()
        .map(|i| i.definition().condition_type().conflict_key())
        .collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 29);

    // The cap itself: a 32nd instance is admitted, a 33rd is refused.
    let mut full = ConditionStore::new();
    apply_at(&mut full, &gate(ConditionGate::Muted, 1_000), 1, 0).unwrap();
    let filler = full.instances[0].clone();
    while full.instances.len() < COND0_RL_01_INSTANCES_PER_ACTOR - 1 {
        full.instances.push(filler.clone());
    }
    apply_at(&mut full, &gate(ConditionGate::Pacified, 1_000), 1, 0).unwrap();
    assert_eq!(full.instances().len(), COND0_RL_01_INSTANCES_PER_ACTOR);
    assert_eq!(
        apply_at(&mut full, &gate(ConditionGate::Powerless, 1_000), 1, 0),
        Err(ConditionRefusal::InstanceLimit)
    );
}
