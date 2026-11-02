//! Crystal 201189f monster.cpp:1188-1242, 1263-1292; no independent melee clock.
use super::*;

fn melee_fixture(
    interval_ms: u64,
) -> (
    ProjectV2BehaviorAuthoring,
    BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
) {
    let (behavior, mut abilities) = fixture(interval_ms);
    let details = abilities
        .values_mut()
        .next()
        .expect("ability")
        .details
        .as_mut()
        .expect("details");
    details.kind = ProjectV2AbilityKind::Melee;
    details.range_tiles = 1;
    (behavior, abilities)
}

fn at(
    state: &mut ProfileScheduleState,
    sequence: u64,
    behavior: &ProjectV2BehaviorAuthoring,
    abilities: &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
    target: Option<AttackTarget>,
    facts: NativeAttackFacts,
) -> SchedulePlan {
    state
        .prepare_native_with_summons(
            ProfileScheduleInput {
                occurrence: occurrence(sequence),
                behavior,
                abilities,
                target,
                revisions: &revisions(),
                root: &root(),
            },
            None,
            facts,
        )
        .expect("native source schedule")
}

fn facts(now_us: u64) -> NativeAttackFacts {
    NativeAttackFacts {
        now_us,
        fleeing: false,
        target_disappeared: false,
    }
}

#[test]
fn melee_participates_in_the_same_longest_attack_window_as_spells() {
    let (mut behavior, mut abilities) = melee_fixture(3000);
    let (spell_behavior, spell_abilities) = fixture(2000);
    let mut spell = spell_behavior.attacks[0].clone();
    spell.ability.key = "oteryn:ability.shared-clock-spell".into();
    abilities.insert(
        spell.ability.clone(),
        spell_abilities.values().next().expect("spell").clone(),
    );
    behavior.attacks.push(spell);
    let mut state = ProfileScheduleState::new(actor(1));
    let one = at(
        &mut state,
        1,
        &behavior,
        &abilities,
        target(Some(1)),
        facts(0),
    );
    assert_eq!(one.attack_ticks_after_ms, 1000);
    assert!(one.native_melee_selected.is_empty() && one.proposals.is_empty());
    let two = at(
        &mut state,
        2,
        &behavior,
        &abilities,
        target(Some(1)),
        facts(1_000_000),
    );
    assert_eq!(
        two.attack_ticks_after_ms, 2000,
        "waiting melee retains the shared clock"
    );
    assert_eq!(two.proposals.len(), 1);
    assert_eq!(two.proposals[0].entry_index, 1);
    let three = at(
        &mut state,
        3,
        &behavior,
        &abilities,
        target(Some(1)),
        facts(2_000_000),
    );
    assert_eq!(three.native_melee_selected, [0]);
    assert!(three.proposals.is_empty());
    assert_eq!(three.attack_ticks_after_ms, 0);
    assert!(state.native_melee_selected(3, 0));
    assert!(!state.native_melee_selected(2, 0));
    let four = at(
        &mut state,
        4,
        &behavior,
        &abilities,
        target(Some(1)),
        facts(3_000_000),
    );
    assert_eq!(four.evaluated_attack_ticks_ms, 1000);
    assert!(four.native_melee_selected.is_empty());
}

#[test]
fn only_a_due_out_of_reach_melee_grants_an_extra_swing() {
    let (behavior, abilities) = melee_fixture(4000);
    let mut state = ProfileScheduleState::new(actor(1));
    for sequence in 1..=3 {
        let plan = at(
            &mut state,
            sequence,
            &behavior,
            &abilities,
            target(Some(2)),
            facts(sequence * 1_000_000),
        );
        assert!(!plan.extra_melee_after);
        assert!(plan.native_melee_selected.is_empty());
    }
    let missed = at(
        &mut state,
        4,
        &behavior,
        &abilities,
        target(Some(2)),
        facts(4_000_000),
    );
    assert!(missed.extra_melee_after);
    assert_eq!(missed.attack_ticks_after_ms, 0);
    let extra = at(
        &mut state,
        5,
        &behavior,
        &abilities,
        target(Some(1)),
        facts(5_000_000),
    );
    assert_eq!(extra.evaluated_attack_ticks_ms, 1000);
    assert_eq!(
        extra.native_melee_selected,
        [0],
        "extra bypasses the authored 4000 ms window"
    );
    assert!(!extra.extra_melee_after);
    assert_eq!(extra.last_extra_melee_us_after, Some(5_000_000));
    assert_eq!(
        extra,
        at(
            &mut state,
            5,
            &behavior,
            &abilities,
            target(Some(1)),
            facts(99_000_000)
        ),
        "retry does not advance the clock or re-evaluate the extra swing"
    );
}

#[test]
fn threshold_is_since_the_last_extra_attempt_not_each_regular_cast() {
    let (behavior, abilities) = melee_fixture(500);
    let mut state = ProfileScheduleState::new(actor(1));
    let missed = at(
        &mut state,
        1,
        &behavior,
        &abilities,
        target(Some(2)),
        facts(0),
    );
    assert!(missed.extra_melee_after);
    for (sequence, now_us, selected) in [
        (2, 1_000_000, true),
        (3, 2_000_000, false),
        (4, 2_499_999, false),
        (5, 2_500_000, true),
        (6, 3_000_000, true),
    ] {
        let plan = at(
            &mut state,
            sequence,
            &behavior,
            &abilities,
            target(Some(1)),
            facts(now_us),
        );
        assert_eq!(
            !plan.native_melee_selected.is_empty(),
            selected,
            "at {now_us}"
        );
        assert_eq!(plan.last_extra_melee_us_after, Some(1_000_000));
    }
}

#[test]
fn a_failed_extra_chance_remains_pending_and_refreshes_its_timestamp() {
    let (mut behavior, abilities) = melee_fixture(500);
    behavior.attacks[0].chance_ppm = 0;
    let mut state = ProfileScheduleState::new(actor(1));
    at(
        &mut state,
        1,
        &behavior,
        &abilities,
        target(Some(2)),
        facts(0),
    );
    for sequence in 2..=3 {
        let plan = at(
            &mut state,
            sequence,
            &behavior,
            &abilities,
            target(Some(1)),
            facts(sequence * 1_000_000),
        );
        assert!(plan.native_melee_selected.is_empty());
        assert!(plan.extra_melee_after);
        assert_eq!(plan.last_extra_melee_us_after, Some(sequence * 1_000_000));
    }
    behavior.attacks[0].chance_ppm = 1_000_000;
    let selected = at(
        &mut state,
        4,
        &behavior,
        &abilities,
        target(Some(1)),
        facts(3_000_001),
    );
    assert_eq!(selected.native_melee_selected, [0]);
    assert!(!selected.extra_melee_after);
}

#[test]
fn disappearing_target_resets_the_clock_and_grants_extra_on_reacquisition() {
    let (behavior, abilities) = melee_fixture(4000);
    let mut state = ProfileScheduleState::new(actor(1));
    let waiting = at(
        &mut state,
        1,
        &behavior,
        &abilities,
        target(Some(1)),
        facts(0),
    );
    assert_eq!(waiting.attack_ticks_after_ms, 1000);
    let lost = at(
        &mut state,
        2,
        &behavior,
        &abilities,
        None,
        NativeAttackFacts {
            target_disappeared: true,
            ..facts(1_000_000)
        },
    );
    assert_eq!(lost.attack_ticks_after_ms, 0);
    assert!(lost.extra_melee_after);
    let acquired = at(
        &mut state,
        3,
        &behavior,
        &abilities,
        target(Some(1)),
        facts(2_000_000),
    );
    assert_eq!(acquired.native_melee_selected, [0]);
    assert_eq!(acquired.evaluated_attack_ticks_ms, 1000);
}

#[test]
fn fleeing_skips_only_melee_and_preserves_pending_extra_for_healing() {
    let (mut behavior, mut abilities) = melee_fixture(4000);
    let (spell_behavior, spell_abilities) = fixture(1000);
    let mut spell = spell_behavior.attacks[0].clone();
    spell.ability.key = "oteryn:ability.fleeing-spell".into();
    abilities.insert(
        spell.ability.clone(),
        spell_abilities.values().next().expect("spell").clone(),
    );
    behavior.attacks.push(spell);
    let mut state = ProfileScheduleState::new(actor(1));
    let fleeing = at(
        &mut state,
        1,
        &behavior,
        &abilities,
        target(Some(1)),
        NativeAttackFacts {
            now_us: 1_000_000,
            fleeing: true,
            target_disappeared: true,
        },
    );
    assert!(fleeing.native_melee_selected.is_empty());
    assert_eq!(fleeing.proposals.len(), 1);
    assert_eq!(fleeing.proposals[0].entry_index, 1);
    assert!(fleeing.extra_melee_after);
    assert_eq!(
        fleeing.last_extra_melee_us_after,
        Some(1_000_000),
        "spells also refresh a pending extra timestamp"
    );
    let healed = at(
        &mut state,
        2,
        &behavior,
        &abilities,
        target(Some(1)),
        facts(2_000_000),
    );
    assert_eq!(healed.native_melee_selected, [0]);
}

#[test]
fn all_melee_source_indices_are_evaluated_in_authored_order() {
    let (mut behavior, abilities) = melee_fixture(1000);
    behavior.attacks.push(behavior.attacks[0].clone());
    let regular = at(
        &mut ProfileScheduleState::new(actor(1)),
        1,
        &behavior,
        &abilities,
        target(Some(1)),
        facts(0),
    );
    assert_eq!(regular.native_melee_selected, [0, 1]);
    let extra = at(
        &mut ProfileScheduleState::new(actor(1)),
        1,
        &behavior,
        &abilities,
        target(Some(1)),
        NativeAttackFacts {
            target_disappeared: true,
            ..facts(0)
        },
    );
    assert_eq!(
        extra.native_melee_selected,
        [0],
        "first extra cast clears the extra flag; the second melee meets its 1500 ms guard"
    );
}

#[test]
fn movement_interval_zero_keeps_both_clocks_and_skips_spells_defences_and_summons() {
    let (mut behavior, mut abilities) = melee_fixture(1000);
    let (spell_behavior, spell_abilities) = fixture(1000);
    let mut spell = spell_behavior.attacks[0].clone();
    spell.ability.key = "oteryn:ability.movement-spell".into();
    abilities.insert(
        spell.ability.clone(),
        spell_abilities.values().next().expect("spell").clone(),
    );
    behavior.attacks.push(spell.clone());
    spell.interval_ms = 4000;
    behavior.defenses.push(spell);
    let summon = ProjectV2DefinitionRef {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.movement-summon".into(),
        revision: "definition-r1".into(),
    };
    behavior.summons = Some(
        serde_json::from_value(serde_json::json!({
            "max_summons":1,
            "entries":[{"creature":summon,"interval_ms":4000,"chance_ppm":1_000_000,"count":1}]
        }))
        .expect("source summon"),
    );
    let mut state = ProfileScheduleState::new(actor(1));
    let initial = state
        .prepare_native_with_summons(
            ProfileScheduleInput {
                occurrence: occurrence(1),
                behavior: &behavior,
                abilities: &abilities,
                target: target(Some(2)),
                revisions: &revisions(),
                root: &root(),
            },
            Some(&MonsterSummonFacts {
                occurrence: occurrence(1),
                is_summon: false,
                target_with_path: Some(actor(2)),
                total_count: 0,
                entry_counts: vec![ObservedSummonCount {
                    creature: summon,
                    count: 0,
                }],
            }),
            facts(0),
        )
        .expect("full native think");
    assert!(initial.extra_melee_after);
    assert_eq!(initial.attack_ticks_after_ms, 0);
    assert_eq!(initial.defence_ticks_after_ms, 1000);
    let extra = state
        .prepare_movement_melee_retry(
            occurrence(2),
            &behavior,
            &abilities,
            target(Some(1)).expect("target"),
            facts(100_000),
        )
        .expect("movement retry does not require a fresh summon path read");
    assert_eq!(extra.native_melee_selected, [0]);
    assert!(extra.proposals.is_empty() && extra.summon_proposals.is_empty());
    assert_eq!(extra.attack_ticks_after_ms, initial.attack_ticks_after_ms);
    assert_eq!(extra.defence_ticks_after_ms, initial.defence_ticks_after_ms);
    assert_eq!(
        extra,
        state
            .prepare_movement_melee_retry(
                occurrence(2),
                &behavior,
                &abilities,
                target(Some(1)).expect("target"),
                facts(999_999),
            )
            .expect("same occurrence is the first result")
    );
    let later = state
        .prepare_movement_melee_retry(
            occurrence(3),
            &behavior,
            &abilities,
            target(Some(1)).expect("target"),
            facts(5_000_000),
        )
        .expect("no extra remains");
    assert!(later.native_melee_selected.is_empty() && later.proposals.is_empty());
    assert_eq!(
        (later.attack_ticks_after_ms, later.defence_ticks_after_ms),
        (0, 1000)
    );
}

#[test]
fn regular_melee_selection_can_be_consumed_before_a_second_range_grants_extra() {
    let (mut behavior, abilities) = melee_fixture(1000);
    // Explicit source-owner fixture: a prior longer-reach entry may cast while a later
    // short-reach entry grants extra. Production must consume this plan before preparing
    // the movement plan; a new occurrence deliberately supersedes its selection.
    behavior.attacks[0].range_tiles = Some(2);
    let mut short = behavior.attacks[0].clone();
    short.range_tiles = Some(1);
    behavior.attacks.push(short);
    let mut state = ProfileScheduleState::new(actor(1));
    let regular = at(
        &mut state,
        1,
        &behavior,
        &abilities,
        target(Some(2)),
        facts(0),
    );
    assert_eq!(regular.native_melee_selected, [0]);
    assert!(regular.extra_melee_after);
    assert!(state.native_melee_selected(1, 0));
    let extra = state
        .prepare_movement_melee_retry(
            occurrence(2),
            &behavior,
            &abilities,
            target(Some(1)).expect("target"),
            facts(100_000),
        )
        .expect("movement after regular selection consumption");
    assert_eq!(extra.native_melee_selected, [0]);
    assert_eq!(extra.attack_ticks_after_ms, regular.attack_ticks_after_ms);
    assert!(!state.native_melee_selected(1, 0));
    assert!(state.native_melee_selected(2, 0));
}
