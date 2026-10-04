#![allow(
    clippy::expect_used,
    reason = "test fixtures and assertions must fail on missing evidence"
)]

use super::*;
use crate::ability::ProposalSource;
use serde_json::{Value, json};

fn actor(seed: u8) -> ExactActorRef {
    let mut bytes = [0; 16];
    bytes[0] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    let world = crate::foundation::WorldId::decode(&bytes).expect("world fixture");
    bytes[15] = seed;
    ExactActorRef::transport_fixture(
        world,
        crate::foundation::ChannelId::decode(&bytes).expect("channel fixture"),
    )
}

fn revisions() -> RevisionSet {
    RevisionSet::new("rules-r1", "content-r1", "world-r1", "formula-r1", "sim-r1")
        .expect("revisions")
}
fn root() -> GameplayDecisionRoot {
    GameplayDecisionRoot::from_bytes([9; 32])
}
fn occurrence(sequence: u64) -> ThinkOccurrence {
    ThinkOccurrence {
        actor: actor(1),
        sequence,
    }
}
fn target(distance: Option<u16>) -> Option<AttackTarget> {
    Some(AttackTarget {
        actor: actor(2),
        same_floor_distance: distance,
    })
}

fn fixture(
    interval: u64,
) -> (
    ProjectV2BehaviorAuthoring,
    BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
) {
    let ability: ProjectV2DefinitionRef = serde_json::from_value(json!({"family":"Ability", "key":"oteryn:ability.schedule-fixture", "revision":"definition-r1"})).expect("reference");
    let behavior = serde_json::from_value(json!({
        "movement":{"can_walk":true,"pass_through":false,"pushable":false,"push_items":false,"push_creatures":false,"walks_on_energy":false,"walks_on_fire":false,"walks_on_poison":false},
        "targeting":{"hostile":true,"can_target":true,"sense_invisible":false,"target_distance_tiles":1,"static_attack_chance_ppm":1000000,"flee_health":0},
        "attacks":[{"ability":ability,"interval_ms":interval,"chance_ppm":1000000,"magnitude":{"minimum":5,"maximum":10}}]
    })).expect("native behavior fixture");
    let profile = serde_json::from_value(json!({"details":{"kind":"Spell","range_tiles":3,"needs_target":true,"needs_direction":false}})).expect("native ability fixture");
    (behavior, BTreeMap::from([(ability, profile)]))
}

fn run(
    state: &mut ProfileScheduleState,
    sequence: u64,
    behavior: &ProjectV2BehaviorAuthoring,
    abilities: &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
    target: Option<AttackTarget>,
) -> Result<SchedulePlan, ScheduleError> {
    state.prepare(
        occurrence(sequence),
        behavior,
        abilities,
        target,
        &revisions(),
        &root(),
    )
}

#[test]
fn native_due_windows_reset_and_retry_are_exact() {
    let (mut behavior, abilities) = fixture(2000);
    behavior.attacks.push(ProjectV2AbilitySchedule {
        interval_ms: 3000,
        ..behavior.attacks[0].clone()
    });
    behavior.defenses.push(ProjectV2AbilitySchedule {
        interval_ms: 1000,
        ..behavior.attacks[0].clone()
    });
    let mut state = ProfileScheduleState::new(actor(1));
    let mut counts = Vec::new();
    for sequence in 0..3 {
        let plan = run(&mut state, sequence, &behavior, &abilities, target(Some(1))).expect("plan");
        assert_eq!(
            plan,
            run(&mut state, sequence, &behavior, &abilities, target(Some(1))).expect("retry")
        );
        assert_eq!(plan.evaluated_defence_ticks_ms, 1000);
        assert_eq!(plan.proposals.last().expect("defence").target, actor(1));
        assert!(
            plan.proposals
                .iter()
                .all(|p| p.intent.proposal_source() == ProposalSource::Ai && p.issuer == actor(1))
        );
        counts.push(plan.proposals.len());
    }
    assert_eq!(counts, [1, 2, 2]);
    assert_eq!(state.attack_ticks_ms, 0);
    assert_eq!(
        run(&mut state, 2, &behavior, &abilities, target(Some(2))),
        Err(ScheduleError::OccurrenceConflict)
    );
    assert_eq!(
        run(&mut state, 1, &behavior, &abilities, target(Some(1))),
        Err(ScheduleError::OccurrenceSuperseded)
    );
    assert_eq!(
        state.prepare(
            ThinkOccurrence {
                actor: actor(2),
                sequence: 3
            },
            &behavior,
            &abilities,
            target(Some(1)),
            &revisions(),
            &root()
        ),
        Err(ScheduleError::ActorMismatch)
    );
    assert_eq!(
        state.prepare(
            occurrence(2),
            &behavior,
            &abilities,
            target(Some(1)),
            &revisions(),
            &GameplayDecisionRoot::from_bytes([10; 32])
        ),
        Err(ScheduleError::OccurrenceConflict)
    );
}

#[test]
fn native_reference_revision_and_invalid_values_fail_closed() {
    for invalid in 0..4 {
        let (mut behavior, abilities) = fixture(1000);
        match invalid {
            0 => behavior.attacks[0].ability.revision = "definition-r2".to_owned(),
            1 => behavior.attacks[0].interval_ms = 0,
            2 => behavior.attacks[0].chance_ppm = 1_000_001,
            _ => {
                behavior.attacks[0].magnitude = Some(ProjectV2Magnitude {
                    minimum: 2,
                    maximum: 1,
                })
            }
        }
        let mut state = ProfileScheduleState::new(actor(1));
        let result = run(&mut state, 0, &behavior, &abilities, target(Some(1)));
        assert_eq!(
            result,
            Err(if invalid == 0 {
                ScheduleError::MissingAbilityProfile
            } else {
                ScheduleError::InvalidSchedule
            })
        );
        assert_eq!(state.attack_ticks_ms, 0);
        assert_eq!(
            result,
            run(&mut state, 0, &behavior, &abilities, target(Some(1)))
        );
    }
}

#[test]
fn native_range_floor_targeting_and_chance_are_enforced() {
    for (range, distance, hostile, can_target, chance, expected) in [
        (None, Some(4), true, true, 1_000_000, 0),
        (Some(5), Some(4), true, true, 1_000_000, 1),
        (Some(0), Some(99), true, true, 1_000_000, 1),
        (Some(0), None, true, true, 1_000_000, 0),
        (None, Some(1), false, true, 1_000_000, 0),
        (None, Some(1), true, false, 1_000_000, 0),
        (None, Some(1), true, true, 0, 0),
    ] {
        let (mut behavior, abilities) = fixture(1000);
        behavior.attacks[0].range_tiles = range;
        behavior.attacks[0].chance_ppm = chance;
        behavior.targeting.hostile = hostile;
        behavior.targeting.can_target = can_target;
        let plan = run(
            &mut ProfileScheduleState::new(actor(1)),
            0,
            &behavior,
            &abilities,
            target(distance),
        )
        .expect("plan");
        assert_eq!(plan.proposals.len(), expected);
    }
}

#[test]
fn melee_budget_and_unavailable_summon_clock_are_explicit() {
    let (mut behavior, mut abilities) = fixture(1000);
    behavior.attacks = vec![behavior.attacks[0].clone(); MAX_ATTACK_ENTRIES];
    behavior.defenses = vec![behavior.attacks[0].clone(); MAX_DEFENCE_ENTRIES];
    let plan = run(
        &mut ProfileScheduleState::new(actor(1)),
        0,
        &behavior,
        &abilities,
        target(Some(1)),
    )
    .expect("24 proposals");
    let ids: std::collections::BTreeSet<_> = plan
        .proposals
        .iter()
        .map(|p| p.occurrence.id().as_str())
        .collect();
    assert_eq!(ids.len(), 24);
    behavior.attacks.push(behavior.attacks[0].clone());
    assert_eq!(
        run(
            &mut ProfileScheduleState::new(actor(1)),
            0,
            &behavior,
            &abilities,
            target(Some(1))
        ),
        Err(ScheduleError::TooManyEntries)
    );
    behavior.attacks.pop();
    behavior.defenses.push(behavior.defenses[0].clone());
    assert_eq!(
        run(
            &mut ProfileScheduleState::new(actor(1)),
            0,
            &behavior,
            &abilities,
            target(Some(1))
        ),
        Err(ScheduleError::TooManyEntries)
    );
    behavior.defenses.pop();
    abilities
        .values_mut()
        .next()
        .expect("ability")
        .details
        .as_mut()
        .expect("details")
        .kind = ProjectV2AbilityKind::Melee;
    assert_eq!(
        run(
            &mut ProfileScheduleState::new(actor(1)),
            0,
            &behavior,
            &abilities,
            target(Some(1))
        ),
        Err(ScheduleError::InvalidDefenceMelee)
    );
    behavior.defenses.clear();
    let plan = run(
        &mut ProfileScheduleState::new(actor(1)),
        0,
        &behavior,
        &abilities,
        target(Some(1)),
    )
    .expect("melee delegated");
    assert!(plan.proposals.is_empty());
    assert_eq!(plan.melee_delegated, 16);
    behavior.summons = Some(
        serde_json::from_value(json!({"max_summons":1,"entries":[]})).expect("summon fixture"),
    );
    assert_eq!(
        run(
            &mut ProfileScheduleState::new(actor(1)),
            0,
            &behavior,
            &abilities,
            target(Some(1))
        ),
        Err(ScheduleError::SummonClockDependency)
    );
    behavior.summons = None;
    let mut state = ProfileScheduleState::new(actor(1));
    state.attack_ticks_ms = u64::MAX;
    assert_eq!(
        run(&mut state, 0, &behavior, &abilities, None),
        Err(ScheduleError::TickOverflow)
    );
}

fn native_population() -> Value {
    let path = std::env::var_os("OTERYN_MONSTER_SCHEDULE_PROFILES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
                "../../docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json",
            )
        });
    serde_json::from_slice(&std::fs::read(path).expect("native profiles input")).expect("stage")
}

fn native_abilities(rows: &[Value]) -> BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring> {
    rows.iter()
        .filter(|row| row["target"]["family"] == "Ability")
        .map(|row| {
            (
                serde_json::from_value(row["target"].clone()).expect("reference"),
                serde_json::from_value(row["data"]["profile"].clone()).expect("native ability"),
            )
        })
        .collect()
}

#[test]
fn prepared_native_population_uses_real_profile_consumer() {
    let stage = native_population();
    let rows = stage["authoring_profiles"].as_array().expect("profiles");
    let abilities = native_abilities(rows);
    let mut count = 0;
    let mut blocked_summons = 0;
    let mut blocked_defence_melee = 0;
    let mut proposals = 0;
    let mut delegated_melee = 0;
    for row in rows
        .iter()
        .filter(|row| row["target"]["family"] == "Behavior")
    {
        let behavior: ProjectV2BehaviorAuthoring =
            serde_json::from_value(row["data"]["profile"].clone()).expect("native behavior");
        count += 1;
        let mut state = ProfileScheduleState::new(actor(1));
        for sequence in 0..8 {
            let result = run(&mut state, sequence, &behavior, &abilities, target(Some(1)));
            assert_eq!(
                result,
                run(&mut state, sequence, &behavior, &abilities, target(Some(1)))
            );
            if behavior.summons.is_some() {
                assert_eq!(result, Err(ScheduleError::SummonClockDependency));
                if sequence == 0 {
                    blocked_summons += 1;
                }
            } else if behavior.defenses.iter().any(|entry| {
                abilities
                    .get(&entry.ability)
                    .and_then(|a| a.details.as_deref())
                    .is_some_and(|d| d.kind == ProjectV2AbilityKind::Melee)
            }) {
                assert_eq!(result, Err(ScheduleError::InvalidDefenceMelee));
                if sequence == 0 {
                    blocked_defence_melee += 1;
                }
            } else {
                let plan = result.expect("actual native profile consumer");
                assert!(
                    plan.proposals
                        .iter()
                        .all(|p| abilities.contains_key(&p.ability)
                            && p.intent.proposal_source() == ProposalSource::Ai)
                );
                proposals += plan.proposals.len();
                delegated_melee += plan.melee_delegated;
            }
        }
    }
    assert_eq!(Some(count), stage["counts"]["creatures"].as_u64());
    assert!(proposals > 0);
    eprintln!(
        "native schedule: profiles={count} summon_dependency={blocked_summons} defence_melee_dependency={blocked_defence_melee} proposals={proposals} delegated_melee={delegated_melee}"
    );
}

fn summon_fixture() -> (
    ProjectV2BehaviorAuthoring,
    BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
) {
    let (mut behavior, abilities) = fixture(1000);
    behavior.defenses.push(behavior.attacks[0].clone());
    behavior.summons = Some(serde_json::from_value(json!({"max_summons":2,"entries":[{
        "creature":{"family":"Creature","key":"oteryn:creature.summon-fixture","revision":"definition-r1"},
        "interval_ms":3000,"chance_ppm":1000000,"count":1
    }]})).expect("native summons"));
    (behavior, abilities)
}

fn summon_facts(behavior: &ProjectV2BehaviorAuthoring, sequence: u64) -> MonsterSummonFacts {
    MonsterSummonFacts {
        occurrence: occurrence(sequence),
        is_summon: false,
        target_with_path: Some(actor(2)),
        total_count: 0,
        entry_counts: behavior
            .summons
            .as_ref()
            .expect("summons")
            .entries
            .iter()
            .map(|entry| ObservedSummonCount {
                creature: entry.creature.clone(),
                count: 0,
            })
            .collect(),
    }
}

fn summon_run(
    state: &mut ProfileScheduleState,
    behavior: &ProjectV2BehaviorAuthoring,
    abilities: &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
    facts: &MonsterSummonFacts,
) -> Result<SchedulePlan, ScheduleError> {
    state.prepare_with_summons(
        ProfileScheduleInput {
            occurrence: facts.occurrence,
            behavior,
            abilities,
            target: target(Some(1)),
            revisions: &revisions(),
            root: &root(),
        },
        Some(facts),
    )
}

#[test]
fn explicit_summon_facts_share_defence_clock_and_preserve_retry() {
    let (behavior, abilities) = summon_fixture();
    let mut state = ProfileScheduleState::new(actor(1));
    for sequence in 0..3 {
        let facts = summon_facts(&behavior, sequence);
        let plan = summon_run(&mut state, &behavior, &abilities, &facts).expect("shared clock");
        assert_eq!(
            plan,
            summon_run(&mut state, &behavior, &abilities, &facts).expect("same retry")
        );
        assert_eq!(plan.evaluated_attack_ticks_ms, 1000);
        assert_eq!(plan.evaluated_defence_ticks_ms, (sequence + 1) * 1000);
        assert_eq!(plan.proposals.len(), 2);
        if sequence < 2 {
            assert_eq!(plan.defence_ticks_after_ms, (sequence + 1) * 1000);
            assert!(plan.summon_proposals.is_empty());
        } else {
            assert_eq!(plan.defence_ticks_after_ms, 0);
            let proposal = plan.summon_proposals.first().expect("typed summon request");
            assert_eq!(proposal.occurrence, facts.occurrence);
            assert_eq!(proposal.revisions, revisions());
            assert_eq!(proposal.entry_index, 0);
            assert_eq!(proposal.target_with_path, actor(2));
            assert_eq!(proposal.creature, facts.entry_counts[0].creature);
            assert_eq!(
                (proposal.maximum_total, proposal.maximum_of_creature),
                (2, 1)
            );
            let mut changed = facts.clone();
            changed.total_count = 1;
            assert_eq!(
                summon_run(&mut state, &behavior, &abilities, &changed),
                Err(ScheduleError::OccurrenceConflict)
            );
        }
    }
    let mut facts = summon_facts(&behavior, 3);
    facts.target_with_path = None;
    let plan =
        summon_run(&mut state, &behavior, &abilities, &facts).expect("no summon eligibility");
    assert_eq!(plan.defence_ticks_after_ms, 0);
    assert!(plan.summon_proposals.is_empty());
}

#[test]
fn summon_facts_bind_occurrence_target_revision_and_counts() {
    let (behavior, abilities) = summon_fixture();
    for mutation in 0..8 {
        let mut facts = summon_facts(&behavior, 0);
        match mutation {
            0 => facts.occurrence.actor = actor(2),
            1 => facts.target_with_path = Some(actor(3)),
            2 => facts.entry_counts[0].creature.revision = "definition-r2".into(),
            3 => facts.entry_counts.clear(),
            4 => facts.entry_counts[0].count = 1,
            5 => facts.total_count = MAX_MONSTER_SUMMONS + 1,
            6 => facts.entry_counts[0].creature.family = ProjectV2Family::Ability,
            _ => facts.occurrence.sequence = 1,
        }
        let result = ProfileScheduleState::new(actor(1)).prepare_with_summons(
            ProfileScheduleInput {
                occurrence: occurrence(0),
                behavior: &behavior,
                abilities: &abilities,
                target: target(Some(1)),
                revisions: &revisions(),
                root: &root(),
            },
            Some(&facts),
        );
        assert_eq!(result, Err(ScheduleError::InvalidSummonFacts));
    }
}

#[test]
fn summon_eligibility_and_accepted_eight_sixteen_limits_are_exact() {
    let (mut behavior, abilities) = summon_fixture();
    for mode in 0..3 {
        let mut facts = summon_facts(&behavior, 0);
        match mode {
            0 => facts.is_summon = true,
            1 => facts.total_count = 2,
            _ => facts.target_with_path = None,
        }
        let plan = summon_run(
            &mut ProfileScheduleState::new(actor(1)),
            &behavior,
            &abilities,
            &facts,
        )
        .expect("ineligible keeps defence");
        assert_eq!(plan.proposals.len(), 2);
        assert_eq!(plan.defence_ticks_after_ms, 0);
        assert!(plan.summon_proposals.is_empty());
    }
    let summons = behavior.summons.as_mut().expect("summons");
    summons.max_summons = 16;
    summons.entries[0].interval_ms = 1000;
    summons.entries[0].count = 16;
    summons.entries = vec![summons.entries[0].clone(); 8];
    let facts = summon_facts(&behavior, 0);
    let plan = summon_run(
        &mut ProfileScheduleState::new(actor(1)),
        &behavior,
        &abilities,
        &facts,
    )
    .expect("8 entries and16 cap");
    assert_eq!(plan.summon_proposals.len(), 8);
    let ninth = behavior.summons.as_ref().expect("summons").entries[0].clone();
    behavior
        .summons
        .as_mut()
        .expect("summons")
        .entries
        .push(ninth);
    let facts = summon_facts(&behavior, 0);
    assert_eq!(
        summon_run(
            &mut ProfileScheduleState::new(actor(1)),
            &behavior,
            &abilities,
            &facts
        ),
        Err(ScheduleError::TooManyEntries)
    );
}

#[test]
fn summon_parameters_reject_invalid_content_and_keep_explicit_zero_counts() {
    let (original, abilities) = summon_fixture();
    for mutation in 0..10 {
        let mut behavior = original.clone();
        let summons = behavior.summons.as_mut().expect("summons");
        match mutation {
            0 => summons.max_summons = 0,
            1 => summons.max_summons = 17,
            2 => summons.entries[0].interval_ms = 0,
            3 => summons.entries[0].chance_ppm = 1_000_001,
            4 => summons.entries[0].count = 0,
            5 => summons.entries[0].count = 17,
            6 => summons.entries[0].creature.family = ProjectV2Family::Ability,
            7 => summons.entries.clear(),
            8 => behavior.targeting.hostile = false,
            _ => behavior.targeting.can_target = false,
        }
        let facts = summon_facts(&behavior, 0);
        let mut state = ProfileScheduleState::new(actor(1));
        assert_eq!(
            summon_run(&mut state, &behavior, &abilities, &facts),
            Err(ScheduleError::InvalidSummonFacts)
        );
        assert_eq!((state.attack_ticks_ms, state.defence_ticks_ms), (0, 0));
    }
    for recipient in [None, target(None)] {
        let facts = summon_facts(&original, 0);
        let result = ProfileScheduleState::new(actor(1)).prepare_with_summons(
            ProfileScheduleInput {
                occurrence: occurrence(0),
                behavior: &original,
                abilities: &abilities,
                target: recipient,
                revisions: &revisions(),
                root: &root(),
            },
            Some(&facts),
        );
        assert_eq!(result, Err(ScheduleError::InvalidSummonFacts));
    }
    let mut behavior = original;
    let extra = behavior.summons.as_ref().expect("summons").entries[0].clone();
    behavior
        .summons
        .as_mut()
        .expect("summons")
        .entries
        .push(extra);
    let mut facts = summon_facts(&behavior, 0);
    facts.total_count = 1;
    facts.entry_counts[1].count = 1;
    assert_eq!(
        summon_run(
            &mut ProfileScheduleState::new(actor(1)),
            &behavior,
            &abilities,
            &facts
        ),
        Err(ScheduleError::InvalidSummonFacts)
    );
    facts.entry_counts[0].count = 1;
    let mut state = ProfileScheduleState::new(actor(1));
    let waiting = summon_run(&mut state, &behavior, &abilities, &facts)
        .expect("capped entries still wait for due");
    assert_eq!(waiting.defence_ticks_after_ms, 1000);
    for sequence in 1..3 {
        facts.occurrence = occurrence(sequence);
        let plan =
            summon_run(&mut state, &behavior, &abilities, &facts).expect("same counted creature");
        assert!(plan.summon_proposals.is_empty());
    }
    assert_eq!(state.defence_ticks_ms, 0);
}

#[test]
fn summon_draws_use_exact_purpose_entry_and_due_window() {
    let (mut behavior, abilities) = summon_fixture();
    let summons = behavior.summons.as_mut().expect("summons");
    summons.entries[0].interval_ms = 1500;
    summons.entries[0].chance_ppm = 500_000;
    let mut state = ProfileScheduleState::new(actor(1));
    for sequence in 0..6 {
        let facts = summon_facts(&behavior, sequence);
        let plan =
            summon_run(&mut state, &behavior, &abilities, &facts).expect("native due window");
        let ticks = if sequence % 2 == 0 { 1000 } else { 2000 };
        assert_eq!(plan.evaluated_defence_ticks_ms, ticks);
        let decision = deterministic_decision_u64(
            &root(),
            decision_occurrence(facts.occurrence),
            "AI_SUMMON",
            0,
        )
        .expect("purpose accepted");
        assert_eq!(
            plan.summon_proposals.len(),
            usize::from(ticks == 2000 && decision % 1_000_000 < 500_000)
        );
    }
    behavior.summons.as_mut().expect("summons").entries[0].chance_ppm = 0;
    let facts = summon_facts(&behavior, 6);
    let first = summon_run(&mut state, &behavior, &abilities, &facts).expect("zero chance waits");
    assert!(first.summon_proposals.is_empty());
    let facts = summon_facts(&behavior, 7);
    let due = summon_run(&mut state, &behavior, &abilities, &facts).expect("zero chance due");
    assert!(due.summon_proposals.is_empty());
    assert_eq!(due.defence_ticks_after_ms, 0);
}

#[test]
fn prepared_native_summon_population_consumes_explicit_owner_fixture_facts() {
    let stage = native_population();
    let rows = stage["authoring_profiles"].as_array().expect("profiles");
    let abilities = native_abilities(rows);
    let creatures: std::collections::BTreeSet<ProjectV2DefinitionRef> = rows
        .iter()
        .filter(|row| row["target"]["family"] == "Creature")
        .map(|row| serde_json::from_value(row["target"].clone()).expect("creature ref"))
        .collect();
    let mut count = 0;
    let mut requests = 0;
    for row in rows
        .iter()
        .filter(|row| row["target"]["family"] == "Behavior")
    {
        let behavior: ProjectV2BehaviorAuthoring =
            serde_json::from_value(row["data"]["profile"].clone()).expect("native behavior");
        if behavior.summons.is_none() {
            continue;
        }
        count += 1;
        let mut state = ProfileScheduleState::new(actor(1));
        // Explicit independent fixture eligibility; not resolved production authority.
        for sequence in 0..32 {
            let facts = summon_facts(&behavior, sequence);
            let plan = summon_run(&mut state, &behavior, &abilities, &facts)
                .expect("native summon profile with explicit facts");
            assert_eq!(
                plan,
                summon_run(&mut state, &behavior, &abilities, &facts).expect("native retry")
            );
            for proposal in &plan.summon_proposals {
                let entry =
                    &behavior.summons.as_ref().expect("summons").entries[proposal.entry_index];
                assert_eq!(proposal.creature, entry.creature);
                assert_eq!(proposal.occurrence, facts.occurrence);
                assert_eq!(proposal.revisions, revisions());
                assert_eq!(proposal.target_with_path, actor(2));
                assert_eq!(proposal.maximum_of_creature, entry.count);
                assert!(creatures.contains(&proposal.creature));
            }
            requests += plan.summon_proposals.len();
        }
    }
    assert!(count > 0 && requests > 0);
    eprintln!(
        "native summon preparation: profiles={count} thinks_per_profile=32 requests={requests}; explicit fixture facts only; owner birth/authority not exercised"
    );
}
