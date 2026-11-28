use crate::content::{
    ProjectReferenceRecord, ProjectV2AbilityEffect, ProjectV2AuthoringProfile,
    ProjectV2AuthoringProfileData, ProjectV2DefinitionRef, ProjectV2Family,
};
use crate::creature_damage_spell::SpellSource;
use crate::foundation::{
    ApplicationFacts, ConditionSourceKind, ConditionStore, ConditionType, TickFacts, TickKind,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use serde_json::{Value, json};

#[test]
fn authored_creature_agony_spell_executes_the_exact_schedule()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture: Value =
        serde_json::from_str(include_str!("../../creature_auto_attack_test_data.json"))?;
    let records: Vec<ProjectReferenceRecord> = serde_json::from_value(fixture["records"].clone())?;
    let mut profiles: Vec<ProjectV2AuthoringProfile> =
        serde_json::from_value(fixture["authoring_profiles"].clone())?;
    let profile = profiles
        .iter_mut()
        .find(|profile| profile.target.key == "oteryn:ability.creature.1st_mate_ratticus.attack-2")
        .ok_or("missing native spell fixture")?;
    let ProjectV2AuthoringProfileData::Ability(ability) = &mut profile.data else {
        return Err("expected an ability profile".into());
    };

    // Use the archived Global Agony cadence with a fixed amount for a 5,000 HP target.
    // The native spell fixture exercises qualification of the inline attachment.
    let count = 11;
    let interval_ms = 1_000;
    let amount = 5_000 * 2 / 100;
    let effect = serde_json::from_value(json!({
        "key": "oteryn:effect.creature.1st_mate_ratticus.attack-2-agony",
        "operation": {
            "operation": "Condition",
            "condition": {
                "condition_type": "agony",
                "lifetime": "DamageSchedule",
                "damage_over_time": {
                    "tick_profile": "Fixed",
                    "first_tick": "AfterInterval",
                    "ticks": [{"count": count, "interval_ms": interval_ms, "amount": amount}]
                }
            }
        }
    }))?;
    ability
        .details
        .as_mut()
        .ok_or("missing spell details")?
        .effects
        .push(ProjectV2AbilityEffect::Inline(Box::new(effect)));
    let creature = ProjectV2DefinitionRef {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.1st_mate_ratticus".into(),
        revision: "definition-r1".into(),
    };
    let source = SpellSource::from_native(&creature, 1, &records, &profiles, [1; 32])
        .map_err(|error| format!("qualify native Agony spell: {error:?}"))?;
    let [definition] = source.conditions.as_slice() else {
        return Err("expected one qualified condition".into());
    };
    assert_eq!(definition.condition_type(), ConditionType::Agony);

    let facts = ApplicationFacts {
        now: 0,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &GameplayDecisionRoot::from_bytes(source.content_digest),
        occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
    };
    let mut store = ConditionStore::<u32>::new();
    store
        .apply(
            definition,
            Some(1),
            ConditionSourceKind::Creature,
            &[],
            &facts,
        )
        .map_err(|error| format!("apply qualified Agony: {error:?}"))?;
    let tick_facts = TickFacts {
        in_protection_zone: true,
        ..TickFacts::default()
    };
    for tick in 1..=count {
        let due = tick * interval_ms * 1_000;
        assert!(store.take_due(due - 1, tick_facts).is_empty());
        let ticks = store.take_due(due, tick_facts);
        assert_eq!(ticks.len(), 1);
        assert_eq!(
            ticks[0].kind,
            TickKind::Agony {
                amount,
                refused: false
            }
        );
    }
    assert!(store.instances().is_empty());
    Ok(())
}
