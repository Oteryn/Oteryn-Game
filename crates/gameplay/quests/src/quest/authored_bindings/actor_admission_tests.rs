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
