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
