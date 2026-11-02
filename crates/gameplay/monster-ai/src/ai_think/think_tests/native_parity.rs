//! W01a source behavior exercised through the same native projection/think as dispatch.
use super::*;
use crate::content::ProjectV2StrategyWeights;

#[test]
fn native_missing_callback_does_not_erase_known_attack_or_change_source_index() {
    let source = behavior(
        rat_targeting(),
        vec![entry("unknown_callback", 1000, 1), entry("bolt", 1000, 8)],
        vec![],
        None,
    );
    assert_eq!(project(&source), Err(ProfileRefusal::Invalid));
    let profile = CreatureBehaviourProfile::project_native(Some(&source), &abilities())
        .expect("native missing callback is isolated");
    assert_eq!(profile.behavior().attacks, source.attacks);
    let mut state = CreatureAiState::new(rat(), profile, at(100, 100));
    let facts = Facts::new();
    let players = [player(10, at(102, 100))];
    let report = think(&mut state, 1, &facts.at(at(100, 100), 20, &players, 0));
    assert_eq!(report.refusal, None);
    assert_eq!(report.proposals.len(), 1);
    assert_eq!(report.proposals[0].entry_index, 1);
    assert_eq!(report.proposals[0].ability, source.attacks[1].ability);
    assert_eq!(state.attack_facing(), CardinalStep::East);
    assert_eq!(
        report,
        think(&mut state, 1, &facts.at(at(100, 100), 20, &players, 0))
    );
}

#[test]
fn native_malformed_unknown_entry_is_still_refused() {
    let source = behavior(
        rat_targeting(),
        vec![entry("unknown_callback", 0, 8)],
        vec![],
        None,
    );
    assert_eq!(
        CreatureBehaviourProfile::project_native(Some(&source), &abilities()),
        Err(ProfileRefusal::Invalid)
    );
}

#[test]
fn native_repeated_summon_names_retain_distinct_authored_slots_and_chances() {
    // Crystal201189f Eshtaba the Conjurer, lines62-70: five Dread Minion entries.
    let mut source = behavior(rat_targeting(), vec![], vec![], None);
    let entries = [200_000, 300_000, 200_000, 300_000, 200_000].map(|chance| json!({
        "creature":{"family":"Creature","key":"oteryn:creature.dread_minion","revision":"definition-r1"},
        "interval_ms":2000,"chance_ppm":chance,"count":1
    }));
    source.summons = Some(
        serde_json::from_value(json!({
            "max_summons":5, "entries":entries,
        }))
        .expect("source summon schedules"),
    );
    assert_eq!(
        CreatureBehaviourProfile::project_with_summons(Some(&source), &abilities()),
        Err(ProfileRefusal::Invalid),
        "legacy admission retains its unique-creature contract"
    );
    let native = CreatureBehaviourProfile::project_native(Some(&source), &abilities())
        .expect("native authored repeated summon entries");
    assert_eq!(native.behavior().summons, source.summons);
}

#[test]
fn native_melee_source_entries_may_be_multiple_or_have_zero_direct_damage() {
    let source = behavior(
        rat_targeting(),
        vec![entry("bite", 2000, 0), entry("bite", 4000, 8)],
        vec![],
        None,
    );
    assert_eq!(project(&source), Err(ProfileRefusal::Invalid));
    let native = CreatureBehaviourProfile::project_native(Some(&source), &abilities())
        .expect("native AutoAttackOwner reads each source entry");
    assert_eq!(native.behavior().attacks, source.attacks);
    assert_eq!(
        native.melee().map(|entry| (entry.index, entry.maximum)),
        Some((0, 0))
    );
}

#[test]
fn native_defence_melee_retains_its_slot_without_disabling_attack_schedules() {
    // Crystal201189f data-crystal/monster/reptiles/seacrest_serpent.lua:111-116
    // includes a zero-damage melee entry in defenses. Execution remains with the
    // separately admitted defence owner; a native attack profile cannot be dropped.
    let source = behavior(
        rat_targeting(),
        vec![entry("bite", 2000, 8), entry("bolt", 2000, 8)],
        vec![entry("bite", 2000, 0)],
        None,
    );
    assert_eq!(project(&source), Err(ProfileRefusal::Invalid));
    let profile = CreatureBehaviourProfile::project_native(Some(&source), &abilities())
        .expect("native source defence slot");
    assert_eq!(profile.behavior().defenses, source.defenses);
    let mut state = CreatureAiState::new(rat(), profile, at(100, 100));
    let facts = Facts::new();
    let players = [player(10, at(101, 100))];
    let waiting = think(&mut state, 1, &facts.at(at(100, 100), 20, &players, 0));
    assert_eq!(waiting.refusal, None);
    assert!(waiting.proposals.is_empty());
    let due = think(
        &mut state,
        2,
        &facts.at(at(100, 100), 20, &players, 1_000_000),
    );
    assert_eq!(due.refusal, None);
    assert!(state.schedule.native_melee_selected(2, 0));
    assert_eq!(due.proposals.len(), 2);
    assert!(
        due.proposals
            .iter()
            .any(|proposal| proposal.list == ScheduleList::Attack
                && proposal.entry_index == 1
                && proposal.target == players[0].actor)
    );
    assert!(
        due.proposals
            .iter()
            .any(|proposal| proposal.list == ScheduleList::Defence
                && proposal.entry_index == 0
                && proposal.target == rat()
                && proposal.ability == source.defenses[0].ability)
    );
}

#[test]
fn native_percentile_strategy_retains_crystal_remainder_and_damage_quirk() {
    let weights = Some(ProjectV2StrategyWeights {
        nearest: 20,
        health: 30,
        damage: 10,
        random: 0,
    });
    for (draw, strategy) in [
        (0, Strategy::Nearest),
        (19, Strategy::Nearest),
        (20, Strategy::LowestHealth),
        (49, Strategy::LowestHealth),
        (50, Strategy::MostDamage),
        (59, Strategy::MostDamage),
        (60, Strategy::Random),
        (99, Strategy::Random),
    ] {
        assert_eq!(targeting::pick_native_strategy(weights, draw), strategy);
    }
    assert_eq!(targeting::pick_native_strategy(None, 99), Strategy::Nearest);
    assert_eq!(
        targeting::pick_native_strategy(
            Some(ProjectV2StrategyWeights {
                nearest: 0,
                health: 0,
                damage: 0,
                random: 0,
            }),
            0
        ),
        Strategy::Random
    );
    let players = [
        TargetCandidate {
            damage: 1000,
            ..player(10, at(101, 100))
        },
        TargetCandidate {
            damage: 1,
            ..player(11, at(102, 100))
        },
        TargetCandidate {
            damage: 1,
            ..player(12, at(103, 100))
        },
    ];
    // Literal Crystal: the first candidate is the fallback and omitted from comparison.
    assert_eq!(
        targeting::select_native(Strategy::MostDamage, at(100, 100), &players, 0),
        Some(players[1].actor)
    );
    let no_damage = players.map(|player| TargetCandidate {
        damage: 0,
        ..player
    });
    assert_eq!(
        targeting::select_native(Strategy::MostDamage, at(100, 100), &no_damage, 0),
        Some(players[0].actor)
    );
    assert_eq!(
        targeting::select_native(Strategy::MostDamage, at(100, 100), &[], 0),
        None
    );
}

#[test]
fn native_summon_does_not_flee_or_run_wild_timed_target_changes() {
    let mut targeting = rat_targeting();
    targeting["change_target"] = json!({"interval_ms":1000,"chance_ppm":1_000_000});
    let source = behavior(targeting, vec![entry("bite", 2000, 8)], vec![], None);
    let profile = CreatureBehaviourProfile::project_native(Some(&source), &abilities())
        .expect("native profile");
    let mut state = CreatureAiState::new(rat(), profile, at(100, 100));
    state.is_summon = true;
    let facts = Facts::new();
    let players = [player(10, at(102, 100))];
    let report = think(&mut state, 1, &facts.at(at(100, 100), 1, &players, 0));
    assert!(!report.fleeing && !report.target_changed);
    assert!(matches!(report.goal, MovementGoal::Chase { .. }));
}

#[test]
fn native_fleeing_keeps_ranged_spells_and_returns_to_fighting_after_heal() {
    let source = behavior(rat_targeting(), vec![entry("bolt", 1000, 8)], vec![], None);
    let profile = CreatureBehaviourProfile::project_native(Some(&source), &abilities())
        .expect("native profile");
    let mut state = CreatureAiState::new(rat(), profile, at(100, 100));
    let facts = Facts::new();
    let players = [player(10, at(102, 100))];
    let fleeing = think(&mut state, 1, &facts.at(at(100, 100), 5, &players, 0));
    assert!(fleeing.fleeing && state.is_fleeing());
    assert_eq!(fleeing.proposals.len(), 1); // Crystal monster.cpp:1204 only suppresses melee.
    assert!(matches!(fleeing.goal, MovementGoal::Flee { .. }));
    let healed = think(
        &mut state,
        2,
        &facts.at(at(100, 100), 6, &players, 1_000_000),
    );
    assert!(!healed.fleeing && !state.is_fleeing());
    assert!(matches!(healed.goal, MovementGoal::Chase { .. }));
    assert_eq!(healed.proposals.len(), 1);
}

#[test]
fn admitted_native_profiles_keep_every_source_attack_and_defence_slot() {
    use crate::content::{
        ProjectV2AuthoringProfileData, native_gameplay::CreatureProfilesDocument,
    };
    let document: CreatureProfilesDocument = serde_json::from_slice(include_bytes!(
        "../../../../../../content/creatures/definitions/spell-native-profiles.json"
    ))
    .expect("admitted native creature profiles");
    let mut checked = 0;
    let mut refused = Vec::new();
    for record in &document.records {
        let Some(ProjectV2AuthoringProfileData::Behavior(source)) =
            record.behavior.as_ref().map(|p| &p.data)
        else {
            refused.push(format!("{}: missing behavior", record.profile.target.key));
            continue;
        };
        let profile = match super::super::native_profile::project_native_record(record) {
            Ok(profile) => profile,
            Err(error) => {
                refused.push(format!("{}: {error:?}", record.profile.target.key));
                continue;
            }
        };
        assert!(profile.is_native());
        assert_eq!(
            profile.behavior().attacks.len(),
            source.attacks.len(),
            "{}",
            record.profile.target.key
        );
        assert_eq!(
            profile.behavior().defenses.len(),
            source.defenses.len(),
            "{}",
            record.profile.target.key
        );
        assert_eq!(
            profile.behavior().event_bindings,
            source.event_bindings,
            "{}",
            record.profile.target.key
        );
        for (actual, expected) in profile
            .behavior()
            .attacks
            .iter()
            .chain(&profile.behavior().defenses)
            .zip(source.attacks.iter().chain(&source.defenses))
        {
            assert_eq!(
                (&actual.ability, actual.interval_ms, actual.chance_ppm),
                (&expected.ability, expected.interval_ms, expected.chance_ppm),
                "{}",
                record.profile.target.key
            );
        }
        checked += 1;
    }
    assert!(refused.is_empty(), "admitted profiles refused: {refused:?}");
    assert_eq!(
        checked,
        document.records.len(),
        "every admitted record carries behavior"
    );
}

#[test]
fn production_distance_area_and_fleeing_profiles_match_crystal() {
    use crate::content::native_gameplay::CreatureProfilesDocument;
    let document: CreatureProfilesDocument = serde_json::from_slice(include_bytes!(
        "../../../../../../content/creatures/definitions/spell-native-profiles.json"
    ))
    .expect("admitted native creature profiles");
    // Crystal201189f: humans/hunter.lua:38-59,101-104;
    // humans/warlock.lua:37-58,114-123; demons/demon.lua:36-60,127-135.
    for (key, flee_health, distance, chance, attack_count, spell_index, spell_chance) in [
        ("hunter", 10, 4, 100_000, 2, 1, 500_000),
        ("warlock", 900, 4, 100_000, 8, 4, 200_000),
        ("demon", 0, 1, 200_000, 7, 2, 200_000),
    ] {
        let record = document
            .records
            .iter()
            .find(|record| record.profile.target.key == format!("oteryn:creature.{key}"))
            .expect("source creature");
        let profile =
            super::super::native_profile::project_native_record(record).expect("native profile");
        let targeting = profile.targeting();
        assert_eq!(
            (targeting.flee_health, targeting.target_distance_tiles),
            (flee_health, distance),
            "{key}"
        );
        assert_eq!(
            targeting.change_target,
            Some(crate::content::ProjectV2ChangeTarget {
                interval_ms: 4000,
                chance_ppm: chance,
            }),
            "{key}"
        );
        assert_eq!(profile.behavior().attacks.len(), attack_count, "{key}");
        let attack = &profile.behavior().attacks[spell_index];
        assert_eq!(
            (attack.interval_ms, attack.chance_ppm),
            (2000, spell_chance),
            "{key}"
        );
        let details = profile
            .abilities()
            .get(&attack.ability)
            .and_then(|ability| ability.details.as_ref())
            .expect("qualified ranged or area spell");
        assert_eq!(details.range_tiles, 7, "{key}");
        let mut state = CreatureAiState::new(rat(), profile, at(100, 100));
        let facts = Facts::new();
        let players = [player(10, at(103, 100))];
        let report = think(
            &mut state,
            1,
            &facts.at(at(100, 100), flee_health + 1, &players, 0),
        );
        assert!(!report.fleeing, "{key}");
        if flee_health > 0 {
            let report = think(
                &mut state,
                2,
                &facts.at(at(100, 100), flee_health, &players, 1_000_000),
            );
            assert!(
                report.fleeing && matches!(report.goal, MovementGoal::Flee { .. }),
                "{key}"
            );
        }
    }
}
