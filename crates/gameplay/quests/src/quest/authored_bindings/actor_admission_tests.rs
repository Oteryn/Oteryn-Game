use super::Bindings;

#[test]
fn imported_level_gate_matches_the_chosen_recipe_before_native_admission() -> Result<(), String> {
    let mut document: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/collection.json"))
            .map_err(|e| e.to_string())?;
    document["quests"][0]["admission"] = serde_json::json!({"minimum_level":50});
    let binding = Bindings::parse(&serde_json::to_vec(&document).map_err(|e| e.to_string())?)?;
    let loaded =
        crate::quest::loader::load_embedded_quest_state_with_bindings("level-fixture", &binding)
            .map_err(|e| format!("{e:?}"))?;
    assert!(
        loaded
            .catalogue()
            .transition("oteryn:quest-collection/m01c/black-knight-key")
            .is_some()
    );
    document["quests"][0]["admission"]["minimum_level"] = 49.into();
    let wrong = Bindings::parse(&serde_json::to_vec(&document).map_err(|e| e.to_string())?)?;
    assert!(
        crate::quest::loader::load_embedded_quest_state_with_bindings("level-fixture", &wrong)
            .is_err()
    );
    Ok(())
}

#[test]
fn production_black_knight_collection_enforces_its_canonical_level() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("black-knight-fixture")
        .map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue
        .transition("oteryn:quest-collection/m01c-black-knight-key")
        .ok_or("missing production Black Knight collection")?;
    assert_eq!(transition.quest, "oteryn:quest.black_knight_quest");
    assert!(!transition.completes);
    let quest = catalogue
        .authored_bindings()
        .and_then(|b| b.quests.iter().find(|q| q.quest == transition.quest))
        .ok_or("binding")?;
    assert_eq!(quest.admission.as_ref().map(|a| a.minimum_level), Some(50));
    let super::Trigger::Collect { collection } = &quest.stages[0].trigger else {
        return Err("collection".into());
    };
    assert_eq!(collection.items[0].item, "oteryn:item.tibia.i2969");
    assert_eq!(collection.items[0].key_number, Some(5010));
    Ok(())
}

#[test]
fn adding_premium_support_preserves_the_published_level_only_shape() -> Result<(), String> {
    let admission: super::actor_admission::ActorAdmission =
        serde_json::from_value(serde_json::json!({"minimum_level":50}))
            .map_err(|e| e.to_string())?;
    assert_eq!(
        serde_json::to_value(admission).map_err(|e| e.to_string())?,
        serde_json::json!({"minimum_level":50})
    );
    Ok(())
}

#[test]
fn production_grimvale_collection_requires_its_premium_blossom_count() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("grimvale-fixture")
        .map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue
        .transition("oteryn:quest-collection/m01c-grimvale-blossoms")
        .ok_or("missing production Grimvale collection")?;
    assert!(!transition.completes);
    let quest = catalogue
        .authored_bindings()
        .and_then(|b| {
            b.quests
                .iter()
                .find(|q| q.quest == "oteryn:quest.grimvale_quest")
        })
        .ok_or("Grimvale binding")?;
    assert!(
        quest
            .admission
            .as_ref()
            .is_some_and(|a| a.premium && a.minimum_level == 0)
    );
    let super::Trigger::Collect { collection } = &quest.stages[0].trigger else {
        return Err("collection".into());
    };
    assert_eq!(collection.items[0].item, "oteryn:item.tibia.i22082");
    assert_eq!(collection.items[0].count, 5);
    Ok(())
}

#[test]
fn production_decaying_defender_boss_entry_preserves_requirements_and_cannot_complete()
-> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("decaying-defender-fixture")
        .map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let key = "oteryn:quest-transition/native-kills/m01c-decaying-defender-bakragore";
    let transition = catalogue
        .transition(key)
        .ok_or("missing Decaying Defender entry")?;
    let quest = catalogue
        .authored_bindings()
        .and_then(|b| b.quests.iter().find(|q| q.quest == transition.quest))
        .ok_or("binding")?;
    let admission = quest.admission.as_ref().ok_or("admission")?;
    assert_eq!(admission.minimum_level, 250);
    assert!(admission.premium);
    assert_eq!(admission.prerequisites.len(), 1);
    assert_eq!(
        admission.prerequisites[0].requirement,
        "Rotten Blood journey"
    );
    assert_eq!(
        admission.prerequisites[0].quest,
        crate::quest::rotten_blood::QUEST
    );
    let counter = crate::quest::native_kills::counter_for(catalogue, key).ok_or("counter")?;
    assert_eq!(counter.creature, "oteryn:creature.bakragore");
    let mut state = std::collections::BTreeMap::from([
        (counter.gate.to_owned(), 0),
        (counter.track.to_owned(), 0),
    ]);
    let changes = catalogue
        .evaluate(transition, &state, 0)
        .map_err(|e| format!("{e:?}"))?;
    assert!(!catalogue.completes_after(transition, &changes));
    for change in changes {
        state.insert(change.track, change.after);
    }
    assert_eq!(state[counter.gate], 1);
    assert_eq!(state[counter.track], 1);
    assert_eq!(
        catalogue.evaluate(transition, &state, 0),
        Err(crate::quest::QuestRefusal::StageMismatch)
    );
    Ok(())
}
