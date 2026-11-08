use super::*;

#[test]
fn m20_carried_recipe_requires_two_explicit_inputs_and_binds_target() -> Result<(), String> {
    let mut document: serde_json::Value =
        serde_json::from_str(DOCUMENTS[0]).map_err(|e| e.to_string())?;
    let row = document["callbacks"]
        .as_array_mut()
        .ok_or("callbacks")?
        .iter_mut()
        .find(|row| row["carried_item"].is_string())
        .ok_or("carried callback")?;
    let key = row["key"].as_str().ok_or("key")?.to_owned();
    let source = row["carried_item"].clone();
    row["carried_target"] = "oteryn:item.tibia.i21507".into();
    row["take"] = serde_json::json!([
        {"item":source,"count":1}, {"item":"oteryn:item.tibia.i21507","count":1}
    ]);
    row["give"] = serde_json::json!({"item":"oteryn:item.tibia.i21506","count":1,"contents":[]});
    row["outfit"] = serde_json::Value::Null;
    let parsed = parse(&document.to_string())?;
    let recipe = parsed.iter().find(|c| c.key == key).ok_or("recipe")?;
    let original = gated_binding(recipe)?.ok_or("recipe target must bind quest effects")?;
    let mut changed = serde_json::to_value(recipe).map_err(|e| e.to_string())?;
    changed["carried_target"] = "oteryn:item.tibia.i21504".into();
    let changed: RewardCallback = serde_json::from_value(changed).map_err(|e| e.to_string())?;
    assert_ne!(gated_binding(&changed)?, Some(original));
    let mut missing = recipe.clone();
    missing.take.pop();
    assert!(validate_callbacks(&[missing]).is_err());
    let mut ambiguous = recipe.clone();
    ambiguous.peer_requires = Some(TrackRequirement {
        track: recipe.storage[0].track.clone(),
        minimum: 0,
        maximum: 1,
    });
    assert!(validate_callbacks(&[ambiguous]).is_err());
    Ok(())
}

#[test]
fn m20_cursed_crystal_final_plans_preserve_higher_progress_and_bind_all_outputs()
-> Result<(), String> {
    let callbacks = load()?;
    let recipes: Vec<_> = callbacks
        .iter()
        .filter(|c| c.key.starts_with("oteryn:quest-reward/m20/"))
        .collect();
    assert_eq!(recipes.len(), 2);
    for recipe in recipes {
        let transition = primary_transition(recipe)?;
        let track = &recipe.storage[0];
        let catalogue = QuestStateCatalogue::new(
            "m20-recipes",
            vec![super::super::QuestTrack {
                key: track.track.clone(),
                quest: recipe.quest.clone(),
                initial: -1,
                min: -1,
                max: 4,
            }],
            vec![transition.clone()],
        )
        .map_err(|e| format!("{e:?}"))?;
        for (before, expected) in [(-1, 2), (1, 2), (2, 2), (3, 3), (4, 4)] {
            let changes = catalogue
                .evaluate(
                    &transition,
                    &BTreeMap::from([(track.track.clone(), before)]),
                    0,
                )
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(changes[0].after, expected);
        }
        let binding = gated_binding(recipe)?.ok_or("recipe binding")?;
        let mut multi = recipe.clone();
        multi.take.push(ItemAmount {
            item: "oteryn:item.tibia.i3031".into(),
            count: 3,
            fluid_subtype: None,
        });
        multi
            .give
            .as_mut()
            .ok_or("give")?
            .additional
            .push(ItemAmount {
                item: "oteryn:item.tibia.i3035".into(),
                count: 2,
                fluid_subtype: None,
            });
        validate_callbacks(std::slice::from_ref(&multi))?;
        assert_ne!(gated_binding(&multi)?, Some(binding));
    }
    Ok(())
}

#[test]
fn m20_durable_recipe_plan_is_distinct_from_legacy_tool_proofs() -> Result<(), String> {
    let callbacks = load()?;
    let recipe = callbacks
        .iter()
        .find(|c| c.carried_target.is_some())
        .ok_or("recipe")?;
    let plan = durable_plan(recipe)?;
    assert!(plan.get("carried_item").is_none());
    assert_eq!(
        plan["recipe_source"].as_str(),
        recipe.carried_item.as_deref()
    );
    assert_eq!(
        plan["carried_target"].as_str(),
        recipe.carried_target.as_deref()
    );
    let legacy = callbacks
        .iter()
        .find(|c| c.carried_item.is_some() && c.carried_target.is_none())
        .ok_or("tool")?;
    assert_eq!(
        durable_plan(legacy)?,
        serde_json::to_value(legacy).map_err(|e| e.to_string())?
    );
    Ok(())
}
