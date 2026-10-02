#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

const MS: u64 = 1_000;

fn root() -> GameplayDecisionRoot {
    GameplayDecisionRoot::from_bytes([7; 32])
}

fn occurrence(n: u8) -> DecisionOccurrenceId {
    DecisionOccurrenceId::from_bytes([n; 16])
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
        occurrence: occurrence(1),
    }
}

fn provenance(
    source: u32,
    kind: ConditionSourceKind,
    def: &ConditionDefinition,
) -> ConditionProvenance<u32> {
    ConditionProvenance {
        source: Some(source),
        source_kind: kind,
        definition_key: def.key().to_owned(),
        definition_revision: def.revision(),
        lineage: None,
    }
}

fn def(key: &str, values: ConditionValues) -> ConditionDefinition {
    ConditionDefinition::new(key, 1, values).expect("valid definition")
}

fn dot(element: DotElement, total_min: u32, total_max: u32, delayed: bool) -> ConditionDefinition {
    def(
        "cond.dot",
        ConditionValues::DamageOverTime {
            element,
            total_min,
            total_max,
            per_tick: 10,
            interval_ms: 2_000,
            delayed,
        },
    )
}

fn speed(paralysis: bool, a: i32, b: i32) -> ConditionDefinition {
    def(
        "cond.speed",
        ConditionValues::Speed {
            paralysis,
            range: SpeedRange {
                a_min: a,
                b_min: b,
                a_max: a,
                b_max: b,
            },
            duration_ms: 6_000,
        },
    )
}

fn apply(
    store: &mut ConditionStore<u32>,
    definition: &ConditionDefinition,
    kind: ConditionSourceKind,
    facts: &ApplicationFacts<'_>,
) -> Result<Applied, ConditionRefusal> {
    store.apply(definition, Some(9), kind, &[], facts)
}

fn damage_amount(tick: &ConditionTick<u32>) -> (u32, bool) {
    match tick.kind {
        TickKind::Damage {
            amount, refused, ..
        } => (amount, refused),
        // Never a pair a damage tick yields: a damage tick's amount is positive.
        TickKind::Regeneration { .. } | TickKind::SpellRegeneration { .. } => (0, false),
    }
}

#[test]
fn definitions_reject_a_tick_interval_under_cond0_rl_02_and_empty_values() {
    let fast = ConditionValues::Recovery {
        duration_ms: 10_000,
        interval_ms: 999,
    };
    assert!(ConditionDefinition::new("cond.recovery", 1, fast).is_none());
    let inverted = ConditionValues::DamageOverTime {
        element: DotElement::Poison,
        total_min: 20,
        total_max: 10,
        per_tick: 1,
        interval_ms: 2_000,
        delayed: false,
    };
    assert!(ConditionDefinition::new("cond.poison", 1, inverted).is_none());
    assert!(
        ConditionDefinition::new("", 1, ConditionValues::ManaShield { duration_ms: 1 }).is_none()
    );
}

#[test]
fn haste_and_paralysis_replace_each_other_on_the_speed_key() {
    let root = root();
    let mut store = ConditionStore::new();
    // Haste 1.3 × (220 − 40) − 24 = 210 → delta −10.
    apply(
        &mut store,
        &speed(false, 1_300, -24),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    assert_eq!(store.speed_delta(), 210 - 220);
    let applied = apply(
        &mut store,
        &speed(true, 250, 0),
        ConditionSourceKind::Creature,
        &facts(MS, &root),
    )
    .unwrap();
    assert!(applied.replaced);
    // Paralysis 0.25 × 180 = 45 → delta −175.
    assert_eq!(store.speed_delta(), 45 - 220);
    assert_eq!(store.instances().len(), 1);
}

#[test]
fn paralysis_keeps_speed_at_least_40_and_leaves_a_base_under_40_unchanged() {
    let root = root();
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &speed(true, 0, 0),
        ConditionSourceKind::Creature,
        &facts(0, &root),
    )
    .unwrap();
    assert_eq!(store.speed_delta(), 40 - 220);
    let mut slow = facts(0, &root);
    slow.base_speed = 30;
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &speed(true, 0, 0),
        ConditionSourceKind::Creature,
        &slow,
    )
    .unwrap();
    assert_eq!(store.speed_delta(), 0);
}

#[test]
fn a_speed_draw_is_uniform_in_the_range_and_reproducible() {
    let root = root();
    let ranged = def(
        "cond.haste",
        ConditionValues::Speed {
            paralysis: false,
            range: SpeedRange {
                a_min: 1_000,
                b_min: 0,
                a_max: 2_000,
                b_max: 0,
            },
            duration_ms: 1_000,
        },
    );
    let draw = || {
        let mut store = ConditionStore::new();
        apply(
            &mut store,
            &ranged,
            ConditionSourceKind::SelfUse,
            &facts(0, &root),
        )
        .unwrap();
        store.speed_delta()
    };
    let first = draw();
    assert_eq!(first, draw());
    assert!((180 - 220..=360 - 220).contains(&first));
}

#[test]
fn damage_over_time_ticks_at_once_unless_delayed_then_at_its_interval() {
    let root = root();
    let mut store = ConditionStore::new();
    let applied = apply(
        &mut store,
        &dot(DotElement::Poison, 30, 30, false),
        ConditionSourceKind::Creature,
        &facts(0, &root),
    )
    .unwrap();
    assert!(!applied.replaced);
    let first = store.take_due(0, TickFacts::default());
    assert_eq!(first.len(), 1);
    assert_eq!(damage_amount(&first[0]), (10, false));
    assert!(store.take_due(1_999 * MS, TickFacts::default()).is_empty());
    let ticks = store.take_due(4_000 * MS, TickFacts::default());
    assert_eq!(
        ticks.iter().map(|t| t.due).collect::<Vec<_>>(),
        [2_000 * MS, 4_000 * MS]
    );
    assert!(store.instances().is_empty(), "the total is used up");

    let mut store = ConditionStore::new();
    let applied = apply(
        &mut store,
        &dot(DotElement::Fire, 30, 30, true),
        ConditionSourceKind::Creature,
        &facts(0, &root),
    )
    .unwrap();
    assert_eq!(applied.sequence, 0);
    assert!(store.take_due(0, TickFacts::default()).is_empty());
    assert_eq!(
        store
            .get(ConflictKey::Element(DotElement::Fire))
            .unwrap()
            .remaining_total(),
        30
    );
}

#[test]
fn a_damage_over_time_replaces_only_a_strictly_smaller_remaining_total_except_from_a_field() {
    let root = root();
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &dot(DotElement::Poison, 40, 40, true),
        ConditionSourceKind::Creature,
        &facts(0, &root),
    )
    .unwrap();
    // Midpoint 40 is not strictly greater than the remaining 40.
    assert_eq!(
        apply(
            &mut store,
            &dot(DotElement::Poison, 30, 50, true),
            ConditionSourceKind::Player,
            &facts(MS, &root)
        ),
        Err(ConditionRefusal::KeptCurrent)
    );
    // A field always replaces, and keeps the current tick timing with the new provenance.
    let field = dot(DotElement::Poison, 20, 20, true);
    let applied = store
        .apply(
            &field,
            Some(77),
            ConditionSourceKind::Field,
            &[],
            &facts(MS, &root),
        )
        .unwrap();
    assert!(applied.replaced);
    let instance = store.get(ConflictKey::Element(DotElement::Poison)).unwrap();
    assert_eq!(instance.provenance().source, Some(77));
    assert_eq!(instance.remaining_total(), 20);
    let ticks = store.take_due(2_000 * MS, TickFacts::default());
    assert_eq!(ticks.len(), 1, "the original 2 s timing is kept");
    // Strictly greater from a creature replaces.
    assert!(
        apply(
            &mut store,
            &dot(DotElement::Poison, 11, 11, true),
            ConditionSourceKind::Creature,
            &facts(3_000 * MS, &root)
        )
        .unwrap()
        .replaced
    );
}

#[test]
fn damage_ticks_run_each_bounded_by_cond0_rl_03_and_none_is_dropped() {
    let root = root();
    let mut store = ConditionStore::new();
    let long = def(
        "cond.bleed",
        ConditionValues::DamageOverTime {
            element: DotElement::Bleeding,
            total_min: 100,
            total_max: 100,
            per_tick: 10,
            interval_ms: 1_000,
            delayed: true,
        },
    );
    apply(
        &mut store,
        &long,
        ConditionSourceKind::Creature,
        &facts(0, &root),
    )
    .unwrap();
    let first = store.take_due(6_000 * MS, TickFacts::default());
    assert_eq!(first.len(), COND0_RL_03_DAMAGE_TICKS_PER_SIM_TICK);
    let second = store.take_due(6_050 * MS, TickFacts::default());
    let dues: Vec<u64> = first.iter().chain(&second).map(|t| t.due / MS).collect();
    assert_eq!(dues, [1_000, 2_000, 3_000, 4_000, 5_000, 6_000]);
}

#[test]
fn a_field_of_the_same_element_does_not_use_ticks_up_and_a_protection_zone_refuses_them() {
    let root = root();
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &dot(DotElement::Fire, 30, 30, true),
        ConditionSourceKind::Field,
        &facts(0, &root),
    )
    .unwrap();
    let on_field = TickFacts {
        in_protection_zone: false,
        standing_on_field: Some(DotElement::Fire),
    };
    let tick = store.take_due(2_000 * MS, on_field);
    assert_eq!(damage_amount(&tick[0]), (10, false));
    assert_eq!(
        store
            .get(ConflictKey::Element(DotElement::Fire))
            .unwrap()
            .remaining_total(),
        30
    );
    let pz = TickFacts {
        in_protection_zone: true,
        standing_on_field: None,
    };
    let tick = store.take_due(4_000 * MS, pz);
    assert_eq!(damage_amount(&tick[0]), (10, true));
    assert_eq!(
        store
            .get(ConflictKey::Element(DotElement::Fire))
            .unwrap()
            .remaining_total(),
        20
    );
}

#[test]
fn the_channel_order_is_due_then_actor_then_sequence() {
    let tick = |due, sequence| ConditionTick {
        due,
        sequence,
        kind: TickKind::Regeneration {
            key: ConflictKey::Recovery,
            suppressed: false,
        },
        provenance: ConditionProvenance::<u32> {
            source: None,
            source_kind: ConditionSourceKind::SelfUse,
            definition_key: "cond.recovery".to_owned(),
            definition_revision: 1,
            lineage: None,
        },
    };
    let order = channel_tick_order([
        (2_u32, vec![tick(5, 0), tick(9, 1)]),
        (1, vec![tick(5, 3), tick(5, 7)]),
    ]);
    let keys: Vec<(u64, u32, u32)> = order.iter().map(|(a, t)| (t.due, *a, t.sequence)).collect();
    assert_eq!(keys, [(5, 1, 3), (5, 1, 7), (5, 2, 0), (9, 2, 1)]);
}

#[test]
fn admission_refuses_immunity_cleanse_immunity_reentry_protection_and_the_instance_limit() {
    let root = root();
    let poison = dot(DotElement::Poison, 10, 10, true);
    let mut store = ConditionStore::new();
    let immune = [ConditionType::DamageOverTime(DotElement::Poison)];
    assert_eq!(
        store.apply(
            &poison,
            Some(1),
            ConditionSourceKind::Creature,
            &immune,
            &facts(0, &root)
        ),
        Err(ConditionRefusal::Immune)
    );
    // Paralysis immunity does not refuse haste.
    let haste = speed(false, 1_000, 0);
    assert!(
        store
            .apply(
                &haste,
                Some(1),
                ConditionSourceKind::SelfUse,
                &[ConditionType::Paralysis],
                &facts(0, &root)
            )
            .is_ok()
    );

    let mut protected = facts(0, &root);
    protected.target_reentry_protected = true;
    assert_eq!(
        apply(
            &mut store,
            &poison,
            ConditionSourceKind::Creature,
            &protected
        ),
        Err(ConditionRefusal::ReentryProtected)
    );
    // Fields and PvP are unaffected.
    assert!(
        apply(
            &mut store,
            &dot(DotElement::Fire, 5, 5, true),
            ConditionSourceKind::Field,
            &protected
        )
        .is_ok()
    );
    assert!(apply(&mut store, &poison, ConditionSourceKind::Player, &protected).is_ok());
    // A protected player's own offensive action applies none to a creature.
    let mut attacking = facts(0, &root);
    attacking.source_reentry_protected = true;
    attacking.target_is_player = false;
    assert_eq!(
        apply(
            &mut ConditionStore::new(),
            &poison,
            ConditionSourceKind::Player,
            &attacking
        ),
        Err(ConditionRefusal::ReentryProtected)
    );

    let mut full = ConditionStore::new();
    let light = def(
        "cond.light",
        ConditionValues::Light {
            level: 6,
            duration_ms: 1_000,
        },
    );
    apply(
        &mut full,
        &light,
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    let filler = full.instances[0].clone();
    while full.instances.len() < COND0_RL_01_INSTANCES_PER_ACTOR {
        full.instances.push(filler.clone());
    }
    assert_eq!(
        apply(
            &mut full,
            &poison,
            ConditionSourceKind::Player,
            &facts(0, &root)
        ),
        Err(ConditionRefusal::InstanceLimit)
    );
}

#[test]
fn food_time_adds_up_to_its_cap_and_regenerates_nothing_in_a_protection_zone() {
    let root = root();
    let food = def(
        "cond.food.ham",
        ConditionValues::FoodRegeneration {
            added_ms: 600_000,
            interval_ms: 3_000,
        },
    );
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &food,
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    assert_eq!(
        apply(
            &mut store,
            &food,
            ConditionSourceKind::SelfUse,
            &facts(0, &root)
        ),
        Err(ConditionRefusal::Full)
    );
    apply(
        &mut store,
        &food,
        ConditionSourceKind::SelfUse,
        &facts(1_000 * MS, &root),
    )
    .unwrap();
    let pz = TickFacts {
        in_protection_zone: true,
        standing_on_field: None,
    };
    let ticks = store.take_due(3_000 * MS, pz);
    assert_eq!(
        ticks[0].kind,
        TickKind::Regeneration {
            key: ConflictKey::FoodRegeneration,
            suppressed: true
        }
    );
}

#[test]
fn light_replaces_only_a_duration_not_longer_and_decays_linearly() {
    let root = root();
    let long = def(
        "cond.light.long",
        ConditionValues::Light {
            level: 8,
            duration_ms: 10_000,
        },
    );
    let short = def(
        "cond.light.short",
        ConditionValues::Light {
            level: 9,
            duration_ms: 4_000,
        },
    );
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &long,
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    assert_eq!(
        store
            .get(ConflictKey::Light)
            .unwrap()
            .light_level(5_000 * MS),
        4
    );
    assert_eq!(
        apply(
            &mut store,
            &short,
            ConditionSourceKind::SelfUse,
            &facts(1_000 * MS, &root)
        ),
        Err(ConditionRefusal::KeptCurrent)
    );
    assert!(
        apply(
            &mut store,
            &short,
            ConditionSourceKind::SelfUse,
            &facts(6_000 * MS, &root)
        )
        .unwrap()
        .replaced
    );
    store.take_due(10_000 * MS, TickFacts::default());
    assert!(store.get(ConflictKey::Light).is_none());
}

#[test]
fn death_clears_every_instance() {
    let root = root();
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &dot(DotElement::Poison, 10, 10, true),
        ConditionSourceKind::Creature,
        &facts(0, &root),
    )
    .unwrap();
    apply(
        &mut store,
        &speed(false, 1_000, 0),
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    store.clear_on_death();
    assert!(store.instances().is_empty());
    assert_eq!(store.speed_delta(), 0);
}

#[test]
fn immediate_ticks_share_the_cond0_rl_03_budget_of_their_simulation_tick() {
    let root = root();
    let mut store = ConditionStore::new();
    for element in [
        DotElement::Poison,
        DotElement::Fire,
        DotElement::Energy,
        DotElement::Bleeding,
        DotElement::Cursed,
    ] {
        apply(
            &mut store,
            &dot(element, 30, 30, false),
            ConditionSourceKind::Creature,
            &facts(0, &root),
        )
        .unwrap();
    }
    let first = store.take_due(0, TickFacts::default());
    assert_eq!(first.len(), COND0_RL_03_DAMAGE_TICKS_PER_SIM_TICK);
    // A second pass in the same simulation tick deals nothing more.
    assert!(store.take_due(0, TickFacts::default()).is_empty());
    let next = store.take_due(50 * MS, TickFacts::default());
    assert_eq!(
        next.iter().map(|t| (t.due, t.sequence)).collect::<Vec<_>>(),
        [(0, 4)]
    );
}

#[test]
fn a_capped_damage_tick_is_not_overtaken_by_a_later_regeneration_tick() {
    let root = root();
    let mut store = ConditionStore::new();
    let bleed = def(
        "cond.bleed",
        ConditionValues::DamageOverTime {
            element: DotElement::Bleeding,
            total_min: 100,
            total_max: 100,
            per_tick: 10,
            interval_ms: 1_000,
            delayed: true,
        },
    );
    let recovery = def(
        "cond.recovery",
        ConditionValues::Recovery {
            duration_ms: 60_000,
            interval_ms: 6_000,
        },
    );
    apply(
        &mut store,
        &bleed,
        ConditionSourceKind::Creature,
        &facts(0, &root),
    )
    .unwrap();
    apply(
        &mut store,
        &recovery,
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    let first: Vec<u64> = store
        .take_due(6_000 * MS, TickFacts::default())
        .iter()
        .map(|t| t.due / MS)
        .collect();
    let second: Vec<(u64, u32)> = store
        .take_due(6_050 * MS, TickFacts::default())
        .iter()
        .map(|t| (t.due / MS, t.sequence))
        .collect();
    assert_eq!(first, [1_000, 2_000, 3_000, 4_000]);
    // Damage 5 and 6 (sequence 0) before the regeneration at 6 (sequence 1).
    assert_eq!(second, [(5_000, 0), (6_000, 0), (6_000, 1)]);
}

#[test]
fn a_regeneration_catch_up_is_bounded_per_simulation_tick_and_nothing_is_dropped() {
    let root = root();
    let mut store = ConditionStore::new();
    let recovery = def(
        "cond.recovery",
        ConditionValues::Recovery {
            duration_ms: 60_000,
            interval_ms: 1_000,
        },
    );
    apply(
        &mut store,
        &recovery,
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    let mut dealt = 0;
    let mut now = 120_000 * MS;
    while store.get(ConflictKey::Recovery).is_some() {
        let ticks = store.take_due(now, TickFacts::default());
        assert!(ticks.len() <= COND0_RL_03_DAMAGE_TICKS_PER_SIM_TICK);
        dealt += ticks.len();
        now += 50 * MS;
    }
    assert_eq!(dealt, 60);
}

#[test]
fn the_last_tick_of_an_instance_carries_its_frozen_provenance() {
    let root = root();
    let mut store = ConditionStore::new();
    let poison = dot(DotElement::Poison, 10, 10, true);
    store
        .apply(
            &poison,
            Some(42),
            ConditionSourceKind::Creature,
            &[],
            &facts(0, &root),
        )
        .unwrap();
    let ticks = store.take_due(2_000 * MS, TickFacts::default());
    assert!(store.instances().is_empty(), "the tick used the total up");
    assert_eq!(
        ticks[0].provenance,
        provenance(42, ConditionSourceKind::Creature, &poison)
    );
}

#[test]
fn the_frozen_provenance_names_the_admitted_definition() {
    let root = root();
    let mut store = ConditionStore::new();
    let poison = ConditionDefinition::new(
        "cond.poison.strong",
        7,
        ConditionValues::DamageOverTime {
            element: DotElement::Poison,
            total_min: 10,
            total_max: 10,
            per_tick: 10,
            interval_ms: 2_000,
            delayed: true,
        },
    )
    .unwrap();
    store
        .apply(
            &poison,
            Some(5),
            ConditionSourceKind::Creature,
            &[],
            &facts(0, &root),
        )
        .unwrap();
    let ticks = store.take_due(2_000 * MS, TickFacts::default());
    assert_eq!(ticks[0].provenance.definition_key, "cond.poison.strong");
    assert_eq!(ticks[0].provenance.definition_revision, 7);
}

#[test]
fn a_capped_actor_does_not_hold_back_the_channel_and_its_carried_tick_leads_the_next_one() {
    let root = root();
    let mut first_actor = ConditionStore::new();
    for element in [
        DotElement::Poison,
        DotElement::Fire,
        DotElement::Energy,
        DotElement::Bleeding,
        DotElement::Cursed,
    ] {
        apply(
            &mut first_actor,
            &dot(element, 30, 30, false),
            ConditionSourceKind::Creature,
            &facts(0, &root),
        )
        .unwrap();
    }
    let mut second_actor = ConditionStore::new();
    let poison = dot(DotElement::Poison, 30, 30, false);
    apply(
        &mut second_actor,
        &poison,
        ConditionSourceKind::Creature,
        &facts(0, &root),
    )
    .unwrap();
    let keys = |order: Vec<(u32, ConditionTick<u32>)>| -> Vec<(u64, u32, u32)> {
        order.iter().map(|(a, t)| (t.due, *a, t.sequence)).collect()
    };

    // Actor 1 is capped at COND0-RL-03; actor 2's tick at the same `due` still runs now.
    let now = channel_tick_order([
        (1_u32, first_actor.take_due(0, TickFacts::default())),
        (2, second_actor.take_due(0, TickFacts::default())),
    ]);
    assert_eq!(
        keys(now),
        [(0, 1, 0), (0, 1, 1), (0, 1, 2), (0, 1, 3), (0, 2, 0)]
    );

    // Next simulation tick: the carried tick keeps `due` 0 and leads what is newly due.
    let fire = dot(DotElement::Fire, 30, 30, false);
    apply(
        &mut second_actor,
        &fire,
        ConditionSourceKind::Creature,
        &facts(50 * MS, &root),
    )
    .unwrap();
    let next = channel_tick_order([
        (1_u32, first_actor.take_due(50 * MS, TickFacts::default())),
        (2, second_actor.take_due(50 * MS, TickFacts::default())),
    ]);
    assert_eq!(keys(next), [(0, 1, 4), (50 * MS, 2, 1)]);
}

#[test]
fn source_field_sequence_keeps_variable_damage_and_real_field_pause_clock() {
    let root = root();
    let mut steps = [DotSequenceStep::default(); MAX_DOT_SEQUENCE_STEPS];
    steps[0] = DotSequenceStep {
        amount: 5,
        interval_ms: 1000,
        repetitions: 1,
    };
    steps[1] = DotSequenceStep {
        amount: 2,
        interval_ms: 5000,
        repetitions: 2,
    };
    let definition = def(
        "source.field",
        ConditionValues::DamageSequence {
            element: DotElement::Poison,
            steps,
            len: 2,
            delayed: false,
        },
    );
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &definition,
        ConditionSourceKind::Field,
        &facts(0, &root),
    )
    .unwrap();
    let field = TickFacts {
        in_protection_zone: false,
        standing_on_field: Some(DotElement::Poison),
    };
    assert_eq!(damage_amount(&store.take_due(0, field)[0]), (5, false));
    assert_eq!(
        store
            .get(ConflictKey::Element(DotElement::Poison))
            .unwrap()
            .remaining_total(),
        4
    );
    assert!(store.take_due(1000 * MS, field).is_empty());
    assert_eq!(
        damage_amount(&store.take_due(5000 * MS, field)[0]),
        (2, false)
    );
    assert_eq!(
        store
            .get(ConflictKey::Element(DotElement::Poison))
            .unwrap()
            .remaining_total(),
        4
    );
    assert!(store.take_due(5000 * MS, field).is_empty());
    assert_eq!(
        damage_amount(&store.take_due(10000 * MS, TickFacts::default())[0]),
        (2, false)
    );
    let pz = TickFacts {
        in_protection_zone: true,
        standing_on_field: Some(DotElement::Poison),
    };
    assert_eq!(damage_amount(&store.take_due(15000 * MS, pz)[0]), (2, true));
    assert!(store.instances().is_empty());
}

#[test]
fn source_field_sequence_is_bounded_and_a_refused_definition_cannot_mutate_store() {
    let mut steps = [DotSequenceStep::default(); MAX_DOT_SEQUENCE_STEPS];
    steps[0] = DotSequenceStep {
        amount: 1,
        interval_ms: 1000,
        repetitions: 10,
    };
    let definition = def(
        "source.field",
        ConditionValues::DamageSequence {
            element: DotElement::Fire,
            steps,
            len: 1,
            delayed: true,
        },
    );
    let root = root();
    let mut store = ConditionStore::new();
    apply(
        &mut store,
        &definition,
        ConditionSourceKind::Field,
        &facts(0, &root),
    )
    .unwrap();
    assert_eq!(store.take_due(10000 * MS, TickFacts::default()).len(), 4);
    assert!(store.take_due(10000 * MS, TickFacts::default()).is_empty());
    assert_eq!(store.take_due(10001 * MS, TickFacts::default()).len(), 4);
    steps[0].interval_ms = 999;
    assert!(
        ConditionDefinition::new(
            "invalid",
            1,
            ConditionValues::DamageSequence {
                element: DotElement::Fire,
                steps,
                len: 1,
                delayed: false,
            }
        )
        .is_none()
    );
}

#[test]
fn unknown_combat_owner_does_not_starve_qualified_regeneration_or_consume_dot() {
    let root = root();
    let mut store = ConditionStore::new();
    let damage = dot(DotElement::Fire, 30, 30, true);
    apply(
        &mut store,
        &damage,
        ConditionSourceKind::Field,
        &facts(0, &root),
    )
    .unwrap();
    let regeneration = def(
        "source.regen",
        ConditionValues::SpellRegeneration {
            sub_id: 0,
            duration_ms: 10000,
            health_gain: 5,
            health_interval_ms: 2000,
            mana_gain: 7,
            mana_interval_ms: 2000,
        },
    );
    apply(
        &mut store,
        &regeneration,
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    let ticks = store.take_due_non_damage(2000 * MS, TickFacts::default());
    assert_eq!(ticks.len(), 1);
    assert!(matches!(
        ticks[0].kind,
        TickKind::SpellRegeneration {
            health_gain: 5,
            mana_gain: 7,
            suppressed: false
        }
    ));
    assert_eq!(
        store
            .get(ConflictKey::Element(DotElement::Fire))
            .unwrap()
            .remaining_total(),
        30
    );
    let damage = store.take_due(2000 * MS, TickFacts::default());
    assert_eq!(damage.len(), 1);
    assert_eq!(damage_amount(&damage[0]), (10, false));
}

#[test]
fn source_party_subids_coexist_expire_and_do_not_sum_recasts() {
    let root = root();
    let mut store = ConditionStore::<u32>::new();
    let skill = |id, magic_level, fist, melee, distance, shielding| {
        def(
            &format!("source.party.skills.{id}"),
            ConditionValues::SpellSkills {
                duration_ms: 120000,
                sub_id: id,
                magic_level,
                fist,
                melee,
                distance,
                shielding,
            },
        )
    };
    for d in [
        skill(1, 0, 3, 3, 3, 0),
        skill(2, 0, 0, 0, 0, 3),
        skill(3, 1, 0, 0, 0, 0),
    ] {
        apply(
            &mut store,
            &d,
            ConditionSourceKind::SelfUse,
            &facts(0, &root),
        )
        .unwrap();
    }
    let avatar = def(
        "source.avatar.attributes",
        ConditionValues::Attributes {
            duration_ms: 15000,
            critical_chance_percent: 100,
            critical_extra_percentage_points: 15,
            damage_dealt_percent: 100,
            incoming_reduction_percent: 15,
        },
    );
    apply(
        &mut store,
        &avatar,
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    let invisible = def(
        "source.invisible",
        ConditionValues::Invisible {
            duration_ms: 200000,
        },
    );
    apply(
        &mut store,
        &invisible,
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    assert_eq!(store.instances().len(), 5);
    assert!(store.invisible_at(199999 * MS));
    assert!(!store.invisible_at(200000 * MS));
    let current = store.skill_adjustments(119999 * MS).unwrap();
    assert_eq!(
        (
            current.magic_level,
            current.fist,
            current.melee,
            current.distance,
            current.shielding
        ),
        (1, 3, 3, 3, 3)
    );
    assert_eq!(current.adjust_skill(50, 2), Some(53));
    assert_eq!(current.adjust_magic_level(10), Some(11));
    apply(
        &mut store,
        &skill(3, 1, 0, 0, 0, 0),
        ConditionSourceKind::SelfUse,
        &facts(1000 * MS, &root),
    )
    .unwrap();
    assert_eq!(store.skill_adjustments(1000 * MS).unwrap().magic_level, 1);
    assert_eq!(store.skill_adjustments(120000 * MS).unwrap().shielding, 0);
    assert_eq!(store.skill_adjustments(120999 * MS).unwrap().magic_level, 1);
    assert_eq!(
        store.skill_adjustments(121000 * MS).unwrap(),
        SkillAdjustments::default()
    );
}
#[test]
fn party_and_ordinary_regeneration_keep_distinct_source_subids() {
    let root = root();
    let mut store = ConditionStore::<u32>::new();
    for id in [0, 1] {
        let d = def(
            &format!("source.regen.{id}"),
            ConditionValues::SpellRegeneration {
                sub_id: id,
                duration_ms: 10000,
                health_gain: 20,
                health_interval_ms: 2000,
                mana_gain: 0,
                mana_interval_ms: 2000,
            },
        );
        apply(
            &mut store,
            &d,
            ConditionSourceKind::SelfUse,
            &facts(0, &root),
        )
        .unwrap();
    }
    assert_eq!(store.instances().len(), 2);
    let due = store.take_due(2000 * MS, TickFacts::default());
    assert_eq!(due.len(), 2);
    assert!(due.iter().all(|tick| matches!(
        tick.kind,
        TickKind::SpellRegeneration {
            health_gain: 20,
            mana_gain: 0,
            suppressed: false
        }
    )));
}
#[test]
fn qualified_curse_seventeen_groups_retain_all_fifty_five_occurrences() {
    let root = root();
    let mut store = ConditionStore::<u32>::new();
    let mut steps = [DotSequenceStep::default(); MAX_DOT_SEQUENCE_STEPS];
    let groups = [
        (45, 1),
        (40, 1),
        (35, 1),
        (34, 1),
        (33, 2),
        (32, 2),
        (31, 2),
        (30, 2),
        (29, 3),
        (25, 3),
        (24, 3),
        (23, 4),
        (20, 4),
        (19, 5),
        (15, 5),
        (10, 6),
        (5, 10),
    ];
    for (slot, (amount, repetitions)) in steps.iter_mut().zip(groups) {
        *slot = DotSequenceStep {
            amount,
            repetitions,
            interval_ms: 3000,
        };
    }
    let d = def(
        "source.curse.qualified17",
        ConditionValues::DamageSequence {
            element: DotElement::Cursed,
            steps,
            len: 17,
            delayed: true,
        },
    );
    apply(
        &mut store,
        &d,
        ConditionSourceKind::SelfUse,
        &facts(0, &root),
    )
    .unwrap();
    let mut total = 0;
    let mut count = 0;
    for index in 1..=55 {
        for tick in store.take_due(index * 3000 * MS, TickFacts::default()) {
            if let TickKind::Damage {
                amount,
                refused: false,
                ..
            } = tick.kind
            {
                total += amount;
                count += 1;
            }
        }
    }
    assert_eq!((count, total), (55, 1092));
    assert!(store.instances().is_empty());
    assert!(
        ConditionDefinition::new(
            "source.curse.bad33",
            1,
            ConditionValues::DamageSequence {
                element: DotElement::Cursed,
                steps,
                len: 33,
                delayed: true
            }
        )
        .is_none()
    );
}
